//! État du dashboard TUI (Task 11) — modèle pur, sans terminal.
//!
//! [`App`] accumule les [`Event`] du pipeline ([`App::apply`]), expose
//! l'arbre visible ([`App::visible_rows`]), la navigation (↑↓/jk), le
//! filtre « échecs seulement », la reconstruction depuis un [`Report`]
//! (mode `browse`) et l'export texte. Le rendu (`tree`, `detail`) lit
//! `App` sans jamais le muter : toute la logique est testée hors
//! terminal (`tests/tui.rs`).

use crate::engine::events::{Event, Step, TestVerdict};
use crate::norme::NormeFault;
use crate::report::{render_text, Report, TestRecord};
use anyhow::{Context, Result};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::time::{Instant, SystemTime};

/// Nombre max de lignes de logs conservées par étape. Au-delà, un
/// marqueur « … N lignes tronquées … » clôt la liste : jamais plus de
/// `MAX_LOGS + 1` entrées, jamais de croissance non bornée.
pub const MAX_LOGS: usize = 500;

/// Pas de défilement du panneau de détail (lignes par PgUp/PgDn).
pub(crate) const SCROLL_PAS: usize = 10;

/// Frames du spinner d'étape en cours (header + arbre).
pub(crate) const SPINNER: &str = "│⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏";

/// Les 7 étapes dans l'ordre d'affichage de l'arbre (= ordre pipeline).
pub(crate) const ETAPES: [Step; 7] = [
    Step::Prelim,
    Step::Build,
    Step::Norme,
    Step::Symbols,
    Step::Unit,
    Step::Functional,
    Step::Verdict,
];

/// Statut d'une étape dans l'arbre.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepStatus {
    Pending,
    Running,
    Ok,
    Ko,
    Skipped,
}

/// Ligne sélectionnée dans l'arbre.
///
/// Les index de [`Selection::Test`] (`group`, `index`) désignent les
/// vecteurs COMPLETS ([`App::groups`], tests du groupe) : la sélection
/// reste stable quand le filtre échecs masque des lignes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Selection {
    Step(Step),
    Fault(usize),
    Test { group: usize, index: usize },
}

/// État affiché d'une étape : statut, logs cappés, checks, résumé.
#[derive(Debug, Clone)]
pub struct StepState {
    pub label: String,
    pub status: StepStatus,
    pub logs: Vec<String>,
    /// (nom, ok, détail) de chaque check remonté par l'étape.
    pub checks: Vec<(String, bool, String)>,
    pub summary: Option<String>,
    /// Logs perdus par troncature (compteur du marqueur final).
    tronquees: usize,
}

impl StepState {
    fn nouvelle(step: Step) -> Self {
        Self {
            label: step.label().to_string(),
            status: StepStatus::Pending,
            logs: Vec::new(),
            checks: Vec::new(),
            summary: None,
            tronquees: 0,
        }
    }

    /// Ajoute une ligne de log, cappée à [`MAX_LOGS`] : au-delà, un
    /// marqueur final « … N lignes tronquées … » (mis à jour) clôt la
    /// liste.
    fn pousse_log(&mut self, line: String) {
        if self.logs.len() < MAX_LOGS {
            self.logs.push(line);
            return;
        }
        self.tronquees += 1;
        let marqueur = format!("… {} lignes tronquées …", self.tronquees);
        if self.logs.len() > MAX_LOGS {
            // Le marqueur existe déjà : on le met à jour en place.
            if let Some(derniere) = self.logs.last_mut() {
                *derniere = marqueur;
            }
        } else {
            self.logs.push(marqueur);
        }
    }
}

/// Un test unitaire/fonctionnel dans l'arbre.
#[derive(Debug, Clone)]
pub struct TestState {
    pub name: String,
    /// None = en cours (Started sans Finished).
    pub verdict: Option<TestVerdict>,
    /// Détail court matérialisé pour l'affichage (« SIGSEGV »,
    /// « délai dépassé ») ; None pour passed/failed (le diff porte tout).
    pub detail: Option<String>,
}

/// Un groupe de tests (« unit », « functional »…), dans l'ordre
/// d'apparition des events.
#[derive(Debug, Clone)]
pub struct GroupState {
    pub name: String,
    pub tests: Vec<TestState>,
}

impl GroupState {
    /// Nombre de tests du groupe.
    pub fn total(&self) -> usize {
        self.tests.len()
    }

    /// Nombre de tests passed.
    pub fn passed(&self) -> usize {
        self.tests
            .iter()
            .filter(|t| t.verdict == Some(TestVerdict::Passed))
            .count()
    }

    /// Pourcentage de passed, arrondi au dixième ; 0.0 sur groupe vide
    /// (jamais de NaN).
    pub fn percent(&self) -> f64 {
        let total = self.total();
        if total == 0 {
            0.0
        } else {
            (1000.0 * self.passed() as f64 / total as f64).round() / 10.0
        }
    }
}

/// Étape parente d'un groupe de tests : `unit`/`functional` sous leur
/// étape ; tout autre nom est rattaché au Verdict — un groupe au nom
/// inconnu reste affiché, jamais perdu.
pub(crate) fn groupe_etape(name: &str) -> Step {
    match Step::from_name(name) {
        Some(s @ (Step::Unit | Step::Functional)) => s,
        _ => Step::Verdict,
    }
}

/// État complet du dashboard.
pub struct App {
    battery: String,
    started: Instant,
    steps: BTreeMap<Step, StepState>,
    groups: Vec<GroupState>,
    norme: Vec<NormeFault>,
    selected: Selection,
    /// Suivi automatique : tant que l'utilisateur ne navigue pas, la
    /// sélection colle à l'étape courante.
    follow: bool,
    filter_failures: bool,
    /// Recul du détail en lignes depuis le bas (0 = suit la fin).
    scroll_back: usize,
    spinner_frame: usize,
    /// Dernière étape ayant démarré ou fini (header entre deux étapes).
    current_step: Option<Step>,
    finished: Option<Report>,
    /// Message éphémère du footer (confirmation/erreur d'export).
    /// `RefCell` : `export` est en `&self` (cf. tests).
    footer_message: RefCell<Option<String>>,
}

impl App {
    /// App fraîche pour la batterie `battery` : les 7 étapes existent
    /// d'emblée en [`StepStatus::Pending`], sélection sur la première.
    pub fn new(battery: String, started: Instant) -> Self {
        let steps = ETAPES
            .iter()
            .map(|s| (*s, StepState::nouvelle(*s)))
            .collect();
        Self {
            battery,
            started,
            steps,
            groups: Vec::new(),
            norme: Vec::new(),
            selected: Selection::Step(Step::Prelim),
            follow: true,
            filter_failures: false,
            scroll_back: 0,
            spinner_frame: 0,
            current_step: None,
            finished: None,
            footer_message: RefCell::new(None),
        }
    }

    /// Reconstruit l'état depuis un rapport existant (mode navigation
    /// seule, `all-seeing-eye report`) : statuts/checks/résumés des
    /// étapes nommées (une étape absente reste Pending, jamais
    /// inventée), groupes dans l'ordre des records, fautes de norme,
    /// et le rapport posé — post-run d'emblée.
    pub fn from_report(report: Report) -> Self {
        let mut app = Self::new(report.project.clone(), Instant::now());
        for sr in &report.steps {
            if let Some(step) = Step::from_name(&sr.step) {
                let st = app
                    .steps
                    .entry(step)
                    .or_insert_with(|| StepState::nouvelle(step));
                // Un skip n'est jamais un succès, même si ok est vrai.
                st.status = if sr.skipped {
                    StepStatus::Skipped
                } else if sr.ok {
                    StepStatus::Ok
                } else {
                    StepStatus::Ko
                };
                st.summary = Some(sr.summary.clone());
                st.checks = sr.checks.clone();
            }
        }
        for t in &report.tests {
            let verdict = verdict_depuis_record(t);
            let detail = t.detail.clone().or_else(|| detail_depuis_verdict(&verdict));
            let (g, i) = app.test_ou_cree(&t.group, &t.name);
            app.groups[g].tests[i].verdict = Some(verdict);
            app.groups[g].tests[i].detail = detail;
        }
        app.norme = report.norme.clone();
        // Post-run d'emblée : pas de suivi auto, rapport posé.
        app.follow = false;
        app.finished = Some(report);
        app
    }

    /// Applique un événement du pipeline.
    pub fn apply(&mut self, ev: Event) {
        match ev {
            Event::StepStarted { step, label } => {
                let st = self
                    .steps
                    .entry(step)
                    .or_insert_with(|| StepState::nouvelle(step));
                st.status = StepStatus::Running;
                st.label = label;
                self.current_step = Some(step);
                // Suivi auto : la sélection colle à l'étape courante,
                // sauf si l'utilisateur a navigué (follow coupé) ou si
                // le filtre échecs masque cette étape.
                let cible = Selection::Step(step);
                if self.follow && self.visible_rows().contains(&cible) {
                    self.selected = cible;
                    self.scroll_back = 0;
                }
            }
            Event::LogLine { step, line } => {
                self.steps
                    .entry(step)
                    .or_insert_with(|| StepState::nouvelle(step))
                    .pousse_log(line);
            }
            Event::CheckFinished {
                step,
                name,
                ok,
                detail,
            } => {
                self.steps
                    .entry(step)
                    .or_insert_with(|| StepState::nouvelle(step))
                    .checks
                    .push((name, ok, detail));
            }
            Event::TestStarted { group, name } => {
                self.test_ou_cree(&group, &name);
            }
            Event::TestFinished {
                group,
                name,
                result,
            } => {
                // Un Finished orphelin crée le test quand même : un
                // résultat réel ne disparaît jamais.
                let detail = detail_depuis_verdict(&result);
                let (g, i) = self.test_ou_cree(&group, &name);
                self.groups[g].tests[i].verdict = Some(result);
                self.groups[g].tests[i].detail = detail;
            }
            Event::NormeFault(f) => self.norme.push(f),
            Event::StepFinished {
                step,
                ok,
                skipped,
                summary,
            } => {
                let st = self
                    .steps
                    .entry(step)
                    .or_insert_with(|| StepState::nouvelle(step));
                // Un skip n'est jamais un succès, même si ok est vrai.
                st.status = if skipped {
                    StepStatus::Skipped
                } else if ok {
                    StepStatus::Ok
                } else {
                    StepStatus::Ko
                };
                st.summary = Some(summary);
                self.current_step = Some(step);
            }
            Event::RunFinished { report } => {
                self.finished = Some(*report);
                self.follow = false;
            }
        }
    }

    /// Groupe + test, créés si absents (ordre d'apparition conservé) ;
    /// retourne leurs index.
    fn test_ou_cree(&mut self, group: &str, name: &str) -> (usize, usize) {
        let g = match self.groups.iter().position(|g| g.name == group) {
            Some(p) => p,
            None => {
                self.groups.push(GroupState {
                    name: group.to_string(),
                    tests: Vec::new(),
                });
                self.groups.len() - 1
            }
        };
        let i = match self.groups[g].tests.iter().position(|t| t.name == name) {
            Some(p) => p,
            None => {
                self.groups[g].tests.push(TestState {
                    name: name.to_string(),
                    verdict: None,
                    detail: None,
                });
                self.groups[g].tests.len() - 1
            }
        };
        (g, i)
    }

    // ── accesseurs ────────────────────────────────────────────────

    /// État d'une étape (toujours présente : pré-enregistrées).
    pub fn step(&self, step: Step) -> &StepState {
        static VIDE: StepState = StepState {
            label: String::new(),
            status: StepStatus::Pending,
            logs: Vec::new(),
            checks: Vec::new(),
            summary: None,
            tronquees: 0,
        };
        self.steps.get(&step).unwrap_or(&VIDE)
    }

    /// Groupes de tests dans l'ordre d'apparition.
    pub fn groups(&self) -> &[GroupState] {
        &self.groups
    }

    /// Fautes de norme accumulées, dans l'ordre d'arrivée.
    pub fn norme(&self) -> &[NormeFault] {
        &self.norme
    }

    /// Sélection courante (peut pointer une ligne masquée par le
    /// filtre — le détail l'affiche quand même).
    pub fn selected(&self) -> Selection {
        self.selected
    }

    /// Rapport final, posé par [`Event::RunFinished`] ou
    /// [`App::from_report`].
    pub fn finished(&self) -> Option<&Report> {
        self.finished.as_ref()
    }

    /// Nom de la batterie (= projet du rapport en mode browse).
    pub fn battery(&self) -> &str {
        &self.battery
    }

    /// Filtre « échecs seulement » actif ?
    pub fn filter_failures(&self) -> bool {
        self.filter_failures
    }

    /// Recul du détail en lignes depuis le bas (0 = suit la fin).
    pub fn scroll_back(&self) -> usize {
        self.scroll_back
    }

    /// Message éphémère du footer (confirmation/erreur d'export).
    pub fn footer_message(&self) -> Option<String> {
        self.footer_message.borrow().clone()
    }

    pub(crate) fn set_footer_message(&self, msg: String) {
        *self.footer_message.borrow_mut() = Some(msg);
    }

    pub(crate) fn started(&self) -> Instant {
        self.started
    }

    pub(crate) fn current_step(&self) -> Option<Step> {
        self.current_step
    }

    /// Frame courante du spinner (avancée par [`App::tick`]).
    pub(crate) fn spinner_char(&self) -> char {
        let n = SPINNER.chars().count();
        SPINNER.chars().nth(self.spinner_frame % n).unwrap_or('│')
    }

    /// Avance le spinner d'une frame (appelé à chaque tour de boucle
    /// tant que le run n'est pas fini).
    pub(crate) fn tick(&mut self) {
        self.spinner_frame = self.spinner_frame.wrapping_add(1);
    }

    // ── arbre visible ─────────────────────────────────────────────

    /// Lignes de l'arbre dans l'ordre d'affichage : les 7 étapes dans
    /// l'ordre pipeline, avec sous Norme ses fautes, sous
    /// Unit/Functional leurs tests (les groupes au nom inconnu sont
    /// rattachés au Verdict). En mode filtre : seules les étapes KO
    /// et les parents d'échecs, avec leurs feuilles en échec.
    pub fn visible_rows(&self) -> Vec<Selection> {
        let mut rows = Vec::new();
        for step in ETAPES {
            let feuilles: Vec<Selection> = self
                .feuilles(step)
                .into_iter()
                .filter(|s| !self.filter_failures || self.echec(*s))
                .collect();
            if self.filter_failures {
                let ko = self.step(step).status == StepStatus::Ko;
                // Visible si KO soi-même ou parent de feuilles en échec.
                if !ko && feuilles.is_empty() {
                    continue;
                }
            }
            rows.push(Selection::Step(step));
            rows.extend(feuilles);
        }
        rows
    }

    /// Feuilles d'une étape : fautes sous Norme, tests sous leur étape
    /// de rattachement, rien pour les autres.
    fn feuilles(&self, step: Step) -> Vec<Selection> {
        if step == Step::Norme {
            return (0..self.norme.len()).map(Selection::Fault).collect();
        }
        let mut out = Vec::new();
        for (g, groupe) in self.groups.iter().enumerate() {
            if groupe_etape(&groupe.name) == step {
                out.extend((0..groupe.tests.len()).map(|i| Selection::Test { group: g, index: i }));
            }
        }
        out
    }

    /// La feuille est-elle un échec (vue filtrée) ? Toute faute de
    /// norme en est un ; un test n'en est un qu'avec un verdict
    /// failed/crashed/timeout (en cours ≠ échec).
    fn echec(&self, s: Selection) -> bool {
        match s {
            Selection::Fault(_) => true,
            Selection::Test { group, index } => matches!(
                self.groups[group].tests[index].verdict,
                Some(TestVerdict::Failed { .. } | TestVerdict::Crashed(_) | TestVerdict::Timeout)
            ),
            Selection::Step(_) => true, // jamais appelé avec une étape
        }
    }

    // ── navigation ────────────────────────────────────────────────

    /// Ligne suivante (bornée en bas) ; coupe le suivi auto.
    pub fn next(&mut self) {
        let rows = self.visible_rows();
        if rows.is_empty() {
            return;
        }
        self.follow = false;
        self.selected = match rows.iter().position(|s| *s == self.selected) {
            Some(p) => rows[(p + 1).min(rows.len() - 1)],
            // Sélection hors champ (filtre) : on redescend en tête.
            None => rows[0],
        };
        self.scroll_back = 0;
    }

    /// Ligne précédente (bornée en haut) ; coupe le suivi auto.
    pub fn prev(&mut self) {
        let rows = self.visible_rows();
        if rows.is_empty() {
            return;
        }
        self.follow = false;
        self.selected = match rows.iter().position(|s| *s == self.selected) {
            Some(p) => rows[p.saturating_sub(1)],
            None => rows[rows.len() - 1],
        };
        self.scroll_back = 0;
    }

    /// Remonte le détail d'un cran (plus on remonte, plus `scroll_back`
    /// grandit ; 0 = collé en bas, suit les derniers logs).
    pub fn scroll_up(&mut self) {
        self.scroll_back = self.scroll_back.saturating_add(SCROLL_PAS);
    }

    /// Redescend le détail d'un cran, sans jamais passer sous 0.
    pub fn scroll_down(&mut self) {
        self.scroll_back = self.scroll_back.saturating_sub(SCROLL_PAS);
    }

    /// Bascule le filtre « échecs seulement ». En l'activant, une
    /// sélection devenue invisible est repliée sur le voisin visible
    /// le plus proche AVANT dans l'ordre de l'arbre (le parent, en
    /// pratique) ; à défaut, la première ligne visible.
    pub fn toggle_failures(&mut self) {
        let avant = self.visible_rows();
        self.filter_failures = !self.filter_failures;
        if !self.filter_failures {
            return; // réactivation : tout redevient visible, rien à faire
        }
        let rows = self.visible_rows();
        if rows.contains(&self.selected) {
            return;
        }
        let repli = avant
            .iter()
            .position(|s| *s == self.selected)
            .and_then(|p| (0..p).rev().find(|i| rows.contains(&avant[*i])))
            .map(|i| avant[i])
            .or_else(|| rows.first().copied());
        if let Some(s) = repli {
            self.selected = s;
            self.scroll_back = 0;
        }
    }

    // ── export ────────────────────────────────────────────────────

    /// Exporte le rapport texte ([`render_text`]) dans
    /// `data_dir/all-seeing-eye/export-<ts>.txt` et pose un message de
    /// confirmation dans le footer. Erreur s'il n'y a pas de rapport
    /// (run non terminé).
    pub fn export(&self) -> Result<PathBuf> {
        let report = self
            .finished
            .as_ref()
            .context("aucun rapport à exporter : le run n'est pas terminé")?;
        let dir = dirs::data_dir()
            .context("aucun data dir (ni XDG_DATA_HOME ni HOME)")?
            .join("all-seeing-eye");
        fs::create_dir_all(&dir)
            .with_context(|| format!("création de {} impossible", dir.display()))?;
        let ts = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let path = dir.join(format!("export-{ts}.txt"));
        fs::write(&path, render_text(report))
            .with_context(|| format!("écriture de {} impossible", path.display()))?;
        self.set_footer_message(format!("exporté → {}", path.display()));
        Ok(path)
    }
}

/// Reconstruit un [`TestVerdict`] depuis un [`TestRecord`] sérialisé.
/// Le rapport ne stocke pas le signal d'un crash :
/// [`TestVerdict::Crashed(-1)`] — code impossible en live — marque
/// « signal inconnu » (affiché « crash » ; le `detail` du record porte
/// le vrai nom quand il existe).
fn verdict_depuis_record(t: &TestRecord) -> TestVerdict {
    match t.verdict.as_str() {
        "passed" => TestVerdict::Passed,
        "failed" => TestVerdict::Failed {
            diff: t.diff.clone().unwrap_or_default(),
            expected: String::new(),
            got: String::new(),
        },
        "crashed" => TestVerdict::Crashed(-1),
        "timeout" => TestVerdict::Timeout,
        // Verdict hors contrat : compté comme un échec visible, jamais
        // rangé silencieusement en passed.
        _ => TestVerdict::Failed {
            diff: String::new(),
            expected: String::new(),
            got: String::new(),
        },
    }
}

/// Détail court matérialisé pour l'affichage : nom du signal pour un
/// crash, « délai dépassé » pour un timeout ; None sinon (passed, ou
/// failed dont le diff porte tout).
fn detail_depuis_verdict(v: &TestVerdict) -> Option<String> {
    match v {
        TestVerdict::Crashed(sig) => Some(nom_signal(*sig)),
        TestVerdict::Timeout => Some("délai dépassé".to_string()),
        TestVerdict::Passed | TestVerdict::Failed { .. } => None,
    }
}

/// Nom lisible d'un signal POSIX ; « crash » pour le code sentinelle
/// des rapports sérialisés (signal inconnu), « signal N » en repli.
fn nom_signal(sig: i32) -> String {
    let nom = match sig {
        2 => "SIGINT",
        4 => "SIGILL",
        6 => "SIGABRT",
        8 => "SIGFPE",
        9 => "SIGKILL",
        11 => "SIGSEGV",
        13 => "SIGPIPE",
        15 => "SIGTERM",
        s if s < 0 => return "crash".to_string(),
        s => return format!("signal {s}"),
    };
    nom.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> App {
        App::new("mini".to_string(), Instant::now())
    }

    #[test]
    fn nom_signal_connus_inconnus_et_sentinelle() {
        assert_eq!(nom_signal(11), "SIGSEGV");
        assert_eq!(nom_signal(6), "SIGABRT");
        assert_eq!(nom_signal(2), "SIGINT");
        assert_eq!(nom_signal(42), "signal 42");
        assert_eq!(nom_signal(-1), "crash");
    }

    #[test]
    fn logs_exactement_max_sans_marqueur_puis_marqueur_compte() {
        let mut a = app();
        a.apply(Event::StepStarted {
            step: Step::Build,
            label: Step::Build.label().to_string(),
        });
        for i in 0..MAX_LOGS {
            a.apply(Event::LogLine {
                step: Step::Build,
                line: format!("ligne {i}"),
            });
        }
        // Pile au cap : aucune perte, aucun marqueur.
        assert_eq!(a.step(Step::Build).logs.len(), MAX_LOGS);
        assert!(!a.step(Step::Build).logs[MAX_LOGS - 1].contains("tronqu"));
        // Premier dépassement : le marqueur apparaît, compteur à 1…
        a.apply(Event::LogLine {
            step: Step::Build,
            line: "de trop".to_string(),
        });
        assert_eq!(a.step(Step::Build).logs.len(), MAX_LOGS + 1);
        assert_eq!(
            a.step(Step::Build).logs.last().unwrap(),
            "… 1 lignes tronquées …"
        );
        // …et il se met à jour sans jamais grandir.
        a.apply(Event::LogLine {
            step: Step::Build,
            line: "encore".to_string(),
        });
        assert_eq!(a.step(Step::Build).logs.len(), MAX_LOGS + 1);
        assert_eq!(
            a.step(Step::Build).logs.last().unwrap(),
            "… 2 lignes tronquées …"
        );
    }

    #[test]
    fn groupe_inconnu_est_ratache_au_verdict_jamais_perdu() {
        let mut a = app();
        a.apply(Event::TestFinished {
            group: "bonus".to_string(),
            name: "x".to_string(),
            result: TestVerdict::Passed,
        });
        let rows = a.visible_rows();
        let verdict = rows
            .iter()
            .position(|s| *s == Selection::Step(Step::Verdict))
            .unwrap();
        let test = rows
            .iter()
            .position(|s| *s == (Selection::Test { group: 0, index: 0 }))
            .unwrap();
        assert_eq!(test, verdict + 1, "le groupe inconnu suit le Verdict");
    }

    #[test]
    fn verdict_depuis_record_mapping_complet() {
        let rec = |verdict: &str, diff: Option<&str>| TestRecord {
            group: "unit".to_string(),
            name: "t".to_string(),
            verdict: verdict.to_string(),
            diff: diff.map(str::to_string),
            detail: None,
        };
        assert_eq!(
            verdict_depuis_record(&rec("passed", None)),
            TestVerdict::Passed
        );
        assert_eq!(
            verdict_depuis_record(&rec("crashed", None)),
            TestVerdict::Crashed(-1)
        );
        assert_eq!(
            verdict_depuis_record(&rec("timeout", None)),
            TestVerdict::Timeout
        );
        match verdict_depuis_record(&rec("failed", Some("-a\n+b"))) {
            TestVerdict::Failed { diff, .. } => assert_eq!(diff, "-a\n+b"),
            v => panic!("failed attendu, obtenu {v:?}"),
        }
        // Hors contrat : échec visible, jamais passed.
        assert!(matches!(
            verdict_depuis_record(&rec("bizarre", None)),
            TestVerdict::Failed { .. }
        ));
    }

    #[test]
    fn percent_groupe_vide_est_zero_jamais_nan() {
        let g = GroupState {
            name: "unit".to_string(),
            tests: Vec::new(),
        };
        assert_eq!(g.percent(), 0.0);
        assert!(g.percent().is_finite());
    }
}

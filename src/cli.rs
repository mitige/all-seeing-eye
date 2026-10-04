//! CLI all-seeing-eye (Task 12).
//!
//! Trois modes :
//! - **défaut** : découvre la batterie du cwd ([`Battery::discover`])
//!   ou charge celle de `--battery <nom>`, lance le pipeline dans un
//!   thread et affiche le TUI live (`--no-tui` : lignes texte sur
//!   stdout, pensé CI, code de sortie 0/1) ;
//! - **`list`** : les batteries connues (config + embarquées) ;
//! - **`report`** : rouvre le dernier rapport (TUI de navigation, ou
//!   texte sur stdout avec `--no-tui`).
//!
//! Le thread pipeline est TOUJOURS enrobé de
//! [`std::panic::catch_unwind`] : un panic pipeline ne doit jamais
//! emporter le process — le sender est droppé dans le thread quoi
//! qu'il arrive (retour normal ou unwind), donc le front voit le
//! canal se fermer et affiche « pipeline interrompu » (Task 11) au
//! lieu d'un écran figé.

use crate::battery::model::ProjectType;
use crate::battery::{self, Battery};
use crate::engine::events::{Event, TestVerdict};
use crate::engine::{self, RunOpts};
use crate::exec::{run_capture, Limits};
use crate::norme::official;
use crate::report::{self, Report};
use crate::tui;
use anyhow::{bail, ensure, Context, Result};
use clap::{Parser, Subcommand};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// Ligne de commande `all-seeing-eye`.
#[derive(Debug, Parser)]
#[command(
    name = "all-seeing-eye",
    version,
    about = "All-Seeing Eye — moulinette locale"
)]
pub struct Cli {
    /// Charge la batterie `<nom>` (config puis embarquées) au lieu de
    /// la découvrir dans le dossier courant.
    #[arg(long)]
    pub battery: Option<String>,
    /// Pas de TUI : events et rapport en texte sur stdout (CI) —
    /// acceptée aussi après une sous-commande.
    // `global = true` : `all-seeing-eye report --no-tui` doit marcher.
    #[arg(long, global = true)]
    pub no_tui: bool,
    /// Norme stricte : epiclang requis, étape Norme KO s'il est absent.
    #[arg(long)]
    pub strict_norme: bool,
    #[command(subcommand)]
    pub cmd: Option<Sub>,
}

/// Sous-commandes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Subcommand)]
pub enum Sub {
    /// Liste les batteries connues (config + embarquées).
    List,
    /// Rouvre le dernier rapport (TUI ; texte avec --no-tui).
    Report,
}

impl Cli {
    /// Dispatch et exécution ; renvoie le code de sortie du process
    /// (0/1 — une `Err` vaut 1, cf. `main`).
    pub fn execute(&self) -> Result<i32> {
        match self.cmd {
            Some(Sub::List) => {
                cmd_list();
                Ok(0)
            }
            Some(Sub::Report) => cmd_report(self.no_tui),
            None => cmd_run(self),
        }
    }
}

/// Mode défaut : batterie (découverte dans le cwd ou `--battery`),
/// détection du compilateur, run TUI ou texte.
fn cmd_run(cli: &Cli) -> Result<i32> {
    let target = std::env::current_dir().context("répertoire courant illisible")?;
    let battery = match &cli.battery {
        Some(nom) => batterie_nommee(nom)?,
        None => Battery::discover(&target)
            .map_err(|e| e.context("astuce : `all-seeing-eye list` liste les batteries connues"))?,
    };
    let opts = detect_opts(cli.strict_norme)?;
    if cli.no_tui {
        run_texte(&battery, &target, &opts)
    } else {
        run_tui(&battery, &target, &opts)
    }
}

/// Timeout des sondes compilateur `<bin> --version`, calqué sur la
/// sonde epiclang de la norme (5 s : un binaire pendu ne doit pas
/// figer le démarrage).
const SONDE_COMPILATEUR_TIMEOUT: Duration = Duration::from_secs(5);

/// `true` si `<bin> --version` est lançable : spawn borné à
/// [`SONDE_COMPILATEUR_TIMEOUT`] via [`run_capture`] — le même
/// traitement que la sonde epiclang de la norme.
fn compilateur_dispo(bin: &str) -> bool {
    run_capture(
        Path::new(bin),
        &["--version".to_string()],
        "",
        Path::new("."),
        SONDE_COMPILATEUR_TIMEOUT,
        Limits::default(),
    )
    .is_ok()
}

/// Options du run, détection compilateur incluse : **epiclang → cc →
/// gcc**. epiclang est sondé avec la MÊME sonde que la norme
/// ([`official::epiclang_available`]) ; cc puis gcc avec une sonde
/// bornée identique ([`compilateur_dispo`]). Erreur claire au
/// démarrage si aucun des trois ne répond — inutile de lancer un
/// pipeline dont l'étape Build est condamnée.
fn detect_opts(strict_norme: bool) -> Result<RunOpts> {
    detect_opts_avec(strict_norme, |bin| {
        if bin == "epiclang" {
            official::epiclang_available()
        } else {
            compilateur_dispo(bin)
        }
    })
}

/// Corps de [`detect_opts`], sondes injectées pour les tests :
/// `sonde(bin)` = « `bin --version` est lançable ».
fn detect_opts_avec(strict_norme: bool, sonde: impl Fn(&str) -> bool) -> Result<RunOpts> {
    let use_epiclang = sonde("epiclang");
    let compiler = if use_epiclang {
        PathBuf::from("epiclang")
    } else if sonde("cc") {
        PathBuf::from("cc")
    } else if sonde("gcc") {
        PathBuf::from("gcc")
    } else {
        bail!("aucun compilateur C trouvé (epiclang, cc, gcc) — vérifiez votre PATH");
    };
    Ok(RunOpts {
        strict_norme,
        use_epiclang,
        compiler,
    })
}

/// Lance le pipeline dans un thread et renvoie le receiver des events
/// plus le handle. `catch_unwind` enrobe TOUT le run : le sender vit
/// dans le thread et y est droppé dans tous les cas (retour normal
/// ou unwind) — le front voit donc toujours le canal se fermer, et
/// le process survit à un panic pipeline.
fn spawn_pipeline(
    battery: &Battery,
    target: &Path,
    opts: &RunOpts,
) -> (mpsc::Receiver<Event>, std::thread::JoinHandle<()>) {
    // Canal non borné : le drain TUI est borné par tick (DRAIN_MAX_*)
    // et la mémoire par le volume d'events d'un run (étapes, checks,
    // tests d'une batterie) — jamais un flux infini.
    let (tx, rx) = mpsc::channel::<Event>();
    let battery = battery.clone();
    let target = target.to_path_buf();
    let opts = opts.clone();
    let handle = std::thread::spawn(move || {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            engine::run_pipeline(&battery, &target, tx, &opts);
        }));
    });
    (rx, handle)
}

/// Run avec TUI : le pipeline tourne dans son thread, le TUI consomme
/// les events sur le thread principal (raw mode crossterm). Au `q`
/// post-run, le rapport texte complet part sur stdout.
fn run_tui(battery: &Battery, target: &Path, opts: &RunOpts) -> Result<i32> {
    let started = Instant::now();
    let (rx, handle) = spawn_pipeline(battery, target, opts);
    // PAS de `?` ici : une erreur TUI (init impossible, « q » avant la
    // fin du run) propagerait sans joindre le thread pipeline — il
    // resterait en vol jusqu'à la fin du process et sa salle blanche
    // (TempDir du build) fuirait dans /tmp. [`fin_de_run`] joint donc
    // d'abord, puis propage.
    let resultat = tui::run(rx, battery.project.name.clone(), started);
    let report = fin_de_run(resultat, handle)?;
    print!("{}", report::render_text(&report));
    Ok(exit_code(&report))
}

/// Fin d'un run TUI : joint le thread pipeline QUOI QU'IL ARRIVE —
/// y compris quand le front est parti en erreur — puis propage le
/// résultat du front.
///
/// Le join ne peut pas PENDRE : au retour de `tui::run` le receiver
/// est droppé, et un receiver mort ne bloque ni n'arrête le pipeline
/// (canal non borné, `engine::send` ignore l'erreur) — le thread finit
/// donc toujours sa course, en sourdine. Il peut en revanche ATTENDRE
/// la fin du travail en cours, bornée par les timeouts propres au
/// pipeline (compiles, make, tests) : trade-off assumé — l'alternative
/// (thread détaché, tué à la sortie du process) laisserait fuiter la
/// salle blanche dans /tmp.
fn fin_de_run(resultat: Result<Report>, handle: std::thread::JoinHandle<()>) -> Result<Report> {
    let _ = handle.join();
    resultat
}

/// Run `--no-tui` : le compilateur détecté est logué en en-tête,
/// chaque event devient une ligne texte immédiate (cf.
/// [`affiche_event`]), le rapport complet termine la sortie. Code de
/// sortie CI via [`exit_code`]. Un pipeline mort sans verdict (panic)
/// est une erreur propre, pas un abort du process.
fn run_texte(battery: &Battery, target: &Path, opts: &RunOpts) -> Result<i32> {
    println!(
        "all-seeing-eye — batterie « {} » — compilateur : {}",
        battery.project.name,
        opts.compiler.display()
    );
    let (rx, handle) = spawn_pipeline(battery, target, opts);
    let mut verdict: Option<Report> = None;
    // Itérateur bloquant : se termine quand le sender (dans le thread
    // pipeline) est droppé — run fini OU panic.
    for ev in &rx {
        match ev {
            Event::RunFinished { report } => verdict = Some(*report),
            _ => affiche_event(&ev),
        }
    }
    let _ = handle.join();
    let Some(report) = verdict else {
        bail!("pipeline interrompu avant le verdict (panic ?) — aucun rapport");
    };
    print!("{}", report::render_text(&report));
    Ok(exit_code(&report))
}

/// Une ligne texte par event (mode `--no-tui`, pensé CI) :
/// `[etape] message` pour la progression, `✓/✗/💥/⏱ nom` pour les
/// tests, `file:line:col severity message (rule)` pour les fautes de
/// norme. `TestStarted` et `RunFinished` n'impriment rien : le verdict
/// du test suffit, et le rapport complet suit.
fn affiche_event(ev: &Event) {
    if let Some(ligne) = ligne_event(ev) {
        println!("{ligne}");
    }
}

/// Formate un event en sa ligne texte `--no-tui` (None = rien à
/// imprimer) — corps pur de [`affiche_event`], testable.
fn ligne_event(ev: &Event) -> Option<String> {
    Some(match ev {
        Event::StepStarted { step, label } => format!("[{}] {label}", step.name()),
        Event::LogLine { step, line } => format!("[{}] {line}", step.name()),
        Event::CheckFinished {
            step,
            name,
            ok,
            detail,
        } => {
            let statut = if *ok { "ok" } else { "KO" };
            if detail.is_empty() {
                format!("[{}] {statut} {name}", step.name())
            } else {
                format!("[{}] {statut} {name} — {detail}", step.name())
            }
        }
        Event::TestStarted { .. } => return None,
        Event::TestFinished { name, result, .. } => {
            let icone = match result {
                TestVerdict::Passed => "✓",
                TestVerdict::Failed { .. } => "✗",
                TestVerdict::Crashed(_) => "💥",
                TestVerdict::Timeout => "⏱",
            };
            format!("  {icone} {name}")
        }
        Event::NormeFault(f) => format!(
            "{}:{}:{}: {} {} ({})",
            f.file.display(),
            f.line,
            f.col,
            f.severity.label(),
            f.message,
            f.rule
        ),
        Event::StepFinished {
            step,
            ok,
            skipped,
            summary,
        } => {
            // Un summary « skipped: … » porte déjà le motif (engine
            // step_skipped/step_disabled) : ne pas préfixer une
            // seconde fois.
            if *skipped && summary.starts_with("skipped:") {
                format!("[{}] {summary}", step.name())
            } else {
                let statut = if *skipped {
                    "skipped"
                } else if *ok {
                    "OK"
                } else {
                    "KO"
                };
                format!("[{}] {statut} — {summary}", step.name())
            }
        }
        Event::RunFinished { .. } => return None,
    })
}

/// Code de sortie CI : 0 si score global 100.0 ET toutes les étapes
/// OK, 1 sinon. (Les étapes désactivées — symbols sans whitelist,
/// unit sans `tests_run_rule` — ont `ok: true` et ne bloquent pas.)
fn exit_code(report: &Report) -> i32 {
    if report.scores.global == 100.0 && report.steps.iter().all(|s| s.ok) {
        0
    } else {
        1
    }
}

/// `--battery <nom>` : `~/.config/all-seeing-eye/batteries/<nom>.toml`
/// d'abord, les embarquées ensuite ([`battery::load_embedded`] :
/// extraction dans un TempDir détenu par la batterie — le `root`
/// doit exister en vrai pour les harness/expected). Introuvable →
/// erreur listant les batteries connues.
fn batterie_nommee(nom: &str) -> Result<Battery> {
    // Un NOM, pas un chemin : on ne lit jamais ailleurs que dans le
    // dossier batteries de la config.
    ensure!(
        !nom.is_empty() && nom != "." && nom != ".." && !nom.contains(['/', '\\']),
        "nom de batterie invalide : « {nom} » (un nom de fichier, pas un chemin)"
    );
    if let Some(config) = dirs::config_dir() {
        let path = config
            .join("all-seeing-eye")
            .join("batteries")
            .join(format!("{nom}.toml"));
        if path.is_file() {
            return Battery::load(&path);
        }
    }
    // Embarquées : extraction sur disque + chargement (le TempDir
    // d'extraction est détenu par la batterie retournée).
    if let Some(b) = battery::load_embedded(nom)? {
        return Ok(b);
    }
    let connues = batteries_connues();
    let mut detail = String::from("batteries connues :");
    if connues.is_empty() {
        detail.push_str(" aucune (ni config, ni embarquée)");
    } else {
        for b in &connues {
            detail.push_str(&format!("\n  - {}", b.ligne()));
        }
    }
    bail!("batterie « {nom} » introuvable — {detail}\n(astuce : `all-seeing-eye list`)")
}

/// Une batterie connue, pour `list` et les messages d'erreur.
struct BatterieConnue {
    nom: String,
    /// « binary » | « functions » ; « ? » si le TOML est illisible.
    kind: String,
    /// Résumé court (comptes de tâches/tests) ou l'erreur de lecture.
    description: String,
    /// Chemin du TOML, ou « embarquée ».
    origine: String,
}

impl BatterieConnue {
    /// Ligne d'affichage : `nom ⇥ type ⇥ description ⇥ (origine)`.
    fn ligne(&self) -> String {
        format!(
            "{}\t{}\t{}\t({})",
            self.nom, self.kind, self.description, self.origine
        )
    }

    /// Entrée d'une batterie chargée.
    fn de_battery(b: &Battery, origine: String) -> Self {
        let kind = match b.project.kind {
            ProjectType::Binary => "binary",
            ProjectType::Functions => "functions",
        };
        Self {
            nom: b.project.name.clone(),
            kind: kind.to_string(),
            description: description_courte(b),
            origine,
        }
    }
}

/// Description d'une batterie illisible, pour `list` : l'erreur est
/// aplatie sur UNE ligne — une erreur TOML est multi-lignes (contexte
/// ligne/colonne) et `list` imprime une ligne par batterie ; « ⏎ »
/// marque les sauts aplatis.
fn description_illisible(message: String) -> String {
    format!("illisible : {}", message.replace('\n', " ⏎ "))
}

/// Toutes les batteries connues :
/// `~/.config/all-seeing-eye/batteries/*.toml` (triées par chemin ;
/// les illisibles restent listées avec leur erreur — jamais
/// silencieusement omises), puis les embarquées
/// ([`battery::embedded_batteries`]).
fn batteries_connues() -> Vec<BatterieConnue> {
    let mut out = Vec::new();
    if let Some(config) = dirs::config_dir() {
        let dir = config.join("all-seeing-eye").join("batteries");
        let mut paths: Vec<PathBuf> = fs::read_dir(&dir)
            .map(|rd| {
                rd.filter_map(|e| e.ok().map(|e| e.path()))
                    .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("toml"))
                    .collect()
            })
            .unwrap_or_default();
        paths.sort();
        for path in paths {
            let entree = match Battery::load(&path) {
                Ok(b) => BatterieConnue::de_battery(&b, path.display().to_string()),
                Err(e) => BatterieConnue {
                    nom: path
                        .file_stem()
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_else(|| path.display().to_string()),
                    kind: "?".to_string(),
                    description: description_illisible(format!("{e:#}")),
                    origine: path.display().to_string(),
                },
            };
            out.push(entree);
        }
    }
    for (nom, contenu) in battery::embedded_batteries() {
        let entree = match toml::from_str::<Battery>(contenu) {
            Ok(b) => BatterieConnue::de_battery(&b, "embarquée".to_string()),
            Err(e) => BatterieConnue {
                nom: nom.to_string(),
                kind: "?".to_string(),
                description: description_illisible(e.to_string()),
                origine: "embarquée".to_string(),
            },
        };
        out.push(entree);
    }
    out
}

/// Résumé court d'une batterie : compte de tâches (functions) ou de
/// tests fonctionnels (binary), règle `tests_run` signalée.
fn description_courte(b: &Battery) -> String {
    let mut d = match b.project.kind {
        ProjectType::Functions => format!("{} tâche{}", b.task.len(), pluriel(b.task.len())),
        ProjectType::Binary => format!(
            "{} test{} fonctionnel{}",
            b.functional_test.len(),
            pluriel(b.functional_test.len()),
            pluriel(b.functional_test.len())
        ),
    };
    if b.project.tests_run_rule {
        d.push_str(" + tests_run");
    }
    d
}

/// « s » au pluriel (au-delà de 1), rien au singulier.
fn pluriel(n: usize) -> &'static str {
    if n > 1 {
        "s"
    } else {
        ""
    }
}

/// `all-seeing-eye list` : une ligne par batterie connue.
fn cmd_list() {
    let connues = batteries_connues();
    if connues.is_empty() {
        println!(
            "aucune batterie connue — déposez des TOML dans \
             ~/.config/all-seeing-eye/batteries/"
        );
        return;
    }
    for b in &connues {
        println!("{}", b.ligne());
    }
}

/// `all-seeing-eye report` : recharge le dernier rapport
/// (`<data_dir>/all-seeing-eye/last.json`) — TUI de navigation, ou
/// texte sur stdout avec `--no-tui`. Erreurs claires si absent ou
/// corrompu.
fn cmd_report(no_tui: bool) -> Result<i32> {
    let path = dirs::data_dir()
        .context("aucun data dir (ni XDG_DATA_HOME ni HOME)")?
        .join("all-seeing-eye")
        .join("last.json");
    let contenu = fs::read_to_string(&path).with_context(|| {
        format!(
            "rapport introuvable : {} — lancez d'abord `all-seeing-eye`",
            path.display()
        )
    })?;
    let report: Report = serde_json::from_str(&contenu)
        .with_context(|| format!("rapport corrompu : {}", path.display()))?;
    if no_tui {
        print!("{}", report::render_text(&report));
    } else {
        tui::browse(report)?;
    }
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::events::Step;
    use crate::report::Scores;
    use crate::report::StepReport;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    /// Rapport minimal au score global et à l'état d'étape voulus.
    fn rapport(global: f64, etapes_ok: bool) -> Report {
        Report {
            project: "x".to_string(),
            steps: vec![StepReport {
                step: "prelim".to_string(),
                ok: etapes_ok,
                skipped: false,
                summary: String::new(),
                checks: Vec::new(),
            }],
            tests: Vec::new(),
            norme: Vec::new(),
            scores: Scores {
                global,
                ..Scores::default()
            },
            duration_secs: 0.0,
        }
    }

    #[test]
    fn exit_code_0_seulement_si_100_et_toutes_etapes_ok() {
        assert_eq!(exit_code(&rapport(100.0, true)), 0);
        assert_eq!(exit_code(&rapport(99.9, true)), 1);
        assert_eq!(exit_code(&rapport(100.0, false)), 1);
        assert_eq!(exit_code(&rapport(0.0, true)), 1);
    }

    #[test]
    fn description_courte_compte_taches_tests_et_tests_run() {
        let functions: Battery = toml::from_str(
            "[project]\nname = \"f\"\ntype = \"functions\"\n\n\
             [[task]]\nname = \"t\"\ndelivery = \"a.c\"\nharness = \"h.c\"\nstdout = \"\"\n",
        )
        .unwrap();
        assert_eq!(description_courte(&functions), "1 tâche");
        let binary: Battery = toml::from_str(
            "[project]\nname = \"b\"\ntype = \"binary\"\nbinary = \"b\"\n\
             tests_run_rule = true\n\n\
             [[functional_test]]\nname = \"t1\"\nstdout = \"\"\n\n\
             [[functional_test]]\nname = \"t2\"\nstdout = \"\"\n",
        )
        .unwrap();
        assert_eq!(
            description_courte(&binary),
            "2 tests fonctionnels + tests_run"
        );
    }

    #[test]
    fn batterie_nommee_rejette_les_chemins() {
        // Rejet AVANT tout accès disque : aucune de ces valeurs ne
        // doit lire hors du dossier batteries de la config.
        for nom in ["", ".", "..", "a/b", "a\\b"] {
            assert!(batterie_nommee(nom).is_err(), "accepté : « {nom} »");
        }
    }

    #[test]
    fn detection_compilateur_chaine_epiclang_puis_cc_puis_gcc() {
        // epiclang présent → choisi, cc/gcc jamais consultés.
        let o = detect_opts_avec(false, |bin| bin == "epiclang").unwrap();
        assert_eq!(o.compiler, PathBuf::from("epiclang"));
        assert!(o.use_epiclang);
        // epiclang absent, cc présent → cc.
        let o = detect_opts_avec(false, |bin| bin == "cc").unwrap();
        assert_eq!(o.compiler, PathBuf::from("cc"));
        assert!(!o.use_epiclang);
        // epiclang et cc absents → gcc ; le reste des options suit.
        let o = detect_opts_avec(true, |bin| bin == "gcc").unwrap();
        assert_eq!(o.compiler, PathBuf::from("gcc"));
        assert!(o.strict_norme);
        // Aucun des trois → erreur claire qui les nomme tous.
        let e = detect_opts_avec(false, |_| false).unwrap_err();
        let msg = format!("{e:#}");
        assert!(
            msg.contains("aucun compilateur C trouvé")
                && msg.contains("epiclang")
                && msg.contains("cc")
                && msg.contains("gcc"),
            "message peu clair : {msg}"
        );
    }

    #[test]
    fn fin_de_run_joint_le_thread_meme_si_le_tui_a_echoue() {
        // Receiver mort d'emblée (le front est parti en erreur, ou « q »
        // avant la fin du run) : les sends échouent instantanément sans
        // bloquer (canal non borné, engine::send ignore l'erreur), le
        // thread finit donc sa course — et fin_de_run le joint AVANT de
        // propager l'erreur : jamais de thread pipeline laissé en vol.
        let (tx, rx) = mpsc::channel::<Event>();
        drop(rx);
        let fini = Arc::new(AtomicBool::new(false));
        let fini_thread = Arc::clone(&fini);
        let handle = std::thread::spawn(move || {
            for i in 0..10_000 {
                let _ = tx.send(Event::LogLine {
                    step: Step::Prelim,
                    line: format!("ligne {i}"),
                });
            }
            fini_thread.store(true, Ordering::SeqCst);
        });
        let debut = Instant::now();
        let res = fin_de_run(Err(anyhow::anyhow!("tui KO")), handle);
        assert!(res.is_err(), "l'erreur du front doit être propagée");
        assert!(
            fini.load(Ordering::SeqCst),
            "le thread pipeline doit avoir été joint (course finie)"
        );
        assert!(
            debut.elapsed() < Duration::from_secs(5),
            "le join ne doit pas attendre ({:?})",
            debut.elapsed()
        );
    }

    #[test]
    fn fin_de_run_propage_le_rapport_sur_succes() {
        let handle = std::thread::spawn(|| {});
        let report = fin_de_run(Ok(rapport(100.0, true)), handle).unwrap();
        assert_eq!(report.scores.global, 100.0);
    }

    #[test]
    fn ligne_event_skipped_ne_double_pas_le_prefixe() {
        // Les summaries d'étapes skipped portent déjà « skipped: <raison> »
        // (engine::step_skipped / step_disabled) : ne pas répéter le mot.
        let ev = Event::StepFinished {
            step: Step::Symbols,
            ok: true,
            skipped: true,
            summary: "skipped: allowed_functions vide".to_string(),
        };
        assert_eq!(
            ligne_event(&ev).as_deref(),
            Some("[symbols] skipped: allowed_functions vide")
        );
    }

    #[test]
    fn ligne_event_statuts_et_silences() {
        let fin = |ok: bool, skipped: bool, summary: &str| Event::StepFinished {
            step: Step::Build,
            ok,
            skipped,
            summary: summary.to_string(),
        };
        assert_eq!(
            ligne_event(&fin(true, false, "build ok")).as_deref(),
            Some("[build] OK — build ok")
        );
        assert_eq!(
            ligne_event(&fin(false, false, "boom")).as_deref(),
            Some("[build] KO — boom")
        );
        // Skipped SANS préfixe dans le summary (défensif) : préfixe normal.
        assert_eq!(
            ligne_event(&fin(false, true, "dépendance KO")).as_deref(),
            Some("[build] skipped — dépendance KO")
        );
        // TestStarted et RunFinished n'impriment rien.
        assert!(ligne_event(&Event::TestStarted {
            group: "unit".to_string(),
            name: "t".to_string(),
        })
        .is_none());
        assert!(ligne_event(&Event::RunFinished {
            report: Box::new(rapport(100.0, true)),
        })
        .is_none());
    }

    #[test]
    fn description_illisible_aplatit_les_erreurs_toml_multilignes() {
        // Une vraie erreur toml est multi-lignes (contexte ligne/colonne) ;
        // `list` imprime UNE ligne par batterie.
        let e = toml::from_str::<Battery>("[[[casse").unwrap_err();
        let d = description_illisible(e.to_string());
        assert!(d.starts_with("illisible : "), "{d}");
        assert!(!d.contains('\n'), "erreur non aplatie : {d:?}");
        assert!(d.contains('⏎'), "sauts aplatis marqués : {d}");
    }
}

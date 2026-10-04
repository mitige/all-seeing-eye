//! Rapport final d'un run de la moulinette : collecte, scores,
//! rendu texte et export JSON/TXT (Task 10).
//!
//! Scoring (règle « vraie moulinette ») : par groupe de tests attendu,
//! `passed/total` en % arrondi au dixième ; un groupe attendu VIDE
//! (étape skipped, build KO…) compte **0 %** — jamais 100 % vacuoleux
//! ni NaN. Le score global est la moyenne SIMPLE des groupes (non
//! pondérée par le nombre de tests — choix documenté Task 10), 0.0
//! s'il n'y a aucun groupe.
//!
//! Rendu texte style intranet : étapes aux labels FR
//! ([`Step::label`]), lignes de tests non-passed indentées avec icône
//! (✗ failed, 💥 crashed, ⏱ timeout), score global au dixième.

use crate::engine::events::Step;
use crate::norme::{NormeFault, Severity};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fmt::Write as _;
use std::fs;
use std::time::SystemTime;

/// Résultat collecté d'un test unitaire ou fonctionnel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestRecord {
    pub group: String,
    pub name: String,
    /// "passed" | "failed" | "crashed" | "timeout"
    pub verdict: String,
    pub diff: Option<String>,
    /// Détail court pour le rendu (« SIGSEGV », « délai dépassé ») ;
    /// None pour passed/failed (le diff porte tout).
    /// `serde(default)` : compatible avec les rapports écrits avant
    /// ce champ.
    #[serde(default)]
    pub detail: Option<String>,
}

/// Bilan d'une étape du pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepReport {
    pub step: String,
    pub ok: bool,
    /// Étape sautée (dépendance échouée). `serde(default)` pour rester
    /// compatible avec les rapports écrits avant ce champ.
    #[serde(default)]
    pub skipped: bool,
    pub summary: String,
    /// (nom du check, ok, détail)
    pub checks: Vec<(String, bool, String)>,
}

/// Scores du run : par groupe de tests (%, arrondi au dixième),
/// global (moyenne simple des groupes), fautes de norme par sévérité.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Scores {
    pub par_groupe: Vec<(String, f64)>,
    pub global: f64,
    pub norme_fatal: u32,
    pub norme_major: u32,
    pub norme_minor: u32,
    pub norme_info: u32,
}

/// Rapport complet d'un run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub project: String,
    pub steps: Vec<StepReport>,
    pub tests: Vec<TestRecord>,
    pub norme: Vec<NormeFault>,
    /// `serde(default)` : compatible avec les rapports écrits avant
    /// ce champ (Task 10).
    #[serde(default)]
    pub scores: Scores,
    pub duration_secs: f64,
}

/// Calcule les scores : par groupe attendu (0.0 si vide — jamais de
/// 100 % vacuoleux), moyenne SIMPLE des groupes en global (0.0 s'il
/// n'y a aucun groupe), décompte des fautes de norme par sévérité.
///
/// Les groupes listés sont l'union des `groupes_attendus` (ceux que la
/// batterie aurait produits — décidés par le verdict, qui connaît la
/// batterie) et des groupes effectivement observés dans les
/// enregistrements (défensif : un résultat réel ne disparaît jamais
/// du score). Les deux coïncident en pipeline réel. Seul le verdict
/// « passed » compte comme réussi ; failed, crashed et timeout pèsent
/// sur le total. Arrondi au dixième partout (33.333… → 33.3).
pub fn compute_scores(
    tests: &[TestRecord],
    groupes_attendus: &[&str],
    norme: &[NormeFault],
) -> Scores {
    let mut groupes: Vec<&str> = Vec::new();
    for g in groupes_attendus {
        if !groupes.contains(g) {
            groupes.push(g);
        }
    }
    for t in tests {
        if !groupes.contains(&t.group.as_str()) {
            groupes.push(t.group.as_str());
        }
    }
    let par_groupe: Vec<(String, f64)> = groupes
        .iter()
        .map(|g| {
            let total = tests.iter().filter(|t| t.group == *g).count();
            let passed = tests
                .iter()
                .filter(|t| t.group == *g && t.verdict == "passed")
                .count();
            // Groupe attendu vide : 0.0 — jamais 100 % vacuoleux ni
            // NaN (division 0/0). Build KO = tests à zéro.
            let score = if total == 0 {
                0.0
            } else {
                arrondi_dixieme(100.0 * passed as f64 / total as f64)
            };
            ((*g).to_string(), score)
        })
        .collect();
    let global = if par_groupe.is_empty() {
        0.0
    } else {
        arrondi_dixieme(par_groupe.iter().map(|(_, s)| s).sum::<f64>() / par_groupe.len() as f64)
    };
    let mut scores = Scores {
        par_groupe,
        global,
        ..Scores::default()
    };
    for f in norme {
        match f.severity {
            Severity::Fatal => scores.norme_fatal += 1,
            Severity::Major => scores.norme_major += 1,
            Severity::Minor => scores.norme_minor += 1,
            Severity::Info => scores.norme_info += 1,
        }
    }
    scores
}

/// Arrondi au dixième (66.666… → 66.7 ; arrondi, pas troncature).
fn arrondi_dixieme(x: f64) -> f64 {
    (x * 10.0).round() / 10.0
}

/// Largeur du champ « label + espace + points » d'une ligne d'étape.
const CHAMP_ETAPE: usize = 31;
/// Largeur du champ « nom + espace + points » d'une ligne de test.
const CHAMP_TEST: usize = 18;
/// Points de calage minimum, même pour les labels dépassant le champ.
const POINTS_MIN: usize = 3;
/// Longueur max du détail d'un test (première ligne de diff tronquée).
const DETAIL_MAX: usize = 40;

/// Écrit le rapport : JSON pretty dans
/// `dirs::data_dir()/seeyou/last.json`, rendu texte dans `last.txt`.
/// Crée le dossier au besoin.
pub fn save(report: &Report) -> Result<()> {
    let dir = dirs::data_dir()
        .context("aucun data dir (ni XDG_DATA_HOME ni HOME)")?
        .join("seeyou");
    fs::create_dir_all(&dir)
        .with_context(|| format!("création de {} impossible", dir.display()))?;
    let json = serde_json::to_string_pretty(report).context("sérialisation JSON du rapport")?;
    let json_path = dir.join("last.json");
    fs::write(&json_path, format!("{json}\n"))
        .with_context(|| format!("écriture de {} impossible", json_path.display()))?;
    let txt_path = dir.join("last.txt");
    fs::write(&txt_path, render_text(report))
        .with_context(|| format!("écriture de {} impossible", txt_path.display()))?;
    Ok(())
}

/// Rendu texte du rapport, horodaté à l'heure courante.
pub fn render_text(report: &Report) -> String {
    render_text_at(report, SystemTime::now())
}

/// Cœur déterministe du rendu : l'horodatage de l'en-tête est injecté
/// (golden tests). [`render_text`] l'appelle avec l'heure courante.
pub fn render_text_at(report: &Report, at: SystemTime) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "═══ SEEYOU ═══ {} ═══ {} ═══",
        report.project,
        horodatage(at)
    );
    for sr in &report.steps {
        let step = Step::from_name(&sr.step);
        // Le verdict n'est pas affiché : il EST ce rapport.
        if step == Some(Step::Verdict) {
            continue;
        }
        let label = step.map_or(sr.step.as_str(), |s| s.label());
        let _ = writeln!(
            out,
            "▸ {label} {} {}",
            points(label, CHAMP_ETAPE),
            statut_etape(sr, report)
        );
        // Tests non-passed du groupe de cette étape, indentés.
        for t in report
            .tests
            .iter()
            .filter(|t| t.verdict != "passed" && t.group == sr.step)
        {
            let _ = writeln!(out, "{}", ligne_test(t));
        }
    }
    let _ = writeln!(out, "SCORE GLOBAL : {:.1}%", report.scores.global);
    out
}

/// « OK » / « KO » / « skipped » selon l'étape — avec deux formes
/// spéciales : la norme affiche ses comptes par sévérité (celles > 0,
/// ou « clean »), les étapes de tests ayant produit des
/// enregistrements affichent « passed/total (score %) ».
fn statut_etape(sr: &StepReport, report: &Report) -> String {
    if sr.skipped {
        return "skipped".to_string();
    }
    match Step::from_name(&sr.step) {
        Some(Step::Norme) if sr.ok => comptes_norme(&report.norme),
        Some(Step::Unit | Step::Functional) => match score_groupe(report, &sr.step) {
            Some((passed, total, score)) => format!("{passed}/{total} ({score:.1}%)"),
            None => ok_ko(sr.ok),
        },
        _ => ok_ko(sr.ok),
    }
}

/// « OK » si l'étape a réussi, « KO » sinon.
fn ok_ko(ok: bool) -> String {
    if ok { "OK" } else { "KO" }.to_string()
}

/// « 3 major, 12 minor » — sévérités non nulles dans l'ordre
/// décroissant de gravité ; « clean » si aucune faute. Compté depuis
/// `report.norme` (la source), JAMAIS depuis `scores.norme_*` : un
/// rapport ancien re-rendu peut n'avoir pas de scores (champ ajouté
/// plus tard, `serde(default)`) alors que ses fautes sont là.
fn comptes_norme(norme: &[NormeFault]) -> String {
    // fatal, major, minor, info — ordre décroissant de gravité.
    let mut comptes = [0u32; 4];
    for f in norme {
        match f.severity {
            Severity::Fatal => comptes[0] += 1,
            Severity::Major => comptes[1] += 1,
            Severity::Minor => comptes[2] += 1,
            Severity::Info => comptes[3] += 1,
        }
    }
    let parties = [
        (comptes[0], "fatal"),
        (comptes[1], "major"),
        (comptes[2], "minor"),
        (comptes[3], "info"),
    ]
    .into_iter()
    .filter(|(n, _)| *n > 0)
    .map(|(n, label)| format!("{n} {label}"))
    .collect::<Vec<_>>();
    if parties.is_empty() {
        "clean".to_string()
    } else {
        parties.join(", ")
    }
}

/// (passed, total, score arrondi) d'un groupe de tests, ou None si le
/// groupe n'a aucun enregistrement. Le score affiché est celui du
/// rapport (déjà arrondi) ; recalculé en repli défensif.
fn score_groupe(report: &Report, groupe: &str) -> Option<(usize, usize, f64)> {
    let total = report.tests.iter().filter(|t| t.group == groupe).count();
    if total == 0 {
        return None;
    }
    let passed = report
        .tests
        .iter()
        .filter(|t| t.group == groupe && t.verdict == "passed")
        .count();
    let score = report
        .scores
        .par_groupe
        .iter()
        .find(|(g, _)| g == groupe)
        .map(|(_, s)| *s)
        .unwrap_or_else(|| arrondi_dixieme(100.0 * passed as f64 / total as f64));
    Some((passed, total, score))
}

/// Ligne indentée d'un test non-passed :
/// « ␣␣␣␣✗ basic_ls … FAILED (première ligne du diff) ».
fn ligne_test(t: &TestRecord) -> String {
    let icone = match t.verdict.as_str() {
        "failed" => "✗",
        "crashed" => "💥",
        "timeout" => "⏱",
        // Verdict inconnu : visible, jamais silencieusement omis.
        _ => "?",
    };
    format!(
        "    {icone} {} {} {} ({})",
        t.name,
        points(&t.name, CHAMP_TEST),
        t.verdict.to_uppercase(),
        detail_test(t),
    )
}

/// Détail court d'un test non-passed : le champ `detail` s'il est
/// porté (crash, timeout), sinon la première ligne du diff tronquée
/// à [`DETAIL_MAX`] caractères, sinon « écart de sortie ».
fn detail_test(t: &TestRecord) -> String {
    if let Some(detail) = &t.detail {
        return detail.clone();
    }
    match t.diff.as_deref().and_then(|d| d.lines().next()) {
        Some(premiere) if !premiere.is_empty() => {
            let mut court: String = premiere.chars().take(DETAIL_MAX).collect();
            if premiere.chars().count() > DETAIL_MAX {
                court.push('…');
            }
            court
        }
        _ => "écart de sortie".to_string(),
    }
}

/// Points de calage : `contenu` + espace + points = `champ`
/// caractères, avec [`POINTS_MIN`] points minimum (un contenu plus
/// large que le champ décale le statut, jamais de chaîne vide de
/// points). Compte en caractères, pas en octets (labels accentués).
fn points(contenu: &str, champ: usize) -> String {
    let n = champ
        .saturating_sub(contenu.chars().count() + 1)
        .max(POINTS_MIN);
    ".".repeat(n)
}

/// « YYYY-MM-DD HH:MM » en UTC, conversion civil-from-days (algorithme
/// de Howard Hinnant, domaine public) — aucune dépendance chrono, pas
/// d'unsafe. Un instant antérieur à l'epoch est rabattu sur
/// 1970-01-01 00:00 (défensif : une horloge cassée ne panique pas le
/// rapport).
fn horodatage(at: SystemTime) -> String {
    let secs = at
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let jours = secs.div_euclid(86_400);
    let sod = secs.rem_euclid(86_400);
    let (hh, mm) = (sod / 3600, (sod % 3600) / 60);
    let z = jours + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02} {hh:02}:{mm:02}")
}

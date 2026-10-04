//! Tests de l'étape Norme (Task 6) : chemin officiel (epiclang +
//! plugin banana), fallback interne, mode strict, et remontée des
//! fautes C-O1/C-O4 de l'étape Prelim dans `ctx.norme_faults`.

mod common;

use seeyou::battery::Battery;
use seeyou::engine::events::{Event, Step};
use seeyou::engine::{norme, prelim};
use seeyou::norme::Severity;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Mutex};
use tempfile::TempDir;

/// Batterie Binary factice (comme tests/prelim.rs).
const BINARY_TOML: &str = r#"
[project]
name = "mini_binary"
type = "binary"
binary = "hello"
"#;

/// Écrit `toml` dans un tempdir et charge la batterie.
fn load_battery(toml: &str) -> (TempDir, Battery) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("moulinette.toml");
    fs::write(&path, toml).unwrap();
    (dir, Battery::load(&path).unwrap())
}

/// Chemin d'une fixture versionnée.
fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

/// `PATH` est process-global : tous les tests qui le vident ou qui
/// lancent des sous-processus via le PATH (epiclang, banana-check-repo)
/// se sérialisent sur ce mutex — un PATH vidé par un test ne doit
/// jamais être visible par un autre.
static ENV_MUTEX: Mutex<()> = Mutex::new(());

/// Positionne `PATH` et le restaure à sa valeur initiale au drop.
struct PathGuard(Option<std::ffi::OsString>);

impl PathGuard {
    fn set(path: &Path) -> Self {
        let ancien = std::env::var_os("PATH");
        std::env::set_var("PATH", path);
        Self(ancien)
    }
}

impl Drop for PathGuard {
    fn drop(&mut self) {
        match &self.0 {
            Some(v) => std::env::set_var("PATH", v),
            None => std::env::remove_var("PATH"),
        }
    }
}

/// `true` si epiclang est lançable (test conditionnel).
fn epiclang_present() -> bool {
    std::process::Command::new("epiclang")
        .arg("--version")
        .output()
        .is_ok()
}

/// `true` si banana-check-repo est lançable (test conditionnel).
fn banana_present() -> bool {
    std::process::Command::new("banana-check-repo")
        .arg("--version")
        .output()
        .is_ok()
}

// ----------------------------------------------------------------
// Trou connu colmaté : les checks Prelim atterrissent dans le
// StepReport (Collect les transmet à step_finished).
// ----------------------------------------------------------------

#[test]
fn prelim_checks_atterrissent_dans_step_report() {
    let _lock = ENV_MUTEX.lock().unwrap_or_else(|p| p.into_inner()); // prelim lance des sous-processus
    let (_bat_dir, battery) = load_battery(BINARY_TOML);
    let mut ctx = common::test_ctx(&battery, &fixture("dirty_repo"));
    let (tx, _rx) = mpsc::channel();

    let ok = prelim::run(&mut ctx, &tx);
    assert!(!ok, "dirty_repo doit faire échouer prelim");

    let prelim_report = ctx
        .steps
        .iter()
        .find(|s| s.step == "prelim")
        .expect("StepReport prelim manquant");
    assert!(
        !prelim_report.checks.is_empty(),
        "StepReport.checks vide pour prelim : {prelim_report:?}"
    );
    assert!(
        prelim_report
            .checks
            .iter()
            .any(|(name, ok, detail)| name == "forbidden files"
                && !*ok
                && detail.contains("(C-O1)")),
        "check C-O1 absent du StepReport : {:?}",
        prelim_report.checks
    );
}

// ----------------------------------------------------------------
// Étape Norme — chemin officiel (epiclang + banana).
// ----------------------------------------------------------------

/// Exécute l'étape norme isolée et collecte les événements.
fn run_norme(ctx: &mut seeyou::engine::PipelineContext) -> (bool, Vec<Event>) {
    let (tx, rx) = mpsc::channel();
    let ok = norme::run(ctx, &tx);
    drop(tx);
    let events: Vec<Event> = rx.iter().collect();
    (ok, events)
}

/// Extrait les fautes norme du flux d'événements.
fn norme_faults_events(events: &[Event]) -> Vec<&seeyou::norme::NormeFault> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::NormeFault(f) => Some(f),
            _ => None,
        })
        .collect()
}

/// Le StepFinished de l'étape norme.
fn step_finished(events: &[Event]) -> Option<(bool, String)> {
    events.iter().find_map(|e| match e {
        Event::StepFinished {
            step: Step::Norme,
            ok,
            summary,
            ..
        } => Some((*ok, summary.clone())),
        _ => None,
    })
}

#[test]
fn officiel_parse_dirty_c() {
    let _lock = ENV_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
    if !epiclang_present() {
        eprintln!("epiclang absent : test skippé");
        return;
    }
    let (_bat_dir, battery) = load_battery(BINARY_TOML);
    let mut ctx = common::test_ctx(&battery, &fixture("norme_ko"));

    let (ok, events) = run_norme(&mut ctx);

    // L'étape réussit MALGRÉ les fautes Major : la sévérité est du
    // ressort du verdict (Task 10), l'étape n'échoue que si elle ne
    // peut pas s'exécuter.
    assert!(ok, "l'étape doit retourner true même avec des fautes");
    let f = &ctx.norme_faults;
    let attend = [
        ("C-G1", Severity::Minor, 1, 1),
        ("C-F5", Severity::Major, 1, 40),
        ("C-F8", Severity::Minor, 3, 5),
        ("C-L1", Severity::Major, 4, 13),
        ("C-G7", Severity::Minor, 4, 21),
        ("C-C3", Severity::Major, 5, 5),
    ];
    for (rule, sev, line, col) in attend {
        assert!(
            f.iter().any(|x| x.rule == rule
                && x.severity == sev
                && x.file == PathBuf::from("dirty.c")
                && x.line == line
                && x.col == col),
            "{rule} ({line}:{col}) absent de {f:?}"
        );
    }
    // Chaque faute a été émise en event NormeFault.
    assert_eq!(norme_faults_events(&events).len(), f.len());
    // Résumé : sévérités > 0 seulement — ici 3 major (F5, L1, C3),
    // 4 minor (G1, G7, F8 + le C-L2 du label `end:`).
    let (fin_ok, summary) = step_finished(&events).expect("StepFinished norme manquant");
    assert!(fin_ok);
    assert_eq!(summary, "3 major, 4 minor", "summary : {summary}");
}

// ----------------------------------------------------------------
// Moteur interne sur la même fixture.
// ----------------------------------------------------------------

#[test]
fn interne_meme_fixture_sous_ensemble() {
    use seeyou::norme::internal::{check_file, SourceKind};
    let content = fs::read_to_string(fixture("norme_ko").join("dirty.c")).unwrap();
    let f = check_file(Path::new("dirty.c"), &content, SourceKind::C);
    let attend = [
        ("C-G1", Severity::Minor, 1, 1),
        ("C-F5", Severity::Major, 1, 40),
        ("C-F8", Severity::Minor, 3, 5),
        ("C-L1", Severity::Major, 4, 13),
        ("C-G7", Severity::Minor, 4, 21),
        ("C-C3", Severity::Major, 5, 5),
    ];
    for (rule, sev, line, col) in attend {
        assert!(
            f.iter().any(|x| x.rule == rule
                && x.severity == sev
                && x.file == PathBuf::from("dirty.c")
                && x.line == line
                && x.col == col),
            "{rule} ({line}:{col}) absent du moteur interne : {f:?}"
        );
    }
}

// ----------------------------------------------------------------
// Sélection du moteur : strict et fallback.
// ----------------------------------------------------------------

#[test]
fn fallback_interne_quand_epiclang_absent() {
    let _lock = ENV_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
    let vide = TempDir::new().unwrap();
    let _path = PathGuard::set(vide.path()); // epiclang introuvable
    let (_bat_dir, battery) = load_battery(BINARY_TOML);
    let mut ctx = common::test_ctx(&battery, &fixture("norme_ko"));

    let (ok, events) = run_norme(&mut ctx);

    // Si le chemin officiel avait été tenté, le spawn epiclang aurait
    // échoué et AUCUNE faute ne serait remontée : la présence des
    // fautes prouve que le moteur interne a tourné.
    assert!(ok, "le fallback interne doit réussir");
    assert!(
        ctx.norme_faults
            .iter()
            .any(|f| f.rule == "C-G1" && f.file == PathBuf::from("dirty.c")),
        "C-G1 interne absente : {:?}",
        ctx.norme_faults
    );
    assert!(
        ctx.norme_faults.iter().any(|f| f.rule == "C-C3"),
        "C-C3 interne absente : {:?}",
        ctx.norme_faults
    );
    let (fin_ok, summary) = step_finished(&events).expect("StepFinished manquant");
    assert!(fin_ok);
    assert!(summary.contains("major"), "summary : {summary}");
}

#[test]
fn strict_sans_epiclang_ko_explicite() {
    let _lock = ENV_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
    let vide = TempDir::new().unwrap();
    let _path = PathGuard::set(vide.path()); // epiclang introuvable
    let (_bat_dir, battery) = load_battery(BINARY_TOML);
    let mut ctx = common::test_ctx(&battery, &fixture("norme_ko"));
    ctx.opts.strict_norme = true;

    let (ok, events) = run_norme(&mut ctx);

    assert!(!ok, "strict + epiclang absent : l'étape doit échouer");
    let (fin_ok, summary) = step_finished(&events).expect("StepFinished manquant");
    assert!(!fin_ok);
    assert!(
        summary.contains("epiclang requis en mode strict, absent"),
        "summary : {summary}"
    );
    // L'échec est aussi visible dans le StepReport.
    let report = ctx
        .steps
        .iter()
        .find(|s| s.step == "norme")
        .expect("StepReport norme manquant");
    assert!(!report.ok);
}

// ----------------------------------------------------------------
// C-O1/C-O4 de prelim → ctx.norme_faults (scoring norme).
// ----------------------------------------------------------------

#[test]
fn prelim_co1_co4_remontes_dans_norme_faults() {
    let _lock = ENV_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
    let (_bat_dir, battery) = load_battery(BINARY_TOML);
    let mut ctx = common::test_ctx(&battery, &fixture("dirty_repo"));

    // Enchaînement réel : prelim (qui remplit désormais son
    // StepReport.checks) puis norme, sur le MÊME ctx.
    let (tx, _rx) = mpsc::channel();
    prelim::run(&mut ctx, &tx);
    drop(tx);
    let (ok, events) = run_norme(&mut ctx);
    assert!(ok);

    let co1: Vec<_> = ctx
        .norme_faults
        .iter()
        .filter(|f| f.rule == "C-O1" && f.file == PathBuf::from("truc~"))
        .collect();
    // La faute est présente UNE seule fois : le check « forbidden
    // files » de prelim et la ligne banana-check-repo doublonnent,
    // la conversion déduplique.
    assert_eq!(co1.len(), 1, "C-O1 truc~ dupliquée ou absente : {co1:?}");
    assert_eq!(co1[0].severity, Severity::Major);
    assert_eq!(co1[0].message, "unwanted file");
    assert_eq!((co1[0].line, co1[0].col), (0, 0));

    if banana_present() {
        assert!(
            ctx.norme_faults
                .iter()
                .any(|f| f.rule == "C-O4" && f.file == PathBuf::from("BadName.c")),
            "C-O4 BadName.c absente : {:?}",
            ctx.norme_faults
        );
    }
    // Les fautes converties sont aussi émises en events.
    let evts = norme_faults_events(&events);
    assert!(
        evts.iter().any(|f| f.rule == "C-O1"),
        "event NormeFault C-O1 manquant : {evts:?}"
    );
}

// ----------------------------------------------------------------
// Résumé « coding style clean ».
// ----------------------------------------------------------------

#[test]
fn repo_propre_resume_clean() {
    let _lock = ENV_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
    let (_bat_dir, battery) = load_battery(BINARY_TOML);
    let mut ctx = common::test_ctx(&battery, &fixture("clean_repo"));

    let (ok, events) = run_norme(&mut ctx);

    assert!(ok);
    assert!(
        ctx.norme_faults.is_empty(),
        "clean_repo flaggé : {:?}",
        ctx.norme_faults
    );
    let (fin_ok, summary) = step_finished(&events).expect("StepFinished manquant");
    assert!(fin_ok);
    assert_eq!(summary, "coding style clean");
}

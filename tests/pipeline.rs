//! Tests du pipeline orchestrateur (Task 3).

use seeyou::battery::Battery;
use seeyou::engine::events::{Event, Step};
use seeyou::engine::{run_pipeline, RunOpts};
use std::fs;
use std::path::PathBuf;
use std::sync::mpsc;
use tempfile::TempDir;

/// Batterie Functions minimale : une seule task.
const MINI_TOML: &str = r#"
[project]
name = "mini_functions"
type = "functions"

[[task]]
name = "ex01"
delivery = "my_putchar.c"
harness = "harness/ex01_main.c"
stdout = ""
"#;

/// Écrit la batterie minimale dans un tempdir et la charge.
/// Le TempDir est renvoyé pour garder `battery.root` valide.
fn mini_battery() -> (TempDir, Battery) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("moulinette.toml");
    fs::write(&path, MINI_TOML).unwrap();
    (dir, Battery::load(&path).unwrap())
}

fn test_opts() -> RunOpts {
    RunOpts {
        strict_norme: false,
        use_epiclang: false,
        compiler: PathBuf::from("cc"),
    }
}

#[test]
fn pipeline_emet_7_step_started_dans_l_ordre_puis_run_finished() {
    let (_bat_dir, battery) = mini_battery();
    let target = TempDir::new().unwrap(); // dossier projet vide

    let (tx, rx) = mpsc::channel();
    // run_pipeline est synchrone : appel direct, tx passé par valeur —
    // droppé au retour, donc l'itérateur sur rx termine.
    let report = run_pipeline(&battery, target.path(), tx, &test_opts());

    let events: Vec<Event> = rx.iter().collect();

    // Exactement 7 StepStarted, dans l'ordre croissant.
    let started: Vec<Step> = events
        .iter()
        .filter_map(|e| match e {
            Event::StepStarted { step, .. } => Some(*step),
            _ => None,
        })
        .collect();
    assert_eq!(
        started,
        vec![
            Step::Prelim,
            Step::Build,
            Step::Norme,
            Step::Symbols,
            Step::Unit,
            Step::Functional,
            Step::Verdict,
        ]
    );

    // Les 7 étapes se terminent ok (stubs) : 7 StepStarted + 7
    // StepFinished + 1 RunFinished = 15 événements, rien d'autre.
    let finished_ok = events
        .iter()
        .filter(|e| matches!(e, Event::StepFinished { ok: true, .. }))
        .count();
    assert_eq!(finished_ok, 7);
    assert_eq!(events.len(), 15, "événements inattendus : {events:?}");

    // Dernier événement : RunFinished, avec un rapport complet.
    match events.last() {
        Some(Event::RunFinished { report }) => {
            assert_eq!(report.project, "mini_functions");
            assert_eq!(report.steps.len(), 7);
        }
        other => panic!("dernier événement attendu : RunFinished, reçu : {other:?}"),
    }

    // Le rapport retourné est cohérent avec celui émis.
    assert_eq!(report.project, "mini_functions");
    assert_eq!(report.steps.len(), 7);
    assert!(report.steps.iter().all(|s| s.ok));
    assert_eq!(
        report
            .steps
            .iter()
            .map(|s| s.step.as_str())
            .collect::<Vec<_>>(),
        vec![
            "prelim",
            "build",
            "norme",
            "symbols",
            "unit",
            "functional",
            "verdict"
        ]
    );
    assert!(report.duration_secs.is_finite());
}

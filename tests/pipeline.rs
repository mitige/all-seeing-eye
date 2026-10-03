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
    let target = TempDir::new().unwrap();
    // L'étape Prelim (Task 4) exige la présence des deliveries :
    // sans ce fichier, elle échouerait légitimement.
    fs::write(
        target.path().join("my_putchar.c"),
        "void my_putchar(char c);\n",
    )
    .unwrap();

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

    // Les 7 étapes se terminent ok, non skipped : 7 StepStarted
    // + 7 StepFinished + 1 RunFinished, plus les CheckFinished de
    // l'étape Prelim (Task 4) — leur nombre exact dépend de la
    // présence de banana-check-repo sur la machine.
    let finished_ok = events
        .iter()
        .filter(|e| {
            matches!(
                e,
                Event::StepFinished {
                    ok: true,
                    skipped: false,
                    ..
                }
            )
        })
        .count();
    assert_eq!(finished_ok, 7);
    assert!(events.len() >= 15, "événements manquants : {events:?}");
    for e in &events {
        match e {
            Event::StepStarted { .. } | Event::StepFinished { .. } | Event::RunFinished { .. } => {}
            Event::CheckFinished { step, ok, .. } => {
                assert_eq!(*step, Step::Prelim, "check hors Prelim : {e:?}");
                assert!(*ok, "check Prelim en échec : {e:?}");
            }
            other => panic!("événement inattendu : {other:?}"),
        }
    }

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
    assert!(report.steps.iter().all(|s| !s.skipped));
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

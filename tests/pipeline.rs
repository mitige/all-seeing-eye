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
    // + 7 StepFinished + 1 RunFinished, plus les CheckFinished des
    // étapes Prelim (Task 4) et Build (Task 5) — leur nombre exact
    // dépend de la présence de banana-check-repo sur la machine.
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
    // L'étape Prelim a réellement émis des checks — sinon le filtre
    // ci-dessous (tout CheckFinished doit être Prelim et OK) ne
    // prouverait rien : une absence totale passerait inaperçue.
    let prelim_checks = events
        .iter()
        .filter(|e| {
            matches!(
                e,
                Event::CheckFinished {
                    step: Step::Prelim,
                    ..
                }
            )
        })
        .count();
    assert!(prelim_checks > 0, "aucun CheckFinished Prelim : {events:?}");
    for e in &events {
        match e {
            Event::StepStarted { .. } | Event::StepFinished { .. } | Event::RunFinished { .. } => {}
            // Task 5 : l'étape Build émet ses propres checks (compile
            // OK ici) et peut relayer des LogLine de sous-processus.
            Event::LogLine { step, .. } => {
                assert_eq!(*step, Step::Build, "LogLine hors Build : {e:?}");
            }
            Event::CheckFinished { step, ok, .. } => {
                assert!(
                    matches!(step, Step::Prelim | Step::Build | Step::Norme),
                    "check hors Prelim/Build/Norme : {e:?}"
                );
                // Prelim/Build passent sur ce rendu minimal. Norme :
                // la delivery n'a pas d'en-tête Epitech → la faute
                // C-G1 est légitime, le check par fichier est KO.
                if *step != Step::Norme {
                    assert!(*ok, "check en échec : {e:?}");
                }
            }
            // Task 6 : la norme émet ses fautes en events. Leur liste
            // dépend du moteur (banana voit aussi la C-H1 du prototype
            // dans le .c ; l'interne non), mais toutes visent la
            // delivery.
            Event::NormeFault(f) => {
                assert_eq!(f.file, PathBuf::from("my_putchar.c"));
            }
            other => panic!("événement inattendu : {other:?}"),
        }
    }
    // La faute C-G1 (pas d'en-tête Epitech — produite par les DEUX
    // moteurs) est remontée jusqu'au rapport : ctx.norme_faults →
    // Report.norme via le verdict.
    assert!(
        report
            .norme
            .iter()
            .any(|f| f.rule == "C-G1" && f.file == PathBuf::from("my_putchar.c")),
        "C-G1 absente du rapport : {:?}",
        report.norme
    );

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

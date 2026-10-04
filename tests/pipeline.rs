//! Tests du pipeline orchestrateur (Task 3).

use seeyou::battery::Battery;
use seeyou::engine::events::{Event, Step};
use seeyou::engine::{run_pipeline, RunOpts};
use std::fs;
use std::path::PathBuf;
use std::sync::mpsc;
use tempfile::TempDir;

mod common;

use common::{XdgGuard, XDG_MUTEX};

/// Positionne `XDG_DATA_HOME` (process-global — les tests qui
/// appellent `run_pipeline`, → verdict → `report::save`, se
/// sérialisent sur [`XDG_MUTEX`]) et le restaure à sa valeur initiale
/// au drop — un `cargo test` ne doit JAMAIS écraser le
/// `~/.local/share/seeyou/last.{json,txt}` de l'utilisateur.
fn xdg_data(path: &std::path::Path) -> XdgGuard {
    XdgGuard::set("XDG_DATA_HOME", path)
}

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
/// Task 9 : l'étape Functional n'est plus un stub — la task ex01 est
/// réellement compilée avec son harness (posé ici dans le root de la
/// batterie) puis exécutée (stdout "" attendu, exit 0).
fn mini_battery() -> (TempDir, Battery) {
    let dir = TempDir::new().unwrap();
    fs::create_dir(dir.path().join("harness")).unwrap();
    fs::write(
        dir.path().join("harness/ex01_main.c"),
        "void my_putchar(char c);\n\nint main(void)\n{\n    return 0;\n}\n",
    )
    .unwrap();
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
    // Le verdict sauvegarde le rapport : data dir isolé, jamais celui
    // de l'utilisateur.
    let _lock = XDG_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
    let data = TempDir::new().unwrap();
    let _xdg = xdg_data(data.path());
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

    // Les 7 étapes se terminent ok : 5 tournent réellement
    // (ok:true, skipped:false) ; Symbols (Task 7) et Unit (Task 8)
    // sont désactivées — la batterie minimale n'a ni
    // allowed_functions ni tests_run_rule — donc ok:true,
    // skipped:true. 7 StepStarted + 7 StepFinished + 1 RunFinished,
    // plus les CheckFinished des étapes Prelim (Task 4), Build
    // (Task 5) et Functional (Task 9 — son check « tests » agrégé)
    // — leur nombre exact dépend de la présence de
    // banana-check-repo sur la machine.
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
    assert_eq!(finished_ok, 5);
    let symbols_finished = events
        .iter()
        .find_map(|e| match e {
            Event::StepFinished {
                step: Step::Symbols,
                ok,
                skipped,
                summary,
            } => Some((*ok, *skipped, summary.clone())),
            _ => None,
        })
        .expect("StepFinished Symbols émis");
    assert!(
        symbols_finished.0 && symbols_finished.1 && symbols_finished.2.contains("skipped"),
        "Symbols attendue désactivée (ok:true, skipped:true) : {symbols_finished:?}"
    );
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
            // Task 5 : l'étape Build peut relayer des LogLine de
            // sous-processus ; Task 9 : Functional aussi (compile du
            // harness — silencieuse ici). Task 10 : Verdict relaie
            // une erreur de sauvegarde du rapport (best-effort) —
            // silencieuse ici, le data dir est un tempdir inscriptible.
            Event::LogLine { step, .. } => {
                assert!(
                    matches!(step, Step::Build | Step::Functional | Step::Verdict),
                    "LogLine hors Build/Functional/Verdict : {e:?}"
                );
            }
            // Task 9 : l'étape Functional exécute réellement la task
            // ex01 (compile + run) — Passed, stdout "" conforme.
            Event::TestStarted { group, name } => {
                assert_eq!((group.as_str(), name.as_str()), ("functional", "ex01"));
            }
            Event::TestFinished {
                group,
                name,
                result,
            } => {
                assert_eq!((group.as_str(), name.as_str()), ("functional", "ex01"));
                assert!(
                    matches!(result, seeyou::engine::events::TestVerdict::Passed),
                    "ex01 attendu Passed : {e:?}"
                );
            }
            Event::CheckFinished { step, ok, .. } => {
                assert!(
                    matches!(
                        step,
                        Step::Prelim | Step::Build | Step::Norme | Step::Functional
                    ),
                    "check hors Prelim/Build/Norme/Functional : {e:?}"
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
            } // Pas d'arm fourre-tout : le match est exhaustif — un
              // nouveau variant d'Event casse la compilation, garde-fou
              // plus fort qu'un panic.
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
    // Symbols (whitelist vide) et Unit (pas de tests_run_rule) sont
    // marquées skipped (désactivées) — avec ok:true, cf. ci-dessus.
    let skipped: Vec<&str> = report
        .steps
        .iter()
        .filter(|s| s.skipped)
        .map(|s| s.step.as_str())
        .collect();
    assert_eq!(skipped, vec!["symbols", "unit"]);
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

    // Task 10 : scores cohérents — la task ex01 passe : groupe
    // "functional" 1/1 = 100.0 ; pas de tests_run_rule → "unit" non
    // attendu. Global = moyenne simple des groupes = 100.0.
    assert_eq!(
        report.scores.par_groupe,
        vec![("functional".to_string(), 100.0)],
        "par_groupe inattendu : {:?}",
        report.scores.par_groupe
    );
    assert_eq!(report.scores.global, 100.0);
    // Les comptes norme du score reflètent exactement les fautes
    // collectées (leur nombre dépend du moteur : banana voit aussi la
    // C-H1 du prototype, l'interne non).
    let total_norme = report.scores.norme_fatal
        + report.scores.norme_major
        + report.scores.norme_minor
        + report.scores.norme_info;
    assert_eq!(
        total_norme as usize,
        report.norme.len(),
        "comptes norme incohérents : {:?} vs {} fautes",
        report.scores,
        report.norme.len()
    );
    // Le résumé du verdict porte le score global.
    let verdict = report
        .steps
        .iter()
        .find(|s| s.step == "verdict")
        .expect("étape verdict présente");
    assert!(
        verdict.summary.contains("score global: 100.0%"),
        "résumé verdict inattendu : {:?}",
        verdict.summary
    );

    // La sauvegarde du verdict a écrit dans le data dir ISOLÉ, jamais
    // dans celui de l'utilisateur.
    assert!(
        data.path().join("seeyou/last.json").is_file(),
        "last.json attendu dans le data dir isolé"
    );
    assert!(
        data.path().join("seeyou/last.txt").is_file(),
        "last.txt attendu dans le data dir isolé"
    );
}

/// Batterie Binary : un functional_test qui passe + règle tests_run.
const E2E_TOML: &str = r#"
[project]
name = "e2e_non_verbose"
type = "binary"
binary = "prog"
tests_run_rule = true

[[functional_test]]
name = "hello"
stdout = "hello\n"
"#;

/// Makefile complet (règles all/clean/fclean/re + tests_run), binaire
/// `prog` compilé depuis main.c. Les tabs sont significatifs.
const E2E_MAKEFILE: &str = "NAME\t=\tprog\n\
\n\
SRC\t=\tmain.c\n\
\n\
OBJ\t=\t$(SRC:.c=.o)\n\
\n\
CC\t=\tcc\n\
CFLAGS\t=\t-Wall -Wextra -Werror\n\
\n\
all:\t$(NAME)\n\
\n\
$(NAME):\t$(OBJ)\n\
\t$(CC) -o $(NAME) $(OBJ)\n\
\n\
tests_run:\n\
\t@./tests_run.sh\n\
\n\
clean:\n\
\trm -f $(OBJ)\n\
\n\
fclean:\tclean\n\
\trm -f $(NAME)\n\
\n\
re:\tfclean all\n\
\n\
.PHONY:\tall clean fclean re tests_run\n";

#[test]
fn pipeline_non_verbose_vert_synthetise_les_records_unit() {
    // Reproduction e2e de l'issue « mensonge de score unit » : le vrai
    // criterion SANS --verbose n'émet QUE la Synthesis pour une suite
    // 100 % verte — sans records synthétiques, le groupe « unit » est
    // vide → 0 %, et le global tombe à 50 % alors que tout est OK.
    let _lock = XDG_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
    let data = TempDir::new().unwrap();
    let _xdg = xdg_data(data.path());

    let bat_dir = TempDir::new().unwrap();
    let bat_path = bat_dir.path().join("moulinette.toml");
    fs::write(&bat_path, E2E_TOML).unwrap();
    let battery = Battery::load(&bat_path).unwrap();

    let target = TempDir::new().unwrap();
    fs::write(target.path().join("Makefile"), E2E_MAKEFILE).unwrap();
    fs::write(
        target.path().join("main.c"),
        "#include <stdio.h>\n\nint main(void)\n{\n    printf(\"hello\\n\");\n    return 0;\n}\n",
    )
    .unwrap();
    // Non-verbose, 100 % vert : Synthesis seule, tout sur stderr comme
    // le vrai criterion.
    let script = target.path().join("tests_run.sh");
    fs::write(
        &script,
        "#!/bin/sh\necho '[====] Synthesis: Tested: 3 | Passing: 3 | Failing: 0 | Crashing: 0 ' >&2\nexit 0\n",
    )
    .unwrap();
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();

    let (tx, rx) = mpsc::channel();
    let report = run_pipeline(&battery, target.path(), tx, &test_opts());
    let events: Vec<Event> = rx.iter().collect();

    // L'étape Unit est OK — et le score doit le REFLETER : groupe
    // « unit » à 100 %, global à 100 % (pas 50 %).
    let unit_step = report
        .steps
        .iter()
        .find(|s| s.step == "unit")
        .expect("étape unit présente");
    assert!(unit_step.ok, "étape unit attendue OK : {unit_step:?}");
    assert_eq!(
        report.scores.par_groupe,
        vec![
            ("functional".to_string(), 100.0),
            ("unit".to_string(), 100.0),
        ],
        "le groupe unit vide ferait chuter le score : {:?}",
        report.scores.par_groupe
    );
    assert_eq!(
        report.scores.global, 100.0,
        "global attendu 100.0 (50.0 = mensonge du groupe vide) : {:?}",
        report.scores
    );
    // Les records synthétiques matérialisent la Synthesis.
    let unit_records: Vec<_> = report.tests.iter().filter(|t| t.group == "unit").collect();
    assert_eq!(
        unit_records.len(),
        3,
        "3 records synthétiques attendus : {:?}",
        report.tests
    );
    for (i, record) in unit_records.iter().enumerate() {
        assert_eq!(record.name, format!("criterion #{}", i + 1));
        assert_eq!(record.verdict, "passed", "{record:?}");
    }
    // Le TUI les voit : TestStarted/TestFinished pour chacun, groupe
    // « unit ».
    let unit_events: Vec<&str> = events
        .iter()
        .filter_map(|e| match e {
            Event::TestStarted { group, name } | Event::TestFinished { group, name, .. }
                if group == "unit" =>
            {
                Some(name.as_str())
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        unit_events,
        vec![
            "criterion #1",
            "criterion #1",
            "criterion #2",
            "criterion #2",
            "criterion #3",
            "criterion #3"
        ],
        "events synthétiques attendus (Started+Finished par test) : {events:?}"
    );
}

#[test]
fn pipeline_survit_a_un_echec_de_sauvegarde_du_rapport() {
    // Branche save-en-échec du verdict : `XDG_DATA_HOME` pointe vers
    // un FICHIER régulier → create_dir_all(fichier/seeyou) échoue
    // (NotADirectory) → save() KO. La sauvegarde est best-effort : le
    // run survit, le rapport est quand même retourné ET émis, et
    // l'erreur est relayée en LogLine Step::Verdict.
    let _lock = XDG_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
    let data = TempDir::new().unwrap();
    let fichier = data.path().join("pas_un_dossier");
    fs::write(&fichier, "x").unwrap();
    let _xdg = xdg_data(&fichier);
    let (_bat_dir, battery) = mini_battery();
    let target = TempDir::new().unwrap();
    fs::write(
        target.path().join("my_putchar.c"),
        "void my_putchar(char c);\n",
    )
    .unwrap();

    let (tx, rx) = mpsc::channel();
    let report = run_pipeline(&battery, target.path(), tx, &test_opts());
    let events: Vec<Event> = rx.iter().collect();

    // Le rapport est retourné, complet, malgré l'échec de sauvegarde.
    assert_eq!(report.project, "mini_functions");
    assert_eq!(report.steps.len(), 7);
    // … et émis en dernier événement, comme toujours.
    assert!(
        matches!(events.last(), Some(Event::RunFinished { .. })),
        "dernier événement attendu : RunFinished : {events:?}"
    );
    // L'erreur de sauvegarde est relayée, visible, jamais silencieuse.
    let erreurs_save: Vec<&str> = events
        .iter()
        .filter_map(|e| match e {
            Event::LogLine {
                step: Step::Verdict,
                line,
            } => Some(line.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(
        erreurs_save.len(),
        1,
        "exactement une LogLine Verdict attendue : {events:?}"
    );
    assert!(
        erreurs_save[0].contains("sauvegarde du rapport impossible"),
        "message inattendu : {:?}",
        erreurs_save[0]
    );
    // Rien n'a été écrit : le « data dir » est un fichier.
    assert!(fichier.is_file(), "le fichier XDG ne doit pas bouger");
}

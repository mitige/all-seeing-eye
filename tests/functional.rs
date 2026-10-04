//! Tests de l'étape Functional (Task 9) : `run_one` au caractère près
//! (diff unified `-attendu`/`+obtenu`, crash, timeout, troncature de
//! capture), tests Binary end-to-end sur la fixture ok_project (vrai
//! build en amont, pour valider le contrat inter-étapes
//! `ctx.build.binary`) et tasks Functions (compile delivery + harness
//! + exécution), étape désactivée et KO d'infrastructure.
//!
//! L'étape est pilotée isolément : pour les Functions, `ctx.build`
//! est construit à la main (salle blanche = tempdir contenant la
//! delivery, exactement ce que le build produit) — même convention
//! que tests/unit.rs. Le `target` factice est droppé : functional ne
//! lit jamais `ctx.target` (elle travaille sur `ctx.build`).

mod common;

use all_seeing_eye::battery::Battery;
use all_seeing_eye::engine::events::{Event, Step, TestVerdict};
use all_seeing_eye::engine::{build, functional, BuildArtifacts, PipelineContext};
use all_seeing_eye::exec::{ExecStatus, MAX_CAPTURE_BYTES};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};
use tempfile::TempDir;

/// Écrit `toml` dans un tempdir et charge la batterie.
/// Le TempDir est renvoyé pour garder `battery.root` valide.
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

/// Batterie Binary adossée à la fixture ok_project, un test
/// fonctionnel vérifiant la sortie hello.
const BINARY_TOML: &str = r#"
[project]
name = "ok_project"
type = "binary"
binary = "ok_project"

[[functional_test]]
name = "hello"
stdout = "Hello, World!\n"
"#;

/// La même, mais l'attendu ne correspond pas à la sortie réelle.
const BINARY_KO_TOML: &str = r#"
[project]
name = "ok_project"
type = "binary"
binary = "ok_project"

[[functional_test]]
name = "hello_ko"
stdout = "Bonjour\n"
"#;

/// Batterie Binary sans aucun test fonctionnel.
const BINARY_SANS_TEST_TOML: &str = r#"
[project]
name = "ok_project"
type = "binary"
binary = "ok_project"
"#;

/// Batterie Functions : my_puts écrit son argument + '\n'. Le harness
/// vit dans `battery.root` (dossier de la batterie), la delivery dans
/// le rendu — comme une vraie batterie piscine.
const PUTS_TOML: &str = r#"
[project]
name = "fn_puts"
type = "functions"

[[task]]
name = "my_puts"
delivery = "my_puts.c"
harness = "harness/main.c"
stdout = "abc\n"
"#;

/// La même avec un timeout court, pour l'impl bouclante.
const PUTS_TIMEOUT_TOML: &str = r#"
[project]
name = "fn_puts"
type = "functions"

[[task]]
name = "my_puts"
delivery = "my_puts.c"
harness = "harness/main.c"
stdout = "abc\n"
timeout_ms = 300
"#;

/// Harness de my_puts : appelle my_puts("abc"), sans stdin ni args.
const PUTS_HARNESS: &str = r#"void my_puts(char const *s);

int main(void)
{
    my_puts("abc");
    return 0;
}
"#;

/// Impl correcte : écrit s puis '\n' (valeur de retour de write
/// consommée : glibc la déclare __wur, -Werror l'exige).
const PUTS_OK: &str = r#"
#include <unistd.h>

void my_puts(char const *s)
{
    while (*s != '\0') {
        if (write(1, s, 1) == -1)
            return;
        s++;
    }
    if (write(1, "\n", 1) == -1)
        return;
}
"#;

/// Impl oubliant le '\n' final.
const PUTS_SANS_NEWLINE: &str = r#"
#include <unistd.h>

void my_puts(char const *s)
{
    while (*s != '\0') {
        if (write(1, s, 1) == -1)
            return;
        s++;
    }
}
"#;

/// Impl bouclante sur toute entrée non vide.
const PUTS_BOUCLE: &str = r#"
void my_puts(char const *s)
{
    while (*s != '\0') {
    }
}
"#;

/// Impl syntaxiquement cassée.
const PUTS_CASSE: &str = r#"
void my_puts(char const *s)
{
    s = ;
}
"#;

/// Batterie my_puts avec son harness posé dans le root.
fn puts_battery(toml: &str) -> (TempDir, Battery) {
    let (dir, battery) = load_battery(toml);
    fs::create_dir(dir.path().join("harness")).unwrap();
    fs::write(dir.path().join("harness/main.c"), PUTS_HARNESS).unwrap();
    (dir, battery)
}

/// Lance l'étape Functional sur une salle blanche factice contenant
/// `impl_c` en `my_puts.c` ; renvoie le verdict, les événements et le
/// ctx (qui garde la salle blanche vivante le temps des assertions).
fn run_functions_step(battery: &Battery, impl_c: &str) -> (bool, Vec<Event>, PipelineContext) {
    let white = TempDir::new().unwrap();
    fs::write(white.path().join("my_puts.c"), impl_c).unwrap();
    let target = TempDir::new().unwrap();
    let mut ctx = common::test_ctx(battery, target.path());
    ctx.build = Some(BuildArtifacts {
        dir: white,
        binary: None,
    });
    let (tx, rx) = mpsc::channel();
    let ok = functional::run(&mut ctx, &tx);
    drop(tx);
    (ok, rx.iter().collect(), ctx)
}

/// Lance le vrai build puis l'étape Functional sur la fixture Binary ;
/// renvoie le verdict functional, tous les événements (Build inclus)
/// et le ctx.
fn run_binary_step(battery: &Battery, target: &Path) -> (bool, Vec<Event>, PipelineContext) {
    let mut ctx = common::test_ctx(battery, target);
    let (tx, rx) = mpsc::channel();
    assert!(
        build::run(&mut ctx, &tx),
        "build KO en amont de l'étape functional"
    );
    let ok = functional::run(&mut ctx, &tx);
    drop(tx);
    (ok, rx.iter().collect(), ctx)
}

/// Le `(ok, skipped, summary)` du StepFinished Functional —
/// exactement un par run (l'invariant de l'orchestrateur vaut aussi
/// à l'isolement).
fn step_finished(events: &[Event]) -> (bool, bool, String) {
    let mut found = events.iter().filter_map(|e| match e {
        Event::StepFinished {
            step: Step::Functional,
            ok,
            skipped,
            summary,
        } => Some((*ok, *skipped, summary.clone())),
        _ => None,
    });
    let first = found.next().expect("StepFinished Functional émis");
    assert!(found.next().is_none(), "plusieurs StepFinished Functional");
    first
}

/// Séquence des événements de test `(true = Started, nom)` /
/// `(false = Finished, nom)` ; tout test doit être dans le groupe
/// « functional ».
fn test_event_sequence(events: &[Event]) -> Vec<(bool, String)> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::TestStarted { group, name } => {
                assert_eq!(group, "functional", "groupe inattendu : {group}");
                Some((true, name.clone()))
            }
            Event::TestFinished { group, name, .. } => {
                assert_eq!(group, "functional", "groupe inattendu : {group}");
                Some((false, name.clone()))
            }
            _ => None,
        })
        .collect()
}

/// Tests terminés `(nom, verdict)`, dans l'ordre d'émission.
fn finished_tests(events: &[Event]) -> Vec<(String, TestVerdict)> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::TestFinished { name, result, .. } => Some((name.clone(), result.clone())),
            _ => None,
        })
        .collect()
}

/// Extrait les checks `(name, ok, detail)` estampillés Functional —
/// le flux des tests Binary contient aussi les checks du Build réel
/// joué en amont : on filtre, on ne les rejette pas.
fn extract_checks(events: &[Event]) -> Vec<(String, bool, String)> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::CheckFinished {
                step: Step::Functional,
                name,
                ok,
                detail,
            } => Some((name.clone(), *ok, detail.clone())),
            _ => None,
        })
        .collect()
}

// ----------------------------------------------------------------
// run_one : comparaison au caractère près
// ----------------------------------------------------------------

#[test]
fn run_one_sortie_exacte_passed() {
    let tmp = TempDir::new().unwrap();
    let (verdict, outcome) = functional::run_one(
        Path::new("echo"),
        &["hello".to_string()],
        "",
        tmp.path(),
        "hello\n",
        "",
        0,
        Duration::from_secs(2),
    );
    assert!(
        matches!(verdict, TestVerdict::Passed),
        "sortie identique attendue Passed : {verdict:?}"
    );
    assert!(matches!(outcome.status, ExecStatus::Exit(0)));
    assert!(!outcome.stdout_truncated && !outcome.stderr_truncated);
}

#[test]
fn run_one_stdin_relaye_au_fils() {
    let tmp = TempDir::new().unwrap();
    let (verdict, _outcome) = functional::run_one(
        Path::new("sh"),
        &["-c".to_string(), "read x; echo \"got:$x\"".to_string()],
        "abc\n",
        tmp.path(),
        "got:abc\n",
        "",
        0,
        Duration::from_secs(2),
    );
    assert!(
        matches!(verdict, TestVerdict::Passed),
        "stdin relayé attendu Passed : {verdict:?}"
    );
}

#[test]
fn run_one_stdout_different_failed_diff_unified() {
    let tmp = TempDir::new().unwrap();
    let (verdict, _outcome) = functional::run_one(
        Path::new("echo"),
        &["obtenu".to_string()],
        "",
        tmp.path(),
        "attendu\n",
        "",
        0,
        Duration::from_secs(2),
    );
    match verdict {
        TestVerdict::Failed {
            diff,
            expected,
            got,
        } => {
            assert_eq!(expected, "attendu\n");
            assert_eq!(got, "obtenu\n");
            assert!(
                diff.contains("-attendu"),
                "ligne -attendu absente du diff : {diff:?}"
            );
            assert!(
                diff.contains("+obtenu"),
                "ligne +obtenu absente du diff : {diff:?}"
            );
        }
        other => panic!("stdout différent attendu Failed : {other:?}"),
    }
}

#[test]
fn run_one_stderr_different_failed() {
    let tmp = TempDir::new().unwrap();
    let (verdict, _outcome) = functional::run_one(
        Path::new("sh"),
        &["-c".to_string(), "echo oops >&2".to_string()],
        "",
        tmp.path(),
        "",
        "",
        0,
        Duration::from_secs(2),
    );
    match verdict {
        TestVerdict::Failed {
            diff,
            expected,
            got,
        } => {
            assert_eq!(expected, "");
            assert_eq!(got, "oops\n");
            assert!(
                diff.contains("+oops"),
                "ligne +oops absente du diff : {diff:?}"
            );
        }
        other => panic!("stderr différent attendu Failed : {other:?}"),
    }
}

#[test]
fn run_one_exit_code_different_failed() {
    let tmp = TempDir::new().unwrap();
    let (verdict, outcome) = functional::run_one(
        Path::new("sh"),
        &["-c".to_string(), "exit 3".to_string()],
        "",
        tmp.path(),
        "",
        "",
        0,
        Duration::from_secs(2),
    );
    match verdict {
        TestVerdict::Failed { expected, got, .. } => {
            assert_eq!(expected, "0");
            assert_eq!(got, "3");
        }
        other => panic!("exit code différent attendu Failed : {other:?}"),
    }
    assert!(matches!(outcome.status, ExecStatus::Exit(3)));
}

#[test]
fn run_one_segv_crashed_11() {
    let tmp = TempDir::new().unwrap();
    let (verdict, outcome) = functional::run_one(
        Path::new("sh"),
        &["-c".to_string(), "kill -SEGV $$".to_string()],
        "",
        tmp.path(),
        "",
        "",
        0,
        Duration::from_secs(2),
    );
    assert!(
        matches!(verdict, TestVerdict::Crashed(11)),
        "SIGSEGV attendu Crashed(11) : {verdict:?}"
    );
    assert!(matches!(outcome.status, ExecStatus::Signal(11)));
}

#[test]
fn run_one_sleep_timeout() {
    let tmp = TempDir::new().unwrap();
    let (verdict, outcome) = functional::run_one(
        Path::new("sh"),
        &["-c".to_string(), "sleep 5".to_string()],
        "",
        tmp.path(),
        "",
        "",
        0,
        Duration::from_millis(200),
    );
    assert!(
        matches!(verdict, TestVerdict::Timeout),
        "sleep borné attendu Timeout : {verdict:?}"
    );
    assert!(matches!(outcome.status, ExecStatus::Timeout));
}

#[test]
fn run_one_sortie_tronquee_jamais_passed() {
    // Plus de 4 Mio sur stdout, attendu = exactement la capture bornée
    // : tout matche, mais un Passed sur une capture incomplète est
    // interdit — KO tronqué explicite.
    let tmp = TempDir::new().unwrap();
    let expected = "x".repeat(MAX_CAPTURE_BYTES);
    let (verdict, outcome) = functional::run_one(
        Path::new("sh"),
        &[
            "-c".to_string(),
            "head -c 5000000 /dev/zero | tr '\\0' 'x'".to_string(),
        ],
        "",
        tmp.path(),
        &expected,
        "",
        0,
        Duration::from_secs(10),
    );
    assert!(
        outcome.stdout_truncated,
        "le drapeau de troncature doit être levé"
    );
    match verdict {
        TestVerdict::Failed { diff, .. } => {
            assert!(
                diff.contains("tronquée"),
                "diff « sortie tronquée » absent : {diff:?}"
            );
        }
        other => panic!("capture tronquée : jamais Passed : {other:?}"),
    }
}

// ----------------------------------------------------------------
// Étape, projet Binary
// ----------------------------------------------------------------

#[test]
fn binary_end_to_end_hello_passed() {
    let (_bat_dir, battery) = load_battery(BINARY_TOML);
    let (ok, events, ctx) = run_binary_step(&battery, &fixture("ok_project"));

    assert!(ok, "hello conforme : l'étape passe : {events:?}");
    // TestStarted précède immédiatement TestFinished, groupe functional.
    assert_eq!(
        test_event_sequence(&events),
        vec![(true, "hello".to_string()), (false, "hello".to_string())]
    );
    let tests = finished_tests(&events);
    assert_eq!(tests.len(), 1, "{tests:?}");
    assert!(
        matches!(tests[0].1, TestVerdict::Passed),
        "hello attendu Passed : {tests:?}"
    );
    // ctx.tests alimenté pour le rapport.
    assert_eq!(ctx.tests.len(), 1, "TestRecords : {:?}", ctx.tests);
    let record = &ctx.tests[0];
    assert_eq!(record.group, "functional");
    assert_eq!(record.name, "hello");
    assert_eq!(record.verdict, "passed");
    assert_eq!(record.diff, None);
    // Étape OK, non skipped, résumé parlant.
    let (ok_ev, skipped, summary) = step_finished(&events);
    assert!(ok_ev && !skipped, "étape OK, non skipped");
    assert!(
        summary.contains("1 tests") && summary.contains("0 en échec"),
        "résumé inattendu : {summary:?}"
    );
    let checks = extract_checks(&events);
    assert!(
        checks.iter().all(|(_, ok, _)| *ok),
        "KO inattendu : {checks:?}"
    );
}

#[test]
fn binary_sortie_differente_etape_ko_avec_diff() {
    let (_bat_dir, battery) = load_battery(BINARY_KO_TOML);
    let (ok, events, ctx) = run_binary_step(&battery, &fixture("ok_project"));

    assert!(!ok, "un test KO fait échouer l'étape");
    let tests = finished_tests(&events);
    assert_eq!(tests.len(), 1, "{tests:?}");
    assert_eq!(tests[0].0, "hello_ko");
    match &tests[0].1 {
        TestVerdict::Failed {
            diff,
            expected,
            got,
        } => {
            assert_eq!(expected, "Bonjour\n");
            assert_eq!(got, "Hello, World!\n");
            assert!(diff.contains("-Bonjour"), "diff : {diff:?}");
            assert!(diff.contains("+Hello, World!"), "diff : {diff:?}");
        }
        other => panic!("hello_ko attendu Failed : {other:?}"),
    }
    let record = &ctx.tests[0];
    assert_eq!(record.verdict, "failed");
    assert!(
        record
            .diff
            .as_deref()
            .is_some_and(|d| d.contains("-Bonjour")),
        "diff du record absent : {record:?}"
    );
    let (ok_ev, skipped, _) = step_finished(&events);
    assert!(!ok_ev && !skipped, "étape KO, non skipped");
}

#[test]
fn binary_sans_functional_test_etape_desactivee() {
    let (_bat_dir, battery) = load_battery(BINARY_SANS_TEST_TOML);
    // Pas même besoin d'un build : le skip de config précède tout.
    let target = TempDir::new().unwrap();
    let mut ctx = common::test_ctx(&battery, target.path());
    let (tx, rx) = mpsc::channel();
    let ok = functional::run(&mut ctx, &tx);
    drop(tx);
    let events: Vec<Event> = rx.iter().collect();

    assert!(ok, "désactivée : l'étape réussit (skip de config)");
    let (ok_ev, skipped, summary) = step_finished(&events);
    assert!(ok_ev, "skip de config : ok:true (pas un échec)");
    assert!(skipped, "skip de config : skipped:true");
    assert!(
        summary.contains("skipped"),
        "summary sans « skipped » : {summary:?}"
    );
    assert!(
        test_event_sequence(&events).is_empty(),
        "aucun test rapporté : {events:?}"
    );
    assert!(
        extract_checks(&events).is_empty(),
        "aucun check quand l'étape est désactivée : {events:?}"
    );
    assert!(ctx.tests.is_empty(), "ctx.tests vide : {:?}", ctx.tests);
}

#[test]
fn binary_sans_build_ko_explicite() {
    // En pipeline, run_step skippe Functional si le build a échoué ;
    // à l'isolement, un ctx sans build est un KO explicite, jamais un
    // vert à vide.
    let (_bat_dir, battery) = load_battery(BINARY_TOML);
    let target = TempDir::new().unwrap();
    let mut ctx = common::test_ctx(&battery, target.path());
    let (tx, rx) = mpsc::channel();
    let ok = functional::run(&mut ctx, &tx);
    drop(tx);
    let events: Vec<Event> = rx.iter().collect();

    assert!(!ok, "build absent : KO explicite");
    assert!(
        test_event_sequence(&events).is_empty(),
        "aucun test sans build : {events:?}"
    );
    let checks = extract_checks(&events);
    assert!(
        checks
            .iter()
            .any(|(_, ok, d)| !*ok && d.contains("build absent")),
        "check KO « build absent » absent : {checks:?}"
    );
    let (ok_ev, skipped, _) = step_finished(&events);
    assert!(!ok_ev && !skipped);
}

#[test]
fn binary_build_sans_binaire_ko_explicite() {
    let (_bat_dir, battery) = load_battery(BINARY_TOML);
    let target = TempDir::new().unwrap();
    let mut ctx = common::test_ctx(&battery, target.path());
    ctx.build = Some(BuildArtifacts {
        dir: TempDir::new().unwrap(),
        binary: None,
    });
    let (tx, rx) = mpsc::channel();
    let ok = functional::run(&mut ctx, &tx);
    drop(tx);
    let events: Vec<Event> = rx.iter().collect();

    assert!(!ok, "binaire absent : KO explicite");
    let checks = extract_checks(&events);
    assert!(
        checks
            .iter()
            .any(|(_, ok, d)| !*ok && d.contains("binaire")),
        "check KO « binaire absent » absent : {checks:?}"
    );
    let (ok_ev, skipped, _) = step_finished(&events);
    assert!(!ok_ev && !skipped);
}

// ----------------------------------------------------------------
// Étape, projet Functions
// ----------------------------------------------------------------

#[test]
fn functions_end_to_end_passed() {
    let (_bat_dir, battery) = puts_battery(PUTS_TOML);
    let (ok, events, ctx) = run_functions_step(&battery, PUTS_OK);

    assert!(ok, "impl correcte : l'étape passe : {events:?}");
    assert_eq!(
        test_event_sequence(&events),
        vec![
            (true, "my_puts".to_string()),
            (false, "my_puts".to_string())
        ]
    );
    let tests = finished_tests(&events);
    assert_eq!(tests.len(), 1, "{tests:?}");
    assert!(
        matches!(tests[0].1, TestVerdict::Passed),
        "my_puts attendu Passed : {tests:?}"
    );
    assert_eq!(ctx.tests.len(), 1, "TestRecords : {:?}", ctx.tests);
    assert_eq!(ctx.tests[0].verdict, "passed");
    // Le binaire de test est produit dans la salle blanche, nommé
    // d'après le stem de la delivery.
    let white = ctx
        .build
        .as_ref()
        .expect("ctx.build posé")
        .dir
        .path()
        .to_path_buf();
    assert!(
        white.join("my_puts_test").is_file(),
        "my_puts_test absent de la salle blanche"
    );
    let (ok_ev, skipped, _) = step_finished(&events);
    assert!(ok_ev && !skipped);
}

#[test]
fn functions_newline_manquante_failed_diff_montre_le_manque() {
    let (_bat_dir, battery) = puts_battery(PUTS_TOML);
    let (ok, events, ctx) = run_functions_step(&battery, PUTS_SANS_NEWLINE);

    assert!(!ok, "impl sans \\n : l'étape échoue");
    let tests = finished_tests(&events);
    assert_eq!(tests.len(), 1, "{tests:?}");
    match &tests[0].1 {
        TestVerdict::Failed { diff, got, .. } => {
            assert_eq!(got, "abc", "sortie sans newline final : {got:?}");
            assert!(diff.contains("+abc"), "ligne +abc absente : {diff:?}");
            // Le manque du newline final est visible dans le diff.
            assert!(
                diff.contains("newline"),
                "marqueur de newline manquant absent du diff : {diff:?}"
            );
        }
        other => panic!("impl sans \\n attendue Failed : {other:?}"),
    }
    assert_eq!(ctx.tests[0].verdict, "failed");
    let (ok_ev, skipped, _) = step_finished(&events);
    assert!(!ok_ev && !skipped);
}

#[test]
fn functions_impl_bouclante_timeout() {
    let (_bat_dir, battery) = puts_battery(PUTS_TIMEOUT_TOML);
    let (ok, events, ctx) = run_functions_step(&battery, PUTS_BOUCLE);

    assert!(!ok, "impl bouclante : l'étape échoue");
    let tests = finished_tests(&events);
    assert_eq!(tests.len(), 1, "{tests:?}");
    assert!(
        matches!(tests[0].1, TestVerdict::Timeout),
        "impl bouclante attendue Timeout : {tests:?}"
    );
    assert_eq!(ctx.tests[0].verdict, "timeout");
    let (ok_ev, skipped, _) = step_finished(&events);
    assert!(!ok_ev && !skipped);
}

#[test]
fn functions_compile_ko_failed_extrait_compile() {
    let (_bat_dir, battery) = puts_battery(PUTS_TOML);
    let (ok, events, ctx) = run_functions_step(&battery, PUTS_CASSE);

    assert!(!ok, "delivery cassée : l'étape échoue");
    let tests = finished_tests(&events);
    // Un seul TestFinished : la compile KO est le verdict du test,
    // nommé d'après la task — jamais d'exécution.
    assert_eq!(tests.len(), 1, "{tests:?}");
    assert_eq!(tests[0].0, "my_puts");
    match &tests[0].1 {
        TestVerdict::Failed {
            diff,
            expected,
            got,
        } => {
            assert!(expected.is_empty() && got.is_empty());
            // LC_ALL=C : le diagnostic du compilateur contient « error ».
            assert!(
                diff.contains("error"),
                "extrait compile absent du diff : {diff:?}"
            );
        }
        other => panic!("compile KO attendue Failed : {other:?}"),
    }
    assert_eq!(ctx.tests[0].verdict, "failed");
    let (ok_ev, skipped, _) = step_finished(&events);
    assert!(!ok_ev && !skipped);
}

// ----------------------------------------------------------------
// Review Task 9 : diff borné (deadline, entrées abrégées, rendu
// capé), CRLF visible, garde troncature AVANT diff, chemins et
// propagations de batterie.
// ----------------------------------------------------------------

/// Variante de [`run_functions_step`] avec un nom de delivery
/// paramétré (la batterie porte `delivery = "<name>"`).
fn run_functions_step_delivery(
    battery: &Battery,
    delivery: &str,
    impl_c: &str,
) -> (bool, Vec<Event>, PipelineContext) {
    let white = TempDir::new().unwrap();
    fs::write(white.path().join(delivery), impl_c).unwrap();
    let target = TempDir::new().unwrap();
    let mut ctx = common::test_ctx(battery, target.path());
    ctx.build = Some(BuildArtifacts {
        dir: white,
        binary: None,
    });
    let (tx, rx) = mpsc::channel();
    let ok = functional::run(&mut ctx, &tx);
    drop(tx);
    (ok, rx.iter().collect(), ctx)
}

/// Pose `files` (chemins relatifs → contenu) dans le root d'une
/// batterie chargée depuis `toml`.
fn battery_avec_fichiers(toml: &str, files: &[(&str, &str)]) -> (TempDir, Battery) {
    let (dir, battery) = load_battery(toml);
    for (rel, contenu) in files {
        let path = dir.path().join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contenu).unwrap();
    }
    (dir, battery)
}

#[test]
fn run_one_diff_20k_lignes_reste_sous_3s() {
    // 20 000 lignes attendues vs 20 000 lignes obtenues, toutes
    // différentes (pire cas Myers, quadratique) : le calcul est borné
    // par une deadline (approximation au-delà) et le rendu est capé —
    // run_one répond en moins de 3 s.
    let tmp = TempDir::new().unwrap();
    let expected: String = (0..20_000)
        .map(|i| format!("ligne_attendue_{i}\n"))
        .collect();
    let start = Instant::now();
    let (verdict, _outcome) = functional::run_one(
        Path::new("seq"),
        &["20000".to_string()],
        "",
        tmp.path(),
        &expected,
        "",
        0,
        Duration::from_secs(10),
    );
    let elapsed = start.elapsed();
    assert!(
        matches!(verdict, TestVerdict::Failed { .. }),
        "20k lignes différentes attendues Failed : {verdict:?}"
    );
    assert!(
        elapsed < Duration::from_secs(3),
        "run_one sur 20k lignes différentes trop lent : {elapsed:?}"
    );
}

#[test]
fn run_one_entrees_enormes_diff_abrege() {
    // Plus d'1 Mio cumulé (expected + got) : les entrées sont abrégées
    // AVANT le calcul du diff — tête de 200 lignes, marqueur
    // « … N lignes omises … », queue de 50 — et le rendu reste
    // quasi-immédiat.
    let tmp = TempDir::new().unwrap();
    let expected: String = (0..100_000).map(|i| format!("attendue_{i:06}\n")).collect();
    assert!(
        expected.len() > 1024 * 1024,
        "l'attendu doit dépasser 1 Mio pour ce test"
    );
    let start = Instant::now();
    let (verdict, _outcome) = functional::run_one(
        Path::new("echo"),
        &["tiny".to_string()],
        "",
        tmp.path(),
        &expected,
        "",
        0,
        Duration::from_secs(5),
    );
    assert!(
        start.elapsed() < Duration::from_secs(3),
        "diff abrégé quasi-immédiat : {:?}",
        start.elapsed()
    );
    match verdict {
        TestVerdict::Failed { diff, .. } => {
            assert!(
                diff.contains("-attendue_000000"),
                "tête abrégée absente du diff : {:?}",
                &diff[..diff.len().min(500)]
            );
            assert!(
                diff.contains("lignes omises"),
                "marqueur d'omission absent du diff : {:?}",
                &diff[..diff.len().min(500)]
            );
        }
        other => panic!("attendu géant vs tiny attendu Failed : {other:?}"),
    }
}

#[test]
fn binary_diff_geant_borne_dans_event_et_record() {
    // 2 000 lignes attendues vs « Hello, World!\n » obtenu : sous la
    // borne d'abrègement (diff complet calculé), mais le RENDU est
    // borné à ~500 lignes avec marqueur de troncature — dans l'event
    // TestFinished comme dans le TestRecord du rapport.
    let attendu: String = (0..2_000).map(|i| format!("ligne_{i}\n")).collect();
    let (_bat_dir, battery) = battery_avec_fichiers(
        r#"
[project]
name = "ok_project"
type = "binary"
binary = "ok_project"

[[functional_test]]
name = "geant"
stdout_file = "attendu.txt"
"#,
        &[("attendu.txt", &attendu)],
    );

    let (ok, events, ctx) = run_binary_step(&battery, &fixture("ok_project"));
    assert!(!ok, "sortie différente : l'étape échoue");

    let tests = finished_tests(&events);
    assert_eq!(tests.len(), 1, "{tests:?}");
    let TestVerdict::Failed { diff, .. } = &tests[0].1 else {
        panic!("geant attendu Failed : {tests:?}")
    };
    assert!(
        diff.lines().count() <= 510,
        "diff de l'event non borné : {} lignes",
        diff.lines().count()
    );
    assert!(
        diff.contains("diff tronqué"),
        "marqueur de troncature absent de l'event : {:?}",
        &diff[..diff.len().min(500)]
    );
    // Le TestRecord porte exactement le même diff (borné, lui aussi).
    let record_diff = ctx.tests[0].diff.as_deref().expect("diff du record");
    assert_eq!(
        record_diff, diff,
        "le TestRecord doit porter le même diff borné que l'event"
    );
}

#[test]
fn run_one_crlf_rendu_visible_dans_le_diff() {
    // « Hello\r\n » attendu vs « Hello\n » obtenu : le \r est invisible
    // dans un diff brut (« -Hello\r\n » s'affiche comme « -Hello\n »)
    // — il est échappé en ␍ pour que l'écart saute aux yeux.
    let tmp = TempDir::new().unwrap();
    let (verdict, _outcome) = functional::run_one(
        Path::new("printf"),
        &["Hello\\n".to_string()],
        "",
        tmp.path(),
        "Hello\r\n",
        "",
        0,
        Duration::from_secs(2),
    );
    match verdict {
        TestVerdict::Failed { diff, .. } => {
            assert!(
                diff.contains("-Hello␍"),
                "marqueur ␍ absent de la ligne attendue : {diff:?}"
            );
            assert!(
                diff.contains("+Hello\n"),
                "la ligne obtenue (LF seul) reste sans marqueur : {diff:?}"
            );
        }
        other => panic!("CRLF vs LF attendu Failed : {other:?}"),
    }
}

#[test]
fn run_one_capture_tronquee_differente_verdict_tronque_direct() {
    // Capture tronquée ≠ attendu : verdict « sortie tronquée » DIRECT,
    // sans diff géant (4 Mio) sur des données partielles.
    let tmp = TempDir::new().unwrap();
    let start = Instant::now();
    let (verdict, outcome) = functional::run_one(
        Path::new("sh"),
        &[
            "-c".to_string(),
            "head -c 5000000 /dev/zero | tr '\\0' 'x'".to_string(),
        ],
        "",
        tmp.path(),
        "autre chose\n",
        "",
        0,
        Duration::from_secs(10),
    );
    assert!(outcome.stdout_truncated, "la capture doit être tronquée");
    assert!(
        start.elapsed() < Duration::from_secs(3),
        "pas de diff sur une capture tronquée : {:?}",
        start.elapsed()
    );
    match verdict {
        TestVerdict::Failed { diff, .. } => {
            assert!(
                diff.contains("tronquée"),
                "verdict « sortie tronquée » attendu : {diff:?}"
            );
            assert!(
                diff.len() < 10_000,
                "diff géant malgré la troncature : {} octets",
                diff.len()
            );
        }
        other => panic!("capture tronquée ≠ attendu : Failed attendu : {other:?}"),
    }
}

#[test]
fn binary_stdout_file_relatif_a_battery_root() {
    // stdout_file est relu depuis battery.root (pas depuis la salle
    // blanche ni le rendu) : chemin Ok de expected_stdout_of_test.
    let (_bat_dir, battery) = battery_avec_fichiers(
        r#"
[project]
name = "ok_project"
type = "binary"
binary = "ok_project"

[[functional_test]]
name = "hello_file"
stdout_file = "expected/hello.txt"
"#,
        &[("expected/hello.txt", "Hello, World!\n")],
    );

    let (ok, events, ctx) = run_binary_step(&battery, &fixture("ok_project"));

    assert!(ok, "stdout_file relu depuis battery.root : {events:?}");
    let tests = finished_tests(&events);
    assert_eq!(tests.len(), 1, "{tests:?}");
    assert!(
        matches!(tests[0].1, TestVerdict::Passed),
        "hello_file attendu Passed : {tests:?}"
    );
    assert_eq!(ctx.tests[0].verdict, "passed");
}

#[test]
fn binary_sans_stdout_ni_stdout_file_ko_erreur_batterie() {
    // Ni stdout inline ni stdout_file : erreur de batterie rapportée
    // comme un KO explicite du test (branche Err de
    // expected_stdout_of_test), jamais un vert à vide.
    let (_bat_dir, battery) = load_battery(
        r#"
[project]
name = "ok_project"
type = "binary"
binary = "ok_project"

[[functional_test]]
name = "sans_attendu"
"#,
    );

    let (ok, events, ctx) = run_binary_step(&battery, &fixture("ok_project"));

    assert!(!ok, "attendu manquant : le test est KO");
    let tests = finished_tests(&events);
    assert_eq!(tests.len(), 1, "{tests:?}");
    match &tests[0].1 {
        TestVerdict::Failed { diff, .. } => {
            assert!(
                diff.contains("ni `stdout` ni `stdout_file`"),
                "erreur « attendu manquant » absente du diff : {diff:?}"
            );
        }
        other => panic!("sans_attendu attendu Failed : {other:?}"),
    }
    assert_eq!(ctx.tests[0].verdict, "failed");
}

/// Batterie Functions my_print_a : la delivery appelle my_putchar,
/// défini dans l'extra_source — sans propagation au link, la
/// compilation échouerait.
const PRINT_A_TOML: &str = r#"
[project]
name = "fn_print_a"
type = "functions"

[[task]]
name = "my_print_a"
delivery = "my_print_a.c"
harness = "harness/main.c"
extra_sources = ["lib/my_putchar.c"]
stdout = "a\n"
"#;

const PRINT_A_HARNESS: &str = r#"void my_print_a(void);

int main(void)
{
    my_print_a();
    return 0;
}
"#;

/// Delivery appelant my_putchar (défini dans l'extra_source).
const PRINT_A_OK: &str = r#"void my_putchar(char c);

void my_print_a(void)
{
    my_putchar('a');
    my_putchar('\n');
}
"#;

/// Extra source façon my_putchar officiel (write __wur consommé :
/// -Werror l'exige).
const MY_PUTCHAR: &str = r#"
#include <unistd.h>

void my_putchar(char c)
{
    if (write(1, &c, 1) == -1)
        return;
}
"#;

#[test]
fn functions_extra_sources_propagées_au_link() {
    let (_bat_dir, battery) = battery_avec_fichiers(
        PRINT_A_TOML,
        &[
            ("harness/main.c", PRINT_A_HARNESS),
            ("lib/my_putchar.c", MY_PUTCHAR),
        ],
    );

    let (ok, events, ctx) = run_functions_step_delivery(&battery, "my_print_a.c", PRINT_A_OK);

    assert!(
        ok,
        "extra_sources liés au binaire de test, l'étape passe : {events:?}"
    );
    let tests = finished_tests(&events);
    assert!(
        matches!(tests[0].1, TestVerdict::Passed),
        "my_print_a attendu Passed : {tests:?}"
    );
    assert_eq!(ctx.tests[0].verdict, "passed");
}

/// Batterie Functions dont le projet porte des cflags custom : sans
/// propagation des cflags à la compile du test, MAGIC serait indéfini
/// et la compile échouerait.
const MAGIC_TOML: &str = r#"
[project]
name = "fn_magic"
type = "functions"
cflags = ["-Wall", "-Wextra", "-Werror", "-DMAGIC=55"]

[[task]]
name = "my_magic"
delivery = "my_magic.c"
harness = "harness/main.c"
stdout = "55\n"
"#;

const MAGIC_HARNESS: &str = r#"void my_magic(void);

int main(void)
{
    my_magic();
    return 0;
}
"#;

const MAGIC_OK: &str = r#"
#include <stdio.h>

void my_magic(void)
{
    printf("%d\n", MAGIC);
}
"#;

#[test]
fn functions_cflags_propagés_a_la_compile() {
    let (_bat_dir, battery) =
        battery_avec_fichiers(MAGIC_TOML, &[("harness/main.c", MAGIC_HARNESS)]);

    let (ok, events, ctx) = run_functions_step_delivery(&battery, "my_magic.c", MAGIC_OK);

    assert!(
        ok,
        "cflags (-DMAGIC=55) propagés à la compile, l'étape passe : {events:?}"
    );
    let tests = finished_tests(&events);
    assert!(
        matches!(tests[0].1, TestVerdict::Passed),
        "my_magic attendu Passed : {tests:?}"
    );
    assert_eq!(ctx.tests[0].verdict, "passed");
}

#[test]
fn run_one_stderr_non_vide_et_exit_non_zero_conformes_passed() {
    // stderr attendu non vide ET exit code non nul, tous deux
    // conformes au caractère près → Passed.
    let tmp = TempDir::new().unwrap();
    let (verdict, outcome) = functional::run_one(
        Path::new("sh"),
        &["-c".to_string(), "echo oops >&2; exit 3".to_string()],
        "",
        tmp.path(),
        "",
        "oops\n",
        3,
        Duration::from_secs(2),
    );
    assert!(
        matches!(verdict, TestVerdict::Passed),
        "stderr et exit code conformes attendus Passed : {verdict:?}"
    );
    assert!(matches!(outcome.status, ExecStatus::Exit(3)));
}

#[test]
fn functions_sans_task_etape_desactivee() {
    // Inatteignable via Battery::load (la validation exige au moins
    // une task) : batterie désérialisée directement (sans validate)
    // pour couvrir la branche défensive — Task 13 : Battery a un
    // champ privé (tempdir d'extraction), le struct literal ne
    // compile plus hors du crate.
    let battery: Battery =
        toml::from_str("[project]\nname = \"vide\"\ntype = \"functions\"\n").unwrap();
    let target = TempDir::new().unwrap();
    let mut ctx = common::test_ctx(&battery, target.path());
    let (tx, rx) = mpsc::channel();
    let ok = functional::run(&mut ctx, &tx);
    drop(tx);
    let events: Vec<Event> = rx.iter().collect();

    assert!(ok, "sans task : skip de config");
    let (ok_ev, skipped, _) = step_finished(&events);
    assert!(ok_ev && skipped, "désactivée : ok:true, skipped:true");
    assert!(ctx.tests.is_empty());
}

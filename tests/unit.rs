//! Tests de l'étape Unit (Task 8) : `make tests_run` en salle blanche,
//! parsing de la sortie criterion (`[OK]`/`[KO]`/`[FAIL]`, résumé
//! « Tests: X | Passing: Y | Failing: Z »), couverture gcovr.
//!
//! La fixture `unit_project` embarque un faux `tests_run` scripté
//! (sortie criterion-format déterministe, sans dépendance) ; les
//! scénarios KO/ANSI/vide réécrivent le script dans la copie en salle
//! blanche. Un test conditionnel vérifie le parsing contre un vrai
//! binaire criterion si `pkg-config --exists criterion` (même
//! convention de skip que `cc` dans tests/symbols.rs) ; idem pour
//! gcovr.
//!
//! L'étape est pilotée isolément : `ctx.build` est construit à la
//! main (salle blanche = tempdir contenant la fixture copiée), sans
//! passer par l'étape Build.

mod common;

use seeyou::battery::Battery;
use seeyou::engine::events::{Event, Step, TestVerdict};
use seeyou::engine::{unit, BuildArtifacts, PipelineContext};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc;
use tempfile::TempDir;

/// Batterie Binary avec la règle tests_run activée ou non.
fn battery_toml(tests_run_rule: bool) -> String {
    format!(
        r#"[project]
name = "unit_project"
type = "binary"
binary = "prog"
tests_run_rule = {tests_run_rule}
"#
    )
}

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

/// Copie la fixture unit_project (plate) dans un tempdir — la salle
/// blanche factice de l'étape — et repose l'exec bit du script.
fn stage_fixture() -> TempDir {
    let src = fixture("unit_project");
    let dir = TempDir::new().unwrap();
    for entry in fs::read_dir(&src).unwrap() {
        let entry = entry.unwrap();
        fs::copy(entry.path(), dir.path().join(entry.file_name())).unwrap();
    }
    let script = dir.path().join("tests_run.sh");
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    dir
}

/// Réécrit le `tests_run.sh` de la salle blanche (scénarios KO, ANSI,
/// exit≠0, vide…) — l'exec bit est reposé explicitement.
fn write_script(white: &Path, body: &str) {
    let path = white.join("tests_run.sh");
    fs::write(&path, body).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
}

/// Pose la salle blanche dans `ctx.build` et lance l'étape ; renvoie
/// le verdict, les événements et le ctx (qui garde la salle blanche
/// vivante le temps des assertions). Le `target` factice est droppé :
/// unit ne lit jamais `ctx.target` (elle travaille sur `ctx.build`).
fn run_unit(battery: &Battery, white: TempDir) -> (bool, Vec<Event>, PipelineContext) {
    let target = TempDir::new().unwrap();
    let mut ctx = common::test_ctx(battery, target.path());
    ctx.build = Some(BuildArtifacts {
        dir: white,
        binary: None,
    });
    let (tx, rx) = mpsc::channel();
    let ok = unit::run(&mut ctx, &tx);
    drop(tx);
    (ok, rx.iter().collect(), ctx)
}

/// Extrait les checks `(name, ok, detail)` du flux ; tout check doit
/// être estampillé Unit.
fn extract_checks(events: &[Event]) -> Vec<(String, bool, String)> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::CheckFinished {
                step,
                name,
                ok,
                detail,
            } => {
                assert_eq!(*step, Step::Unit, "check émis pour la mauvaise étape");
                Some((name.clone(), *ok, detail.clone()))
            }
            _ => None,
        })
        .collect()
}

/// Le `(ok, skipped, summary)` du StepFinished Unit — exactement un
/// par run (l'invariant de l'orchestrateur vaut aussi à l'isolement).
fn step_finished(events: &[Event]) -> (bool, bool, String) {
    let mut found = events.iter().filter_map(|e| match e {
        Event::StepFinished {
            step: Step::Unit,
            ok,
            skipped,
            summary,
        } => Some((*ok, *skipped, summary.clone())),
        _ => None,
    });
    let first = found.next().expect("StepFinished Unit émis");
    assert!(found.next().is_none(), "plusieurs StepFinished Unit");
    first
}

/// Tests terminés `(nom, verdict)`, dans l'ordre d'émission ; tout
/// test doit être dans le groupe « unit ».
fn finished_tests(events: &[Event]) -> Vec<(String, TestVerdict)> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::TestFinished {
                group,
                name,
                result,
            } => {
                assert_eq!(group, "unit", "groupe inattendu : {group}");
                Some((name.clone(), result.clone()))
            }
            _ => None,
        })
        .collect()
}

/// Séquence des événements de test `(true = Started, nom)` /
/// `(false = Finished, nom)` — pour vérifier l'entrelacement.
fn test_event_sequence(events: &[Event]) -> Vec<(bool, String)> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::TestStarted { group, name } => {
                assert_eq!(group, "unit", "groupe inattendu : {group}");
                Some((true, name.clone()))
            }
            Event::TestFinished { name, .. } => Some((false, name.clone())),
            _ => None,
        })
        .collect()
}

/// Lignes relayées en LogLine ; toutes estampillées Unit.
fn log_lines(events: &[Event]) -> Vec<&str> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::LogLine { step, line } => {
                assert_eq!(*step, Step::Unit, "LogLine pour la mauvaise étape");
                Some(line.as_str())
            }
            _ => None,
        })
        .collect()
}

/// `true` si `cc` est disponible (convention banana de
/// tests/prelim.rs).
fn cc_present() -> bool {
    Command::new("cc").arg("--version").output().is_ok()
}

/// `true` si criterion est installé (sonde pkg-config).
fn criterion_present() -> bool {
    Command::new("pkg-config")
        .args(["--exists", "criterion"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// `true` si gcovr est installé (sonde --version).
fn gcovr_present() -> bool {
    Command::new("gcovr").arg("--version").output().is_ok()
}

/// Noms des trois tests de la fixture, dans l'ordre de la sortie.
const FIXTURE_TESTS: [&str; 3] = ["tests::my_strlen", "tests::my_strcpy", "tests::my_strdup"];

#[test]
fn sans_regle_tests_run_etape_desactivee() {
    // Pas besoin de make : l'étape est désactivée avant tout spawn.
    let (_bat_dir, battery) = load_battery(&battery_toml(false));
    let (ok, events, ctx) = run_unit(&battery, stage_fixture());

    assert!(ok, "désactivée : l'étape réussit (skip de config)");
    assert!(matches!(
        events.first(),
        Some(Event::StepStarted {
            step: Step::Unit,
            ..
        })
    ));
    let (ok_ev, skipped, summary) = step_finished(&events);
    assert!(ok_ev, "skip de config : ok:true (pas un échec)");
    assert!(skipped, "skip de config : skipped:true");
    assert_eq!(summary, "skipped: pas de règle tests_run");
    assert!(
        extract_checks(&events).is_empty(),
        "aucun check quand l'étape est désactivée : {events:?}"
    );
    assert!(
        test_event_sequence(&events).is_empty(),
        "aucun test rapporté quand l'étape est désactivée : {events:?}"
    );
    // Le StepReport suit la même sémantique.
    let report = ctx.steps.last().expect("StepReport posé");
    assert_eq!(report.step, "unit");
    assert!(report.ok && report.skipped, "StepReport : {report:?}");
}

#[test]
fn tests_run_ok_trois_tests_passes() {
    let (_bat_dir, battery) = load_battery(&battery_toml(true));
    let (ok, events, ctx) = run_unit(&battery, stage_fixture());

    assert!(ok, "3 tests OK + exit 0 : l'étape passe : {events:?}");
    let tests = finished_tests(&events);
    assert_eq!(
        tests.len(),
        3,
        "trois tests parsés depuis la fixture : {tests:?}"
    );
    for (i, attendu) in FIXTURE_TESTS.iter().enumerate() {
        assert_eq!(&tests[i].0, attendu, "ordre de parsing");
        assert!(
            matches!(tests[i].1, TestVerdict::Passed),
            "{attendu} attendu Passed : {tests:?}"
        );
    }
    // Chaque TestStarted précède immédiatement son TestFinished.
    let attendu: Vec<(bool, String)> = FIXTURE_TESTS
        .iter()
        .flat_map(|n| [(true, n.to_string()), (false, n.to_string())])
        .collect();
    assert_eq!(test_event_sequence(&events), attendu);
    // ctx.tests alimenté pour le rapport.
    assert_eq!(ctx.tests.len(), 3, "TestRecords : {:?}", ctx.tests);
    for record in &ctx.tests {
        assert_eq!(record.group, "unit");
        assert_eq!(record.verdict, "passed");
        assert_eq!(record.diff, None);
    }
    // La sortie est relayée en LogLine (résumé criterion inclus).
    let logs = log_lines(&events);
    assert!(
        logs.iter()
            .any(|l| l.contains("Tests: 3 | Passing: 3 | Failing: 0")),
        "résumé criterion non relayé : {logs:?}"
    );
    // Sans .gcda dans la salle blanche : aucune trace de gcovr.
    assert!(
        !logs.iter().any(|l| l.contains("gcovr")),
        "gcovr ne doit pas tourner sans .gcda : {logs:?}"
    );
    // Checks OK, étape OK, StepReport cohérent.
    let checks = extract_checks(&events);
    assert!(
        checks.iter().all(|(_, ok, _)| *ok),
        "KO inattendu : {checks:?}"
    );
    let names: Vec<&str> = checks.iter().map(|(n, ..)| n.as_str()).collect();
    assert!(
        names.contains(&"tests_run") && names.contains(&"tests"),
        "checks tests_run/tests attendus : {checks:?}"
    );
    let (ok_ev, skipped, _) = step_finished(&events);
    assert!(ok_ev && !skipped, "étape OK, non skipped");
    let report = ctx.steps.last().expect("StepReport posé");
    assert_eq!(report.step, "unit");
    assert!(report.ok && !report.skipped, "StepReport : {report:?}");
}

#[test]
fn un_test_ko_etape_ko() {
    let (_bat_dir, battery) = load_battery(&battery_toml(true));
    let white = stage_fixture();
    // Un [KO] parmi les [OK] — exit 0 : c'est le parsing qui condamne.
    write_script(
        white.path(),
        "#!/bin/sh
echo 'tests::my_strlen: [OK]'
echo 'tests::my_strcpy: [KO]'
echo 'tests::my_strdup: [OK]'
echo 'Tests: 3 | Passing: 2 | Failing: 1'
exit 0
",
    );

    let (ok, events, ctx) = run_unit(&battery, white);
    assert!(!ok, "un test KO fait échouer l'étape");
    let tests = finished_tests(&events);
    assert_eq!(tests.len(), 3, "{tests:?}");
    assert!(matches!(tests[0].1, TestVerdict::Passed));
    // Criterion ne donne pas de diff stdout : champs vides, le détail
    // vit dans les LogLine.
    match &tests[1].1 {
        TestVerdict::Failed {
            diff,
            expected,
            got,
        } => {
            assert!(diff.is_empty() && expected.is_empty() && got.is_empty());
        }
        other => panic!("my_strcpy attendu Failed : {other:?}"),
    }
    assert!(matches!(tests[2].1, TestVerdict::Passed));
    assert_eq!(tests[1].0, "tests::my_strcpy");
    // Le TestRecord du KO est « failed » dans le rapport.
    let ko = ctx
        .tests
        .iter()
        .find(|t| t.name == "tests::my_strcpy")
        .expect("record my_strcpy");
    assert_eq!(ko.verdict, "failed");
    let (ok_ev, skipped, _) = step_finished(&events);
    assert!(!ok_ev && !skipped, "étape KO, non skipped");
}

#[test]
fn codes_ansi_strippes_avant_parsing() {
    let (_bat_dir, battery) = load_battery(&battery_toml(true));
    let white = stage_fixture();
    // Ligne entière colorée, marqueur seul coloré, résumé coloré.
    write_script(
        white.path(),
        "#!/bin/sh
printf '\\033[32mtests::ansi_green: [OK]\\033[0m\\n'
printf 'tests::ansi_marker: \\033[1;32m[OK]\\033[0m\\n'
printf '\\033[31mtests::ansi_red: [KO]\\033[0m\\n'
printf '\\033[1mTests: 3 | Passing: 2 | Failing: 1\\033[0m\\n'
exit 0
",
    );

    let (ok, events, _ctx) = run_unit(&battery, white);
    assert!(!ok, "un [KO] coloré reste un KO");
    let tests = finished_tests(&events);
    assert_eq!(tests.len(), 3, "ANSI strippés, trois tests : {tests:?}");
    assert_eq!(tests[0].0, "tests::ansi_green");
    assert_eq!(tests[1].0, "tests::ansi_marker");
    assert_eq!(tests[2].0, "tests::ansi_red");
    assert!(matches!(tests[0].1, TestVerdict::Passed));
    assert!(matches!(tests[1].1, TestVerdict::Passed));
    assert!(matches!(tests[2].1, TestVerdict::Failed { .. }));
    // Aucun résidu d'échappement dans les noms (les assertions
    // d'égalité exactes ci-dessus le garantissent déjà ; ceci est le
    // garde-fou explicite).
    for (name, _) in &tests {
        assert!(
            !name.contains('\x1b') && !name.contains('[') && !name.contains(']'),
            "résidu ANSI dans le nom : {name:?}"
        );
    }
}

#[test]
fn exit_non_zero_tests_quand_meme_rapportes() {
    let (_bat_dir, battery) = load_battery(&battery_toml(true));
    let white = stage_fixture();
    // Tous les tests passent mais la règle échoue (exit≠0) : KO étape,
    // tests rapportés quand même — la vraie moulinette montre les
    // tests passés même si la suite échoue.
    write_script(
        white.path(),
        "#!/bin/sh
echo 'tests::my_strlen: [OK]'
echo 'tests::my_strcpy: [OK]'
echo 'Tests: 2 | Passing: 2 | Failing: 0'
exit 1
",
    );

    let (ok, events, ctx) = run_unit(&battery, white);
    assert!(!ok, "exit≠0 : étape KO malgré les tests OK");
    let tests = finished_tests(&events);
    assert_eq!(tests.len(), 2, "tests parsés rapportés : {tests:?}");
    assert!(
        tests.iter().all(|(_, v)| matches!(v, TestVerdict::Passed)),
        "les deux tests sont Passed : {tests:?}"
    );
    assert_eq!(
        ctx.tests.len(),
        2,
        "TestRecords rapportés : {:?}",
        ctx.tests
    );
    // Le KO de la règle est un check explicite.
    let checks = extract_checks(&events);
    assert!(
        checks
            .iter()
            .any(|(n, ok, d)| n == "tests_run" && !*ok && d.contains("exit")),
        "check KO « tests_run » (exit) absent : {checks:?}"
    );
    let (ok_ev, skipped, _) = step_finished(&events);
    assert!(!ok_ev && !skipped);
}

#[test]
fn tests_run_vide_ko_sans_test_detecte() {
    let (_bat_dir, battery) = load_battery(&battery_toml(true));
    let white = stage_fixture();
    // Règle qui réussit sans émettre aucun test : un zero déguisé.
    write_script(white.path(), "#!/bin/sh\nexit 0\n");

    let (ok, events, _ctx) = run_unit(&battery, white);
    assert!(!ok, "un tests_run vide est un KO, pas un succès");
    assert!(
        finished_tests(&events).is_empty(),
        "aucun test à rapporter : {events:?}"
    );
    let checks = extract_checks(&events);
    assert!(
        checks
            .iter()
            .any(|(_, ok, d)| !*ok && d.contains("sans test détecté")),
        "check KO « sans test détecté » absent : {checks:?}"
    );
    let (ok_ev, skipped, _) = step_finished(&events);
    assert!(!ok_ev && !skipped);
}

#[test]
fn synthese_seule_non_verbose_etape_ok() {
    // Reproduction live : le vrai criterion SANS --verbose n'émet QUE
    // la ligne Synthesis pour une suite 100 % verte (aucun [PASS]) —
    // et tout sur stderr. Le résumé atteste 3 tests : pas un KO
    // « sans test détecté ».
    let (_bat_dir, battery) = load_battery(&battery_toml(true));
    let white = stage_fixture();
    write_script(
        white.path(),
        "#!/bin/sh
echo '[====] Synthesis: Tested: 3 | Passing: 3 | Failing: 0 | Crashing: 0 ' >&2
exit 0
",
    );

    let (ok, events, _ctx) = run_unit(&battery, white);
    assert!(
        ok,
        "suite 100 % verte non-verbose : l'étape passe : {events:?}"
    );
    assert!(
        finished_tests(&events).is_empty(),
        "aucun test individuel en non-verbose : {events:?}"
    );
    // Le verdict est conduit par le résumé : check récapitulatif OK.
    let checks = extract_checks(&events);
    assert!(
        checks
            .iter()
            .any(|(n, ok, d)| n == "tests" && *ok && d.contains("résumé")),
        "check récapitulatif conduit par le résumé absent : {checks:?}"
    );
    let (ok_ev, skipped, _) = step_finished(&events);
    assert!(ok_ev && !skipped, "étape OK, non skipped");
}

#[test]
fn synthese_seule_avec_echec_etape_ko() {
    // Contrepartie : Synthesis Failing: 1 + exit 1, toujours sans
    // tests individuels → KO conduit par le résumé (et par l'exit).
    let (_bat_dir, battery) = load_battery(&battery_toml(true));
    let white = stage_fixture();
    write_script(
        white.path(),
        "#!/bin/sh
echo '[====] Synthesis: Tested: 3 | Passing: 2 | Failing: 1 | Crashing: 0 ' >&2
exit 1
",
    );

    let (ok, events, _ctx) = run_unit(&battery, white);
    assert!(!ok, "résumé attestant un échec : étape KO");
    assert!(
        finished_tests(&events).is_empty(),
        "aucun test individuel en non-verbose : {events:?}"
    );
    let checks = extract_checks(&events);
    assert!(
        checks
            .iter()
            .any(|(n, ok, d)| n == "tests" && !*ok && d.contains("résumé")),
        "check KO conduit par le résumé absent : {checks:?}"
    );
    let (ok_ev, skipped, _) = step_finished(&events);
    assert!(!ok_ev && !skipped);
}

#[test]
fn crash_criterion_rapporte_crashed() {
    // Format réel vérifié (criterion 2.4.3) : « [FAIL] sample::segv:
    // CRASH! ». Le suffixe « CRASH! » est strippé du nom et le verdict
    // est Crashed(-1) — criterion ne donne pas le signal sur cette
    // ligne.
    let (_bat_dir, battery) = load_battery(&battery_toml(true));
    let white = stage_fixture();
    write_script(
        white.path(),
        "#!/bin/sh
echo '[----] crash.c:4: Unexpected signal caught below this line!' >&2
echo '[FAIL] sample::segv: CRASH!' >&2
echo '[====] Synthesis: Tested: 2 | Passing: 1 | Failing: 1 | Crashing: 1 ' >&2
exit 1
",
    );

    let (ok, events, ctx) = run_unit(&battery, white);
    assert!(!ok, "un crash : étape KO");
    let tests = finished_tests(&events);
    assert_eq!(tests.len(), 1, "un seul test parsé : {tests:?}");
    assert_eq!(
        tests[0].0, "sample::segv",
        "suffixe CRASH! strippé : {tests:?}"
    );
    assert!(
        matches!(tests[0].1, TestVerdict::Crashed(-1)),
        "verdict Crashed(-1) attendu : {tests:?}"
    );
    let record = ctx
        .tests
        .iter()
        .find(|t| t.name == "sample::segv")
        .expect("record sample::segv");
    assert_eq!(record.verdict, "crashed", "{record:?}");
}

#[test]
fn sortie_tronquee_ko() {
    // Plus de MAX_CAPTURE_BYTES (4 Mio) sur stdout : le parsing — seul
    // verdict quand la règle sort 0 — a pu perdre des lignes. KO
    // explicite, jamais un vert sur du partiel.
    let (_bat_dir, battery) = load_battery(&battery_toml(true));
    let white = stage_fixture();
    write_script(
        white.path(),
        "#!/bin/sh
echo 'tests::my_strlen: [OK]'
echo 'Tests: 1 | Passing: 1 | Failing: 0'
head -c 5000000 /dev/zero | tr '\\0' 'x'
exit 0
",
    );

    let (ok, events, _ctx) = run_unit(&battery, white);
    assert!(!ok, "sortie tronquée : KO malgré les tests verts");
    let checks = extract_checks(&events);
    assert!(
        checks
            .iter()
            .any(|(_, ok, d)| !*ok && d.contains("tronquée")),
        "check KO « sortie tronquée » absent : {checks:?}"
    );
}

#[test]
fn resume_critere_en_plus_des_tests_parses() {
    // Non-verbose avec échec : criterion n'émet que les [FAIL] — sans
    // enregistrement de vérité du résumé, le décompte global
    // rapporterait « 0/1 » alors que la suite est « 3 pass / 1 fail ».
    let (_bat_dir, battery) = load_battery(&battery_toml(true));
    let white = stage_fixture();
    write_script(
        white.path(),
        "#!/bin/sh
echo '[FAIL] sample::failing: (0.01s)' >&2
echo '[====] Synthesis: Tested: 4 | Passing: 3 | Failing: 1 | Crashing: 0 ' >&2
exit 1
",
    );

    let (ok, events, _ctx) = run_unit(&battery, white);
    assert!(!ok, "un échec : étape KO");
    let tests = finished_tests(&events);
    assert_eq!(tests.len(), 1, "seul le FAIL est parsé : {tests:?}");
    let checks = extract_checks(&events);
    assert!(
        checks
            .iter()
            .any(|(_, _, d)| d.contains("résumé criterion : 3 pass / 1 fail")),
        "enregistrement de vérité du résumé absent : {checks:?}"
    );
}

#[test]
fn couverture_jamais_bloquante() {
    let (_bat_dir, battery) = load_battery(&battery_toml(true));
    let white = stage_fixture();
    // Un .gcda pourri : si gcovr est installé il plantera dessus, sinon
    // la sonde échoue — dans les deux cas, jamais de blocage.
    fs::write(white.path().join("main.gcda"), b"pas du gcov data").unwrap();

    let (ok, events, _ctx) = run_unit(&battery, white);
    assert!(ok, "une erreur gcovr ne fait jamais échouer l'étape");
    let tests = finished_tests(&events);
    assert_eq!(tests.len(), 3, "{tests:?}");
    assert!(tests.iter().all(|(_, v)| matches!(v, TestVerdict::Passed)));
}

#[test]
fn criterion_reel_si_disponible() {
    if !cc_present() || !criterion_present() {
        eprintln!("cc ou criterion absent : test skippé");
        return;
    }
    let (_bat_dir, battery) = load_battery(&battery_toml(true));
    let white = stage_fixture();
    // Un vrai binaire criterion : 2 tests qui passent, 1 qui échoue.
    fs::write(
        white.path().join("real_tests.c"),
        "#include <criterion/criterion.h>\n\n\
         Test(sample, passing) { cr_assert_eq(1, 1); }\n\
         Test(sample, passing2) { cr_assert(1); }\n\
         Test(sample, failing) { cr_assert_eq(1, 2); }\n",
    )
    .unwrap();
    let flags = Command::new("pkg-config")
        .args(["--cflags", "--libs", "criterion"])
        .output()
        .expect("pkg-config criterion");
    let flags: Vec<String> = String::from_utf8_lossy(&flags.stdout)
        .split_whitespace()
        .map(String::from)
        .collect();
    let status = Command::new("cc")
        .args(["real_tests.c", "-o", "unit_tests"])
        .args(&flags)
        .current_dir(white.path())
        .status()
        .expect("lancement de cc impossible");
    assert!(status.success(), "compilation criterion impossible");
    // --verbose : le vrai criterion émet « [PASS]/[FAIL] name: (0.00s) ».
    write_script(white.path(), "#!/bin/sh\nexec ./unit_tests --verbose\n");

    let (ok, events, _ctx) = run_unit(&battery, white);
    assert!(!ok, "un vrai test criterion KO : étape KO");
    let mut tests = finished_tests(&events);
    tests.sort_by(|a, b| a.0.cmp(&b.0));
    let names: Vec<&str> = tests.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(
        names,
        vec!["sample::failing", "sample::passing", "sample::passing2"],
        "vrais noms criterion parsés : {tests:?}"
    );
    for (name, verdict) in &tests {
        if name == "sample::failing" {
            assert!(matches!(verdict, TestVerdict::Failed { .. }), "{tests:?}");
        } else {
            assert!(matches!(verdict, TestVerdict::Passed), "{tests:?}");
        }
    }
}

#[test]
fn couverture_gcovr_si_disponible() {
    if !cc_present() || !gcovr_present() {
        eprintln!("cc ou gcovr absent : test skippé");
        return;
    }
    let (_bat_dir, battery) = load_battery(&battery_toml(true));
    let white = stage_fixture();
    // Vraies données de couverture : compilation --coverage + run.
    let status = Command::new("cc")
        .args(["--coverage", "-c", "main.c", "-o", "main.o"])
        .current_dir(white.path())
        .status()
        .expect("lancement de cc impossible");
    assert!(status.success());
    let status = Command::new("cc")
        .args(["--coverage", "main.o", "-o", "prog"])
        .current_dir(white.path())
        .status()
        .expect("lancement de cc impossible");
    assert!(status.success());
    let status = Command::new("./prog")
        .current_dir(white.path())
        .status()
        .expect("lancement de prog impossible");
    assert!(status.success());
    assert!(
        white.path().join("main.gcda").is_file(),
        "le run instrumenté doit produire main.gcda"
    );

    let (ok, events, _ctx) = run_unit(&battery, white);
    assert!(ok, "la couverture ne change pas le verdict");
    let logs = log_lines(&events);
    // gcovr --print-summary produit « lines: 100.0% (…) » ; une erreur
    // gcovr serait relayée « gcovr: … » — l'un ou l'autre prouve que
    // la couverture a tourné.
    assert!(
        logs.iter()
            .any(|l| l.contains("lines:") || l.contains("gcovr")),
        "résumé gcovr attendu : {logs:?}"
    );
}

//! Tests de l'étape Prelim (Task 4) : fichiers interdits,
//! banana-check-repo, Makefile/règles (Binary), rendus/prototypes
//! (Functions).

mod common;

use seeyou::battery::Battery;
use seeyou::engine::events::{Event, Step};
use seeyou::engine::prelim;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use tempfile::TempDir;

/// Batterie Binary factice : règles par défaut (all/clean/fclean/re).
const BINARY_TOML: &str = r#"
[project]
name = "mini_binary"
type = "binary"
binary = "hello"
"#;

/// Batterie Functions factice : une task sans prototype.
const FUNCTIONS_TOML: &str = r#"
[project]
name = "mini_functions"
type = "functions"

[[task]]
name = "ex01"
delivery = "my_putchar.c"
harness = "harness/ex01_main.c"
stdout = ""
"#;

/// Batterie Functions avec un prototype attendu pour ex01.
const PROTOTYPE_TOML: &str = r#"
[project]
name = "mini_proto"
type = "functions"

[[task]]
name = "ex01"
delivery = "my_strlen.c"
harness = "harness/ex01_main.c"
prototype = "int my_strlen(char const *str)"
stdout = ""
"#;

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

/// Exécute l'étape prelim sur `target` et collecte tous les événements.
fn run_prelim(battery: &Battery, target: &Path) -> (bool, Vec<Event>) {
    let mut ctx = common::test_ctx(battery, target);
    let (tx, rx) = mpsc::channel();
    let ok = prelim::run(&mut ctx, &tx);
    drop(tx);
    let events: Vec<Event> = rx.iter().collect();
    (ok, events)
}

/// Extrait les checks `(name, ok, detail)` du flux ; tout check doit
/// être estampillé Prelim.
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
                assert_eq!(*step, Step::Prelim, "check émis pour la mauvaise étape");
                Some((name.clone(), *ok, detail.clone()))
            }
            _ => None,
        })
        .collect()
}

/// Détails des checks en échec (pour les messages d'assert).
fn failed_details(checks: &[(String, bool, String)]) -> Vec<&str> {
    checks
        .iter()
        .filter(|(_, ok, _)| !*ok)
        .map(|(_, _, d)| d.as_str())
        .collect()
}

#[test]
fn clean_repo_binary_etape_ok() {
    let (_bat_dir, battery) = load_battery(BINARY_TOML);
    let (ok, events) = run_prelim(&battery, &fixture("clean_repo"));

    assert!(ok, "prelim devrait passer sur clean_repo");
    // L'étape est correctement encadrée.
    assert!(matches!(
        events.first(),
        Some(Event::StepStarted {
            step: Step::Prelim,
            ..
        })
    ));
    assert!(events.iter().any(|e| matches!(
        e,
        Event::StepFinished {
            step: Step::Prelim,
            ok: true,
            skipped: false,
            ..
        }
    )));

    let checks = extract_checks(&events);
    assert!(!checks.is_empty(), "aucun check émis");
    assert!(
        checks.iter().all(|(_, ok, _)| *ok),
        "checks en échec sur un repo propre : {checks:?}"
    );

    // Les 4 règles make par défaut sont vérifiées, toutes OK.
    for rule in ["all", "clean", "fclean", "re"] {
        let want = format!("rule {rule} present");
        assert!(
            checks
                .iter()
                .any(|(name, ok, detail)| *ok && name == "makefile rules" && *detail == want),
            "check OK manquant pour la règle {rule} : {checks:?}"
        );
    }
    // Présence du Makefile et fichiers interdits : un check OK chacun.
    assert!(checks.iter().any(|(name, ok, _)| *ok && name == "makefile"));
    assert!(checks
        .iter()
        .any(|(name, ok, _)| *ok && name == "forbidden files"));
}

#[test]
fn dirty_repo_fichiers_interdits_et_regle_manquante() {
    let (_bat_dir, battery) = load_battery(BINARY_TOML);
    let (ok, events) = run_prelim(&battery, &fixture("dirty_repo"));

    assert!(!ok, "prelim devrait échouer sur dirty_repo");
    let checks = extract_checks(&events);
    let failed = failed_details(&checks);

    assert!(
        failed
            .iter()
            .any(|d| d.contains("./truc~") && d.contains("unwanted file (C-O1)")),
        "truc~ non signalé : {checks:?}"
    );
    assert!(
        failed.iter().any(|d| d.contains("./src/#save.c#")),
        "src/#save.c# non signalé : {checks:?}"
    );
    assert!(
        failed
            .iter()
            .any(|d| d.contains("missing makefile rule: re")),
        "règle re manquante non signalée : {checks:?}"
    );
}

#[test]
fn binary_sans_makefile_regles_non_sondees() {
    let (_bat_dir, battery) = load_battery(BINARY_TOML);
    let target = TempDir::new().unwrap(); // repo vide : pas de Makefile

    let (ok, events) = run_prelim(&battery, target.path());
    assert!(!ok);
    let checks = extract_checks(&events);
    assert!(
        checks
            .iter()
            .any(|(_, ok, d)| !*ok && d.contains("missing Makefile")),
        "Makefile manquant non signalé : {checks:?}"
    );
    // Sans Makefile, inutile de sonder les règles (bruit garanti).
    assert!(
        !checks.iter().any(|(name, _, _)| name == "makefile rules"),
        "règles sondées sans Makefile : {checks:?}"
    );
}

#[test]
fn functions_delivery_manquante() {
    let (_bat_dir, battery) = load_battery(FUNCTIONS_TOML);
    let target = TempDir::new().unwrap(); // repo vide : pas de my_putchar.c

    let (ok, events) = run_prelim(&battery, target.path());
    assert!(!ok);
    let checks = extract_checks(&events);
    assert!(
        checks
            .iter()
            .any(|(_, ok, d)| !*ok && d.contains("missing delivery: my_putchar.c")),
        "delivery manquante non signalée : {checks:?}"
    );
}

#[test]
fn functions_prototype_match_malgré_espaces_exotiques() {
    let (_bat_dir, battery) = load_battery(PROTOTYPE_TOML);
    let target = TempDir::new().unwrap();
    // Sauts de ligne, tabulations et espaces multiples : la
    // normalisation doit les gommer avant la comparaison.
    fs::write(
        target.path().join("my_strlen.c"),
        "int\n   my_strlen(char  const\t*str)\n{\n    return 0;\n}\n",
    )
    .unwrap();

    let (ok, events) = run_prelim(&battery, target.path());
    let checks = extract_checks(&events);
    assert!(ok, "prototype présent mais étape KO : {checks:?}");
    assert!(checks
        .iter()
        .any(|(name, ok, _)| name == "prototype" && *ok));
}

#[test]
fn functions_prototype_mismatch() {
    let (_bat_dir, battery) = load_battery(PROTOTYPE_TOML);
    let target = TempDir::new().unwrap();
    // Signature différente : char *str au lieu de char const *str.
    fs::write(
        target.path().join("my_strlen.c"),
        "int my_strlen(char *str)\n{\n    return 0;\n}\n",
    )
    .unwrap();

    let (ok, events) = run_prelim(&battery, target.path());
    assert!(!ok);
    let checks = extract_checks(&events);
    assert!(
        checks
            .iter()
            .any(|(_, ok, d)| !*ok && d.contains("prototype mismatch: ex01")),
        "mismatch non signalé : {checks:?}"
    );
}

#[test]
fn banana_check_repo_signale_chaque_ligne() {
    // Test conditionnel : si banana-check-repo n'est pas installé, on
    // skip proprement (le skip silencieux est lui-même le comportement
    // attendu de l'étape dans ce cas).
    if std::process::Command::new("banana-check-repo")
        .arg("--version")
        .output()
        .is_err()
    {
        eprintln!("banana-check-repo absent du PATH : test skippé");
        return;
    }

    let (_bat_dir, battery) = load_battery(BINARY_TOML);
    let (ok, events) = run_prelim(&battery, &fixture("dirty_repo"));
    assert!(!ok);
    let checks = extract_checks(&events);
    let banana: Vec<&(String, bool, String)> = checks
        .iter()
        .filter(|(name, _, _)| name == "banana-check-repo")
        .collect();
    assert!(
        !banana.is_empty(),
        "banana installé mais aucun check émis : {checks:?}"
    );
    // Chaque infraction est une ligne complète au format banana.
    assert!(
        banana.iter().any(|(_, ok, d)| !*ok
            && d.starts_with("./truc~: [Banana] [Major]")
            && d.contains("unwanted file (C-O1)")),
        "ligne truc~ absente : {banana:?}"
    );
    assert!(
        banana
            .iter()
            .any(|(_, ok, d)| !*ok && d.starts_with("./BadName.c: [Banana]") && d.contains("C-O4")),
        "ligne BadName.c (C-O4) absente : {banana:?}"
    );
    // La ligne de service n'est jamais parsée comme une faute.
    assert!(
        banana
            .iter()
            .all(|(_, _, d)| !d.contains("Checking delivery files")),
        "ligne de service parsée comme faute : {banana:?}"
    );

    // Sur un repo propre : un check banana OK.
    let (ok, events) = run_prelim(&battery, &fixture("clean_repo"));
    assert!(ok);
    let checks = extract_checks(&events);
    assert!(
        checks
            .iter()
            .any(|(name, ok, _)| name == "banana-check-repo" && *ok),
        "check banana OK absent sur clean_repo : {checks:?}"
    );
}

#[test]
fn bad_makefile_regles_ko_et_court_circuit() {
    let (_bat_dir, battery) = load_battery(BINARY_TOML);
    let (ok, events) = run_prelim(&battery, &fixture("bad_makefile"));

    assert!(!ok, "prelim devrait échouer sur un Makefile invalide");
    let checks = extract_checks(&events);
    // Le Makefile est bien présent : son check reste OK.
    assert!(checks.iter().any(|(name, ok, _)| name == "makefile" && *ok));
    let rules: Vec<&(String, bool, String)> = checks
        .iter()
        .filter(|(name, _, _)| name == "makefile rules")
        .collect();
    // Aucune règle ne peut passer OK : le Makefile ne parse pas.
    assert!(
        !rules.is_empty() && rules.iter().all(|(_, ok, _)| !*ok),
        "une règle passe OK sur un Makefile invalide : {checks:?}"
    );
    // La première sonde rapporte l'erreur de parse avec un extrait de
    // stderr (LC_ALL=C : le message « missing separator » est stable).
    assert!(
        rules
            .iter()
            .any(|(_, _, d)| d.starts_with("makefile invalide:") && d.contains("missing separator")),
        "détail 'makefile invalide' absent ou sans extrait : {rules:?}"
    );
    // Court-circuit : 1 KO de parse + 1 KO global « makefile
    // illisible », pas une sonde par règle (qui paieraient chacune le
    // timeout pour le même verdict).
    assert_eq!(
        rules.len(),
        2,
        "les règles restantes ne devraient pas être sondées : {rules:?}"
    );
    let global = &rules[1].2;
    assert!(
        global.contains("makefile illisible")
            && global.contains("clean")
            && global.contains("fclean")
            && global.contains("re"),
        "KO global 'makefile illisible' absent ou incomplet : {rules:?}"
    );
}

#[test]
fn slow_makefile_timeout_ko_et_court_circuit() {
    let (_bat_dir, battery) = load_battery(BINARY_TOML);
    // $(shell sleep 60) à l'évaluation : chaque sonde paierait le
    // timeout (10 s) — le court-circuit borne le coût à une sonde.
    let (ok, events) = run_prelim(&battery, &fixture("slow_makefile"));

    assert!(!ok, "un Makefile dont l'évaluation bloque doit être KO");
    let checks = extract_checks(&events);
    let rules: Vec<&(String, bool, String)> = checks
        .iter()
        .filter(|(name, _, _)| name == "makefile rules")
        .collect();
    assert!(
        rules.iter().all(|(_, ok, _)| !*ok),
        "une règle passe OK malgré le timeout : {checks:?}"
    );
    assert!(
        rules
            .iter()
            .any(|(_, _, d)| d.contains("make timeout sur la règle all")),
        "détail 'make timeout' absent : {rules:?}"
    );
    assert_eq!(
        rules.len(),
        2,
        "timeout non court-circuité (4 sondes × 10 s) : {rules:?}"
    );
}

#[test]
fn prerequis_irresoluble_n_est_pas_regle_manquante() {
    let (_bat_dir, battery) = load_battery(BINARY_TOML);
    let (ok, events) = run_prelim(&battery, &fixture("missing_prereq"));

    assert!(!ok, "un prérequis sans règle doit faire échouer prelim");
    let checks = extract_checks(&events);
    // « No rule to make target 'dep.txt', needed by 'all' » : la cible
    // entre guillemets est un prérequis, pas la règle sondée — all et
    // re (qui dépend de all) existent bel et bien.
    for rule in ["all", "re"] {
        let want = format!("prérequis irrésoluble: dep.txt (règle {rule})");
        assert!(
            checks
                .iter()
                .any(|(name, ok, d)| name == "makefile rules" && !*ok && *d == want),
            "prérequis de {rule} absent ou mal classifié : {checks:?}"
        );
    }
    assert!(
        !checks
            .iter()
            .any(|(_, _, d)| d.contains("missing makefile rule")),
        "prérequis confondu avec une règle manquante : {checks:?}"
    );
    // clean et fclean ne dépendent pas de dep.txt : OK — et leur
    // présence prouve qu'un prérequis irrésoluble ne court-circuite
    // pas les sondes suivantes.
    for rule in ["clean", "fclean"] {
        let want = format!("rule {rule} present");
        assert!(
            checks
                .iter()
                .any(|(name, ok, d)| name == "makefile rules" && *ok && *d == want),
            "règle {rule} devrait passer OK : {checks:?}"
        );
    }
}

#[test]
fn functions_delivery_illisible_sans_prototype() {
    let (_bat_dir, battery) = load_battery(FUNCTIONS_TOML); // ex01 sans prototype
    let target = TempDir::new().unwrap();
    let path = target.path().join("my_putchar.c");
    fs::write(&path, "void my_putchar(char c);\n").unwrap();
    // chmod 000 : présent mais illisible → KO même sans prototype à
    // vérifier (sinon un rendu vide de droits passerait inaperçu).
    fs::set_permissions(&path, fs::Permissions::from_mode(0o000)).unwrap();

    let (ok, events) = run_prelim(&battery, target.path());
    assert!(!ok, "une delivery illisible devrait faire échouer prelim");
    let checks = extract_checks(&events);
    assert!(
        checks.iter().any(|(name, ok, d)| name == "delivery"
            && !*ok
            && d.contains("my_putchar.c")
            && d.contains("unreadable")),
        "delivery illisible non signalée : {checks:?}"
    );
}

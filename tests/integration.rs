//! Tests d'intégration end-to-end du pipeline complet (Task 14).
//!
//! [`run_pipeline`] est pilotée directement (le CLI est déjà couvert
//! par cli.rs) sur des fixtures versionnées ; le rapport JSON relu
//! depuis le data dir isolé EST le livrable asserté — pas seulement
//! la structure [`Report`] retournée (déjà couverte par day03.rs) :
//! - `day03_solution_ok` → `scores.global == 100.0` dans le JSON ;
//! - `day03_solution_ko` → global < 100 et ≥ 2 tests « failed » ;
//! - `ok_project` (fixture Task 5) avec une batterie Binary +
//!   functional_test → run Binary end-to-end à 100.0.
//!
//! Helpers partagés : [`common`] (batterie embarquée, run isolé XDG).

use all_seeing_eye::report::Report;
use tempfile::TempDir;

mod common;

use common::{batterie_day03, batterie_toml, fixture, run_pipeline_isole};

/// Batterie Binary adossée à la fixture ok_project : un test
/// fonctionnel vérifiant la sortie hello, au caractère près.
const BINARY_TOML: &str = r#"
[project]
name = "ok_project"
type = "binary"
binary = "ok_project"

[[functional_test]]
name = "hello"
stdout = "Hello, World!\n"
"#;

/// Relit le `last.json` sauvegardé par le verdict dans le data dir
/// isolé du run — le rapport JSON est un livrable à part entière.
fn lire_last_json(data: &TempDir) -> serde_json::Value {
    let path = data.path().join("all-seeing-eye").join("last.json");
    let contenu = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("last.json illisible ({}) : {e}", path.display()));
    // Le second livrable doit exister à côté.
    let txt = data.path().join("all-seeing-eye").join("last.txt");
    assert!(txt.exists(), "last.txt manquant ({})", txt.display());
    serde_json::from_str(&contenu).expect("last.json doit être du JSON valide")
}

/// Les 7 étapes du rapport JSON, toutes `ok` (les étapes désactivées
/// ont `ok: true` — jamais un vert à vide, cf. engine::step_disabled).
fn assert_etapes_ok(report: &Report) {
    assert_eq!(
        report.steps.len(),
        7,
        "7 étapes attendues : {:?}",
        report.steps
    );
    assert!(
        report.steps.iter().all(|s| s.ok),
        "toutes les étapes doivent être OK : {:?}",
        report.steps
    );
}

#[test]
fn day03_solution_ok_rapport_json_global_100() {
    let (data, report) = run_pipeline_isole(&batterie_day03(), &fixture("day03_solution_ok"));

    assert_eq!(report.scores.global, 100.0, "{:?}", report.scores);
    assert_etapes_ok(&report);

    // Le JSON sauvegardé porte le même verdict : 100.0 global, 8
    // tests, tous « passed ».
    let json = lire_last_json(&data);
    assert_eq!(
        json["scores"]["global"].as_f64(),
        Some(100.0),
        "scores.global du JSON : {}",
        json["scores"]
    );
    let tests = json["tests"].as_array().expect("tests est un tableau");
    assert_eq!(tests.len(), 8, "8 tasks attendues : {tests:?}");
    assert!(
        tests.iter().all(|t| t["verdict"] == "passed"),
        "8/8 passed attendus : {tests:?}"
    );
}

#[test]
fn day03_solution_ko_global_sous_100_et_au_moins_2_failed() {
    let (data, report) = run_pipeline_isole(&batterie_day03(), &fixture("day03_solution_ko"));

    assert!(
        report.scores.global < 100.0,
        "global doit être < 100 : {:?}",
        report.scores
    );

    // Dans le JSON : global < 100 et au moins 2 tests « failed »
    // (my_print_alpha sans le 'z' final, my_put_nbr sur INT_MIN).
    let json = lire_last_json(&data);
    let global = json["scores"]["global"]
        .as_f64()
        .expect("scores.global est un nombre");
    assert!(global < 100.0, "global du JSON doit être < 100 : {global}");
    let failed = json["tests"]
        .as_array()
        .expect("tests est un tableau")
        .iter()
        .filter(|t| t["verdict"] == "failed")
        .count();
    assert!(failed >= 2, "au moins 2 failed dans le JSON : {failed}");
}

#[test]
fn binary_ok_project_end_to_end_global_100() {
    // Batterie Binary : build par make (fclean/re), puis le binaire
    // produit est exécuté en salle blanche et sa sortie comparée au
    // caractère près — le chemin Binary complet, jamais couvert
    // end-to-end ailleurs (functional.rs pilote l'étape isolément).
    let (_bat_dir, battery) = batterie_toml(BINARY_TOML);
    let (_data, report) = run_pipeline_isole(&battery, &fixture("ok_project"));

    assert_eq!(report.scores.global, 100.0, "{:?}", report.scores);
    assert_etapes_ok(&report);
    // Le test fonctionnel a réellement tourné (pas un 100 % vacuoleux).
    let hello = report
        .tests
        .iter()
        .find(|t| t.name == "hello")
        .expect("le test hello doit être enregistré");
    assert_eq!(hello.group, "functional");
    assert_eq!(hello.verdict, "passed", "{hello:?}");
}

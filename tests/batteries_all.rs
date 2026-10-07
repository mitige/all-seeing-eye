//! Intégration globale : CHAQUE batterie embarquée est éprouvée contre
//! sa solution de référence (tests/fixtures/<nom>_solution_ok/) et doit
//! scorer 100.0 — la preuve que la collection entière est fonctionnelle.
//!
//! Day13 (CSFML) est conditionnelle : sautée proprement si la lib est
//! absente de la machine.

mod common;

use common::{run_pipeline_isole, fixture};
use all_seeing_eye::battery::Battery;

/// Batteries éprouvées end-to-end : (nom embarqué, fixture).
const BATTERIES: &[(&str, &str)] = &[
    ("cpool_day03", "day03_solution_ok"),
    ("cpool_day04", "cpool_day04_solution_ok"),
    ("cpool_day05", "cpool_day05_solution_ok"),
    ("cpool_day06", "cpool_day06_solution_ok"),
    ("cpool_day07", "cpool_day07_solution_ok"),
    ("cpool_day08", "cpool_day08_solution_ok"),
    ("cpool_day09", "cpool_day09_solution_ok"),
    ("cpool_day10", "cpool_day10_solution_ok"),
    ("cpool_day11", "cpool_day11_solution_ok"),
    ("cpool_day12", "cpool_day12_solution_ok"),
    ("cpool_rush1", "cpool_rush1_solution_ok"),
    ("cpool_star", "cpool_star_solution_ok"),
    ("cpool_workshoplib", "cpool_workshoplib_solution_ok"),
    ("cpool_countisland", "cpool_countisland_solution_ok"),
    ("cpool_rush2", "cpool_rush2_solution_ok"),
];

fn batterie(nom: &str) -> Battery {
    all_seeing_eye::battery::load_embedded(nom)
        .unwrap_or_else(|e| panic!("batterie embarquée {nom} : {e:#}"))
        .unwrap_or_else(|| panic!("batterie embarquée {nom} introuvable"))
}

#[test]
fn toutes_les_batteries_scorcent_100_sur_leurs_solutions() {
    let mut details = Vec::new();
    for (nom, fixture_name) in BATTERIES {
        let fix = fixture(fixture_name);
        if !fix.is_dir() {
            panic!("fixture manquante pour {nom} : {}", fix.display());
        }
        let (_data, report) = run_pipeline_isole(&batterie(nom), &fix);
        details.push(format!(
            "{nom}: {} ({} tests)",
            report.scores.global,
            report.tests.len()
        ));
        assert_eq!(
            report.scores.global, 100.0,
            "{nom} doit scorer 100.0 sur sa solution de référence : {:#?}",
            report.scores
        );
        assert!(
            !report.tests.is_empty(),
            "{nom} : aucun test exécuté — batterie vacuoleuse ?"
        );
    }
    eprintln!("batteries éprouvées :\n{}", details.join("\n"));
}

/// Day13 (CSFML) : éprouvée si la lib est présente, sautée sinon —
/// l'échec propre sans CSFML est couvert par les tests de la batterie.
#[test]
fn day13_csdfml_conditionnelle() {
    let csfml = std::process::Command::new("pkg-config")
        .args(["--exists", "csfml-graphics"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !csfml {
        eprintln!("csfml absent : cpool_day13 skippée (batterie livrée, éprouvée sur dump)");
        return;
    }
    let fix = fixture("cpool_day13_solution_ok");
    let (_data, report) = run_pipeline_isole(&batterie("cpool_day13"), &fix);
    assert_eq!(report.scores.global, 100.0, "{:#?}", report.scores);
}

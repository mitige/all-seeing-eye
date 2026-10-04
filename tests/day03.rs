//! Tests E2E de la batterie embarquée cpool_day03 (Task 13).
//!
//! La batterie est chargée via [`battery::load_embedded`] (extraction
//! du contenu embarqué dans un TempDir détenu par la batterie), puis le
//! pipeline complet tourne sur les fixtures :
//! - `day03_solution_ok` : les 8 fonctions de référence → 100.0 ;
//! - `day03_solution_ko` : my_print_alpha sans le 'z' final et
//!   my_put_nbr qui déborde sur INT_MIN → global < 100, au moins 2
//!   failed avec diff.
//!
//! Les deux fixtures sont volontairement norme-clean (modulo la C-H1
//! structurelle : le sujet impose de déclarer `my_putchar` dans chaque
//! fichier sans jamais le rendre) pour que seul le comportement
//! fonctionnel diffère.

use all_seeing_eye::battery::{self, Battery};
use all_seeing_eye::engine::{run_pipeline, RunOpts};
use all_seeing_eye::norme::official;
use all_seeing_eye::report::Report;
use std::path::PathBuf;
use std::sync::mpsc;
use tempfile::TempDir;

mod common;

use common::{XdgGuard, XDG_MUTEX};

/// La batterie embarquée, extraite et chargée.
fn batterie_day03() -> Battery {
    battery::load_embedded("cpool_day03")
        .unwrap()
        .expect("cpool_day03 embarquée")
}

/// Options de test : cc partout (portable, avec ou sans epiclang).
fn test_opts() -> RunOpts {
    RunOpts {
        strict_norme: false,
        use_epiclang: false,
        compiler: PathBuf::from("cc"),
    }
}

/// Pipeline complet sur `tests/fixtures/<fixture>`, rapport retourné.
/// Le verdict sauvegarde le rapport : data dir isolé, jamais celui de
/// l'utilisateur (XDG_MUTEX, comme pipeline.rs).
fn run_sur_fixture(fixture: &str) -> Report {
    let _lock = XDG_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
    let data = TempDir::new().unwrap();
    let _xdg = XdgGuard::set("XDG_DATA_HOME", data.path());
    let target = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(fixture);
    let (tx, _rx) = mpsc::channel();
    run_pipeline(&batterie_day03(), &target, tx, &test_opts())
}

#[test]
fn pipeline_day03_solution_ok_score_100() {
    let report = run_sur_fixture("day03_solution_ok");
    assert_eq!(
        report.scores.global, 100.0,
        "global attendu 100.0 : {:?}",
        report.scores
    );
    assert!(
        report.steps.iter().all(|s| s.ok),
        "toutes les étapes doivent être OK : {:?}",
        report.steps
    );
    assert_eq!(report.tests.len(), 8, "8 tasks attendues");
    assert!(
        report.tests.iter().all(|t| t.verdict == "passed"),
        "8/8 passed attendus : {:?}",
        report.tests
    );
}

#[test]
fn pipeline_day03_solution_ko_failed_avec_diffs_et_norme_active() {
    let report = run_sur_fixture("day03_solution_ko");

    // 6/8 passed → global 75.0 (< 100).
    assert!(
        report.scores.global < 100.0,
        "global doit être < 100 : {:?}",
        report.scores
    );
    assert_eq!(
        report
            .tests
            .iter()
            .filter(|t| t.verdict == "passed")
            .count(),
        6,
        "6 tasks correctes attendues : {:?}",
        report.tests
    );

    // Au moins 2 failed, dont my_print_alpha et my_put_nbr.
    let failed: Vec<_> = report
        .tests
        .iter()
        .filter(|t| t.verdict == "failed")
        .collect();
    assert!(failed.len() >= 2, "au moins 2 failed : {:?}", report.tests);
    let alpha = failed
        .iter()
        .find(|t| t.name == "my_print_alpha")
        .expect("my_print_alpha doit être failed");
    let diff = alpha.diff.as_deref().expect("diff de my_print_alpha");
    assert!(
        diff.contains("-abcdefghijklmnopqrstuvwxyz"),
        "attendu absent du diff : {diff}"
    );
    assert!(
        diff.contains("+abcdefghijklmnopqrstuvwxy"),
        "obtenu (sans z) absent du diff : {diff}"
    );
    let nbr = failed
        .iter()
        .find(|t| t.name == "my_put_nbr")
        .expect("my_put_nbr doit être failed");
    let diff = nbr.diff.as_deref().expect("diff de my_put_nbr");
    assert!(
        diff.contains("-2147483648"),
        "INT_MIN attendu absent du diff : {diff}"
    );

    // La norme a tourné (ni KO, ni skipped) ; avec epiclang, la C-H1
    // structurelle des deliveries est remontée.
    let norme = report
        .steps
        .iter()
        .find(|s| s.step == "norme")
        .expect("étape norme présente");
    assert!(
        norme.ok && !norme.skipped,
        "l'étape norme doit avoir tourné : {norme:?}"
    );
    if official::epiclang_available() {
        assert!(
            report.norme.iter().any(|f| f.rule == "C-H1"),
            "C-H1 structurelle attendue sous epiclang : {:?}",
            report.norme
        );
    }
}

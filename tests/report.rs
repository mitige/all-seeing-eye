//! Tests du rapport final : scores, rendu texte (golden), export
//! JSON/TXT (Task 10).

use seeyou::norme::{NormeFault, Severity};
use seeyou::report::{
    compute_scores, render_text, render_text_at, save, Report, Scores, StepReport, TestRecord,
};
use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use tempfile::TempDir;

mod common;

use common::{XdgGuard, XDG_MUTEX};

/// Positionne `XDG_DATA_HOME` (process-global — les tests qui y
/// touchent se sérialisent sur [`XDG_MUTEX`]) et le restaure à sa
/// valeur initiale au drop.
fn xdg_data(path: &std::path::Path) -> XdgGuard {
    XdgGuard::set("XDG_DATA_HOME", path)
}

/// StepReport compact pour les rapports construits à la main.
fn sr(step: &str, ok: bool, skipped: bool, summary: &str) -> StepReport {
    StepReport {
        step: step.to_string(),
        ok,
        skipped,
        summary: summary.to_string(),
        checks: Vec::new(),
    }
}

/// TestRecord compact pour les rapports construits à la main.
fn rec(
    group: &str,
    name: &str,
    verdict: &str,
    diff: Option<&str>,
    detail: Option<&str>,
) -> TestRecord {
    TestRecord {
        group: group.to_string(),
        name: name.to_string(),
        verdict: verdict.to_string(),
        diff: diff.map(str::to_string),
        detail: detail.map(str::to_string),
    }
}

/// Une faute de norme quelconque à la sévérité demandée.
fn faute(severity: Severity) -> NormeFault {
    NormeFault {
        file: PathBuf::from("main.c"),
        line: 1,
        col: 1,
        severity,
        rule: "C-X0".to_string(),
        message: "faute de test".to_string(),
    }
}

/// Rapport « mini_ls » construit à la main : 6 étapes + verdict (2 OK
/// visibles hors norme/symboles, unit désactivée → skipped), 3 tests
/// fonctionnels (1 passed, 1 failed, 1 crashed), 3 major + 12 minor.
/// Les scores sont cohérents avec ce que `compute_scores` produirait :
/// groupe "functional" attendu seul, 1/3 → 33.3 %, global 33.3.
fn report_golden() -> Report {
    let mut norme = Vec::new();
    for _ in 0..3 {
        norme.push(faute(Severity::Major));
    }
    for _ in 0..12 {
        norme.push(faute(Severity::Minor));
    }
    Report {
        project: "mini_ls".to_string(),
        steps: vec![
            sr("prelim", true, false, "4 checks ok"),
            sr("build", true, false, "make ok"),
            sr("norme", true, false, "3 major, 12 minor"),
            sr("symbols", true, false, "aucun symbole interdit"),
            sr("unit", true, true, "skipped: pas de règle tests_run"),
            sr("functional", false, false, "3 tests, 2 en échec"),
            sr("verdict", true, false, "score global: 33.3%"),
        ],
        tests: vec![
            rec("functional", "empty_dir", "passed", None, None),
            rec(
                "functional",
                "basic_ls",
                "failed",
                Some("-file_a\n+file_b\n"),
                None,
            ),
            rec("functional", "big_dir", "crashed", None, Some("SIGSEGV")),
        ],
        norme,
        scores: Scores {
            par_groupe: vec![("functional".to_string(), 33.3)],
            global: 33.3,
            norme_fatal: 0,
            norme_major: 3,
            norme_minor: 12,
            norme_info: 0,
        },
        duration_secs: 12.34,
    }
}

/// 2026-10-03 23:42:00 UTC (vérifié avec `date -u`).
const INSTANT_GOLDEN: u64 = 1_791_070_920;

/// Golden du rendu texte : labels FR de `Step::label()`, points de
/// calage (champ étape = label + espace + points = 31 caractères, 3
/// points minimum ; champ test = nom + espace + points = 18), icônes
/// ✗/💥, détail court entre parenthèses, score au dixième.
const GOLDEN: &str = r#"═══ SEEYOU ═══ mini_ls ═══ 2026-10-03 23:42 ═══
▸ Vérifications préliminaires ... OK
▸ Compilation ................... OK
▸ Norme ......................... 3 major, 12 minor
▸ Symboles ...................... OK
▸ Tests unitaires ............... skipped
▸ Tests fonctionnels ............ 1/3 (33.3%)
    ✗ basic_ls ......... FAILED (-file_a)
    💥 big_dir .......... CRASHED (SIGSEGV)
SCORE GLOBAL : 33.3%
"#;

#[test]
fn render_text_at_golden() {
    let at = SystemTime::UNIX_EPOCH + Duration::from_secs(INSTANT_GOLDEN);
    assert_eq!(render_text_at(&report_golden(), at), GOLDEN);
}

#[test]
fn render_text_at_epoch_zero() {
    // Conversion civil-from-days : l'epoch est le point fixe.
    let rendu = render_text_at(&report_golden(), SystemTime::UNIX_EPOCH);
    assert!(
        rendu.starts_with("═══ SEEYOU ═══ mini_ls ═══ 1970-01-01 00:00 ═══\n"),
        "en-tête inattendu : {:?}",
        rendu.lines().next()
    );
}

#[test]
fn render_text_utilise_l_heure_courante() {
    // render_text (sans horloge injectée) : l'en-tête porte l'heure du
    // run, pas l'epoch — et le score reste au dixième.
    let rendu = render_text(&report_golden());
    assert!(
        rendu.starts_with("═══ SEEYOU ═══ mini_ls ═══ "),
        "en-tête inattendu : {:?}",
        rendu.lines().next()
    );
    assert!(
        !rendu.contains("1970-01-01"),
        "render_text ne doit pas utiliser l'epoch : {:?}",
        rendu.lines().next()
    );
    assert!(rendu.ends_with("SCORE GLOBAL : 33.3%\n"));
}

#[test]
fn render_scenario_catastrophe_ko_skip_clean_timeout() {
    // Scénario cohérent : build OK, norme clean, symbols désactivée,
    // unit KO (tests_run en échec, aucun test parsé), functional avec
    // un timeout. Groupes attendus functional + unit → 0.0 partout.
    let report = Report {
        project: "cata".to_string(),
        steps: vec![
            sr("prelim", true, false, "ok"),
            sr("build", true, false, "make ok"),
            sr("norme", true, false, "coding style clean"),
            sr("symbols", true, true, "skipped: pas de whitelist"),
            sr("unit", false, false, "make tests_run (exit 2)"),
            sr("functional", false, false, "1 tests, 1 en échec"),
            sr("verdict", true, false, "score global: 0.0%"),
        ],
        tests: vec![rec(
            "functional",
            "lent",
            "timeout",
            None,
            Some("délai dépassé"),
        )],
        norme: Vec::new(),
        scores: Scores {
            par_groupe: vec![("functional".to_string(), 0.0), ("unit".to_string(), 0.0)],
            global: 0.0,
            norme_fatal: 0,
            norme_major: 0,
            norme_minor: 0,
            norme_info: 0,
        },
        duration_secs: 1.0,
    };
    let rendu = render_text_at(&report, SystemTime::UNIX_EPOCH);
    assert!(
        rendu.contains("▸ Norme ......................... clean\n"),
        "norme clean : {rendu}"
    );
    assert!(
        rendu.contains("▸ Symboles ...................... skipped\n"),
        "symbols skipped : {rendu}"
    );
    assert!(
        rendu.contains("▸ Tests unitaires ............... KO\n"),
        "unit KO : {rendu}"
    );
    assert!(
        rendu.contains("▸ Tests fonctionnels ............ 0/1 (0.0%)\n"),
        "functional 0/1 : {rendu}"
    );
    assert!(
        rendu.contains("    ⏱ lent ............. TIMEOUT (délai dépassé)\n"),
        "ligne timeout : {rendu}"
    );
    assert!(rendu.ends_with("SCORE GLOBAL : 0.0%\n"), "score : {rendu}");
}

#[test]
fn scores_groupe_attendu_vide_vaut_zero_jamais_nan() {
    // Règle critique : un groupe attendu sans TestRecord (étape
    // skipped, build KO…) compte 0 % — jamais 100 % vacuoleux ni NaN.
    let s = compute_scores(&[], &["functional"], &[]);
    assert_eq!(s.par_groupe, vec![("functional".to_string(), 0.0)]);
    assert_eq!(s.global, 0.0);
    assert!(s.global.is_finite());
}

#[test]
fn scores_sans_groupe_attendu_global_zero() {
    // Batterie sans aucun test : pas de 100 % vacuoleux, pas de NaN.
    let s = compute_scores(&[], &[], &[]);
    assert!(s.par_groupe.is_empty());
    assert_eq!(s.global, 0.0);
    assert!(s.global.is_finite());
}

#[test]
fn scores_moyenne_simple_non_ponderee() {
    // functional 1/1 = 100 (1 test), unit 1/4 = 25 (4 tests) :
    // moyenne SIMPLE = 62.5 (une moyenne pondérée donnerait 40).
    let mut tests = vec![rec("functional", "f1", "passed", None, None)];
    for i in 0..4 {
        let verdict = if i == 0 { "passed" } else { "failed" };
        tests.push(rec("unit", &format!("u{i}"), verdict, None, None));
    }
    let s = compute_scores(&tests, &["functional", "unit"], &[]);
    assert_eq!(
        s.par_groupe,
        vec![
            ("functional".to_string(), 100.0),
            ("unit".to_string(), 25.0),
        ]
    );
    assert_eq!(s.global, 62.5);
}

#[test]
fn scores_arrondi_au_dixieme() {
    // 1/3 → 33.3, 2/3 → 66.7, 1/6 → 16.7 (arrondi, pas troncature).
    let un_sur_trois = vec![
        rec("functional", "a", "passed", None, None),
        rec("functional", "b", "failed", None, None),
        rec("functional", "c", "failed", None, None),
    ];
    assert_eq!(
        compute_scores(&un_sur_trois, &["functional"], &[]).par_groupe,
        vec![("functional".to_string(), 33.3)]
    );
    let deux_sur_trois = vec![
        rec("functional", "a", "passed", None, None),
        rec("functional", "b", "passed", None, None),
        rec("functional", "c", "failed", None, None),
    ];
    assert_eq!(
        compute_scores(&deux_sur_trois, &["functional"], &[]).par_groupe,
        vec![("functional".to_string(), 66.7)]
    );
    let mut un_sur_six = vec![rec("unit", "a", "passed", None, None)];
    for i in 0..5 {
        un_sur_six.push(rec("unit", &format!("x{i}"), "failed", None, None));
    }
    assert_eq!(
        compute_scores(&un_sur_six, &["unit"], &[]).par_groupe,
        vec![("unit".to_string(), 16.7)]
    );
}

#[test]
fn scores_seuls_les_passed_comptent() {
    // failed, crashed et timeout pèsent sur le total, jamais sur passed.
    let tests = vec![
        rec("functional", "p", "passed", None, None),
        rec("functional", "f", "failed", None, None),
        rec("functional", "c", "crashed", None, None),
        rec("functional", "t", "timeout", None, None),
    ];
    let s = compute_scores(&tests, &["functional"], &[]);
    assert_eq!(s.par_groupe, vec![("functional".to_string(), 25.0)]);
    assert_eq!(s.global, 25.0);
}

#[test]
fn scores_groupe_observe_non_attendu_est_compte() {
    // Défensif (inatteignable via le pipeline) : un TestRecord dont le
    // groupe n'est pas attendu est quand même listé et compté — des
    // résultats réels ne disparaissent pas du score.
    let tests = vec![rec("exotique", "t", "passed", None, None)];
    let s = compute_scores(&tests, &["functional"], &[]);
    assert_eq!(
        s.par_groupe,
        vec![
            ("functional".to_string(), 0.0),
            ("exotique".to_string(), 100.0),
        ]
    );
    assert_eq!(s.global, 50.0);
}

#[test]
fn scores_norme_comptee_par_severite() {
    let norme = vec![
        faute(Severity::Fatal),
        faute(Severity::Major),
        faute(Severity::Major),
        faute(Severity::Minor),
        faute(Severity::Info),
        faute(Severity::Info),
        faute(Severity::Info),
    ];
    let s = compute_scores(&[], &[], &norme);
    assert_eq!(s.norme_fatal, 1);
    assert_eq!(s.norme_major, 2);
    assert_eq!(s.norme_minor, 1);
    assert_eq!(s.norme_info, 3);
}

#[test]
fn render_record_criterion_synthetique_failed_diff_vide_fallback() {
    // Record synthétique criterion (Synthesis non-verbose, Task 10) en
    // échec : `diff` vide. La ligne affichée porte le fallback
    // « écart de sortie » — jamais des parenthèses vides.
    let mut report = report_golden();
    // Champ test = nom + espace + points = 18 caractères (3 minimum) :
    // « criterion #2 » (12) → 5 points de calage.
    let points = |nom: &str| ".".repeat(18usize.saturating_sub(nom.chars().count() + 1).max(3));
    report.tests = vec![rec("functional", "criterion #2", "failed", Some(""), None)];
    let rendu = render_text_at(&report, SystemTime::UNIX_EPOCH);
    let ligne = format!(
        "✗ criterion #2 {} FAILED (écart de sortie)\n",
        points("criterion #2")
    );
    assert!(
        rendu.contains(&ligne),
        "fallback « écart de sortie » attendu : {rendu}"
    );
    // Diff absent (None) : même fallback.
    report.tests = vec![rec("functional", "criterion #3", "failed", None, None)];
    let rendu = render_text_at(&report, SystemTime::UNIX_EPOCH);
    let ligne = format!(
        "✗ criterion #3 {} FAILED (écart de sortie)\n",
        points("criterion #3")
    );
    assert!(
        rendu.contains(&ligne),
        "fallback « écart de sortie » attendu (diff None) : {rendu}"
    );
}

#[test]
fn render_detail_long_tronque_a_40_avec_ellipse() {
    // Première ligne de diff > 40 caractères : tronquée à 40 + « … ».
    let diff_long = "x".repeat(60);
    let mut report = report_golden();
    report.tests = vec![rec("functional", "long", "failed", Some(&diff_long), None)];
    let rendu = render_text_at(&report, SystemTime::UNIX_EPOCH);
    let tronque = format!(
        "✗ long {} FAILED ({}…)\n",
        ".".repeat(13), // champ test 18 : « long » (4) + espace + 13 points
        "x".repeat(40)
    );
    assert!(
        rendu.contains(&tronque),
        "détail tronqué à 40 + … attendu : {rendu}"
    );
    // Le 41e caractère ne passe pas : aucune suite de 41 x.
    assert!(
        !rendu.contains(&"x".repeat(41)),
        "détail non tronqué : {rendu}"
    );
    // Frontière exacte : 40 caractères → PAS d'ellipse.
    let diff_40 = "y".repeat(40);
    report.tests = vec![rec("functional", "bord", "failed", Some(&diff_40), None)];
    let rendu = render_text_at(&report, SystemTime::UNIX_EPOCH);
    assert!(
        rendu.contains(&format!("FAILED ({})\n", "y".repeat(40))),
        "40 caractères pile : pas d'ellipse : {rendu}"
    );
}

#[test]
fn render_norme_compte_depuis_les_fautes_pas_les_scores() {
    // Minor 4 : un rapport ancien (JSON écrit avant le champ `scores`,
    // re-rendu) a `scores.norme_*` à zéro via serde(default) alors que
    // `norme` porte les fautes — le statut de l'étape Norme doit être
    // dérivé de `report.norme`, jamais des scores.
    let mut report = report_golden();
    report.scores = Scores::default();
    let rendu = render_text_at(&report, SystemTime::UNIX_EPOCH);
    assert!(
        rendu.contains("▸ Norme ......................... 3 major, 12 minor\n"),
        "comptes dérivés de report.norme attendus : {rendu}"
    );
    // Réciproque : norme vide → « clean », même si les scores
    // rapporteraient des fautes (scores mensongers).
    report.norme = Vec::new();
    report.scores.norme_major = 7;
    let rendu = render_text_at(&report, SystemTime::UNIX_EPOCH);
    assert!(
        rendu.contains("▸ Norme ......................... clean\n"),
        "norme vide → clean, quels que soient les scores : {rendu}"
    );
}

#[test]
fn save_ecrit_last_json_et_last_txt() {
    let _lock = XDG_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
    let data = TempDir::new().unwrap();
    let _xdg = xdg_data(data.path());

    let mut report = report_golden();
    report.project = "json_txt".to_string();
    save(&report).unwrap();

    let dir = data.path().join("seeyou");
    let json = std::fs::read_to_string(dir.join("last.json")).unwrap();
    let relu: Report = serde_json::from_str(&json).unwrap();
    assert_eq!(relu.project, "json_txt");
    assert_eq!(relu.steps.len(), 7);
    assert_eq!(relu.tests.len(), 3);
    assert_eq!(relu.norme.len(), 15);
    assert_eq!(
        relu.scores.par_groupe,
        vec![("functional".to_string(), 33.3)]
    );
    assert_eq!(relu.scores.global, 33.3);
    assert_eq!(relu.scores.norme_major, 3);
    assert_eq!(relu.scores.norme_minor, 12);

    let txt = std::fs::read_to_string(dir.join("last.txt")).unwrap();
    assert!(
        txt.starts_with("═══ SEEYOU ═══ json_txt ═══ "),
        "en-tête txt inattendu : {:?}",
        txt.lines().next()
    );
    assert!(txt.ends_with("SCORE GLOBAL : 33.3%\n"), "txt : {txt}");
}

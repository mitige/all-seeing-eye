//! Tests du dashboard TUI (Task 11) : logique d'état uniquement —
//! `App::apply`, navigation, filtre, reconstruction depuis un
//! rapport, export. PAS de test E2E terminal (instable) : le rendu
//! lit `App` sans le muter, les smoke tests de rendu sont unitaires
//! (`src/tui/mod.rs`, `TestBackend`).

use all_seeing_eye::engine::events::{Event, Step, TestVerdict};
use all_seeing_eye::norme::{NormeFault, Severity};
use all_seeing_eye::report::{Report, Scores, StepReport, TestRecord};
use all_seeing_eye::tui::dashboard::{App, Selection, StepStatus, MAX_LOGS};
use std::time::Instant;

mod common;

use common::{XdgGuard, XDG_MUTEX};

/// App fraîche, batterie « mini ».
fn app() -> App {
    App::new("mini".to_string(), Instant::now())
}

/// Enchaîne tout un lot d'events.
fn feed(app: &mut App, events: impl IntoIterator<Item = Event>) {
    for ev in events {
        app.apply(ev);
    }
}

fn started(step: Step) -> Event {
    Event::StepStarted {
        step,
        label: step.label().to_string(),
    }
}

fn finished_ok(step: Step) -> Event {
    Event::StepFinished {
        step,
        ok: true,
        skipped: false,
        summary: format!("{} ok", step.name()),
    }
}

fn fault(file: &str, line: u32, severity: Severity) -> NormeFault {
    NormeFault {
        file: file.into(),
        line,
        col: 1,
        severity,
        rule: "C-G1".to_string(),
        message: "mauvais en-tête".to_string(),
    }
}

fn mini_report() -> Report {
    Report {
        project: "mini".to_string(),
        steps: Vec::new(),
        tests: Vec::new(),
        norme: Vec::new(),
        scores: Scores::default(),
        duration_secs: 1.0,
    }
}

// ── apply : étapes, logs, checks ────────────────────────────────

#[test]
fn step_started_marque_running_et_la_selection_suit() {
    let mut app = app();
    assert_eq!(app.step(Step::Build).status, StepStatus::Pending);
    app.apply(started(Step::Build));
    assert_eq!(app.step(Step::Build).status, StepStatus::Running);
    assert_eq!(app.step(Step::Build).label, Step::Build.label());
    // Suivi automatique : tant que l'utilisateur ne navigue pas, la
    // sélection colle à l'étape courante.
    assert_eq!(app.selected(), Selection::Step(Step::Build));
    // Les autres étapes restent en attente.
    assert_eq!(app.step(Step::Norme).status, StepStatus::Pending);
}

#[test]
fn logs_et_checks_s_accumulent_par_etape() {
    let mut app = app();
    feed(
        &mut app,
        [
            started(Step::Build),
            Event::LogLine {
                step: Step::Build,
                line: "cc -c main.c".to_string(),
            },
            Event::LogLine {
                step: Step::Build,
                line: "cc -o binaire".to_string(),
            },
            Event::CheckFinished {
                step: Step::Build,
                name: "make".to_string(),
                ok: true,
                detail: String::new(),
            },
            // Un log d'une AUTRE étape ne se mélange pas.
            started(Step::Norme),
            Event::LogLine {
                step: Step::Norme,
                line: "scan src/".to_string(),
            },
        ],
    );
    assert_eq!(
        app.step(Step::Build).logs,
        vec!["cc -c main.c".to_string(), "cc -o binaire".to_string()]
    );
    assert_eq!(app.step(Step::Build).checks.len(), 1);
    assert_eq!(app.step(Step::Build).checks[0].0, "make");
    assert!(app.step(Step::Build).checks[0].1);
    assert_eq!(app.step(Step::Norme).logs, vec!["scan src/".to_string()]);
    assert!(app.step(Step::Norme).checks.is_empty());
}

#[test]
fn logs_sont_cappes_avec_marqueur_visible() {
    let mut app = app();
    app.apply(started(Step::Build));
    for i in 0..MAX_LOGS + 10 {
        app.apply(Event::LogLine {
            step: Step::Build,
            line: format!("ligne {i}"),
        });
    }
    let logs = &app.step(Step::Build).logs;
    // MAX lignes + 1 marqueur de troncature, jamais plus.
    assert_eq!(logs.len(), MAX_LOGS + 1);
    assert!(
        logs.last().unwrap().contains("tronqu"),
        "marqueur attendu, obtenu : {}",
        logs.last().unwrap()
    );
}

// ── apply : tests groupés ───────────────────────────────────────

#[test]
fn tests_groupes_dans_l_ordre_et_verdict_pose() {
    let mut app = app();
    feed(
        &mut app,
        [
            Event::TestStarted {
                group: "unit".to_string(),
                name: "a".to_string(),
            },
            Event::TestFinished {
                group: "unit".to_string(),
                name: "a".to_string(),
                result: TestVerdict::Passed,
            },
            Event::TestStarted {
                group: "unit".to_string(),
                name: "b".to_string(),
            },
            Event::TestStarted {
                group: "functional".to_string(),
                name: "c".to_string(),
            },
            Event::TestFinished {
                group: "functional".to_string(),
                name: "c".to_string(),
                result: TestVerdict::Crashed(11),
            },
        ],
    );
    let groups = app.groups();
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].name, "unit");
    assert_eq!(groups[1].name, "functional");
    // Ordre conservé dans le groupe.
    assert_eq!(groups[0].tests[0].name, "a");
    assert_eq!(groups[0].tests[1].name, "b");
    // Verdict posé sur le test démarré ; « b » reste en cours.
    assert_eq!(groups[0].tests[0].verdict, Some(TestVerdict::Passed));
    assert_eq!(groups[0].tests[1].verdict, None);
    assert_eq!(groups[1].tests[0].verdict, Some(TestVerdict::Crashed(11)));
    // Détail court matérialisé pour les crashs (affichage detail).
    assert_eq!(groups[1].tests[0].detail.as_deref(), Some("SIGSEGV"));
}

#[test]
fn test_finished_sans_started_cree_le_test_quand_meme() {
    // Robustesse au flux d'events : un Finished orphelin ne doit
    // jamais perdre un résultat.
    let mut app = app();
    app.apply(Event::TestFinished {
        group: "unit".to_string(),
        name: "orphelin".to_string(),
        result: TestVerdict::Timeout,
    });
    assert_eq!(app.groups().len(), 1);
    assert_eq!(app.groups()[0].tests[0].name, "orphelin");
    assert_eq!(app.groups()[0].tests[0].verdict, Some(TestVerdict::Timeout));
    assert_eq!(
        app.groups()[0].tests[0].detail.as_deref(),
        Some("délai dépassé")
    );
}

#[test]
fn jauge_du_groupe_compte_les_passed() {
    let mut app = app();
    for (name, v) in [
        ("a", TestVerdict::Passed),
        ("b", TestVerdict::Passed),
        ("c", TestVerdict::Timeout),
    ] {
        app.apply(Event::TestFinished {
            group: "unit".to_string(),
            name: name.to_string(),
            result: v,
        });
    }
    let g = &app.groups()[0];
    assert_eq!(g.total(), 3);
    assert_eq!(g.passed(), 2);
    // 66.6… arrondi au dixième, jamais de NaN sur groupe vide.
    assert!((g.percent() - 66.7).abs() < 0.05);
}

// ── apply : norme, fin d'étape, fin de run ──────────────────────

#[test]
fn norme_faults_s_accumulent() {
    let mut app = app();
    app.apply(Event::NormeFault(fault("src/a.c", 3, Severity::Major)));
    app.apply(Event::NormeFault(fault("src/b.c", 7, Severity::Minor)));
    assert_eq!(app.norme().len(), 2);
    assert_eq!(app.norme()[0].file.to_str().unwrap(), "src/a.c");
    assert_eq!(app.norme()[1].severity, Severity::Minor);
}

#[test]
fn step_finished_fixe_ok_ko_skipped_et_resume() {
    let mut app = app();
    feed(
        &mut app,
        [
            started(Step::Build),
            finished_ok(Step::Build),
            started(Step::Norme),
            Event::StepFinished {
                step: Step::Norme,
                ok: false,
                skipped: false,
                summary: "3 major".to_string(),
            },
            started(Step::Unit),
            Event::StepFinished {
                step: Step::Unit,
                ok: false,
                skipped: true,
                summary: "skipped: build failed".to_string(),
            },
        ],
    );
    assert_eq!(app.step(Step::Build).status, StepStatus::Ok);
    assert_eq!(app.step(Step::Norme).status, StepStatus::Ko);
    assert_eq!(app.step(Step::Unit).status, StepStatus::Skipped);
    // Un skip n'est jamais un succès, même si le pipeline dit ok.
    assert_eq!(
        app.step(Step::Unit).summary.as_deref(),
        Some("skipped: build failed")
    );
}

#[test]
fn run_finished_pose_le_rapport() {
    let mut app = app();
    assert!(app.finished().is_none());
    app.apply(Event::RunFinished {
        report: Box::new(mini_report()),
    });
    assert_eq!(app.finished().map(|r| r.project.as_str()), Some("mini"));
}

// ── navigation ──────────────────────────────────────────────────

/// App garnie : 2 fautes de norme, unit[a ✓, b ✗], functional[c ✓].
fn app_garnie() -> App {
    let mut app = app();
    app.apply(Event::NormeFault(fault("src/a.c", 3, Severity::Major)));
    app.apply(Event::NormeFault(fault("src/b.c", 7, Severity::Minor)));
    for (group, name, v) in [
        ("unit", "a", TestVerdict::Passed),
        (
            "unit",
            "b",
            TestVerdict::Failed {
                diff: "-x\n+y".to_string(),
                expected: "x".to_string(),
                got: "y".to_string(),
            },
        ),
        ("functional", "c", TestVerdict::Passed),
    ] {
        app.apply(Event::TestFinished {
            group: group.to_string(),
            name: name.to_string(),
            result: v,
        });
    }
    app
}

#[test]
fn arbre_visible_dans_l_ordre_steps_puis_feuilles() {
    let app = app_garnie();
    // groupes : 0 = unit, 1 = functional (ordre d'apparition).
    let attendu = vec![
        Selection::Step(Step::Prelim),
        Selection::Step(Step::Build),
        Selection::Step(Step::Norme),
        Selection::Fault(0),
        Selection::Fault(1),
        Selection::Step(Step::Symbols),
        Selection::Step(Step::Unit),
        Selection::Test { group: 0, index: 0 },
        Selection::Test { group: 0, index: 1 },
        Selection::Step(Step::Functional),
        Selection::Test { group: 1, index: 0 },
        Selection::Step(Step::Verdict),
    ];
    assert_eq!(app.visible_rows(), attendu);
}

#[test]
fn navigation_parcourt_tout_l_arbre() {
    let mut app = app_garnie();
    let rows = app.visible_rows();
    assert_eq!(app.selected(), rows[0]);
    for attendu in &rows[1..] {
        app.next();
        assert_eq!(app.selected(), *attendu);
    }
    for attendu in rows.iter().rev().skip(1) {
        app.prev();
        assert_eq!(app.selected(), *attendu);
    }
}

#[test]
fn navigation_respecte_les_bornes() {
    let mut app = app_garnie();
    app.prev();
    assert_eq!(app.selected(), Selection::Step(Step::Prelim));
    let dernier = *app.visible_rows().last().unwrap();
    for _ in 0..20 {
        app.next();
    }
    assert_eq!(app.selected(), dernier);
}

#[test]
fn navigation_manuelle_desactive_le_suivi_auto() {
    let mut app = app();
    app.apply(started(Step::Build));
    assert_eq!(app.selected(), Selection::Step(Step::Build));
    // L'utilisateur remonte : le suivi auto se coupe...
    app.prev();
    assert_eq!(app.selected(), Selection::Step(Step::Prelim));
    // ...et l'étape suivante ne vole plus la sélection.
    app.apply(started(Step::Norme));
    assert_eq!(app.selected(), Selection::Step(Step::Prelim));
}

#[test]
fn scroll_detail_remonte_redescend_et_se_reset() {
    let mut app = app_garnie();
    assert_eq!(app.scroll_back(), 0);
    app.scroll_up();
    app.scroll_up();
    assert_eq!(app.scroll_back(), 20);
    app.scroll_down();
    assert_eq!(app.scroll_back(), 10);
    // En bas, on reste en bas (suivi des derniers logs).
    app.scroll_down();
    assert_eq!(app.scroll_back(), 0);
    app.scroll_up();
    assert_eq!(app.scroll_back(), 10);
    // Changer de sélection rattache le détail en bas.
    app.next();
    assert_eq!(app.scroll_back(), 0);
}

// ── filtre échecs ───────────────────────────────────────────────

#[test]
fn filtre_ne_garde_que_les_echecs() {
    let mut app = app();
    feed(
        &mut app,
        [
            started(Step::Prelim),
            finished_ok(Step::Prelim),
            started(Step::Build),
            Event::StepFinished {
                step: Step::Build,
                ok: false,
                skipped: false,
                summary: "make KO".to_string(),
            },
        ],
    );
    app.apply(Event::NormeFault(fault("src/a.c", 3, Severity::Major)));
    // Norme ok mais AVEC fautes : visible (parent d'échecs).
    app.apply(started(Step::Norme));
    app.apply(finished_ok(Step::Norme));
    // Unit ok mais avec un test en échec : visible (parent d'échec).
    app.apply(started(Step::Unit));
    app.apply(Event::TestFinished {
        group: "unit".to_string(),
        name: "a".to_string(),
        result: TestVerdict::Passed,
    });
    app.apply(Event::TestFinished {
        group: "unit".to_string(),
        name: "b".to_string(),
        result: TestVerdict::Timeout,
    });
    app.apply(finished_ok(Step::Unit));
    // Functional ok, tout passe : masqué.
    app.apply(started(Step::Functional));
    app.apply(Event::TestFinished {
        group: "functional".to_string(),
        name: "c".to_string(),
        result: TestVerdict::Passed,
    });
    app.apply(finished_ok(Step::Functional));

    assert!(!app.filter_failures());
    app.toggle_failures();
    assert!(app.filter_failures());
    assert_eq!(
        app.visible_rows(),
        vec![
            Selection::Step(Step::Build), // KO
            Selection::Step(Step::Norme), // parent de fautes
            Selection::Fault(0),
            Selection::Step(Step::Unit), // parent d'un test en échec
            Selection::Test { group: 0, index: 1 }, // b, timeout
        ]
    );
    // Le test « a » (passed) et Functional (tout vert) sont masqués.
    app.toggle_failures();
    assert!(!app.filter_failures());
    // 7 étapes + 1 faute + 3 tests = 11 lignes (même structure que
    // `arbre_visible_dans_l_ordre_steps_puis_feuilles`).
    assert_eq!(app.visible_rows().len(), 11);
}

#[test]
fn filtre_post_run_garde_le_verdict_visible() {
    let mut app = app();
    app.apply(started(Step::Verdict));
    app.apply(finished_ok(Step::Verdict));
    app.toggle_failures();
    // En cours de run (pas de rapport) : Verdict ok sans échec, masqué.
    assert!(
        !app.visible_rows().contains(&Selection::Step(Step::Verdict)),
        "avant le rapport, le Verdict n'a rien à montrer en filtre"
    );
    // Post-run : le Verdict mène au détail des scores — toujours visible.
    app.apply(Event::RunFinished {
        report: Box::new(mini_report()),
    });
    assert!(
        app.visible_rows().contains(&Selection::Step(Step::Verdict)),
        "le Verdict doit rester visible en filtre post-run"
    );
    // Un arbre qui n'est QUE le Verdict reste navigable.
    app.next();
    app.prev();
    assert_eq!(app.selected(), Selection::Step(Step::Verdict));
}

#[test]
fn filtre_reclame_la_selection_devenue_invisible() {
    let mut app = app_garnie();
    // Sélectionne le test « a » (passed) : le filtre va le masquer.
    while app.selected() != (Selection::Test { group: 0, index: 0 }) {
        app.next();
    }
    app.toggle_failures();
    // Repli : voisin visible le plus proche AVANT dans l'ordre de
    // l'arbre — ici l'étape Unit, parent du test masqué.
    assert_eq!(app.selected(), Selection::Step(Step::Unit));
}

#[test]
fn filtre_sans_aucun_echec_donne_un_arbre_vide_sans_panique() {
    let mut app = app();
    app.apply(started(Step::Prelim));
    app.apply(finished_ok(Step::Prelim));
    app.apply(Event::TestFinished {
        group: "unit".to_string(),
        name: "a".to_string(),
        result: TestVerdict::Passed,
    });
    app.toggle_failures();
    assert!(app.visible_rows().is_empty());
    // Navigation sur arbre vide : borne basse/haute, jamais de panic.
    app.next();
    app.prev();
    app.toggle_failures();
    assert!(!app.visible_rows().is_empty());
}

// ── reconstruction depuis un rapport (mode browse) ──────────────

#[test]
fn from_report_reconstruit_l_etat_complet() {
    let report = Report {
        project: "my_ls".to_string(),
        steps: vec![
            StepReport {
                step: "prelim".to_string(),
                ok: true,
                skipped: false,
                summary: "3/3 checks".to_string(),
                checks: vec![("Makefile présent".to_string(), true, String::new())],
            },
            StepReport {
                step: "build".to_string(),
                ok: false,
                skipped: false,
                summary: "make KO".to_string(),
                checks: vec![],
            },
            StepReport {
                step: "unit".to_string(),
                ok: false,
                skipped: true,
                summary: "skipped: build failed".to_string(),
                checks: vec![],
            },
        ],
        tests: vec![
            TestRecord {
                group: "unit".to_string(),
                name: "a".to_string(),
                verdict: "passed".to_string(),
                diff: None,
                detail: None,
            },
            TestRecord {
                group: "unit".to_string(),
                name: "b".to_string(),
                verdict: "failed".to_string(),
                diff: Some("-x\n+y\n".to_string()),
                detail: None,
            },
            TestRecord {
                group: "functional".to_string(),
                name: "segv".to_string(),
                verdict: "crashed".to_string(),
                diff: None,
                detail: Some("SIGSEGV".to_string()),
            },
        ],
        norme: vec![fault("src/main.c", 12, Severity::Fatal)],
        scores: Scores {
            global: 66.7,
            ..Scores::default()
        },
        duration_secs: 3.5,
    };
    let app = App::from_report(report);
    // Étapes : statuts et checks reconstruits ; une étape absente du
    // rapport reste Pending (jamais inventée).
    assert_eq!(app.step(Step::Prelim).status, StepStatus::Ok);
    assert_eq!(app.step(Step::Prelim).checks.len(), 1);
    assert_eq!(app.step(Step::Build).status, StepStatus::Ko);
    assert_eq!(app.step(Step::Unit).status, StepStatus::Skipped);
    assert_eq!(app.step(Step::Symbols).status, StepStatus::Pending);
    assert_eq!(app.step(Step::Build).summary.as_deref(), Some("make KO"));
    // Groupes dans l'ordre d'apparition des records, verdicts reconnus.
    assert_eq!(app.groups().len(), 2);
    assert_eq!(app.groups()[0].name, "unit");
    assert_eq!(
        app.groups()[0].tests[1].verdict,
        Some(TestVerdict::Failed {
            diff: "-x\n+y\n".to_string(),
            expected: String::new(),
            got: String::new(),
        })
    );
    assert_eq!(app.groups()[1].tests[0].detail.as_deref(), Some("SIGSEGV"));
    assert_eq!(app.norme().len(), 1);
    // Post-run d'emblée : rapport posé, nom de batterie = projet.
    assert!(app.finished().is_some());
    assert_eq!(app.battery(), "my_ls");
    assert_eq!(app.finished().unwrap().scores.global, 66.7);
}

// ── export ──────────────────────────────────────────────────────

#[test]
fn export_ecrit_le_rapport_texte_dans_le_data_dir() {
    // data dir isolé : jamais d'écriture dans le vrai ~/.local/share.
    let _lock = XDG_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
    let data = tempfile::TempDir::new().unwrap();
    let _xdg = XdgGuard::set("XDG_DATA_HOME", data.path());
    let mut app = app();
    app.apply(Event::RunFinished {
        report: Box::new(mini_report()),
    });
    let path = app.export().expect("export avec rapport terminé");
    assert!(path.starts_with(data.path()));
    let nom = path.file_name().unwrap().to_str().unwrap();
    assert!(
        nom.starts_with("export-") && nom.ends_with(".txt"),
        "nom : {nom}"
    );
    let contenu = std::fs::read_to_string(&path).unwrap();
    assert!(
        contenu.contains("ALL-SEEING EYE"),
        "rendu texte du rapport attendu"
    );
    assert!(contenu.contains("mini"));
    // Message de confirmation affiché dans le footer.
    assert!(app.footer_message().is_some());
}

#[test]
fn export_sans_rapport_est_une_erreur_visible() {
    let app = app();
    assert!(app.export().is_err());
}

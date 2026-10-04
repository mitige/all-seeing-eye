//! Étape Verdict — scores, rapport complet et émission finale
//! (Task 10).
//!
//! Règle de scoring (notée en Task 3) : un groupe de tests attendu
//! mais VIDE (étape skipped → aucun TestRecord) compte **0 %** dans
//! le score global — jamais 100 % vacuoleux ni NaN (division 0/0).
//! Comportement vraie moulinette : build KO = tests à zéro. Le calcul
//! lui-même vit dans [`crate::report::compute_scores`] ; ici, le
//! verdict décide les groupes ATTENDUS depuis la batterie (les tasks
//! Functions sont émises groupe "functional", cf. functional.rs),
//! construit le [`Report`] complet, le sauvegarde (best-effort : une
//! erreur de sauvegarde est relayée en LogLine et ne fait PAS échouer
//! le run), émet `RunFinished` et pose le rapport dans `ctx.report`.

use super::events::{Event, Step, TestVerdict};
use super::PipelineContext;
use crate::battery::model::ProjectType;
use crate::battery::Battery;
use crate::report::{compute_scores, save, Report, TestRecord};
use std::sync::mpsc;

/// Étape 7 du pipeline : scores, rapport complet, sauvegarde,
/// `RunFinished`. Réussit toujours : le verdict rapporte, il ne juge
/// pas (les étapes ont déjà jugé).
pub fn run(ctx: &mut PipelineContext, tx: &mpsc::Sender<Event>) {
    super::step_started(tx, Step::Verdict);
    let scores = compute_scores(
        &ctx.tests,
        &groupes_attendus(&ctx.battery),
        &ctx.norme_faults,
    );
    // step_finished AVANT la construction du rapport : le Verdict
    // lui-même est la 7e entrée de `report.steps`.
    super::step_finished(
        ctx,
        tx,
        Step::Verdict,
        true,
        format!("score global: {:.1}%", scores.global),
        Vec::new(),
    );
    let report = Report {
        project: ctx.battery.project.name.clone(),
        steps: ctx.steps.clone(),
        tests: ctx.tests.clone(),
        norme: ctx.norme_faults.clone(),
        scores,
        duration_secs: ctx.started.elapsed().as_secs_f64(),
    };
    // Best-effort : un data dir inscriptible ne doit pas faire
    // échouer un run dont le rapport est déjà construit et émis.
    if let Err(e) = save(&report) {
        super::send(
            tx,
            Event::LogLine {
                step: Step::Verdict,
                line: format!("sauvegarde du rapport impossible : {e:#}"),
            },
        );
    }
    super::send(
        tx,
        Event::RunFinished {
            report: Box::new(report.clone()),
        },
    );
    ctx.report = Some(report);
}

/// Groupes de tests que la batterie aurait produits — la règle « un
/// groupe attendu vide compte 0 % » s'applique à eux. Les tasks
/// Functions comme les functional_test Binary sont émises groupe
/// « functional » (cf. functional.rs) ; `tests_run_rule` attend le
/// groupe « unit ». Ordre : functional, puis unit.
fn groupes_attendus(battery: &Battery) -> Vec<&'static str> {
    let mut groupes = Vec::new();
    let functional_attendu = match battery.project.kind {
        ProjectType::Binary => !battery.functional_test.is_empty(),
        ProjectType::Functions => !battery.task.is_empty(),
    };
    if functional_attendu {
        groupes.push(Step::Functional.name());
    }
    if battery.project.tests_run_rule {
        groupes.push(Step::Unit.name());
    }
    groupes
}

/// Nom canonique d'un signal pour le rendu (« SIGSEGV ») ; repli
/// « signal N » hors table, « signal inconnu » pour les numéros
/// négatifs (crash criterion « CRASH! » : le parsing ne donne pas le
/// signal, cf. unit.rs).
fn nom_signal(sig: i32) -> String {
    let nom = match sig {
        libc::SIGHUP => "SIGHUP",
        libc::SIGINT => "SIGINT",
        libc::SIGILL => "SIGILL",
        libc::SIGTRAP => "SIGTRAP",
        libc::SIGABRT => "SIGABRT",
        libc::SIGBUS => "SIGBUS",
        libc::SIGFPE => "SIGFPE",
        libc::SIGKILL => "SIGKILL",
        libc::SIGSEGV => "SIGSEGV",
        libc::SIGPIPE => "SIGPIPE",
        libc::SIGTERM => "SIGTERM",
        _ => {
            return if sig < 0 {
                "signal inconnu".to_string()
            } else {
                format!("signal {sig}")
            };
        }
    };
    nom.to_string()
}

/// Convertit un [`TestVerdict`] en [`TestRecord`] : libellé canonique
/// ("passed"|"failed"|"crashed"|"timeout"), diff extrait pour Failed,
/// detail court pour Crashed (signal) et Timeout.
///
/// Appelé par les étapes unit (Task 8) et functional (Task 9) pour
/// alimenter `ctx.tests`.
pub(crate) fn test_record(group: &str, name: &str, verdict: &TestVerdict) -> TestRecord {
    let (label, diff, detail) = match verdict {
        TestVerdict::Passed => ("passed", None, None),
        TestVerdict::Failed { diff, .. } => ("failed", Some(diff.clone()), None),
        TestVerdict::Crashed(sig) => ("crashed", None, Some(nom_signal(*sig))),
        TestVerdict::Timeout => ("timeout", None, Some("délai dépassé".to_string())),
    };
    TestRecord {
        group: group.to_string(),
        name: name.to_string(),
        verdict: label.to_string(),
        diff,
        detail,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battery::Battery;

    /// Charge une batterie TOML écrite dans un tempdir (le TempDir
    /// meurt en fin de portée : `groupes_attendus` ne touche pas le
    /// disque, seul le modèle parsé compte).
    fn batterie(toml: &str) -> Battery {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("moulinette.toml");
        std::fs::write(&path, toml).unwrap();
        Battery::load(&path).unwrap()
    }

    #[test]
    fn test_verdict_converti_en_test_record() {
        let r = test_record("unit", "ex01", &TestVerdict::Passed);
        assert_eq!(r.group, "unit");
        assert_eq!(r.name, "ex01");
        assert_eq!(r.verdict, "passed");
        assert_eq!(r.diff, None);

        let r = test_record(
            "unit",
            "ex02",
            &TestVerdict::Failed {
                diff: "-a\n+b".to_string(),
                expected: "a".to_string(),
                got: "b".to_string(),
            },
        );
        assert_eq!(r.verdict, "failed");
        assert_eq!(r.diff.as_deref(), Some("-a\n+b"));

        let r = test_record("functional", "segv", &TestVerdict::Crashed(11));
        assert_eq!(r.verdict, "crashed");
        assert_eq!(r.diff, None);

        let r = test_record("functional", "lent", &TestVerdict::Timeout);
        assert_eq!(r.verdict, "timeout");
        assert_eq!(r.diff, None);
    }

    #[test]
    fn test_record_detail_crash_et_timeout() {
        // Le signal d'un crash est conservé en detail pour le rendu
        // (« 💥 big_dir … CRASHED (SIGSEGV) ») ; -1 = crash criterion
        // dont le signal est inconnu au parsing.
        let r = test_record("functional", "segv", &TestVerdict::Crashed(11));
        assert_eq!(r.detail.as_deref(), Some("SIGSEGV"));
        let r = test_record("unit", "crash", &TestVerdict::Crashed(-1));
        assert_eq!(r.detail.as_deref(), Some("signal inconnu"));
        let r = test_record("functional", "lent", &TestVerdict::Timeout);
        assert_eq!(r.detail.as_deref(), Some("délai dépassé"));
        // Passed/Failed : pas de detail — le diff porte tout.
        let r = test_record("unit", "ok", &TestVerdict::Passed);
        assert_eq!(r.detail, None);
        let r = test_record(
            "unit",
            "ko",
            &TestVerdict::Failed {
                diff: "-a\n+b".to_string(),
                expected: "a".to_string(),
                got: "b".to_string(),
            },
        );
        assert_eq!(r.detail, None);
    }

    #[test]
    fn nom_signal_connus_et_inconnus() {
        assert_eq!(nom_signal(libc::SIGSEGV), "SIGSEGV");
        assert_eq!(nom_signal(libc::SIGABRT), "SIGABRT");
        assert_eq!(nom_signal(libc::SIGILL), "SIGILL");
        assert_eq!(nom_signal(libc::SIGFPE), "SIGFPE");
        assert_eq!(nom_signal(libc::SIGBUS), "SIGBUS");
        assert_eq!(nom_signal(libc::SIGKILL), "SIGKILL");
        assert_eq!(nom_signal(libc::SIGTERM), "SIGTERM");
        assert_eq!(nom_signal(libc::SIGPIPE), "SIGPIPE");
        // Numéro sans nom canonique : repli explicite, jamais vide.
        assert_eq!(nom_signal(42), "signal 42");
        // -1 : crash criterion « CRASH! », signal inconnu au parsing.
        assert_eq!(nom_signal(-1), "signal inconnu");
    }

    #[test]
    fn groupes_attendus_selon_batterie() {
        // Binary avec functional_test → "functional" ; la règle
        // tests_run ajoute "unit" (ordre : functional, puis unit).
        let b = batterie(
            "[project]\nname = \"x\"\ntype = \"binary\"\nbinary = \"x\"\n\n\
             [[functional_test]]\nname = \"t\"\nstdout = \"\"\n",
        );
        assert_eq!(groupes_attendus(&b), vec!["functional"]);
        let b = batterie(
            "[project]\nname = \"x\"\ntype = \"binary\"\nbinary = \"x\"\n\
             tests_run_rule = true\n\n\
             [[functional_test]]\nname = \"t\"\nstdout = \"\"\n",
        );
        assert_eq!(groupes_attendus(&b), vec!["functional", "unit"]);
        // Binary sans functional_test, avec tests_run → "unit" seul.
        let b = batterie(
            "[project]\nname = \"x\"\ntype = \"binary\"\nbinary = \"x\"\n\
             tests_run_rule = true\n",
        );
        assert_eq!(groupes_attendus(&b), vec!["unit"]);
        // Functions : les tasks sont émises groupe "functional".
        let b = batterie(
            "[project]\nname = \"x\"\ntype = \"functions\"\n\n\
             [[task]]\nname = \"ex01\"\ndelivery = \"a.c\"\n\
             harness = \"h.c\"\nstdout = \"\"\n",
        );
        assert_eq!(groupes_attendus(&b), vec!["functional"]);
        // Rien à tester : aucun groupe attendu (score global 0.0, pas
        // de 100 % vacuoleux).
        let b = batterie("[project]\nname = \"x\"\ntype = \"binary\"\nbinary = \"x\"\n");
        assert_eq!(groupes_attendus(&b), Vec::<&str>::new());
    }
}

//! Étape Verdict — agrégation finale et émission du rapport (stub, Task 3).
//!
//! Le scoring détaillé arrive en Task 10 ; ici, le verdict réussit
//! toujours et le rapport est une simple collecte du ctx.

use super::events::{Event, Step, TestVerdict};
use super::PipelineContext;
use crate::report::{Report, TestRecord};
use std::sync::mpsc;

/// Stub : émet Started/Finished, construit le [`Report`] final à partir
/// du ctx, le pose dans `ctx.report` et émet `RunFinished`.
pub fn run(ctx: &mut PipelineContext, tx: &mpsc::Sender<Event>) {
    super::step_started(tx, Step::Verdict);
    // step_finished AVANT la construction du rapport : le Verdict
    // lui-même est la 7e entrée de `report.steps`.
    super::step_finished(ctx, tx, Step::Verdict, true, "stub".to_string());
    let report = Report {
        project: ctx.battery.project.name.clone(),
        steps: ctx.steps.clone(),
        tests: ctx.tests.clone(),
        norme: ctx.norme_faults.clone(),
        duration_secs: ctx.started.elapsed().as_secs_f64(),
    };
    super::send(
        tx,
        Event::RunFinished {
            report: Box::new(report.clone()),
        },
    );
    ctx.report = Some(report);
}

/// Convertit un [`TestVerdict`] en [`TestRecord`] : libellé canonique
/// ("passed"|"failed"|"crashed"|"timeout"), diff extrait pour Failed.
///
/// Sera appelé par les étapes unit/functional pour alimenter
/// `ctx.tests` (câblage en Tasks 6-7).
#[allow(dead_code)] // pas encore de producteur de TestVerdict
pub(crate) fn test_record(group: &str, name: &str, verdict: &TestVerdict) -> TestRecord {
    let (label, diff) = match verdict {
        TestVerdict::Passed => ("passed", None),
        TestVerdict::Failed { diff, .. } => ("failed", Some(diff.clone())),
        TestVerdict::Crashed(_) => ("crashed", None),
        TestVerdict::Timeout => ("timeout", None),
    };
    TestRecord {
        group: group.to_string(),
        name: name.to_string(),
        verdict: label.to_string(),
        diff,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}

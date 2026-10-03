//! Événements émis par le pipeline, consommés par le front (TUI Task 11).

use crate::norme::NormeFault;
use crate::report::Report;

/// Les 7 étapes du pipeline, dans l'ordre d'exécution (spec §5).
/// L'ordre de déclaration est significatif : `Ord` suit l'enchaînement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Step {
    Prelim,
    Build,
    Norme,
    Symbols,
    Unit,
    Functional,
    Verdict,
}

impl Step {
    /// Libellé affichable (FR) pour le front.
    pub fn label(self) -> &'static str {
        match self {
            Step::Prelim => "Vérifications préliminaires",
            Step::Build => "Compilation",
            Step::Norme => "Norme",
            Step::Symbols => "Symboles",
            Step::Unit => "Tests unitaires",
            Step::Functional => "Tests fonctionnels",
            Step::Verdict => "Verdict",
        }
    }

    /// Identifiant stable, utilisé dans le rapport (`StepReport.step`).
    pub fn name(self) -> &'static str {
        match self {
            Step::Prelim => "prelim",
            Step::Build => "build",
            Step::Norme => "norme",
            Step::Symbols => "symbols",
            Step::Unit => "unit",
            Step::Functional => "functional",
            Step::Verdict => "verdict",
        }
    }
}

/// Verdict d'un test unitaire ou fonctionnel.
#[derive(Debug, Clone, PartialEq)]
pub enum TestVerdict {
    Passed,
    Failed {
        diff: String,
        expected: String,
        got: String,
    },
    Crashed(i32),
    Timeout,
}

/// Un événement de progression du pipeline.
#[derive(Debug, Clone)]
pub enum Event {
    StepStarted {
        step: Step,
        label: String,
    },
    LogLine {
        step: Step,
        line: String,
    },
    CheckFinished {
        step: Step,
        name: String,
        ok: bool,
        detail: String,
    },
    TestStarted {
        group: String,
        name: String,
    },
    TestFinished {
        group: String,
        name: String,
        result: TestVerdict,
    },
    NormeFault(NormeFault),
    StepFinished {
        step: Step,
        ok: bool,
        summary: String,
    },
    RunFinished {
        report: Box<Report>,
    },
}

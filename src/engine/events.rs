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

    /// Inverse de [`Step::name`] : retrouve l'étape d'un
    /// `StepReport.step` sérialisé (rendu du rapport). None si le nom
    /// ne correspond à aucune étape connue.
    pub fn from_name(name: &str) -> Option<Step> {
        Some(match name {
            "prelim" => Step::Prelim,
            "build" => Step::Build,
            "norme" => Step::Norme,
            "symbols" => Step::Symbols,
            "unit" => Step::Unit,
            "functional" => Step::Functional,
            "verdict" => Step::Verdict,
            _ => return None,
        })
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
        /// Étape sautée (dépendance échouée) ; `summary` garde la raison
        /// humaine (ex. "skipped: build failed").
        skipped: bool,
        summary: String,
    },
    RunFinished {
        report: Box<Report>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_from_name_roundtrip_sur_les_7_etapes() {
        // `from_name` est l'inverse exact de `name` — le rendu du
        // rapport (statut d'étape, regroupement des tests) s'appuie
        // sur cette bijection via `StepReport.step`.
        let sept = [
            Step::Prelim,
            Step::Build,
            Step::Norme,
            Step::Symbols,
            Step::Unit,
            Step::Functional,
            Step::Verdict,
        ];
        for step in sept {
            assert_eq!(
                Step::from_name(step.name()),
                Some(step),
                "roundtrip cassé pour {step:?}"
            );
        }
        // Nom inconnu : None — jamais de repli silencieux.
        assert_eq!(Step::from_name("inconnu"), None);
        assert_eq!(Step::from_name(""), None);
    }
}

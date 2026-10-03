//! Rapport final d'un run de la moulinette.
//!
//! Structs de collecte uniquement : le scoring détaillé et l'export
//! (JSON, HTML) arrivent en Task 10.

use crate::norme::NormeFault;
use serde::{Deserialize, Serialize};

/// Résultat collecté d'un test unitaire ou fonctionnel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestRecord {
    pub group: String,
    pub name: String,
    /// "passed" | "failed" | "crashed" | "timeout"
    pub verdict: String,
    pub diff: Option<String>,
}

/// Bilan d'une étape du pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepReport {
    pub step: String,
    pub ok: bool,
    pub summary: String,
    /// (nom du check, ok, détail)
    pub checks: Vec<(String, bool, String)>,
}

/// Rapport complet d'un run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub project: String,
    pub steps: Vec<StepReport>,
    pub tests: Vec<TestRecord>,
    pub norme: Vec<NormeFault>,
    pub duration_secs: f64,
}

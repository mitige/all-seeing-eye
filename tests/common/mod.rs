//! Helpers partagés par les tests d'intégration.

use seeyou::battery::Battery;
use seeyou::engine::{PipelineContext, RunOpts};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Construit un [`PipelineContext`] prêt à tester une étape isolée :
/// même contenu que celui de `run_pipeline`, sans passer par lui.
pub fn test_ctx(battery: &Battery, target: &Path) -> PipelineContext {
    PipelineContext {
        battery: battery.clone(),
        target: target.to_path_buf(),
        opts: RunOpts {
            strict_norme: false,
            use_epiclang: false,
            compiler: PathBuf::from("cc"),
        },
        build: None,
        norme_faults: Vec::new(),
        tests: Vec::new(),
        step_ok: BTreeMap::new(),
        started: Instant::now(),
        steps: Vec::new(),
        report: None,
    }
}

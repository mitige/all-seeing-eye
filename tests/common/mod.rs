//! Helpers partagés par les tests d'intégration. Chaque binaire de
//! test n'en utilise qu'une partie : `dead_code` est désactivé pour
//! tout le module plutôt que d'émailler chaque item.

#![allow(dead_code)]

use all_seeing_eye::battery::Battery;
use all_seeing_eye::engine::{PipelineContext, RunOpts};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
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

/// Les variables `XDG_*` sont process-global : tous les tests qui y
/// touchent (XDG_CONFIG_HOME dans battery_parse, XDG_DATA_HOME dans
/// pipeline/report) se sérialisent sur ce mutex. Chaque binaire de
/// test est un processus séparé : un mutex par module suffit.
pub static XDG_MUTEX: Mutex<()> = Mutex::new(());

/// Positionne une variable d'environnement (`XDG_CONFIG_HOME`,
/// `XDG_DATA_HOME`) vers `path` et la restaure à sa valeur initiale au
/// drop — un `cargo test` ne doit JAMAIS polluer l'environnement réel
/// de l'utilisateur (ni écraser son `~/.local/share/all-seeing-eye/last.*`).
pub struct XdgGuard {
    var: &'static str,
    ancien: Option<std::ffi::OsString>,
}

impl XdgGuard {
    pub fn set(var: &'static str, path: &Path) -> Self {
        let ancien = std::env::var_os(var);
        std::env::set_var(var, path);
        Self { var, ancien }
    }
}

impl Drop for XdgGuard {
    fn drop(&mut self) {
        match &self.ancien {
            Some(v) => std::env::set_var(self.var, v),
            None => std::env::remove_var(self.var),
        }
    }
}

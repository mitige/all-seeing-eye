//! Helpers partagés par les tests d'intégration. Chaque binaire de
//! test n'en utilise qu'une partie : `dead_code` est désactivé pour
//! tout le module plutôt que d'émailler chaque item.

#![allow(dead_code)]

use all_seeing_eye::battery::Battery;
use all_seeing_eye::engine::{run_pipeline, PipelineContext, RunOpts};
use all_seeing_eye::report::Report;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Mutex};
use std::time::Instant;
use tempfile::TempDir;

/// Construit un [`PipelineContext`] prêt à tester une étape isolée :
/// même contenu que celui de `run_pipeline`, sans passer par lui.
pub fn test_ctx(battery: &Battery, target: &Path) -> PipelineContext {
    PipelineContext {
        battery: battery.clone(),
        target: target.to_path_buf(),
        opts: test_opts(),
        build: None,
        norme_faults: Vec::new(),
        tests: Vec::new(),
        step_ok: BTreeMap::new(),
        started: Instant::now(),
        steps: Vec::new(),
        report: None,
    }
}

/// Options de test : cc partout (portable, avec ou sans epiclang).
pub fn test_opts() -> RunOpts {
    RunOpts {
        strict_norme: false,
        use_epiclang: false,
        compiler: PathBuf::from("cc"),
    }
}

/// Chemin d'une fixture versionnée (`tests/fixtures/<name>`).
pub fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

/// La batterie embarquée cpool_day03, extraite et chargée.
pub fn batterie_day03() -> Battery {
    all_seeing_eye::battery::load_embedded("cpool_day03")
        .unwrap()
        .expect("cpool_day03 embarquée")
}

/// Écrit `toml` dans un tempdir et charge la batterie. Le TempDir est
/// renvoyé : il doit vivre tant que `battery.root` peut être relu
/// (harness, expected).
pub fn batterie_toml(toml: &str) -> (TempDir, Battery) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("moulinette.toml");
    std::fs::write(&path, toml).unwrap();
    (dir, Battery::load(&path).unwrap())
}

/// Pipeline complet sur `target` (options [`test_opts`]), rapport
/// retourné. Le verdict sauvegarde le rapport : data dir isolé dans
/// un tempdir — retourné avec le rapport pour permettre de relire
/// `last.json` — jamais celui de l'utilisateur ([`XDG_MUTEX`], les
/// variables XDG sont process-global).
pub fn run_pipeline_isole(battery: &Battery, target: &Path) -> (TempDir, Report) {
    let _lock = XDG_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
    let data = TempDir::new().unwrap();
    let _xdg = XdgGuard::set("XDG_DATA_HOME", data.path());
    let (tx, _rx) = mpsc::channel();
    let report = run_pipeline(battery, target, tx, &test_opts());
    (data, report)
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

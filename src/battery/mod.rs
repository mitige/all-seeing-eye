//! Chargement et découverte des batteries TOML.

pub mod model;

use anyhow::{bail, ensure, Context, Result};
use model::{FunctionalTest, ProjectMeta, ProjectType, Prototype, Task};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

/// Une batterie complète : métadonnées projet + exercices/tests,
/// racinée dans `root` (dossier contenant le TOML).
#[derive(Debug, Clone, Deserialize)]
pub struct Battery {
    pub project: ProjectMeta,
    #[serde(default)]
    pub prototype: Vec<Prototype>,
    #[serde(default)]
    pub task: Vec<Task>,
    #[serde(default)]
    pub functional_test: Vec<FunctionalTest>,
    #[serde(skip)]
    pub root: PathBuf,
}

impl Battery {
    /// Charge une batterie depuis un fichier TOML et la valide.
    /// `root` = dossier parent du TOML.
    pub fn load(path: &Path) -> Result<Battery> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("lecture de {} impossible", path.display()))?;
        let mut battery: Battery = toml::from_str(&content)
            .with_context(|| format!("TOML invalide dans {}", path.display()))?;
        battery.root = path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();
        battery.validate()?;
        Ok(battery)
    }

    /// Découvre la batterie applicable à `dir`, dans l'ordre :
    /// 1. `./moulinette.toml` dans `dir` ;
    /// 2. `~/.config/seeyou/batteries/*.toml` dont `project.name`
    ///    correspond au nom du dossier ;
    /// 3. batteries embarquées (voir [`embedded_batteries`]).
    pub fn discover(dir: &Path) -> Result<Battery> {
        // 1. moulinette.toml local
        let local = dir.join("moulinette.toml");
        if local.is_file() {
            return Battery::load(&local);
        }

        let canon = dir
            .canonicalize()
            .with_context(|| format!("dossier {} introuvable", dir.display()))?;
        let dir_name = canon
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string();

        // 2. ~/.config/seeyou/batteries/*.toml
        if let Some(config) = dirs::config_dir() {
            let batteries_dir = config.join("seeyou").join("batteries");
            if batteries_dir.is_dir() {
                let mut paths: Vec<PathBuf> = fs::read_dir(&batteries_dir)
                    .with_context(|| format!("lecture de {} impossible", batteries_dir.display()))?
                    .filter_map(|e| e.ok().map(|e| e.path()))
                    .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("toml"))
                    .collect();
                paths.sort();
                for path in paths {
                    if let Ok(b) = Battery::load(&path) {
                        if b.project.name == dir_name {
                            return Ok(b);
                        }
                    }
                }
            }
        }

        // 3. Batteries embarquées — Task 13 ajoutera cpool_day03 :
        //    le TOML + ses assets seront extraits dans un tempfile::TempDir
        //    pour que `root` puisse être résolu sur disque, puis matchées
        //    sur `dir_name` ici.
        let _ = embedded_batteries();

        bail!(
            "aucune batterie trouvée pour « {} » ({})",
            dir_name,
            dir.display()
        )
    }

    /// stdout attendu d'un exercice piscine : inline, ou relu depuis
    /// `root.join(stdout_file)`.
    pub fn expected_stdout_of_task(&self, t: &Task) -> Result<String> {
        expected_stdout(&self.root, &t.name, &t.stdout, &t.stdout_file)
    }

    /// stdout attendu d'un test fonctionnel : inline, ou relu depuis
    /// `root.join(stdout_file)`.
    pub fn expected_stdout_of_test(&self, t: &FunctionalTest) -> Result<String> {
        expected_stdout(&self.root, &t.name, &t.stdout, &t.stdout_file)
    }

    fn validate(&self) -> Result<()> {
        if self.project.kind == ProjectType::Binary {
            ensure!(
                self.project.binary.is_some(),
                "projet « {} » de type binary : le champ `binary` est requis",
                self.project.name
            );
        }
        if self.project.kind == ProjectType::Functions {
            ensure!(
                !self.task.is_empty(),
                "projet « {} » de type functions : au moins une [[task]] est requise",
                self.project.name
            );
        }
        for t in &self.task {
            ensure!(
                !(t.stdout.is_some() && t.stdout_file.is_some()),
                "task « {} » : `stdout` et `stdout_file` sont mutuellement exclusifs",
                t.name
            );
        }
        for t in &self.functional_test {
            ensure!(
                !(t.stdout.is_some() && t.stdout_file.is_some()),
                "functional_test « {} » : `stdout` et `stdout_file` sont mutuellement exclusifs",
                t.name
            );
        }
        Ok(())
    }
}

/// Résout le stdout attendu : priorité à l'inline, sinon lecture du
/// fichier relatif à `root`. Erreur claire si aucune des deux sources.
fn expected_stdout(
    root: &Path,
    name: &str,
    stdout: &Option<String>,
    stdout_file: &Option<PathBuf>,
) -> Result<String> {
    if let Some(inline) = stdout {
        return Ok(inline.clone());
    }
    if let Some(rel) = stdout_file {
        let path = root.join(rel);
        return fs::read_to_string(&path).with_context(|| {
            format!(
                "lecture de {} impossible (stdout_file de « {} »)",
                path.display(),
                name
            )
        });
    }
    bail!("« {} » : ni `stdout` ni `stdout_file` fourni", name)
}

/// Batteries embarquées dans le binaire : `(nom, contenu TOML)`.
///
/// Task 13 ajoutera cpool_day03 (via `include_str!` des fichiers de
/// `batteries/`). Vide pour l'instant — aucun `include_str!` ne doit
/// pointer vers un fichier inexistant.
fn embedded_batteries() -> Vec<(&'static str, &'static str)> {
    // Task 13 ajoutera cpool_day03
    Vec::new()
}

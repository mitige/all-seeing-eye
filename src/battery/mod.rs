//! Chargement et découverte des batteries TOML.

pub mod model;

use anyhow::{bail, ensure, Context, Result};
use model::{FunctionalTest, ProjectMeta, ProjectType, Prototype, Task};
use serde::Deserialize;
use std::fs;
use std::path::{Component, Path, PathBuf};

/// Une batterie complète : métadonnées projet + exercices/tests,
/// racinée dans `root` (dossier contenant le TOML).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)] // ok avec `root` en skip : absent du TOML
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
    /// 2. `~/.config/all-seeing-eye/batteries/*.toml` dont `project.name`
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

        // Batteries du tier 2 ignorées car invalides (chemin + erreur) —
        // listées dans l'erreur finale si rien ne matche.
        let mut invalides: Vec<(PathBuf, anyhow::Error)> = Vec::new();

        // 2. ~/.config/all-seeing-eye/batteries/*.toml
        if let Some(config) = dirs::config_dir() {
            let batteries_dir = config.join("all-seeing-eye").join("batteries");
            if batteries_dir.is_dir() {
                let mut paths: Vec<PathBuf> = fs::read_dir(&batteries_dir)
                    .with_context(|| format!("lecture de {} impossible", batteries_dir.display()))?
                    .filter_map(|e| e.ok().map(|e| e.path()))
                    .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("toml"))
                    .collect();
                paths.sort();
                for path in paths {
                    match Battery::load(&path) {
                        Ok(b) if b.project.name == dir_name => return Ok(b),
                        Ok(_) => {}
                        Err(err) => invalides.push((path, err)),
                    }
                }
            }
        }

        // 3. Batteries embarquées — Task 13 ajoutera cpool_day03 :
        //    le TOML + ses assets seront extraits dans un tempfile::TempDir
        //    pour que `root` puisse être résolu sur disque, puis matchées
        //    sur `dir_name` ici.
        let _ = embedded_batteries();

        let detail_invalides = if invalides.is_empty() {
            String::new()
        } else {
            let mut s = String::from("\nbatteries ignorées car invalides :");
            for (path, err) in &invalides {
                s.push_str(&format!("\n  - {} : {err:#}", path.display()));
            }
            s
        };
        bail!(
            "aucune batterie trouvée pour « {} » ({}){}",
            dir_name,
            dir.display(),
            detail_invalides
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
        if let Some(b) = &self.project.binary {
            ensure_chemin_rendu("binary", Path::new(b))?;
        }
        if self.project.kind == ProjectType::Functions {
            ensure!(
                !self.task.is_empty(),
                "projet « {} » de type functions : au moins une [[task]] est requise",
                self.project.name
            );
        }
        for t in &self.task {
            ensure_chemin_rendu("delivery", Path::new(&t.delivery))?;
            ensure_chemin_rendu("harness", &t.harness)?;
            if let Some(f) = &t.stdout_file {
                ensure_chemin_rendu("stdout_file", f)?;
            }
            for s in &t.extra_sources {
                ensure_chemin_rendu("extra_sources", s)?;
            }
            ensure!(
                !(t.stdout.is_some() && t.stdout_file.is_some()),
                "task « {} » : `stdout` et `stdout_file` sont mutuellement exclusifs",
                t.name
            );
            ensure!(
                (0..=255).contains(&t.exit_code),
                "task « {} » : `exit_code` hors 0..=255 ({})",
                t.name,
                t.exit_code
            );
            ensure!(
                t.timeout_ms > 0,
                "task « {} » : `timeout_ms` doit être non nul",
                t.name
            );
        }
        for t in &self.functional_test {
            ensure!(
                !(t.stdout.is_some() && t.stdout_file.is_some()),
                "functional_test « {} » : `stdout` et `stdout_file` sont mutuellement exclusifs",
                t.name
            );
            if let Some(f) = &t.stdout_file {
                ensure_chemin_rendu("stdout_file", f)?;
            }
            ensure!(
                (0..=255).contains(&t.exit_code),
                "functional_test « {} » : `exit_code` hors 0..=255 ({})",
                t.name,
                t.exit_code
            );
            ensure!(
                t.timeout_ms > 0,
                "functional_test « {} » : `timeout_ms` doit être non nul",
                t.name
            );
        }
        Ok(())
    }
}

/// Rejette un chemin de champ de batterie qui sortirait du rendu :
/// absolu ou contenant `..`. Sécurité : une moulinette.toml hostile
/// (versionnée dans le repo noté) ne doit pointer ni hors du rendu,
/// ni hors de la racine de la batterie.
fn ensure_chemin_rendu(champ: &str, valeur: &Path) -> Result<()> {
    ensure!(
        !valeur.is_absolute()
            && !valeur
                .components()
                .any(|c| matches!(c, Component::ParentDir)),
        "champ `{champ}` : chemin « {} » interdit (absolu ou hors du rendu)",
        valeur.display()
    );
    Ok(())
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
/// pointer vers un fichier inexistant. Publique pour le CLI
/// (Task 12 : `list` et `--battery <nom>`).
pub fn embedded_batteries() -> Vec<(&'static str, &'static str)> {
    // Task 13 ajoutera cpool_day03
    Vec::new()
}

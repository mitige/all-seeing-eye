//! Chargement et découverte des batteries TOML.

pub mod model;

use anyhow::{bail, ensure, Context, Result};
use include_dir::{include_dir, Dir};
use model::{FunctionalTest, ProjectMeta, ProjectType, Prototype, Task};
use serde::Deserialize;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use tempfile::TempDir;

/// Une batterie complète : métadonnées projet + exercices/tests,
/// racinée dans `root` (dossier contenant le TOML).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)] // ok avec `root`/`tempdir` en skip : absents du TOML
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
    /// Garde le TempDir d'extraction vivant pour une batterie
    /// embarquée ([`load_embedded`]) : `root` pointe DEDANS, il doit
    /// vivre aussi longtemps que la batterie. `Arc` parce que
    /// Battery est `Clone` (le CLI la clone dans le thread pipeline) :
    /// le dossier est supprimé au drop du dernier clone. `None` pour
    /// une batterie lue depuis le disque.
    #[serde(skip)]
    tempdir: Option<Arc<TempDir>>,
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

        // 3. Batteries embarquées : même règle de match
        //    (`project.name` == nom du dossier). Le TOML est sondé en
        //    mémoire pour le nom ; l'extraction sur disque
        //    ([`load_embedded`]) n'a lieu qu'en cas de match — le
        //    `root` doit être un vrai dossier (harness, expected), la
        //    batterie retournée détient le TempDir d'extraction.
        //    Asymétrie assumée avec le tier 2 : là, chaque TOML est
        //    chargé ET validé (Battery::load) ; ici on sonde le nom
        //    SANS valider (from_str seul) — la validation complète
        //    n'a lieu qu'en cas de match, dans load_embedded.
        for (nom, contenu) in embedded_batteries() {
            match toml::from_str::<Battery>(contenu) {
                Ok(sonde) if sonde.project.name == dir_name => match load_embedded(nom) {
                    // Ok(None) : inatteignable — `nom` vient
                    // d'embedded_batteries, le TOML existe forcément.
                    Ok(Some(b)) => return Ok(b),
                    Ok(None) => {}
                    Err(err) => invalides.push((pseudo_chemin_embarquee(nom), err)),
                },
                Ok(_) => {}
                Err(err) => invalides.push((pseudo_chemin_embarquee(nom), err.into())),
            }
        }

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

/// Le dossier `batteries/` du manifest, embarqué dans le binaire :
/// TOML à la racine + dossiers d'assets (`<nom>/harness/`,
/// `<nom>/expected/`…). Accessible sans installation sur disque.
static EMBEDDED: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/batteries");

/// Batteries embarquées dans le binaire : `(nom, contenu TOML)` pour
/// chaque `*.toml` à la racine de `batteries/`, triées par nom.
/// Publique pour le CLI (`list` et `--battery <nom>`).
pub fn embedded_batteries() -> Vec<(&'static str, &'static str)> {
    let mut out: Vec<(&'static str, &'static str)> = EMBEDDED
        .files()
        .filter(|f| f.path().extension().and_then(|e| e.to_str()) == Some("toml"))
        .filter_map(|f| {
            let nom = f.path().file_stem()?.to_str()?;
            let contenu = f.contents_utf8()?;
            Some((nom, contenu))
        })
        .collect();
    out.sort();
    out
}

/// Rejette un « nom » de batterie qui serait un chemin : un nom
/// embarqué ne doit jamais être résolu hors du contenu embarqué.
fn ensure_nom_simple(name: &str) -> Result<()> {
    ensure!(
        !name.is_empty() && name != "." && name != ".." && !name.contains(['/', '\\']),
        "nom de batterie invalide : « {name} » (un nom de fichier, pas un chemin)"
    );
    Ok(())
}

/// Pseudo-chemin d'une batterie embarquée, pour les messages
/// d'erreur (liste des invalides de [`Battery::discover`]).
fn pseudo_chemin_embarquee(nom: &str) -> PathBuf {
    PathBuf::from(format!("<embarquée:{nom}>"))
}

/// Extrait la batterie embarquée `name` — le TOML `<name>.toml` et
/// l'éventuel dossier d'assets `<name>/` — dans un TempDir neuf, et
/// renvoie ce TempDir (le TOML extrait est `<tempdir>/<name>.toml`).
/// Ok(None) si `name` n'est pas une batterie embarquée. `name` est un
/// NOM, jamais un chemin.
pub fn extract_embedded(name: &str) -> Result<Option<TempDir>> {
    ensure_nom_simple(name)?;
    let Some(toml) = EMBEDDED.get_file(format!("{name}.toml")) else {
        return Ok(None);
    };
    let tmp = tempfile::tempdir().context("création du tempdir d'extraction impossible")?;
    let dest_toml = tmp.path().join(format!("{name}.toml"));
    fs::write(&dest_toml, toml.contents())
        .with_context(|| format!("extraction de {} impossible", dest_toml.display()))?;
    if let Some(assets) = EMBEDDED.get_dir(name) {
        write_embedded_dir(assets, &tmp.path().join(name))?;
    }
    Ok(Some(tmp))
}

/// Recopie un dossier embarqué (fichiers + sous-dossiers) dans
/// `dest`. Le contenu est fixé à la compilation : la récursion est
/// bornée par la profondeur réelle de `batteries/`.
fn write_embedded_dir(dir: &Dir<'_>, dest: &Path) -> Result<()> {
    fs::create_dir_all(dest)
        .with_context(|| format!("création de {} impossible", dest.display()))?;
    for f in dir.files() {
        let Some(nom) = f.path().file_name() else {
            continue;
        };
        let d = dest.join(nom);
        fs::write(&d, f.contents())
            .with_context(|| format!("extraction de {} impossible", d.display()))?;
    }
    for sous in dir.dirs() {
        let Some(nom) = sous.path().file_name() else {
            continue;
        };
        write_embedded_dir(sous, &dest.join(nom))?;
    }
    Ok(())
}

/// Charge la batterie embarquée `name` : extraction dans un TempDir
/// ([`extract_embedded`]) puis [`Battery::load`] du TOML extrait. La
/// batterie retournée DÉTIENT le TempDir (`root` pointe dedans) : il
/// vit aussi longtemps qu'elle et ses clones. Ok(None) si `name`
/// n'est pas une batterie embarquée.
pub fn load_embedded(name: &str) -> Result<Option<Battery>> {
    let Some(tmp) = extract_embedded(name)? else {
        return Ok(None);
    };
    let path = tmp.path().join(format!("{name}.toml"));
    let mut b = Battery::load(&path)?;
    b.tempdir = Some(Arc::new(tmp));
    Ok(Some(b))
}

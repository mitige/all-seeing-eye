//! Types du schéma TOML d'une batterie all-seeing-eye.
//!
//! Ces types sont finaux : tout le reste du crate (runner, TUI, rapports)
//! les consomme tels quels.

use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ProjectType {
    Binary,
    Functions,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectMeta {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: ProjectType,
    pub binary: Option<String>, // requis si kind == Binary
    #[serde(default = "default_rules")]
    pub makefile_rules: Vec<String>,
    #[serde(default = "default_cflags")]
    pub cflags: Vec<String>,
    #[serde(default)]
    pub allowed_functions: Vec<String>, // vide = pas de check C-forbidden
    #[serde(default)]
    pub tests_run_rule: bool,
}

fn default_rules() -> Vec<String> {
    ["all", "clean", "fclean", "re"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

fn default_cflags() -> Vec<String> {
    ["-Wall", "-Wextra", "-Werror"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Prototype {
    pub name: String,
    pub signature: String,
    pub file: String,
}

/// Table `[build]` optionnelle d'une batterie (Rush2, CountIsland) :
/// préparation du rendu et build sur mesure en remplacement du make
/// standard. Table top-level, PAS dans `[project]`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildSpec {
    /// Commandes shell (`/bin/sh -c`) jouées dans la salle blanche
    /// AVANT le build, binary ou functions (ex. construire une lib :
    /// `cd lib/my && ./build.sh`).
    #[serde(default)]
    pub pre_commands: Vec<String>,
    /// Si présent : commande shell REMPLAÇANT `make fclean` + `make
    /// re` (binary). La présence du binaire annoncé reste vérifiée.
    pub command: Option<String>,
}

/// Type Functions : 1 exercice piscine.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Task {
    pub name: String,
    /// Fichier rendu (ex. "my_print_alpha.c"), ou DOSSIER dont tous
    /// les *.c directs sont livrés (Rush1 : "rush-1-1", Day12 : "cat").
    pub delivery: String,
    pub prototype: Option<String>,
    pub harness: PathBuf, // main fourni par la batterie
    #[serde(default)]
    pub extra_sources: Vec<PathBuf>, // ex. my_putchar.c officiel
    #[serde(default)]
    pub args: Vec<String>, // argv[1..] du harness compilé
    #[serde(default)]
    pub stdin: String, // écrit sur stdin du harness compilé
    #[serde(default)]
    pub include_dirs: Vec<String>, // deviennent -I<dir>, résolus en salle blanche
    #[serde(default)]
    pub link_flags: Vec<String>, // passés au link du harness
    pub stdout: Option<String>,
    pub stdout_file: Option<PathBuf>,
    #[serde(default)]
    pub stderr: String,
    #[serde(default)]
    pub exit_code: i32,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
}

/// Type Binary : 1 test end-to-end.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FunctionalTest {
    pub name: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub stdin: String,
    /// Source C compilé et linké avec le binaire produit (WorkshopLib :
    /// le produit est une `lib/libmy.a`, le test est ce main), relatif
    /// au TOML. Si absent, le binaire produit est exécuté directement.
    pub harness: Option<PathBuf>,
    #[serde(default)]
    pub link_flags: Vec<String>, // link du harness, résolus en salle blanche
    pub stdout: Option<String>,
    pub stdout_file: Option<PathBuf>,
    #[serde(default)]
    pub stderr: String,
    #[serde(default)]
    pub exit_code: i32,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
}

fn default_timeout() -> u64 {
    2000
}

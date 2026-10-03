//! Types du schéma TOML d'une batterie seeyou.
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
pub struct Prototype {
    pub name: String,
    pub signature: String,
    pub file: String,
}

/// Type Functions : 1 exercice piscine.
#[derive(Debug, Clone, Deserialize)]
pub struct Task {
    pub name: String,
    pub delivery: String, // ex. "my_print_alpha.c"
    pub prototype: Option<String>,
    pub harness: PathBuf, // main fourni par la batterie
    #[serde(default)]
    pub extra_sources: Vec<PathBuf>, // ex. my_putchar.c officiel
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
pub struct FunctionalTest {
    pub name: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub stdin: String,
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

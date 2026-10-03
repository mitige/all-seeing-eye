//! Modèle des fautes de norme Epitech.
//!
//! Le détecteur arrive en Task 5 ; ici, uniquement les types partagés
//! (rapport, événements, contexte du pipeline).

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Gravité d'une faute de norme.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Severity {
    Fatal,
    Major,
    Minor,
    Info,
}

/// Une faute de norme, localisée dans un fichier source.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NormeFault {
    pub file: PathBuf,
    pub line: u32,
    pub col: u32,
    pub severity: Severity,
    pub rule: String,
    pub message: String,
}

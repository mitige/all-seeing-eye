//! Modèle des fautes de norme Epitech et moteurs de détection.
//!
//! - [`official`] : epiclang + plugin banana (chemin par défaut quand
//!   epiclang est détecté) ;
//! - [`internal`] : moteur Rust de repli, règles mécaniques du coding
//!   style v7.1 avec les messages exacts de banana (calibrés).

pub mod internal;
pub mod official;

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Gravité d'une faute de norme.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Severity {
    Fatal,
    Major,
    Minor,
    Info,
}

impl Severity {
    /// Libellé banana (`Fatal`/`Major`/`Minor`/`Info`) → [`Severity`].
    pub fn from_banana(s: &str) -> Option<Severity> {
        match s {
            "Fatal" => Some(Severity::Fatal),
            "Major" => Some(Severity::Major),
            "Minor" => Some(Severity::Minor),
            "Info" => Some(Severity::Info),
            _ => None,
        }
    }

    /// Libellé minuscule pour les résumés d'étape.
    pub fn label(&self) -> &'static str {
        match self {
            Severity::Fatal => "fatal",
            Severity::Major => "major",
            Severity::Minor => "minor",
            Severity::Info => "info",
        }
    }
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

/// Profondeur maximale du walk des sources : au-delà, les
/// sous-dossiers sont ignorés. Traversée itérative (pile explicite) —
/// un arbre hostile ne peut pas déborder la pile d'appels.
const MAX_SCAN_DEPTH: usize = 64;

/// Chemins RELATIFS (à `target`) des fichiers à analyser : `.c`/`.h`,
/// plus `Makefile`/`*.mk` si `with_makefile` (moteur interne — banana
/// ne lit que le C). `.git` ignoré, symlinks jamais suivis, ordre
/// trié (déterministe), profondeur bornée à [`MAX_SCAN_DEPTH`].
pub(crate) fn collect_sources(target: &Path, with_makefile: bool) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![(target.to_path_buf(), 0usize)];
    while let Some((dir, depth)) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(ft) = entry.file_type() else {
                continue;
            };
            if ft.is_dir() {
                if entry.file_name() == ".git" || depth >= MAX_SCAN_DEPTH {
                    continue;
                }
                stack.push((entry.path(), depth + 1));
            } else if ft.is_file() {
                let path = entry.path();
                let Ok(rel) = path.strip_prefix(target) else {
                    continue;
                };
                match internal::kind_of(rel) {
                    Some(internal::SourceKind::Makefile) if !with_makefile => {}
                    Some(_) => out.push(rel.to_path_buf()),
                    None => {}
                }
            }
        }
    }
    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_depuis_banana() {
        assert_eq!(Severity::from_banana("Fatal"), Some(Severity::Fatal));
        assert_eq!(Severity::from_banana("Major"), Some(Severity::Major));
        assert_eq!(Severity::from_banana("Minor"), Some(Severity::Minor));
        assert_eq!(Severity::from_banana("Info"), Some(Severity::Info));
        assert_eq!(Severity::from_banana("Warning"), None);
        assert_eq!(Severity::from_banana(""), None);
    }

    #[test]
    fn severity_label_minuscule() {
        assert_eq!(Severity::Fatal.label(), "fatal");
        assert_eq!(Severity::Major.label(), "major");
        assert_eq!(Severity::Minor.label(), "minor");
        assert_eq!(Severity::Info.label(), "info");
    }

    #[test]
    fn collect_sources_filtre_et_trie() {
        let dir = tempfile::tempdir().unwrap();
        let w = |rel: &str| {
            let p = dir.path().join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(&p, "x").unwrap();
        };
        w("a.c");
        w("b.h");
        w("Makefile");
        w("sub/c.c");
        w("notes.txt");
        w(".git/hidden.c");

        let sans_makefile = collect_sources(dir.path(), false);
        assert_eq!(
            sans_makefile,
            vec![
                PathBuf::from("a.c"),
                PathBuf::from("b.h"),
                PathBuf::from("sub/c.c")
            ]
        );
        let avec = collect_sources(dir.path(), true);
        assert_eq!(
            avec,
            vec![
                PathBuf::from("Makefile"),
                PathBuf::from("a.c"),
                PathBuf::from("b.h"),
                PathBuf::from("sub/c.c")
            ]
        );
    }
}

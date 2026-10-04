//! Chemin officiel de la norme : `epiclang -fsyntax-only` (wrapper
//! clang avec le plugin banana chargé automatiquement).
//!
//! Chaque fichier est analysé dans le cwd = racine du rendu (pour que
//! les includes relatifs fonctionnent), avec un chemin RELATIF — la
//! sortie banana préfixe alors les fautes du chemin relatif, tel quel
//! dans le rapport. L'exit code reste 0 même avec des warnings ; les
//! lignes de contexte (`   6 | ...`) et le bilan (`N warnings
//! generated.`) ne matchent pas la regex et sont ignorés.

use crate::exec::{run_capture, ExecStatus, Limits};
use crate::norme::{NormeFault, Severity};
use anyhow::Result;
use regex::Regex;
use std::path::Path;
use std::sync::LazyLock;
use std::time::Duration;

/// Timeout d'une analyse epiclang (spec : 30 s).
const EPICLANG_TIMEOUT: Duration = Duration::from_secs(30);

/// Ligne d'avertissement banana :
/// `FICHIER:LIGNE:COL: warning: [Banana] [SEV] MESSAGE (C-RULE)`.
/// Compilée une fois (LazyLock). Le fichier est non-greedy : un chemin
/// contenant `:` resterait mal géré, sans importance ici (chemins de
/// rendu relatifs).
static BANANA_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^(.+?):(\d+):(\d+): warning: \[Banana\] \[(Fatal|Major|Minor|Info)\] (.+) \((C-[A-Z]\d+)\)$",
    )
    .expect("regex banana invalide")
});

/// `true` si `epiclang --version` est lançable (présence sur la
/// machine, via le PATH).
pub fn epiclang_available() -> bool {
    std::process::Command::new("epiclang")
        .arg("--version")
        .output()
        .is_ok()
}

/// Parse la sortie (stdout et/ou stderr) d'epiclang : une
/// [`NormeFault`] par ligne d'avertissement banana. Toute autre ligne
/// (contexte, bilan, erreurs clang) est ignorée.
pub fn parse_banana_output(output: &str) -> Vec<NormeFault> {
    output
        .lines()
        .filter_map(|line| {
            let cap = BANANA_RE.captures(line)?;
            Some(NormeFault {
                file: Path::new(&cap[1]).to_path_buf(),
                line: cap[2].parse().ok()?,
                col: cap[3].parse().ok()?,
                severity: Severity::from_banana(&cap[4])?,
                message: cap[5].to_string(),
                rule: cap[6].to_string(),
            })
        })
        .collect()
}

/// Résultat de l'analyse epiclang d'un fichier.
#[derive(Debug)]
pub struct FileOutcome {
    /// Fautes banana parsées (stdout + stderr).
    pub faults: Vec<NormeFault>,
    /// Exécution anormale (timeout, signal) : la faute n'est pas dans
    /// le code mais dans l'outil — l'étape le signale en check KO.
    pub note: Option<String>,
}

/// Analyse `rel` (chemin relatif à `target`) via
/// `epiclang -fsyntax-only`, cwd = `target`, borné à
/// [`EPICLANG_TIMEOUT`]. Parse stderr ET stdout (les warnings banana
/// vont sur stderr, calibré ; on parse les deux par robustesse).
pub fn check_one(target: &Path, rel: &Path) -> Result<FileOutcome> {
    let rel_str = rel.to_string_lossy();
    let outcome = run_capture(
        Path::new("epiclang"),
        &["-fsyntax-only".to_string(), rel_str.into_owned()],
        "",
        target,
        EPICLANG_TIMEOUT,
        Limits::default(),
    )?;
    let mut faults = parse_banana_output(&outcome.stderr);
    faults.extend(parse_banana_output(&outcome.stdout));
    let note = match outcome.status {
        ExecStatus::Exit(_) => None,
        ExecStatus::Signal(sig) => Some(format!("epiclang tué par le signal {sig}")),
        ExecStatus::Timeout => Some(format!(
            "epiclang timeout ({} s)",
            EPICLANG_TIMEOUT.as_secs()
        )),
    };
    Ok(FileOutcome { faults, note })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// Sortie réelle capturée (epiclang 21, plugin banana).
    const SAMPLE: &str = "\
dirty.c:6:17: warning: [Banana] [Minor] trailing space (C-G7)
    6 |     return b;    
      |                 ^
dirty.c:1:1: warning: [Banana] [Minor] file not starting with standard Epitech header (C-G1)
sub/defs.h:8:1: warning: [Banana] [Major] file not protected against double inclusion (C-H2)
big.c:58:1: warning: [Banana] [Major] 11th non-static and 11th function in the file (C-O3)
bad.c:11:2: warning: [Banana] [Info] file not ending with a newline (C-A3)
bad.c:12:1: warning: [Banana] [Fatal] some fatal (C-F6)
3 warnings generated.
";

    #[test]
    fn parse_les_lignes_banana_uniquement() {
        let f = parse_banana_output(SAMPLE);
        assert_eq!(f.len(), 6, "{f:?}");
        assert_eq!(f[0].file, PathBuf::from("dirty.c"));
        assert_eq!((f[0].line, f[0].col), (6, 17));
        assert_eq!(f[0].severity, Severity::Minor);
        assert_eq!(f[0].rule, "C-G7");
        assert_eq!(f[0].message, "trailing space");
        // Sous-dossier : chemin relatif conservé tel quel.
        assert_eq!(f[2].file, PathBuf::from("sub/defs.h"));
        assert_eq!(f[2].rule, "C-H2");
        // Message composé (C-O3) et Info.
        assert_eq!(
            f[3].message,
            "11th non-static and 11th function in the file"
        );
        assert_eq!(f[3].rule, "C-O3");
        assert_eq!(f[4].severity, Severity::Info);
        assert_eq!(f[4].message, "file not ending with a newline");
        assert_eq!(f[5].severity, Severity::Fatal);
        assert_eq!(f[5].rule, "C-F6");
    }

    #[test]
    fn parse_ignore_contexte_erreurs_clang_et_bilan() {
        let bruit = "\
    6 |     return b;
      |                 ^
dirty.c:3:1: error: expected ';' after expression
1 error generated.

";
        assert!(parse_banana_output(bruit).is_empty());
        assert!(parse_banana_output("").is_empty());
    }
}

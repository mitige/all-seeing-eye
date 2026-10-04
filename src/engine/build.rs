//! Étape Build — compilation en salle blanche.
//!
//! Le rendu de l'étudiant n'est JAMAIS modifié : tout le travail se
//! passe dans une « salle blanche » (tempdir) recevant une copie
//! filtrée de `ctx.target` (`.git`, `*.o`, `*~`, `#*#`, `*.swp`
//! ignorés — la moulinette part d'un clone propre). La salle blanche
//! et le binaire éventuel sont posés dans `ctx.build` pour les étapes
//! suivantes (Unit, Functional).
//!
//! - Projet Binary : vérification des cflags dans le Makefile (NON
//!   bloquante : la vraie moulinette laisse compiler puis pénalise),
//!   puis `make fclean` et `make re` (LC_ALL=C, timeout 120 s), puis
//!   présence du binaire annoncé par la batterie.
//! - Projet Functions : chaque delivery (`task.delivery`) est compilée
//!   en `.o` dans la salle blanche avec les cflags de la batterie —
//!   epiclang si demandé et détecté, sinon le compilateur des options.
//!   Un seul échec de compilation fait échouer l'étape (« the
//!   Autograder will not be able to correct your work »), mais toutes
//!   les deliveries sont compilées pour tout remonter d'un coup.
//!
//! Les sorties des sous-processus sont relayées ligne à ligne en
//! [`Event::LogLine`] pour le front.

use super::events::{Event, Step};
use super::{BuildArtifacts, PipelineContext, RunOpts};
use crate::battery::model::ProjectType;
use crate::exec::{run_capture, ExecStatus, Limits};
use anyhow::{Context, Result};
use regex::Regex;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

/// Timeout d'un `make fclean` / `make re` (spec : 120 s).
const MAKE_TIMEOUT: Duration = Duration::from_secs(120);

/// Timeout de la compilation d'une delivery (`cc -c` : instantané en
/// pratique, on reste borné).
const COMPILE_TIMEOUT: Duration = Duration::from_secs(60);

/// Nombre de lignes de stderr conservées dans le détail d'un KO.
const STDERR_TAIL: usize = 10;

/// Collecteur de checks : émet un [`Event::CheckFinished`] par
/// vérification et compte pour le résumé d'étape.
struct Collect<'a> {
    tx: &'a mpsc::Sender<Event>,
    total: usize,
    failed: usize,
}

impl Collect<'_> {
    /// Émet un check et renvoie son statut (pour l'`&=` de l'étape).
    fn check(&mut self, name: &str, ok: bool, detail: String) -> bool {
        self.total += 1;
        if !ok {
            self.failed += 1;
        }
        super::send(
            self.tx,
            Event::CheckFinished {
                step: Step::Build,
                name: name.to_string(),
                ok,
                detail,
            },
        );
        ok
    }
}

/// Étape 2 du pipeline : compilation en salle blanche (Binary ou
/// Functions selon le type de projet). Renvoie `true` si le build
/// passe — pour un Binary, le binaire doit en plus être présent.
pub fn run(ctx: &mut PipelineContext, tx: &mpsc::Sender<Event>) -> bool {
    super::step_started(tx, Step::Build);
    let dir = match stage_clean_room(&ctx.target) {
        Ok(dir) => dir,
        Err(e) => {
            super::step_finished(
                ctx,
                tx,
                Step::Build,
                false,
                format!("salle blanche : {e:#}"),
            );
            return false;
        }
    };
    ctx.build = Some(BuildArtifacts { dir, binary: None });
    // Chemin possédé : ctx.build est re-emprunté en mutable plus bas
    // (binaire) pendant que la salle blanche y vit.
    let white = ctx
        .build
        .as_ref()
        .expect("salle blanche posée à l'instant")
        .dir
        .path()
        .to_path_buf();
    let mut c = Collect {
        tx,
        total: 0,
        failed: 0,
    };
    let ok = match ctx.battery.project.kind {
        ProjectType::Binary => build_binary(ctx, &white, &mut c),
        ProjectType::Functions => build_functions(ctx, &white, &mut c),
    };
    let summary = if ok {
        format!("build ok ({} vérifications)", c.total)
    } else {
        format!(
            "build failed ({} vérifications, {} en échec)",
            c.total, c.failed
        )
    };
    super::step_finished(ctx, tx, Step::Build, ok, summary);
    ok
}

/// Crée la salle blanche : tempdir + copie filtrée de `target`.
/// En cas d'échec de copie, le tempdir est droppé — rien ne fuite.
fn stage_clean_room(target: &Path) -> Result<tempfile::TempDir> {
    let dir = tempfile::tempdir().context("création de la salle blanche impossible")?;
    copy_dir(target, dir.path()).context("copie du rendu en salle blanche impossible")?;
    Ok(dir)
}

/// `true` si `name` est ignoré par la copie en salle blanche :
/// `*.o`, `*~`, `#*#` (au moins deux caractères), `*.swp`.
fn is_ignored_name(name: &str) -> bool {
    name.ends_with('~')
        || (name.len() >= 2 && name.starts_with('#') && name.ends_with('#'))
        || [".o", ".swp"].iter().any(|ext| name.ends_with(ext))
}

/// Copie récursive de `src` dans `dst` (qui doit exister). Les entrées
/// `.git` et les fichiers ignorés ([`is_ignored_name`]) sont exclus.
/// Les symlinks et fichiers spéciaux (fifo, socket…) ne sont jamais
/// suivis ni copiés : seuls les fichiers réguliers traversent — pas
/// de lecture hors du rendu, pas de blocage sur un fifo.
fn copy_dir(src: &Path, dst: &Path) -> Result<()> {
    for entry in
        fs::read_dir(src).with_context(|| format!("lecture de {} impossible", src.display()))?
    {
        let entry = entry.with_context(|| format!("entrée illisible sous {}", src.display()))?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let ftype = entry
            .file_type()
            .with_context(|| format!("type de {} illisible", entry.path().display()))?;
        if name == ".git" {
            continue; // dossier .git, mais aussi gitfile d'un worktree
        }
        if ftype.is_dir() {
            let sub = dst.join(name.as_ref());
            fs::create_dir(&sub)
                .with_context(|| format!("création de {} impossible", sub.display()))?;
            copy_dir(&entry.path(), &sub)?;
        } else if ftype.is_file() && !is_ignored_name(&name) {
            let dest = dst.join(name.as_ref());
            fs::copy(entry.path(), &dest)
                .with_context(|| format!("copie vers {} impossible", dest.display()))?;
        }
    }
    Ok(())
}

/// Relaie chaque ligne d'un flux capturé en [`Event::LogLine`].
fn log_lines(tx: &mpsc::Sender<Event>, s: &str) {
    for line in s.lines() {
        super::send(
            tx,
            Event::LogLine {
                step: Step::Build,
                line: line.to_string(),
            },
        );
    }
}

/// Les `n` dernières lignes de `s` (trim fin), jointes par '\n'.
fn tail(s: &str, n: usize) -> String {
    let lines: Vec<&str> = s.trim_end().lines().collect();
    let start = lines.len().saturating_sub(n);
    lines[start..].join("\n")
}

/// Statut d'exécution en clair, pour le détail d'un KO.
fn describe_status(status: ExecStatus) -> String {
    match status {
        ExecStatus::Exit(code) => format!("exit {code}"),
        ExecStatus::Signal(sig) => format!("signal {sig}"),
        ExecStatus::Timeout => "timeout".to_string(),
    }
}

/// Lance `env LC_ALL=C make <rule>` dans `dir` (borné à
/// [`MAKE_TIMEOUT`]), relaie stdout/stderr en LogLine et émet le check
/// « make ». `true` si exit 0 ; tout le reste (exit ≠ 0, signal,
/// timeout, spawn en échec) est un KO.
fn run_make(dir: &Path, rule: &str, c: &mut Collect) -> bool {
    let args = vec!["LC_ALL=C".to_string(), "make".to_string(), rule.to_string()];
    let outcome = match run_capture(
        Path::new("/usr/bin/env"),
        &args,
        "",
        dir,
        MAKE_TIMEOUT,
        Limits::default(),
    ) {
        Ok(o) => o,
        Err(e) => {
            return c.check(
                "make",
                false,
                format!("build failed: make {rule} (lancement impossible : {e:#})"),
            );
        }
    };
    log_lines(c.tx, &outcome.stdout);
    log_lines(c.tx, &outcome.stderr);
    match outcome.status {
        ExecStatus::Exit(0) => c.check("make", true, format!("make {rule} ok")),
        status => {
            let stderr_tail = tail(&outcome.stderr, STDERR_TAIL);
            let detail = if stderr_tail.is_empty() {
                format!("build failed: make {rule} ({})", describe_status(status))
            } else {
                format!("build failed: make {rule}\n{stderr_tail}")
            };
            c.check("make", false, detail)
        }
    }
}

/// Vérifie que chaque cflag attendu apparaît dans le Makefile de la
/// salle blanche, comme un mot entouré de non-lettres (`-Wall` ne vaut
/// pas `-Wally`). Pénalise sans bloquer : le résultat n'est PAS intégré
/// au verdict de l'étape — la vraie moulinette laisse compiler puis
/// pénalise. Sans Makefile (déjà un KO de prelim) : un seul check KO
/// « missing Makefile » et la vérification des flags est court-circuitée.
fn check_cflags(ctx: &PipelineContext, dir: &Path, c: &mut Collect) {
    let content = match fs::read_to_string(dir.join("Makefile")) {
        Ok(content) => content,
        Err(e) if e.kind() == ErrorKind::NotFound => {
            c.check("makefile", false, "missing Makefile".to_string());
            return;
        }
        Err(e) => {
            c.check("makefile", false, format!("Makefile illisible : {e}"));
            return;
        }
    };
    for flag in &ctx.battery.project.cflags {
        // regex::escape rend le flag littéral : le pattern est toujours
        // valide, d'où l'expect.
        let re = Regex::new(&format!(r"(?:^|\W){}(?:\W|$)", regex::escape(flag)))
            .expect("flag échappé : regex toujours valide");
        if re.is_match(&content) {
            c.check("cflags", true, format!("cflag {flag} present"));
        } else {
            c.check("cflags", false, format!("missing cflag: {flag}"));
        }
    }
}

/// Build Binary : cflags (non bloquant), `make fclean`, `make re`,
/// puis présence du binaire annoncé par la batterie — répertorié dans
/// `ctx.build.binary`.
fn build_binary(ctx: &mut PipelineContext, dir: &Path, c: &mut Collect) -> bool {
    // Les flags d'abord : un flag manquant pénalise sans bloquer, et
    // le diagnostic « missing Makefile » précède le bruit de make.
    check_cflags(ctx, dir, c);
    for rule in ["fclean", "re"] {
        if !run_make(dir, rule, c) {
            return false;
        }
    }
    let Some(name) = &ctx.battery.project.binary else {
        // Inatteignable via Battery::load (validation) — défensif.
        return c.check(
            "binary",
            false,
            "batterie binary sans champ `binary`".to_string(),
        );
    };
    let path = dir.join(name);
    if !path.is_file() {
        return c.check("binary", false, format!("missing binary: {name}"));
    }
    ctx.build
        .as_mut()
        .expect("salle blanche posée à l'instant")
        .binary = Some(path);
    c.check("binary", true, format!("binary {name} present"))
}

/// `true` si epiclang est installé (sonde `--version`).
fn epiclang_present() -> bool {
    std::process::Command::new("epiclang")
        .arg("--version")
        .output()
        .is_ok()
}

/// Compilateur des deliveries Functions : epiclang si demandé par les
/// options ET détecté sur la machine, sinon `opts.compiler`.
fn select_compiler(opts: &RunOpts) -> PathBuf {
    if opts.use_epiclang && epiclang_present() {
        PathBuf::from("epiclang")
    } else {
        opts.compiler.clone()
    }
}

/// Compile une delivery en `.o` dans la salle blanche :
/// `<compiler> -c <cflags…> <delivery> -o <stem>.o` via
/// `env LC_ALL=C`, borné à [`COMPILE_TIMEOUT`]. Émet le check
/// « compile » — KO avec extrait stderr en cas d'échec.
fn compile_one(
    compiler: &Path,
    cflags: &[String],
    delivery: &str,
    dir: &Path,
    c: &mut Collect,
) -> bool {
    if !dir.join(delivery).is_file() {
        // Déjà un KO de prelim — ici la compilation est impossible.
        return c.check("compile", false, format!("missing delivery: {delivery}"));
    }
    let stem = Path::new(delivery)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(delivery);
    let obj = format!("{stem}.o");
    let mut args = vec![
        "LC_ALL=C".to_string(),
        compiler.to_string_lossy().into_owned(),
        "-c".to_string(),
    ];
    args.extend(cflags.iter().cloned());
    args.push(delivery.to_string());
    args.push("-o".to_string());
    args.push(obj.clone());
    let outcome = match run_capture(
        Path::new("/usr/bin/env"),
        &args,
        "",
        dir,
        COMPILE_TIMEOUT,
        Limits::default(),
    ) {
        Ok(o) => o,
        Err(e) => {
            return c.check(
                "compile",
                false,
                format!("{delivery}: lancement du compilateur impossible : {e:#}"),
            );
        }
    };
    log_lines(c.tx, &outcome.stdout);
    log_lines(c.tx, &outcome.stderr);
    match outcome.status {
        ExecStatus::Exit(0) if dir.join(&obj).is_file() => {
            c.check("compile", true, format!("{delivery} ok"))
        }
        ExecStatus::Exit(0) => c.check(
            "compile",
            false,
            format!("{delivery}: objet {obj} absent après une compilation réussie"),
        ),
        status => {
            let stderr_tail = tail(&outcome.stderr, STDERR_TAIL);
            let detail = if stderr_tail.is_empty() {
                format!("compile failed: {delivery} ({})", describe_status(status))
            } else {
                format!("compile failed: {delivery}\n{stderr_tail}")
            };
            c.check("compile", false, detail)
        }
    }
}

/// Build Functions : compile chaque delivery de la batterie en `.o`
/// dans la salle blanche. Un seul échec fait échouer l'étape (« the
/// Autograder will not be able to correct your work »), mais toutes
/// les deliveries sont compilées pour tout remonter d'un coup.
fn build_functions(ctx: &PipelineContext, dir: &Path, c: &mut Collect) -> bool {
    let compiler = select_compiler(&ctx.opts);
    let mut ok = true;
    for task in &ctx.battery.task {
        ok &= compile_one(
            &compiler,
            &ctx.battery.project.cflags,
            &task.delivery,
            dir,
            c,
        );
    }
    ok
}

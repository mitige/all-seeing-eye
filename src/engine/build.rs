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
//! Déviation assumée au plan initial (Functions) : le plan disait
//! « compiler les *.c de la racine + les extra_sources » ; on ne
//! compile QUE les `task.delivery`. C'est le bon choix : granularité
//! par exercice, et un source officiel livré en extra_sources (ex.
//! my_putchar.c) n'est pas compilé ici — il serait sinon analysé comme
//! un rendu et déclencherait à tort le check des fonctions interdites.
//! Contrat pour Task 7 (tests unitaires) : les `.o` produits sont un
//! par SOURCE de delivery, à la RACINE de la salle blanche, nommés
//! `<stem>.o` (stem = nom du fichier sans extension) — une delivery
//! DOSSIER (Rush1 : `rush-1-1/*`) produit donc un `.o` par `*.c`
//! direct. D'où la garde d'unicité des stems dans `build_functions`,
//! étendue à toutes les sources résolues.
//!
//! Les sorties des sous-processus sont relayées ligne à ligne en
//! [`Event::LogLine`] pour le front.

use super::events::{Event, Step};
use super::{tail, BuildArtifacts, Collect, PipelineContext, RunOpts, STDERR_TAIL};
use crate::battery::model::ProjectType;
use crate::exec::{run_capture, ExecStatus, Limits};
use anyhow::{bail, Context, Result};
use regex::Regex;
use std::collections::HashMap;
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

/// Profondeur maximale de la copie en salle blanche : au-delà, les
/// sous-dossiers sont ignorés. La traversée est itérative (pile
/// explicite) — un arbre hostile ne peut ni déborder la pile d'appels
/// ni faire copier des profondeurs absurdes.
const MAX_COPY_DEPTH: usize = 64;

/// Taille cumulée maximale copiée en salle blanche : au-delà, l'étape
/// échoue explicitement (« rendu trop gros ») plutôt que de remplir
/// le tempdir (souvent un tmpfs).
const MAX_COPY_BYTES: u64 = 512 * 1024 * 1024; // 512 Mio

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
                Vec::new(),
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
    let mut c = Collect::new(tx, Step::Build);
    // [build].pre_commands d'abord, quel que soit le type de projet
    // (binary ET functions) : elles préparent la salle blanche (ex.
    // construire une lib). Un échec arrête l'étape — construire par-
    // dessus une préparation ratée ne produirait que du bruit.
    let ok = run_pre_commands(&ctx.battery, &white, &mut c)
        && match ctx.battery.project.kind {
            ProjectType::Binary => build_binary(ctx, &white, &mut c),
            ProjectType::Functions => build_functions(ctx, &white, &mut c),
        };
    // Les KO non bloquants (ex. cflag manquant) restent visibles dans
    // le résumé — jamais masqués par un verdict global OK.
    let (total, failed, checks) = c.finish();
    let summary = format!(
        "build {} ({total} vérifications, {failed} en échec)",
        if ok { "ok" } else { "failed" },
    );
    super::step_finished(ctx, tx, Step::Build, ok, summary, checks);
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
//
// ⚠ Liste cousine de `is_forbidden_name` (prelim.rs) : si tu modifies
// cette liste, vérifie l'autre. La différence `.a`/`.so` est
// volontaire : prelim les signale comme fichiers interdits dans le
// rendu (C-O1), tandis que la salle blanche ne filtre que les
// artefacts qui parasiteraient la compilation — un `.a`/`.so` copié y
// reste inerte (jamais linké par `cc -c` ni par `make re`, qui ne
// consomment que les sources et règles du rendu).
fn is_ignored_name(name: &str) -> bool {
    name.ends_with('~')
        || (name.len() >= 2 && name.starts_with('#') && name.ends_with('#'))
        || [".o", ".swp"].iter().any(|ext| name.ends_with(ext))
}

/// Copie de `src` dans `dst` (qui doit exister), bornée par
/// [`MAX_COPY_DEPTH`] et [`MAX_COPY_BYTES`].
fn copy_dir(src: &Path, dst: &Path) -> Result<()> {
    copy_dir_capped(src, dst, MAX_COPY_DEPTH, MAX_COPY_BYTES)
}

/// Corps de [`copy_dir`], plafonds paramétrés pour les tests (le
/// plafond réel de 512 Mio serait trop lent à atteindre). Itératif —
/// pile explicite, jamais de récursion. Les entrées `.git` et les
/// fichiers ignorés ([`is_ignored_name`]) sont exclus. Les symlinks
/// et fichiers spéciaux (fifo, socket…) ne sont jamais suivis ni
/// copiés : seuls les fichiers réguliers traversent — pas de lecture
/// hors du rendu, pas de blocage sur un fifo.
fn copy_dir_capped(src: &Path, dst: &Path, max_depth: usize, max_bytes: u64) -> Result<()> {
    let mut copied: u64 = 0;
    // Pile des (source, destination, profondeur) restant à copier.
    let mut stack = vec![(src.to_path_buf(), dst.to_path_buf(), 0usize)];
    while let Some((src, dst, depth)) = stack.pop() {
        for entry in fs::read_dir(&src)
            .with_context(|| format!("lecture de {} impossible", src.display()))?
        {
            let entry =
                entry.with_context(|| format!("entrée illisible sous {}", src.display()))?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            let ftype = entry
                .file_type()
                .with_context(|| format!("type de {} illisible", entry.path().display()))?;
            if name == ".git" {
                continue; // dossier .git, mais aussi gitfile d'un worktree
            }
            if ftype.is_dir() {
                if depth >= max_depth {
                    continue; // arbre trop profond : ignoré (borne anti-abus)
                }
                let sub = dst.join(name.as_ref());
                fs::create_dir(&sub)
                    .with_context(|| format!("création de {} impossible", sub.display()))?;
                stack.push((entry.path(), sub, depth + 1));
            } else if ftype.is_file() && !is_ignored_name(&name) {
                let size = entry
                    .metadata()
                    .with_context(|| format!("taille de {} illisible", entry.path().display()))?
                    .len();
                if copied.saturating_add(size) > max_bytes {
                    bail!(
                        "rendu trop gros : la copie dépasserait le plafond de {:.1} Mio",
                        max_bytes as f64 / (1024.0 * 1024.0)
                    );
                }
                copied += size;
                let dest = dst.join(name.as_ref());
                fs::copy(entry.path(), &dest)
                    .with_context(|| format!("copie vers {} impossible", dest.display()))?;
            }
        }
    }
    Ok(())
}

/// Timeout d'une commande shell de `[build]` (pre_command ou commande
/// de build) : shell arbitraire, même borne qu'un make.
const SHELL_CMD_TIMEOUT: Duration = MAKE_TIMEOUT;

/// Lance les `[build].pre_commands` dans la salle blanche, AVANT tout
/// build. Le premier échec est un KO « pre_command failed: <cmd> » qui
/// fait échouer l'étape — les commandes suivantes et le build ne sont
/// pas tentés. Sans `[build]` : rien à faire, succès à vide.
fn run_pre_commands(battery: &crate::battery::Battery, dir: &Path, c: &mut Collect) -> bool {
    let Some(spec) = &battery.build else {
        return true;
    };
    for cmd in &spec.pre_commands {
        if !run_shell(dir, cmd, "pre_command", c) {
            return false;
        }
    }
    true
}

/// Exécute une ligne shell via `env LC_ALL=C /bin/sh -c <cmd>` dans
/// `dir` (bornée à [`SHELL_CMD_TIMEOUT`]), relaie stdout/stderr ligne
/// à ligne en LogLine et émet le check `name` : OK « <name> ok:
/// <cmd> », KO « <name> failed: <cmd> » (avec extrait stderr si
/// présent). Tout statut non nul, signal, timeout ou lancement
/// impossible est un KO.
fn run_shell(dir: &Path, cmd: &str, name: &str, c: &mut Collect) -> bool {
    let args = vec![
        "LC_ALL=C".to_string(),
        "/bin/sh".to_string(),
        "-c".to_string(),
        cmd.to_string(),
    ];
    let outcome = match run_capture(
        Path::new("/usr/bin/env"),
        &args,
        "",
        dir,
        SHELL_CMD_TIMEOUT,
        Limits::default(),
    ) {
        Ok(o) => o,
        Err(e) => {
            return c.check(
                name,
                false,
                format!("{name} failed: {cmd} (lancement impossible : {e:#})"),
            );
        }
    };
    c.log(&outcome.stdout);
    c.log(&outcome.stderr);
    match outcome.status {
        ExecStatus::Exit(0) => c.check(name, true, format!("{name} ok: {cmd}")),
        status => {
            let stderr_tail = tail(&outcome.stderr, STDERR_TAIL);
            let detail = if stderr_tail.is_empty() {
                format!("{name} failed: {cmd} ({})", describe_status(status))
            } else {
                format!("{name} failed: {cmd}\n{stderr_tail}")
            };
            c.check(name, false, detail)
        }
    }
}

/// Statut d'exécution en clair, pour le détail d'un KO.
/// Partagé avec unit.rs (Task 8 : KO de `make tests_run`).
pub(crate) fn describe_status(status: ExecStatus) -> String {
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
    c.log(&outcome.stdout);
    c.log(&outcome.stderr);
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
/// `ctx.build.binary`. Avec `[build].command` (Rush2) : la commande
/// shell REMPLACE make fclean/re — les checks de règles Makefile sont
/// skippés (voir prelim.rs), le check des cflags n'est appliqué que
/// si un Makefile existe, et la présence du binaire produit reste
/// vérifiée à l'identique.
fn build_binary(ctx: &mut PipelineContext, dir: &Path, c: &mut Collect) -> bool {
    let commande = ctx.battery.build.as_ref().and_then(|b| b.command.clone());
    match commande {
        Some(cmd) => {
            // Build sur mesure : sans Makefile, rien à sonder ; avec
            // un Makefile, les cflags pénalisent sans bloquer (comme
            // sur le chemin make).
            if dir.join("Makefile").is_file() {
                check_cflags(ctx, dir, c);
            }
            if !run_shell(dir, &cmd, "build", c) {
                return false;
            }
        }
        None => {
            // Les flags d'abord : un flag manquant pénalise sans
            // bloquer, et le diagnostic « missing Makefile » précède
            // le bruit de make.
            check_cflags(ctx, dir, c);
            for rule in ["fclean", "re"] {
                if !run_make(dir, rule, c) {
                    return false;
                }
            }
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

/// `true` si epiclang est installé : sonde `--version` bornée à 5 s
/// via [`run_capture`] — le même traitement que la sonde epiclang de
/// la norme ([`crate::norme::official::epiclang_available`]) et que
/// les sondes compilateur du CLI : un binaire pendu ne doit pas figer
/// le pipeline.
fn epiclang_present() -> bool {
    run_capture(
        Path::new("epiclang"),
        &["--version".to_string()],
        "",
        Path::new("."),
        Duration::from_secs(5),
        Limits::default(),
    )
    .is_ok()
}

/// Compilateur des deliveries Functions : epiclang si demandé par les
/// options ET détecté sur la machine, sinon `opts.compiler`.
/// Partagé avec functional.rs (Task 9 : compile delivery + harness).
pub(crate) fn select_compiler(opts: &RunOpts) -> PathBuf {
    if opts.use_epiclang && epiclang_present() {
        PathBuf::from("epiclang")
    } else {
        opts.compiler.clone()
    }
}

/// Stem d'une delivery : nom de fichier sans extension (« a/foo.c » →
/// « foo ») — le nom du `.o` produit à la racine de la salle blanche.
/// Partagé avec symbols.rs (Task 7) : c'est lui qui consomme les `.o`.
pub(crate) fn stem_of(delivery: &str) -> &str {
    Path::new(delivery)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(delivery)
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
    let obj = format!("{}.o", stem_of(delivery));
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
    c.log(&outcome.stdout);
    c.log(&outcome.stderr);
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
/// dans la salle blanche — une delivery DOSSIER (Rush1 : `rush-1-1`)
/// livre tous ses *.c directs, chacun compilé en son `.o`. Un seul
/// échec fait échouer l'étape (« the Autograder will not be able to
/// correct your work »), mais toutes les deliveries sont compilées
/// pour tout remonter d'un coup.
fn build_functions(ctx: &PipelineContext, dir: &Path, c: &mut Collect) -> bool {
    let compiler = select_compiler(&ctx.opts);
    // Résolution des deliveries AVANT la garde des stems : un dossier
    // livre plusieurs sources, chacune produit son .o. Une delivery
    // irrésoluble (dossier sans .c) est un KO « compile » — prelim
    // l'a déjà signalé, la compilation est impossible — sans empêcher
    // les autres deliveries d'être compilées.
    let mut sources: Vec<Vec<String>> = Vec::new();
    let mut ok = true;
    for task in &ctx.battery.task {
        match crate::battery::delivery_sources(dir, &task.delivery) {
            Ok(srcs) => sources.push(
                srcs.iter()
                    .map(|s| s.to_string_lossy().into_owned())
                    .collect(),
            ),
            Err(e) => {
                ok &= c.check("compile", false, format!("{e:#}"));
            }
        }
    }
    // Garde d'unicité des stems (étendue aux sources des dossiers) :
    // deux sources de même stem (« a/foo.c » et « b/foo.c »)
    // produiraient le même `.o` à la racine de la salle blanche — le
    // second écraserait silencieusement le premier. KO explicite avant
    // toute compilation.
    let mut vus: HashMap<&str, &str> = HashMap::new();
    for srcs in &sources {
        for s in srcs {
            let stem = stem_of(s);
            if let Some(autre) = vus.insert(stem, s.as_str()) {
                return c.check(
                    "compile",
                    false,
                    format!(
                        "collision de .o : « {autre} » et « {s} » produisent tous les deux {stem}.o"
                    ),
                );
            }
        }
    }
    for srcs in &sources {
        for s in srcs {
            ok &= compile_one(&compiler, &ctx.battery.project.cflags, s, dir, c);
        }
    }
    ok
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Chaîne de `n` sous-dossiers « d » sous `root` ; renvoie le
    /// chemin du plus profond.
    fn deep_chain(root: &Path, n: usize) -> PathBuf {
        let mut p = root.to_path_buf();
        for _ in 0..n {
            p.push("d");
        }
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn copy_dir_itere_sans_deborder_et_cappe_la_profondeur() {
        let src = TempDir::new().unwrap();
        let dst = TempDir::new().unwrap();
        let deep = deep_chain(src.path(), 100);
        fs::write(deep.join("trop_profond.txt"), "x").unwrap();
        fs::write(src.path().join("racine.txt"), "x").unwrap();

        // 100 niveaux : pas de débordement de pile (itératif).
        copy_dir(src.path(), dst.path()).unwrap();

        assert!(dst.path().join("racine.txt").is_file());
        // Les dossiers au-delà du cap ne sont pas copiés.
        let mut p = dst.path().to_path_buf();
        let mut depth = 0;
        while p.join("d").is_dir() {
            p.push("d");
            depth += 1;
        }
        assert_eq!(depth, MAX_COPY_DEPTH, "profondeur copiée : {depth}");
        assert!(!p.join("trop_profond.txt").exists());
    }

    #[test]
    fn copy_dir_rendu_trop_gros_ko_explicite() {
        let src = TempDir::new().unwrap();
        let dst = TempDir::new().unwrap();
        fs::write(src.path().join("gros.bin"), vec![0u8; 2048]).unwrap();

        // Plafond paramétré (le réel est 512 Mio — trop lent en test).
        let err = copy_dir_capped(src.path(), dst.path(), MAX_COPY_DEPTH, 1024).unwrap_err();
        assert!(
            format!("{err:#}").contains("rendu trop gros"),
            "KO 'rendu trop gros' absent : {err:#}"
        );
    }
}

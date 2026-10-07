//! Étape Functional — tests fonctionnels end-to-end (Task 9).
//!
//! Cœur « zéro pitié » du pipeline, partagé Binary/Functions : chaque
//! test est exécuté dans la salle blanche et sa sortie comparée AU
//! CARACTÈRE PRÈS (stdout d'abord, puis stderr, puis exit code) par
//! [`run_one`]. Un écart produit `Failed` avec un diff unified —
//! lignes `-attendu` / `+obtenu` (similar) que le panneau détail du
//! TUI colorera. Timeout → `Timeout`, mort par signal → `Crashed(n)`.
//! Une capture tronquée (borne de 4 Mio d'exec.rs) ne peut JAMAIS
//! conclure un Passed : tout match sur une capture incomplète est un
//! KO explicite — on ne peut pas conclure sur du partiel.
//!
//! - Projet Binary : le binaire compilé par le build
//!   (`ctx.build.binary`) est lancé pour chaque
//!   `battery.functional_test`, avec ses args et son stdin. Si le test
//!   porte un `harness` (WorkshopLib : le produit est une lib), le
//!   harness est compilé et linké avec le produit
//!   (`<compiler> <cflags…> <harness> <link_flags…> <produit> -o
//!   <salle blanche>/harness_test_<n>`) et C'EST lui qui est exécuté ;
//!   sinon un produit non exécutable est un KO clair.
//! - Projet Functions : pour chaque `battery.task`, la delivery —
//!   fichier unique, ou tous les *.c directs d'un dossier (Rush1) —
//!   est d'abord compilée et liée avec le harness et les
//!   extra_sources de la batterie, plus les `-I` de `include_dirs` et
//!   les `link_flags` (`<compiler> <cflags…> <sources…> <harness>
//!   <extra…> <-I…> <link_flags…> -o <salle blanche>/<stem>_test` —
//!   même sélection de compilateur que build.rs) ; une compile KO est
//!   le verdict du test — `Failed` dont le diff porte l'extrait
//!   stderr du compilateur — sans exécution. Sinon le binaire de test
//!   est exécuté avec les `args` et le `stdin` de la task.
//!
//! Chaque test émet `TestStarted` AVANT l'exécution (le front voit un
//! test qui pend), puis `TestFinished`, et alimente `ctx.tests` pour
//! le rapport. Les sorties des exécutions ne sont PAS relayées en
//! LogLine : le verdict `Failed` porte tout le nécessaire (diff,
//! expected, got) ; seules les compiles Functions relayent leur
//! sortie (comme build.rs).
//!
//! V1 : tous les tests s'exécutent avec cwd = salle blanche. Un cwd
//! dédié par test (fixtures) n'est PAS implémenté — le jour où la
//! batterie portera un champ `cwd`, il sera relatif à la salle
//! blanche.
//!
//! Cas particuliers :
//! - batterie sans `functional_test` (Binary) ou sans `task`
//!   (Functions) → étape désactivée : `StepFinished ok:true,
//!   skipped:true` ([`super::step_disabled`]) ;
//! - build absent (ctx construit à la main — en pipeline, `run_step`
//!   garantit un build OK avant Functional) → KO explicite, jamais
//!   un vert à vide ; idem Binary dont le build n'a pas répertorié
//!   de binaire.

use super::events::{Event, Step, TestVerdict};
use super::{tail, Collect, PipelineContext, STDERR_TAIL};
use crate::battery::model::{FunctionalTest, ProjectType, Task};
use crate::battery::Battery;
use crate::exec::{run_capture, ExecOutcome, ExecStatus, Limits, MAX_CAPTURE_BYTES};
use similar::{ChangeTag, TextDiff};
use std::borrow::Cow;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

/// Timeout de la compile + link d'une task Functions (comme la
/// compile des deliveries de build.rs : instantané en pratique, on
/// reste borné).
const COMPILE_TIMEOUT: Duration = Duration::from_secs(60);

/// Deadline du calcul de diff : Myers est quadratique sur de grosses
/// entrées très différentes ; au-delà, similar abandonne
/// l'optimalité et approxime (le diff reste complet).
const DIFF_DEADLINE: Duration = Duration::from_millis(500);

/// Seuil de taille cumulée (expected + got) au-delà duquel les
/// entrées du diff sont abrégées AVANT le calcul ([`abridge`]) : un
/// diff n'a pas à brasser des captures géantes.
const DIFF_ABRIDGE_BYTES: usize = 1024 * 1024;

/// Lignes conservées en tête d'une entrée abrégée.
const DIFF_ABRIDGE_HEAD: usize = 200;

/// Lignes conservées en queue d'une entrée abrégée.
const DIFF_ABRIDGE_TAIL: usize = 50;

/// Borne de rendu du diff, en lignes : le diff est cloné dans
/// l'event `TestFinished` ET dans le `TestRecord` du rapport —
/// jamais de full-dump de plusieurs Mio.
const DIFF_MAX_LINES: usize = 500;

/// Borne de rendu du diff, en octets : le cap en lignes ne suffit
/// pas — 500 lignes de plusieurs Mio chacune resteraient géantes.
const DIFF_MAX_BYTES: usize = 256 * 1024;

/// Étape 6 du pipeline : tests fonctionnels. Renvoie `true` ssi tous
/// les tests sont Passed (désactivation exceptée).
pub fn run(ctx: &mut PipelineContext, tx: &mpsc::Sender<Event>) -> bool {
    super::step_started(tx, Step::Functional);

    // Rien à jouer : skip de config — ni échec, ni skip de dépendance.
    // (Functions sans task est inatteignable via Battery::load —
    // validation — branche défensive pour les ctx construits à la
    // main.)
    let rien_a_jouer = match ctx.battery.project.kind {
        ProjectType::Binary => ctx.battery.functional_test.is_empty(),
        ProjectType::Functions => ctx.battery.task.is_empty(),
    };
    if rien_a_jouer {
        super::step_disabled(
            ctx,
            tx,
            Step::Functional,
            "skipped: aucun test fonctionnel".to_string(),
        );
        return true;
    }

    let mut c = Collect::new(tx, Step::Functional);

    // Sans salle blanche, rien à lancer : KO explicite (en pipeline,
    // run_step a déjà skip Functional si le build a échoué — ce cas ne
    // vient que d'un ctx construit à la main).
    let Some(build) = &ctx.build else {
        c.check(
            "build",
            false,
            "pas de salle blanche (build absent)".to_string(),
        );
        let (_, _, checks) = c.finish();
        super::step_finished(
            ctx,
            tx,
            Step::Functional,
            false,
            "build absent".to_string(),
            checks,
        );
        return false;
    };
    // Chemins et config possédés : ctx est re-emprunté en mutable dans
    // les boucles (ctx.tests) pendant que la salle blanche y vit.
    let white = build.dir.path().to_path_buf();
    let binary = build.binary.clone();
    let battery = ctx.battery.clone();
    let compiler = super::build::select_compiler(&ctx.opts);

    let (total, failed) = match battery.project.kind {
        ProjectType::Binary => match binary {
            Some(binary) => run_binary(&battery, &compiler, &binary, &white, ctx, tx, &mut c),
            None => {
                c.check(
                    "binary",
                    false,
                    "binaire absent (build sans produit répertorié)".to_string(),
                );
                let (_, _, checks) = c.finish();
                super::step_finished(
                    ctx,
                    tx,
                    Step::Functional,
                    false,
                    "binaire absent".to_string(),
                    checks,
                );
                return false;
            }
        },
        ProjectType::Functions => run_functions(&battery, &compiler, &white, ctx, tx, &mut c),
    };

    let ok = c.check(
        "tests",
        failed == 0,
        format!("{} passed, {failed} failed", total - failed),
    );
    let summary = format!("{total} tests, {failed} en échec");
    let (_, _, checks) = c.finish();
    super::step_finished(ctx, tx, Step::Functional, ok, summary, checks);
    ok
}

/// Exécute `cmd args` dans `cwd` (stdin fourni, borné à `timeout`) et
/// compare sa sortie à l'attendu AU CARACTÈRE PRÈS : stdout d'abord,
/// puis stderr, puis exit code — le premier écart est un `Failed` avec
/// diff unified (`-attendu` / `+obtenu`). Timeout → `Timeout` ; mort
/// par signal → `Crashed(n)`. Si tout matche mais qu'une capture est
/// tronquée (surplus jeté à la borne de 4 Mio), KO quand même : un
/// Passed sur une capture incomplète est interdit.
// Signature plate imposée par la spec (Task 9) : les trois attendus
// et le timeout sont des paramètres distincts du test à jouer.
#[allow(clippy::too_many_arguments)]
pub fn run_one(
    cmd: &Path,
    args: &[String],
    stdin: &str,
    cwd: &Path,
    expected_stdout: &str,
    expected_stderr: &str,
    expected_code: i32,
    timeout: Duration,
) -> (TestVerdict, ExecOutcome) {
    let outcome = match run_capture(cmd, args, stdin, cwd, timeout, Limits::default()) {
        Ok(outcome) => outcome,
        Err(e) => {
            // Binaire introuvable, permission refusée… : Failed
            // explicite. L'ExecOutcome synthétique porte Exit(-1) —
            // code impossible pour un vrai processus, donc repérable.
            return (
                TestVerdict::Failed {
                    diff: format!("lancement de {} impossible : {e:#}", cmd.display()),
                    expected: expected_stdout.to_string(),
                    got: String::new(),
                },
                ExecOutcome {
                    stdout: String::new(),
                    stderr: String::new(),
                    stdout_truncated: false,
                    stderr_truncated: false,
                    status: ExecStatus::Exit(-1),
                    elapsed: Duration::ZERO,
                },
            );
        }
    };
    let verdict = match outcome.status {
        ExecStatus::Timeout => TestVerdict::Timeout,
        ExecStatus::Signal(sig) => TestVerdict::Crashed(sig),
        ExecStatus::Exit(code) => verdict_on_exit(
            &outcome,
            code,
            expected_stdout,
            expected_stderr,
            expected_code,
        ),
    };
    (verdict, outcome)
}

/// Verdict d'un processus sorti normalement : garde troncature AVANT
/// tout calcul de diff, puis stdout, stderr, exit code — le premier
/// écart est un `Failed`. Enfin, la garde troncature du cas « tout
/// matche » : tout match sur une capture bornée reste un KO.
fn verdict_on_exit(
    outcome: &ExecOutcome,
    code: i32,
    expected_stdout: &str,
    expected_stderr: &str,
    expected_code: i32,
) -> TestVerdict {
    // Une capture tronquée qui diffère de l'attendu ne produirait
    // qu'un diff géant (jusqu'à 4 Mio) sur des données partielles —
    // verdict « sortie tronquée » direct, sans calcul de diff.
    if (outcome.stdout_truncated && outcome.stdout != expected_stdout)
        || (outcome.stderr_truncated && outcome.stderr != expected_stderr)
    {
        return verdict_sortie_tronquee(outcome, expected_stdout, expected_stderr);
    }
    if outcome.stdout != expected_stdout {
        return TestVerdict::Failed {
            diff: render_diff(expected_stdout, &outcome.stdout),
            expected: expected_stdout.to_string(),
            got: outcome.stdout.clone(),
        };
    }
    if outcome.stderr != expected_stderr {
        return TestVerdict::Failed {
            diff: render_diff(expected_stderr, &outcome.stderr),
            expected: expected_stderr.to_string(),
            got: outcome.stderr.clone(),
        };
    }
    if code != expected_code {
        return TestVerdict::Failed {
            diff: format!("exit code : attendu {expected_code}, obtenu {code}"),
            expected: expected_code.to_string(),
            got: code.to_string(),
        };
    }
    if outcome.stdout_truncated || outcome.stderr_truncated {
        return verdict_sortie_tronquee(outcome, expected_stdout, expected_stderr);
    }
    TestVerdict::Passed
}

/// Verdict « sortie tronquée » : la capture bornée à
/// [`MAX_CAPTURE_BYTES`] ne permet ni un Passed (données partielles)
/// ni un diff fiable. expected/got : le premier flux tronqué, celui
/// dont la capture est incomplète.
fn verdict_sortie_tronquee(
    outcome: &ExecOutcome,
    expected_stdout: &str,
    expected_stderr: &str,
) -> TestVerdict {
    let mut flux = Vec::new();
    if outcome.stdout_truncated {
        flux.push("stdout");
    }
    if outcome.stderr_truncated {
        flux.push("stderr");
    }
    let (expected, got) = if outcome.stdout_truncated {
        (expected_stdout, outcome.stdout.as_str())
    } else {
        (expected_stderr, outcome.stderr.as_str())
    };
    TestVerdict::Failed {
        diff: format!(
            "sortie tronquée (capture bornée à {} Mio, {}) — impossible de conclure un Passed",
            MAX_CAPTURE_BYTES / (1024 * 1024),
            flux.join(" et "),
        ),
        expected: expected.to_string(),
        got: got.to_string(),
    }
}

/// Diff unified texte de `expected` vs `got` : lignes `-attendu` /
/// `+obtenu` / ` contexte` (le panneau détail du TUI les colorera).
/// Une ligne sans terminaison est suivie du marqueur « \ pas de
/// newline final » — sinon le manque du newline final serait
/// invisible (« abc\n » vs « abc »).
///
/// Quatre bornes protègent le rendu d'entrées arbitrairement grosses :
/// - au-delà d'[`DIFF_ABRIDGE_BYTES`] cumulés, les entrées sont
///   abrégées ([`abridge`]) AVANT le calcul ;
/// - le calcul est borné par [`DIFF_DEADLINE`] (similar approxime
///   au-delà — le diff reste complet, non nécessairement minimal) ;
/// - le rendu est capé à [`DIFF_MAX_LINES`] lignes puis à
///   [`DIFF_MAX_BYTES`] octets (il est cloné dans l'event
///   `TestFinished` et le `TestRecord` du rapport — le cap lignes
///   seul laisserait passer 500 lignes géantes).
///
/// Les caractères de contrôle des lignes sont échappés
/// ([`escape_controls`]) : un `\r` sinon invisible rendrait un écart
/// CRLF/LF muet.
fn render_diff(expected: &str, got: &str) -> String {
    let (expected, got) = if expected.len() + got.len() > DIFF_ABRIDGE_BYTES {
        (abridge(expected), abridge(got))
    } else {
        (Cow::Borrowed(expected), Cow::Borrowed(got))
    };
    let diff = TextDiff::configure()
        .timeout(DIFF_DEADLINE)
        .diff_lines(expected.as_ref(), got.as_ref());
    // Compte total des lignes de diff (itérateur paresseux, rien de
    // construit) pour le marqueur de troncature.
    let total = diff.iter_all_changes().count();
    let mut out = String::new();
    for change in diff.iter_all_changes().take(DIFF_MAX_LINES) {
        let sign = match change.tag() {
            ChangeTag::Delete => '-',
            ChangeTag::Insert => '+',
            ChangeTag::Equal => ' ',
        };
        out.push(sign);
        out.push_str(&escape_controls(change.value()));
        if change.missing_newline() {
            out.push('\n');
            out.push_str("\\ pas de newline final\n");
        }
    }
    if total > DIFF_MAX_LINES {
        let _ = writeln!(out, "… diff tronqué, {total} lignes au total …");
    }
    if cap_octets(&mut out, DIFF_MAX_BYTES) {
        let _ = writeln!(out, "\n… diff tronqué à {} Kio …", DIFF_MAX_BYTES / 1024);
    }
    out
}

/// Tronque `out` à `max` octets (sur une frontière de caractère) si
/// besoin. Renvoie `true` si tronqué.
fn cap_octets(out: &mut String, max: usize) -> bool {
    if out.len() <= max {
        return false;
    }
    let mut fin = max;
    while !out.is_char_boundary(fin) {
        fin -= 1;
    }
    out.truncate(fin);
    true
}

/// Abrège une entrée de diff géante : tête de [`DIFF_ABRIDGE_HEAD`]
/// lignes, marqueur « … N lignes omises … », queue de
/// [`DIFF_ABRIDGE_TAIL`] lignes. Une entrée déjà courte passe telle
/// quelle.
///
/// Découpe par `split_inclusive('\n')` et NON `str::lines()` : ce
/// dernier strippe le `\r` final de chaque ligne CRLF — le marqueur
/// ␍ ([`escape_controls`]) doit survivre au chemin abrégé. Chaque
/// tronçon garde sa terminaison : la concaténation tête + marqueur +
/// queue est exacte, newline final compris.
fn abridge(s: &str) -> Cow<'_, str> {
    let total = s.split_inclusive('\n').count();
    if total <= DIFF_ABRIDGE_HEAD + DIFF_ABRIDGE_TAIL {
        return Cow::Borrowed(s);
    }
    let omises = total - DIFF_ABRIDGE_HEAD - DIFF_ABRIDGE_TAIL;
    let mut lignes = s.split_inclusive('\n');
    let mut out = String::new();
    for ligne in lignes.by_ref().take(DIFF_ABRIDGE_HEAD) {
        out.push_str(ligne);
    }
    // La tête finit par '\n' (total > HEAD : sa dernière ligne n'est
    // pas la dernière du string) : le marqueur ouvre sa propre ligne.
    let _ = writeln!(out, "… {omises} lignes omises …");
    // `split_inclusive` est double-entrée : la queue se lit à
    // l'envers depuis la fin de l'itérateur restant, puis se remet à
    // l'endroit.
    let queue: Vec<&str> = lignes.rev().take(DIFF_ABRIDGE_TAIL).collect();
    for ligne in queue.iter().rev() {
        out.push_str(ligne);
    }
    Cow::Owned(out)
}

/// Rend visibles les caractères de contrôle d'une ligne de diff :
/// `\r` → `␍`, les autres contrôles < 0x20 (hors `\n`, la
/// terminaison) → `\xNN`. La tabulation est conservée telle quelle :
/// légitime dans une sortie C.
fn escape_controls(s: &str) -> Cow<'_, str> {
    // Chemin rapide : rien à échapper (les octets < 0x20 ne sont
    // jamais des octets de continuation UTF-8, le scan par octets est
    // exact).
    if !s.bytes().any(|b| b < 0x20 && b != b'\n' && b != b'\t') {
        return Cow::Borrowed(s);
    }
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\r' => out.push('␍'),
            '\n' | '\t' => out.push(c),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\x{:02x}", c as u32);
            }
            c => out.push(c),
        }
    }
    Cow::Owned(out)
}

/// Un `Failed` sans sortie à comparer (compile KO, stdout attendu
/// illisible) : tout le détail vit dans le diff.
fn failed_sans_sortie(diff: String) -> TestVerdict {
    TestVerdict::Failed {
        diff,
        expected: String::new(),
        got: String::new(),
    }
}

/// Clôt un test : `TestFinished` émis, `TestRecord` empilé dans
/// `ctx.tests` pour le rapport, groupe « functional ». (`TestStarted`
/// part AVANT l'exécution, dans les boucles d'étape — le front voit
/// un test qui pend.)
fn emit_test(
    ctx: &mut PipelineContext,
    tx: &mpsc::Sender<Event>,
    name: &str,
    verdict: &TestVerdict,
) {
    super::send(
        tx,
        Event::TestFinished {
            group: Step::Functional.name().to_string(),
            name: name.to_string(),
            result: verdict.clone(),
        },
    );
    ctx.tests.push(super::verdict::test_record(
        Step::Functional.name(),
        name,
        verdict,
    ));
}

/// Binary : exécute le binaire du build pour chaque
/// `battery.functional_test`, dans la salle blanche. Si le test porte
/// un `harness` (WorkshopLib : le produit est une `lib/libmy.a`), le
/// harness est d'abord compilé et linké avec le binaire produit et
/// C'EST ce binaire de test qui est exécuté ; sinon le binaire
/// produit doit être directement exécutable — une lib sans harness
/// est un KO clair. Renvoie `(total, en échec)`.
fn run_binary(
    battery: &Battery,
    compiler: &Path,
    binary: &Path,
    white: &Path,
    ctx: &mut PipelineContext,
    tx: &mpsc::Sender<Event>,
    c: &mut Collect,
) -> (usize, usize) {
    let mut failed = 0;
    for (n, t) in battery.functional_test.iter().enumerate() {
        super::send(
            tx,
            Event::TestStarted {
                group: Step::Functional.name().to_string(),
                name: t.name.clone(),
            },
        );
        let verdict = match battery.expected_stdout_of_test(t) {
            Ok(expected) => match binaire_de_test(battery, compiler, t, n, binary, white, c) {
                Ok(cmd) => {
                    run_one(
                        &cmd,
                        &t.args,
                        &t.stdin,
                        white,
                        &expected,
                        &t.stderr,
                        t.exit_code,
                        Duration::from_millis(t.timeout_ms),
                    )
                    .0
                }
                Err(diff) => failed_sans_sortie(diff),
            },
            // stdout attendu illisible (stdout_file) ou absent de la
            // batterie : erreur de batterie, rapportée comme un KO.
            Err(e) => failed_sans_sortie(format!("{e:#}")),
        };
        failed += usize::from(!matches!(verdict, TestVerdict::Passed));
        emit_test(ctx, tx, &t.name, &verdict);
    }
    (battery.functional_test.len(), failed)
}

/// `true` si `path` est un fichier exécutable (au moins un bit x) :
/// une lib statique (`lib/libmy.a`, 0644) ne l'est pas, un binaire
/// produit par make l'est.
fn est_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(path)
        .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

/// Le binaire À EXÉCUTER pour un functional_test : le binaire produit
/// lui-même, ou — si le test porte un `harness` — le harness compilé
/// et linké avec le produit ([`compile_harness`]). Sans harness, un
/// produit non exécutable (ex. lib/libmy.a) est un KO clair.
fn binaire_de_test(
    battery: &Battery,
    compiler: &Path,
    t: &FunctionalTest,
    n: usize,
    binary: &Path,
    white: &Path,
    c: &mut Collect,
) -> Result<PathBuf, String> {
    if t.harness.is_some() {
        return compile_harness(compiler, battery, t, n, binary, white, c);
    }
    if !est_executable(binary) {
        return Err(format!(
            "binaire non exécutable sans harness : {}",
            binary.display()
        ));
    }
    Ok(binary.to_path_buf())
}

/// Compile le harness d'un functional_test et le lie au binaire
/// produit : `<compiler> <cflags…> <harness (résolu depuis
/// battery.root)> <link_flags…> <binaire> -o <salle
/// blanche>/harness_test_<n>` via `env LC_ALL=C`, borné à
/// [`COMPILE_TIMEOUT`]. Les link_flags sont résolus relativement à la
/// salle blanche (le cwd) ; le chemin du binaire produit vient APRÈS
/// les sources (ordre de link). Renvoie le chemin du binaire de test,
/// ou le diff KO (extrait stderr borné, comme [`compile_task`]).
/// `t.harness` doit être `Some` (garanti par [`binaire_de_test`]).
fn compile_harness(
    compiler: &Path,
    battery: &Battery,
    t: &FunctionalTest,
    n: usize,
    binary: &Path,
    white: &Path,
    c: &mut Collect,
) -> Result<PathBuf, String> {
    let harness = t.harness.as_ref().expect("harness Some : garanti");
    let out = white.join(format!("harness_test_{n}"));
    let mut args = vec![
        "LC_ALL=C".to_string(),
        compiler.to_string_lossy().into_owned(),
    ];
    args.extend(battery.project.cflags.iter().cloned());
    args.push(battery.root.join(harness).to_string_lossy().into_owned());
    args.extend(t.link_flags.iter().cloned());
    args.push(binary.to_string_lossy().into_owned());
    args.push("-o".to_string());
    args.push(out.to_string_lossy().into_owned());
    let outcome = run_capture(
        Path::new("/usr/bin/env"),
        &args,
        "",
        white,
        COMPILE_TIMEOUT,
        Limits::default(),
    )
    .map_err(|e| format!("lancement du compilateur impossible : {e:#}"))?;
    c.log(&outcome.stdout);
    c.log(&outcome.stderr);
    match outcome.status {
        ExecStatus::Exit(0) if out.is_file() => Ok(out),
        ExecStatus::Exit(0) => Err(format!(
            "compile failed: {} (binaire {} absent après une compilation réussie)",
            t.name,
            out.display()
        )),
        status => {
            let stderr_tail = tail(&outcome.stderr, STDERR_TAIL);
            if stderr_tail.is_empty() {
                Err(format!(
                    "compile failed: {} ({})",
                    t.name,
                    super::build::describe_status(status)
                ))
            } else {
                Err(format!("compile failed: {}\n{stderr_tail}", t.name))
            }
        }
    }
}

/// Functions : pour chaque `battery.task`, compile delivery (fichier
/// ou tous les *.c d'un dossier), harness et extra_sources en
/// `<stem>_test` dans la salle blanche, puis exécute le binaire avec
/// les `args` et le `stdin` de la task. Une compile KO est le verdict
/// du test (diff = extrait compile), jamais d'exécution. Renvoie
/// `(total, en échec)`.
fn run_functions(
    battery: &Battery,
    compiler: &Path,
    white: &Path,
    ctx: &mut PipelineContext,
    tx: &mpsc::Sender<Event>,
    c: &mut Collect,
) -> (usize, usize) {
    let mut failed = 0;
    for task in &battery.task {
        super::send(
            tx,
            Event::TestStarted {
                group: Step::Functional.name().to_string(),
                name: task.name.clone(),
            },
        );
        let verdict = match compile_task(compiler, battery, task, white, c) {
            Ok(test_bin) => match battery.expected_stdout_of_task(task) {
                Ok(expected) => {
                    run_one(
                        &test_bin,
                        &task.args,
                        &task.stdin,
                        white,
                        &expected,
                        &task.stderr,
                        task.exit_code,
                        Duration::from_millis(task.timeout_ms),
                    )
                    .0
                }
                Err(e) => failed_sans_sortie(format!("{e:#}")),
            },
            Err(diff) => failed_sans_sortie(diff),
        };
        failed += usize::from(!matches!(verdict, TestVerdict::Passed));
        emit_test(ctx, tx, &task.name, &verdict);
    }
    (battery.task.len(), failed)
}

/// Compile et lie le binaire de test d'une task Functions :
/// `<compiler> <cflags…> <sources de la delivery> <harness> <extra…>
/// <-I include_dirs…> <link_flags…> -o <salle blanche>/<stem>_test`
/// via `env LC_ALL=C`, borné à [`COMPILE_TIMEOUT`]. Les sources de la
/// delivery (le fichier, ou tous les *.c directs du dossier — Rush1)
/// sont relatives à la salle blanche (le cwd), comme les
/// `include_dirs` (-I) et les `link_flags` ; harness et extra_sources
/// viennent de `battery.root`. Renvoie le chemin du binaire produit,
/// ou le diff KO (extrait stderr borné, comme les KO de compile de
/// build.rs).
fn compile_task(
    compiler: &Path,
    battery: &Battery,
    task: &Task,
    white: &Path,
    c: &mut Collect,
) -> Result<PathBuf, String> {
    let out = white.join(format!("{}_test", super::build::stem_of(&task.delivery)));
    // Delivery dossier : tous les *.c directs ; fichier : tel quel.
    // Un dossier sans .c est un KO clair (prelim l'a déjà signalé).
    let sources =
        crate::battery::delivery_sources(white, &task.delivery).map_err(|e| format!("{e:#}"))?;
    let mut args = vec![
        "LC_ALL=C".to_string(),
        compiler.to_string_lossy().into_owned(),
    ];
    args.extend(battery.project.cflags.iter().cloned());
    for s in &sources {
        args.push(s.to_string_lossy().into_owned());
    }
    args.push(
        battery
            .root
            .join(&task.harness)
            .to_string_lossy()
            .into_owned(),
    );
    for s in &task.extra_sources {
        args.push(battery.root.join(s).to_string_lossy().into_owned());
    }
    for d in &task.include_dirs {
        args.push(format!("-I{d}"));
    }
    args.extend(task.link_flags.iter().cloned());
    args.push("-o".to_string());
    args.push(out.to_string_lossy().into_owned());
    let outcome = run_capture(
        Path::new("/usr/bin/env"),
        &args,
        "",
        white,
        COMPILE_TIMEOUT,
        Limits::default(),
    )
    .map_err(|e| format!("lancement du compilateur impossible : {e:#}"))?;
    c.log(&outcome.stdout);
    c.log(&outcome.stderr);
    match outcome.status {
        ExecStatus::Exit(0) if out.is_file() => Ok(out),
        ExecStatus::Exit(0) => Err(format!(
            "compile failed: {} (binaire {} absent après une compilation réussie)",
            task.delivery,
            out.display()
        )),
        status => {
            let stderr_tail = tail(&outcome.stderr, STDERR_TAIL);
            if stderr_tail.is_empty() {
                Err(format!(
                    "compile failed: {} ({})",
                    task.delivery,
                    super::build::describe_status(status)
                ))
            } else {
                Err(format!("compile failed: {}\n{stderr_tail}", task.delivery))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn abridge_conserve_le_cr_pour_le_marqueur_visible() {
        // 300 lignes CRLF (> HEAD + TAIL = 250) : le chemin abrégé
        // doit conserver les '\r' — str::lines() les stripperait, ce
        // qui rendrait muet un écart CRLF/LF une fois l'entrée
        // abrégée (le marqueur ␍ d'escape_controls n'apparaîtrait
        // plus).
        let s: String = (0..300).map(|i| format!("ligne {i}\r\n")).collect();
        let abrege = abridge(&s);
        assert!(
            abrege.contains("\r\n"),
            "les CR doivent survivre à l'abridgement : {:?}",
            &abrege[..abrege.len().min(400)]
        );
        assert!(abrege.contains("lignes omises"), "marqueur : {abrege:?}");
        assert!(abrege.starts_with("ligne 0\r\n"), "tête : {abrege:?}");
        assert!(abrege.ends_with("ligne 299\r\n"), "queue : {abrege:?}");
    }

    #[test]
    fn abridge_courte_passe_telle_quelle_crlf_compris() {
        let s = "a\r\nb\r\nc"; // 3 lignes, dernière sans terminaison
        assert_eq!(abridge(s), s, "une entrée courte ne change pas");
    }

    #[test]
    fn render_diff_est_cape_en_octets_sur_des_lignes_geantes() {
        // Deux lignes uniques de 300 Kio chacune : 600 Kio au total,
        // SOUS le seuil d'abridgement (1 Mio) et très en dessous du
        // cap de 500 lignes — seul le cap octets du rendu final
        // protège le rapport.
        let expected = "a".repeat(300 * 1024);
        let got = "b".repeat(300 * 1024);
        let diff = render_diff(&expected, &got);
        assert!(
            diff.len() <= DIFF_MAX_BYTES + 128,
            "le rendu doit être cappé en octets : {} > {} + marge",
            diff.len(),
            DIFF_MAX_BYTES
        );
        assert!(
            diff.contains("diff tronqué"),
            "marqueur de troncature attendu : {:?}",
            &diff[diff.len().saturating_sub(200)..]
        );
    }

    #[test]
    fn render_diff_petit_rest_intact() {
        let diff = render_diff("abc\n", "abd\n");
        assert!(diff.contains("-abc\n"), "{diff:?}");
        assert!(diff.contains("+abd\n"), "{diff:?}");
        assert!(!diff.contains("diff tronqué"), "{diff:?}");
    }
}

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
//!   `battery.functional_test`, avec ses args et son stdin.
//! - Projet Functions : pour chaque `battery.task`, la delivery est
//!   d'abord compilée et liée avec le harness et les extra_sources de
//!   la batterie (`<compiler> <cflags…> <delivery> <harness> <extra…>
//!   -o <salle blanche>/<stem>_test` — même sélection de compilateur
//!   que build.rs) ; une compile KO est le verdict du test — `Failed`
//!   dont le diff porte l'extrait stderr du compilateur — sans
//!   exécution. Sinon le binaire de test est exécuté sans args ni
//!   stdin.
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
use crate::battery::model::{ProjectType, Task};
use crate::battery::Battery;
use crate::exec::{run_capture, ExecOutcome, ExecStatus, Limits, MAX_CAPTURE_BYTES};
use similar::{ChangeTag, TextDiff};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

/// Timeout de la compile + link d'une task Functions (comme la
/// compile des deliveries de build.rs : instantané en pratique, on
/// reste borné).
const COMPILE_TIMEOUT: Duration = Duration::from_secs(60);

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
            Some(binary) => run_binary(&battery, &binary, &white, ctx, tx),
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

/// Verdict d'un processus sorti normalement : stdout d'abord, puis
/// stderr, puis exit code — le premier écart est un `Failed`. Enfin,
/// la garde troncature : tout match sur une capture bornée reste un KO.
fn verdict_on_exit(
    outcome: &ExecOutcome,
    code: i32,
    expected_stdout: &str,
    expected_stderr: &str,
    expected_code: i32,
) -> TestVerdict {
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
        let mut flux = Vec::new();
        if outcome.stdout_truncated {
            flux.push("stdout");
        }
        if outcome.stderr_truncated {
            flux.push("stderr");
        }
        // expected/got : le premier flux tronqué, celui dont la
        // capture est incomplète.
        let (expected, got) = if outcome.stdout_truncated {
            (expected_stdout, outcome.stdout.as_str())
        } else {
            (expected_stderr, outcome.stderr.as_str())
        };
        return TestVerdict::Failed {
            diff: format!(
                "sortie tronquée (capture bornée à {} Mio, {}) — impossible de conclure un Passed",
                MAX_CAPTURE_BYTES / (1024 * 1024),
                flux.join(" et "),
            ),
            expected: expected.to_string(),
            got: got.to_string(),
        };
    }
    TestVerdict::Passed
}

/// Diff unified texte de `expected` vs `got` : lignes `-attendu` /
/// `+obtenu` / ` contexte` (le panneau détail du TUI les colorera).
/// Une ligne sans terminaison est suivie du marqueur « \ pas de
/// newline final » — sinon le manque du newline final serait
/// invisible (« abc\n » vs « abc »).
fn render_diff(expected: &str, got: &str) -> String {
    let diff = TextDiff::from_lines(expected, got);
    let mut out = String::new();
    for change in diff.iter_all_changes() {
        let sign = match change.tag() {
            ChangeTag::Delete => '-',
            ChangeTag::Insert => '+',
            ChangeTag::Equal => ' ',
        };
        // Display d'un Change texte écrit sa valeur, terminaison
        // incluse si présente ; le write! dans un String est
        // infaillible.
        let _ = write!(out, "{sign}{change}");
        if change.missing_newline() {
            out.push('\n');
            out.push_str("\\ pas de newline final\n");
        }
    }
    out
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
/// `battery.functional_test`, dans la salle blanche. Renvoie
/// `(total, en échec)`.
fn run_binary(
    battery: &Battery,
    binary: &Path,
    white: &Path,
    ctx: &mut PipelineContext,
    tx: &mpsc::Sender<Event>,
) -> (usize, usize) {
    let mut failed = 0;
    for t in &battery.functional_test {
        super::send(
            tx,
            Event::TestStarted {
                group: Step::Functional.name().to_string(),
                name: t.name.clone(),
            },
        );
        let verdict = match battery.expected_stdout_of_test(t) {
            Ok(expected) => {
                run_one(
                    binary,
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
            // stdout attendu illisible (stdout_file) ou absent de la
            // batterie : erreur de batterie, rapportée comme un KO.
            Err(e) => failed_sans_sortie(format!("{e:#}")),
        };
        failed += usize::from(!matches!(verdict, TestVerdict::Passed));
        emit_test(ctx, tx, &t.name, &verdict);
    }
    (battery.functional_test.len(), failed)
}

/// Functions : pour chaque `battery.task`, compile delivery, harness
/// et extra_sources en `<stem>_test` dans la salle blanche, puis
/// exécute le binaire sans args ni stdin. Une compile KO est le
/// verdict du test (diff = extrait compile), jamais d'exécution.
/// Renvoie `(total, en échec)`.
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
                        &[],
                        "",
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
/// `<compiler> <cflags…> <delivery> <harness> <extra…> -o
/// <salle blanche>/<stem>_test` via `env LC_ALL=C`, borné à
/// [`COMPILE_TIMEOUT`]. La delivery est relative à la salle blanche
/// (le cwd) ; harness et extra_sources viennent de `battery.root`.
/// Renvoie le chemin du binaire produit, ou le diff KO (extrait
/// stderr borné, comme les KO de compile de build.rs).
fn compile_task(
    compiler: &Path,
    battery: &Battery,
    task: &Task,
    white: &Path,
    c: &mut Collect,
) -> Result<PathBuf, String> {
    let out = white.join(format!("{}_test", super::build::stem_of(&task.delivery)));
    let mut args = vec![
        "LC_ALL=C".to_string(),
        compiler.to_string_lossy().into_owned(),
    ];
    args.extend(battery.project.cflags.iter().cloned());
    args.push(task.delivery.clone());
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

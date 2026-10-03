//! Tests de `exec` (Task 2) : capture stdout/stderr, code de sortie,
//! signaux, timeout et extermination du groupe de processus.

use seeyou::exec::{run_capture, ExecOutcome, ExecStatus, Limits};
use std::path::Path;
use std::time::Duration;

/// Lance `program args` avec `stdin`, dans un dossier qui existe
/// (`temp_dir()`), et fait échouer le test si le lancement lui-même
/// erre (les assertions portent sur l'`ExecOutcome`, pas sur le spawn).
fn run(program: &str, args: &[&str], stdin: &str, timeout: Duration) -> ExecOutcome {
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    run_capture(
        Path::new(program),
        &args,
        stdin,
        &std::env::temp_dir(),
        timeout,
        Limits::default(),
    )
    .expect("run_capture ne doit pas renvoyer d'erreur")
}

#[test]
fn echo_hi_capture_stdout_et_exit_0() {
    let out = run("echo", &["hi"], "", Duration::from_secs(2));
    assert_eq!(out.stdout, "hi\n");
    assert_eq!(out.status, ExecStatus::Exit(0));
}

#[test]
fn false_renvoie_exit_1() {
    let out = run("false", &[], "", Duration::from_secs(2));
    assert_eq!(out.status, ExecStatus::Exit(1));
}

#[test]
fn segfault_detecte_signal_11() {
    let out = run("sh", &["-c", "kill -SEGV $$"], "", Duration::from_secs(2));
    assert_eq!(out.status, ExecStatus::Signal(11));
}

#[test]
fn timeout_sur_sleep_elapsed_sous_1s() {
    let out = run("sleep", &["5"], "", Duration::from_millis(200));
    assert_eq!(out.status, ExecStatus::Timeout);
    assert!(
        out.elapsed < Duration::from_secs(1),
        "elapsed trop grand : {:?}",
        out.elapsed
    );
}

#[test]
fn stdin_ressort_identique_via_cat() {
    let data = "première ligne\naccentué : héllo ✓\n\ndernière ligne sans newline final";
    let out = run("cat", &[], data, Duration::from_secs(2));
    assert_eq!(out.status, ExecStatus::Exit(0));
    assert_eq!(out.stdout, data);
}

#[test]
fn timeout_tue_aussi_les_descendants() {
    let out = run(
        "sh",
        &["-c", "sleep 30 & sleep 30"],
        "",
        Duration::from_millis(300),
    );
    assert_eq!(out.status, ExecStatus::Timeout);

    // Les petits-fils tués par SIGKILL sont réapés par init de façon
    // asynchrone : courte fenêtre de tolérance avant le verdict.
    let mut survivants = String::new();
    for _ in 0..20 {
        let pgrep = std::process::Command::new("pgrep")
            .args(["-f", "sleep 30"])
            .output()
            .expect("pgrep doit tourner");
        survivants = String::from_utf8_lossy(&pgrep.stdout).into_owned();
        if survivants.is_empty() {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    assert!(
        survivants.is_empty(),
        "des « sleep 30 » survivent au timeout :\n{survivants}"
    );
}

//! Tests de `exec` : capture stdout/stderr (bornée), code de sortie,
//! signaux, timeout, extermination du groupe de processus, rlimits et
//! garantie « jamais bloquer, jamais fuiter » (descendants `setsid`
//! inclus).

use seeyou::exec::{run_capture, ExecOutcome, ExecStatus, Limits, MAX_CAPTURE_BYTES};
use std::path::Path;
use std::time::{Duration, Instant};

/// Lance `program args` avec `stdin`, dans un dossier qui existe
/// (`temp_dir()`), et fait échouer le test si le lancement lui-même
/// erre (les assertions portent sur l'`ExecOutcome`, pas sur le spawn).
fn run(program: &str, args: &[&str], stdin: &str, timeout: Duration) -> ExecOutcome {
    run_lim(program, args, stdin, timeout, Limits::default())
}

/// Variante de [`run`] avec des `Limits` explicites.
fn run_lim(
    program: &str,
    args: &[&str],
    stdin: &str,
    timeout: Duration,
    limits: Limits,
) -> ExecOutcome {
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    run_capture(
        Path::new(program),
        &args,
        stdin,
        &std::env::temp_dir(),
        timeout,
        limits,
    )
    .expect("run_capture ne doit pas renvoyer d'erreur")
}

/// Compte les processus dont la ligne de commande contient `marqueur`.
fn pgrep_compte(marqueur: &str) -> usize {
    let out = std::process::Command::new("pgrep")
        .args(["-f", marqueur])
        .output()
        .expect("pgrep doit tourner");
    String::from_utf8_lossy(&out.stdout).lines().count()
}

/// Sonde lancée en parallèle du scénario : renvoie un handle qui ne
/// vaut `true` que si au moins `au_moins` processus portant `marqueur`
/// ont existé pendant la fenêtre — preuve que le scénario a réellement
/// été instancié (sinon le test serait vacu).
fn temoin_pendant(
    marqueur: &'static str,
    au_moins: usize,
    fenetre: Duration,
) -> std::thread::JoinHandle<bool> {
    std::thread::spawn(move || {
        let debut = Instant::now();
        while debut.elapsed() < fenetre {
            if pgrep_compte(marqueur) >= au_moins {
                return true;
            }
            std::thread::sleep(Duration::from_millis(15));
        }
        false
    })
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
    // Marqueur unique dans argv[0] : les pgrep ci-dessous ne peuvent
    // pas confondre nos cobayes avec un `sleep` quelconque de la
    // machine. Le scénario exige un fork : le défaut nproc (8192)
    // laisse assez de marge au-dessus de l'ambiant système.
    // Témoin : les deux cobayes taggés doivent exister pendant le run
    // (état stable : sh a exec'd le second, le subshell le premier).
    let temoin = temoin_pendant("seeyou_test_tag", 2, Duration::from_millis(280));
    let out = run(
        "sh",
        &[
            "-c",
            "exec -a seeyou_test_tag sleep 30 & exec -a seeyou_test_tag sleep 30",
        ],
        "",
        Duration::from_millis(300),
    );
    assert!(
        temoin.join().expect("le témoin ne doit pas paniquer"),
        "les cobayes taggés n'ont jamais existé — test vacu (fork refusé ?)"
    );
    assert_eq!(out.status, ExecStatus::Timeout);

    // Les petits-fils tués par SIGKILL sont réapés par init de façon
    // asynchrone : courte fenêtre de tolérance avant le verdict.
    let mut survivants = 1;
    for _ in 0..20 {
        survivants = pgrep_compte("seeyou_test_tag");
        if survivants == 0 {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    assert_eq!(
        survivants, 0,
        "des « seeyou_test_tag » survivent au timeout"
    );
}

// --- Scénarios justifiant l'architecture (review) ---------------------

/// (a) Un fils qui ne lit jamais stdin + un input plus gros que le
/// buffer du pipe (~64 Kio) : le thread writer absorbe le blocage
/// pendant le run, et au timeout le fils meurt → EPIPE → retour
/// propre et borné.
#[test]
fn fils_qui_ne_lit_pas_stdin_gros_input_retour_propre() {
    let input = "x".repeat(1024 * 1024); // 1 Mio >> buffer du pipe
    let t0 = Instant::now();
    let out = run("sleep", &["5"], &input, Duration::from_millis(200));
    let wall = t0.elapsed();
    assert_eq!(out.status, ExecStatus::Timeout);
    assert!(
        wall < Duration::from_secs(3),
        "run_capture ne doit pas bloquer sur stdin : {wall:?}"
    );
}

/// (b) Binaire inexistant : `Err`, et le message nomme le programme.
#[test]
fn binaire_inexistant_err_avec_contexte() {
    let err = run_capture(
        Path::new("/n/existe/pas/seeyou_binaire_inexistant"),
        &[],
        "",
        &std::env::temp_dir(),
        Duration::from_secs(1),
        Limits::default(),
    )
    .expect_err("un binaire inexistant doit produire une Err");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("/n/existe/pas/seeyou_binaire_inexistant"),
        "le message doit nommer le programme : {msg}"
    );
}

/// (c) Flood : `yes` produit > 1 Gio en 500 ms sans cap. La capture
/// doit être bornée à `MAX_CAPTURE_BYTES`, le surplus drainé puis jeté
/// (jamais de fermeture du pipe, jamais de SIGPIPE pour un fils
/// propre), et le retour rester borné.
#[test]
fn flood_yes_stdout_cappe_et_retour_borne() {
    let t0 = Instant::now();
    let out = run("yes", &[], "", Duration::from_millis(500));
    let wall = t0.elapsed();
    assert_eq!(out.status, ExecStatus::Timeout);
    assert!(
        out.stdout_truncated,
        "le flood doit lever stdout_truncated (capture = {} octets)",
        out.stdout.len()
    );
    assert!(!out.stderr_truncated);
    assert!(
        out.stdout.len() <= MAX_CAPTURE_BYTES + 8192,
        "capture au-delà du cap + marge : {} octets",
        out.stdout.len()
    );
    assert!(wall < Duration::from_secs(5), "retour non borné : {wall:?}");
}

/// Symétrie de (c) : le cap et le drapeau s'appliquent aussi à stderr.
#[test]
fn flood_stderr_cappe_aussi() {
    let out = run(
        "sh",
        &["-c", "exec yes >&2"],
        "",
        Duration::from_millis(500),
    );
    assert_eq!(out.status, ExecStatus::Timeout);
    assert!(out.stderr_truncated, "le flood doit lever stderr_truncated");
    assert!(!out.stdout_truncated);
    assert!(out.stdout.is_empty());
    assert!(
        out.stderr.len() <= MAX_CAPTURE_BYTES + 8192,
        "capture stderr au-delà du cap + marge : {} octets",
        out.stderr.len()
    );
}

/// (d) `RLIMIT_CPU` = 1 s contre une boucle busy : mort par signal en
/// ~1 s. soft == hard, donc selon l'ordonnancement la mort est vue
/// comme `Signal(24)` (`SIGXCPU`, envoyé à la limite soft) ou
/// `Signal(9)` (`SIGKILL`, un cran après) — les deux sont acceptés.
#[test]
fn rlimit_cpu_tue_la_boucle_busy_en_1s() {
    let args: Vec<String> = ["-c", "while :; do :; done"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let out = run_capture(
        Path::new("sh"),
        &args,
        "",
        &std::env::temp_dir(),
        Duration::from_secs(10),
        Limits {
            cpu_secs: 1,
            ..Limits::default()
        },
    )
    .expect("run_capture ne doit pas renvoyer d'erreur");
    assert!(
        matches!(out.status, ExecStatus::Signal(9) | ExecStatus::Signal(24)),
        "status inattendu : {:?}",
        out.status
    );
    assert!(
        (Duration::from_millis(900)..=Duration::from_secs(3)).contains(&out.elapsed),
        "elapsed hors [0.9s, 3s] : {:?}",
        out.elapsed
    );
}

/// Un descendant qui a fait `setsid` quitte le groupe de processus et
/// échappe au kill : s'il garde stdout/stderr ouverts, les reader
/// threads ne voient jamais EOF. run_capture ne doit PAS pendre sur
/// eux — grâce bornée, snapshot, retour. (Reproduit avant le fix :
/// 6 s pour un timeout de 300 ms.)
#[test]
fn descendant_setsid_garde_stdout_retour_borne() {
    // Le descendant taggé fait setsid : il échappe au kill de groupe
    // (échappatoire documentée) et garde les pipes ouverts. Fork requis
    // : couvert par le défaut nproc (8192).
    let temoin = temoin_pendant("seeyou_setsid_tag", 1, Duration::from_millis(280));
    let t0 = Instant::now();
    let out = run(
        "sh",
        &[
            "-c",
            "setsid sh -c 'exec -a seeyou_setsid_tag sleep 6' & exec sleep 5",
        ],
        "",
        Duration::from_millis(300),
    );
    let wall = t0.elapsed();
    assert!(
        temoin.join().expect("le témoin ne doit pas paniquer"),
        "le descendant setsid n'a jamais existé — test vacu (fork refusé ?)"
    );
    assert_eq!(out.status, ExecStatus::Timeout);
    assert!(
        wall < Duration::from_secs(3),
        "run_capture ne doit pas pendre sur les readers : {wall:?}"
    );
    // Le descendant a survécu au kill : les pipes étaient réellement
    // encore ouverts au retour — c'est exactement le cas qui faisait
    // pendre les joins avant le fix. (Il meurt tout seul à 6 s.)
    assert!(
        pgrep_compte("seeyou_setsid_tag") >= 1,
        "le descendant setsid doit survivre au kill de groupe"
    );
}

/// Même trou côté stdin : un descendant `setsid` garde le read-end du
/// pipe ouvert sans jamais lire → le writer thread reste bloqué dans
/// `write` ; run_capture ne doit pas le joindre sans borne.
/// (`exec 3<&0` + `0<&3` : le read-end est transmis au descendant de
/// façon robuste, que le shell redirige ou non le stdin des jobs en
/// arrière-plan vers /dev/null.)
#[test]
fn descendant_setsid_garde_stdin_retour_borne() {
    // Fork requis : couvert par le défaut nproc (8192).
    let temoin = temoin_pendant("seeyou_stdin_tag", 1, Duration::from_millis(280));
    let input = "x".repeat(1024 * 1024); // > buffer du pipe : write bloque
    let t0 = Instant::now();
    let out = run(
        "sh",
        &[
            "-c",
            "exec 3<&0; setsid sh -c 'exec -a seeyou_stdin_tag sleep 6' 0<&3 & exec sleep 5",
        ],
        &input,
        Duration::from_millis(300),
    );
    let wall = t0.elapsed();
    assert!(
        temoin.join().expect("le témoin ne doit pas paniquer"),
        "le descendant setsid n'a jamais existé — test vacu (fork refusé ?)"
    );
    assert_eq!(out.status, ExecStatus::Timeout);
    assert!(
        wall < Duration::from_secs(3),
        "run_capture ne doit pas pendre sur le writer stdin : {wall:?}"
    );
    // Même preuve que ci-dessus : le read-end était réellement encore
    // ouvert au retour.
    assert!(
        pgrep_compte("seeyou_stdin_tag") >= 1,
        "le descendant setsid doit survivre au kill de groupe"
    );
}

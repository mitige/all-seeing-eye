//! Exécution de sous-processus : capture, timeout, limites de ressources.
//!
//! Brique de base de tout le pipeline seeyou (build, norme, tests) :
//! - le fils tourne dans sa propre session / groupe de processus
//!   (`setsid`) : au timeout, `SIGKILL` est envoyé à tout le groupe,
//!   descendants compris — une boucle infinie ou un fork bomb ne bloque
//!   jamais seeyou et ne laisse rien survivre ;
//! - des `rlimits` (mémoire, temps CPU, taille de fichier) sont posés
//!   dans le fils avant l'`exec` ;
//! - stdin est écrit sur un thread dédié et stdout/stderr sont lus sur
//!   des threads dédiés : un fils qui ne lit pas son entrée ou qui
//!   remplit un pipe ne provoque aucun deadlock.

use anyhow::{Context, Result};
use std::io::{Read, Write};
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

/// Limites de ressources posées dans le fils (soft = hard, donc le fils
/// ne peut pas relever lui-même la limite soft).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// `RLIMIT_AS` — espace d'adressage (défaut : 512 Mio).
    pub mem_bytes: u64,
    /// `RLIMIT_CPU` — temps CPU (défaut : 10 s).
    pub cpu_secs: u64,
    /// `RLIMIT_FSIZE` — taille max des fichiers créés (défaut : 16 Mio).
    pub fsize_bytes: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            mem_bytes: 512 * 1024 * 1024,
            cpu_secs: 10,
            fsize_bytes: 16 * 1024 * 1024,
        }
    }
}

/// Comment le processus s'est terminé.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecStatus {
    /// Sortie normale avec ce code.
    Exit(i32),
    /// Tué par ce signal (ex. 11 = `SIGSEGV`).
    Signal(i32),
    /// Timeout dépassé : le groupe de processus a reçu `SIGKILL`.
    Timeout,
}

/// Résultat d'une exécution capturée.
#[derive(Debug)]
pub struct ExecOutcome {
    pub stdout: String,
    pub stderr: String,
    pub status: ExecStatus,
    pub elapsed: Duration,
}

impl From<ExitStatus> for ExecStatus {
    fn from(status: ExitStatus) -> Self {
        match status.code() {
            Some(code) => ExecStatus::Exit(code),
            // code() == None ⟺ tué par un signal (Unix).
            None => ExecStatus::Signal(status.signal().unwrap_or(-1)),
        }
    }
}

/// Lance `program args` dans `cwd`, écrit `stdin_data` sur son stdin,
/// capture stdout/stderr et attend au plus `timeout`.
///
/// Au timeout, tout le groupe de processus du fils reçoit `SIGKILL`,
/// puis le fils est réapé (`wait`) : pas de zombie, pas de descendant
/// survivant. (Seule échappatoire connue : un descendant qui appelle
/// lui-même `setsid` quitte le groupe et échappe au kill — hors modèle
/// de menace d'une moulinette.)
pub fn run_capture(
    program: &Path,
    args: &[String],
    stdin_data: &str,
    cwd: &Path,
    timeout: Duration,
    limits: Limits,
) -> Result<ExecOutcome> {
    let mut cmd = Command::new(program);
    cmd.args(args)
        .current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // SAFETY (contrat de `pre_exec`) : la closure tourne dans le fils,
    // entre fork et exec, en contexte async-signal-safe — on n'y appelle
    // que `setsid` et `setrlimit` (tous deux async-signal-safe), sans
    // toucher à la mémoire partagée avec le parent.
    unsafe {
        cmd.pre_exec(move || {
            // Le fils devient leader d'une nouvelle session, donc d'un
            // nouveau groupe de processus de pgid == son pid : un
            // kill(-pid) ultérieur atteindra aussi tous ses descendants.
            if libc::setsid() == -1 {
                return Err(std::io::Error::last_os_error());
            }
            // Les rlimits se transmettent à travers fork/exec : ils
            // bornent le fils et toute sa descendance.
            for (resource, value) in [
                (libc::RLIMIT_AS, limits.mem_bytes),
                (libc::RLIMIT_CPU, limits.cpu_secs),
                (libc::RLIMIT_FSIZE, limits.fsize_bytes),
            ] {
                let rlim = libc::rlimit {
                    rlim_cur: value,
                    rlim_max: value,
                };
                if libc::setrlimit(resource, &rlim) == -1 {
                    return Err(std::io::Error::last_os_error());
                }
            }
            Ok(())
        });
    }

    let start = Instant::now();
    let mut child = cmd
        .spawn()
        .with_context(|| format!("lancement de {} impossible", program.display()))?;
    let pid = child.id() as i32;

    // Thread d'écriture stdin : un fils qui ne lit pas ne nous bloque
    // pas (si le fils meurt, write échoue en EPIPE — ignoré).
    let mut stdin_pipe = child.stdin.take().expect("stdin est piped");
    let stdin_bytes = stdin_data.as_bytes().to_vec();
    let stdin_writer = thread::spawn(move || {
        let _ = stdin_pipe.write_all(&stdin_bytes);
        // drop(stdin_pipe) ici : ferme le pipe → EOF côté fils.
    });

    // Threads de lecture : vident les pipes en continu, donc un fils
    // bavard ne se bloque jamais sur un pipe plein.
    let mut stdout_pipe = child.stdout.take().expect("stdout est piped");
    let stdout_reader = thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stdout_pipe.read_to_end(&mut buf);
        buf
    });
    let mut stderr_pipe = child.stderr.take().expect("stderr est piped");
    let stderr_reader = thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stderr_pipe.read_to_end(&mut buf);
        buf
    });

    // Attente active : `Child::wait` n'a pas de timeout, donc on sonde.
    let status = loop {
        match child.try_wait() {
            Ok(Some(exit)) => break ExecStatus::from(exit),
            Ok(None) if start.elapsed() >= timeout => {
                // SAFETY : pid est notre fils direct et `setsid`
                // (pre_exec) en a fait un leader de groupe (pgid == pid)
                // : kill(-pid) envoie SIGKILL au fils et à tous ses
                // descendants. Le fils, même déjà mort, reste un zombie
                // non réapé tant qu'on n'a pas wait() : pid ne peut pas
                // avoir été réutilisé. Si tout le groupe est déjà mort,
                // kill renvoie ESRCH — ignoré.
                unsafe {
                    libc::kill(-pid, libc::SIGKILL);
                }
                // Réaper le fils (sinon zombie dans notre /proc).
                let _ = child.wait();
                break ExecStatus::Timeout;
            }
            Ok(None) => thread::sleep(Duration::from_millis(2)),
            Err(err) => return Err(err).context("attente du fils (try_wait) impossible"),
        }
    };

    // Tous les détenteurs des bouts d'écriture des pipes sont morts :
    // les lectures voient EOF et ces joins terminent.
    let _ = stdin_writer.join();
    let stdout = stdout_reader.join().unwrap_or_default();
    let stderr = stderr_reader.join().unwrap_or_default();

    Ok(ExecOutcome {
        stdout: String::from_utf8_lossy(&stdout).into_owned(),
        stderr: String::from_utf8_lossy(&stderr).into_owned(),
        status,
        elapsed: start.elapsed(),
    })
}

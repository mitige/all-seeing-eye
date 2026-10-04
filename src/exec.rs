//! Exécution de sous-processus : capture bornée, timeout, limites de
//! ressources. Promesse : jamais bloquer, jamais fuiter.
//!
//! Brique de base de tout le pipeline all-seeing-eye (build, norme, tests) :
//! - le fils tourne dans sa propre session / groupe de processus
//!   (`setsid`) : au timeout, `SIGSTOP` puis `SIGKILL` sont envoyés à
//!   tout le groupe — un processus stoppé ne peut plus forker, donc
//!   une fork bomb cesse de se régénérer pendant le kill ;
//! - des `rlimits` (mémoire, temps CPU, taille de fichier, nombre de
//!   processus) sont posés dans le fils avant l'`exec` ;
//! - stdin est écrit sur un thread dédié et stdout/stderr sont lus sur
//!   des threads dédiés : un fils qui ne lit pas son entrée ou qui
//!   remplit un pipe ne provoque aucun deadlock ;
//! - la capture est bornée (`MAX_CAPTURE_BYTES` par flux ; au-delà, le
//!   flux est drainé puis jeté — jamais fermé, donc jamais de `SIGPIPE`
//!   pour un fils qui finirait proprement) ;
//! - les threads ne sont jamais joints sans borne : après la décision
//!   du statut, l'EOF est attendu avec une grâce bornée puis le buffer
//!   est snapshoté tel quel — un descendant `setsid` qui garderait un
//!   pipe ouvert ne peut pas faire pendre `run_capture`.

use anyhow::{Context, Result};
use std::io::{Read, Write};
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant};

/// Taille maximale capturée par flux (stdout comme stderr) : 4 Mio.
/// Au-delà, le flux continue d'être drainé mais son contenu est jeté —
/// le pipe n'est jamais fermé, donc un fils qui finirait proprement ne
/// prend pas de `SIGPIPE` — et le drapeau `stdout_truncated` /
/// `stderr_truncated` de [`ExecOutcome`] est levé.
pub const MAX_CAPTURE_BYTES: usize = 4 * 1024 * 1024;

/// Grâce accordée aux reader threads pour constater l'EOF après la
/// décision du statut. Un descendant détaché (`setsid`) peut garder un
/// pipe ouvert indéfiniment : passé ce délai, on snapshotte le buffer
/// sans attendre les threads.
const EOF_GRACE: Duration = Duration::from_millis(200);

/// Pas de sondage de l'état des captures pendant la grâce.
const POLL_PAS: Duration = Duration::from_millis(2);

/// Limites de ressources posées dans le fils (soft = hard, donc le fils
/// ne peut pas relever lui-même la limite soft ; effet visible côté
/// `RLIMIT_CPU` : la mort peut apparaître comme `Signal(24)` —
/// `SIGXCPU`, posée à la limite — ou `Signal(9)` — `SIGKILL`, un cran
/// après — selon l'ordonnancement).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// `RLIMIT_AS` — espace d'adressage (défaut : 512 Mio).
    pub mem_bytes: u64,
    /// `RLIMIT_CPU` — temps CPU (défaut : 10 s).
    pub cpu_secs: u64,
    /// `RLIMIT_FSIZE` — taille max des fichiers créés (défaut : 16 Mio).
    pub fsize_bytes: u64,
    /// `RLIMIT_NPROC` — nombre maximal de processus de l'uid réel
    /// (défaut : 8192). La comptabilisation est par uid réel sur tout
    /// le système, pas par arbre de processus : une machine de dev
    /// normale compte déjà ~2000 tâches, donc un défaut bas (ex. 256)
    /// ferait échouer tout fork du fils (EAGAIN). 8192 laisse passer
    /// les projets légitimes tout en cappant une fork bomb ; sur une
    /// machine de correction dédiée, baisser (ex. 256).
    pub nproc: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            mem_bytes: 512 * 1024 * 1024,
            cpu_secs: 10,
            fsize_bytes: 16 * 1024 * 1024,
            nproc: 8192,
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
    /// `true` si stdout a dépassé `MAX_CAPTURE_BYTES` (surplus jeté).
    pub stdout_truncated: bool,
    /// `true` si stderr a dépassé `MAX_CAPTURE_BYTES` (surplus jeté).
    pub stderr_truncated: bool,
    pub status: ExecStatus,
    /// Durée mesurée à la décision du statut (sortie du wait-loop) :
    /// le teardown (attente d'EOF bornée, snapshot) n'y est pas compté.
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

/// Buffer de capture partagé entre un reader thread et le thread
/// principal : le thread écrit, le principal snapshotte.
#[derive(Default)]
struct CaptureBuffer {
    /// Octets conservés (au plus `MAX_CAPTURE_BYTES`).
    data: Vec<u8>,
    /// `true` dès qu'un octet a dû être jeté (cap atteint).
    truncated: bool,
    /// `true` quand le reader a vu EOF (ou une erreur de lecture).
    eof: bool,
}

/// Verrouille le buffer sans propager de poison : un reader qui
/// paniquerait ne doit pas emporter le thread principal — les données
/// restent lisibles.
fn lock_capture(buf: &Arc<Mutex<CaptureBuffer>>) -> MutexGuard<'_, CaptureBuffer> {
    buf.lock().unwrap_or_else(|poison| poison.into_inner())
}

/// Lance un reader thread (détaché : le handle est droppé, jamais
/// joini) qui vide le pipe en continu, conserve les
/// `MAX_CAPTURE_BYTES` premiers octets et jette le surplus.
fn spawn_capture_reader<R: Read + Send + 'static>(mut pipe: R) -> Arc<Mutex<CaptureBuffer>> {
    let shared = Arc::new(Mutex::new(CaptureBuffer::default()));
    let reader_side = Arc::clone(&shared);
    thread::spawn(move || {
        let mut chunk = [0u8; 8192];
        loop {
            match pipe.read(&mut chunk) {
                Ok(0) => break, // EOF
                Ok(n) => {
                    let mut guard = lock_capture(&reader_side);
                    let reste = MAX_CAPTURE_BYTES.saturating_sub(guard.data.len());
                    if n > reste {
                        guard.truncated = true;
                    }
                    guard.data.extend_from_slice(&chunk[..n.min(reste)]);
                }
                // Erreur de lecture : fin de flux constatée, on arrête.
                Err(_) => break,
            }
        }
        lock_capture(&reader_side).eof = true;
    });
    shared
}

/// Attend l'EOF du buffer au plus jusqu'à `deadline` (sondage toutes
/// les 2 ms), puis snapshotte le contenu quel qu'il soit. Jamais de
/// join : le reader thread reste détaché s'il est encore bloqué.
fn snapshot_capture(buf: &Arc<Mutex<CaptureBuffer>>, deadline: Instant) -> (Vec<u8>, bool) {
    loop {
        {
            let mut guard = lock_capture(buf);
            if guard.eof || Instant::now() >= deadline {
                return (std::mem::take(&mut guard.data), guard.truncated);
            }
        }
        thread::sleep(POLL_PAS);
    }
}

/// Lance `program args` dans `cwd`, écrit `stdin_data` sur son stdin,
/// capture stdout/stderr (chacun cappé à `MAX_CAPTURE_BYTES`, drapeaux
/// `*_truncated` levés au-delà) et attend au plus `timeout`.
///
/// Au timeout, tout le groupe de processus du fils reçoit `SIGSTOP`
/// puis `SIGKILL`, puis le fils est réapé (`wait`) : pas de zombie, pas
/// de descendant survivant. (Échappatoire connue : un descendant qui
/// appelle lui-même `setsid` quitte le groupe et échappe au kill. S'il
/// garde un pipe ouvert, les threads de capture/écriture concernés
/// restent détachés et meurent avec ce processus — fuite bornée à sa
/// durée de vie, jamais de blocage.)
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
                (libc::RLIMIT_NPROC, limits.nproc),
            ] {
                // soft = hard = value : le fils ne peut pas relever
                // lui-même la limite soft.
                let rlim = libc::rlimit {
                    rlim_cur: value,
                    rlim_max: value,
                };
                if libc::setrlimit(resource, &rlim) == 0 {
                    continue;
                }
                // Refus du noyau — typiquement `value` au-dessus de la
                // hard limit courante (EPERM, non privilégié) : clamper
                // à cette hard limit (getrlimit + min) plutôt que
                // d'échouer le spawn — une limite plus lâche que
                // demandé n'est pas fatale.
                let mut courante = libc::rlimit {
                    rlim_cur: 0,
                    rlim_max: 0,
                };
                if libc::getrlimit(resource, &mut courante) == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                let clamp = value.min(courante.rlim_max);
                let rlim = libc::rlimit {
                    rlim_cur: clamp,
                    rlim_max: clamp,
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

    // Thread d'écriture stdin, détaché (jamais joini) : un fils qui ne
    // lit pas ne nous bloque pas. Si le fils meurt, write échoue en
    // EPIPE — ignoré. Si un descendant `setsid` garde le read-end
    // ouvert sans lire, le thread reste bloqué dans write : fuite
    // bornée à la durée de vie de ce processus, jamais de blocage.
    let mut stdin_pipe = child.stdin.take().expect("stdin est piped");
    let stdin_bytes = stdin_data.as_bytes().to_vec();
    let _stdin_writer = thread::spawn(move || {
        let _ = stdin_pipe.write_all(&stdin_bytes);
        // drop(stdin_pipe) ici : ferme le pipe → EOF côté fils.
    });

    // Threads de lecture (détachés eux aussi) : vident les pipes en
    // continu — un fils bavard ne se bloque jamais sur un pipe plein —
    // et bornent la capture à MAX_CAPTURE_BYTES.
    let stdout_pipe = child.stdout.take().expect("stdout est piped");
    let stdout_buf = spawn_capture_reader(stdout_pipe);
    let stderr_pipe = child.stderr.take().expect("stderr est piped");
    let stderr_buf = spawn_capture_reader(stderr_pipe);

    // Attente active : `Child::wait` n'a pas de timeout, donc on sonde.
    let status = loop {
        match child.try_wait() {
            Ok(Some(exit)) => break ExecStatus::from(exit),
            Ok(None) if start.elapsed() >= timeout => {
                // SAFETY : pid est notre fils direct et `setsid`
                // (pre_exec) en a fait un leader de groupe (pgid ==
                // pid) : kill(-pid) vise le fils et tous ses
                // descendants. SIGSTOP d'abord : un processus stoppé
                // ne peut plus forker — une fork bomb cesse de se
                // régénérer pendant le kill ; SIGKILL juste après
                // emporte même les processus stoppés. Le fils, même
                // déjà mort, reste un zombie non réapé tant qu'on n'a
                // pas wait() : pid ne peut pas avoir été réutilisé.
                // Si tout le groupe est déjà mort, kill renvoie ESRCH
                // — ignoré.
                unsafe {
                    libc::kill(-pid, libc::SIGSTOP);
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
    // L'horloge s'arrête à la décision du statut : l'attente d'EOF
    // bornée et le snapshot ci-dessous sont du teardown, pas du temps
    // passé à exécuter le fils.
    let elapsed = start.elapsed();

    // Les reader threads voient EOF dès que tous les détenteurs des
    // bouts d'écriture sont morts — mais un descendant `setsid` peut
    // garder un pipe ouvert indéfiniment : grâce bornée partagée, puis
    // snapshot quel que soit l'état. Les threads ne sont jamais joinis.
    let deadline = Instant::now() + EOF_GRACE;
    let (stdout, stdout_truncated) = snapshot_capture(&stdout_buf, deadline);
    let (stderr, stderr_truncated) = snapshot_capture(&stderr_buf, deadline);

    Ok(ExecOutcome {
        stdout: String::from_utf8_lossy(&stdout).into_owned(),
        stderr: String::from_utf8_lossy(&stderr).into_owned(),
        stdout_truncated,
        stderr_truncated,
        status,
        elapsed,
    })
}

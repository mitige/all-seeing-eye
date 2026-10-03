//! Étape Prelim — vérifications préliminaires.
//!
//! Trois familles de contrôles, dans l'ordre :
//! 1. fichiers interdits dans le repo (`*.o`, `*.a`, `*.so`, `*~`,
//!    `#*#`, `*.swp` — C-O1) ;
//! 2. `banana-check-repo`, s'il est installé (ses C-O1/C-O4 peuvent
//!    doublonner avec le point 1 : la vraie chaîne fait les deux, le
//!    rapport déduplique) ;
//! 3. contrôles dépendant du type de projet : Makefile + règles pour
//!    un Binary, fichiers de rendu + prototypes pour un Functions.
//!
//! L'étape ne panique jamais : un repo tordu (fichier illisible,
//! binaire absent, Makefile boiteux) produit des checks KO, pas un
//! crash.

use super::events::{Event, Step};
use super::PipelineContext;
use crate::battery::model::ProjectType;
use crate::exec::{run_capture, ExecStatus, Limits};
use regex::Regex;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

/// Timeout d'une invocation de `make -n` (vérification d'une règle).
const MAKE_TIMEOUT: Duration = Duration::from_secs(10);

/// Timeout de `banana-check-repo` (script shell : normalement
/// instantané, mais on reste borné).
const BANANA_TIMEOUT: Duration = Duration::from_secs(30);

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
                step: Step::Prelim,
                name: name.to_string(),
                ok,
                detail,
            },
        );
        ok
    }
}

/// Étape 1 du pipeline : fichiers interdits, banana-check-repo, puis
/// Makefile/règles (Binary) ou rendus/prototypes (Functions).
/// Renvoie `true` si tous les checks passent.
pub fn run(ctx: &mut PipelineContext, tx: &mpsc::Sender<Event>) -> bool {
    super::step_started(tx, Step::Prelim);
    let mut c = Collect {
        tx,
        total: 0,
        failed: 0,
    };
    let mut ok = check_forbidden_files(&ctx.target, &mut c);
    if let Some(banana_ok) = check_banana(&ctx.target, &mut c) {
        ok &= banana_ok;
    }
    ok &= match ctx.battery.project.kind {
        ProjectType::Binary => check_makefile(ctx, &mut c),
        ProjectType::Functions => check_functions(ctx, &mut c),
    };
    let summary = format!("{} vérifications, {} en échec", c.total, c.failed);
    super::step_finished(ctx, tx, Step::Prelim, ok, summary);
    ok
}

/// `true` si `name` matche un motif interdit : `*.o`, `*.a`, `*.so`,
/// `*~`, `#*#` (au moins deux caractères), `*.swp`.
fn is_forbidden_name(name: &str) -> bool {
    name.ends_with('~')
        || (name.len() >= 2 && name.starts_with('#') && name.ends_with('#'))
        || [".o", ".a", ".so", ".swp"]
            .iter()
            .any(|ext| name.ends_with(ext))
}

/// Remplit `out` des chemins relatifs (à `base`) des fichiers
/// interdits sous `dir`, récursivement, en ignorant `.git`. Un
/// sous-dossier illisible est simplement ignoré. Les symlinks ne sont
/// jamais suivis (`file_type` ne déréférence pas) : pas de boucle.
fn collect_forbidden(dir: &Path, base: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let Ok(ft) = entry.file_type() else {
            continue;
        };
        if ft.is_dir() {
            if entry.file_name() == ".git" {
                continue;
            }
            collect_forbidden(&entry.path(), base, out);
        } else if is_forbidden_name(&entry.file_name().to_string_lossy()) {
            if let Ok(rel) = entry.path().strip_prefix(base) {
                out.push(rel.to_path_buf());
            }
        }
    }
}

/// Check « forbidden files » : un check KO par fichier interdit
/// (chemin relatif préfixé `./`, comme banana-check-repo), un seul
/// check OK si le repo est propre.
fn check_forbidden_files(target: &Path, c: &mut Collect) -> bool {
    let mut found = Vec::new();
    collect_forbidden(target, target, &mut found);
    found.sort(); // ordre déterministe
    if found.is_empty() {
        return c.check("forbidden files", true, "no unwanted file".to_string());
    }
    let mut ok = true;
    for rel in found {
        ok &= c.check(
            "forbidden files",
            false,
            format!("./{}: unwanted file (C-O1)", rel.display()),
        );
    }
    ok
}

/// Regex stricte d'une ligne d'infraction banana :
/// `./FICHIER: [Banana] [SEV] MSG (CODE)`. Les lignes de service
/// (« Checking delivery files… », « No infractions found ») ne
/// commencent pas par `./` et ne matchent donc jamais.
fn banana_line() -> Regex {
    Regex::new(r"^\./.+: \[Banana\] \[[A-Za-z]+\] .+ \([A-Z]-[A-Z0-9]+\)$")
        .expect("regex banana invalide")
}

/// `true` si l'erreur vient d'un binaire introuvable (ENOENT), en
/// regardant toute la chaîne de causes (`run_capture` ajoute du
/// contexte au-dessus de l'`io::Error` de spawn).
fn is_not_found(err: &anyhow::Error) -> bool {
    err.chain().any(|cause| {
        cause
            .downcast_ref::<std::io::Error>()
            .is_some_and(|io| io.kind() == ErrorKind::NotFound)
    })
}

/// Check « banana-check-repo » : `None` (skip silencieux) si le
/// binaire n'est pas installé ; sinon un check KO par ligne
/// d'infraction de sa sortie, un seul check OK si rien n'est remonté.
fn check_banana(target: &Path, c: &mut Collect) -> Option<bool> {
    let outcome = match run_capture(
        Path::new("banana-check-repo"),
        &[],
        "",
        target,
        BANANA_TIMEOUT,
        Limits::default(),
    ) {
        Ok(o) => o,
        Err(e) if is_not_found(&e) => return None,
        Err(e) => {
            return Some(c.check(
                "banana-check-repo",
                false,
                format!("banana-check-repo : {e:#}"),
            ));
        }
    };
    let re = banana_line();
    let mut found = false;
    let mut ok = true;
    for line in outcome.stdout.lines() {
        if re.is_match(line) {
            found = true;
            ok &= c.check("banana-check-repo", false, line.to_string());
        }
    }
    if !found {
        ok &= match outcome.status {
            ExecStatus::Exit(0) => c.check("banana-check-repo", true, "no infraction".to_string()),
            // Sortie inattendue (crash, timeout, ligne de service
            // inconnue) : on ne peut conclure → check KO.
            status => c.check(
                "banana-check-repo",
                false,
                format!("banana-check-repo sans ligne exploitable ({status:?})"),
            ),
        };
    }
    Some(ok)
}

/// Checks Binary : présence du Makefile à la racine, puis existence
/// de chaque règle (`make -n <rule>` ; « No rule to make target » sur
/// stderr = règle manquante — l'exit code de `-q` serait trompeur,
/// cf. « pas à jour »).
fn check_makefile(ctx: &PipelineContext, c: &mut Collect) -> bool {
    if !ctx.target.join("Makefile").is_file() {
        // Sans Makefile, chaque sonde répondrait « No rule to make
        // target » : du bruit sans valeur, on s'arrête là.
        return c.check("makefile", false, "missing Makefile".to_string());
    }
    let mut ok = c.check("makefile", true, "Makefile present".to_string());
    for rule in &ctx.battery.project.makefile_rules {
        let args = vec!["-n".to_string(), rule.clone()];
        match run_capture(
            Path::new("make"),
            &args,
            "",
            &ctx.target,
            MAKE_TIMEOUT,
            Limits::default(),
        ) {
            Ok(o) if o.stderr.contains("No rule to make target") => {
                ok &= c.check(
                    "makefile rules",
                    false,
                    format!("missing makefile rule: {rule}"),
                );
            }
            Ok(_) => {
                ok &= c.check("makefile rules", true, format!("rule {rule} present"));
            }
            Err(e) => {
                ok &= c.check(
                    "makefile rules",
                    false,
                    format!("make -n {rule} impossible : {e:#}"),
                );
            }
        }
    }
    ok
}

/// Normalise une signature ou un source : tout blanc consécutif
/// (espaces, tabs, sauts de ligne) collapsé en un espace, trim aux
/// bords — deux écritures de la même signature deviennent comparables.
fn normalize_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Checks Functions : chaque `task.delivery` doit exister à la racine
/// du repo ; si la task annonce un prototype, la signature normalisée
/// doit apparaître dans le contenu normalisé du fichier.
fn check_functions(ctx: &PipelineContext, c: &mut Collect) -> bool {
    let mut ok = true;
    for task in &ctx.battery.task {
        let path = ctx.target.join(&task.delivery);
        if !path.is_file() {
            ok &= c.check(
                "delivery",
                false,
                format!("missing delivery: {}", task.delivery),
            );
            continue; // pas de fichier → pas de check de prototype
        }
        ok &= c.check("delivery", true, format!("{} present", task.delivery));
        let Some(sig) = &task.prototype else {
            continue;
        };
        match fs::read_to_string(&path) {
            Ok(content) => {
                let sig = normalize_ws(sig);
                if !sig.is_empty() && normalize_ws(&content).contains(&sig) {
                    ok &= c.check("prototype", true, format!("{}: prototype ok", task.name));
                } else {
                    ok &= c.check(
                        "prototype",
                        false,
                        format!("prototype mismatch: {}", task.name),
                    );
                }
            }
            // Fichier illisible → check KO, jamais de panic.
            Err(e) => {
                ok &= c.check(
                    "prototype",
                    false,
                    format!("{} unreadable: {e}", task.delivery),
                );
            }
        }
    }
    ok
}

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
use super::{tail, Collect, PipelineContext, STDERR_TAIL};
use crate::battery::model::ProjectType;
use crate::exec::{run_capture, ExecStatus, Limits};
use regex::Regex;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::LazyLock;
use std::time::Duration;

/// Timeout d'une invocation de `make -n` (vérification d'une règle).
const MAKE_TIMEOUT: Duration = Duration::from_secs(10);

/// Timeout de `banana-check-repo` (script shell : normalement
/// instantané, mais on reste borné).
const BANANA_TIMEOUT: Duration = Duration::from_secs(30);

/// Profondeur maximale du scan de fichiers interdits : au-delà, les
/// sous-dossiers sont ignorés. La traversée est itérative (pile
/// explicite) — un arbre hostile ne peut pas déborder la pile
/// d'appels.
const MAX_SCAN_DEPTH: usize = 64;

/// Étape 1 du pipeline : fichiers interdits, banana-check-repo, puis
/// Makefile/règles (Binary) ou rendus/prototypes (Functions).
/// Renvoie `true` si tous les checks passent.
pub fn run(ctx: &mut PipelineContext, tx: &mpsc::Sender<Event>) -> bool {
    super::step_started(tx, Step::Prelim);
    let mut c = Collect::new(tx, Step::Prelim);
    // Le header TUI n'affiche pas le compilateur choisi par le CLI
    // (chaîne epiclang → cc → gcc) : on le logue ici, première étape
    // du run — visible dans le détail TUI de l'étape comme dans le
    // flux --no-tui. select_compiler donne le compilateur EFFECTIF
    // (celui de build/unit/functional), pas seulement opts.compiler.
    c.log(&format!(
        "compilateur : {}",
        super::build::select_compiler(&ctx.opts).display()
    ));
    let mut ok = check_forbidden_files(&ctx.target, &mut c);
    if let Some(banana_ok) = check_banana(&ctx.target, &mut c) {
        ok &= banana_ok;
    }
    ok &= match ctx.battery.project.kind {
        ProjectType::Binary => check_makefile(ctx, &mut c),
        ProjectType::Functions => check_functions(ctx, &mut c),
    };
    let (total, failed, checks) = c.finish();
    let summary = format!("{total} vérifications, {failed} en échec");
    super::step_finished(ctx, tx, Step::Prelim, ok, summary, checks);
    ok
}

/// `true` si `name` matche un motif interdit : `*.o`, `*.a`, `*.so`,
/// `*~`, `#*#` (au moins deux caractères), `*.swp`.
//
// ⚠ Liste cousine de `is_ignored_name` (build.rs) : si tu modifies
// cette liste, vérifie l'autre. La différence `.a`/`.so` est
// volontaire : prelim les signale comme fichiers interdits dans le
// rendu (C-O1), tandis que la salle blanche ne filtre que les
// artefacts qui parasiteraient la compilation — un `.a`/`.so` copié y
// reste inerte.
fn is_forbidden_name(name: &str) -> bool {
    name.ends_with('~')
        || (name.len() >= 2 && name.starts_with('#') && name.ends_with('#'))
        || [".o", ".a", ".so", ".swp"]
            .iter()
            .any(|ext| name.ends_with(ext))
}

/// Remplit `out` des chemins relatifs (à `base`) des fichiers
/// interdits sous `dir`, en ignorant `.git`. Itératif (pile
/// explicite), borné à [`MAX_SCAN_DEPTH`] niveaux : un arbre hostile
/// ne peut pas déborder la pile d'appels. Un sous-dossier illisible
/// est simplement ignoré. Les symlinks ne sont jamais suivis
/// (`file_type` ne déréférence pas) : pas de boucle.
fn collect_forbidden(dir: &Path, base: &Path, out: &mut Vec<PathBuf>) {
    // Pile des (dossier, profondeur) restant à scanner.
    let mut stack = vec![(dir.to_path_buf(), 0usize)];
    while let Some((dir, depth)) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(ft) = entry.file_type() else {
                continue;
            };
            if ft.is_dir() {
                if entry.file_name() == ".git" || depth >= MAX_SCAN_DEPTH {
                    continue; // .git, et borne anti-abus de profondeur
                }
                stack.push((entry.path(), depth + 1));
            } else if is_forbidden_name(&entry.file_name().to_string_lossy()) {
                if let Ok(rel) = entry.path().strip_prefix(base) {
                    out.push(rel.to_path_buf());
                }
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
/// commencent pas par `./` et ne matchent donc jamais. Compilée une
/// fois pour toutes (LazyLock), pas à chaque appel.
static BANANA_LINE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\./.+: \[Banana\] \[[A-Za-z]+\] .+ \([A-Z]-[A-Z0-9]+\)$")
        .expect("regex banana invalide")
});

/// Cible entre guillemets du message GNU make « No rule to make
/// target '<cible>'[, needed by '<parent>'] ». Le message est
/// stable : LC_ALL=C est forcé dans [`probe_rule`] — le matcher ne
/// dépend pas de la locale de la machine.
static NO_RULE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"No rule to make target '([^']+)'").expect("regex no-rule invalide")
});

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
    let mut found = false;
    let mut ok = true;
    for line in outcome.stdout.lines() {
        if BANANA_LINE_RE.is_match(line) {
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

/// Verdict d'une sonde `make -n <rule>`, avant émission du check.
enum RuleProbe {
    /// Exit 0 : la règle existe et make sait la développer.
    Present,
    /// « No rule to make target '<rule>' » : la cible entre
    /// guillemets est la règle sondée elle-même — elle n'existe pas.
    MissingRule,
    /// « No rule to make target '<cible>', needed by ... » : la règle
    /// existe, mais un de ses prérequis est irrésoluble.
    MissingPrereq(String),
    /// make ne parvient pas à évaluer le Makefile (erreur de parse,
    /// timeout, signal) : toute autre sonde donnerait le même verdict
    /// au prix d'un timeout chacune — le check est émis puis les
    /// sondes restantes sont court-circuitées.
    MakefileIlisible(String),
    /// L'outil est indisponible (spawn en échec, make introuvable via
    /// env) : échec de sonde, pas un verdict sur le Makefile — pas de
    /// court-circuit.
    SondeEchouee(String),
}

/// Sonde une règle via `env LC_ALL=C make -n <rule>` borné à
/// [`MAKE_TIMEOUT`]. LC_ALL=C est forcé en passant par `/usr/bin/env`
/// ([`run_capture`] n'a pas de paramètre d'environnement) : le stderr
/// matché par [`NO_RULE_RE`] ne doit pas dépendre de la locale de la
/// machine.
fn probe_rule(dir: &Path, rule: &str) -> RuleProbe {
    let args = vec![
        "LC_ALL=C".to_string(),
        "make".to_string(),
        "-n".to_string(),
        rule.to_string(),
    ];
    let outcome = match run_capture(
        Path::new("/usr/bin/env"),
        &args,
        "",
        dir,
        MAKE_TIMEOUT,
        Limits::default(),
    ) {
        Ok(o) => o,
        Err(e) => return RuleProbe::SondeEchouee(format!("make -n {rule} impossible : {e:#}")),
    };
    match outcome.status {
        ExecStatus::Exit(0) => RuleProbe::Present,
        // 126/127 viennent d'env (commande introuvable ou non
        // exécutable) : make lui-même ne rend que 0 ou 2.
        ExecStatus::Exit(code @ (126 | 127)) => RuleProbe::SondeEchouee(format!(
            "make introuvable ou inexécutable via /usr/bin/env (exit {code})"
        )),
        ExecStatus::Exit(_) => {
            match NO_RULE_RE
                .captures(&outcome.stderr)
                .map(|cap| cap[1].to_string())
            {
                Some(cible) if cible == rule => RuleProbe::MissingRule,
                Some(cible) => RuleProbe::MissingPrereq(cible),
                None => RuleProbe::MakefileIlisible(format!(
                    "makefile invalide: {}",
                    tail(&outcome.stderr, STDERR_TAIL)
                )),
            }
        }
        ExecStatus::Signal(sig) => {
            RuleProbe::MakefileIlisible(format!("make tué par le signal {sig} sur la règle {rule}"))
        }
        ExecStatus::Timeout => {
            RuleProbe::MakefileIlisible(format!("make timeout sur la règle {rule}"))
        }
    }
}

/// Checks Binary : présence du Makefile à la racine, puis existence
/// de chaque règle (`make -n <rule>`). Le verdict repose sur le
/// statut d'exécution : exit 0 = OK, tout le reste (exit ≠ 0, signal,
/// timeout, spawn en échec) = KO — un « No rule to make target »
/// n'est plus qu'un détail de KO parmi d'autres, et son contenu entre
/// guillemets distingue la règle manquante du prérequis irrésoluble.
fn check_makefile(ctx: &PipelineContext, c: &mut Collect) -> bool {
    if !ctx.target.join("Makefile").is_file() {
        // Sans Makefile, chaque sonde répondrait « No rule to make
        // target » : du bruit sans valeur, on s'arrête là.
        return c.check("makefile", false, "missing Makefile".to_string());
    }
    let mut ok = c.check("makefile", true, "Makefile present".to_string());
    let rules = &ctx.battery.project.makefile_rules;
    for (i, rule) in rules.iter().enumerate() {
        match probe_rule(&ctx.target, rule) {
            RuleProbe::Present => {
                ok &= c.check("makefile rules", true, format!("rule {rule} present"));
            }
            RuleProbe::MissingRule => {
                ok &= c.check(
                    "makefile rules",
                    false,
                    format!("missing makefile rule: {rule}"),
                );
            }
            RuleProbe::MissingPrereq(cible) => {
                ok &= c.check(
                    "makefile rules",
                    false,
                    format!("prérequis irrésoluble: {cible} (règle {rule})"),
                );
            }
            RuleProbe::MakefileIlisible(detail) => {
                // Le Makefile ne peut pas être évalué : les sondes
                // suivantes échoueraient à l'identique en payant
                // chacune le timeout — un seul KO global pour les
                // règles restantes.
                ok &= c.check("makefile rules", false, detail);
                let reste = &rules[i + 1..];
                if !reste.is_empty() {
                    ok &= c.check(
                        "makefile rules",
                        false,
                        format!(
                            "makefile illisible : règles {} non sondées",
                            reste.join(", ")
                        ),
                    );
                }
                break;
            }
            RuleProbe::SondeEchouee(detail) => {
                ok &= c.check("makefile rules", false, detail);
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
/// du repo et être lisible ; si la task annonce un prototype, la
/// signature normalisée doit apparaître dans le contenu normalisé du
/// fichier.
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
            // Pas de prototype annoncé : on sonde au moins la
            // lisibilité — un rendu chmod 000 passerait sinon pour
            // acceptable.
            if let Err(e) = fs::File::open(&path) {
                ok &= c.check(
                    "delivery",
                    false,
                    format!("{} unreadable: {e}", task.delivery),
                );
            }
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

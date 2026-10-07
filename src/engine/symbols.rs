//! Étape Symbols — fonctions interdites (C-forbidden).
//!
//! Scanne les artefacts du build à la recherche de références externes
//! hors whitelist (`project.allowed_functions`) :
//! - Projet Binary : le binaire linké (`ctx.build.binary`) ;
//! - Projet Functions : les `.o` des deliveries, à la racine de la
//!   salle blanche (contrat build.rs : un `<stem>.o` par delivery).
//!
//! Un symbole est flaggé s'il est INDÉFINI (référence externe) dans un
//! artefact, et :
//! - ni DÉFINI par un artefact du rendu (une fonction du projet qui en
//!   appelle une autre est un appel interne, pas une dépendance
//!   externe) ;
//! - ni un symbole du runtime C ([`RUNTIME_SYMBOLS`] : `_start`,
//!   `__libc_start_main`, `_GLOBAL_OFFSET_TABLE_`… — bookkeeping
//!   compilateur/éditeur de liens, jamais une fonction appelable) ;
//! - ni une donnée globale libc ([`DATA_SYMBOLS`] : `stdin`, `stdout`,
//!   `stderr`, `environ`… — dans un `.o`, une donnée externe est un
//!   indéfini NOTYPE, indistinguable d'une fonction par son kind) ;
//! - ni dans `allowed_functions`.
//!
//! Retraitement des noms : le suffixe de version ELF
//! (`printf@@GLIBC_2.2.5`, `printf@GLIBC_2.2.5`) est tronqué avant
//! comparaison ([`strip_version`]).
//!
//! ## Guide des whitelists
//!
//! Le sujet whiteliste des fonctions vues du C ; la libc et le
//! compilateur substituent parfois l'implémentation réellement
//! référencée. À prévoir dans `allowed_functions` selon le sujet :
//! - `errno` → `__errno_location` : `errno` est une macro qui expande
//!   en `(*__errno_location())` — autoriser `errno` implique de
//!   whitelister `__errno_location`, sinon KO « forbidden function » ;
//! - variantes fortifiées (`-D_FORTIFY_SOURCE`, activé par défaut sur
//!   plusieurs distributions) : `printf` → `__printf_chk`,
//!   `fprintf` → `__fprintf_chk`, `memcpy` → `__memcpy_chk`… — la
//!   whitelist doit couvrir la variante `_chk`, ou le build doit
//!   tourner sans fortify.
//!
//! Cas particuliers :
//! - `allowed_functions` vide → l'étape est désactivée :
//!   `StepFinished ok:true, skipped:true` ([`super::step_disabled`] —
//!   un skip de configuration n'est ni un échec ni un skip de
//!   dépendance) ;
//! - build absent ou KO (rien à scanner) → l'étape tourne quand même
//!   (spec §5 : Symbols ne dépend pas du build dans `skip_reason`)
//!   mais réussit à vide : `StepFinished ok:true, skipped:false`,
//!   summary « aucun artefact à scanner ». PAS un KO : la pénalité
//!   du build KO est déjà portée par l'étape Build ;
//! - artefact illisible ou corrompu → check KO explicite, étape KO —
//!   jamais de panic sur input hostile. Un `.o` manquant (delivery
//!   non compilée, build déjà KO) est en revanche ignoré : le KO a
//!   déjà été remonté par l'étape Build.

use super::events::{Event, Step};
use super::{Collect, PipelineContext};
use crate::battery::model::ProjectType;
use anyhow::{Context, Result};
use object::{Object, ObjectSymbol};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;

/// Symboles du runtime C (crt0, glibc, stack protector…) présents dans
/// un binaire ou objet sans être des appels de l'étudiant.
///
/// TRIÉ (recherche dichotomique) — à garder trié, le test unitaire
/// `runtime_symbols_reste_trie` veille.
const RUNTIME_SYMBOLS: &[&str] = &[
    "_GLOBAL_OFFSET_TABLE_", // base GOT : bookkeeping (-fPIC, symboles weak)
    "_ITM_deregisterTMCloneTable", // dé/enregistrement des clones transactionnels
    "_ITM_registerTMCloneTable",
    "__bss_start",  // bornes de section, posées par le crt
    "__cxa_atexit", // atexit/destruction des statiques, côté glibc
    "__cxa_finalize",
    "__do_global_dtors_aux", // passage des destructeurs
    "__gmon_start__",        // init du profiling gprof (faible)
    "__libc_start_main",     // point d'entrée réel : appelle main
    "__stack_chk_fail",      // stack protector (-fstack-protector)
    "__stack_chk_guard",
    "_edata",
    "_end",
    "_fini", // destructeurs crt
    "_frame_dummy_init_array_entry",
    "_init",  // constructeurs crt
    "_start", // point d'entrée ELF
    "deregister_tm_clones",
    "main", // le rendu Binary le définit ; jamais une dépendance externe
    "register_tm_clones",
];

/// Données globales libc licites. Dans un `.o`, une donnée externe
/// (`stderr`, `environ`…) est un indéfini NOTYPE, indistinguable d'une
/// fonction par son kind : sans cette liste, `fprintf(stderr, "e")`
/// avec `fprintf` whitelisté serait KO « forbidden function: stderr ».
/// Les sujets Epitech whitelistent des fonctions, pas des variables —
/// ces globals sont licites dès qu'une fonction whitelistée les
/// utilise.
///
/// TRIÉ (recherche dichotomique) — à garder trié, le test unitaire
/// `data_symbols_reste_trie` veille.
const DATA_SYMBOLS: &[&str] = &[
    "environ", // variables d'environnement (unistd.h)
    "optarg",  // getopt : argument de l'option courante
    "opterr",  // getopt : émission des messages d'erreur
    "optind",  // getopt : index du prochain argument
    "optopt",  // getopt : option inconnue ou argument manquant
    "stderr",  // flux d'erreur standard (stdio.h)
    "stdin",   // flux d'entrée standard
    "stdout",  // flux de sortie standard
];

/// Étape 4 du pipeline : fonctions interdites. Renvoie `true` ssi
/// aucun symbole interdit n'est trouvé (skip de configuration et
/// absence d'artefacts inclus).
pub fn run(ctx: &mut PipelineContext, tx: &mpsc::Sender<Event>) -> bool {
    super::step_started(tx, Step::Symbols);

    // Whitelist vide : le check n'a pas vocation à tourner.
    if ctx.battery.project.allowed_functions.is_empty() {
        super::step_disabled(
            ctx,
            tx,
            Step::Symbols,
            "skipped: allowed_functions vide".to_string(),
        );
        return true;
    }

    // Artefacts à scanner. Rien si le build est absent/KO : l'étape
    // tourne (spec §5) mais réussit à vide — voir l'en-tête.
    let artefacts = artefacts_a_scanner(ctx);
    if artefacts.is_empty() {
        super::step_finished(
            ctx,
            tx,
            Step::Symbols,
            true,
            "aucun artefact à scanner".to_string(),
            Vec::new(),
        );
        return true;
    }

    let mut c = Collect::new(tx, Step::Symbols);
    let mut table = SymbolTable::default();
    let mut ok = true;
    // Premier passage : tous les symboles de tous les artefacts,
    // définis ET indéfinis (un appel interne au rendu n'est flaggable
    // que si l'on connaît les définitions).
    for path in &artefacts {
        if let Err(e) = collect_symbols(path, &mut table) {
            ok &= c.check("artefact", false, format!("{}: {e:#}", path.display()));
        }
    }
    // Second passage : un indéfini est interdit s'il n'est ni défini
    // par le rendu, ni runtime, ni donnée libc licite, ni whitelisté.
    // BTreeSet : dédupliqué et ordre déterministe.
    let allowed: BTreeSet<&str> = ctx
        .battery
        .project
        .allowed_functions
        .iter()
        .map(String::as_str)
        .collect();
    let mut flagged = false;
    for name in &table.undefined {
        if table.defined.contains(name)
            || RUNTIME_SYMBOLS.binary_search(&name.as_str()).is_ok()
            || DATA_SYMBOLS.binary_search(&name.as_str()).is_ok()
            || allowed.contains(name.as_str())
        {
            continue;
        }
        flagged = true;
        ok &= c.check(
            "forbidden function",
            false,
            format!("forbidden function: {name}"),
        );
    }
    // Aucun interdit ET tout artefact lu : un seul check OK global.
    // (Si un artefact est illisible, ses KO portent déjà le verdict —
    // un « no forbidden function » serait un faux vert.)
    if !flagged && ok {
        c.check(
            "forbidden function",
            true,
            "no forbidden function".to_string(),
        );
    }
    let (total, failed, checks) = c.finish();
    let summary = format!("{total} vérifications, {failed} en échec");
    super::step_finished(ctx, tx, Step::Symbols, ok, summary, checks);
    ok
}

/// Artefacts à scanner selon le type de projet : le binaire linké
/// (Binary), ou les `.o` des deliveries à la racine de la salle
/// blanche (Functions — contrat build.rs : un `.o` par SOURCE de
/// delivery, y compris chaque *.c d'une delivery dossier). Un `.o`
/// manquant (delivery non compilée) est ignoré : l'étape Build a déjà
/// remonté le KO.
fn artefacts_a_scanner(ctx: &PipelineContext) -> Vec<PathBuf> {
    let Some(build) = &ctx.build else {
        return Vec::new();
    };
    match ctx.battery.project.kind {
        ProjectType::Binary => build.binary.iter().cloned().collect(),
        ProjectType::Functions => {
            let mut out = Vec::new();
            for t in &ctx.battery.task {
                // Une delivery irrésoluble (dossier sans .c) a déjà
                // été KO en prelim/build : rien à scanner pour elle.
                let Ok(sources) = crate::battery::delivery_sources(build.dir.path(), &t.delivery)
                else {
                    continue;
                };
                out.extend(sources.iter().map(|s| {
                    build
                        .dir
                        .path()
                        .join(format!("{}.o", super::build::stem_of(&s.to_string_lossy())))
                }));
            }
            out.into_iter().filter(|p| p.is_file()).collect()
        }
    }
}

/// Symboles définis et indéfinis d'un artefact (noms strippés,
/// dédupliqués).
#[derive(Default)]
struct SymbolTable {
    defined: BTreeSet<String>,
    undefined: BTreeSet<String>,
}

/// Lit les symboles (`.symtab` + `.dynsym`) d'un artefact ELF et les
/// range dans `table`. Jamais de panic : un octet pourri, une lecture
/// impossible ou un format non-ELF remontent en `Err` — l'appelant en
/// fait un KO explicite.
fn collect_symbols(path: &Path, table: &mut SymbolTable) -> Result<()> {
    let data =
        fs::read(path).with_context(|| format!("lecture de {} impossible", path.display()))?;
    let file = object::File::parse(data.as_slice())
        .with_context(|| format!("{} n'est pas un objet ELF lisible", path.display()))?;
    absorb(file.symbols(), table);
    absorb(file.dynamic_symbols(), table);
    Ok(())
}

/// Range les symboles d'un itérateur (`.symtab` ou `.dynsym`) dans
/// `table` : nom strippé, vide ignoré, classé défini/indéfini.
fn absorb<'data, S: ObjectSymbol<'data>>(syms: impl Iterator<Item = S>, table: &mut SymbolTable) {
    for sym in syms {
        // Un symbole sans nom décodable ne dit rien : ignoré.
        let Ok(name) = sym.name() else { continue };
        let name = strip_version(name);
        if name.is_empty() {
            continue;
        }
        if sym.is_undefined() {
            table.undefined.insert(name.to_string());
        } else {
            table.defined.insert(name.to_string());
        }
    }
}

/// Tronque le suffixe de version ELF d'un nom de symbole :
/// `printf@@GLIBC_2.2.5` / `printf@GLIBC_2.2.5` → `printf`.
fn strip_version(name: &str) -> &str {
    // split('@') sur une chaîne non vide donne toujours au moins un
    // segment — d'où le unwrap_or défensif, jamais pris.
    name.split('@').next().unwrap_or(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_version_tronque_les_suffixes_elf() {
        assert_eq!(strip_version("printf@@GLIBC_2.2.5"), "printf");
        assert_eq!(strip_version("printf@GLIBC_2.2.5"), "printf");
        assert_eq!(strip_version("printf"), "printf");
        assert_eq!(strip_version("__stack_chk_fail"), "__stack_chk_fail");
        // Forme tordue : tout ce qui suit le premier '@' disparaît.
        assert_eq!(strip_version("sym@1@2"), "sym");
    }

    #[test]
    fn runtime_symbols_reste_trie() {
        assert!(
            RUNTIME_SYMBOLS.windows(2).all(|w| w[0] < w[1]),
            "RUNTIME_SYMBOLS doit rester trié (recherche dichotomique)"
        );
    }

    #[test]
    fn data_symbols_reste_trie() {
        assert!(
            DATA_SYMBOLS.windows(2).all(|w| w[0] < w[1]),
            "DATA_SYMBOLS doit rester trié (recherche dichotomique)"
        );
    }
}

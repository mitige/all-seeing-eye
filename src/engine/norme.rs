//! Étape Norme — coding style Epitech.
//!
//! Deux moteurs, choisis à la détection (`epiclang --version`
//! spawnable) :
//! - **officiel** : `epiclang -fsyntax-only` par fichier (cwd = rendu,
//!   chemins relatifs), sortie banana parsée ([`official`]) ;
//! - **interne** (fallback) : moteur Rust aux messages calibrés sur
//!   banana ([`internal`]), qui couvre aussi les `Makefile` (C-G1).
//!
//! `--strict-norme` sans epiclang → KO explicite de l'étape.
//!
//! Les fautes C-O1/C-O4 déjà rapportées par l'étape Prelim sont
//! converties en [`NormeFault`] (dédupliquées par (fichier, règle) :
//! le check « forbidden files » et la ligne banana-check-repo
//! doublonnent) pour le scoring norme du verdict.
//!
//! DÉCISION (documentée, à relire avec Task 10) : l'étape renvoie
//! `true` dès qu'elle a pu s'exécuter — même avec des fautes Major ou
//! Fatal. La pénalité des fautes est du ressort du verdict ; bloquer
//! ici masquerait le détail des fautes derrière un simple « KO » et
//! court-circuiterait le skip des étapes dépendantes du build, qui ne
//! concerne pas la norme. `false` uniquement si l'étape ne peut pas
//! s'exécuter (strict + epiclang absent). Les problèmes par fichier
//! (epiclang en timeout, source illisible) dégradent en check KO sans
//! faire échouer l'étape.
//!
//! `RunOpts::use_epiclang` pilote la COMPILATION (build) ; la norme
//! suit la spec : officiel dès qu'epiclang est détecté.

use super::events::{Event, Step};
use super::{Collect, PipelineContext};
use crate::norme::{collect_sources, internal, official, NormeFault, Severity};
use regex::Regex;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::LazyLock;

/// Ligne de check prelim porteuse d'une faute C-O1/C-O4, qu'elle
/// vienne de banana-check-repo (`./f: [Banana] [SEV] msg (C-Ox)`) ou
/// du check « forbidden files » maison (`./f: msg (C-Ox)` — sévérité
/// absente → Major, la sévérité banana de ces règles).
static PRELIM_FAULT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\./(.+?): (?:\[Banana\] \[(Fatal|Major|Minor|Info)\] )?(.+) \((C-O[14])\)$")
        .expect("regex faute prelim invalide")
});

/// Étape 3 du pipeline : détection des fautes de norme. Renvoie
/// `true` si l'étape a pu s'exécuter (voir le module pour la
/// décision), `false` en mode strict sans epiclang.
pub fn run(ctx: &mut PipelineContext, tx: &mpsc::Sender<Event>) -> bool {
    super::step_started(tx, Step::Norme);
    let mut c = Collect::new(tx, Step::Norme);
    merge_prelim_faults(ctx, tx);
    if !official::epiclang_available() {
        if ctx.opts.strict_norme {
            let summary = "epiclang requis en mode strict, absent".to_string();
            c.check("epiclang", false, summary.clone());
            let (_, _, checks) = c.finish();
            super::step_finished(ctx, tx, Step::Norme, false, summary, checks);
            return false;
        }
        run_internal(ctx, tx, &mut c);
    } else {
        run_official(ctx, tx, &mut c);
    }
    let summary = summarize(&ctx.norme_faults);
    let (_, _, checks) = c.finish();
    super::step_finished(ctx, tx, Step::Norme, true, summary, checks);
    true
}

/// Émet la faute en event et l'enregistre dans le contexte.
fn emit_fault(ctx: &mut PipelineContext, tx: &mpsc::Sender<Event>, f: NormeFault) {
    super::send(tx, Event::NormeFault(f.clone()));
    ctx.norme_faults.push(f);
}

/// Convertit les checks KO C-O1/C-O4 du StepReport prelim en
/// [`NormeFault`] (ligne/col inconnues → 0), dédupliquées par
/// (fichier, règle) — prelim émet sa propre ligne ET celle de
/// banana-check-repo pour le même fichier.
fn merge_prelim_faults(ctx: &mut PipelineContext, tx: &mpsc::Sender<Event>) {
    let mut extra = Vec::new();
    for report in &ctx.steps {
        if report.step != Step::Prelim.name() {
            continue;
        }
        for (_, ok, detail) in &report.checks {
            if *ok {
                continue;
            }
            for cap in detail.lines().filter_map(|l| PRELIM_FAULT_RE.captures(l)) {
                extra.push(NormeFault {
                    file: PathBuf::from(&cap[1]),
                    line: 0,
                    col: 0,
                    severity: cap
                        .get(2)
                        .and_then(|m| Severity::from_banana(m.as_str()))
                        .unwrap_or(Severity::Major),
                    message: cap[3].to_string(),
                    rule: cap[4].to_string(),
                });
            }
        }
    }
    for f in extra {
        if !ctx
            .norme_faults
            .iter()
            .any(|d| d.file == f.file && d.rule == f.rule)
        {
            emit_fault(ctx, tx, f);
        }
    }
}

/// « 3 major, 4 minor » — sévérités non nulles dans l'ordre
/// décroissant de gravité ; chaîne vide si aucune faute.
fn resume_severites(faults: &[NormeFault]) -> String {
    [
        Severity::Fatal,
        Severity::Major,
        Severity::Minor,
        Severity::Info,
    ]
    .into_iter()
    .filter_map(|sev| {
        let n = faults.iter().filter(|f| f.severity == sev).count();
        (n > 0).then(|| format!("{n} {}", sev.label()))
    })
    .collect::<Vec<_>>()
    .join(", ")
}

/// Résumé d'étape : décompte par sévérité, ou « coding style clean ».
fn summarize(faults: &[NormeFault]) -> String {
    let r = resume_severites(faults);
    if r.is_empty() {
        "coding style clean".to_string()
    } else {
        r
    }
}

/// Check « norme » d'un fichier : OK si aucune faute, sinon KO avec
/// le décompte des sévérités (ou la note d'exécution) en détail.
fn check_file_result(
    c: &mut Collect,
    rel: &std::path::Path,
    faults: &[NormeFault],
    note: Option<&str>,
) {
    if let Some(note) = note {
        c.check("norme", false, format!("{}: {note}", rel.display()));
    } else if faults.is_empty() {
        c.check("norme", true, format!("{}: clean", rel.display()));
    } else {
        c.check(
            "norme",
            false,
            format!("{}: {}", rel.display(), resume_severites(faults)),
        );
    }
}

/// Chemin officiel : epiclang par `.c`/`.h` du rendu.
fn run_official(ctx: &mut PipelineContext, tx: &mpsc::Sender<Event>, c: &mut Collect) {
    for rel in collect_sources(&ctx.target, false) {
        match official::check_one(&ctx.target, &rel) {
            Ok(outcome) => {
                check_file_result(c, &rel, &outcome.faults, outcome.note.as_deref());
                for f in outcome.faults {
                    emit_fault(ctx, tx, f);
                }
            }
            Err(e) => {
                c.check(
                    "norme",
                    false,
                    format!("{}: epiclang : {e:#}", rel.display()),
                );
            }
        }
    }
}

/// Fallback interne : moteur Rust sur `.c`/`.h`/`Makefile` (lus en
/// octets, convertis en lossy — le contenu non-UTF8 ne fait pas
/// paniquer l'étape).
fn run_internal(ctx: &mut PipelineContext, tx: &mpsc::Sender<Event>, c: &mut Collect) {
    for rel in collect_sources(&ctx.target, true) {
        let Some(kind) = internal::kind_of(&rel) else {
            continue;
        };
        match std::fs::read(ctx.target.join(&rel)) {
            Ok(bytes) => {
                let content = String::from_utf8_lossy(&bytes);
                let faults = internal::check_file(&rel, &content, kind);
                check_file_result(c, &rel, &faults, None);
                for f in faults {
                    emit_fault(ctx, tx, f);
                }
            }
            Err(e) => {
                c.check(
                    "norme",
                    false,
                    format!("{}: illisible : {e}", rel.display()),
                );
            }
        }
    }
}

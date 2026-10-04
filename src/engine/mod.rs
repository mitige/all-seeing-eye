//! Orchestrateur du pipeline de notation.
//!
//! Exécute les 7 étapes DANS L'ORDRE (spec §5) : Prelim, Build, Norme,
//! Symbols, Unit, Functional, Verdict. Un échec d'étape bloquante
//! (Prelim, Build) n'empêche PAS les étapes indépendantes (Norme,
//! Symbols) de tourner ; les étapes dépendantes du build (Unit,
//! Functional) sont marquées skipped si le build a échoué.
//!
//! Aucune dépendance TUI ici : la progression part dans un channel
//! d'[`Event`] consommé par le front.

pub mod events;

/// Publique pour permettre aux tests d'intégration de piloter l'étape
/// isolément (PipelineContext construit à la main).
pub mod build;
/// Publique pour permettre aux tests d'intégration de piloter l'étape
/// isolément (PipelineContext construit à la main).
pub mod functional;
/// Publique pour permettre aux tests d'intégration de piloter l'étape
/// isolément (PipelineContext construit à la main).
pub mod norme;
/// Publique pour permettre aux tests d'intégration de piloter l'étape
/// isolément (PipelineContext construit à la main).
pub mod prelim;
/// Publique pour permettre aux tests d'intégration de piloter l'étape
/// isolément (PipelineContext construit à la main).
pub mod symbols;
/// Publique pour permettre aux tests d'intégration de piloter l'étape
/// isolément (PipelineContext construit à la main).
pub mod unit;
mod verdict;

use crate::battery::Battery;
use crate::norme::NormeFault;
use crate::report::{Report, StepReport, TestRecord};
pub use events::{Event, Step, TestVerdict};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Instant;

/// Options d'un run, fournies par l'appelant (CLI/TUI).
#[derive(Debug, Clone)]
pub struct RunOpts {
    pub strict_norme: bool,
    pub use_epiclang: bool,
    pub compiler: PathBuf,
}

/// Artefacts produits par l'étape de compilation.
#[derive(Debug)]
pub struct BuildArtifacts {
    pub dir: tempfile::TempDir,
    pub binary: Option<PathBuf>,
}

/// Contexte partagé, mutable, passé à chaque étape.
///
/// `steps` et `report` complètent le plan initial : les résumés d'étapes
/// sont collectés au fil de l'eau (sinon le [`Report`] final ne pourrait
/// pas être construit depuis le ctx), et le rapport est posé par
/// l'étape verdict puis retourné par [`run_pipeline`].
#[derive(Debug)]
pub struct PipelineContext {
    pub battery: Battery,
    pub target: PathBuf,
    pub opts: RunOpts,
    pub build: Option<BuildArtifacts>,
    pub norme_faults: Vec<NormeFault>,
    pub tests: Vec<TestRecord>,
    pub step_ok: BTreeMap<Step, bool>,
    pub started: Instant,
    /// StepReports dans l'ordre d'exécution (alimenté par `step_finished`).
    pub steps: Vec<StepReport>,
    /// Posé par l'étape verdict, consommé par `run_pipeline`.
    pub report: Option<Report>,
}

/// Exécute les 7 étapes dans l'ordre et renvoie le rapport final.
pub fn run_pipeline(
    battery: &Battery,
    target: &Path,
    tx: mpsc::Sender<Event>,
    opts: &RunOpts,
) -> Report {
    let mut ctx = PipelineContext {
        battery: battery.clone(),
        target: target.to_path_buf(),
        opts: opts.clone(),
        build: None,
        norme_faults: Vec::new(),
        tests: Vec::new(),
        step_ok: BTreeMap::new(),
        started: Instant::now(),
        steps: Vec::new(),
        report: None,
    };

    run_step(Step::Prelim, &mut ctx, &tx, prelim::run);
    run_step(Step::Build, &mut ctx, &tx, build::run);
    run_step(Step::Norme, &mut ctx, &tx, norme::run);
    run_step(Step::Symbols, &mut ctx, &tx, symbols::run);
    run_step(Step::Unit, &mut ctx, &tx, unit::run);
    run_step(Step::Functional, &mut ctx, &tx, functional::run);
    verdict::run(&mut ctx, &tx);

    ctx.report.expect("le verdict pose toujours le rapport")
}

/// Lance une étape, ou la marque skipped si une dépendance a échoué.
fn run_step(
    step: Step,
    ctx: &mut PipelineContext,
    tx: &mpsc::Sender<Event>,
    f: fn(&mut PipelineContext, &mpsc::Sender<Event>) -> bool,
) {
    if let Some(reason) = skip_reason(step, &ctx.step_ok) {
        step_started(tx, step);
        step_skipped(ctx, tx, step, format!("skipped: {reason}"));
        ctx.step_ok.insert(step, false);
        return;
    }
    let ok = f(ctx, tx);
    // Invariant : toute étape exécutée appelle step_finished exactement une
    // fois. Un oubli devient un échec visible en debug au lieu d'un rapport
    // silencieusement incomplet.
    debug_assert!(
        ctx.steps.last().is_some_and(|s| s.step == step.name()),
        "étape {step:?} terminée sans step_finished : StepReport manquant"
    );
    ctx.step_ok.insert(step, ok);
}

/// Raison de skip d'une étape dépendante, le cas échéant (spec §5) :
/// Unit et Functional dépendent du build ; les autres sont indépendantes.
fn skip_reason(step: Step, step_ok: &BTreeMap<Step, bool>) -> Option<&'static str> {
    let build_ok = step_ok.get(&Step::Build).copied().unwrap_or(false);
    match step {
        Step::Unit | Step::Functional if !build_ok => Some("build failed"),
        _ => None,
    }
}

/// Envoie un event ; un receiver parti (front fermé) n'arrête pas le run.
fn send(tx: &mpsc::Sender<Event>, event: Event) {
    let _ = tx.send(event);
}

/// Nombre de lignes de stderr conservées dans le détail d'un KO.
pub(crate) const STDERR_TAIL: usize = 10;

/// Les `n` dernières lignes de `s` (trim fin), jointes par '\n' —
/// extrait borné d'un flux d'erreur pour le détail d'un KO.
pub(crate) fn tail(s: &str, n: usize) -> String {
    let lines: Vec<&str> = s.trim_end().lines().collect();
    let start = lines.len().saturating_sub(n);
    lines[start..].join("\n")
}

/// Collecteur de checks d'une étape : émet un [`Event::CheckFinished`]
/// par vérification, relaie les sorties de sous-processus en
/// [`Event::LogLine`], compte pour le résumé d'étape, et conserve les
/// checks pour le [`StepReport`] (transmis par [`Collect::finish`] à
/// [`step_finished`]).
pub(crate) struct Collect<'a> {
    tx: &'a mpsc::Sender<Event>,
    step: Step,
    total: usize,
    failed: usize,
    checks: Vec<(String, bool, String)>,
}

impl Collect<'_> {
    /// Crée un collecteur estampillé `step`, émettant sur `tx`.
    pub(crate) fn new(tx: &mpsc::Sender<Event>, step: Step) -> Collect<'_> {
        Collect {
            tx,
            step,
            total: 0,
            failed: 0,
            checks: Vec::new(),
        }
    }

    /// Émet un check, le conserve pour le [`StepReport`], et renvoie
    /// son statut (pour l'`&=` de l'étape).
    pub(crate) fn check(&mut self, name: &str, ok: bool, detail: String) -> bool {
        self.total += 1;
        if !ok {
            self.failed += 1;
        }
        self.checks.push((name.to_string(), ok, detail.clone()));
        send(
            self.tx,
            Event::CheckFinished {
                step: self.step,
                name: name.to_string(),
                ok,
                detail,
            },
        );
        ok
    }

    /// Relaie chaque ligne d'un flux capturé en [`Event::LogLine`].
    pub(crate) fn log(&self, s: &str) {
        for line in s.lines() {
            send(
                self.tx,
                Event::LogLine {
                    step: self.step,
                    line: line.to_string(),
                },
            );
        }
    }

    /// Bilan `(total, en échec, checks émis)` pour le résumé d'étape
    /// et le [`StepReport`] — consomme le collecteur : les checks sont
    /// transmis tels quels à [`step_finished`].
    pub(crate) fn finish(self) -> (usize, usize, Vec<(String, bool, String)>) {
        (self.total, self.failed, self.checks)
    }
}

/// Début d'étape : émet `StepStarted`.
fn step_started(tx: &mpsc::Sender<Event>, step: Step) {
    send(
        tx,
        Event::StepStarted {
            step,
            label: step.label().to_string(),
        },
    );
}

/// Fin d'étape normale : émet `StepFinished` et enregistre le
/// `StepReport`, checks inclus (`Collect::finish` les transmet ; une
/// étape sans collecteur passe `Vec::new()`).
fn step_finished(
    ctx: &mut PipelineContext,
    tx: &mpsc::Sender<Event>,
    step: Step,
    ok: bool,
    summary: String,
    checks: Vec<(String, bool, String)>,
) {
    step_finished_impl(ctx, tx, step, ok, false, summary, checks);
}

/// Étape sautée (dépendance échouée) : `StepFinished` avec `skipped: true`
/// et `ok: false` — un skip n'est jamais un succès. `summary` porte la
/// raison humaine ("skipped: …").
fn step_skipped(ctx: &mut PipelineContext, tx: &mpsc::Sender<Event>, step: Step, summary: String) {
    step_finished_impl(ctx, tx, step, false, true, summary, Vec::new());
}

/// Étape désactivée par la configuration de la batterie (ex. Symbols
/// sans `allowed_functions`) : `StepFinished` avec `skipped: true`
/// ET `ok: true` — le check n'a pas vocation à tourner ; ce n'est ni
/// un échec ni le skip d'une dépendance échouée ([`step_skipped`],
/// où ok reste false).
fn step_disabled(ctx: &mut PipelineContext, tx: &mpsc::Sender<Event>, step: Step, summary: String) {
    step_finished_impl(ctx, tx, step, true, true, summary, Vec::new());
}

/// Implémentation commune : émet `StepFinished` et enregistre le `StepReport`.
fn step_finished_impl(
    ctx: &mut PipelineContext,
    tx: &mpsc::Sender<Event>,
    step: Step,
    ok: bool,
    skipped: bool,
    summary: String,
    checks: Vec<(String, bool, String)>,
) {
    send(
        tx,
        Event::StepFinished {
            step,
            ok,
            skipped,
            summary: summary.clone(),
        },
    );
    ctx.steps.push(StepReport {
        step: step.name().to_string(),
        ok,
        skipped,
        summary,
        checks,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// step_ok avec Prelim ok et Build au statut demandé.
    fn step_ok_with_build(build_ok: bool) -> BTreeMap<Step, bool> {
        let mut m = BTreeMap::new();
        m.insert(Step::Prelim, true);
        m.insert(Step::Build, build_ok);
        m
    }

    #[test]
    fn unit_et_functional_skips_si_build_ko() {
        let m = step_ok_with_build(false);
        assert_eq!(skip_reason(Step::Unit, &m), Some("build failed"));
        assert_eq!(skip_reason(Step::Functional, &m), Some("build failed"));
    }

    #[test]
    fn norme_et_symbols_tournent_meme_si_build_ko() {
        let m = step_ok_with_build(false);
        assert_eq!(skip_reason(Step::Norme, &m), None);
        assert_eq!(skip_reason(Step::Symbols, &m), None);
        assert_eq!(skip_reason(Step::Prelim, &m), None);
        assert_eq!(skip_reason(Step::Verdict, &m), None);
    }

    #[test]
    fn aucun_skip_si_build_ok() {
        let m = step_ok_with_build(true);
        for s in [
            Step::Prelim,
            Step::Build,
            Step::Norme,
            Step::Symbols,
            Step::Unit,
            Step::Functional,
            Step::Verdict,
        ] {
            assert_eq!(skip_reason(s, &m), None, "skip inattendu pour {s:?}");
        }
    }
}

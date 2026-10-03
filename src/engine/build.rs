//! Étape Build — compilation (stub, Task 3).

use super::events::{Event, Step};
use super::{BuildArtifacts, PipelineContext};
use std::sync::mpsc;

/// Stub : réussit toujours, mais pose la structure [`BuildArtifacts`]
/// (dossier temporaire dédié au build) pour les vraies étapes à venir.
pub fn run(ctx: &mut PipelineContext, tx: &mpsc::Sender<Event>) -> bool {
    super::step_started(tx, Step::Build);
    match tempfile::tempdir() {
        Ok(dir) => {
            ctx.build = Some(BuildArtifacts { dir, binary: None });
            super::step_finished(ctx, tx, Step::Build, true, "stub".to_string());
            true
        }
        Err(e) => {
            super::step_finished(
                ctx,
                tx,
                Step::Build,
                false,
                format!("tempdir impossible : {e}"),
            );
            false
        }
    }
}

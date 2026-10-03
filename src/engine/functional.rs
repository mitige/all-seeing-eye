//! Étape Functional — tests fonctionnels end-to-end (stub, Task 3).

use super::events::{Event, Step};
use super::PipelineContext;
use std::sync::mpsc;

/// Stub : émet Started/Finished et réussit toujours.
pub fn run(ctx: &mut PipelineContext, tx: &mpsc::Sender<Event>) -> bool {
    super::step_started(tx, Step::Functional);
    super::step_finished(ctx, tx, Step::Functional, true, "stub".to_string());
    true
}

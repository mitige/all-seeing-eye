//! Étape Unit — tests unitaires des exercices (stub, Task 3).

use super::events::{Event, Step};
use super::PipelineContext;
use std::sync::mpsc;

/// Stub : émet Started/Finished et réussit toujours.
pub fn run(ctx: &mut PipelineContext, tx: &mpsc::Sender<Event>) -> bool {
    super::step_started(tx, Step::Unit);
    super::step_finished(ctx, tx, Step::Unit, true, "stub".to_string());
    true
}

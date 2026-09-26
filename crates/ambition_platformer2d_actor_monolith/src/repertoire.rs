//! Scheduling of each body's effective repertoire.
//!
//! [`reconcile_effective_repertoire`](ambition_combat::hand::reconcile_effective_repertoire)
//! folds what a body can do from its identity, its worn equipment and what it
//! holds. It runs after the persona phase, which rewrites the identity kit on a
//! kit swap, so a verb gained or lost is in the same tick's repertoire.
//!
//! The slot-to-action scheme is not stored. Each reader derives it from the
//! body's live authorities with `derive_action_scheme`: the persona gate
//! (`gate_body_control`) and the control prompt. So the button and what it
//! fires cannot disagree.

use ambition_platformer2d_shared_tangle::schedule::{
    Platformer2dSimulationPhaseMonolith, SimScheduleExt,
};
use bevy::prelude::*;

/// Wires the effective-repertoire reconcile into the sim schedule.
pub struct EffectiveRepertoirePlugin;

impl Plugin for EffectiveRepertoirePlugin {
    fn build(&self, app: &mut App) {
        let sim = app.sim_schedule();
        app.add_systems(
            sim,
            ambition_combat::hand::reconcile_effective_repertoire
                .in_set(ambition_combat::hand::EffectiveRepertoireReconciled)
                .after(ambition_platformer2d_shared_tangle::schedule::PlayerInputSet::Persona)
                .in_set(Platformer2dSimulationPhaseMonolith::PlayerInput),
        );
    }
}

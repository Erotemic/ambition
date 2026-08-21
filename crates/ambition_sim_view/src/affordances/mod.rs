//! **What the interact button would do right now.**
//!
//! One question, answered once per tick and read by the HUD and the portal
//! input adapter: is there something in reach, and what kind of thing is it?
//! [`NearestInteractable`] carries the answer as an [`InteractVariant`].
//!
//! ## What this used to be, and why it is not
//!
//! ⛔⛔ **it was a whole affordance TABLE — *"what would each input do right
//! now?"* for jump, attack, shield, dash, special and interact — and 1,200 of
//! its 1,725 lines had no reader at all.** Its own module doc said the HUD read
//! the table and that gameplay would follow; the HUD had since stopped, and
//! gameplay never started. A census on 2026-08-21 found `PlayerAffordances`,
//! `PlayerIntent`, `Aim`, `PogoTargetBelow` and the Attack/Jump/Shield/Dash/
//! Special variant families named NOWHERE outside this module — the only
//! mention of the central product was its own rollback declaration.
//!
//! ⚠ **and it had just been CARVED out of the monolith into this crate**, which
//! is how a dead subsystem earns a second life: a move is easy to justify and
//! says nothing about whether anything wants the thing. Counting consumers is
//! what "does this belong here" actually means, and it should have come first.
//!
//! ⇒ what survives is what somebody calls: the interactable proximity scan and
//! its classification. The rest is deleted rather than relocated again.

use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt;
use bevy::prelude::*;

pub mod interactable_proximity;
pub mod variants;

pub use interactable_proximity::{NearestInteractable, update_nearest_interactable};
pub use variants::{InteractVariant, VariantLabel};

/// SystemSet for the interactable scan. Readers run
/// `.after(AffordancesSystemSet::Compute)` so they see this tick's value —
/// the touch overlay orders its interact button that way.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AffordancesSystemSet {
    /// The proximity scan and its classification.
    Compute,
}

/// Installs the interactable scan.
pub struct AffordancesPlugin;

impl Plugin for AffordancesPlugin {
    fn build(&self, app: &mut App) {
        let sim = app.sim_schedule();
        app.init_resource::<NearestInteractable>().add_systems(
            sim,
            update_nearest_interactable
                // It reads the controlled body's position against feature
                // volumes, so it runs after this tick's input has settled which
                // body is controlled.
                .after(ambition_platformer2d_actor_monolith::control::PrimarySlotInputCommit)
                .in_set(AffordancesSystemSet::Compute),
        );
    }
}

//! **Wiring the reset horizon into the tick.**
//!
//! The vocabulary and the domain systems live in
//! [`ambition_platformer2d_shared_tangle::lifecycle::horizon`] and beside the
//! authorities they snapshot. What lives HERE is the composition, because the
//! two ordering facts the horizon depends on name three different crates: the
//! item chain that writes a checkpoint commit (`ambition_platformer2d_actor_monolith`),
//! the room replay that a restore must precede ([`crate::sandbox_reset`]), and
//! the sets themselves (`ambition_platformer2d_shared_tangle`). Registering from
//! any one of those would make it depend upward on another.
//!
//! # The two placements, and both are load-bearing
//!
//! ```text
//! PlayerInput        CheckpointRestore  →  RoomReplayApplied
//! PlayerSimulation   ItemPickupSet::CoreHeldItems  →  CheckpointCapture
//! ```
//!
//! ⭐ **RESTORE EARLY, CAPTURE LATE, and they are in different phases on
//! purpose.** A restore must land before anything rebuilds a room, because the
//! rebuild's whole input is the occurrence ledger this restores; a capture must
//! land after everything that could still change what is being captured this
//! tick.
//!
//! ⛔⛔ **the capture placement is the one that is easy to get subtly wrong.**
//! The shrine writes [`CheckpointCommitted`] from `PlayerSimulation`. Put the
//! capture in `PlayerInput` — the tidy-looking choice, next to its sibling — and
//! it reads the message on the FOLLOWING tick, with a whole player simulation in
//! between. An object picked up in that window lands in the baseline as though
//! it had been in the player's hands when they touched the shrine, so a later
//! death gives it back to them. That is the third line of the maintainer's
//! fixture failing for a reason that has nothing to do with the fixture.

use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::lifecycle::{
    capture_custody_baseline, capture_occurrence_baseline, restore_occurrence_baseline,
    retract_custody_to_checkpoint, CheckpointCapture, CheckpointCommitted, CheckpointRestore,
    CustodyBaseline, OccurrenceBaseline, ResetToCheckpoint,
};
use ambition_platformer2d_shared_tangle::schedule::{
    Platformer2dSimulationPhaseMonolith, SimScheduleExt,
};

/// Installs the reset horizon: its two channels, the two domain baselines, and
/// the placement of the capture and restore sets.
///
/// ⚠ **a host that installs this and never emits [`CheckpointCommitted`] gets
/// the empty baseline**, so a [`ResetToCheckpoint`] returns every authored
/// occurrence to where its record puts it. That is the sandbox reset's meaning,
/// reached as the degenerate case rather than as a separate road — which is the
/// property that keeps a game with no checkpoints from needing a second answer.
pub struct CheckpointHorizonPlugin;

impl Plugin for CheckpointHorizonPlugin {
    fn build(&self, app: &mut App) {
        let sim = app.sim_schedule();
        app.add_message::<CheckpointCommitted>()
            .add_message::<ResetToCheckpoint>()
            .init_resource::<OccurrenceBaseline>()
            .init_resource::<CustodyBaseline>();

        app.configure_sets(
            sim,
            CheckpointRestore
                .in_set(Platformer2dSimulationPhaseMonolith::PlayerInput)
                // ⭐ THE edge the whole transaction rests on. The room replay
                // reads the ledger this restores; after it, the baseline would
                // take effect one room load late.
                .before(crate::sandbox_reset::RoomReplayApplied),
        );
        app.configure_sets(
            sim,
            CheckpointCapture
                .in_set(Platformer2dSimulationPhaseMonolith::PlayerSimulation)
                // The shrine (the commit producer) and the custody projection
                // (which settles the ledger this tick) are both in this chain.
                .after(ambition_platformer2d_actor_monolith::items::pickup::ItemPickupSet::CoreHeldItems),
        );

        app.add_systems(
            sim,
            // ⚠ NOT chained, and that is the design rather than an omission: a
            // capture reads live state and writes only its own domain's
            // snapshot, so no capture can observe another's output and no order
            // between them is expressible as a bug. The day one of them wants
            // another's result, they are one domain wearing two names.
            (capture_occurrence_baseline, capture_custody_baseline).in_set(CheckpointCapture),
        );
        app.add_systems(
            sim,
            // Same independence on the way back: the ledger restore touches no
            // entity, and the custody retraction touches no ledger row.
            (restore_occurrence_baseline, retract_custody_to_checkpoint).in_set(CheckpointRestore),
        );
    }
}

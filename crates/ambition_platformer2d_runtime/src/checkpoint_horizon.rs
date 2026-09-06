//! Host wiring for the reset/checkpoint horizon.
//!
//! The host owns only the two cross-domain ordering facts:
//!
//! ```text
//! PlayerInput        CheckpointRestore  ->  RoomReplayApplied
//! PlayerSimulation ...               ->  CheckpointCapture
//! ```
//!
//! Concrete baseline resources and capture/restore systems are domain-owned.
//! The lifecycle layer contributes occurrence/custody state and the actor domain
//! contributes item/shrine policy. This is intentionally the same ownership
//! shape as rollback federation: composition names a domain offer, never the
//! concrete types inside it.

use bevy::prelude::{App, IntoScheduleConfigs, Plugin};

use ambition_platformer2d_shared_tangle::lifecycle::{
    CheckpointCapture, CheckpointCommitted, CheckpointRestore,
    LifecycleCheckpointHorizonPlugin, ResetToCheckpoint,
};
use ambition_platformer2d_shared_tangle::schedule::{
    Platformer2dSimulationPhaseMonolith, SimScheduleExt,
};

/// Installs the reset horizon's channels, host-level ordering, and typed domain
/// contributions.
///
/// A host that never emits [`CheckpointCommitted`] keeps each domain's empty
/// baseline. [`ResetToCheckpoint`] therefore reconstructs authored/start state as
/// the degenerate no-checkpoint case rather than through a second reset road.
pub struct CheckpointHorizonPlugin;

impl Plugin for CheckpointHorizonPlugin {
    fn build(&self, app: &mut App) {
        let sim = app.sim_schedule();

        app.add_message::<CheckpointCommitted>()
            .add_message::<ResetToCheckpoint>();

        app.configure_sets(
            sim,
            CheckpointRestore
                .in_set(Platformer2dSimulationPhaseMonolith::PlayerInput)
                .before(crate::sandbox_reset::RoomReplayAdmission),
        );
        app.configure_sets(
            sim,
            CheckpointCapture.in_set(Platformer2dSimulationPhaseMonolith::PlayerSimulation),
        );

        // The host composes domains. It deliberately does not name occurrence,
        // custody, minted-item, or entitlement baseline types — those lists now
        // live with the domains that own their semantics.
        app.add_plugins((
            LifecycleCheckpointHorizonPlugin,
            ambition_platformer2d_actor_monolith::ActorCheckpointHorizonPlugin,
        ));
    }
}

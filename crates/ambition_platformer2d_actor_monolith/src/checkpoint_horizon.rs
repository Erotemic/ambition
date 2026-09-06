//! Actor-side contribution to the reset/checkpoint horizon.
//!
//! The generic runtime chooses *when* checkpoint capture/restore runs. This
//! module owns which actor/item policies participate in those sets. Keeping the
//! offer here means adding an item-domain baseline changes the item domain, not
//! `ambition_platformer2d_runtime`.

use bevy::prelude::{App, IntoScheduleConfigs, Plugin};

use ambition_platformer2d_shared_tangle::lifecycle::CheckpointRestore;
use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt;

/// Typed actor-domain checkpoint contribution composed by the platformer host.
pub struct ActorCheckpointHorizonPlugin;

impl Plugin for ActorCheckpointHorizonPlugin {
    fn build(&self, app: &mut App) {
        let sim = app.sim_schedule();

        app.add_plugins(crate::items::pickup::minted_horizon::ItemCheckpointHorizonPlugin)
            .add_systems(
                sim,
                crate::shrine::resume_at_checkpoint_on_reset.in_set(CheckpointRestore),
            );
    }
}

//! Actor-side contribution to the reset/checkpoint horizon.
//!
//! The generic runtime chooses *when* checkpoint capture/restore runs. This
//! module owns which actor/item policies participate in those sets. Keeping the
//! offer here means adding an item-domain baseline changes the item domain, not
//! `ambition_platformer2d_runtime`.

use bevy::prelude::{App, Plugin};

/// Typed actor-domain checkpoint contribution composed by the platformer host.
///
/// ⭐ TWO OFFERS, NAMED SEPARATELY. This used to install the session's reset
/// resume inline beside the item plugin, which made "the actor domain
/// contributes to the checkpoint horizon" one indivisible thing. It is two: a
/// session lifecycle offer and an item-domain offer. A profile that wants
/// checkpoints without held items adds
/// [`SessionCheckpointHorizonPlugin`](crate::session::checkpoint::SessionCheckpointHorizonPlugin)
/// alone; this wrapper is the full Ambition composition of both.
pub struct ActorCheckpointHorizonPlugin;

impl Plugin for ActorCheckpointHorizonPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            crate::session::checkpoint::SessionCheckpointHorizonPlugin,
            crate::items::pickup::minted_horizon::ItemCheckpointHorizonPlugin,
        ));
    }
}

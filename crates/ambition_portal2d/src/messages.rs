//! Portal intent and outcome messages.
//!
//! Portal core consumes host-neutral intents. A generator's gestures (fire,
//! toggle) name the body that is the [`PortalGenerator`](crate::PortalGenerator).

use bevy::prelude::*;

use super::color::PortalChannel;

/// Intent to fire a portal generator; the host lowers it to
/// [`PortalFireIntent`].
#[derive(Message, Clone, Copy, Debug)]
pub struct FirePortalGenerator {
    /// World-space aim direction for the shot (need not be normalized; the
    /// resolver normalizes and ignores a zero vector).
    pub aim: Vec2,
    /// The generator whose press this is. Every generator gesture names it, so
    /// each seat can fire its own.
    ///
    /// An `Entity` in a message is safe here: these messages are
    /// `clear_message_on_rollback`, so they never cross a rollback boundary.
    pub body: Entity,
}

/// Host-neutral request to fire a portal shot from `origin` along `dir` for `channel`.
///
/// Not `Copy`: the shot's `SimId` holds a `String`. Readers clone it.
#[derive(Message, Clone, Debug)]
pub struct PortalFireIntent {
    /// World-space spawn point of the shot.
    pub origin: Vec2,
    /// World-space fire direction (need not be normalized; core normalizes and
    /// ignores a zero vector).
    pub dir: Vec2,
    /// Which portal channel the shot opens on contact.
    pub channel: PortalChannel,
    /// The shot's simulation identity, minted by whoever fired it.
    ///
    /// A portal shot is a rollback anchor (`require_rollback::<PortalShot>`),
    /// so it needs an identity. Only the emitter can derive one, as with
    /// `spawn_module_entity` and `drop_hazard`.
    ///
    /// `None` is a shot with no identity (a script or fixture without a body).
    /// The populated timeline's identity census reports every anonymous
    /// anchor. Shots are short-lived, so the census must sample while they
    /// exist.
    pub id: Option<ambition_platformer2d_shared_tangle::sim_id::SimId>,
    /// The live room the shot is fired in. The emitter knows it, as it knows
    /// [`Self::id`]: it is the room of the body that fired.
    ///
    /// The shot carries it as its `InRoomInstance`, and the portal it opens
    /// carries it too. With two live rooms, the host steps the shot against
    /// that room's solids. `None` is a shot in no known room; with two live
    /// rooms it does not move.
    pub room: Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>,
}

/// Intent: toggle which end of its pair a generator opens next. The host has
/// already decided this gesture belongs to the generator.
#[derive(Message, Clone, Copy, Debug)]
pub struct TogglePortalGenerator {
    /// The generator whose press this is — see [`FirePortalGenerator::body`].
    pub body: Entity,
}

/// Developer intent: turn every portal generator on or off. A host asks for
/// it and the simulation flips it, so every replay of the tick flips it again.
#[derive(Message, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TogglePortalGeneratorsActive;

/// Portal-owned reset intent: clear placed portals and body transit cooldowns.
#[derive(Message, Clone, Copy, Debug)]
pub struct ClearPortals;

/// Outcome emitted when [`PortalFireIntent`] spawns a portal shot.
#[derive(Message, Clone, Copy, Debug)]
pub struct PortalShotFired {
    /// World-space spawn point of the shot (where the fire cue plays).
    pub origin: Vec2,
}

/// Outcome emitted when a body begins straddling an aperture.
/// Audio/presentation adapters may use `pos`; portal core owns no audio policy.
#[derive(Message, Clone, Copy, Debug)]
pub struct PortalBodyEntered {
    /// World position of the entry portal (where the ENTER cue plays).
    pub pos: Vec2,
}

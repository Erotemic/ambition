//! Portal intent and outcome messages.
//!
//! Portal core consumes host-neutral intents. Gun-specific gesture messages remain a compatibility
//! surface until equivalent generic ownership/emitter intents exist.

use bevy::prelude::*;

use super::color::PortalChannel;

/// Compatibility intent for firing a held portal gun; the host lowers it to
/// [`PortalFireIntent`].
#[derive(Message, Clone, Copy, Debug)]
pub struct FirePortalGun {
    /// World-space aim direction for the shot (need not be normalized; the
    /// resolver normalizes and ignores a zero vector).
    pub aim: Vec2,
    /// The body whose press this is. Every gun gesture names its body, so
    /// each seat can fire its own gun.
    ///
    /// An `Entity` in a message is safe here: these four messages are
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
    /// `deploy_sentry`, `open_vortex_well`, and `drop_hazard`.
    ///
    /// `None` is a shot with no identity (a script or fixture without a body).
    /// The populated timeline's identity census reports every anonymous
    /// anchor. Shots are short-lived, so the census must sample while they
    /// exist.
    pub id: Option<ambition_platformer2d_shared_tangle::sim_id::SimId>,
}

/// Compatibility intent: toggle which color the held portal gun will place
/// next. The host has already decided this gesture belongs to the gun.
#[derive(Message, Clone, Copy, Debug)]
pub struct TogglePortalGun {
    /// The body whose press this is — see [`FirePortalGun::body`].
    pub body: Entity,
}

/// Compatibility intent to drop the held portal gun as a world pickup.
#[derive(Message, Clone, Copy, Debug)]
pub struct DropPortalGun {
    /// The body whose press this is — see [`FirePortalGun::body`].
    pub body: Entity,
}

/// Compatibility intent to acquire an overlapping portal-gun pickup.
#[derive(Message, Clone, Copy, Debug)]
pub struct PickUpPortalGun {
    /// The body whose press this is — see [`FirePortalGun::body`].
    pub body: Entity,
}

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

/// Compatibility outcome emitted when an entity acquires a portal-gun pickup.
#[derive(Message, Clone, Copy, Debug)]
pub struct PortalGunEquipped {
    /// Entity that now holds the gun.
    ///
    /// FIXME(portal-gun-seam): rename this field to `carrier` when the host
    /// adapter migration can tolerate the API break.
    pub player: Entity,
}

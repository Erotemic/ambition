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
}

/// Host-neutral request to fire a portal shot from `origin` along `dir` for `channel`.
#[derive(Message, Clone, Copy, Debug)]
pub struct PortalFireIntent {
    /// World-space spawn point of the shot.
    pub origin: Vec2,
    /// World-space fire direction (need not be normalized; core normalizes and
    /// ignores a zero vector).
    pub dir: Vec2,
    /// Which portal channel the shot opens on contact.
    pub channel: PortalChannel,
}

/// Compatibility intent: toggle which color the held portal gun will place
/// next. The host has already decided this gesture belongs to the gun.
#[derive(Message, Clone, Copy, Debug)]
pub struct TogglePortalGun;

/// Compatibility intent to drop the held portal gun as a world pickup.
#[derive(Message, Clone, Copy, Debug)]
pub struct DropPortalGun;

/// Compatibility intent to acquire an overlapping portal-gun pickup.
#[derive(Message, Clone, Copy, Debug)]
pub struct PickUpPortalGun;

/// Portal-owned reset intent: clear placed portals and body transit cooldowns.
#[derive(Message, Clone, Copy, Debug)]
pub struct ClearPortals;

/// Outcome emitted when [`PortalFireIntent`] spawns a portal shot.
#[derive(Message, Clone, Copy, Debug)]
pub struct PortalShotFired {
    /// World-space spawn point of the shot (where the fire cue plays).
    pub origin: Vec2,
}

/// Outcome emitted when a [`PortalBody`](super::PortalBody) begins straddling an aperture.
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

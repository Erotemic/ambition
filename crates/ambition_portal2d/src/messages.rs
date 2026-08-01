//! Reusable portal intent / outcome messages.
//!
//! Portal core consumes these instead of reading host-specific input or
//! inventory types directly. A host adapter translates controls, scripts, AI,
//! or inventory state into these messages.
//!
//! FIXME(portal-gun-seam): the `*PortalGun` gesture messages are Ambition's
//! current compatibility surface. The reusable portal API should prefer generic
//! portal-open / portal-clear / portal-emitter intents, keeping gun vocabulary
//! in a host or optional gun module.

use bevy::prelude::*;

use super::color::PortalChannel;

/// Compatibility intent: fire the held portal gun this frame along `aim`
/// (already resolved to a world-space direction by the host adapter). The host
/// turns this gun-specific gesture into a generic [`PortalFireIntent`] that the
/// portal core spawns a shot from.
#[derive(Message, Clone, Copy, Debug)]
pub struct FirePortalGun {
    /// World-space aim direction for the shot (need not be normalized; the
    /// resolver normalizes and ignores a zero vector).
    pub aim: Vec2,
}

/// Generic fire intent the portal core consumes to place/replace a portal: a
/// shot of `channel` from `origin` along `dir`. This is the generic portal
/// opener seam: a replay, AI, script, moving emitter, authored trigger, or gun
/// adapter can all place a portal by emitting this. Portal core never reaches
/// for a controlled actor, gun, or inventory.
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

/// Compatibility intent: drop the held portal gun, leaving a grabbable pickup
/// at the carrier's feet. The host owns gesture recognition and inventory
/// bookkeeping; core performs the entity-level drop while this workflow exists.
#[derive(Message, Clone, Copy, Debug)]
pub struct DropPortalGun;

/// Compatibility intent: attempt to pick up an overlapping portal gun. Core
/// checks armed pickups and emits [`PortalGunEquipped`]; the host reflects the
/// grant into inventory / ability state.
#[derive(Message, Clone, Copy, Debug)]
pub struct PickUpPortalGun;

/// Intent: clear all placed portals and any body's transit cooldown — the
/// portal-owned reset signal. Portal core consumes this instead of reading the
/// host's reset event; a host portal adapter emits it when a room resets /
/// transitions, so portal core never names that event.
#[derive(Message, Clone, Copy, Debug)]
pub struct ClearPortals;

/// Outcome: a portal shot was just fired (a `PortalFireIntent` was consumed and
/// a [`PortalShot`](super::PortalShot) spawned). Carries the shot's spawn point
/// so a host audio adapter can play the fire-blast + travel-whizz cues —
/// the portal crate emits the event, not the sfx (it owns neither audio nor the
/// sfx vocabulary). `origin` is the shot's world-space spawn position.
#[derive(Message, Clone, Copy, Debug)]
pub struct PortalShotFired {
    /// World-space spawn point of the shot (where the fire cue plays).
    pub origin: Vec2,
}

/// Outcome: a [`PortalBody`](super::PortalBody) just BEGAN straddling a portal
/// aperture (the leading edge entered the opening, before the centroid crosses).
/// Carries the entry portal's world position so a host audio adapter can play
/// the ENTER cue. The companion EXIT cue rides
/// [`PortalBodyTransited`](super::PortalBodyTransited) (its `exit_pos`). Portal
/// core emits these events; the adapter owns the audio.
#[derive(Message, Clone, Copy, Debug)]
pub struct PortalBodyEntered {
    /// World position of the entry portal (where the ENTER cue plays).
    pub pos: Vec2,
}

/// Compatibility outcome: an entity acquired a portal gun via a world pickup.
/// The host inventory adapter listens for this to reflect ownership/equipped
/// state into its own roster — portal core never touches that roster.
#[derive(Message, Clone, Copy, Debug)]
pub struct PortalGunEquipped {
    /// Entity that now holds the gun.
    ///
    /// FIXME(portal-gun-seam): rename this field to `carrier` when the host
    /// adapter migration can tolerate the API break.
    pub player: Entity,
}

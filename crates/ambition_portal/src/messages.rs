//! Reusable portal intent / outcome messages.
//!
//! Portal core consumes these instead of reading Ambition-specific input
//! (`ControlFrame`) or inventory (`Item` / `OwnedItems`) types directly. An
//! Ambition adapter (the host portal adapter) translates control
//! frames and inventory state into these intents, so the portal simulation
//! stays content-agnostic: anything that can emit a `FirePortalGun` (a replay,
//! an AI, a different game's input layer) drives the gun the same way.

use bevy::prelude::*;

use super::color::PortalChannel;

/// Intent: fire the held portal gun this frame along `aim` (already resolved to
/// a world-space direction by the input adapter — right-stick aim, else
/// movement axis, else facing). The Ambition resolver
/// (the host portal adapter) turns this
/// player-and-gun-implying gesture into a generic [`PortalFireIntent`] that the
/// portal core spawns a shot from. The shield-gated "this is actually a drop
/// gesture" decision is made by the input adapter, so a `FirePortalGun` here is
/// always a genuine fire.
#[derive(Message, Clone, Copy, Debug)]
pub struct FirePortalGun {
    /// World-space aim direction for the shot (need not be normalized; the
    /// resolver normalizes and ignores a zero vector).
    pub aim: Vec2,
}

/// Generic fire intent the portal core consumes to place/replace a portal: a
/// shot of `channel` from `origin` along `dir`. It drops the "primary player +
/// held `PortalGun`" assumption of [`FirePortalGun`] — anything (a replay, an AI,
/// a future emitter that isn't the player) can place a portal by emitting this.
/// The Ambition resolver maps `FirePortalGun` (gesture) → this (origin/dir from
/// the primary player's body, channel from the held gun's current color), so
/// behavior is identical; portal core never reaches for the player / gun /
/// inventory.
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

/// Intent: toggle which color the held portal gun will place next. The adapter
/// has already decided this press belongs to the gun (e.g. no interactable
/// claimed it), so core just flips the color.
#[derive(Message, Clone, Copy, Debug)]
pub struct TogglePortalGun;

/// Intent: drop the held portal gun, leaving a grabbable pickup at the player's
/// feet. The adapter owns the gesture recognition (Shield+Attack) and the
/// inventory bookkeeping; core performs the entity-level drop.
#[derive(Message, Clone, Copy, Debug)]
pub struct DropPortalGun;

/// Intent: attempt to pick up a portal gun the primary player is overlapping
/// (the adapter recognizes the Attack-while-not-holding gesture). Core checks
/// overlap with armed pickups and grants the gun; it emits [`PortalGunEquipped`]
/// so the inventory adapter can reflect the grant into the Ambition roster.
#[derive(Message, Clone, Copy, Debug)]
pub struct PickUpPortalGun;

/// Intent: clear all placed portals and any body's transit cooldown — the
/// portal-owned reset signal. Portal core consumes this instead of reading the
/// Ambition `ResetRoomFeaturesEvent`; the room-reset adapter
/// (the host portal adapter) emits
/// it when a room resets / transitions, so portal core never names the Ambition
/// reset event.
#[derive(Message, Clone, Copy, Debug)]
pub struct ClearPortals;

/// Outcome: a portal shot was just fired (a `PortalFireIntent` was consumed and
/// a [`PortalShot`](super::PortalShot) spawned). Carries the shot's spawn point
/// so an Ambition audio adapter can play the fire-blast + travel-whizz cues —
/// the portal crate emits the event, not the sfx (it owns neither audio nor the
/// sfx vocabulary). `origin` is the shot's world-space spawn position.
#[derive(Message, Clone, Copy, Debug)]
pub struct PortalShotFired {
    /// World-space spawn point of the shot (where the fire cue plays).
    pub origin: Vec2,
}

/// Outcome: a [`PortalBody`](super::PortalBody) just BEGAN straddling a portal
/// aperture (the leading edge entered the opening, before the centroid crosses).
/// Carries the entry portal's world position so an Ambition audio adapter can
/// play the ENTER cue. The companion EXIT cue rides
/// [`PortalBodyTransited`](super::PortalBodyTransited) (its `exit_pos`). Portal
/// core emits these events; the adapter owns the audio.
#[derive(Message, Clone, Copy, Debug)]
pub struct PortalBodyEntered {
    /// World position of the entry portal (where the ENTER cue plays).
    pub pos: Vec2,
}

/// Outcome: the primary player just acquired a portal gun (via a world pickup).
/// The inventory adapter listens for this to reflect ownership / equipped state
/// into the Ambition item roster — portal core never touches that roster.
#[derive(Message, Clone, Copy, Debug)]
pub struct PortalGunEquipped {
    /// The player entity that now holds the gun.
    pub player: Entity,
}

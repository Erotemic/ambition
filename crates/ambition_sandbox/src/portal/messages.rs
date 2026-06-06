//! Reusable portal intent / outcome messages.
//!
//! Portal core consumes these instead of reading Ambition-specific input
//! (`ControlFrame`) or inventory (`Item` / `OwnedItems`) types directly. An
//! Ambition adapter (`crate::ambition_content::portal`) translates control
//! frames and inventory state into these intents, so the portal simulation
//! stays content-agnostic: anything that can emit a `FirePortalGun` (a replay,
//! an AI, a different game's input layer) drives the gun the same way.

use bevy::prelude::*;

/// Intent: fire the held portal gun this frame along `aim` (already resolved to
/// a world-space direction by the input adapter — right-stick aim, else
/// movement axis, else facing). Portal core spawns the shot of the gun's
/// current color. The shield-gated "this is actually a drop gesture" decision
/// is made by the adapter, so a `FirePortalGun` here is always a genuine fire.
#[derive(Message, Clone, Copy, Debug)]
pub struct FirePortalGun {
    /// World-space aim direction for the shot (need not be normalized; core
    /// normalizes and ignores a zero vector).
    pub aim: Vec2,
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

/// Outcome: the primary player just acquired a portal gun (via a world pickup).
/// The inventory adapter listens for this to reflect ownership / equipped state
/// into the Ambition item roster — portal core never touches that roster.
#[derive(Message, Clone, Copy, Debug)]
pub struct PortalGunEquipped {
    /// The player entity that now holds the gun.
    pub player: Entity,
}

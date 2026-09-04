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
    /// The body whose press this is.
    ///
    /// ⭐⭐ THE GESTURE NAMES ITS BODY, and every gun gesture below does the
    /// same. It used to carry an aim and nothing else, so the resolver had to
    /// re-derive the firer from `ControlledSubject` — one entity by construction
    /// — and a couch's second seat holding a portal gun could not fire. A
    /// resolver that instead looped every driven body would have had to GUESS
    /// whose press it was, and would have fired one shot per body for one press.
    ///
    /// ⚠ An `Entity` in a message is safe HERE and only here: these four are
    /// `clear_message_on_rollback`, so they are produced and consumed inside one
    /// tick and never cross a rollback boundary.
    pub body: Entity,
}

/// Host-neutral request to fire a portal shot from `origin` along `dir` for `channel`.
///
/// ⚠ NOT `Copy` since 2026-09-04: it carries the shot's `SimId`, whose payload
/// is a `String`. Readers clone it.
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
    /// ⛔⛔ A PORTAL SHOT IS A ROLLBACK ANCHOR (`require_rollback::<PortalShot>`)
    /// and shipped ANONYMOUS: it rewound by entity index rather than by
    /// identity, and it is the entity that decides where a portal opens. The
    /// emitter mints it because the emitter is the only party that HAS an
    /// identity to derive from — the same shape `deploy_sentry`,
    /// `open_vortex_well` and `drop_hazard` already take (`Some(mint())` as
    /// their last argument), and for the same reason.
    ///
    /// ⚠ `None` is a shot with no identity, which is what a script or a fixture
    /// firing without a body produces. It is not a silent default: the populated
    /// timeline's identity census names every anonymous anchor, so a `None` that
    /// reaches production reddens there.
    ///
    /// ⭐ MEASURED 2026-09-04, and the reason it went unseen for so long is
    /// worth carrying: the census walked the world only at frame 60, by which
    /// time every shot has fizzled or placed. Widening WHEN it looks — not what
    /// `populate` creates — is what found this.
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

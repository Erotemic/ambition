//! Shared portal types, geometry constants, and small helpers used across the
//! portal submodules (placement, transit, presentation, …).

use bevy::prelude::*;

use crate::pieces::PortalFrame;

use super::color::PortalChannel;

/// One placed portal. The pair is linked implicitly by `channel` — two portals
/// pair iff they share a channel (the gun's Blue/Orange, or an authored pair).
#[derive(Component, Clone, Copy, Debug)]
pub struct PlacedPortal {
    pub channel: PortalChannel,
    /// World-space center (on the hit surface).
    pub pos: Vec2,
    /// Unit surface normal, pointing out of the wall into the room.
    pub normal: Vec2,
    /// Half-extent of the portal's overlap region.
    pub half_extent: Vec2,
}

impl PlacedPortal {
    /// The pure-geometry frame this portal presents to [`crate::pieces`]
    /// (the Core invariant math: piece decomposition, carve, portal map).
    pub fn frame(&self) -> PortalFrame {
        PortalFrame {
            pos: self.pos,
            normal: self.normal,
            half_extent: self.half_extent,
        }
    }
}

/// The placed portal on `channel`, if any.
pub fn find_portal<'a>(
    portals: impl IntoIterator<Item = &'a PlacedPortal>,
    channel: PortalChannel,
) -> Option<PlacedPortal> {
    portals.into_iter().find(|p| p.channel == channel).copied()
}

/// A portal opening is the SAME size in every orientation: a doorway
/// `PORTAL_OPENING_HALF * 2` long along the surface, and thin perpendicular to
/// it (we only see its side profile in 2D). Both the drawn face AND the capture
/// box that warps the player are built from these, so the warp happens right at
/// the visual face regardless of whether the portal is on a wall, floor, or
/// ceiling.
pub(crate) const PORTAL_OPENING_HALF: f32 = 46.0;
pub(crate) const PORTAL_THICKNESS_HALF: f32 = 9.0;
pub(crate) const PORTAL_MAX_RANGE: f32 = 6000.0;
/// PlacedPortal shot travel speed (px/s) — fast, but slow enough to see the streak.
pub(crate) const PORTAL_SHOT_SPEED: f32 = 1900.0;
pub(crate) const TELEPORT_COOLDOWN_S: f32 = 0.25;
/// Floor on exit speed so a slow walk into a portal still pops you out the
/// far side instead of stalling inside the exit portal. Public so a host's
/// transit invariant tests can assert against the floor.
pub const MIN_EXIT_SPEED: f32 = 220.0;
/// On-screen thickness of the thin portal doorway (side profile in 2D). The
/// bar's *length* comes from the portal opening; this is its narrow dimension,
/// matched to the capture box so the player warps right at the drawn face.
pub const PORTAL_VISUAL_THICKNESS: f32 = PORTAL_THICKNESS_HALF * 2.0;

/// Oriented half-extent for a portal on a surface with the given `normal`:
/// `PORTAL_OPENING_HALF` along the surface (perpendicular to the normal) and
/// `PORTAL_THICKNESS_HALF` through it. So the opening (face) is the same length
/// in every orientation and the box is thin in the normal direction. An
/// axis-aligned normal gives an exact thin box; a slanted normal gives the
/// axis-aligned box that bounds the tilted face (good enough until slanted
/// portals are real).
pub fn portal_half_extent(normal: Vec2) -> Vec2 {
    portal_half_extent_with_length(normal, PORTAL_OPENING_HALF)
}

/// [`portal_half_extent`] with an explicit along-surface half-length (e.g. the
/// authored LDtk box), keeping the standard through-surface thickness. For
/// portals whose opening size is authored rather than the fixed default.
pub fn portal_half_extent_with_length(normal: Vec2, along_half: f32) -> Vec2 {
    let n = normal.normalize_or_zero();
    let along = Vec2::new(-n.y, n.x);
    Vec2::new(
        along.x.abs() * along_half + n.x.abs() * PORTAL_THICKNESS_HALF,
        along.y.abs() * along_half + n.y.abs() * PORTAL_THICKNESS_HALF,
    )
}

/// The along-surface half-length (opening size) of an oriented half-extent —
/// the inverse of [`portal_half_extent_with_length`]'s along component.
pub fn portal_opening_half(normal: Vec2, half_extent: Vec2) -> f32 {
    let n = normal.normalize_or_zero();
    half_extent.dot(Vec2::new(-n.y, n.x).abs())
}

/// Standard through-surface half-thickness, exposed so the aperture-equalizer
/// can rebuild a half-extent from a new along-length.
pub const PORTAL_FACE_HALF_THICKNESS: f32 = PORTAL_THICKNESS_HALF;

/// How far out of the exit portal (along its normal) to pop a body so it clears
/// the thin portal face without immediately re-entering: the body's half-size
/// projected onto the normal, plus the portal's thickness and a hair of margin.
/// Pops the body out right next to the face — NOT the old over-large
/// `half_extent.length()` push that included the full opening length.
pub(crate) fn portal_exit_clearance(half_size: Vec2, exit_normal: Vec2) -> f32 {
    half_size.dot(exit_normal.abs()) + PORTAL_THICKNESS_HALF + 3.0
}

/// Per-actor cooldown after a portal jump, so an actor that pops out of the
/// exit doesn't immediately re-enter and ping-pong. Inserted on teleport and
/// ticked down by [`super::transit::tick_portal_cooldowns`].
#[derive(Component, Clone, Copy, Debug)]
pub struct PortalTransitCooldown(pub f32);

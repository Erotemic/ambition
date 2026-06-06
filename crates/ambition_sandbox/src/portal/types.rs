//! Shared portal types, geometry constants, and small helpers used across the
//! portal submodules (placement, transit, presentation, …).

use bevy::prelude::*;

use crate::portal_pieces::PortalFrame;

use super::color::PortalColor;

/// One placed portal. The pair is linked implicitly by `color` (one Blue +
/// one Orange exist at most).
#[derive(Component, Clone, Copy, Debug)]
pub struct Portal {
    pub color: PortalColor,
    /// World-space center (on the hit surface).
    pub pos: Vec2,
    /// Unit surface normal, pointing out of the wall into the room.
    pub normal: Vec2,
    /// Half-extent of the portal's overlap region.
    pub half_extent: Vec2,
}

impl Portal {
    /// The pure-geometry frame this portal presents to [`crate::portal_pieces`]
    /// (the Core invariant math: piece decomposition, carve, portal map).
    pub fn frame(&self) -> PortalFrame {
        PortalFrame {
            pos: self.pos,
            normal: self.normal,
            half_extent: self.half_extent,
        }
    }
}

/// The placed portal of `color`, if any.
pub(crate) fn find_portal<'a>(
    portals: impl IntoIterator<Item = &'a Portal>,
    color: PortalColor,
) -> Option<Portal> {
    portals.into_iter().find(|p| p.color == color).copied()
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
/// Portal shot travel speed (px/s) — fast, but slow enough to see the streak.
pub(crate) const PORTAL_SHOT_SPEED: f32 = 1900.0;
pub(crate) const TELEPORT_COOLDOWN_S: f32 = 0.25;
/// Floor on exit speed so a slow walk into a portal still pops you out the
/// far side instead of stalling inside the exit portal.
pub(crate) const MIN_EXIT_SPEED: f32 = 220.0;
/// On-screen thickness of the thin portal doorway (side profile in 2D). The
/// bar's *length* comes from the portal opening; this is its narrow dimension,
/// matched to the capture box so the player warps right at the drawn face.
pub(crate) const PORTAL_VISUAL_THICKNESS: f32 = PORTAL_THICKNESS_HALF * 2.0;

/// Oriented half-extent for a portal on a surface with the given `normal`:
/// `PORTAL_OPENING_HALF` along the surface (perpendicular to the normal) and
/// `PORTAL_THICKNESS_HALF` through it. So the opening (face) is the same length
/// in every orientation and the box is thin in the normal direction. An
/// axis-aligned normal gives an exact thin box; a slanted normal gives the
/// axis-aligned box that bounds the tilted face (good enough until slanted
/// portals are real).
pub fn portal_half_extent(normal: Vec2) -> Vec2 {
    let n = normal.normalize_or_zero();
    let along = Vec2::new(-n.y, n.x);
    Vec2::new(
        along.x.abs() * PORTAL_OPENING_HALF + n.x.abs() * PORTAL_THICKNESS_HALF,
        along.y.abs() * PORTAL_OPENING_HALF + n.y.abs() * PORTAL_THICKNESS_HALF,
    )
}

/// How far out of the exit portal (along its normal) to pop a body so it clears
/// the thin portal face without immediately re-entering: the body's half-size
/// projected onto the normal, plus the portal's thickness and a hair of margin.
/// Pops the body out right next to the face — NOT the old over-large
/// `half_extent.length()` push that included the full opening length.
pub(crate) fn portal_exit_clearance(half_size: Vec2, exit_normal: Vec2) -> f32 {
    half_size.dot(exit_normal.abs()) + PORTAL_THICKNESS_HALF + 3.0
}

/// One-frame flag: set true the frame the player teleports through a portal,
/// so the trace position-delta detector treats it as an *intentional* teleport
/// and doesn't auto-dump. Read + cleared by the gameplay-trace system.
#[derive(Resource, Default)]
pub struct IntentionalTeleport(pub bool);

/// Per-actor cooldown after a portal jump, so an actor that pops out of the
/// exit doesn't immediately re-enter and ping-pong. Inserted on teleport and
/// ticked down by [`super::transit::tick_portal_cooldowns`].
#[derive(Component, Clone, Copy, Debug)]
pub struct PortalCooldown(pub f32);

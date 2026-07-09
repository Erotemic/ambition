//! Shared portal types, geometry constants, and small helpers used across the
//! portal submodules (placement, transit, presentation, …).

use ambition_engine_core as ae;
use bevy::prelude::*;

use crate::pieces::{PortalAperture, PortalFrame};

use super::color::PortalChannel;

/// One placed portal. The pair is linked implicitly by `channel` — two portals
/// pair iff their channels are partners.
///
/// FIXME(portal-api): this is still Ambition's compact runtime component. A
/// standalone crate should expose a less-opinionated portal descriptor that can
/// represent authored/static portals, runtime-opened portals, moving portals,
/// arbitrary aperture bases, and host-defined link keys.
#[derive(Component, Clone, Debug)]
pub struct PlacedPortal {
    pub channel: PortalChannel,
    /// World-space center (on the hit surface). For a HOSTED portal this is a
    /// per-frame derived cache — the host refresh re-derives it from
    /// [`Self::host`] each frame (§5-P2); for an unhosted portal it is the
    /// placement value, unchanged forever.
    pub pos: Vec2,
    /// Unit surface normal, pointing out of the host surface into the room.
    /// Current collision/render helpers are cardinal-first; future APIs should
    /// make the tangent/aperture basis explicit for non-axis-aligned portals.
    pub normal: Vec2,
    /// Half-extent of the portal's overlap region.
    pub half_extent: Vec2,
    /// CC6 host attachment: the durable face this aperture rides
    /// (`PortalHostRef = GeoFaceRef` — §3.6). `None` = an unhosted STATIC
    /// aperture (fixtures, worlds without identified geometry): frame velocity
    /// zero, byte-identical to the pre-CC6 portal. Attribution is lazy — the
    /// host adapter attaches placed portals to identified faces; a hosted
    /// portal whose face disappears from the composed world CLOSES.
    pub host: Option<ae::GeoFaceRef>,
    /// The placement's authored lift of `pos` off the host face along
    /// `normal` (the gun places 2px proud of the wall). Recorded at
    /// attachment so the per-frame re-derivation preserves it exactly.
    pub host_lift: f32,
    /// The aperture's own velocity in px/s (`PortalFrame::velocity` — feeds
    /// the Galilean transfer map). ZERO for unhosted/static portals; the host
    /// refresh derives it from the host block's authoritative velocity.
    pub vel: Vec2,
    /// `pos` at the START of this frame — the aperture's own sweep sample.
    /// `pos - prev_pos` is the exact frame displacement the RELATIVE swept
    /// transit trigger subtracts (§5-P2 step 5). Maintained by the host
    /// refresh; equal to `pos` for unhosted portals.
    pub prev_pos: Vec2,
}

impl PlacedPortal {
    /// A static (unhosted) portal — the pre-CC6 shape. Fixtures and
    /// placement sites construct through this; the host adapter may attach
    /// a host afterward.
    pub fn fixed(channel: PortalChannel, pos: Vec2, normal: Vec2, half_extent: Vec2) -> Self {
        Self {
            channel,
            pos,
            normal,
            half_extent,
            host: None,
            host_lift: 0.0,
            vel: Vec2::ZERO,
            prev_pos: pos,
        }
    }

    /// The aperture's own displacement THIS frame (§5-P2 relative sweep
    /// term). Zero for unhosted portals by construction.
    pub fn frame_delta(&self) -> Vec2 {
        if self.host.is_some() {
            self.pos - self.prev_pos
        } else {
            Vec2::ZERO
        }
    }
}

impl PlacedPortal {
    /// The pure-geometry frame this portal presents to the portal map (the
    /// engine-level CC5 type: origin + normal; velocity ZERO — static portals.
    /// CC6 moving portals derive it from the host's pose + mover velocity).
    pub fn frame(&self) -> PortalFrame {
        PortalFrame {
            origin: self.pos,
            normal: self.normal,
            velocity: self.vel,
        }
    }

    /// Frame + opening extent — what the piece decomposition, straddle test,
    /// carve, and (CC5) portal-aware casts consume.
    pub fn aperture(&self) -> PortalAperture {
        PortalAperture {
            frame: self.frame(),
            half_length: portal_opening_half(self.normal, self.half_extent),
        }
    }
}

/// The placed portal on `channel`, if any.
pub fn find_portal<'a>(
    portals: impl IntoIterator<Item = &'a PlacedPortal>,
    channel: PortalChannel,
) -> Option<PlacedPortal> {
    portals.into_iter().find(|p| p.channel == channel).cloned()
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

/// AABB half-extent for a portal on a surface with the given `normal`:
/// `PORTAL_OPENING_HALF` along the surface (perpendicular to the normal) and
/// `PORTAL_THICKNESS_HALF` through it. So the opening (face) is the same length
/// in every orientation and the box is thin in the normal direction. An
/// axis-aligned normal gives an exact thin box; a slanted normal gives the
/// axis-aligned box that bounds the tilted face.
///
/// FIXME(portal-api): keep this helper for Ambition's AABB world, but do not
/// make bounding boxes the only public representation of slanted portals.
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

/// How far out of the exit portal (along its normal) to pop a body so it clears
/// the thin portal face without immediately re-entering: the body's half-size
/// projected onto the normal, plus the portal's thickness and a hair of margin.
/// Pops the body out right next to the face — NOT the old over-large
/// `half_extent.length()` push that included the full opening length.
pub(crate) fn portal_exit_clearance(half_size: Vec2, exit_normal: Vec2) -> f32 {
    half_size.dot(exit_normal.abs()) + PORTAL_THICKNESS_HALF + 3.0
}

/// Per-actor, PAIR-SCOPED cooldown after a portal jump, so an actor that pops
/// out of the exit doesn't immediately re-Begin into the pair it just crossed.
/// Scoped to the crossed pair: entering a DIFFERENT pair immediately after a
/// crossing is legitimate (chained-portal rooms). Inserted on teleport and
/// ticked down by [`super::transit::tick_portal_cooldowns`]. The rescue path
/// in `transit_step` ignores it entirely (a centroid mid-fall-through must
/// always transfer).
#[derive(Component, Clone, Copy, Debug)]
pub struct PortalTransitCooldown {
    /// Remaining latch time (s).
    pub remaining: f32,
    /// The pair the body just crossed (either end's channel; the latch matches
    /// both partners).
    pub pair: PortalChannel,
}

/// Measured solid host material behind each placed portal's face, published by
/// the HOST (it owns the collision world) each frame — portal core stays
/// world-free. Consumed by the transit rescue and the carve so a portal on a
/// THIN wall never grabs or engages a body standing in the open room BEHIND
/// that wall: the aperture volume ends where the wall does. A channel with no
/// entry reads as unmeasured (`f32::INFINITY` = unclipped), which callers
/// bound by [`crate::pieces::CARVE_DEPTH`].
#[derive(bevy::prelude::Resource, Clone, Debug, Default)]
pub struct PortalHostDepths(pub Vec<(PortalChannel, f32)>);

impl PortalHostDepths {
    /// The measured host depth for `channel`, or `INFINITY` when unmeasured.
    pub fn depth(&self, channel: PortalChannel) -> f32 {
        self.0
            .iter()
            .find(|(c, _)| *c == channel)
            .map(|(_, d)| *d)
            .unwrap_or(f32::INFINITY)
    }
}

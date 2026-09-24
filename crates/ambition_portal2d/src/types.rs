//! Shared portal types, geometry constants, and small helpers used across the
//! portal submodules (placement, transit, presentation, …).

use ambition_platformer2d_core as ae;
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
    /// World-space center (on the hit surface). For a hosted portal the host
    /// refresh re-derives it from [`Self::host`] each frame. For an unhosted
    /// portal it is the fixed placement value.
    pub pos: Vec2,
    /// Unit surface normal, pointing out of the host surface into the room.
    /// Current collision/render helpers are cardinal-first; future APIs should
    /// make the tangent/aperture basis explicit for non-axis-aligned portals.
    pub normal: Vec2,
    /// Half-extent of the portal's overlap region.
    pub half_extent: Vec2,
    /// The host face this aperture rides (`GeoFaceRef`). `None` is an unhosted
    /// static aperture (fixtures, worlds without identified geometry) with zero
    /// frame velocity. The host adapter attaches placed portals to faces later.
    /// A hosted portal closes when its face disappears.
    pub host: Option<ae::GeoFaceRef>,
    /// The offset of `pos` off the host face along `normal` (the gun places
    /// 2 px out from the wall). Recorded at attachment so the per-frame
    /// re-derivation keeps it.
    pub host_lift: f32,
    /// The aperture velocity in px/s (`PortalFrame::velocity`, used by the
    /// Galilean transfer map). Zero for unhosted portals; the host refresh
    /// derives it from the host block velocity.
    pub vel: Vec2,
    /// `pos` at the start of this frame. `pos - prev_pos` is the frame
    /// displacement that the relative swept transit trigger subtracts. Set by
    /// the host refresh; equal to `pos` for unhosted portals.
    pub prev_pos: Vec2,
}

impl PlacedPortal {
    /// A static (unhosted) portal. The host adapter may attach a host later.
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

    /// The aperture displacement this frame (the relative sweep term). Zero
    /// for unhosted portals.
    pub fn frame_delta(&self) -> Vec2 {
        if self.host.is_some() {
            self.pos - self.prev_pos
        } else {
            Vec2::ZERO
        }
    }
}

impl PlacedPortal {
    /// The geometry frame this portal gives the portal map: origin, normal,
    /// and velocity.
    pub fn frame(&self) -> PortalFrame {
        PortalFrame {
            origin: self.pos,
            normal: self.normal,
            velocity: self.vel,
        }
    }

    /// Frame and opening extent, used by the piece decomposition, straddle
    /// test, carve, and portal-aware casts.
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
    // Callers pass portals collected from a `Query` (archetype order, which a
    // rollback resimulation may not reproduce). Pick by
    // `stable_portal_order`, not the first match. Channels are usually unique
    // after `resolve_portal_links`, but nothing here requires it.
    //
    // This makes the choice deterministic. Which portal a shared channel
    // should lead to is an authoring question (awaiting-maintainer-decision
    // #65).
    portals
        .into_iter()
        .filter(|p| p.channel == channel)
        .min_by(|a, b| stable_portal_order(a, b))
        .cloned()
}

/// The crate's one tie-break between portals, for every place that picks one
/// of several and must pick the same one each time: [`find_portal`],
/// `link::equalize_pair_apertures`, and the transit loops.
///
/// Lowest position first. `Query` order is archetype order, which a rollback
/// resimulation may not reproduce. One shared rule also keeps the sites from
/// disagreeing with each other. Uses `total_cmp`, so there is no `unwrap` and
/// no NaN gap.
pub fn stable_portal_order(a: &PlacedPortal, b: &PlacedPortal) -> std::cmp::Ordering {
    a.pos
        .x
        .total_cmp(&b.pos.x)
        .then_with(|| a.pos.y.total_cmp(&b.pos.y))
}

/// A portal opening is the same size in every orientation:
/// `PORTAL_OPENING_HALF * 2` long along the surface, and thin across it. The
/// drawn face and the capture box both use these, so the warp happens at the
/// visible face.
pub(crate) const PORTAL_OPENING_HALF: f32 = 46.0;
/// Standard through-surface half-thickness, exposed so the aperture-equalizer
/// can rebuild a half-extent from a new along-length.
pub(crate) const PORTAL_THICKNESS_HALF: f32 = 9.0;
pub(crate) const PORTAL_MAX_RANGE: f32 = 6000.0;
/// PlacedPortal shot travel speed (px/s) — fast, but slow enough to see the streak.
pub(crate) const PORTAL_SHOT_SPEED: f32 = 1900.0;
pub(crate) const TELEPORT_COOLDOWN_S: f32 = 0.25;
/// Minimum exit speed, so a slow walk into a portal still comes out of the
/// exit. Public for host transit tests.
pub const MIN_EXIT_SPEED: f32 = 220.0;
/// On-screen thickness of the portal doorway. Matches the capture box, so
/// the player warps at the drawn face.
pub const PORTAL_VISUAL_THICKNESS: f32 = PORTAL_THICKNESS_HALF * 2.0;

/// AABB half-extent for a portal on a surface with the given `normal`:
/// `PORTAL_OPENING_HALF` along the surface and `PORTAL_THICKNESS_HALF` through
/// it. A slanted normal gives the axis-aligned box that bounds the tilted face.
///
/// FIXME(portal-api): keep this helper for Ambition's AABB world, but do not
/// make bounding boxes the only public representation of slanted portals.
pub fn portal_half_extent(normal: Vec2) -> Vec2 {
    portal_half_extent_with_length(normal, PORTAL_OPENING_HALF)
}

/// [`portal_half_extent`] with an explicit along-surface half-length (e.g. the authored LDtk
/// box), keeping the standard through-surface thickness.
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

/// How far out of the exit portal (along its normal) to place a body so it
/// clears the face and does not re-enter: the body's half-size projected on
/// the normal, plus the portal thickness and a small margin.
pub(crate) fn portal_exit_clearance(half_size: Vec2, exit_normal: Vec2) -> f32 {
    half_size.dot(exit_normal.abs()) + PORTAL_THICKNESS_HALF + 3.0
}

/// Per-actor cooldown after a portal jump, so the actor does not Begin into
/// the pair it just crossed. A different pair can be entered at once
/// (chained-portal rooms). Inserted on teleport and ticked down by
/// [`super::transit::tick_portal_cooldowns`]. The rescue path in
/// `transit_step` ignores it.
#[derive(Component, Clone, Copy, Debug)]
pub struct PortalTransitCooldown {
    /// Remaining latch time (s).
    pub remaining: f32,
    /// The pair the body just crossed (either end's channel; the latch matches
    /// both partners).
    pub pair: PortalChannel,
}

/// The transit rescue and the carve use it, so a portal on a thin wall does
/// not grab a body behind the wall. A channel with no entry is unmeasured
/// (`f32::INFINITY`), and callers bound it by [`crate::pieces::CARVE_DEPTH`].
#[derive(bevy::prelude::Resource, Clone, Debug, Default)]
pub struct PortalHostDepths(pub Vec<(PortalChannel, f32)>);

impl PortalHostDepths {
    pub fn depth(&self, channel: PortalChannel) -> f32 {
        self.0
            .iter()
            .find(|(c, _)| *c == channel)
            .map(|(_, d)| *d)
            .unwrap_or(f32::INFINITY)
    }
}

#[cfg(test)]
mod find_portal_determinism_tests {
    use super::*;
    use crate::color::PortalChannelColor;

    fn portal(channel: PortalChannel, x: f32, y: f32) -> PlacedPortal {
        PlacedPortal {
            channel,
            pos: Vec2::new(x, y),
            normal: Vec2::new(0.0, 1.0),
            half_extent: Vec2::new(PORTAL_OPENING_HALF, PORTAL_THICKNESS_HALF),
            host: None,
            host_lift: 0.0,
            vel: Vec2::ZERO,
            prev_pos: Vec2::new(x, y),
        }
    }

    /// A contrived tie: `find_portal` does not require unique channels. The
    /// same set in a different order must give the same answer.
    #[test]
    fn the_same_apertures_in_a_different_order_resolve_to_the_same_one() {
        let purple = PortalChannelColor::Purple.channel();
        let mut apertures: Vec<PlacedPortal> = (0..7)
            .map(|i| portal(purple, 400.0 - (i as f32) * 37.0, 100.0 + (i as f32) * 11.0))
            .collect();

        let forward = find_portal(&apertures, purple).expect("a purple aperture");
        apertures.reverse();
        let reversed = find_portal(&apertures, purple).expect("a purple aperture");
        apertures.rotate_left(3);
        let rotated = find_portal(&apertures, purple).expect("a purple aperture");

        assert_eq!(
            forward.pos, reversed.pos,
            "reversing the aperture order changed which portal the channel \
             resolves to — a rollback resimulation reorders the query, so this \
             is a body warping somewhere else on a replayed frame"
        );
        assert_eq!(rotated.pos, forward.pos, "rotating the order changed the answer");
    }

    /// Control: a channel with one aperture finds it, and a channel with none
    /// gives `None`.
    #[test]
    fn a_single_aperture_and_an_absent_channel_are_unchanged() {
        let purple = PortalChannelColor::Purple.channel();
        let yellow = PortalChannelColor::Yellow.channel();
        let only = vec![portal(yellow, 12.0, 34.0)];
        assert_eq!(
            find_portal(&only, yellow).map(|p| p.pos),
            Some(Vec2::new(12.0, 34.0))
        );
        assert!(find_portal(&only, purple).is_none());
    }
}

#[cfg(test)]
mod stable_order_tests {
    use super::*;
    use crate::color::PortalChannelColor;

    fn at(x: f32, y: f32) -> PlacedPortal {
        PlacedPortal {
            channel: PortalChannelColor::Purple.channel(),
            pos: Vec2::new(x, y),
            normal: Vec2::new(0.0, 1.0),
            half_extent: Vec2::new(PORTAL_OPENING_HALF, PORTAL_THICKNESS_HALF),
            host: None,
            host_lift: 0.0,
            vel: Vec2::ZERO,
            prev_pos: Vec2::new(x, y),
        }
    }

    /// The same portals in any order sort the same way. Checked from three
    /// starting permutations, so a comparator that always returns `Equal`
    /// fails.
    #[test]
    fn any_permutation_of_the_same_portals_sorts_identically() {
        let scene = [at(300.0, 10.0), at(100.0, 50.0), at(300.0, 5.0), at(-40.0, 0.0)];

        let mut forward = scene.to_vec();
        forward.sort_by(stable_portal_order);

        let mut reversed = scene.to_vec();
        reversed.reverse();
        reversed.sort_by(stable_portal_order);

        let mut rotated = scene.to_vec();
        rotated.rotate_left(2);
        rotated.sort_by(stable_portal_order);

        let key = |v: &Vec<PlacedPortal>| v.iter().map(|p| p.pos).collect::<Vec<_>>();
        assert_eq!(key(&forward), key(&reversed));
        assert_eq!(key(&forward), key(&rotated));
        // A real ordering: leftmost first, and equal x is split by y.
        assert_eq!(forward[0].pos, Vec2::new(-40.0, 0.0));
        assert_eq!(forward[2].pos, Vec2::new(300.0, 5.0));
        assert_eq!(forward[3].pos, Vec2::new(300.0, 10.0));
    }
}

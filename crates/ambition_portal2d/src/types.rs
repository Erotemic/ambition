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
    /// Unit surface normal, pointing out of the host surface into the room,
    /// at any angle. The opening runs along its tangent.
    pub normal: Vec2,
    /// The pose (`pos`, `normal`) at the start of this tick, before its host
    /// moved it. The host refresh sets it for every portal each tick, and
    /// anything that places a portal sets it to the new pose. So a portal
    /// whose start pose is not the last tick's end pose was placed again
    /// (eviction), and `prev → now` is the host's rigid motion this tick
    /// (the relative swept transit trigger).
    pub prev_pos: Vec2,
    pub prev_normal: Vec2,
    /// The opening as its owner placed it: half its length along the face,
    /// and half its thickness across it. The opening that transit, carve and
    /// drawing use is [`Self::opening_half_length`]: a pair opens the
    /// smaller of its two openings, and nothing writes that back here.
    pub half_length: f32,
    pub half_thickness: f32,
    /// The authored opening half-length of this portal's partner, or `None`
    /// when it has no partner. `equalize_pair_apertures` derives it each
    /// frame from the two authored lengths. It is not authored.
    pub partner_opening_half: Option<f32>,
    /// The face this aperture rides, if any. A hosted portal closes when its
    /// face disappears.
    pub host: PortalHost,
    /// The host's motion of the aperture, as the host published it (CC6,
    /// never a difference of positions): the velocity in px/s of its point at
    /// `pivot`, and its turn about `pivot` in rad/s
    /// ([`PortalFrame::velocity_at`]). Zero for unhosted portals.
    pub vel: Vec2,
    pub spin: f32,
    pub pivot: Vec2,
}

/// The channel whose colours draw a portal, when that is not the channel that
/// pairs it. Presentation reads it; pairing, transit and carve never do. A
/// portal paired by a [`crate::PortalLink`] gets its channel from the free
/// pairs of its room, and an owner that wants its own colours (a boss whose
/// pair is blue and orange) says so here. Colour is a look, not an identity.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PortalLook(pub PortalChannel);

/// Which face, if any, a portal rides. One value, so "not looked at yet",
/// "static" and "hosted" cannot disagree, and a rewind restores the answer
/// with the portal.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum PortalHost {
    /// The host adapter has not looked for a face yet. It looks on its next
    /// pass, before the portal links.
    #[default]
    Unattributed,
    /// No identified face was under the portal when the adapter looked. The
    /// aperture stays static, with zero frame velocity, for its lifetime.
    Static,
    /// The aperture rides `face`. `lift` is the offset of `pos` off the face
    /// along the normal (the gun places a portal 2 px out from the wall), so
    /// each re-derivation keeps the placement pose.
    Face { face: ae::GeoFaceRef, lift: f32 },
    /// The aperture rides the line named `line` (a plank platform, a slope
    /// that moves), `along` its length from its first point, `lift` out along
    /// its normal: it moves with the line and turns as it turns.
    Line { line: String, along: f32, lift: f32 },
}

impl PortalHost {
    /// The face and lift of a portal hosted on a face.
    pub fn face(&self) -> Option<(&ae::GeoFaceRef, f32)> {
        match self {
            Self::Face { face, lift } => Some((face, *lift)),
            Self::Unattributed | Self::Static | Self::Line { .. } => None,
        }
    }

    /// Does the aperture ride something (a face or a line)?
    pub fn is_hosted(&self) -> bool {
        matches!(self, Self::Face { .. } | Self::Line { .. })
    }
}

impl PlacedPortal {
    /// A portal placed at a fixed pose, its opening `half_length` long each
    /// way along the face ([`PORTAL_OPENING_HALF`] is the standard) and the
    /// standard thickness across it. The host adapter then decides whether
    /// it rides a face.
    pub fn fixed(channel: PortalChannel, pos: Vec2, normal: Vec2, half_length: f32) -> Self {
        let half_thickness = PORTAL_THICKNESS_HALF;
        Self {
            channel,
            pos,
            normal,
            prev_normal: normal,
            half_length,
            half_thickness,
            partner_opening_half: None,
            host: PortalHost::Unattributed,
            vel: Vec2::ZERO,
            spin: 0.0,
            pivot: pos,
            prev_pos: pos,
        }
    }

    /// Place it at a new pose, as its owner does (not its host): the tick
    /// starts there, so eviction sees a new plane and transit no motion.
    pub fn place_at(&mut self, pos: Vec2, normal: Vec2) {
        self.pos = pos;
        self.normal = normal.normalize_or_zero();
        self.prev_pos = self.pos;
        self.prev_normal = self.normal;
        self.vel = Vec2::ZERO;
        self.spin = 0.0;
        self.pivot = pos;
    }

    /// Did its host move or turn it this tick?
    pub fn moved_this_tick(&self) -> bool {
        self.pos != self.prev_pos || self.normal != self.prev_normal
    }
}

impl PlacedPortal {
    /// The geometry frame this portal gives the portal map: origin, normal,
    /// and motion.
    pub fn frame(&self) -> PortalFrame {
        PortalFrame {
            origin: self.pos,
            normal: self.normal,
            velocity: self.vel,
            spin: self.spin,
            pivot: self.pivot,
        }
    }

    /// `v` (of a body at `p`) relative to this portal's surface there: what
    /// says whether the body moves into the portal, and what it keeps when it
    /// crosses. A still body that a moving portal sweeps onto moves into it.
    pub fn relative_velocity(&self, p: Vec2, v: Vec2) -> Vec2 {
        v - self.frame().velocity_at(p)
    }

    /// The opening this frame, turned to its face: the opening length along
    /// it and the thickness across it.
    pub fn opening_face(&self) -> ae::frame::FaceRect {
        ae::frame::FaceRect {
            center: self.pos,
            normal: self.normal.normalize_or_zero(),
            half_along: self.opening_half_length(),
            half_out: self.half_thickness,
        }
    }

    /// The frame at the start of this tick, before its host moved it. A
    /// point fixed to the aperture is at `frame().from_local(l)` where it was
    /// at `prev_frame().from_local(l)`.
    pub fn prev_frame(&self) -> PortalFrame {
        PortalFrame { origin: self.prev_pos, normal: self.prev_normal, ..self.frame() }
    }

    /// The half-length of the opening this frame: the authored one, made
    /// shorter (centered) to the partner's when the partner's is smaller, so
    /// the two ends of a pair are always one size. The thickness does not
    /// change, so transit stays an isometry.
    pub fn opening_half_length(&self) -> f32 {
        match self.partner_opening_half {
            Some(partner) => self.half_length.min(partner),
            None => self.half_length,
        }
    }

    /// The half-thickness of the opening across the face.
    pub fn opening_half_thickness(&self) -> f32 {
        self.half_thickness
    }

    /// The axis-aligned box that bounds the opening this frame (the opening
    /// itself, for a floor, wall or ceiling portal). For a reader that wants
    /// a box; the turned opening is [`Self::aperture`].
    pub fn opening_half_extent(&self) -> Vec2 {
        portal_box(self.normal, self.opening_half_length(), self.half_thickness)
    }

    /// Turn the portal to face `normal` (its host turned it): its opening
    /// keeps its length and thickness.
    pub fn turn_to(&mut self, normal: Vec2) {
        self.normal = normal.normalize_or_zero();
    }

    /// Frame and opening extent, used by the piece decomposition, straddle
    /// test, carve, and portal-aware casts.
    pub fn aperture(&self) -> PortalAperture {
        PortalAperture { frame: self.frame(), half_length: self.opening_half_length() }
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
pub const PORTAL_OPENING_HALF: f32 = 46.0;
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

/// The box `(x, y)` half-extents of an opening of `half_length` along the
/// face of `normal` and `half_thickness` across it: the box that bounds it.
pub fn portal_box(normal: Vec2, half_length: f32, half_thickness: f32) -> Vec2 {
    let n = normal.normalize_or_zero();
    let along = Vec2::new(-n.y, n.x);
    along.abs() * half_length + n.abs() * half_thickness
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
            prev_normal: Vec2::new(0.0, 1.0),
            half_length: PORTAL_OPENING_HALF,
            half_thickness: PORTAL_THICKNESS_HALF,
            partner_opening_half: None,
            host: Default::default(),
            vel: Vec2::ZERO,
            spin: 0.0,
            pivot: Vec2::ZERO,
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
            prev_normal: Vec2::new(0.0, 1.0),
            half_length: PORTAL_OPENING_HALF,
            half_thickness: PORTAL_THICKNESS_HALF,
            partner_opening_half: None,
            host: Default::default(),
            vel: Vec2::ZERO,
            spin: 0.0,
            pivot: Vec2::ZERO,
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

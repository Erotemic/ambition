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
    // ⛔⛔ THIS WAS `.find(..)` — THE FIRST MATCH IN ITERATION ORDER, AND ITS
    // CALLERS FEED IT A BEVY QUERY. `portal_list` in
    // `ambition_platformer2d_host::portal` is `Query<&PlacedPortal>` collected
    // into a `Vec`, so "first" meant ARCHETYPE ORDER: not a promise, and not
    // reproduced by a rollback resimulation.
    //
    // ⛔⛔ AND MY FIRST VERSION OF THIS COMMENT WAS WRONG, which is worth keeping
    // rather than quietly deleting. It said the tie was REACHABLE IN SHIPPED
    // CONTENT because `sandbox.ldtk`'s `portal_lab` authors SEVEN `purple`
    // apertures against one `yellow`. That is true of the AUTHORED DATA and
    // false at RUNTIME: all 14 of that level's portals also carry a link id, and
    // `resolve_portal_links` — which runs FIRST in the sim chain, before transit,
    // carve and eviction — REASSIGNS every one of them to a distinct
    // `Indexed(base + slot)`. The authored colour is a placeholder that never
    // survives to a lookup.
    //
    // ⇒ So this is PREVENTIVE, and the honest reason to keep it is that the
    // guarantee lives in another system: nothing here requires channels to be
    // unique, and a portal that reaches a lookup WITHOUT having been link-
    // resolved (a fixture, a future authoring road, a host that reorders the
    // chain) would land back on archetype order. A total order costs nothing and
    // does not depend on a promise made three systems away.
    //
    // ⭐ LOWEST POSITION WINS, which is arbitrary but REPRODUCIBLE: placements
    // are authored, so the order is identical on every run and every machine.
    // `total_cmp` rather than `partial_cmp` so there is no `unwrap` and no NaN
    // ordering hole.
    //
    // ⚠ THIS SETTLES DETERMINISM, NOT DESIGN. Which purple the yellow SHOULD
    // lead to is an authoring question (awaiting-maintainer-decision #65); what
    // this fixes is that the answer no longer changes between two runs of the
    // same content. The projectile sweep learned the same lesson first — see
    // `projectile/systems.rs`, "NEAREST FIRST, AND IT USED TO BE QUERY ORDER".
    portals
        .into_iter()
        .filter(|p| p.channel == channel)
        .min_by(|a, b| stable_portal_order(a, b))
        .cloned()
}

/// The crate's ONE tie-break between portals, for every place that has to pick
/// among several and must pick the same one twice.
///
/// ⛔⛔ THERE WERE THREE FIRST-MATCH SITES AND NO SHARED RULE.
/// [`find_portal`] took the first row of a collected `Query`;
/// `link::equalize_pair_apertures` ran its own `.find()` over another snapshot of
/// the same query; and `transit::portal_teleport_ground_items` loops the
/// collected portals and `break`s on the first one a moving item is entering.
/// **Archetype order in all three**, which is not a promise and is not
/// reproduced by a rollback resimulation — so a replayed frame could send a
/// thrown item through a DIFFERENT aperture.
///
/// ⭐ LOWEST POSITION, and the point is that it is ONE rule rather than which
/// rule it is. Placements are authored, so the order is identical on every run
/// and every machine, and three sites that each invented their own stable rule
/// could still disagree with each other — which is how a doorway gets sized
/// against one aperture while the body warps to another.
///
/// ⚠ `total_cmp` rather than `partial_cmp`: no `unwrap`, and no NaN hole.
pub fn stable_portal_order(a: &PlacedPortal, b: &PlacedPortal) -> std::cmp::Ordering {
    a.pos
        .x
        .total_cmp(&b.pos.x)
        .then_with(|| a.pos.y.total_cmp(&b.pos.y))
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

/// Consumed by the transit rescue and the carve so a portal on a THIN wall never grabs or engages a
/// body standing in the open room BEHIND that wall: the aperture volume ends where the wall does. A
/// channel with no entry reads as unmeasured (`f32::INFINITY` = unclipped), which callers bound by
/// [`crate::pieces::CARVE_DEPTH`].
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

    /// ⚠ A CONTRIVED TIE, DELIBERATELY, and the comment on `find_portal` says why
    /// the shipped one I first cited is not real: `portal_lab`'s seven authored
    /// `purple` apertures are all link-resolved to distinct channels before any
    /// lookup sees them. The property is still worth pinning — nothing in this
    /// function requires channels to be unique.
    ///
    /// ⭐ THE PROPERTY IS ORDER-INDEPENDENCE, so the test states it the only way
    /// that means anything: the SAME set in a DIFFERENT order must answer the
    /// same. A test that fed one order and asserted one answer would pass on the
    /// `.find()` this replaced.
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

    /// ⚠ The control: a channel with ONE aperture is unaffected, and a channel
    /// with none still answers `None`. Without these the test above would pass
    /// on a function that returned the same portal for everything.
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

    /// ⛔⛔ THE SAME PORTALS IN ANY ORDER SORT THE SAME WAY.
    ///
    /// Three sites collect portals off a `Query` -- archetype order, which a
    /// rollback resimulation does not reproduce -- and each then picks a WINNER:
    /// the body transit path breaks on its first match, the item path breaks on
    /// its own, and `find_portal` takes a minimum. Sorting at the collection
    /// point is what makes all three agree with each other AND with themselves
    /// across a replay.
    ///
    /// ⚠ The scene is deliberately not pre-sorted in any input order, and is
    /// checked from three different starting permutations: a comparator that
    /// returned `Equal` for everything would leave each input unchanged and pass
    /// a test that only reversed once.
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
        // ⚠ And it is a real ordering, not the identity: the leftmost is first,
        // and the two sharing an x are split by y.
        assert_eq!(forward[0].pos, Vec2::new(-40.0, 0.0));
        assert_eq!(forward[2].pos, Vec2::new(300.0, 5.0));
        assert_eq!(forward[3].pos, Vec2::new(300.0, 10.0));
    }
}

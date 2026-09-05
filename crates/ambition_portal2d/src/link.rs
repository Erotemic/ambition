//! Explicit portal linking by id, plus the min-aperture equalizer.
//!
//! Authoring portals by complementary color (purple↔yellow) is implicit. The
//! preferred model is an explicit shared link id: two portals carrying the
//! same [`PortalLink`] are partners. [`resolve_portal_links`] turns that into
//! the channel-based pairing the rest of the mechanic already uses — it assigns
//! each valid link group a pair of [`Indexed`](crate::PortalChannelColor::Indexed)
//! channels (partner = `^1`), distinguishing the two ends by position. A group
//! that is NOT exactly two members is closed: every member gets a slot-0
//! channel whose partner is absent, so it never carves and never transits — the
//! mis-linkage just reads as a dead portal.
//!
//! [`equalize_pair_apertures`] then enforces "the opening is the MINIMUM of the
//! linked pair, centered" — the aperture (and the drawn bar) of both ends
//! shrinks to the smaller authored length, so a mismatched pair opens a
//! consistent doorway in the middle with NO scaling (transit stays a pure
//! isometry; scaling portals are a deliberate future mechanic, not this).

use bevy::platform::collections::HashMap;
use bevy::prelude::*;

use crate::color::{PortalChannel, PortalChannelColor};
use crate::types::{portal_half_extent_with_length, portal_opening_half, PlacedPortal};

/// A portal authored with an explicit link id (the hash of the LDtk `link`
/// field). Two portals with the same id are a pair; the channel is DERIVED each
/// frame by [`resolve_portal_links`], so a link portal's [`PlacedPortal::channel`]
/// is provisional until then.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PortalLink(pub u64);

/// FNV-1a 64-bit hash of a link string — stable across runs (unlike
/// `DefaultHasher`), so the host can compute it at spawn and the channel
/// assignment is deterministic.
pub fn link_hash(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// Link channels live in the HIGH `Indexed` range (groups offset by
/// [`LINK_GROUP_BASE`]) to avoid colliding with hand-authored `cN` channels,
/// which authors use from 0 up.
const LINK_GROUP_BASE: u8 = 64;

/// Highest representable link group index: bases run `(64+gi)*2`, and index
/// 254/255 is reserved as the dead (never-paired) channel for refused groups.
const MAX_LINK_GROUPS: usize = 62;

/// Channel index for REFUSED link groups: its partner (254) is never
/// assigned, so a dead portal never carves and never transits.
const DEAD_LINK_CHANNEL: u8 = 255;

/// The set [`resolve_portal_links`] runs in.
///
/// Link resolution is the FIRST thing in `PortalSet::Transit`: it turns authored
/// link ids into channel pairs, and everything downstream in that set —
/// aperture equalisation, straddler eviction, transit itself — reads the result.
/// A host adapter that must publish portal frames before resolution therefore
/// needs a boundary INSIDE the set, which `PortalSet::Transit` cannot give it
/// (it is already in that set; pinning the parent would be a cycle).
///
/// ONE member. `equalize_pair_apertures` is chained immediately after and is
/// the obvious candidate to include, but the adapter's rule is specifically
/// "before links are resolved" — widening the set would silently also demand
/// "before apertures are equalised", a stronger claim nobody has made.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PortalLinkResolution;

/// Resolve [`PortalLink`] groups into channel pairs. Valid (exactly-two) groups
/// get partner-able `Indexed` channels distinguished by position; every other
/// group is closed (slot-0 channel with no partner).
pub fn resolve_portal_links(mut portals: Query<(&PortalLink, &mut PlacedPortal)>) {
    // Pass 1: collect each link group's member positions.
    let mut groups: HashMap<u64, Vec<Vec2>> = HashMap::default();
    for (link, p) in portals.iter() {
        groups.entry(link.0).or_default().push(p.pos);
    }
    if groups.is_empty() {
        return;
    }
    // Deterministic group index from the sorted hashes; member order from the
    // sorted positions (so each end's slot is stable).
    let mut hashes: Vec<u64> = groups.keys().copied().collect();
    hashes.sort_unstable();
    let group_index: HashMap<u64, usize> =
        hashes.iter().enumerate().map(|(i, h)| (*h, i)).collect();
    for members in groups.values_mut() {
        members.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
    }

    // Pass 2: assign each link portal its channel. Group indices above the
    // representable range are REFUSED (dead channel, never paired) rather
    // than clamped — a clamp would silently cross-link two unrelated groups.
    for (link, mut p) in portals.iter_mut() {
        let gi = group_index[&link.0];
        let members = &groups[&link.0];
        let channel = if gi > MAX_LINK_GROUPS {
            PortalChannel::Authored(PortalChannelColor::Indexed(DEAD_LINK_CHANNEL))
        } else {
            let base = (LINK_GROUP_BASE + gi as u8).wrapping_mul(2);
            // Exactly two members  slot by position; otherwise everyone slot
            // 0, which has no partner (slot 1 absent)  closed.
            let slot = if members.len() == 2 {
                members.iter().position(|m| *m == p.pos).unwrap_or(0) as u8
            } else {
                0
            };
            PortalChannel::Authored(PortalChannelColor::Indexed(base + slot))
        };
        if p.channel != channel {
            if gi > MAX_LINK_GROUPS {
                bevy::log::warn!(
                    target: "ambition_platformer2d::portal",
                    "portal link group {} exceeds the {} representable groups;                      refusing to link (dead channel) — reduce distinct link ids",
                    gi,
                    MAX_LINK_GROUPS + 1,
                );
            }
            p.channel = channel;
        }
    }
}

/// Shrink every linked pair's opening to the MINIMUM of the two authored
/// lengths, centered (the bar + aperture both follow). No scaling — the transit
/// map is untouched; only the doorway size changes. Runs after
/// [`resolve_portal_links`] so link channels are already paired.
pub fn equalize_pair_apertures(mut portals: Query<&mut PlacedPortal>) {
    // ⛔⛔ THE PARTNER IS CHOSEN BY `find_portal`, NOT BY A SECOND `.find()` HERE.
    // This used to snapshot `(channel, normal, half_extent)` and take the FIRST
    // row matching the partner channel — archetype order, because the snapshot
    // comes from a `Query`. That is the same defect `find_portal` had, and
    // TWO INDEPENDENT FIRST-MATCH RULES OVER ONE POPULATION IS WORSE THAN ONE:
    // wherever a channel is not unique, this could equalize a doorway against
    // one aperture while transit warped the body to a DIFFERENT one — a doorway
    // sized for a portal you do not arrive at. ⚠ PREVENTIVE: I first cited
    // `sandbox.ldtk`'s seven `purple` apertures as the live case and that was
    // WRONG — they are all link-resolved to distinct channels before any lookup
    // runs. See `find_portal`'s comment.
    //
    // ⭐ One reading of "which portal is the partner", so the two cannot
    // disagree. The snapshot is full portals now because that is what the shared
    // lookup takes; the clone is per-frame and small.
    let snapshot: Vec<PlacedPortal> = portals.iter().cloned().collect();
    for mut p in portals.iter_mut() {
        let partner = p.channel.partner();
        let Some(partner_portal) = crate::find_portal(&snapshot, partner) else {
            continue; // no partner placed — leave the authored opening as-is
        };
        let self_open = portal_opening_half(p.normal, p.half_extent);
        let partner_open =
            portal_opening_half(partner_portal.normal, partner_portal.half_extent);
        let min = self_open.min(partner_open);
        if (self_open - min).abs() > 1e-3 {
            p.half_extent = portal_half_extent_with_length(p.normal, min);
        }
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod aperture_partner_tests {
    use super::*;
    use crate::color::PortalChannelColor;
    use crate::types::PORTAL_THICKNESS_HALF;
    use bevy::prelude::App;

    fn portal(channel: PortalChannel, x: f32, opening: f32) -> PlacedPortal {
        PlacedPortal {
            channel,
            pos: Vec2::new(x, 0.0),
            normal: Vec2::new(0.0, 1.0),
            // Normal is +Y, so the OPENING runs along x.
            half_extent: Vec2::new(opening, PORTAL_THICKNESS_HALF),
            host: None,
            host_lift: 0.0,
            vel: Vec2::ZERO,
            prev_pos: Vec2::new(x, 0.0),
        }
    }

    fn equalized(order: &[PlacedPortal]) -> f32 {
        let mut app = App::new();
        for p in order {
            app.world_mut().spawn(p.clone());
        }
        app.add_systems(bevy::prelude::Update, equalize_pair_apertures);
        app.update();
        let yellow = PortalChannelColor::Yellow.channel();
        let mut q = app.world_mut().query::<&PlacedPortal>();
        q.iter(app.world())
            .find(|p| p.channel == yellow)
            .expect("the yellow aperture")
            .half_extent
            .x
    }

    /// ⛔⛔ TWO SAME-CHANNEL APERTURES. This pass used to run its OWN `.find()`
    /// over a snapshot of a `Query` — archetype order — while transit chose
    /// through `find_portal`. ⇒ Two independent first-match rules over one
    /// population, which could size a doorway against one aperture while warping
    /// the body to a DIFFERENT one. ⚠ Contrived, not shipped: the seven authored
    /// `purple` apertures I first cited are link-resolved to distinct channels
    /// before any lookup sees them.
    ///
    /// ⭐ The property is that the two agree AND that neither depends on spawn
    /// order, so the test spawns the same set twice in opposite orders and
    /// requires the same doorway. The purples have deliberately different
    /// openings, or every answer would look correct.
    #[test]
    fn the_doorway_is_sized_against_the_same_partner_transit_would_choose() {
        let purple = PortalChannelColor::Purple.channel();
        let yellow = PortalChannelColor::Yellow.channel();
        // Lowest `pos.x` wins, so the 10.0-wide one at x=100 is the partner and
        // the wide one at x=900 must NOT be.
        let near = portal(purple, 100.0, 10.0);
        let far = portal(purple, 900.0, 90.0);
        let gate = portal(yellow, 500.0, 50.0);

        let forward = equalized(&[near.clone(), far.clone(), gate.clone()]);
        let reversed = equalized(&[gate.clone(), far.clone(), near.clone()]);

        assert_eq!(
            forward, reversed,
            "spawn order changed the yellow doorway ({forward} vs {reversed}) — \
             the aperture pass is choosing a partner by iteration order"
        );
        assert_eq!(
            forward, 10.0,
            "the doorway was not sized against the partner `find_portal` \
             returns (lowest pos), so transit and the aperture disagree about \
             which purple is the pair"
        );
    }
}

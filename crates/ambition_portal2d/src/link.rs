//! Explicit portal linking by id, plus the min-aperture equalizer.
//!
//! Two portals with the same [`PortalLink`] are partners. This is preferred
//! over pairing by complementary color. [`resolve_portal_links`] gives each
//! valid link group a pair of [`Indexed`](crate::PortalChannelColor::Indexed)
//! channels (partner = `^1`), with the ends ordered by position. A group
//! without exactly two members is closed: its portals get a channel with no
//! partner, so they never carve or transit.
//!
//! [`equalize_pair_apertures`] then shrinks both openings (and drawn bars) of
//! a pair to the smaller authored length, centered. There is no scaling, so
//! transit stays an isometry.

use bevy::platform::collections::HashMap;
use bevy::prelude::*;

use crate::color::{PortalChannel, PortalChannelColor};
use crate::types::{portal_half_extent_with_length, portal_opening_half, PlacedPortal};

/// A portal authored with an explicit link id (the hash of the LDtk `link`
/// field). Two portals with the same id are a pair. [`resolve_portal_links`]
/// derives the channel each frame, so [`PlacedPortal::channel`] is provisional
/// until then.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PortalLink(pub u64);

/// FNV-1a 64-bit hash of a link string. Stable across runs (unlike
/// `DefaultHasher`), so channel assignment is deterministic.
pub fn link_hash(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// Link channels use the high `Indexed` range (offset by [`LINK_GROUP_BASE`]),
/// so they do not collide with hand-authored `cN` channels, which start at 0.
const LINK_GROUP_BASE: u8 = 64;

/// Highest representable link group index: bases run `(64+gi)*2`, and index
/// 254/255 is reserved as the dead (never-paired) channel for refused groups.
const MAX_LINK_GROUPS: usize = 62;

/// Channel index for refused link groups. Its partner (254) is never
/// assigned, so a dead portal never carves or transits.
const DEAD_LINK_CHANNEL: u8 = 255;

/// The set [`resolve_portal_links`] runs in.
///
/// Link resolution runs first in `PortalSet::Frame`, and everything after it
/// reads the result. A host adapter that must publish portal frames before
/// resolution needs this boundary inside `PortalSet::Frame`.
///
/// It has one member. Adding `equalize_pair_apertures` would also force
/// adapters to run before equalization, which no one requires.
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

    // Pass 2: assign each link portal its channel. Group indices out of range
    // are refused (dead channel), not clamped: a clamp would link two
    // unrelated groups.
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

/// Shrink every linked pair's opening to the smaller of the two authored
/// lengths, centered. The transit map does not change. Runs after
/// [`resolve_portal_links`].
pub fn equalize_pair_apertures(mut portals: Query<&mut PlacedPortal>) {
    // Choose the partner with `find_portal`, the same rule transit uses, so
    // the doorway is sized for the portal the body arrives at.
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

    /// Two same-channel apertures (a contrived case). Equalization must pick
    /// the same partner as `find_portal`, whatever the spawn order. The test
    /// spawns the set in both orders. The purples have different openings, so
    /// a wrong pick is visible.
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

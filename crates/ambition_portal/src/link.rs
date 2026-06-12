//! Explicit portal **linking by id**, plus the min-aperture equalizer.
//!
//! Authoring portals by complementary color (purple↔yellow) is implicit. The
//! preferred model is an explicit shared **link id**: two portals carrying the
//! same [`PortalLink`] are partners. [`resolve_portal_links`] turns that into
//! the channel-based pairing the rest of the mechanic already uses — it assigns
//! each valid link group a pair of [`Indexed`](crate::PortalChannelColor::Indexed)
//! channels (partner = `^1`), distinguishing the two ends by position. A group
//! that is NOT exactly two members is **closed**: every member gets a slot-0
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

    // Pass 2: assign each link portal its channel.
    for (link, mut p) in portals.iter_mut() {
        let gi = (group_index[&link.0] as u8).min(63);
        let members = &groups[&link.0];
        let base = (LINK_GROUP_BASE + gi).wrapping_mul(2);
        // Exactly two members ⇒ slot by position; otherwise everyone slot 0,
        // which has no partner (slot 1 absent) ⇒ closed.
        let slot = if members.len() == 2 {
            members.iter().position(|m| *m == p.pos).unwrap_or(0) as u8
        } else {
            0
        };
        let channel = PortalChannel::Authored(PortalChannelColor::Indexed(base + slot));
        if p.channel != channel {
            p.channel = channel;
        }
    }
}

/// Shrink every linked pair's opening to the MINIMUM of the two authored
/// lengths, centered (the bar + aperture both follow). No scaling — the transit
/// map is untouched; only the doorway size changes. Runs after
/// [`resolve_portal_links`] so link channels are already paired.
pub fn equalize_pair_apertures(mut portals: Query<&mut PlacedPortal>) {
    // Snapshot (channel, normal, half_extent) so each portal can read its
    // partner's opening.
    let snapshot: Vec<(PortalChannel, Vec2, Vec2)> = portals
        .iter()
        .map(|p| (p.channel, p.normal, p.half_extent))
        .collect();
    for mut p in portals.iter_mut() {
        let partner = p.channel.partner();
        let Some((_, pn, phe)) = snapshot.iter().find(|(c, _, _)| *c == partner) else {
            continue; // no partner placed — leave the authored opening as-is
        };
        let self_open = portal_opening_half(p.normal, p.half_extent);
        let partner_open = portal_opening_half(*pn, *phe);
        let min = self_open.min(partner_open);
        if (self_open - min).abs() > 1e-3 {
            p.half_extent = portal_half_extent_with_length(p.normal, min);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::portal_half_extent;

    fn floor(pos: Vec2) -> PlacedPortal {
        PlacedPortal {
            channel: PortalChannel::Authored(PortalChannelColor::Indexed(0)),
            pos,
            normal: Vec2::new(0.0, -1.0),
            half_extent: portal_half_extent(Vec2::new(0.0, -1.0)),
        }
    }

    fn app() -> App {
        let mut app = App::new();
        app.add_systems(
            Update,
            (resolve_portal_links, equalize_pair_apertures).chain(),
        );
        app
    }

    #[test]
    fn same_link_pairs_by_position_distinct_partners() {
        let mut app = app();
        let a = app
            .world_mut()
            .spawn((
                PortalLink(link_hash("door")),
                floor(Vec2::new(100.0, 300.0)),
            ))
            .id();
        let b = app
            .world_mut()
            .spawn((
                PortalLink(link_hash("door")),
                floor(Vec2::new(500.0, 300.0)),
            ))
            .id();
        app.update();
        let ca = app.world().get::<PlacedPortal>(a).unwrap().channel;
        let cb = app.world().get::<PlacedPortal>(b).unwrap().channel;
        assert_ne!(ca, cb, "the two ends get distinct channels");
        assert_eq!(ca.partner(), cb, "and they are each other's partners");
        assert_eq!(cb.partner(), ca);
    }

    #[test]
    fn wrong_arity_link_is_closed() {
        let mut app = app();
        // THREE portals share a link → all closed (no member has a partner).
        let es: Vec<_> = [100.0, 300.0, 500.0]
            .iter()
            .map(|x| {
                app.world_mut()
                    .spawn((PortalLink(link_hash("trio")), floor(Vec2::new(*x, 300.0))))
                    .id()
            })
            .collect();
        // A lone portal on another link → also closed.
        let lone = app
            .world_mut()
            .spawn((
                PortalLink(link_hash("solo")),
                floor(Vec2::new(700.0, 300.0)),
            ))
            .id();
        app.update();
        let all: Vec<PlacedPortal> = {
            let mut q = app.world_mut().query::<&PlacedPortal>();
            q.iter(app.world()).copied().collect()
        };
        for e in es.iter().chain(std::iter::once(&lone)) {
            let c = app.world().get::<PlacedPortal>(*e).unwrap().channel;
            assert!(
                !all.iter().any(|p| p.channel == c.partner()),
                "closed: no partner exists for {c:?}"
            );
        }
    }

    #[test]
    fn aperture_shrinks_to_the_pair_minimum() {
        let mut app = app();
        let mut big = floor(Vec2::new(100.0, 300.0));
        big.half_extent = portal_half_extent_with_length(big.normal, 80.0); // wide
        let mut small = floor(Vec2::new(500.0, 300.0));
        small.half_extent = portal_half_extent_with_length(small.normal, 30.0); // narrow
        let a = app
            .world_mut()
            .spawn((PortalLink(link_hash("d")), big))
            .id();
        let b = app
            .world_mut()
            .spawn((PortalLink(link_hash("d")), small))
            .id();
        app.update();
        // Both ends now open the minimum (30), centered (pos unchanged).
        for e in [a, b] {
            let p = app.world().get::<PlacedPortal>(e).unwrap();
            assert!(
                (portal_opening_half(p.normal, p.half_extent) - 30.0).abs() < 1e-3,
                "opening shrank to min: {:?}",
                p.half_extent
            );
        }
    }
}

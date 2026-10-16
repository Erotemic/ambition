//! Explicit portal linking by id, plus the min-aperture equalizer.
//!
//! Two portals with the same [`PortalLink`] in one live room are partners.
//! This is preferred over pairing by complementary color.
//! [`resolve_portal_links`] gives each link group of a live room a pair of
//! [`Indexed`](crate::PortalChannelColor::Indexed) channels (partner = `^1`).
//! A group without exactly two members is closed: its portals get a channel
//! with no partner, so they never carve or transit.
//!
//! ⭐ A PAIR KEEPS ITS CHANNELS. A group that has a channel pair keeps it, and
//! each end keeps its end of the pair, while the group lives. A new group
//! takes the lowest free pair OF ITS ROOM. Transit, its cooldown and the
//! eviction of a straddling body know a portal by its channel, so a channel
//! that another room's pair, a new pair or a moved end could change was a
//! body that lost its portal in the middle of a crossing.
//!
//! [`equalize_pair_apertures`] derives the opening of each end from the two
//! authored openings (the smaller, centered). It writes
//! [`PlacedPortal::partner_opening_half`] and never the authored extent, so a
//! partner that grows again gives the wider opening back. There is no
//! scaling, so transit stays an isometry.

use std::collections::BTreeMap;

use bevy::prelude::*;

use crate::color::{PortalChannel, PortalChannelColor};
use crate::types::{stable_portal_order, PlacedPortal};

/// A portal authored with an explicit link id (the hash of the LDtk `link`
/// field). Two portals with the same id are a pair. [`resolve_portal_links`]
/// derives the channel, so [`PlacedPortal::channel`] is provisional until
/// then.
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

/// The first channel of the link range. A hand-authored `cN` channel stops
/// below it ([`PortalChannelColor::from_name`] refuses `c128` and up), so an
/// authored pair and a link pair can not have one channel.
pub const LINK_CHANNEL_FIRST: u8 = 128;

/// The highest base of a link pair. Bases are 128, 130, ..., 252: 63 pairs in
/// one live room. 254/255 is the dead channel.
const LINK_CHANNEL_LAST_BASE: u8 = 252;

/// Channel index for refused link groups. Its partner (254) is never
/// assigned, so a dead portal never carves or transits.
const DEAD_LINK_CHANNEL: u8 = 255;

/// The base of the link pair that `channel` is an end of, if it is one.
fn link_base(channel: PortalChannel) -> Option<u8> {
    match channel {
        PortalChannel::Authored(PortalChannelColor::Indexed(c))
            if (LINK_CHANNEL_FIRST..=LINK_CHANNEL_LAST_BASE + 1).contains(&c) =>
        {
            Some(c & !1)
        }
        _ => None,
    }
}

/// The channel to place a linked portal on before [`resolve_portal_links`]
/// gives it its pair: the dead channel, which has no partner, so the portal
/// takes nothing through until its pair is known.
pub const UNRESOLVED_LINK_CHANNEL: PortalChannel =
    PortalChannel::Authored(PortalChannelColor::Indexed(DEAD_LINK_CHANNEL));

fn link_channel(index: u8) -> PortalChannel {
    PortalChannel::Authored(PortalChannelColor::Indexed(index))
}

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

/// One portal of a link group.
struct Member {
    entity: Entity,
    portal: PlacedPortal,
}

/// Resolve [`PortalLink`] groups into channel pairs (see the module doc).
///
/// ⭐ A GROUP IS A LINK IN ONE LIVE ROOM (OW1). Two live rooms that author
/// one link id (two instances of a room, or two rooms that reuse an id) are
/// two groups, and each room has its own 63 pairs: a pair is found among the
/// portals of its own room only, so two rooms can use one channel.
pub fn resolve_portal_links(
    mut portals: Query<(Entity, &PortalLink, &mut PlacedPortal)>,
    live: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
) {
    // Each room's groups in link-hash order, so a new group takes the same
    // free pair on every run, whatever the query order.
    let mut rooms: BTreeMap<crate::PortalRoom, BTreeMap<u64, Vec<Member>>> = BTreeMap::new();
    for (entity, link, p) in portals.iter() {
        rooms
            .entry(live.of(entity))
            .or_default()
            .entry(link.0)
            .or_default()
            .push(Member { entity, portal: p.clone() });
    }
    let mut assigned: Vec<(Entity, PortalChannel)> = Vec::new();
    for groups in rooms.values_mut() {
        assign_room(groups, &mut assigned);
    }
    for (entity, channel) in assigned {
        let Ok((_, _, mut p)) = portals.get_mut(entity) else {
            continue;
        };
        if p.channel != channel {
            if channel == link_channel(DEAD_LINK_CHANNEL) {
                bevy::log::warn!(
                    target: "ambition_platformer2d::portal",
                    "a live room has more than {} portal link groups; the new group is refused \
                     (dead channel) and the groups that have a pair keep it",
                    usize::from((LINK_CHANNEL_LAST_BASE - LINK_CHANNEL_FIRST) / 2 + 1),
                );
            }
            p.channel = channel;
        }
    }
}

/// The channels of one live room's link groups.
fn assign_room(groups: &mut BTreeMap<u64, Vec<Member>>, assigned: &mut Vec<(Entity, PortalChannel)>) {
    for members in groups.values_mut() {
        members.sort_by(|a, b| stable_portal_order(&a.portal, &b.portal));
    }
    // 1. A group keeps the pair its ends have. When two groups claim one pair
    //    (it can not happen by this system), the first in hash order keeps it.
    let mut taken: Vec<u8> = Vec::new();
    let mut bases: BTreeMap<u64, u8> = BTreeMap::new();
    for (hash, members) in groups.iter() {
        let claim = members.iter().filter_map(|m| link_base(m.portal.channel)).min();
        if let Some(base) = claim.filter(|base| !taken.contains(base)) {
            taken.push(base);
            bases.insert(*hash, base);
        }
    }
    // 2. A group with no pair takes the lowest free one, in hash order.
    for hash in groups.keys() {
        if bases.contains_key(hash) {
            continue;
        }
        let free = (LINK_CHANNEL_FIRST..=LINK_CHANNEL_LAST_BASE)
            .step_by(2)
            .find(|base| !taken.contains(base));
        if let Some(base) = free {
            taken.push(base);
            bases.insert(*hash, base);
        }
    }
    // 3. The ends.
    for (hash, members) in groups.iter() {
        let Some(&base) = bases.get(hash) else {
            for m in members {
                assigned.push((m.entity, link_channel(DEAD_LINK_CHANNEL)));
            }
            continue;
        };
        if members.len() != 2 {
            // Closed: every member on the first end, which has no partner.
            for m in members {
                assigned.push((m.entity, link_channel(base)));
            }
            continue;
        }
        // An end keeps its end of the pair. Position decides only for an end
        // that has none yet (or when both say the same end), so two ends that
        // pass each other or meet keep their roles.
        let end = |m: &Member| match m.portal.channel {
            PortalChannel::Authored(PortalChannelColor::Indexed(c)) if link_base(m.portal.channel) == Some(base) => {
                Some(c & 1)
            }
            _ => None,
        };
        let ends = match (end(&members[0]), end(&members[1])) {
            (Some(a), Some(b)) if a != b => [a, b],
            (Some(a), _) => [a, a ^ 1],
            (None, Some(b)) => [b ^ 1, b],
            (None, None) => [0, 1],
        };
        for (m, end) in members.iter().zip(ends) {
            assigned.push((m.entity, link_channel(base + end)));
        }
    }
}

/// Derive each linked portal's opening this frame: the smaller of its own
/// authored opening and its partner's, centered. The transit map does not
/// change. Runs after [`resolve_portal_links`].
///
/// It reads only authored extents and writes only
/// [`PlacedPortal::partner_opening_half`], so the result does not depend on
/// which end it does first, and an end whose partner grows opens wide again.
pub fn equalize_pair_apertures(
    mut portals: Query<(Entity, &mut PlacedPortal)>,
    live: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
) {
    // Choose the partner with `find_portal` among the portals of the same
    // live room, the same rule transit uses, so the doorway is sized for the
    // portal the body arrives at.
    let by_room = crate::PortalsByRoom::collect(
        portals.iter().map(|(entity, portal)| (entity, portal)),
        &live,
    );
    for (entity, mut p) in portals.iter_mut() {
        let partner = crate::find_portal(by_room.in_room(live.of(entity)), p.channel.partner())
            .map(|partner| partner.half_length);
        if p.partner_opening_half != partner {
            p.partner_opening_half = partner;
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
            prev_normal: Vec2::new(0.0, 1.0),
            half_length: opening,
            half_thickness: PORTAL_THICKNESS_HALF,
            partner_opening_half: None,
            host: Default::default(),
            vel: Vec2::ZERO,
            spin: 0.0,
            pivot: Vec2::ZERO,
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
            .opening_half_extent()
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

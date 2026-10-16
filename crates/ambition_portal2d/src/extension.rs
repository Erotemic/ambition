//! The portal domain's extension request adapters: a module's HELD pair of
//! portals ([`HeldPortalPairPort`]), and its lines of light that are no
//! portal ([`LightLinePort`]).
//!
//! A module asks for a pair each tick it wants one. This adapter spawns the
//! two [`PlacedPortal`]s, moves them to the asked places, and despawns them
//! when the module stops asking or its body is gone. Each pair is one
//! [`PortalLink`] of its owner, so the portal core gives it a channel pair of
//! the owner's live room and keeps it while it is held. The module never
//! holds an entity. See the port card on `ambition_portal_port`.
//!
//! A line of light is a look: this adapter keeps this tick's lines of each
//! body as a view on the body ([`HeldLightLines`]), and the presentation
//! draws them. The simulation spawns nothing for a line.

use std::sync::Arc;

use ambition_extension_host::{ExtensionAppExt, ExtensionOutbox};
use ambition_extension_sdk::phases::BOSS_CONDUCT;
use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::lifecycle::{InRoomInstance, LiveRooms, SessionCommands, SpawnSessionScopedExt};
use ambition_platformer2d_shared_tangle::sim_id::SimId;
use ambition_portal_port::{HeldPortalPairPort, LightLinePort};
use bevy::prelude::*;

use crate::{link_hash, PlacedPortal, PortalChannel, PortalChannelColor, PortalLink, PortalLook};

/// Install the held pair and the lines of light in `boss_conduct`.
pub fn install_for_boss_conduct(app: &mut App) {
    app.install_extension_request::<HeldPortalPairPort, _>(BOSS_CONDUCT, "ambition_portal2d", lower_held_portal_pairs)
        .install_extension_request::<LightLinePort, _>(BOSS_CONDUCT, "ambition_portal2d", lower_light_lines);
}

/// The lines of light that a body's modules asked for this tick
/// ([`LightLinePort`]): a view, which the presentation draws.
///
/// Derived: written again each tick from the module's record, which is
/// rollback state, and no simulation system reads it.
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub struct HeldLightLines(pub Vec<HeldLightLine>);

#[derive(Clone, Debug, PartialEq)]
pub struct HeldLightLine {
    /// `provider::module/entry`: slots are per entry.
    pub entry: Arc<str>,
    pub slot: u32,
    /// Its middle.
    pub at: ae::Vec2,
    /// The side of the line that the room is on.
    pub normal: ae::Vec2,
    pub length: f32,
    /// The authored channel index whose colour it has.
    pub look: u8,
}

/// See the port card on `LightLinePort`: each body's lines are the lines its
/// entries asked for this tick, and a body that asked for none has none.
/// A second line in one slot of one entry takes the place of the first.
fn lower_light_lines(
    mut commands: Commands,
    mut outbox: ResMut<ExtensionOutbox>,
    mut holders: Query<(Entity, &mut HeldLightLines)>,
) {
    let mut wanted: Vec<(Entity, Vec<HeldLightLine>)> = Vec::new();
    for s in outbox.drain::<LightLinePort>(&BOSS_CONDUCT) {
        let line = HeldLightLine {
            entry: s.entry.clone(),
            slot: s.value.slot,
            at: ae::Vec2::from(s.value.at),
            normal: ae::Vec2::from(s.value.normal),
            length: s.value.length,
            look: s.value.look,
        };
        let lines = match wanted.iter().position(|(owner, _)| *owner == s.scope) {
            Some(index) => &mut wanted[index].1,
            None => {
                wanted.push((s.scope, Vec::new()));
                &mut wanted.last_mut().expect("just pushed").1
            }
        };
        match lines.iter_mut().find(|held| held.entry == line.entry && held.slot == line.slot) {
            Some(held) => *held = line,
            None => lines.push(line),
        }
    }
    for (owner, mut held) in &mut holders {
        match wanted.iter().position(|(o, _)| *o == owner) {
            Some(index) => {
                let (_, lines) = wanted.swap_remove(index);
                if held.0 != lines {
                    held.0 = lines;
                }
            }
            None => {
                commands.entity(owner).try_remove::<HeldLightLines>();
            }
        }
    }
    for (owner, lines) in wanted {
        commands.entity(owner).try_insert(HeldLightLines(lines));
    }
}

/// The pairs a body's modules hold: the two portal entities this adapter
/// spawned for each (entry, slot). The module never sees these entities.
///
/// Rollback state: a rewind must restore which pair each slot holds, or the
/// replay spawns a second pair. The entity references are mapped
/// (`map.portal.held_pairs`); the checksum reads the entries and slots, not
/// the allocator-local entities.
#[derive(Component, Clone, Debug, Default)]
pub struct HeldPortalPairs(pub Vec<HeldPortalPairRecord>);

#[derive(Clone, Debug)]
pub struct HeldPortalPairRecord {
    /// `provider::module/entry`: slots are per entry.
    pub entry: Arc<str>,
    pub slot: u32,
    /// The two ends, in the order the module gave them.
    pub ends: [Entity; 2],
}

impl bevy::ecs::entity::MapEntities for HeldPortalPairs {
    fn map_entities<M: bevy::ecs::entity::EntityMapper>(&mut self, mapper: &mut M) {
        for record in &mut self.0 {
            for end in &mut record.ends {
                *end = mapper.get_mapped(*end);
            }
        }
    }
}

impl ae::snapshot::SnapshotCursor for HeldPortalPairs {
    fn encode_cursor(&self, out: &mut Vec<u8>) {
        ae::snapshot::put_u32(out, self.0.len() as u32);
        for record in &self.0 {
            ae::snapshot::put_str(out, &record.entry);
            ae::snapshot::put_u32(out, record.slot);
        }
    }
}

/// On a portal this adapter spawned: the body that holds it. A portal whose
/// holder is gone, or no longer holds it, is despawned.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct HeldPortalOf(pub Entity);

impl bevy::ecs::entity::MapEntities for HeldPortalOf {
    fn map_entities<M: bevy::ecs::entity::EntityMapper>(&mut self, mapper: &mut M) {
        self.0 = mapper.get_mapped(self.0);
    }
}

/// The link of `owner`'s pair in `slot` of `entry`: its identity, from which
/// the portal core gives it a channel pair of its room.
pub fn held_pair_link(owner: &SimId, entry: &str, slot: u32) -> PortalLink {
    PortalLink(link_hash(&format!("held_pair/{owner}/{entry}/{slot}")))
}

/// See the port card on `HeldPortalPairPort`: release what was not asked for
/// this tick, make what is held what was asked, spawn what is asked for and
/// not held, and despawn a portal whose holder is gone.
///
/// Each request is the whole of its pair this tick: its places, normals,
/// openings and looks. A pair is held only while its entry asks for it, so
/// the pairs of an entry that is no longer admitted (a reload that removed
/// it) are released: nothing can ask for them again.
#[allow(clippy::type_complexity)]
fn lower_held_portal_pairs(
    mut commands: SessionCommands,
    mut outbox: ResMut<ExtensionOutbox>,
    live: LiveRooms,
    mut holders: Query<(Entity, &mut HeldPortalPairs)>,
    ids: Query<&SimId>,
    mut portals: Query<(Entity, &HeldPortalOf, &mut PlacedPortal, &mut PortalLook)>,
) {
    let submitted = outbox.drain::<HeldPortalPairPort>(&BOSS_CONDUCT);
    let asked = |owner: Entity, r: &HeldPortalPairRecord| {
        submitted.iter().any(|s| s.scope == owner && *s.entry == *r.entry && s.value.slot == r.slot)
    };
    for (owner, mut held) in &mut holders {
        if held.0.iter().all(|r| asked(owner, r)) {
            continue;
        }
        held.0.retain(|r| {
            let keep = asked(owner, r);
            if !keep {
                for end in r.ends {
                    if let Ok(mut e) = commands.get_entity(end) {
                        e.try_despawn();
                    }
                }
            }
            keep
        });
    }
    // ⛔ A portal whose holder is gone, or that its holder no longer holds:
    // a body that is despawned takes its record with it, and its portals
    // must not stay open.
    for (portal, of, ..) in &portals {
        let held = holders
            .get(of.0)
            .is_ok_and(|(_, held)| held.0.iter().any(|r| r.ends.contains(&portal)));
        if !held {
            commands.entity(portal).try_despawn();
        }
    }

    let Some(scope) = commands.spawn_scope() else {
        return;
    };
    let mut new: Vec<(Entity, Vec<HeldPortalPairRecord>)> = Vec::new();
    for s in &submitted {
        let pair = &s.value;
        let held = holders
            .get(s.scope)
            .ok()
            .and_then(|(_, h)| h.0.iter().find(|r| *r.entry == *s.entry && r.slot == pair.slot).cloned());
        if let Some(record) = held {
            // Held: each end is what was asked this tick, in place, normal,
            // opening and look. Written only where it changed, so an
            // unchanged pair is not marked changed.
            for (index, (end, asked)) in record.ends.iter().zip(&pair.ends).enumerate() {
                if let Ok((_, _, mut portal, mut look)) = portals.get_mut(*end) {
                    let at = ae::Vec2::from(asked.at);
                    let normal = ae::Vec2::from(asked.normal);
                    // A place or turn that is asked is a jump, not a host's
                    // motion: the tick starts there, so eviction sees it.
                    if portal.pos != at || portal.normal != normal {
                        portal.place_at(at, normal);
                    }
                    if portal.half_length != asked.half_length {
                        portal.half_length = asked.half_length;
                    }
                    let want = PortalLook(PortalChannel::Authored(PortalChannelColor::Indexed(pair.looks[index])));
                    if *look != want {
                        *look = want;
                    }
                }
            }
            continue;
        }
        if new.iter().any(|(o, records)| *o == s.scope && records.iter().any(|r| *r.entry == *s.entry && r.slot == pair.slot)) {
            continue;
        }
        // ⛔ SUBMITTED IS NOT APPLIED: a body with no simulation identity has
        // no stable link, and a body in no live room has no room to open in.
        let (Ok(owner_id), Some(room)) = (ids.get(s.scope), live.of(s.scope)) else {
            warn!(
                "extension entry {} asked for a held portal pair for {:?}, which has no simulation identity or live room; refused",
                s.entry, s.scope
            );
            continue;
        };
        let link = held_pair_link(owner_id, &s.entry, pair.slot);
        let ends = [0usize, 1].map(|index| {
            let end = &pair.ends[index];
            let normal = ae::Vec2::from(end.normal);
            commands
                .spawn_room_in_session(
                    scope,
                    (
                        PlacedPortal::fixed(
                            crate::UNRESOLVED_LINK_CHANNEL,
                            ae::Vec2::from(end.at),
                            normal,
                            end.half_length,
                        ),
                        link,
                        PortalLook(PortalChannel::Authored(PortalChannelColor::Indexed(pair.looks[index]))),
                        HeldPortalOf(s.scope),
                        SimId::singleton("held_portal", &format!("{owner_id}/{}/{}/{index}", s.entry, pair.slot)),
                        Name::new(format!("held portal {index} of {} slot {}", s.entry, pair.slot)),
                    ),
                )
                .insert(InRoomInstance(room))
                .id()
        });
        let record = HeldPortalPairRecord { entry: s.entry.clone(), slot: pair.slot, ends };
        match new.iter_mut().find(|(o, _)| *o == s.scope) {
            Some((_, records)) => records.push(record),
            None => new.push((s.scope, vec![record])),
        }
    }
    for (owner, records) in new {
        if let Ok((_, mut held)) = holders.get_mut(owner) {
            held.0.extend(records);
        } else {
            commands.entity(owner).insert(HeldPortalPairs(records));
        }
    }
}

#[cfg(test)]
mod tests;

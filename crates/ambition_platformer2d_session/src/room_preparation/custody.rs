//! Occurrence custody that ends when a room operation commits.

use bevy::prelude::{Entity, Query, Res, With};

use crate::rooms;

/// Away occurrences whose rooms retire when the admitted roster commits.
/// Selected bodies and their custody survive. The destination plan receives
/// the same release decision that the room application will use.
#[derive(bevy::ecs::system::SystemParam)]
pub struct CustodyEndingAtCommit<'w, 's> {
    away: Option<Res<'w, ambition_platformer2d_shared_tangle::lifecycle::AwayFromAuthoredRoom>>,
    bodies: Query<
        'w,
        's,
        (
            Entity,
            &'static ambition_platformer2d_shared_tangle::sim_id::SimId,
            Option<&'static ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance>,
        ),
    >,
    roots: Query<'w, 's, (
        &'static ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance,
        Option<&'static ambition_platformer2d_world::rooms::LiveRoomDefinition>,
    ), With<ambition_platformer2d_shared_tangle::lifecycle::RoomInstanceRoot>>,
    drivers: Query<
        'w,
        's,
        (
            Entity,
            &'static ambition_characters::control::DrivingParticipant,
        ),
    >,
    custody: Query<'w, 's, (Entity, &'static ambition_platformer2d_shared_tangle::lifecycle::InCustodyOf)>,
    room_sets: Query<'w, 's, &'static ambition_platformer2d_world::rooms::RoomSet,
        With<ambition_platformer2d_shared_tangle::lifecycle::SessionRoot>>,
    live_bodies: ambition_platformer2d_shared_tangle::lifecycle::LiveBodies<'w, 's>,
}

impl CustodyEndingAtCommit<'_, '_> {
    /// The live rooms `intent` changes: each transferring member's room and
    /// the rooms of the bodies its custody carries along, the room it
    /// departs, a live room it joins, and each room it retires. A source
    /// another seat keeps is in it: it loses a member. The same plan
    /// preparation and publication use, so networking keeps no second room
    /// list (ISL-3a).
    pub fn footprint(
        &self,
        intent: &crate::lifecycle_commit::LifecycleIntent,
        restart: bool,
    ) -> super::TransferFootprint {
        let stamp = |entity| {
            self.bodies
                .get(entity)
                .ok()
                .and_then(|(_, _, stamp)| stamp)
                .map(|stamp| stamp.0)
        };
        let members: Vec<_> = intent.transfers().map(|transfer|
            (self.live_bodies.entity_of(&transfer.subject), transfer.participant)).collect();
        let plan = rooms::room_transfer_plan(
            members.iter().map(|(entity, slot)| (*entity, *slot, entity.and_then(stamp))),
            self.drivers.iter().map(|(entity, driver)| (entity, driver.0, stamp(entity))),
            self.roots.iter().map(|(live, definition)| (*live, definition.copied())),
            self.room_sets.single().ok().and_then(|rooms| rooms.definition_by_id(intent.target_room())),
            intent.rebuilds_source_room(), restart,
        );
        let custody: Vec<_> = self.custody.iter().map(|(entity, custody)| (entity, custody.custodian)).collect();
        let moving = ambition_platformer2d_shared_tangle::lifecycle::custody_closure(
            members.iter().filter_map(|(entity, _)| *entity), &custody);
        // Where the operation was decided: its members' rooms, or for a
        // rebuild nobody takes part in, the room it rebuilds.
        let mut sources: std::collections::BTreeSet<_> =
            members.iter().filter_map(|(entity, _)| entity.and_then(stamp)).collect();
        if members.is_empty() {
            sources.extend(plan.departing);
        }
        let mut rooms: std::collections::BTreeSet<_> = moving.iter().filter_map(|entity| stamp(*entity)).collect();
        rooms.extend(sources.iter().copied());
        rooms.extend(plan.departing);
        rooms.extend(plan.joins);
        rooms.extend(plan.retiring);
        super::TransferFootprint { sources, rooms }
    }

    /// Away occurrences left in source rooms that the selected roster releases.
    pub fn released_by(
        &self,
        intent: &crate::lifecycle_commit::LifecycleIntent,
        restart: bool,
    ) -> std::collections::BTreeSet<ambition_platformer2d_shared_tangle::sim_id::SimId> {
        let mut released = std::collections::BTreeSet::new();
        let Some(away) = self.away.as_deref().filter(|away| !away.0.is_empty()) else {
            return released;
        };
        let rebuilds_source = intent.rebuilds_source_room();
        let target_room = intent.target_room();
        let stamp = |entity| {
            self.bodies
                .get(entity)
                .ok()
                .and_then(|(_, _, stamp)| stamp)
                .map(|stamp| stamp.0)
        };
        let members: Vec<_> = intent.transfers().map(|transfer|
            (self.live_bodies.entity_of(&transfer.subject), transfer.participant)).collect();
        let room_plan = rooms::room_transfer_plan(
            members.iter().map(|(entity, slot)| (*entity, *slot, entity.and_then(stamp))),
            self.drivers.iter().map(|(entity, driver)| (entity, driver.0, stamp(entity))),
            self.roots.iter().map(|(live, definition)| (*live, definition.copied())),
            self.room_sets.single().ok().and_then(|rooms| rooms.definition_by_id(target_room)),
            rebuilds_source, restart,
        );
        let retiring = room_plan.retiring;
        let custody: Vec<_> = self.custody.iter().map(|(entity, custody)| (entity, custody.custodian)).collect();
        let moving = ambition_platformer2d_shared_tangle::lifecycle::custody_closure(
            members.iter().filter_map(|(entity, _)| *entity).chain(
                self.drivers.iter().filter(|(entity, _)| rebuilds_source && stamp(*entity).is_some_and(|room| retiring.contains(&room)))
                    .map(|(entity, _)| entity)), &custody);
        for (entity, sim_id, room) in &self.bodies {
            if !moving.contains(&entity) && away.0.contains(sim_id)
                && room.is_some_and(|room| retiring.contains(&room.0)) {
                released.insert(sim_id.clone());
            }
        }
        released
    }

    /// The away occurrences that live on after `intent` commits: every one
    /// that [`Self::released_by`] does not release. A construction that
    /// rebuilds a room from an older ledger (a checkpoint reset) holds these
    /// as carried, because the rooms they live in are not rewound.
    pub fn surviving(
        &self,
        intent: &crate::lifecycle_commit::LifecycleIntent,
        restart: bool,
    ) -> std::collections::BTreeSet<ambition_platformer2d_shared_tangle::sim_id::SimId> {
        let Some(away) = self.away.as_deref().filter(|away| !away.0.is_empty()) else {
            return std::collections::BTreeSet::new();
        };
        let released = self.released_by(intent, restart);
        away.0.difference(&released).cloned().collect()
    }
}

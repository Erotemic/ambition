//! Where an authored body is when it is not in the room that authored it
//! (Q38, OW3): a persistent character's durable whereabouts, and the
//! population occurrences that live away from home.
//!
//! The ruling: a persistent open-world character can be carried anywhere and
//! stays there. Its authored room is not a tether. So the authored home, the
//! durable whereabouts and the live occurrence are three facts, and this module
//! writes the second one for bodies, as `record_placed_ground_items` writes it
//! for objects.
//!
//! A body enters the ledger the way an object does: through custody. A
//! possessed or carried body has an `InCustody` row (the custody projection
//! writes it). When the custody ends, this producer writes `Placed { room, at }`
//! for the live room the body is in. From then on the room that authored it
//! does not build it (the outlook suppresses it there), and the room it lies in
//! builds it at `at` (the outlook reinstates it there). `at` is the body's
//! FEET point, because a placement record is placed by its footprint's bottom
//! (`actor_spawn_center_for_collision`), and the feet do not depend on the
//! size the character is built at.
//!
//! Only a persistent character that a room can build again somewhere else is
//! recorded: a `DeadStaysDead` NPC placement (an NPC placement always is,
//! `NPC_PLACEMENT_RESPAWN`). A respawning population occurrence stays where it
//! is carried while it lives and comes back from its authored room when its
//! room retires, which is what having no row does. A persistent enemy has no
//! row yet either, so it does the same.

use bevy::prelude::*;

/// Does this body keep durable whereabouts: a `DeadStaysDead` body that a
/// room can build again somewhere else? Today that is an authored PLACEMENT
/// record (an NPC), which the room it lies in plans through its home room's
/// lowering.
///
/// One rule for both producers here. A body that this says `true` for gets a
/// `Placed` row from [`record_placed_bodies`]; every other authored body that
/// lives away from home is held as carried by
/// [`record_bodies_away_from_home`]. A persistent enemy or boss has no row,
/// so it is held as carried like a population body while it lives, and its
/// home builds it when its room retires.
pub(crate) fn keeps_durable_whereabouts(
    room_set: &ambition_platformer2d_world::rooms::LiveRoomSpecs,
    config: &ambition_combat::actor_tuning::ActorConfig,
    origin: Option<&ambition_platformer2d_shared_tangle::construction::SpawnOrigin>,
) -> bool {
    if config.tuning.respawn != ambition_entity_catalog::placements::RespawnPolicy::DeadStaysDead {
        return false;
    }
    match origin {
        Some(ambition_platformer2d_shared_tangle::construction::SpawnOrigin::Authored { source, instance }) => room_set
            .rooms()
            .rooms
            .iter()
            .find(|home| &home.id == source)
            .is_some_and(|home| home.placements.iter().any(|record| record.id.as_str() == instance)),
        _ => false,
    }
}

/// A body's row is written again only when it changes rooms or moves this far
/// from the recorded point. A roaming character would otherwise rewrite the
/// ledger (and the save rows that mirror it) on every tick it walks. The live
/// body is the authority while its room is live; the row is what is left when
/// the room retires, so a lag under this distance is the price.
pub const WHEREABOUTS_REFRESH_DISTANCE: f32 = 32.0;

/// The live room a body is in and its feet point, for a persistent authored
/// body that came to rest after custody, by room id.
///
/// The rule for "came to rest here" is the item producer's: the row is
/// `InCustody` (a hand or a seat just let go of it), or `Placed` in the room
/// it is in now (it is being republished where it already is). A `Placed` row
/// naming another room is a stale duplicate, and a body with no row was never
/// moved, so its authored record is the whole story.
#[allow(clippy::type_complexity)]
pub fn record_placed_bodies(
    room_set: Option<ambition_platformer2d_world::rooms::LiveRoomSpecs>,
    bodies: Query<
        (
            Entity,
            &ambition_platformer2d_shared_tangle::sim_id::SimId,
            &ambition_platformer2d_core::body_clusters::BodyKinematics,
            &ambition_combat::actor_tuning::ActorConfig,
            Option<&ambition_platformer2d_shared_tangle::construction::SpawnOrigin>,
        ),
        (
            With<ambition_platformer2d_shared_tangle::lifecycle::RoomScopedEntity>,
            Without<ambition_platformer2d_shared_tangle::lifecycle::InCustodyOf>,
        ),
    >,
    // Required: the runtime that schedules this system also adds
    // `HeldItemSimulationPlugin`, which owns the ledger.
    mut occurrences: ResMut<ambition_platformer2d_shared_tangle::lifecycle::AuthoredOccurrences>,
) {
    use ambition_platformer2d_shared_tangle::lifecycle::OccurrenceWhereabouts;
    let Some(room_set) = room_set else {
        return;
    };
    // Nothing is in custody and nothing was ever placed: the common tick.
    if occurrences.is_empty() {
        return;
    }
    // BTreeMaps, not the query's order: this value reaches a construction plan.
    let mut rooms: std::collections::BTreeMap<
        String,
        std::collections::BTreeMap<ambition_platformer2d_shared_tangle::sim_id::SimId, ambition_platformer2d_core::Vec2>,
    > = std::collections::BTreeMap::new();
    for (entity, sim_id, kinematics, config, origin) in &bodies {
        if !keeps_durable_whereabouts(&room_set, config, origin) {
            continue;
        }
        let Some(definition) = room_set.definition_of(entity) else {
            continue;
        };
        let room = &room_set.rooms().spec(definition).id;
        let feet = ambition_platformer2d_core::Vec2::new(
            kinematics.pos.x,
            kinematics.pos.y + kinematics.size.y * 0.5,
        );
        let write = match occurrences.whereabouts(sim_id) {
            Some(OccurrenceWhereabouts::InCustody) => true,
            Some(OccurrenceWhereabouts::Placed { room: recorded, at }) => {
                recorded == room && at.distance(feet) > WHEREABOUTS_REFRESH_DISTANCE
            }
            None | Some(OccurrenceWhereabouts::Consumed | OccurrenceWhereabouts::Spent) => false,
        };
        // A row for a family no room can build elsewhere would suppress the
        // body at home and build it nowhere (`keeps_durable_whereabouts`).
        if write {
            rooms.entry(room.clone()).or_default().insert(sim_id.clone(), feet);
        }
    }
    for (room, placed) in rooms {
        let refused = occurrences.republish_placements(&room, placed);
        debug_assert!(
            refused.is_empty(),
            "the ledger refused bodies this producer should never have offered: {refused:?}"
        );
    }
}

/// The authored bodies with no durable whereabouts (the population, and a
/// persistent enemy) that live in a live room other than the room that
/// authored them, for the custody projection (Q38).
///
/// A respawning occurrence carried into another room and let go there has no
/// durable row (it is not a persistent character), so without this its home
/// room authored it again while it still lived in the other room: two bodies
/// of one identity, when another player holds that room. In the set, the
/// custody projection holds it as carried, so the home room does not author
/// it. When it dies or its room retires, it leaves the set, and the home
/// room authors the replacement.
///
/// A crossing's destination is prepared from the ledger as it stood before
/// the crossing was recorded, so an away body in the room the crossing retires
/// is released for that preparation by [`CustodyEndingAtCommit`], not here.
///
/// Written every tick from rollback state, before its one reader. A body in
/// custody is already carried, and a body with durable whereabouts
/// (`keeps_durable_whereabouts`) has a row instead. A persistent body with
/// no row (an enemy) is in the set.
#[allow(clippy::type_complexity)]
pub fn record_bodies_away_from_home(
    room_set: Option<ambition_platformer2d_world::rooms::LiveRoomSpecs>,
    bodies: Query<
        (
            Entity,
            &ambition_platformer2d_shared_tangle::sim_id::SimId,
            &ambition_combat::actor_tuning::ActorConfig,
            &ambition_platformer2d_shared_tangle::construction::SpawnOrigin,
        ),
        (
            With<ambition_platformer2d_shared_tangle::lifecycle::RoomScopedEntity>,
            Without<ambition_platformer2d_shared_tangle::lifecycle::InCustodyOf>,
        ),
    >,
    away: Option<ResMut<ambition_platformer2d_shared_tangle::lifecycle::AwayFromAuthoredRoom>>,
) {
    let Some(mut away) = away else {
        return;
    };
    let mut now = std::collections::BTreeSet::new();
    if let Some(room_set) = room_set {
        for (entity, sim_id, config, origin) in &bodies {
            // A body with durable whereabouts has a row instead.
            if keeps_durable_whereabouts(&room_set, config, Some(origin)) {
                continue;
            }
            let ambition_platformer2d_shared_tangle::construction::SpawnOrigin::Authored { source, .. } = origin else {
                continue;
            };
            let Some(definition) = room_set.definition_of(entity) else {
                continue;
            };
            if &room_set.rooms().spec(definition).id != source {
                now.insert(sim_id.clone());
            }
        }
    }
    // Compared first, so the reader's change detection stays quiet.
    if away.0 != now {
        away.0 = now;
    }
}

/// The away occurrences whose custody a pending crossing ends at its commit
/// (Q38).
///
/// A crossing's destination is prepared before its commit, from the ledger as
/// it stood before the crossing was recorded. The room the crossing leaves is
/// retired at the commit unless another player stays in it
/// ([`crate::rooms::another_player_stays`], the commit's own rule), and an
/// away population occurrence there dies with it. So the destination, when it
/// is the occurrence's home, must author its replacement: the ledger it is
/// prepared from has these custody rows released.
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
    roots: Query<
        'w,
        's,
        &'static ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance,
        With<ambition_platformer2d_shared_tangle::lifecycle::RoomInstanceRoot>,
    >,
    drivers: Query<'w, 's, (Entity, &'static ambition_characters::control::DrivingParticipant)>,
    live_bodies: ambition_platformer2d_shared_tangle::lifecycle::LiveBodies<'w, 's>,
}

impl CustodyEndingAtCommit<'_, '_> {
    /// The away occurrences in the live room `intent` retires. Empty when the
    /// intent is not a crossing, when another player stays in the room it
    /// leaves, or when nothing lives away from home.
    pub fn released_by(
        &self,
        intent: &crate::session::lifecycle_commit::LifecycleIntent,
    ) -> std::collections::BTreeSet<ambition_platformer2d_shared_tangle::sim_id::SimId> {
        let mut released = std::collections::BTreeSet::new();
        let Some(away) = self.away.as_deref().filter(|away| !away.0.is_empty()) else {
            return released;
        };
        let crate::session::lifecycle_commit::LifecycleIntent::Transition(intent) = intent else {
            return released;
        };
        let stamp = |entity| {
            self.bodies
                .get(entity)
                .ok()
                .and_then(|(_, _, stamp)| stamp)
                .map(|stamp| stamp.0)
        };
        let subject = self.live_bodies.entity_of(&intent.subject);
        // The commit's own departing room: the subject's, else the sole one.
        let Some(departing) = subject.and_then(stamp).or_else(|| self.roots.single().ok().copied()) else {
            return released;
        };
        if crate::rooms::another_player_stays(
            subject,
            intent.participant,
            departing,
            self.drivers.iter().map(|(entity, driver)| (entity, driver.0, stamp(entity))),
        ) {
            return released;
        }
        for (_, sim_id, room) in &self.bodies {
            if away.0.contains(sim_id) && room.map(|room| room.0) == Some(departing) {
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
        intent: &crate::session::lifecycle_commit::LifecycleIntent,
    ) -> std::collections::BTreeSet<ambition_platformer2d_shared_tangle::sim_id::SimId> {
        let Some(away) = self.away.as_deref().filter(|away| !away.0.is_empty()) else {
            return std::collections::BTreeSet::new();
        };
        let released = self.released_by(intent);
        away.0.difference(&released).cloned().collect()
    }
}

//! Where a persistent character was left: the durable whereabouts of an
//! authored body (Q38, OW3).
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
//! Only a persistent character is recorded: a body whose respawn policy is
//! `DeadStaysDead` (an NPC placement always is, `NPC_PLACEMENT_RESPAWN`). A
//! respawning population occurrence stays where it is carried while it lives
//! and comes back from its authored room when its room retires, which is what
//! having no row does.

use bevy::prelude::*;

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
    occurrences: Option<ResMut<ambition_platformer2d_shared_tangle::lifecycle::AuthoredOccurrences>>,
) {
    use ambition_platformer2d_shared_tangle::lifecycle::OccurrenceWhereabouts;
    let (Some(room_set), Some(mut occurrences)) = (room_set, occurrences) else {
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
        if config.tuning.respawn != ambition_entity_catalog::placements::RespawnPolicy::DeadStaysDead {
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
            None | Some(OccurrenceWhereabouts::Consumed) => false,
        };
        // Only a body a room can build again somewhere else gets a row: an
        // authored PLACEMENT record (an NPC), which the room it lies in plans
        // through its home room's lowering. A row for any other family would
        // suppress the body at home and build it nowhere, so a persistent
        // enemy or boss keeps going home until its family is reinstatable.
        let reinstatable = || match origin {
            Some(ambition_platformer2d_shared_tangle::construction::SpawnOrigin::Authored { source, instance }) => room_set
                .rooms()
                .rooms
                .iter()
                .find(|home| &home.id == source)
                .is_some_and(|home| home.placements.iter().any(|record| record.id.as_str() == instance)),
            _ => false,
        };
        if write && reinstatable() {
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

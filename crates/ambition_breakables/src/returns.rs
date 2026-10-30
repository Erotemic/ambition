//! Mirror live breakable state into the world ledgers: the due time of a
//! respawning breakable, and the `Spent` row of one broken for good.

use super::*;

/// Keep each live breakable's record in step with its live timer: a broken
/// breakable whose timer runs gets its due time when it has none, and a whole
/// breakable loses its record. Writes only on those two edges.
///
/// ⛔ NO CHANGE-TICK GATE (`Added<RespawnTimer>`) for the start edge. A
/// rollback load inserts the timer again, so a gate would open on a load
/// frame, and only on a peer that loads. The rule is on values: no record,
/// and a timer runs.
pub fn mirror_breakable_respawns(
    elapsed: SessionWorldRef<GameplayElapsed>,
    rooms: ambition_platformer2d_world::rooms::LiveRoomSpecs,
    breakables: Query<
        (Entity, &FeatureId, &BreakableFeature, Option<&RespawnTimer>),
        With<FeatureSimEntity>,
    >,
    participants: Query<(Entity, &DrivingParticipant)>,
    mut schedule: SessionWorldMut<WorldTimeSchedule>,
) {
    for (entity, feature, breakable, timer) in &breakables {
        let Some(definition) = rooms.definition_of(entity) else {
            continue;
        };
        let room = &rooms.rooms().spec(definition).id;
        match (breakable.broken(), timer) {
            (true, Some(timer)) => schedule.record(
                room,
                feature.as_str(),
                elapsed.0 + timer.0,
                &owners_beside(entity, rooms.live(), &participants),
            ),
            (false, _) if schedule.holds(room, feature.as_str()) => {
                schedule.forget(room, feature.as_str())
            }
            (false, _) => {}
            // Broken for good (`Never`, `OnRoomReload`): no respawn to schedule.
            (true, None) => {}
        }
    }
}

/// Remember each authored breakable broken under `HazardRespawn::Persistent`:
/// its row in the occurrence ledger becomes `Spent`, so each later build of
/// its room builds it broken (`spend_request`), and the save carries the row.
/// The participants in its room own the row (Q151), as they own an opened
/// chest.
///
/// A rule on values, like [`mirror_breakable_respawns`]: a broken persistent
/// breakable with no row gets one. Only an id with no row is written.
#[allow(clippy::type_complexity)]
pub fn record_spent_breakables(
    breakables: Query<
        (
            Entity,
            &BreakableFeature,
            &ambition_platformer2d_shared_tangle::sim_id::SimId,
            &ambition_platformer2d_shared_tangle::construction::SpawnOrigin,
        ),
        With<FeatureSimEntity>,
    >,
    rooms: Option<ambition_platformer2d_world::rooms::LiveRoomSpecs>,
    participants: Query<(Entity, &DrivingParticipant)>,
    occurrences: Option<ResMut<ambition_platformer2d_shared_tangle::lifecycle::AuthoredOccurrences>>,
    spent_since: Option<ResMut<ambition_platformer2d_items::pickups::ConsumedSinceCheckpoint>>,
) {
    let Some(mut occurrences) = occurrences else {
        return;
    };
    let spent: Vec<_> = breakables
        .iter()
        .filter(|(_, breakable, sim_id, origin)| {
            breakable.broken()
                && breakable.state().respawn
                    == ambition_entity_catalog::placements::HazardRespawn::Persistent
                && matches!(
                    origin,
                    ambition_platformer2d_shared_tangle::construction::SpawnOrigin::Authored { .. }
                )
                && occurrences.whereabouts(sim_id).is_none()
        })
        .map(|(entity, _, sim_id, _)| (entity, sim_id.clone()))
        .collect();
    // Written only on a new row, so an unchanged ledger is not marked changed.
    if spent.is_empty() {
        return;
    }
    if let (Some(rooms), Some(mut since)) = (rooms, spent_since) {
        for (entity, sim_id) in &spent {
            let Some(definition) = rooms.definition_of(*entity) else {
                continue;
            };
            since.record_spent(
                sim_id.clone(),
                rooms.rooms().spec(definition).id.clone(),
                owners_beside(*entity, rooms.live(), &participants),
            );
        }
    }
    occurrences.spend(spent.into_iter().map(|(_, sim_id)| sim_id));
}

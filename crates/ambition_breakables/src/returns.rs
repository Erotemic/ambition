//! Mirror live breakable timers into the world return ledger.

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

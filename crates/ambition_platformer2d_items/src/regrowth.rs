//! Regrow collected pickups and keep their admitted return times.

use super::pickups::*;
use ambition_characters::control::DrivingParticipant;
#[cfg(test)]
use ambition_characters::control::PlayerSlot;
use ambition_combat::components::FeatureId;
#[cfg(test)]
use ambition_platformer2d_shared_tangle::lifecycle::session_world_component;
use ambition_platformer2d_shared_tangle::lifecycle::{
    FeatureSimEntity, SessionWorldMut, SessionWorldRef,
};
use ambition_time::GameplayElapsed;
use ambition_world_returns::{RespawnTimer, WorldTimeSchedule, owners_beside};
use bevy::prelude::*;

/// A collected pickup authored `AfterSeconds(n)` counts its regrowth down on
/// its `RespawnTimer` (inserted at its collection, or at the rebuild of its
/// room while the regrowth is not yet due) and is collectable again when the
/// timer ends. The schedule mirrors the timer as [`mirror_breakable_respawns`]
/// mirrors a breakable's: a record when the timer runs and there is none, and
/// no record once the pickup is collectable.
///
/// ⛔ NO CHANGE-TICK GATE, for the reason the breakable mirror gives.
pub fn regrow_pickups(
    world_time: Res<ambition_time::WorldTime>,
    elapsed: SessionWorldRef<GameplayElapsed>,
    rooms: ambition_platformer2d_world::rooms::LiveRoomSpecs,
    mut pickups: Query<
        (
            Entity,
            &FeatureId,
            Has<Collected>,
            Option<&mut RespawnTimer>,
        ),
        (With<PickupFeature>, With<FeatureSimEntity>),
    >,
    participants: Query<(Entity, &DrivingParticipant)>,
    mut schedule: SessionWorldMut<WorldTimeSchedule>,
    mut commands: Commands,
) {
    // The sim clock, as a breakable's respawn: the regrowth freezes in
    // bullet time with everything else (ADR 0010).
    let dt = world_time.sim_dt();
    for (entity, feature, collected, timer) in &mut pickups {
        let Some(definition) = rooms.definition_of(entity) else {
            continue;
        };
        let room = &rooms.rooms().spec(definition).id;
        match (collected, timer) {
            (true, Some(mut timer)) => {
                timer.0 = (timer.0 - dt).max(0.0);
                if timer.0 <= 0.0 {
                    commands
                        .entity(entity)
                        .remove::<(Collected, RespawnTimer)>();
                    schedule.forget(room, feature.as_str());
                } else {
                    schedule.record(
                        room,
                        feature.as_str(),
                        elapsed.0 + timer.0,
                        &owners_beside(entity, rooms.live(), &participants),
                    );
                }
            }
            (false, _) if schedule.holds(room, feature.as_str()) => {
                schedule.forget(room, feature.as_str())
            }
            (false, _) => {}
            // Collected for good (`Never`, `OnRoomReload`): nothing to schedule.
            (true, None) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A pickup that is there asks the schedule each tick whether it has a
    /// record to forget. With none, the tick writes nothing: the schedule is
    /// not marked changed. A stale record (a rewind put the pickup back) is
    /// still forgotten on the next tick.
    #[test]
    fn a_tick_with_nothing_to_forget_leaves_the_schedule_unchanged() {
        use ambition_platformer2d_shared_tangle::lifecycle::{
            SessionRoot, SessionScopeId, require_on_session_root, session_world_component_mut,
        };
        use ambition_platformer2d_world::rooms::{RoomSet, RoomSpec};
        let dt = 1.0 / 60.0;
        let mut app = App::new();
        app.insert_resource(ambition_time::WorldTime::new(dt, dt));
        require_on_session_root::<GameplayElapsed>(&mut app);
        require_on_session_root::<WorldTimeSchedule>(&mut app);
        let room = RoomSpec::new(
            "hall",
            ambition_platformer2d_core::World::new(
                "hall",
                ambition_platformer2d_core::Vec2::new(320.0, 240.0),
                ambition_platformer2d_core::Vec2::new(16.0, 16.0),
                Vec::new(),
            ),
        );
        app.world_mut().spawn((
            SessionRoot(SessionScopeId(1)),
            RoomSet::from_parts_or_panic("hall", vec![room], Vec::new()),
        ));
        ambition_platformer2d_world::rooms::seat_sole_live_room_by_id(app.world_mut(), "hall")
            .expect("the hall is live");
        let live = *ambition_platformer2d_shared_tangle::lifecycle::sole_live_room_component::<
            ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance,
        >(app.world())
        .expect("a live room");
        app.world_mut().spawn((
            FeatureId::new("coin"),
            PickupFeature::new(ambition_interaction::Pickup::new(
                "coin",
                ambition_interaction::PickupKind::Currency { amount: 1 },
            )),
            FeatureSimEntity,
            ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance(live),
        ));
        #[derive(Resource, Default)]
        struct Changed(Vec<bool>);
        fn watch(schedule: SessionWorldRef<WorldTimeSchedule>, mut seen: ResMut<Changed>) {
            seen.0.push(schedule.is_changed());
        }
        app.init_resource::<Changed>()
            .add_systems(Update, (regrow_pickups, watch).chain());
        app.update();
        app.update();
        app.update();
        let seen = std::mem::take(&mut app.world_mut().resource_mut::<Changed>().0);
        assert_eq!(
            &seen[1..],
            &[false, false],
            "a tick with nothing to forget marked the schedule changed"
        );

        session_world_component_mut::<WorldTimeSchedule>(app.world_mut())
            .expect("a schedule")
            .record("hall", "coin", 5.0, &[PlayerSlot::PRIMARY]);
        app.update();
        let schedule =
            session_world_component::<WorldTimeSchedule>(app.world()).expect("a schedule");
        assert!(
            !schedule.holds("hall", "coin"),
            "the stale record of a pickup that is there was not forgotten"
        );
    }
}

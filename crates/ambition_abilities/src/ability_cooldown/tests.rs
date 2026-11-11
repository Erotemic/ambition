use super::*;
use ambition_platformer2d_shared_tangle::markers::{PlayerEntity, PrimaryPlayer};

/// The possessed body could blink exactly once, ever.
#[test]
fn a_possessed_body_cooldown_ticks_down_too() {
    let mut app = App::new();
    app.insert_resource(ambition_time::WorldTime::new(0.0, 0.1));
    app.add_systems(Update, tick_ability_cooldown);
    // No `PlayerEntity` / `PrimaryPlayer`: an ordinary actor body, driven.
    let possessed = app
        .world_mut()
        .spawn(AbilityCooldown { remaining: 0.5 })
        .id();

    for _ in 0..6 {
        app.update();
    }
    assert!(
        app.world()
            .get::<AbilityCooldown>(possessed)
            .unwrap()
            .ready(),
        "a possessed body's cooldown must expire like the home avatar's",
    );
}

#[test]
fn cooldown_ticks_down_to_ready() {
    let mut app = App::new();
    app.insert_resource(ambition_time::WorldTime::new(0.0, 0.1));
    app.add_systems(Update, tick_ability_cooldown);
    let player = app
        .world_mut()
        .spawn((
            PlayerEntity,
            PrimaryPlayer,
            AbilityCooldown { remaining: 0.5 },
        ))
        .id();

    app.update(); // 0.5 -> 0.4
    assert!(
        !app.world().get::<AbilityCooldown>(player).unwrap().ready(),
        "still cooling down after one tick",
    );
    for _ in 0..5 {
        app.update();
    }
    assert!(
        app.world().get::<AbilityCooldown>(player).unwrap().ready(),
        "ready again after enough ticks elapse",
    );
}

#[derive(Resource, Default)]
struct UseLog(Vec<bool>);

fn use_once_system(
    mut commands: Commands,
    mut players: Query<(Entity, Option<&mut AbilityCooldown>), With<PlayerEntity>>,
    mut log: ResMut<UseLog>,
) {
    if let Ok((player, mut cd)) = players.single_mut() {
        log.0
            .push(try_use_ability(&mut cd, &mut commands, player, 0.5));
    }
}

#[test]
fn first_use_inserts_and_allows_then_armed_use_is_blocked() {
    let mut app = App::new();
    app.init_resource::<UseLog>();
    app.add_systems(Update, use_once_system);
    let player = app.world_mut().spawn(PlayerEntity).id();

    app.update(); // None -> insert + allow
    assert!(
        app.world().get::<AbilityCooldown>(player).is_some(),
        "first use lazily inserts the cooldown component",
    );
    app.update(); // Some(armed) -> blocked (no tick system here)
    assert_eq!(
        app.world().resource::<UseLog>().0,
        vec![true, false],
        "first use allowed, second (still armed) blocked",
    );
}

/// ISL-1b: a cooldown runs at its body's room's pace: held while a connect
/// holds the room, and at full pace in the other room.
#[test]
fn a_cooldown_waits_while_its_room_is_held() {
    use ambition_combat::impact_hitstop::RoomPace;
    use ambition_platformer2d_shared_tangle::lifecycle::{InRoomInstance, LiveRoomInstance, RoomInstanceRoot};
    let (held, free) = (LiveRoomInstance::from_ordinal(1), LiveRoomInstance::from_ordinal(2));
    let mut app = App::new();
    app.insert_resource(ambition_time::WorldTime::new(0.0, 0.1));
    app.insert_resource(ambition_time::SimTick(10));
    app.world_mut().spawn((RoomInstanceRoot, held, RoomPace { held_until: 100, clock_scale: 1.0 }));
    app.world_mut().spawn((RoomInstanceRoot, free));
    let in_held = app.world_mut().spawn((AbilityCooldown { remaining: 0.5 }, InRoomInstance(held))).id();
    let in_free = app.world_mut().spawn((AbilityCooldown { remaining: 0.5 }, InRoomInstance(free))).id();
    app.add_systems(Update, tick_ability_cooldown);
    app.update();
    let left = |body| app.world().get::<AbilityCooldown>(body).unwrap().remaining;
    assert_eq!(left(in_held), 0.5, "a cooldown ran while its room was held");
    assert!((left(in_free) - 0.4).abs() < 1e-6, "a cooldown in the other room did not run: {}", left(in_free));
}

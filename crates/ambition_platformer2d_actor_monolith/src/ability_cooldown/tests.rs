//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

use super::*;
use crate::actor::{PlayerEntity, PrimaryPlayer};

/// The bug the S5/S6 fold found: `blink`/`grapple` act on the
/// `ControlledSubject`, so a POSSESSED actor arms this cooldown on itself —
/// and the old `With<PlayerEntity>, With<PrimaryPlayer>` tick filter never
/// counted it down. The possessed body could blink exactly once, ever.
#[test]
fn a_possessed_body_cooldown_ticks_down_too() {
    let mut app = App::new();
    app.insert_resource(ambition_time::WorldTime {
        scaled_dt: 0.1,
        ..Default::default()
    });
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
    app.insert_resource(ambition_time::WorldTime {
        scaled_dt: 0.1,
        ..Default::default()
    });
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

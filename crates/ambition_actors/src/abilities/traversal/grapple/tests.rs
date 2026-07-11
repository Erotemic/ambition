//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

use super::*;

fn world_with_right_wall() -> ambition_engine_core::RoomGeometry {
    // A solid wall at x[380,400], y[0,400]; open space to its left.
    let blocks = vec![ae::Block::solid(
        "wall",
        ae::Vec2::new(380.0, 0.0),
        ae::Vec2::new(20.0, 400.0),
    )];
    ambition_engine_core::RoomGeometry(ae::World::new(
        "grapple_test",
        ae::Vec2::new(400.0, 400.0),
        ae::Vec2::new(100.0, 200.0),
        blocks,
    ))
}

fn test_app(world: Option<ambition_engine_core::RoomGeometry>) -> App {
    let mut app = App::new();
    app.add_message::<ambition_sfx::SfxMessage>();
    app.add_message::<ambition_vfx::vfx::VfxMessage>();
    if let Some(w) = world {
        app.insert_resource(w);
    }
    app.add_systems(Update, grapple_system);
    app
}

fn spawn_player_holding(app: &mut App, id: &str, pos: ae::Vec2, facing: f32) -> Entity {
    crate::abilities::test_support::spawn_primary_player_holding_at(app, id, pos, facing)
}

fn player_vel(app: &App, player: Entity) -> ae::Vec2 {
    app.world().get::<BodyKinematics>(player).unwrap().vel
}

#[test]
fn grapple_yanks_the_player_toward_a_grappled_wall() {
    let mut app = test_app(Some(world_with_right_wall()));
    // Player to the left of the wall, facing/aiming right.
    let player = spawn_player_holding(&mut app, GRAPPLE_ID, ae::Vec2::new(100.0, 200.0), 1.0);
    app.world_mut()
        .get_mut::<ActorControl>(player)
        .unwrap()
        .0
        .melee_pressed = true;
    app.update();
    let vel = player_vel(&app, player);
    assert!(
        vel.x > 0.0,
        "the yank velocity points toward the wall (right)"
    );
    assert!(
        vel.x.abs() > vel.y.abs(),
        "a horizontal grapple yanks mostly horizontally ({vel:?})"
    );
    assert!(
        (vel.length() - GRAPPLE_PULL_SPEED).abs() < 1.0,
        "the yank is at the pull speed",
    );
}

#[test]
fn grapple_into_empty_space_does_not_move_the_player() {
    // No world (or no wall in range) → fizzle, velocity untouched.
    let mut app = test_app(None);
    let player = spawn_player_holding(&mut app, GRAPPLE_ID, ae::Vec2::new(100.0, 200.0), 1.0);
    app.world_mut()
        .get_mut::<ActorControl>(player)
        .unwrap()
        .0
        .melee_pressed = true;
    app.update();
    assert_eq!(
        player_vel(&app, player),
        ae::Vec2::ZERO,
        "a dry grapple yanks nothing"
    );
}

#[test]
fn no_grapple_without_attack_or_with_a_different_item() {
    // Holding grapple but not attacking → no pull.
    let mut app = test_app(Some(world_with_right_wall()));
    let player = spawn_player_holding(&mut app, GRAPPLE_ID, ae::Vec2::new(100.0, 200.0), 1.0);
    app.update();
    assert_eq!(player_vel(&app, player), ae::Vec2::ZERO);
    // Holding the bomb + attacking → grapple_system ignores it.
    let mut app2 = test_app(Some(world_with_right_wall()));
    let player2 = spawn_player_holding(&mut app2, "bomb", ae::Vec2::new(100.0, 200.0), 1.0);
    app2.world_mut()
        .get_mut::<ActorControl>(player2)
        .unwrap()
        .0
        .melee_pressed = true;
    app2.update();
    assert_eq!(player_vel(&app2, player2), ae::Vec2::ZERO);
}

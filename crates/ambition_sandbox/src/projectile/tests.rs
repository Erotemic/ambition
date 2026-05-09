use ambition_engine as ae;
use ambition_engine::{Block, MotionDirection, ProjectileKind, World};
use bevy::prelude::*;

use super::state::{PlayerProjectile, PlayerProjectileState};
use super::systems::update_projectiles;
use crate::audio::SfxMessage;
use crate::features::FeatureEventBus;
use crate::fx::VfxMessage;
use crate::input::ControlFrame;
use crate::physics::DebrisBurstMessage;
use crate::trace::GameplayTraceBuffer;
use crate::{GameWorld, SandboxRuntime};

fn dummy_world() -> World {
    World::new(
        "test",
        ae::Vec2::new(2000.0, 2000.0),
        ae::Vec2::new(200.0, 200.0),
        vec![Block::solid(
            "right wall",
            ae::Vec2::new(800.0, 100.0),
            ae::Vec2::new(40.0, 400.0),
        )],
    )
}

fn min_app() -> App {
    let mut app = App::new();
    app.insert_resource(Time::<()>::default());
    app.insert_resource(GameWorld(dummy_world()));
    let mut runtime = SandboxRuntime::new(
        &dummy_world(),
        ae::AbilitySet::sandbox_all(),
        ae::DEFAULT_TUNING,
        crate::physics::PhysicsSandboxSettings::default(),
    );
    runtime.player.pos = ae::Vec2::new(300.0, 300.0);
    runtime.player.facing = 1.0;
    app.insert_resource(runtime);
    app.insert_resource(ControlFrame::default());
    app.insert_resource(crate::settings::UserSettings::default());
    app.insert_resource(GameplayTraceBuffer::default());
    app.insert_resource(PlayerProjectileState::default());
    app.insert_resource(FeatureEventBus::default());
    // Buffered-message channels the system writes into.
    app.add_message::<SfxMessage>();
    app.add_message::<VfxMessage>();
    app.add_message::<DebrisBurstMessage>();
    app.add_systems(Update, update_projectiles);
    app
}

fn advance_time(app: &mut App, dt_seconds: f32) {
    let mut time = app.world_mut().resource_mut::<Time<()>>();
    time.advance_by(std::time::Duration::from_secs_f32(dt_seconds));
}

/// Helper: press the projectile button (no motion) and immediately
/// release it on the same press-release pair. Equivalent to a
/// "tap" in the new charge model — fires a tier-0 fireball.
fn tap_projectile(app: &mut App) {
    // Press frame: just_pressed=true, held=true (Bevy's button
    // semantics — pressed state lasts as long as held), released=false.
    // The system enters the press branch and starts charging.
    {
        let mut frame = app.world_mut().resource_mut::<ControlFrame>();
        frame.projectile_pressed = true;
        frame.projectile_held = true;
        frame.projectile_released = false;
    }
    advance_time(app, 0.016);
    app.update();
    // Release frame: just_pressed=false, held=false, released=true.
    {
        let mut frame = app.world_mut().resource_mut::<ControlFrame>();
        frame.projectile_pressed = false;
        frame.projectile_held = false;
        frame.projectile_released = true;
    }
    advance_time(app, 0.016);
    app.update();
    // Reset the edge for the next test step.
    {
        let mut frame = app.world_mut().resource_mut::<ControlFrame>();
        frame.projectile_released = false;
    }
}

#[test]
fn tap_release_fires_one_fireball() {
    let mut app = min_app();
    tap_projectile(&mut app);
    let state = app.world().resource::<PlayerProjectileState>();
    assert_eq!(state.bodies.len(), 1);
    assert_eq!(state.bodies[0].body.kind, ProjectileKind::Fireball);
    // Tap-release is below the medium threshold → tier 0.
    assert_eq!(state.bodies[0].body.kind, ProjectileKind::Fireball);
}

/// Pressing without releasing is "still charging" — no body
/// spawns yet, but `state.charging` is `Some`.
#[test]
fn press_without_release_only_starts_charge() {
    let mut app = min_app();
    {
        let mut frame = app.world_mut().resource_mut::<ControlFrame>();
        frame.projectile_pressed = true;
        frame.projectile_held = true;
    }
    advance_time(&mut app, 0.016);
    app.update();
    let state = app.world().resource::<PlayerProjectileState>();
    assert!(
        state.bodies.is_empty(),
        "press without release must not spawn a fireball"
    );
    assert!(
        state.charging.is_some(),
        "press without motion must start a charge"
    );
}

/// Holding past the medium-charge threshold and releasing fires a
/// fireball with tier 1 (visibly bigger half-extent + bumped damage).
#[test]
fn held_release_after_medium_threshold_fires_charged_fireball() {
    let mut app = min_app();
    // Press frame.
    {
        let mut frame = app.world_mut().resource_mut::<ControlFrame>();
        frame.projectile_pressed = true;
        frame.projectile_held = true;
    }
    advance_time(&mut app, 0.016);
    app.update();
    // Hold for several ticks — accumulate ~0.5s, well past the
    // 0.35s medium threshold but under the 0.85s heavy threshold.
    for _ in 0..30 {
        {
            let mut frame = app.world_mut().resource_mut::<ControlFrame>();
            frame.projectile_pressed = false;
            frame.projectile_held = true;
        }
        advance_time(&mut app, 0.016);
        app.update();
    }
    // Release.
    {
        let mut frame = app.world_mut().resource_mut::<ControlFrame>();
        frame.projectile_held = false;
        frame.projectile_released = true;
    }
    advance_time(&mut app, 0.016);
    app.update();
    let state = app.world().resource::<PlayerProjectileState>();
    assert_eq!(state.bodies.len(), 1);
    let body = &state.bodies[0].body;
    assert_eq!(body.kind, ProjectileKind::Fireball);
    // Tier-1 size scaling is 1.4x on baseline half-extent (12, 9)
    // → at least 16x12 — meaningfully bigger than tier 0.
    let baseline = ae::ProjectileKind::Fireball.half_extent();
    assert!(
        body.half_extent.x > baseline.x * 1.2,
        "charged fireball must be visibly larger; got {:?} vs baseline {:?}",
        body.half_extent,
        baseline
    );
    // Tier-1 damage scaling is 2x baseline (1) = 2.
    assert!(body.damage >= 2);
}

/// Grace QCF (Down → Right) + press fires a regular Hadouken
/// IMMEDIATELY (no charging needed). The motion press takes
/// precedence over the charge-start path.
#[test]
fn grace_qcf_then_press_fires_hadouken_immediately() {
    let mut app = min_app();
    {
        let mut state = app.world_mut().resource_mut::<PlayerProjectileState>();
        let mut t = 0.0;
        for dir in [MotionDirection::Down, MotionDirection::Right] {
            state.motion_buffer.push(dir, t);
            t += 0.04;
        }
        state.clock = t;
    }
    {
        let mut frame = app.world_mut().resource_mut::<ControlFrame>();
        frame.projectile_pressed = true;
        frame.projectile_held = true;
    }
    advance_time(&mut app, 0.016);
    app.update();
    let state = app.world().resource::<PlayerProjectileState>();
    assert_eq!(state.bodies.len(), 1);
    assert_eq!(state.bodies[0].body.kind, ProjectileKind::Hadouken);
    assert!(
        state.charging.is_none(),
        "motion-press must NOT start a charge"
    );
}

/// Full QCF (Down → DownRight → Right) + press fires the SUPER
/// variant. The Super gate is checked before the grace gate so a
/// 3-step input fires the stronger projectile, not a weak one.
#[test]
fn full_qcf_then_press_fires_hadouken_super() {
    let mut app = min_app();
    {
        let mut state = app.world_mut().resource_mut::<PlayerProjectileState>();
        let mut t = 0.0;
        for dir in [
            MotionDirection::Down,
            MotionDirection::DownRight,
            MotionDirection::Right,
        ] {
            state.motion_buffer.push(dir, t);
            t += 0.04;
        }
        state.clock = t;
    }
    {
        let mut frame = app.world_mut().resource_mut::<ControlFrame>();
        frame.projectile_pressed = true;
        frame.projectile_held = true;
    }
    advance_time(&mut app, 0.016);
    app.update();
    let state = app.world().resource::<PlayerProjectileState>();
    assert_eq!(state.bodies.len(), 1);
    assert_eq!(state.bodies[0].body.kind, ProjectileKind::HadoukenSuper);
}

/// Half-circle motion (the historic Hadouken trigger) keeps
/// firing the SUPER variant — pin behavior so users with that
/// muscle memory still get a Hadouken (just upgraded to the
/// stronger one). The 3-step QCF is the simpler new path.
#[test]
fn half_circle_still_fires_hadouken_super() {
    let mut app = min_app();
    {
        let mut state = app.world_mut().resource_mut::<PlayerProjectileState>();
        let mut t = 0.0;
        for dir in [
            MotionDirection::Right,
            MotionDirection::DownRight,
            MotionDirection::Down,
            MotionDirection::DownLeft,
            MotionDirection::Left,
        ] {
            state.motion_buffer.push(dir, t);
            t += 0.04;
        }
        state.clock = t;
    }
    {
        let mut frame = app.world_mut().resource_mut::<ControlFrame>();
        frame.projectile_pressed = true;
        frame.projectile_held = true;
    }
    advance_time(&mut app, 0.016);
    app.update();
    let state = app.world().resource::<PlayerProjectileState>();
    assert_eq!(state.bodies.len(), 1);
    assert_eq!(state.bodies[0].body.kind, ProjectileKind::HadoukenSuper);
}

#[test]
fn cooldown_blocks_second_fire_in_same_window() {
    let mut app = min_app();
    tap_projectile(&mut app);
    // Don't advance past the cooldown — second tap should be no-op.
    tap_projectile(&mut app);
    let state = app.world().resource::<PlayerProjectileState>();
    assert_eq!(state.bodies.len(), 1);
}

#[test]
fn out_of_resource_blocks_fire() {
    let mut app = min_app();
    {
        let mut state = app.world_mut().resource_mut::<PlayerProjectileState>();
        state.spawner.meter.current = 0.0;
    }
    tap_projectile(&mut app);
    let state = app.world().resource::<PlayerProjectileState>();
    assert!(state.bodies.is_empty());
}

/// Pre-spawn a fireball directly into the body list and place it
/// just above an enemy. After one tick the fireball moves into
/// the enemy's AABB → `apply_damage_event` does its thing → enemy
/// loses HP and the projectile is despawned.
#[test]
fn fireball_damages_enemy_on_intersect() {
    let mut app = min_app();
    // Spawn an enemy in the runtime.
    {
        let mut runtime = app.world_mut().resource_mut::<SandboxRuntime>();
        runtime.features.spawn_enemy(
            "test_enemy".into(),
            ae::EnemyBrain::Custom("medium_striker".into()),
            ae::Vec2::new(400.0, 300.0),
            ae::Vec2::new(28.0, 46.0),
        );
    }
    // Inject a fireball moving toward the enemy.
    {
        let mut state = app.world_mut().resource_mut::<PlayerProjectileState>();
        let spec = ae::ProjectileSpec::new(
            ae::ProjectileKind::Fireball,
            ae::Vec2::new(395.0, 300.0),
            ae::Vec2::new(1.0, 0.0),
            1.0,
        );
        let mut body = ae::ProjectileBody::from_spec(spec);
        // Override velocity / pos so the next tick definitely
        // overlaps the enemy AABB regardless of arc tuning.
        body.pos = ae::Vec2::new(395.0, 300.0);
        body.vel = ae::Vec2::new(50.0, 0.0);
        state.bodies.push(PlayerProjectile { body });
    }
    let starting_health = {
        let runtime = app.world().resource::<SandboxRuntime>();
        runtime.features.enemies[0].health.current
    };
    advance_time(&mut app, 0.016);
    app.update();
    let runtime = app.world().resource::<SandboxRuntime>();
    let state = app.world().resource::<PlayerProjectileState>();
    assert!(
        runtime.features.enemies[0].health.current < starting_health,
        "enemy must lose HP from a projectile hit (was {}, now {})",
        starting_health,
        runtime.features.enemies[0].health.current
    );
    assert!(
        state.bodies.is_empty(),
        "fireball must despawn after hitting an actor"
    );
}

/// Drop a fireball onto a floor block. The first tick should
/// produce a bounce (vy reflects upward, bounce budget drops by
/// one) and the projectile must remain in the body list.
#[test]
fn fireball_bounces_off_floor_in_system() {
    let mut app = App::new();
    app.insert_resource(Time::<()>::default());
    // World with a single floor block well below the spawn point.
    let world = ae::World::new(
        "bounce_test",
        ae::Vec2::new(2000.0, 2000.0),
        ae::Vec2::new(200.0, 200.0),
        vec![ae::Block::solid(
            "floor",
            ae::Vec2::new(0.0, 400.0),
            ae::Vec2::new(2000.0, 32.0),
        )],
    );
    app.insert_resource(GameWorld(world.clone()));
    let mut runtime = SandboxRuntime::new(
        &world,
        ae::AbilitySet::sandbox_all(),
        ae::DEFAULT_TUNING,
        crate::physics::PhysicsSandboxSettings::default(),
    );
    runtime.player.pos = ae::Vec2::new(200.0, 200.0);
    app.insert_resource(runtime);
    app.insert_resource(ControlFrame::default());
    app.insert_resource(crate::settings::UserSettings::default());
    app.insert_resource(GameplayTraceBuffer::default());
    app.insert_resource(PlayerProjectileState::default());
    app.insert_resource(FeatureEventBus::default());
    app.add_message::<SfxMessage>();
    app.add_message::<VfxMessage>();
    app.add_message::<DebrisBurstMessage>();
    app.add_systems(Update, update_projectiles);

    // Spawn a fireball just above the floor moving downward.
    let starting_bounces;
    {
        let mut state = app.world_mut().resource_mut::<PlayerProjectileState>();
        let spec = ae::ProjectileSpec::new(
            ae::ProjectileKind::Fireball,
            ae::Vec2::new(500.0, 380.0),
            ae::Vec2::new(1.0, 0.0),
            1.0,
        );
        let mut body = ae::ProjectileBody::from_spec(spec);
        body.pos = ae::Vec2::new(500.0, 395.0);
        body.vel = ae::Vec2::new(60.0, 240.0);
        starting_bounces = body.bounces_remaining;
        assert!(starting_bounces > 0);
        state.bodies.push(PlayerProjectile { body });
    }
    advance_time(&mut app, 0.016);
    app.update();
    let state = app.world().resource::<PlayerProjectileState>();
    assert_eq!(
        state.bodies.len(),
        1,
        "fireball must survive a floor bounce"
    );
    let body = &state.bodies[0].body;
    assert!(
        body.vel.y < 0.0,
        "post-bounce vy must be upward; got {}",
        body.vel.y
    );
    assert_eq!(body.bounces_remaining, starting_bounces - 1);
}

/// Hadouken spawns with `bounces_remaining = 0`. Hitting any solid
/// expires it on the first contact — pinning the "horizontal
/// projectile that disappears on first wall" behavior at the
/// system level (engine test pinned it at the unit level).
#[test]
fn hadouken_expires_on_solid_in_system() {
    let mut app = App::new();
    app.insert_resource(Time::<()>::default());
    let world = ae::World::new(
        "wall_test",
        ae::Vec2::new(2000.0, 2000.0),
        ae::Vec2::new(200.0, 200.0),
        vec![ae::Block::solid(
            "wall",
            ae::Vec2::new(600.0, 0.0),
            ae::Vec2::new(40.0, 800.0),
        )],
    );
    app.insert_resource(GameWorld(world.clone()));
    let mut runtime = SandboxRuntime::new(
        &world,
        ae::AbilitySet::sandbox_all(),
        ae::DEFAULT_TUNING,
        crate::physics::PhysicsSandboxSettings::default(),
    );
    runtime.player.pos = ae::Vec2::new(500.0, 300.0);
    app.insert_resource(runtime);
    app.insert_resource(ControlFrame::default());
    app.insert_resource(crate::settings::UserSettings::default());
    app.insert_resource(GameplayTraceBuffer::default());
    app.insert_resource(PlayerProjectileState::default());
    app.insert_resource(FeatureEventBus::default());
    app.add_message::<SfxMessage>();
    app.add_message::<VfxMessage>();
    app.add_message::<DebrisBurstMessage>();
    app.add_systems(Update, update_projectiles);

    {
        let mut state = app.world_mut().resource_mut::<PlayerProjectileState>();
        let spec = ae::ProjectileSpec::new(
            ae::ProjectileKind::Hadouken,
            ae::Vec2::new(580.0, 300.0),
            ae::Vec2::new(1.0, 0.0),
            1.0,
        );
        let mut body = ae::ProjectileBody::from_spec(spec);
        body.pos = ae::Vec2::new(595.0, 300.0);
        body.vel = ae::Vec2::new(520.0, 0.0);
        state.bodies.push(PlayerProjectile { body });
    }
    advance_time(&mut app, 0.016);
    app.update();
    let state = app.world().resource::<PlayerProjectileState>();
    assert!(
        state.bodies.is_empty(),
        "Hadouken must expire on first solid hit (no bounces); still alive: {}",
        state.bodies.len()
    );
}

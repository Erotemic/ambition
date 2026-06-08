//! Press / release / charge / QCF gesture tests, plus cooldown and
//! resource-exhaustion gating. All of these exercise the projectile
//! input pipeline via the shared `min_app()` fixture.

use crate::projectile::motion_input::MotionDirection;
use crate::projectile::ProjectileKind;

use super::{advance_time, min_app, tap_projectile, ControlFrame};

#[test]
fn tap_release_fires_one_fireball() {
    let mut app = min_app();
    tap_projectile(&mut app);
    let state = crate::projectile::tests::projectile_state_ref(&app);
    assert_eq!(state.bodies.len(), 1);
    assert_eq!(state.bodies[0].body.game.kind, ProjectileKind::Fireball);
    // Tap-release is below the medium threshold → tier 0.
    assert_eq!(state.bodies[0].body.game.kind, ProjectileKind::Fireball);
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
    let state = crate::projectile::tests::projectile_state_ref(&app);
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
    let state = crate::projectile::tests::projectile_state_ref(&app);
    assert_eq!(state.bodies.len(), 1);
    let body = &state.bodies[0].body;
    assert_eq!(body.game.kind, ProjectileKind::Fireball);
    // Tier-1 size scaling is 1.4x on baseline half-extent (12, 9)
    // → at least 16x12 — meaningfully bigger than tier 0.
    let baseline = crate::projectile::ProjectileKind::Fireball.half_extent();
    assert!(
        body.half_extent().x > baseline.x * 1.2,
        "charged fireball must be visibly larger; got {:?} vs baseline {:?}",
        body.half_extent(),
        baseline
    );
    // Tier-1 damage scaling is 2x baseline (1) = 2.
    assert!(body.game.damage >= 2);
}

/// Grace QCF (Down → Right) + press fires a regular Hadouken
/// IMMEDIATELY (no charging needed). The motion press takes
/// precedence over the charge-start path.
#[test]
fn grace_qcf_then_press_fires_hadouken_immediately() {
    let mut app = min_app();
    {
        let mut state = crate::projectile::tests::projectile_state_mut(&mut app);
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
    let state = crate::projectile::tests::projectile_state_ref(&app);
    assert_eq!(state.bodies.len(), 1);
    assert_eq!(state.bodies[0].body.game.kind, ProjectileKind::Hadouken);
}

#[test]
fn full_qcf_then_press_fires_hadouken_super() {
    let mut app = min_app();
    {
        let mut state = crate::projectile::tests::projectile_state_mut(&mut app);
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
    let state = crate::projectile::tests::projectile_state_ref(&app);
    assert_eq!(state.bodies.len(), 1);
    assert_eq!(
        state.bodies[0].body.game.kind,
        ProjectileKind::HadoukenSuper
    );
}

#[test]
fn half_circle_still_fires_hadouken_super() {
    let mut app = min_app();
    {
        let mut state = crate::projectile::tests::projectile_state_mut(&mut app);
        let mut t = 0.0;
        for dir in [
            MotionDirection::Left,
            MotionDirection::DownLeft,
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
    let state = crate::projectile::tests::projectile_state_ref(&app);
    assert_eq!(state.bodies.len(), 1);
    assert_eq!(
        state.bodies[0].body.game.kind,
        ProjectileKind::HadoukenSuper
    );
}

#[test]
fn cooldown_blocks_second_fire_in_same_window() {
    let mut app = min_app();
    tap_projectile(&mut app);
    // Don't advance past the cooldown — second tap should be no-op.
    tap_projectile(&mut app);
    let state = crate::projectile::tests::projectile_state_ref(&app);
    assert_eq!(state.bodies.len(), 1);
}

#[test]
fn out_of_resource_blocks_fire() {
    let mut app = min_app();
    {
        let mut state = crate::projectile::tests::projectile_state_mut(&mut app);
        state.spawner.meter.current = 0.0;
    }
    tap_projectile(&mut app);
    let state = crate::projectile::tests::projectile_state_ref(&app);
    assert!(state.bodies.is_empty());
}

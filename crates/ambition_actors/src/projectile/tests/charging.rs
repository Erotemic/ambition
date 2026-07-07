//! Press / release / charge / QCF gesture tests, plus cooldown and
//! resource-exhaustion gating. All of these exercise the projectile
//! input pipeline via the shared `min_app()` fixture.

use crate::projectile::MotionDirection;
use crate::projectile::ProjectileKind;

use super::{
    advance_time, dummy_world, min_app, projectile_test_app, tap_projectile, ControlFrame,
};

#[test]
fn tap_release_fires_one_fireball() {
    let mut app = min_app();
    tap_projectile(&mut app);
    let bodies = crate::projectile::tests::projectile_bodies(&mut app);
    assert_eq!(bodies.len(), 1);
    // Tap-release is below the medium threshold → tier 0 Fireball.
    assert_eq!(
        crate::projectile::tests::projectile_kinds(&mut app)[0],
        Some(ProjectileKind::Fireball)
    );
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
    let charging = crate::projectile::tests::projectile_state_ref(&app)
        .charging
        .is_some();
    let bodies = crate::projectile::tests::projectile_bodies(&mut app);
    assert!(
        bodies.is_empty(),
        "press without release must not spawn a fireball"
    );
    assert!(charging, "press without motion must start a charge");
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
    let bodies = crate::projectile::tests::projectile_bodies(&mut app);
    assert_eq!(bodies.len(), 1);
    let body = &bodies[0];
    assert_eq!(
        crate::projectile::tests::projectile_kinds(&mut app)[0],
        Some(ProjectileKind::Fireball)
    );
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

/// Fork D — the charge-projectile INPUT is BODY/ability-subject, not player-marker.
/// A charge-capable body that is NOT the home avatar (no `PlayerEntity`, no
/// `BodyAnimFacts` — the shape a possessed body driving the player's kit takes)
/// fires from ITS OWN muzzle when its `ActorControl` commands a shot, while the home
/// avatar (neutral input) fires nothing. Pins "projectile origin = the emitting
/// body, not the home avatar".
#[test]
fn a_non_home_charge_body_fires_from_its_own_muzzle_not_the_home_avatar() {
    use ambition_characters::actor::control::ActorControlFrame;
    use ambition_characters::brain::{ActorControl, ChargesProjectiles};
    use ambition_engine_core as ae;

    // Home player spawns far to the LEFT; the "possessed" charge body sits far RIGHT.
    let home_pos = ae::Vec2::new(-500.0, 300.0);
    let body_pos = ae::Vec2::new(500.0, 300.0);
    let mut app = projectile_test_app(dummy_world(), home_pos, 1.0);

    // A non-home charge body: the capability + charge state + a body + control, but
    // NO PlayerEntity / BodyAnimFacts / Brain — so `tick_player_brains` skips it
    // and we drive its `ActorControl` directly (as a possessed body would be driven).
    let body = app
        .world_mut()
        .spawn((
            ChargesProjectiles,
            crate::projectile::PlayerProjectileState::default(),
            crate::actor::BodyKinematics {
                pos: body_pos,
                vel: ae::Vec2::ZERO,
                size: ae::Vec2::new(24.0, 40.0),
                facing: 1.0,
            },
            ActorControl::default(),
        ))
        .id();

    let set_ctrl = |app: &mut bevy::prelude::App, pressed: bool, held: bool, released: bool| {
        let mut f = ActorControlFrame::neutral();
        f.projectile_pressed = pressed;
        f.projectile_held = held;
        f.projectile_released = released;
        f.aim = ae::Vec2::new(1.0, 0.0);
        app.world_mut().get_mut::<ActorControl>(body).unwrap().0 = f;
    };

    // Press (start charge), then release (fire).
    set_ctrl(&mut app, true, true, false);
    advance_time(&mut app, 0.016);
    app.update();
    set_ctrl(&mut app, false, false, true);
    advance_time(&mut app, 0.016);
    app.update();

    // Exactly one shot, owned by the non-home body, spawned at its muzzle (RIGHT
    // side of the map), NOT at the home avatar (far LEFT).
    let world = app.world_mut();
    let mut q = world
        .try_query::<(
            &crate::actor::BodyKinematics,
            &crate::projectile::ProjectileOwner,
        )>()
        .unwrap();
    let shots: Vec<(ae::Vec2, bevy::prelude::Entity)> =
        q.iter(world).map(|(k, o)| (k.pos, o.0)).collect();
    assert_eq!(
        shots.len(),
        1,
        "exactly one shot fired — the non-home charge body; the home avatar's neutral \
         input fires nothing"
    );
    let (origin, owner) = shots[0];
    assert_eq!(
        owner, body,
        "the shot is owned by the emitting (non-home) body"
    );
    assert!(
        origin.x > 0.0 && (origin - body_pos).length() < 120.0,
        "origin is the non-home body's OWN muzzle (~{body_pos:?}), not the home \
         avatar (~{home_pos:?}); got {origin:?}"
    );
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
    let bodies = crate::projectile::tests::projectile_bodies(&mut app);
    assert_eq!(bodies.len(), 1);
    assert_eq!(
        crate::projectile::tests::projectile_kinds(&mut app)[0],
        Some(ProjectileKind::Hadouken)
    );
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
    let bodies = crate::projectile::tests::projectile_bodies(&mut app);
    assert_eq!(bodies.len(), 1);
    assert_eq!(
        crate::projectile::tests::projectile_kinds(&mut app)[0],
        Some(ProjectileKind::HadoukenSuper)
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
    let bodies = crate::projectile::tests::projectile_bodies(&mut app);
    assert_eq!(bodies.len(), 1);
    assert_eq!(
        crate::projectile::tests::projectile_kinds(&mut app)[0],
        Some(ProjectileKind::HadoukenSuper)
    );
}

#[test]
fn cooldown_blocks_second_fire_in_same_window() {
    let mut app = min_app();
    tap_projectile(&mut app);
    // Don't advance past the cooldown — second tap should be no-op.
    tap_projectile(&mut app);
    let bodies = crate::projectile::tests::projectile_bodies(&mut app);
    assert_eq!(bodies.len(), 1);
}

#[test]
fn out_of_resource_blocks_fire() {
    let mut app = min_app();
    {
        let mut state = crate::projectile::tests::projectile_state_mut(&mut app);
        state.spawner.meter.current = 0.0;
    }
    tap_projectile(&mut app);
    let bodies = crate::projectile::tests::projectile_bodies(&mut app);
    assert!(bodies.is_empty());
}

#[test]
fn released_fireball_uses_controlled_body_local_aim_under_sideways_gravity() {
    let player_pos = ambition_engine_core::Vec2::new(300.0, 300.0);
    let mut app = projectile_test_app(dummy_world(), player_pos, 1.0);
    app.insert_resource(crate::physics::GravityField {
        dir: ambition_engine_core::Vec2::new(1.0, 0.0),
    });
    // Precision aiming (charged fire) now defaults to screen-directed, so opt
    // into a body-relative aim mode to exercise the controlled-body-local seam
    // this test is about.
    {
        let mut settings = app
            .world_mut()
            .resource_mut::<ambition_persistence::settings::UserSettings>();
        settings.gameplay.aim_frame_mode = ambition_engine_core::InputFrameMode::BodyRelativeStrict;
    }

    {
        let mut frame = app.world_mut().resource_mut::<ControlFrame>();
        frame.projectile_pressed = true;
        frame.projectile_held = true;
    }
    advance_time(&mut app, 0.016);
    app.update();

    {
        let mut frame = app.world_mut().resource_mut::<ControlFrame>();
        frame.projectile_pressed = false;
        frame.projectile_held = false;
        frame.projectile_released = true;
        // In the controlled-body frame, this is local head/away-from-feet.
        frame.aim_x = 0.0;
        frame.aim_y = -1.0;
    }
    advance_time(&mut app, 0.016);
    app.update();

    let bodies = crate::projectile::tests::projectile_bodies(&mut app);
    assert_eq!(bodies.len(), 1);
    let shot = &bodies[0].kin;
    assert!(
        shot.vel.x < 0.0 && shot.vel.y.abs() < 0.01,
        "right-gravity local-head aim should launch world-left; got {:?}",
        shot.vel
    );

    let frame =
        ambition_engine_core::AccelerationFrame::new(ambition_engine_core::Vec2::new(1.0, 0.0));
    let local_offset = frame.to_local(shot.pos - player_pos);
    assert!(
        local_offset.y < -20.0,
        "local-head shots should spawn beyond the controlled body's head; got {:?}",
        local_offset
    );
}

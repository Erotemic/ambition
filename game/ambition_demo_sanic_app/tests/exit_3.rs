//! **Playbook exit 3, gate-enforced.**
//!
//! > *"A demo app builds from runtime+host groups + its content crate with zero
//! > engine edits (the oracle, executable)."*
//! > — `docs/planning/engine/decomposition.md`, exit criterion 3
//!
//! The exit was "Jon-gated, not agent-gated" only because nothing had assembled
//! the app. This file assembles it and steps the real simulation. If a future
//! engine change breaks a demo's ability to boot from `PlatformerEnginePlugins` +
//! `PlatformerHostPlugins` + a content crate, THIS test fails — before anyone
//! builds the windowed half.
//!
//! What it deliberately does NOT assert: anything about FEEL. The momentum
//! tuning, the character sheet, and the drawn frame are the interactive build
//! fable ruled un-shippable headless. The SHELL is architecture and ships now.

use bevy::prelude::*;

use ambition_demo_sanic::{SanicActState, SANIC_MODE};
use ambition_demo_sanic_app::build_demo_app;

/// Seconds per sim tick under `PlatformerEnginePlugins::fixed_tick()`.
const TICK_DT: f32 = 1.0 / 60.0;

fn act_elapsed(app: &mut App) -> Option<f32> {
    let mut q = app.world_mut().query::<&SanicActState>();
    q.iter(app.world()).next().map(|s| s.elapsed)
}

fn player_body(app: &mut App) -> Option<ambition::actors::actor::BodyKinematics> {
    let mut q = app
        .world_mut()
        .query_filtered::<&ambition::actors::actor::BodyKinematics, With<
            ambition::actors::actor::PrimaryPlayer,
        >>();
    q.iter(app.world()).next().copied()
}

#[test]
fn the_demo_shell_boots_from_the_engine_and_host_groups_alone() {
    let mut app = build_demo_app();
    app.update(); // Startup only; the fixed accumulator expends nothing at dt=0.

    assert_eq!(
        app.world().resource::<ambition::runtime::SimTick>().get(),
        0,
        "Startup alone must not advance the timeline"
    );
    assert!(
        player_body(&mut app).is_some(),
        "the content plugin's `simulation_world` must spawn the player body — \
         that is what the host's input attach binds to"
    );
}

#[test]
fn the_demo_steps_the_real_simulation_on_the_fixed_timeline() {
    let mut app = build_demo_app();
    app.update();
    let spawn = player_body(&mut app).expect("player spawned").pos;

    for _ in 0..120 {
        app.update();
    }

    assert_eq!(
        app.world().resource::<ambition::runtime::SimTick>().get(),
        119,
        "one frame at exactly the tick dt expends exactly one tick"
    );

    // The body is in the REAL sim: it fell under gravity and landed on the
    // authored speedway floor. (Feel is not asserted — only that physics ran.)
    let body = player_body(&mut app).expect("player still present");
    assert!(
        body.pos.y > spawn.y,
        "the body should have fallen toward the floor (y grows downward): \
         spawned {spawn:?}, now {:?}",
        body.pos
    );
    assert!(
        body.vel.y.abs() < 1.0,
        "and come to rest on it, not tunnelled through: vel {:?}",
        body.vel
    );
}

#[test]
fn the_demos_own_rules_run_because_its_room_claims_its_mode() {
    let mut app = build_demo_app();
    app.update();
    for _ in 0..60 {
        app.update();
    }

    let elapsed = act_elapsed(&mut app).expect(
        "the mode-scoped act state must exist — `sanic_speedway` claims \
         `mode: sanic` and the rules plugin spawns its owner",
    );
    // 60 ticks at the fixed dt. The act clock is the SIM clock, so bullet-time
    // and pause would slow it exactly as they slow everything else.
    assert!(
        (elapsed - 60.0 * TICK_DT).abs() < 1e-3,
        "the act timer runs on `WorldTime::scaled_dt`: expected {}, got {elapsed}",
        60.0 * TICK_DT
    );
    assert_eq!(SANIC_MODE, "sanic");
}

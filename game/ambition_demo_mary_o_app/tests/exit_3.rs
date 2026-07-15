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

use ambition_demo_mary_o::{MaryOLevelState, MARY_O_MODE, STARTING_TIME};
use ambition_demo_mary_o_app::build_demo_app;

/// Seconds per sim tick under `PlatformerEnginePlugins::fixed_tick()`.
const TICK_DT: f32 = 1.0 / 60.0;

fn clock_remaining(app: &mut App) -> Option<f32> {
    let mut q = app.world_mut().query::<&MaryOLevelState>();
    q.iter(app.world()).next().map(|s| s.time_remaining)
}

fn player_body(app: &mut App) -> Option<ambition::actors::actor::BodyKinematics> {
    let mut q = app
        .world_mut()
        .query_filtered::<&ambition::actors::actor::BodyKinematics, With<
            ambition::actors::actor::PrimaryPlayer,
        >>();
    q.iter(app.world()).next().copied()
}

fn sim_tick(app: &App) -> u64 {
    app.world().resource::<ambition::runtime::SimTick>().get()
}

/// Bevy's fixed-time accumulator can expend the activation frame before the
/// provider publishes its player. Advance to the first post-activation frame
/// that actually executes one simulation tick, then measure the strict
/// one-frame/one-tick contract from that aligned boundary.
fn align_post_activation_fixed_timeline(app: &mut App) {
    for _ in 0..4 {
        let before = sim_tick(app);
        app.update();
        let delta = sim_tick(app) - before;
        match delta {
            1 => return,
            0 => continue,
            other => panic!(
                "one fixed frame must not expend more than one tick while aligning; got {other}"
            ),
        }
    }
    panic!("the fixed timeline did not advance after gameplay activation");
}

fn activate_player(app: &mut App) -> ambition::actors::actor::BodyKinematics {
    for _ in 0..16 {
        app.update();
        if let Some(body) = player_body(app) {
            return body;
        }
    }
    panic!("the fresh load transaction must prepare and activate Mary-O within the test budget");
}

#[test]
fn the_demo_shell_boots_from_the_engine_and_host_groups_alone() {
    let mut app = build_demo_app();
    let _player = activate_player(&mut app);

    assert!(
        app.world()
            .resource::<ambition::game_shell::ActiveGameplaySession>()
            .0
            .is_some(),
        "the standalone shell must authorize the prepared gameplay session"
    );
    assert!(
        player_body(&mut app).is_some(),
        "the provider's prepared-session activation must spawn the player body — \
         that is what the host's input attach binds to"
    );
}

#[test]
fn the_demo_steps_the_real_simulation_on_the_fixed_timeline() {
    let mut app = build_demo_app();
    activate_player(&mut app);
    align_post_activation_fixed_timeline(&mut app);
    let spawn = player_body(&mut app)
        .expect("player remains after alignment")
        .pos;
    let start_tick = sim_tick(&app);

    for frame in 0..120 {
        let before = sim_tick(&app);
        app.update();
        let after = sim_tick(&app);
        assert_eq!(
            after - before,
            1,
            "aligned fixed frame {frame} at exactly the tick dt must expend exactly one tick"
        );
    }

    let end_tick = sim_tick(&app);
    assert_eq!(end_tick - start_tick, 120);

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
    // Drive past the shell's frame-1 activation so the session is live and the
    // mode-scoped level owner exists; measure a delta, not an absolute (the shell
    // activates in `Update`, one frame after a direct-entry spawn would).
    for _ in 0..3 {
        app.update();
    }
    let start = clock_remaining(&mut app).expect(
        "the session's level state must exist — `level_1_1` claims \
         `mode: mary_o` and the rules plugin spawns its owner once the session \
         is live",
    );

    for _ in 0..60 {
        app.update();
    }
    let end = clock_remaining(&mut app).expect("the level state persists across the session");

    // 60 ticks at the fixed dt, counting DOWN. The level clock is the SIM clock,
    // so bullet-time and pause slow it exactly as they slow everything else.
    assert!(
        (start - end - 60.0 * TICK_DT).abs() < 1e-3,
        "the level clock runs on `WorldTime::scaled_dt`: expected -{}, got -{}",
        60.0 * TICK_DT,
        start - end
    );
    assert_eq!(MARY_O_MODE, "mary_o");
    assert!(STARTING_TIME > 0.0);
}

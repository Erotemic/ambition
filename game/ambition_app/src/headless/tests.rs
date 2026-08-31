use super::*;
use ambition_platformer2d::input::ControlFrame;
// presses go through the SEAM, not at the resource. `ControlFrame` is seat zero's OUTPUT
// mirror since; assigning it delivers a press to nobody and a fixture doing so asserts its way
// to a green run against a simulation that never received an input.
use ambition_platformer2d::sfx::SfxMessage;
use ambition_platformer2d::sim::drive_control_frame;
use bevy::ecs::message::Messages;

/// K2b edit 2: the ONE composition, shell and all.
fn sandbox_sim_app() -> App {
    let mut app = App::new();
    ambition_platformer2d::runtime::add_headless_foundation(&mut app);
    crate::app::shell_host::compose_ambition_gameplay_host(&mut app);
    app
}

/// The settle helper waits for the session world rather than guessing a frame count, and PANICS
/// with the budget when it does not arrive — a fixture that silently returned an un-activated
/// App would make every test using it fail somewhere less informative.
fn initialized_sandbox_sim_app() -> App {
    let mut app = sandbox_sim_app();
    ambition_platformer2d::platformer::lifecycle::settle_until_session_world(
        &mut app,
        ambition_platformer2d::platformer::lifecycle::SESSION_SETTLE_FRAMES,
    )
    .unwrap_or_else(|budget| {
        panic!(
            "the shell-composed sandbox produced no session world in {budget} frames, \
             so every test built on this fixture would fail against an empty world"
        )
    });
    app
}

#[test]
fn run_headless_completes_one_tick_without_panicking() {
    let report = run_headless(1).expect("headless one-tick run succeeds");
    assert_eq!(report.ticks_run, 1);
    assert!(
        report.room_count > 0,
        "embedded LDtk should produce at least one room"
    );
    assert!(!report.active_room.is_empty());
}

#[test]
fn run_headless_runs_multiple_ticks() {
    let report = run_headless(8).expect("headless eight-tick run succeeds");
    assert_eq!(report.ticks_run, 8);
}

/// ADR 0012 step B stop gate: with `MinimalPlugins` only and no
/// AudioPlugin / RenderPlugin / inspector, can we drive the player tick
/// end-to-end and observe `SfxMessage` flow? This proves the sim/presentation
/// seam holds for the input + sfx channels. Reset is the cheapest path — no
/// spawn-position dependence.
///
/// ⛔⛔ IT IS NO LONGER SYNCHRONOUS, and this test asserted that it was. Its own
/// doc said *"pressing Reset emits `SfxMessage::Reset` synchronously"* and it
/// read `iter_current_update_messages` after ONE update. A same-room replay
/// became a canonical room REBUILD, which may only commit at a confirmed
/// lifecycle boundary — the log shows `room-replay admitted reason=Manual` on
/// frame 2 — so the cue arrives a couple of frames after the press. The seam it
/// exists to prove is intact; the frame it looked at was not.
///
/// ⚠ SO IT SCANS A WINDOW, and the window is the assertion: an unbounded loop
/// would hang on a genuinely silent reset, and reading one frame is what went
/// stale. If the boundary moves further out, this fails with the count it saw.
#[test]
fn sim_emits_sfx_reset_when_control_frame_requests_reset() {
    let mut app = initialized_sandbox_sim_app();

    // Inject a "press reset" frame on the sim/presentation input seam.
    drive_control_frame(
        app.world_mut(),
        ControlFrame {
            reset_pressed: true,
            ..ControlFrame::default()
        },
    );

    let mut reset_count = 0usize;
    let mut any_cue = 0usize;
    let mut frames = 0usize;
    for _ in 0..30 {
        app.update();
        frames += 1;
        let messages = app
            .world()
            .resource::<Messages<ambition_platformer2d::sfx::OwnedSfxMessage>>();
        any_cue += messages.iter_current_update_messages().count();
        reset_count += messages
            .iter_current_update_messages()
            .filter(|m| matches!(m.request, SfxMessage::Reset { .. }))
            .count();
        if reset_count > 0 {
            break;
        }
    }
    // ⛔ THE SECOND NUMBER IS WHAT NARROWS IT, and it is why this reports both.
    // `any_cue == 0` says the SEAM is silent — no cue of any kind crossed it —
    // which is a different fault from "the reset specifically stopped cueing",
    // and the two want different fixes. Measured 2026-08-31: it is ZERO.
    assert!(
        reset_count >= 1,
        "no `SfxMessage::Reset` reached the presentation seam in {frames} frames \
         after a reset press ({any_cue} cues of ANY kind crossed it in that \
         window). The replay IS admitted — the world log says so — so either the \
         reset is silent or nothing writes this channel in a headless sandbox at \
         all. See D-SFX-RESET-RED.",
    );
}

#[test]
fn sim_completes_60_ticks_with_counter_intact() {
    use ambition_platformer2d::characters::brain::BrainActionCounter;
    let mut app = sandbox_sim_app();
    // Run 60 ticks (1 sim second at 60Hz).
    for _ in 0..60 {
        app.update();
    }
    let counter = app.world().resource::<BrainActionCounter>();
    // Total is a running sum, last_frame is per-frame count;
    // last_frame must never exceed total (would indicate the
    // observer is double-counting or the reset got out of
    // order).
    assert!(
        counter.last_frame as u64 <= counter.total,
        "last_frame={} exceeds total={}",
        counter.last_frame,
        counter.total,
    );
}

/// Verify the BrainPlugin is installed by AmbitionGameSimulationPlugin
/// — adding the plugin should mean ActorActionMessage +
/// BrainActionCounter are both registered. Catches a future
/// app-plugin refactor that accidentally drops the
/// `app.add_plugins(ambition_platformer2d::characters::brain::BrainPlugin)` call.
#[test]
fn sim_includes_brain_plugin_registration() {
    use ambition_platformer2d::characters::brain::{ActorActionMessage, BrainActionCounter};
    use bevy::ecs::message::Messages;
    let app = initialized_sandbox_sim_app();
    // Both resources should be present.
    assert!(
        app.world()
            .get_resource::<Messages<ActorActionMessage>>()
            .is_some(),
        "ActorActionMessage registered via BrainPlugin",
    );
    assert!(
        app.world().get_resource::<BrainActionCounter>().is_some(),
        "BrainActionCounter registered via BrainPlugin",
    );
}

/// Sustained run with multiple player attack presses: stamp
/// attack on every other tick for 20 ticks and verify the
/// counter accumulates at least 10 melee messages. Pins that
/// the seam survives sustained brain-message production
/// (not just single-tick poison).
#[test]
fn sim_accumulates_messages_across_repeated_attacks() {
    use ambition_platformer2d::characters::brain::BrainActionCounter;
    let mut app = initialized_sandbox_sim_app();
    for i in 0..20 {
        let attack = i % 2 == 0;
        drive_control_frame(
            app.world_mut(),
            ControlFrame {
                attack_pressed: attack,
                ..ControlFrame::default()
            },
        );
        app.update();
    }
    let counter = app.world().resource::<BrainActionCounter>();
    // 10 attack-press ticks × 1 melee message each = 10 total.
    // Other ticks may emit zero or other actions; assert
    // floor.
    assert!(
        counter.total >= 10,
        "expected ≥ 10 ActorActionMessages over 20-tick mix; got {}",
        counter.total,
    );
}

/// Universal-brain integration check: spawning the
/// AmbitionGameSimulationPlugin yields a player entity holding the primary
/// participant's seat and an ActionSet — verifies the bundle
/// path injects the components even when the spawn flow
/// runs through the real Startup schedule.
#[test]
fn sim_spawns_player_with_brain_and_action_set() {
    use ambition_platformer2d::characters::brain::ActionSet;
    use ambition_platformer2d::characters::control::ActorControl;
    use ambition_platformer2d::characters::control::{DrivingParticipant, PlayerSlot};
    use ambition_platformer2d::platformer::markers::PlayerEntity;
    let mut app = initialized_sandbox_sim_app();
    let mut q = app
        .world_mut()
        .query_filtered::<(&DrivingParticipant, &ActionSet, &ActorControl), With<PlayerEntity>>();
    let count = q.iter(app.world()).count();
    assert_eq!(
        count, 1,
        "player should spawn with a seat + ActionSet + ActorControl"
    );
    let (driver, action_set, _control) = q.iter(app.world()).next().expect("player exists");
    assert_eq!(
        driver.0,
        PlayerSlot::PRIMARY,
        "the home avatar holds the primary participant's seat"
    );
    assert!(
        action_set.melee.is_some(),
        "player ActionSet has Swipe melee"
    );
}

/// Universal-brain integration check: with the full
/// AmbitionGameSimulationPlugin installed, the player carries a
/// Brain + ActionSet + ActorControl, the brain ticks each
/// frame, and the ActionSet resolver writes an
/// ActorActionMessage when the input frame triggers attack.
/// Validates the production wiring (vs the synthetic mini-app
/// in `player/systems.rs` tests).
#[test]
fn sim_emits_action_messages_when_player_attacks() {
    use ambition_platformer2d::characters::brain::{ActorActionMessage, BrainActionCounter};
    let mut app = initialized_sandbox_sim_app();
    // Stamp an attack press into the control frame.
    drive_control_frame(
        app.world_mut(),
        ControlFrame {
            attack_pressed: true,
            ..ControlFrame::default()
        },
    );
    app.update();
    let counter = app.world().resource::<BrainActionCounter>();
    let messages = app.world().resource::<Messages<ActorActionMessage>>();
    let melee_count = messages
        .iter_current_update_messages()
        .filter(|m| m.is_melee())
        .count();
    assert!(
        melee_count >= 1,
        "expected at least one Melee ActorActionMessage; counter.last_frame={}",
        counter.last_frame,
    );
}

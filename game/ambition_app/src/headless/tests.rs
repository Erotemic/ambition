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
    // ⛔ A FRAME IS A TICK HERE, AND IT WAS NOT BEFORE. `add_headless_foundation`
    // brings `MinimalPlugins`, which leaves `TimeUpdateStrategy::Automatic`, so
    // `update()` steps the fixed schedule a WALL-TIME-derived number of times.
    // Measured on this fixture: settling banked enough wall time to spend 15 fixed
    // steps in 2 frames, and the next twenty frames bought SEVEN ticks with
    // thirteen of them stepping none at all. Every arm below that counts `update()`
    // calls was counting frames and calling them ticks.
    let timestep = app
        .world()
        .resource::<bevy::time::Time<bevy::time::Fixed>>()
        .timestep();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(timestep));
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
///
/// ⛔⛔ AND THE WINDOW WAS NEVER THE PROBLEM. Instrumented 2026-08-31 inside
/// `admit_room_replay`: `ControlledSubject` is present and holds **`None`**, so
/// the admission carries NO SUBJECT, takes its *"no body, no crossing to
/// describe"* arm, and resets nothing — no body moved, no cue written, zero SFX
/// messages of any kind in forty frames.
///
/// ⭐⭐ AND THE ENGINE IS RIGHT; THE FIXTURE PRESSED ONE FRAME TOO EARLY.
/// `ControlledSubject` is resolved from `DrivingParticipant`, and
/// `settle_until_session_world` stops as soon as a session WORLD exists — one
/// frame before the resolver has run. See the `app.update()` below and the truth
/// table beside it.
///
/// ⛔ do not "fix" a future failure here by asserting the count it happens to
/// produce, and do not loosen it to "the replay was admitted": the admission is
/// exactly the half that kept working while nothing was reset.
#[test]
fn sim_emits_sfx_reset_when_control_frame_requests_reset() {
    let mut app = initialized_sandbox_sim_app();

    // ⭐⭐ ONE MORE FRAME BEFORE THE PRESS, and it is the whole fix.
    // `settle_until_session_world` stops as soon as a session WORLD exists,
    // which is one frame before the session is DRIVABLE:
    // `resolve_controlled_subject` has not yet copied the seat's
    // `DrivingParticipant` into `ControlledSubject`. The reset resolves its
    // subject from that resource, so a press on the gap frame finds `None`,
    // takes the replay's "no body, no crossing to describe" arm, and resets
    // nothing — silently, because the branch's own `info!` needs a `LogPlugin`
    // this fixture does not install.
    //
    // ⛔ MEASURED AS A TRUTH TABLE, because the obvious repair was the wrong
    // one. Staging a `DrivingParticipant` by hand does NOT fix it without this
    // line (the resolver still has not run), and this line fixes it WITHOUT the
    // staging (a body was already seated). The missing thing was a frame, not a
    // seat.
    app.update();

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

/// `last_frame` is a per-frame count and `total` a running sum, so the observer
/// double-counting or resetting out of order shows up as `last_frame > total`.
///
/// ⛔ THE FLOOR IS THE HALF THAT MAKES THIS AN ASSERTION. `0 <= 0` is true, so
/// the comparison alone passed against a counter that had never been written —
/// which is exactly what an unpinned clock produced, since the fixed schedule
/// could step zero times for the whole run. The clock is pinned in the fixture
/// now and the counter must have observed something.
#[test]
fn sim_completes_60_ticks_with_counter_intact() {
    use ambition_platformer2d::characters::brain::BrainActionCounter;
    let mut app = sandbox_sim_app();
    for _ in 0..60 {
        app.update();
    }
    let counter = app.world().resource::<BrainActionCounter>();
    assert!(
        counter.total > 0,
        "sixty pinned ticks produced no brain action at all, so the ordering \
         assertion below would pass on an empty counter"
    );
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

/// Sustained play with the attack pressed on every other tick emits melee
/// messages, and the seam survives the repetition rather than only the first one.
///
/// IT USED TO READ `BrainActionCounter::total`, WHICH COUNTS EVERY ACTION BY
/// EVERY ACTOR. Poisoned 2026-09-16 by holding the button un-pressed for the whole
/// run: it still passed, because ambient brains clear a floor of ten on their own.
/// The arm was named for attacks and measured the room. So it counts MELEE
/// messages, and it runs the same twenty ticks with the button down and with it
/// up — the difference is the assertion, and neither number alone is one.
#[test]
fn sim_accumulates_messages_across_repeated_attacks() {
    use ambition_platformer2d::characters::brain::ActorActionMessage;

    fn melee_over_twenty_ticks(press: bool) -> usize {
        let mut app = initialized_sandbox_sim_app();
        let mut melee = 0usize;
        for i in 0..20 {
            drive_control_frame(
                app.world_mut(),
                ControlFrame {
                    attack_pressed: press && i % 2 == 0,
                    ..ControlFrame::default()
                },
            );
            app.update();
            melee += app
                .world()
                .resource::<Messages<ActorActionMessage>>()
                .iter_current_update_messages()
                .filter(|m| m.is_melee())
                .count();
        }
        melee
    }

    let pressed = melee_over_twenty_ticks(true);
    let idle = melee_over_twenty_ticks(false);
    assert!(
        pressed > idle,
        "ten attack presses over twenty ticks produced {pressed} melee messages \
         against {idle} with the button never pressed, so this arm cannot tell a \
         working attack seam from a room full of brains"
    );
    assert!(
        pressed >= 10,
        "expected one melee message per attack press over twenty ticks; got \
         {pressed} (idle baseline {idle})"
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
/// Validates the PRODUCTION wiring rather than a synthetic mini-app.
/// (This used to contrast itself with tests in `player/systems.rs`;
/// `5ba894709` ended that directory and the mini-app went with it, so
/// there is no longer a second road to be the production half OF.)
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

#[derive(bevy::prelude::Resource, Default)]
struct ProbeFixedSteps(Vec<u32>, u32);

/// How many fixed steps each `update()` of this file's fixture actually buys.
#[test]
#[ignore = "PROBE, print-only: reports the per-frame fixed-step count of the unpinned sandbox fixture"]
fn probe_how_many_fixed_steps_the_sandbox_fixture_takes() {
    let mut app = sandbox_sim_app();
    app.init_resource::<ProbeFixedSteps>();
    app.add_systems(
        bevy::prelude::FixedUpdate,
        |mut probe: bevy::prelude::ResMut<ProbeFixedSteps>| probe.1 += 1,
    );
    app.add_systems(bevy::prelude::Last, |mut probe: bevy::prelude::ResMut<ProbeFixedSteps>| {
        let taken = probe.1;
        probe.0.push(taken);
        probe.1 = 0;
    });

    let settled = ambition_platformer2d::platformer::lifecycle::settle_until_session_world(
        &mut app,
        ambition_platformer2d::platformer::lifecycle::SESSION_SETTLE_FRAMES,
    );
    let settle_frames = app.world().resource::<ProbeFixedSteps>().0.len();
    let settle_steps: u32 = app.world().resource::<ProbeFixedSteps>().0.iter().sum();
    for _ in 0..20 {
        app.update();
    }
    let per_frame = app.world().resource::<ProbeFixedSteps>().0.clone();
    let after: Vec<u32> = per_frame[settle_frames..].to_vec();
    let total: u32 = after.iter().sum();
    eprintln!(
        "PROBE settle={settled:?} settle_frames={settle_frames} settle_steps={settle_steps} \
         after_settle_per_frame={after:?} after_settle_total={total}"
    );
}

//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

use super::*;
use ambition::input::ControlFrame;
use ambition::sfx::SfxMessage;
use bevy::ecs::message::Messages;

fn sandbox_sim_app() -> App {
    let mut app = App::new();
    ambition::runtime::add_headless_foundation(&mut app);
    app.add_plugins(crate::app::SandboxSimulationPlugin);
    app
}

fn initialized_sandbox_sim_app() -> App {
    let mut app = sandbox_sim_app();
    app.update();
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
/// AudioPlugin / RenderPlugin / inspector, can we drive
/// the player tick end-to-end and observe `SfxMessage` flow? This
/// proves the sim/presentation seam holds for the input + sfx
/// channels. Reset is the cheapest path: pressing Reset emits
/// `SfxMessage::Reset` synchronously, no spawn-position dependence.
#[test]
fn sim_emits_sfx_reset_when_control_frame_requests_reset() {
    let mut app = initialized_sandbox_sim_app();

    // Inject a "press reset" frame on the sim/presentation input seam.
    *app.world_mut().resource_mut::<ControlFrame>() = ControlFrame {
        reset_pressed: true,
        ..ControlFrame::default()
    };

    app.update();

    let messages = app
        .world()
        .resource::<Messages<ambition::sfx::OwnedSfxMessage>>();
    let reset_count = messages
        .iter_current_update_messages()
        .filter(|m| matches!(m.request, SfxMessage::Reset { .. }))
        .count();
    assert!(
        reset_count >= 1,
        "expected at least one SfxMessage::Reset emitted by the sim; got {reset_count}",
    );
}

/// Sustained run check: drive the full sim for 60 ticks and
/// verify the brain action counter is present and its
/// `last_frame` is internally consistent (≤ total). Catches a
/// future regression where the brain tick + resolver path
/// starts panicking somewhere mid-room (player loading the wrong
/// room and brain seeing inconsistent state) or where the
/// counter resource gets reset/leaked between frames.
#[test]
fn sim_completes_60_ticks_with_counter_intact() {
    use ambition::characters::brain::BrainActionCounter;
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

/// Verify the BrainPlugin is installed by SandboxSimulationPlugin
/// — adding the plugin should mean ActorActionMessage +
/// BrainActionCounter are both registered. Catches a future
/// app-plugin refactor that accidentally drops the
/// `app.add_plugins(ambition::characters::brain::BrainPlugin)` call.
#[test]
fn sim_includes_brain_plugin_registration() {
    use ambition::characters::brain::{ActorActionMessage, BrainActionCounter};
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
    use ambition::characters::brain::BrainActionCounter;
    let mut app = initialized_sandbox_sim_app();
    for i in 0..20 {
        let attack = i % 2 == 0;
        *app.world_mut().resource_mut::<ControlFrame>() = ControlFrame {
            attack_pressed: attack,
            ..ControlFrame::default()
        };
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
/// SandboxSimulationPlugin yields a player entity carrying
/// Brain::Player and an ActionSet — verifies the bundle
/// path injects the components even when the spawn flow
/// runs through the real Startup schedule.
#[test]
fn sim_spawns_player_with_brain_and_action_set() {
    use ambition::actors::actor::PlayerEntity;
    use ambition::characters::brain::{ActionSet, ActorControl, Brain};
    let mut app = initialized_sandbox_sim_app();
    let mut q = app
        .world_mut()
        .query_filtered::<(&Brain, &ActionSet, &ActorControl), With<PlayerEntity>>();
    let count = q.iter(app.world()).count();
    assert_eq!(
        count, 1,
        "player should spawn with Brain + ActionSet + ActorControl"
    );
    let (brain, action_set, _control) = q.iter(app.world()).next().expect("player exists");
    assert!(brain.is_player(), "player carries Brain::Player");
    assert!(
        action_set.melee.is_some(),
        "player ActionSet has Swipe melee"
    );
}

/// Universal-brain integration check: with the full
/// SandboxSimulationPlugin installed, the player carries a
/// Brain + ActionSet + ActorControl, the brain ticks each
/// frame, and the ActionSet resolver writes an
/// ActorActionMessage when the input frame triggers attack.
/// Validates the production wiring (vs the synthetic mini-app
/// in `player/systems.rs` tests).
#[test]
fn sim_emits_action_messages_when_player_attacks() {
    use ambition::characters::brain::{ActorActionMessage, BrainActionCounter};
    let mut app = initialized_sandbox_sim_app();
    // Stamp an attack press into the control frame.
    *app.world_mut().resource_mut::<ControlFrame>() = ControlFrame {
        attack_pressed: true,
        ..ControlFrame::default()
    };
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

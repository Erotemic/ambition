#[allow(unused_imports)]
use super::cli::*;
#[allow(unused_imports)]
use super::dev_runtime::*;
#[allow(unused_imports)]
use super::feedback::*;
#[allow(unused_imports)]
use super::hud::*;
#[allow(unused_imports)]
use super::input_systems::*;
#[allow(unused_imports)]
use super::phases::*;
#[allow(unused_imports)]
use super::plugins::*;
#[allow(unused_imports)]
use super::resources::*;
#[allow(unused_imports)]
use super::setup_systems::*;
#[allow(unused_imports)]
use super::world_flow::*;
#[allow(unused_imports)]
use super::*;

/// Bevy gameplay system that drives the sandbox simulation.
///
/// This is intentionally a thin orchestrator around named `*_phase`
/// helpers — the function body should make the gameplay frame order
/// readable in one screen so future agents can find the right phase by
/// grep without reading the whole loop.
///
/// The next likely refactor is promoting these phase helpers into
/// individually ordered Bevy systems, one at a time, once their behavior
/// is covered by tests. Until then, keep them as plain functions sharing
/// `&mut PlayerMovementAuthority` and `&mut FrameFeedback` so the borrow
/// graph stays linear.
///
/// Phase order (each phase comments its scope and what it should not own).
/// Phases marked "(extracted)" are real Bevy systems in `sim_systems.rs`
/// or `world_flow.rs`; they no longer run inline here.
///
/// Pre-tick (CoreSimulation, before sandbox_update):
/// - `sync_live_player_dev_edits_system` (extracted) — F3 inspector
///   ability/tuning edits, runs every frame including paused.
/// - `apply_player_reset_input_system` (extracted) — input-driven
///   reset (`controls.reset_pressed`); runs `reset_sandbox` and
///   clears the press so the engine path inside
///   `player_control_phase` doesn't double-fire.
/// - `input_timer_system` (extracted) — gameplay timer decay +
///   double-tap detection, gated by `gameplay_allowed`.
/// - `interaction_input_system` (extracted) — fold raw Interact +
///   double-tap-up signal, gate by hit-stun, update
///   `PlayerInteractionState`'s buffer.
/// - `apply_suspended_time_scale_system` (extracted) — zeros
///   `SandboxSimState::time_scale` when gameplay is suspended;
///   complement of `sandbox_update`'s `gameplay_allowed` gate.
///
/// Inside sandbox_update (gated by `run_if(gameplay_allowed)`):
/// 1. `player_control_phase` — control-clock player update + pogo
///    routing. Still inline because `update_player_control_with_tuning`
///    may return `events.reset = true` (engine-driven respawn), which
///    needs immediate sandbox-side cleanup. Also handles
///    `handle_player_events` for sfx/vfx that still flow through
///    `FrameFeedback`.
/// 2. `player_simulation_phase` — sim-clock player update + landing dust.
/// 3. `flush_feedback` — drains the two remaining Vec collectors
///    (`SfxMessage`, `VfxMessage`) into the bundled writers, once, at
///    the bottom of the `'frame` labeled block.
///
/// Post-tick (CoreSimulation, after sandbox_update):
/// - `apply_player_damage_system` (extracted) — drains
///   `MessageReader<PlayerDamageEvent>`, runs
///   `handle_player_damage_events` + `remember_safe_player_position`.
///   Writes sfx / vfx / died directly via `MessageWriter`s.
/// - `detect_room_transition_system` (extracted) — loading-zone overlap
///   detection; emits `RoomTransitionRequested`.
/// - `attack_advance_system` (extracted) — slash / pogo attack lifecycle;
///   writes sfx/vfx/damage/pogo channels directly via `MessageWriter`s.
/// - `apply_room_transition_system` (extracted) — consumes the message
///   and runs `load_room`.
/// - `cleanup_timers_system` (extracted) — flash / preset / slash /
///   blink animation timer decay, runs every frame unconditionally.
pub fn sandbox_update(
    time: Res<Time>,
    world: Res<GameWorld>,
    editable_tuning: Res<EditableMovementTuning>,
    feel_tuning: Res<SandboxFeelTuning>,
    mut event_writers: SandboxEventWriters,
    control_frame: Res<ControlFrame>,
    mut queues: SandboxQueues,
    mut player_q: Query<
        (
            &mut crate::player::PlayerMovementAuthority,
            &mut crate::player::PlayerAnimState,
            &mut crate::player::PlayerCombatState,
            &mut crate::player::PlayerInteractionState,
            &mut crate::player::PlayerBlinkCameraState,
            &mut crate::player::PlayerPlatformRideState,
        ),
        With<crate::player::PlayerEntity>,
    >,
) {
    let mut feedback = FrameFeedback::new();
    let tuning = editable_tuning.as_engine();
    let feel = *feel_tuning;

    // Each surviving inline phase appends sfx/vfx to `feedback`; the
    // labeled block below lets any phase short-circuit the tick via
    // `break` while keeping the single `flush_feedback` drain at the
    // bottom. Also guarantees feedback is drained on the "no player
    // entity yet" path, since that's modeled as `break` here.
    'frame: {
        // Acquire ECS player components for this frame.
        let Ok((mut authority, mut anim, mut combat, mut interaction, mut blink_cam, mut ride)) = player_q.single_mut() else {
            break 'frame;
        };
        let player = &mut authority.player;
        // Note: `sync_live_player_dev_edits_system` (in sim_systems) runs
        // unconditionally before sandbox_update so dev-tool ability /
        // tuning edits land even while the sim is paused.

        // sandbox_update no longer queries leafwing directly. Input arrives
        // through `Res<ControlFrame>` — visible builds derive it from
        // ActionState in `populate_control_frame_from_actions` (runs
        // `.before(sandbox_update)`); headless / RL drivers can write the
        // resource directly. Debug hotkeys live in their own presentation-
        // side system, also `.before(sandbox_update)`. The local copy is
        // read-only for the rest of this function; `interaction_input_system`
        // already wrote the buffered interact result into
        // `PlayerInteractionState` before sandbox_update started, so
        // `controls.interact_pressed` is just the raw frame input.
        let controls = *control_frame;
        let frame_dt = time.delta_secs();

        // Pause/resume toggling has moved to `pause_menu::pause_menu_toggle`,
        // which runs `.before(SandboxSet::CoreSimulation)`. The
        // `start_pressed` flag is still read here for compile-completeness;
        // the pause logic lives in the pause menu so it can drive a real
        // overlay.
        let _ = controls.start_pressed;

        // Input-driven reset (controls.reset_pressed) was extracted to
        // `apply_player_reset_input_system` in sim_systems, which runs
        // pre-tick and clears `controls.reset_pressed` so the engine
        // path inside player_control_phase doesn't double-trigger.
        // Engine-driven resets (control_events.reset / sim_events.reset)
        // still run inline below.

        if matches!(
            player_control_phase(
                controls,
                &world.0,
                player,
                &mut queues.sim_state,
                &queues.moving_platforms.0,
                &mut queues.current_attack.0,
                &mut feedback,
                tuning,
                feel,
                frame_dt,
                &queues.feature_ecs_overlay,
                &mut queues.reset_room_features,
                &mut queues.pogo_bounces,
                &mut *anim,
                &mut *combat,
                &mut *interaction,
                &mut *blink_cam,
            ),
            PhaseOutcome::Return
        ) {
            break 'frame;
        }

        if matches!(
            player_simulation_phase(
                controls,
                &world.0,
                player,
                &queues.dev_state,
                &mut queues.sim_state,
                &mut queues.moving_platforms.0,
                &mut queues.current_attack.0,
                &mut feedback,
                tuning,
                feel,
                frame_dt,
                &queues.feature_ecs_overlay,
                &mut queues.reset_room_features,
                &mut *anim,
                &mut *combat,
                &mut *interaction,
                &mut *blink_cam,
                &mut *ride,
            ),
            PhaseOutcome::Return
        ) {
            break 'frame;
        }

        // interaction_input_phase, damage_heal_dialogue_phase,
        // room_transition_phase, and attack_phase have all moved to
        // Bevy systems in `sim_systems`. They run before or after
        // sandbox_update in the CoreSimulation chain and write
        // directly to their `MessageWriter` outputs.
        //
        // cleanup_timers_system runs after write_player_ecs_components in
        // the CoreSimulation chain every frame unconditionally (it lives
        // outside sandbox_update so paused/dialogue modes still wind down
        // flash and landing-pose timers).
    }

    flush_feedback(&mut feedback, &mut event_writers);
}

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
/// What's left inline is the two-clock player update — control then
/// simulation — because both halves share `&mut ae::Player` and both
/// can early-return on an engine-driven reset (`update_player_*_with_tuning`
/// returning `events.reset = true`). Extracting either half would require
/// either a player-borrow split or a deferred reset message, both of
/// which carry more risk than the current minor inline orchestration.
///
/// `FrameFeedback` is gone — `player_control_phase` and
/// `player_simulation_phase` now take `&mut MessageWriter<SfxMessage>`
/// and `&mut MessageWriter<VfxMessage>` directly via
/// [`SandboxEventWriters`]'s split borrows.
///
/// Pre-tick (in `SandboxSet::PlayerInput`, before this system):
/// - `sync_live_player_dev_edits_system` — F3 inspector edits, always-on.
/// - `apply_player_reset_input_system` — input-driven reset; clears
///   `controls.reset_pressed` so the engine path here doesn't double-fire.
/// - `input_timer_system` — gameplay timer decay + double-tap detection.
/// - `interaction_input_system` — fold raw Interact + double-tap-up,
///   gate by hit-stun, update `PlayerInteractionState`'s interact buffer.
/// - `apply_suspended_time_scale_system` — zeros `time_scale` while
///   gameplay is suspended (complement of this system's `gameplay_allowed`
///   run-condition).
///
/// Inside `sandbox_update` (gated by `run_if(gameplay_allowed)`):
/// 1. `player_control_phase` — control-clock player update + pogo
///    routing. Returns early on engine-driven reset.
/// 2. `player_simulation_phase` — sim-clock player update + landing dust.
///    Returns early on engine-driven reset.
///
/// Post-tick (in `SandboxSet::PlayerSimulation` / `RoomTransition` /
/// `Combat` / `PresentationSync`, after this system):
/// - `apply_player_damage_system` — drains `MessageReader<PlayerDamageEvent>`,
///   runs `handle_player_damage_events` + `remember_safe_player_position`.
/// - `detect_room_transition_system` / `apply_room_transition_system`.
/// - `attack_advance_system` — slash / pogo attack lifecycle.
/// - `write_player_ecs_components` + `cleanup_timers_system`.
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
            &mut crate::player::ActivePlayerAttack,
            &mut crate::player::PlayerSafetyState,
        ),
        With<crate::player::PlayerEntity>,
    >,
) {
    let Ok((
        mut authority,
        mut anim,
        mut combat,
        mut interaction,
        mut blink_cam,
        mut ride,
        mut attack,
        mut safety,
    )) = player_q.single_mut()
    else {
        return;
    };
    let player = &mut authority.player;
    let tuning = editable_tuning.as_engine();
    let feel = *feel_tuning;
    let controls = *control_frame;
    let frame_dt = time.delta_secs();

    // Pause/resume toggling has moved to `pause_menu::pause_menu_toggle`,
    // which runs `.before(SandboxSet::CoreSimulation)`. The
    // `start_pressed` flag is still read here for compile-completeness.
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
            &mut safety,
            &queues.moving_platforms.0,
            &mut attack.0,
            &mut event_writers.sfx,
            &mut event_writers.vfx,
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
        return;
    }

    if matches!(
        player_simulation_phase(
            controls,
            &world.0,
            player,
            &queues.dev_state,
            &mut queues.sim_state,
            &mut safety,
            &mut queues.moving_platforms.0,
            &mut attack.0,
            &mut event_writers.sfx,
            &mut event_writers.vfx,
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
        return;
    }
}

//! Per-frame player tick: the scheduled control-clock and sim-clock
//! systems that integrate the player entity each frame.
//!
//! Two systems live here, chained in this order inside the
//! `PlayerSimulation` set:
//!
//! 1. [`clear_sandbox_reset_this_frame`] — resets the per-frame
//!    `SandboxResetThisFrame` flag so the two systems below can use
//!    it as a one-way signal that a reset fired this frame.
//! 2. [`player_control_system`] — control-clock pass. Reads
//!    `ActorControl` (the brain's intent frame) and the engine
//!    tuning, then runs the control phase. Sets
//!    `SandboxResetThisFrame` if the engine reports a reset.
//! 3. [`player_simulation_system`] — sim-clock pass. Short-circuits
//!    when `SandboxResetThisFrame` is set; otherwise runs the
//!    sim-clock player update.
//!
//! The systems query the 18 player cluster components through
//! [`crate::engine_core::PlayerClusterQueryData`] and call the
//! cluster-native engine entry points
//! (`player_control_phase` / `player_simulation_phase`) directly.
//! The legacy `PlayerMovementAuthority` wrapper + tick-local
//! `ae::Player` scratchpad were deleted 2026-05-28.

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

use crate::engine_core as ae;

/// First system in the player tick chain: clear the per-frame
/// `SandboxResetThisFrame` flag.
///
/// Either [`player_control_system`] or [`player_simulation_system`]
/// may set the flag during this frame. When set, the simulation
/// system short-circuits so the reset's state changes aren't
/// clobbered by a same-frame sim integration. Centralizing the
/// clear here keeps the protocol obvious in the schedule —
/// previously the two systems had no shared state and the
/// short-circuit was an in-function early-return.
pub fn clear_sandbox_reset_this_frame(mut flag: ResMut<SandboxResetThisFrame>) {
    flag.0 = false;
}

/// Control-clock player update. Runs first in the player tick.
///
/// Reads the player's brain output (`ActorControl`) as the authority
/// for the abstract intent verbs (movement, jump, attack, dash,
/// interact, shield) and the raw `PlayerInputFrame` for the player-
/// specific verbs not yet translated by the player brain. The
/// `engine_input_from_actor_control` helper builds the resulting
/// `ae::InputState` for `update_player_control_with_tuning`.
///
/// Sets `SandboxResetThisFrame` when the engine reports a reset so
/// the simulation system can skip this frame.
pub fn player_control_system(
    time: Res<Time>,
    world: Res<GameWorld>,
    editable_tuning: Res<EditableMovementTuning>,
    feel_tuning: Res<SandboxFeelTuning>,
    mut reset_this_frame: ResMut<SandboxResetThisFrame>,
    mut event_writers: SandboxEventWriters,
    mut queues: SandboxQueues,
    mut player_q: Query<
        (
            ae::PlayerClusterQueryData,
            &mut crate::player::PlayerAnimState,
            &mut crate::player::PlayerCombatState,
            &mut crate::player::PlayerInteractionState,
            &mut crate::player::PlayerBlinkCameraState,
            &mut crate::player::ActivePlayerAttack,
            &mut crate::player::PlayerSafetyState,
            &crate::player::PlayerInputFrame,
            &crate::brain::ActorControl,
        ),
        With<crate::player::PlayerEntity>,
    >,
) {
    let Ok((
        mut cluster_item,
        mut anim,
        mut combat,
        mut interaction,
        mut blink_cam,
        mut attack,
        mut safety,
        input,
        actor_control,
    )) = player_q.single_mut()
    else {
        return;
    };
    let tuning = editable_tuning.as_engine();
    let feel = *feel_tuning;
    let frame_dt = time.delta_secs();
    // PlayerInputFrame is still kept on the player entity (story-
    // content systems read it for upstream input edge cases like
    // start-press for pause menu). The player simulation no longer
    // touches it directly — `ActorControl` is the sole input source.
    let _ = input;

    let mut clusters = cluster_item.as_clusters_mut();
    let outcome = player_control_phase(
        actor_control.0,
        &world.0,
        &mut clusters,
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
        &mut anim,
        &mut combat,
        &mut interaction,
        &mut blink_cam,
    );
    if matches!(outcome, PhaseOutcome::Return) {
        reset_this_frame.0 = true;
    }
}

/// Sim-clock player update. Runs after `player_control_system`.
///
/// Short-circuits when `SandboxResetThisFrame` is set so a reset
/// fired in the control phase doesn't get partially overwritten by
/// the sim phase this frame. Otherwise runs the sim-clock player
/// update and updates the flag itself if its own reset fires.
pub fn player_simulation_system(
    time: Res<Time>,
    world: Res<GameWorld>,
    editable_tuning: Res<EditableMovementTuning>,
    feel_tuning: Res<SandboxFeelTuning>,
    mut reset_this_frame: ResMut<SandboxResetThisFrame>,
    mut event_writers: SandboxEventWriters,
    mut queues: SandboxQueues,
    mut shake: ResMut<crate::time::camera_ease::CameraShakeState>,
    mut player_q: Query<
        (
            ae::PlayerClusterQueryData,
            &mut crate::player::PlayerAnimState,
            &mut crate::player::PlayerCombatState,
            &mut crate::player::PlayerInteractionState,
            &mut crate::player::PlayerBlinkCameraState,
            &mut crate::player::PlayerPlatformRideState,
            &mut crate::player::ActivePlayerAttack,
            &mut crate::player::PlayerSafetyState,
            &crate::player::PlayerInputFrame,
            &crate::brain::ActorControl,
        ),
        With<crate::player::PlayerEntity>,
    >,
) {
    if reset_this_frame.0 {
        return;
    }
    let Ok((
        mut cluster_item,
        mut anim,
        mut combat,
        mut interaction,
        mut blink_cam,
        mut ride,
        mut attack,
        mut safety,
        input,
        actor_control,
    )) = player_q.single_mut()
    else {
        return;
    };
    let tuning = editable_tuning.as_engine();
    let feel = *feel_tuning;
    let frame_dt = time.delta_secs();
    // Same polarity flip as the control phase — ActorControl is the
    // sole input source. PlayerInputFrame stays attached for legacy
    // story-content callers but isn't read here.
    let _ = input;

    let mut clusters = cluster_item.as_clusters_mut();
    let outcome = player_simulation_phase(
        actor_control.0,
        &world.0,
        &mut clusters,
        &queues.dev_state,
        &mut queues.sim_state,
        &mut safety,
        &mut queues.moving_platforms.0,
        &mut attack.0,
        &mut event_writers.sfx,
        &mut event_writers.vfx,
        &mut shake,
        tuning,
        feel,
        frame_dt,
        &queues.feature_ecs_overlay,
        &mut queues.reset_room_features,
        &mut anim,
        &mut combat,
        &mut interaction,
        &mut blink_cam,
        &mut ride,
    );
    if matches!(outcome, PhaseOutcome::Return) {
        reset_this_frame.0 = true;
    }
}

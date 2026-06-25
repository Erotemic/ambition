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
//! [`ambition_engine_core::PlayerClusterQueryData`] and call the
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
#[allow(unused_imports)]
use ambition_gameplay_core::schedule::*;

use ambition_engine_core as ae;

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
/// `ae::InputState` for `update_player_control_with_clusters`.
///
/// Sets `SandboxResetThisFrame` when the engine reports a reset so
/// the simulation system can skip this frame.
pub fn player_control_system(
    time: Res<Time>,
    world: Res<GameWorld>,
    editable_tuning: Res<EditableMovementTuning>,
    user_settings: Res<ambition_gameplay_core::persistence::settings::UserSettings>,
    feel_tuning: Res<SandboxFeelTuning>,
    gravity_field: Option<Res<ambition_gameplay_core::physics::GravityField>>,
    mut reset_this_frame: ResMut<SandboxResetThisFrame>,
    mut event_writers: SandboxEventWriters,
    mut queues: SandboxQueues,
    mut player_q: Query<
        (
            Entity,
            ae::PlayerClusterQueryData,
            &mut ambition_gameplay_core::player::PlayerAnimState,
            &mut ambition_gameplay_core::player::PlayerCombatState,
            &mut ambition_gameplay_core::player::PlayerInteractionState,
            &mut ambition_gameplay_core::player::PlayerBlinkCameraState,
            &mut ambition_gameplay_core::player::ActivePlayerAttack,
            &mut ambition_gameplay_core::player::PlayerSafetyState,
            &ambition_gameplay_core::player::PlayerInputFrame,
            &ambition_gameplay_core::brain::ActorControl,
            Option<&ambition_gameplay_core::player::PrimaryPlayer>,
        ),
        With<ambition_gameplay_core::player::PlayerEntity>,
    >,
) {
    let mut tuning = editable_tuning.as_engine();
    // The control phase runs the engine pogo (try_pogo_clusters), which launches
    // OPPOSITE tuning.gravity_dir — so sync it from the live gravity, exactly as
    // the simulation phase does. Without this the pogo used default `(0,1)` and
    // bounced into gravity under a flip.
    let gdir = ambition_gameplay_core::physics::gravity_dir_or_default(gravity_field.as_deref());
    ambition_gameplay_core::physics::apply_gravity_dir(&mut tuning, gdir);
    // The input-frame control preference is a per-frame application alongside the
    // gravity direction (the engine's `as_engine()` baseline is Hybrid; the live
    // gameplay setting wins here). Drives run + descend gate mapping under rotated
    // gravity. Default Hybrid == the historical feel, so normal play is unchanged.
    tuning.input_frame_mode = user_settings.gameplay.input_frame_mode;
    let feel = *feel_tuning;
    let frame_dt = time.delta_secs();

    // Iterate EVERY player-bodied entity (primary + brain-driven clones): each runs
    // the SAME per-entity control core, driven by its own `ActorControl`. The
    // world-global reset is gated to the primary via `is_primary`.
    for (
        player_entity,
        mut cluster_item,
        mut anim,
        mut combat,
        mut interaction,
        mut blink_cam,
        mut attack,
        mut safety,
        input,
        actor_control,
        primary,
    ) in &mut player_q
    {
        // PlayerInputFrame is kept on the entity for story-content edge cases; the
        // simulation reads `ActorControl` as the sole input source.
        let _ = input;
        let is_primary = primary.is_some();
        let mut clusters = cluster_item.as_clusters_mut();
        let outcome = player_control_phase(
            player_entity,
            actor_control.0,
            &world.0,
            &mut clusters,
            &mut queues.sim_state,
            &mut queues.clock,
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
            &mut queues.hit_events,
            &mut anim,
            &mut combat,
            &mut interaction,
            &mut blink_cam,
            is_primary,
        );
        if is_primary && matches!(outcome, PhaseOutcome::Return) {
            reset_this_frame.0 = true;
        }
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
    user_settings: Res<ambition_gameplay_core::persistence::settings::UserSettings>,
    feel_tuning: Res<SandboxFeelTuning>,
    mut reset_this_frame: ResMut<SandboxResetThisFrame>,
    mut event_writers: SandboxEventWriters,
    mut queues: SandboxQueues,
    mut shake: ResMut<ambition_gameplay_core::time::camera_ease::CameraShakeState>,
    gravity_field: Option<Res<ambition_gameplay_core::physics::GravityField>>,
    mut player_q: Query<
        (
            ae::PlayerClusterQueryData,
            &mut ambition_gameplay_core::player::PlayerAnimState,
            &mut ambition_gameplay_core::player::PlayerCombatState,
            &mut ambition_gameplay_core::player::PlayerInteractionState,
            &mut ambition_gameplay_core::player::PlayerBlinkCameraState,
            &mut ambition_gameplay_core::player::ActivePlayerAttack,
            &mut ambition_gameplay_core::player::PlayerSafetyState,
            &ambition_gameplay_core::player::PlayerInputFrame,
            &ambition_gameplay_core::brain::ActorControl,
            Option<&ambition_gameplay_core::player::PrimaryPlayer>,
        ),
        With<ambition_gameplay_core::player::PlayerEntity>,
    >,
) {
    if reset_this_frame.0 {
        return;
    }
    let mut tuning = editable_tuning.as_engine();
    // Cardinal gravity DIRECTION from the world GravityField (the gravity-flip
    // switch / gravity rooms / wall-walking zones). Snapped to a cardinal unit
    // vector so the AABB collision stays axis-aligned. The player movement model
    // is gravity-direction-relative (`gravity_dir`); `gravity_sign` is kept in
    // sync for the down/up case (the legacy Y-only scalar).
    let gdir = ambition_gameplay_core::physics::gravity_dir_or_default(gravity_field.as_deref());
    ambition_gameplay_core::physics::apply_gravity_dir(&mut tuning, gdir);
    // The input-frame control preference is a per-frame application alongside the
    // gravity direction (the engine's `as_engine()` baseline is Hybrid; the live
    // gameplay setting wins here). Drives run + descend gate mapping under rotated
    // gravity. Default Hybrid == the historical feel, so normal play is unchanged.
    tuning.input_frame_mode = user_settings.gameplay.input_frame_mode;
    let feel = *feel_tuning;
    let frame_dt = time.delta_secs();

    // Iterate EVERY player-bodied entity (primary + brain-driven clones): the SAME
    // per-entity simulation core, driven by each body's own `ActorControl`. The
    // camera shake + world-global sandbox reset are gated to the primary via
    // `is_primary`. Platforms are advanced once per frame by
    // `advance_moving_platforms` ahead of this system.
    for (
        mut cluster_item,
        mut anim,
        mut combat,
        mut interaction,
        mut blink_cam,
        mut attack,
        mut safety,
        input,
        actor_control,
        primary,
    ) in &mut player_q
    {
        let _ = input; // ActorControl is the sole input source.
        let is_primary = primary.is_some();
        let mut clusters = cluster_item.as_clusters_mut();
        let outcome = player_simulation_phase(
            actor_control.0,
            &world.0,
            &mut clusters,
            &queues.dev_state,
            &mut queues.sim_state,
            &mut queues.clock,
            &mut safety,
            &queues.moving_platforms.0,
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
            is_primary,
        );
        if is_primary && matches!(outcome, PhaseOutcome::Return) {
            reset_this_frame.0 = true;
        }
    }
}

/// Advance the world's moving platforms ONCE per frame, ahead of the player tick
/// and the actor ticks, so every body (player, clone, enemy, slug) rides this
/// frame's platform positions. Peeled out of the per-entity player simulation so it
/// can't multiply when that loop iterates multiple player bodies. Uses the PRIMARY
/// player's hitstop for `sim_dt` (so platforms freeze during the player's hitstop,
/// exactly as before).
pub fn advance_moving_platforms(
    time: Res<Time>,
    clock: Res<ambition_gameplay_core::time::clock_state::ClockState>,
    reset_this_frame: Res<SandboxResetThisFrame>,
    mut platforms: ResMut<ambition_gameplay_core::MovingPlatformSet>,
    primary_combat: Query<
        &ambition_gameplay_core::player::PlayerCombatState,
        ambition_gameplay_core::player::PrimaryPlayerOnly,
    >,
) {
    if reset_this_frame.0 {
        return;
    }
    let Ok(combat) = primary_combat.single() else {
        return;
    };
    let sim_dt = sandbox_dt(combat.hitstop_timer, clock.time_scale, time.delta_secs());
    for platform in platforms.0.iter_mut() {
        platform.update(sim_dt);
    }
}

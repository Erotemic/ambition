//! Per-frame player body pipeline — the home/player body decomposed into the SAME
//! phases as actors (`tick_actor_brains` / `integrate_actor_bodies` /
//! `sync_actor_read_model`), not a separate gameplay tick.
//!
//! - [`player_body_tick`] — MOVEMENT phase. Reads `ActorControl` (the brain's
//!   intent frame) + engine tuning and runs [`player_body_phase`]: the combined
//!   control+simulation body tick through `ae::update_body_with_tuning_clusters` —
//!   the LITERAL same engine entry a brain-driven actor uses — plus the player
//!   respawn POLICY hook. Writes the [`PlayerBodyFrameOutput`] hand-off. The
//!   two-clock split (responsive aim during precision-blink bullet-time) is carried
//!   entirely by `InputState::control_dt`, an input affordance.
//! - [`sync_player_presentation`] — PRESENTATION phase (separate scheduled system,
//!   mirroring `sync_actor_read_model`). Reads the hand-off and emits screen shake /
//!   landing SFX / per-op anim/SFX/VFX. Moves no body.
//! - [`advance_moving_platforms`] — advances platforms once, ahead of every body.
//!
//! The system queries the 18 body cluster components through
//! [`ambition_engine_core::BodyClusterQueryData`] — the exact aggregate the actor
//! path borrows — so player and actor bodies integrate through one function.

use bevy::prelude::*;

use ambition_engine_core as ae;
use ambition_gameplay_core::dev::dev_tools::EditableMovementTuning;
use ambition_gameplay_core::time::feel::SandboxFeelTuning;
use ambition_gameplay_core::RoomGeometry;

use super::feedback::{SandboxEventWriters, SandboxQueues};
use super::phases::{
    player_body_phase, sync_player_presentation as sync_player_presentation_phase,
};
use super::world_flow::sandbox_dt;

/// Movement→presentation hand-off for the player body, written by the movement
/// phase (`player_body_tick` → `player_body_phase`) and read by the presentation
/// phase (`sync_player_presentation`). Carries this frame's movement `FrameEvents`
/// plus the landing inputs the screen-shake reads, so presentation is a separate
/// scheduled phase (mirroring the actor `sync_actor_read_model`) rather than fused
/// into movement. A required component of every player body.
#[derive(Component, Default)]
pub struct PlayerBodyFrameOutput {
    /// The movement tick's events (jump/dash/blink ops, blink endpoints, …).
    pub events: ae::FrameEvents,
    /// Grounded state ENTERING the movement tick (for the hard-fall shake edge).
    pub was_grounded: bool,
    /// Vertical velocity entering the tick (hard-fall shake magnitude).
    pub pre_sim_vy: f32,
    /// The movement phase fully reset the body this frame (primary death/hazard);
    /// presentation is skipped because `reset_sandbox` already reset its state.
    pub full_reset: bool,
}

/// The unified player tick. Runs after the brain-driver systems (which populate
/// each player body's `ActorControl` in `SandboxSet::PlayerInput`) and after
/// `advance_moving_platforms` (so the body reads this frame's platform
/// positions, exactly like the actor tick does).
///
/// Iterates EVERY player-bodied entity (the primary + any brain-driven clone):
/// each runs the SAME per-entity body core, driven by its own `ActorControl`.
/// The world-global reset + camera shake are gated to the primary inside
/// [`player_body_phase`] via `is_primary`.
pub fn player_body_tick(
    time: Res<Time>,
    world: Res<RoomGeometry>,
    editable_tuning: Res<EditableMovementTuning>,
    user_settings: Res<ambition_gameplay_core::persistence::settings::UserSettings>,
    feel_tuning: Res<SandboxFeelTuning>,
    gravity_field: Option<Res<ambition_gameplay_core::physics::GravityField>>,
    mut event_writers: SandboxEventWriters,
    mut queues: SandboxQueues,
    mut player_q: Query<
        (
            ae::BodyClusterQueryData,
            &mut ambition_gameplay_core::player::PlayerAnimState,
            &mut ambition_gameplay_core::actor::BodyCombat,
            &mut ambition_gameplay_core::player::PlayerInteractionState,
            &mut ambition_gameplay_core::player::PlayerBlinkCameraState,
            &mut ambition_gameplay_core::player::BodyMelee,
            &mut ambition_gameplay_core::player::PlayerSafetyState,
            &ambition_gameplay_core::player::PlayerInputFrame,
            &ambition_characters::brain::ActorControl,
            &mut PlayerBodyFrameOutput,
            Option<&ambition_gameplay_core::actor::PrimaryPlayer>,
        ),
        With<ambition_gameplay_core::actor::PlayerEntity>,
    >,
) {
    let mut tuning = editable_tuning.as_engine();
    // Cardinal gravity DIRECTION from the world GravityField (gravity-flip switch /
    // gravity rooms / wall-walking zones), snapped to a cardinal unit vector so AABB
    // collision stays axis-aligned. The body movement model is gravity-direction-
    // relative; this drives both the control pogo and the simulation gravity.
    let gdir = ambition_gameplay_core::physics::gravity_dir_or_default(gravity_field.as_deref());
    ambition_gameplay_core::physics::apply_gravity_dir(&mut tuning, gdir);
    // The input-frame control preference is applied per-frame alongside the gravity
    // direction (the engine `as_engine()` baseline is Hybrid; the live gameplay
    // setting wins here). Default Hybrid == the historical feel, so normal play is
    // unchanged.
    tuning.movement_frame_mode = user_settings.gameplay.movement_frame_mode;
    let feel = *feel_tuning;
    let frame_dt = time.delta_secs();

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
        mut frame_out,
        primary,
    ) in &mut player_q
    {
        let _ = input; // PlayerInputFrame is kept for story-content edge cases; ActorControl is the sole input source.
        let is_primary = primary.is_some();
        let mut clusters = cluster_item.as_clusters_mut();
        player_body_phase(
            actor_control.0,
            &world.0,
            &mut clusters,
            &mut queues.sim_state,
            &mut queues.clock,
            &mut safety,
            &queues.moving_platforms.0,
            &mut attack.swing,
            &mut event_writers.sfx,
            &mut event_writers.vfx,
            &mut frame_out,
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
    }
}

/// PHASE — sync player presentation. The presentation half of the player body
/// tick, a SEPARATE scheduled system from the movement phase (`player_body_tick`),
/// mirroring the actor `sync_actor_read_model` split. Reads the
/// `PlayerBodyFrameOutput` the movement phase wrote and emits the screen-facing
/// feedback (hard-fall shake + landing SFX, and the per-op anim/SFX/VFX) via
/// `sync_player_presentation` in `phases`. Moves no body, resolves no physics.
pub fn sync_player_presentation(
    mut event_writers: SandboxEventWriters,
    mut shake: ResMut<ambition_gameplay_core::time::camera_ease::CameraShakeState>,
    mut player_q: Query<
        (
            ae::BodyClusterQueryData,
            &mut ambition_gameplay_core::player::PlayerAnimState,
            &mut ambition_gameplay_core::actor::BodyCombat,
            &mut ambition_gameplay_core::player::PlayerBlinkCameraState,
            &PlayerBodyFrameOutput,
            Option<&ambition_gameplay_core::actor::PrimaryPlayer>,
        ),
        With<ambition_gameplay_core::actor::PlayerEntity>,
    >,
) {
    for (mut cluster_item, mut anim, mut combat, mut blink_cam, frame_out, primary) in &mut player_q
    {
        let is_primary = primary.is_some();
        let clusters = cluster_item.as_clusters_mut();
        sync_player_presentation_phase(
            frame_out,
            &clusters,
            &mut combat,
            &mut blink_cam,
            &mut anim,
            &mut event_writers.sfx,
            &mut event_writers.vfx,
            &mut shake,
            is_primary,
        );
    }
}

/// Advance the world's moving platforms ONCE per frame, ahead of the player tick
/// and the actor ticks, so every body (player, clone, enemy, slug) rides this
/// frame's platform positions. Peeled out of the per-entity player simulation so it
/// can't multiply when that loop iterates multiple player bodies. Uses the PRIMARY
/// player's hitstop for `sim_dt` (so platforms freeze during the player's hitstop).
pub fn advance_moving_platforms(
    time: Res<Time>,
    clock: Res<ambition_gameplay_core::time::clock_state::ClockState>,
    mut platforms: ResMut<ambition_gameplay_core::MovingPlatformSet>,
    primary_combat: Query<
        &ambition_gameplay_core::actor::BodyCombat,
        ambition_gameplay_core::actor::PrimaryPlayerOnly,
    >,
) {
    let Ok(combat) = primary_combat.single() else {
        return;
    };
    let sim_dt = sandbox_dt(combat.hitstop_timer, clock.time_scale, time.delta_secs());
    for platform in platforms.0.iter_mut() {
        platform.update(sim_dt);
    }
}

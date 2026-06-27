//! Core simulation Bevy systems.
//!
//! Each function is a narrow query/resource system registered in the
//! [`SandboxSet::CoreSimulation`] chain configured by
//! [`super::schedule::configure_sandbox_sets`]. Cross-set ordering lives in the
//! schedule; intra-set ordering is expressed by `.chain()` where registered.

use ambition_engine_core as ae;
use bevy::prelude::*;

use ambition_gameplay_core::audio::SfxMessage;
use ambition_gameplay_core::dev::dev_tools::{self, EditableAbilitySet, EditableMovementTuning};
use ambition_gameplay_core::features;
use ambition_input::ControlFrame;
use ambition_gameplay_core::time::feel::SandboxFeelTuning;
use ambition_gameplay_core::{RoomGeometry, SandboxSimState};
use ambition_render::fx::VfxMessage;

/// Push live dev-tools ability/tuning edits onto the authoritative player.
/// Runs even while gameplay is suspended so the F3 inspector remains responsive.
pub fn sync_live_player_dev_edits_system(
    editable_tuning: Res<EditableMovementTuning>,
    editable_abilities: Res<EditableAbilitySet>,
    mut player_q: Query<
        (
            &mut ambition_gameplay_core::player::PlayerAbilities,
            &mut ambition_gameplay_core::player::PlayerFlightState,
            &mut ambition_gameplay_core::player::PlayerBlinkState,
            &mut ambition_gameplay_core::player::PlayerDashState,
            &mut ambition_gameplay_core::player::PlayerJumpState,
        ),
        ambition_gameplay_core::player::PrimaryPlayerOnly,
    >,
) {
    let Ok((mut abilities, mut flight, mut blink, mut dash, mut jump)) = player_q.single_mut()
    else {
        return;
    };
    dev_tools::sync_live_ability_edits_clusters(
        &mut abilities,
        &mut flight,
        &mut blink,
        &mut dash,
        &mut jump,
        editable_abilities.as_engine(),
        editable_tuning.as_engine(),
    );
}

/// While gameplay is suspended, force both live and requested sim-clock scale to
/// zero so presentation animations freeze and the smoother cannot ramp up next
/// frame. Gameplay mode leaves scale control to the normal time-control pipeline.
pub fn apply_suspended_time_scale_system(
    mut clock: ResMut<ambition_gameplay_core::time::clock_state::ClockState>,
    mut target: ResMut<ambition_gameplay_core::time::time_control::RequestedClockScale>,
) {
    clock.time_scale = 0.0;
    target.sim_clock = 0.0;
}

/// Tick per-frame gameplay timers and detect double-tap gestures.
///
/// Registered with `run_if(gameplay_allowed)` so it only runs in
/// `GameMode::Playing`. Writes `fast_fall_pressed` back to
/// `Res<ControlFrame>` so the player tick sees the updated flag.
/// Sets `PlayerInteractionState::double_tap_up_pending` so the
/// subsequent interaction phase inside the player tick can activate
/// doors/NPCs.
pub fn input_timer_system(
    time: Res<Time>,
    feel_tuning: Res<SandboxFeelTuning>,
    gravity_field: Option<Res<ambition_gameplay_core::physics::GravityField>>,
    user_settings: Option<Res<ambition_gameplay_core::persistence::settings::UserSettings>>,
    mut sim_state: ResMut<ambition_gameplay_core::SandboxSimState>,
    mut control_frame: ResMut<ControlFrame>,
    mut player_q: Query<
        (
            &mut ambition_gameplay_core::actor::BodyCombat,
            &mut ambition_gameplay_core::player::PlayerInteractionState,
        ),
        ambition_gameplay_core::player::PrimaryPlayerOnly,
    >,
) {
    let frame_dt = time.delta_secs();
    let feel = *feel_tuning;
    let Ok((mut combat, mut interaction)) = player_q.single_mut() else {
        return;
    };
    sim_state.room_transition_cooldown = (sim_state.room_transition_cooldown - frame_dt).max(0.0);
    combat.damage_invuln_timer = (combat.damage_invuln_timer - frame_dt).max(0.0);
    combat.hitstun_timer = (combat.hitstun_timer - frame_dt).max(0.0);
    combat.recoil_lock_timer = (combat.recoil_lock_timer - frame_dt).max(0.0);
    // Fast-fall = double-tap local-down for the controlled body. Raw cardinal
    // edges are resolved through the same input mapping policy as locomotion,
    // so ScreenDirected sideways gravity can map raw-right/raw-left into local
    // down/up without bespoke cases here.
    let gravity_dir =
        ambition_gameplay_core::physics::gravity_dir_or_default(gravity_field.as_deref());
    let movement_mode = user_settings
        .as_deref()
        .map_or(ae::InputFrameMode::DEFAULT_MOVEMENT, |s| s.gameplay.movement_frame_mode);
    let resolved = ae::AccelerationFrame::new(gravity_dir).resolve_control(
        movement_mode,
        control_frame.axis_x,
        control_frame.axis_y,
    );
    let raw_edges = control_frame.raw_direction_edges();
    let descend_pressed = resolved.local_down_pressed(raw_edges);
    let ascend_pressed = resolved.local_up_pressed(raw_edges);
    let double_tap_down =
        interaction.register_down_tap(descend_pressed, frame_dt, feel.down_double_tap_window);
    control_frame.fast_fall_pressed = double_tap_down;
    if double_tap_down {
        interaction.double_tap_down_pending = true;
    }
    let door_double_tap_up =
        interaction.register_up_tap(ascend_pressed, frame_dt, feel.up_double_tap_window);
    if door_double_tap_up {
        interaction.double_tap_up_pending = true;
    }
    combat.hitstop_timer = (combat.hitstop_timer - frame_dt).max(0.0);
}

/// Fold the explicit `Interact` action together with the
/// `double_tap_up_pending` gesture, gate the result on hit-stun, and
/// advance the per-frame interact buffer on
/// [`ambition_gameplay_core::player::PlayerInteractionState`].
///
/// Downstream consumers read the buffered signal from
/// `PlayerInteractionState::buffered()`. Gated by `gameplay_allowed` so the
/// buffer does not tick down while paused, in dialogue, or mid-cutscene.
///
/// Ordering: must run after `input_timer_system` (which decrements
/// `combat.hitstun_timer` and sets `double_tap_up_pending` from
/// `register_up_tap`) and before `detect_room_transition_system`
/// (which consumes the buffered signal post-player-tick).
pub fn interaction_input_system(
    time: Res<Time>,
    feel_tuning: Res<SandboxFeelTuning>,
    control_frame: Res<ControlFrame>,
    mut player_q: Query<
        (
            &ambition_gameplay_core::actor::BodyCombat,
            &mut ambition_gameplay_core::player::PlayerInteractionState,
        ),
        ambition_gameplay_core::player::PrimaryPlayerOnly,
    >,
) {
    let frame_dt = time.delta_secs();
    let feel = *feel_tuning;
    let Ok((combat, mut interaction)) = player_q.single_mut() else {
        return;
    };
    let door_double_tap_up = std::mem::take(&mut interaction.double_tap_up_pending);
    // Reads `Res<ControlFrame>` directly rather than `PlayerInputFrame`
    // because this system runs mid-input-chain — `input_timer_system`
    // writes `fast_fall_pressed` to the resource and the per-player
    // `sync_local_player_input_frame` mirror only fires at the END of
    // the chain. Switching to `PlayerInputFrame` here would read the
    // previous frame's snapshot.
    let raw_interact_pressed = if combat.hitstun_timer > 0.0 {
        false
    } else {
        control_frame.interact_pressed || door_double_tap_up
    };
    let _live =
        interaction.buffered_interact(raw_interact_pressed, frame_dt, feel.interaction_buffer_time);
}

/// Detect a player-pressed reset (the Reset button / `controls.reset_pressed`)
/// and execute the full sandbox reset before the rest of the gameplay
/// chain runs.
///
/// Handles input-driven resets before the rest of gameplay. Engine-driven resets
/// still finish in their player-control/simulation call sites because those paths
/// have already mutated the player and must complete cleanup immediately.
///
/// This system clears `ControlFrame::reset_pressed` after handling it
/// so the engine path inside `update_player_control_with_clusters`
/// does not re-trigger a reset on the same frame. Writes sfx/vfx directly to
/// `MessageWriter`s via local Vec buffers (the engine helper
/// `reset_sandbox` still uses Vec push semantics).
///
/// Gated by `gameplay_allowed`: paused / dialogue modes don't process
/// reset input.
pub fn apply_player_reset_input_system(
    mut control_frame: ResMut<ControlFrame>,
    world: Res<RoomGeometry>,
    editable_tuning: Res<EditableMovementTuning>,
    feel_tuning: Res<SandboxFeelTuning>,
    mut sim_state: ResMut<SandboxSimState>,
    mut clock: ResMut<ambition_gameplay_core::time::clock_state::ClockState>,
    mut reset_room_features: MessageWriter<features::ResetRoomFeaturesEvent>,
    mut sfx_writer: MessageWriter<SfxMessage>,
    mut vfx_writer: MessageWriter<VfxMessage>,
    mut player_q: Query<
        (
            ae::PlayerClusterQueryData,
            &mut ambition_gameplay_core::player::PlayerAnimState,
            &mut ambition_gameplay_core::actor::BodyCombat,
            &mut ambition_gameplay_core::player::PlayerInteractionState,
            &mut ambition_gameplay_core::player::PlayerBlinkCameraState,
            &mut ambition_gameplay_core::player::ActivePlayerAttack,
            &mut ambition_gameplay_core::player::PlayerSafetyState,
        ),
        ambition_gameplay_core::player::PrimaryPlayerOnly,
    >,
) {
    if !control_frame.reset_pressed {
        return;
    }
    let Ok((
        mut cluster_item,
        mut anim,
        mut combat,
        mut interaction,
        mut blink_cam,
        mut attack,
        mut safety,
    )) = player_q.single_mut()
    else {
        return;
    };
    // Clear the press immediately so the inline engine update in
    // `player_control_phase` doesn't trigger a redundant `player.reset_to`
    // followed by another sandbox-side reset later this frame.
    control_frame.reset_pressed = false;

    let mut clusters = cluster_item.as_clusters_mut();
    super::world_flow::reset_sandbox(
        &world.0,
        &mut sfx_writer,
        &mut vfx_writer,
        &mut clusters,
        &mut sim_state,
        &mut clock,
        &mut safety,
        &mut attack.0,
        &mut anim,
        &mut combat,
        &mut interaction,
        &mut blink_cam,
        editable_tuning.as_engine(),
        *feel_tuning,
    );
    reset_room_features.write(features::ResetRoomFeaturesEvent {
        reason: features::RoomResetReason::Manual,
    });
}

/// Replay the cut-rope boss room from a Yarn/dialogue command.
///
/// This intentionally mirrors `apply_player_reset_input_system` instead of
/// driving `ControlFrame::reset_pressed`: the command can run while gameplay
/// input is suspended by dialogue, so relying on the input frame would make the
/// reset timing depend on UI/game-mode scheduling.
pub fn apply_cut_rope_room_replay_request_system(
    mut replay_requests: MessageReader<ambition_content::bosses::CutRopeRoomReplayRequested>,
    world: Res<RoomGeometry>,
    editable_tuning: Res<EditableMovementTuning>,
    feel_tuning: Res<SandboxFeelTuning>,
    mut sim_state: ResMut<SandboxSimState>,
    mut clock: ResMut<ambition_gameplay_core::time::clock_state::ClockState>,
    boss_registry: Res<ambition_gameplay_core::boss_encounter::BossEncounterRegistry>,
    mut save: Option<ResMut<ambition_gameplay_core::persistence::save::SandboxSave>>,
    mut boss_music: Option<ResMut<ambition_gameplay_core::encounter::BossEncounterMusicRequest>>,
    // Cut-rope boss placements in the room — R4 keys "cleared" by placement
    // (`config.id`), so the replay clears those keys (the respawned boss carries
    // the same LDtk id).
    cut_rope_bosses: Query<&ambition_gameplay_core::combat::boss_clusters::BossConfig>,
    mut reset_room_features: MessageWriter<features::ResetRoomFeaturesEvent>,
    mut sfx_writer: MessageWriter<SfxMessage>,
    mut vfx_writer: MessageWriter<VfxMessage>,
    mut player_q: Query<
        (
            ae::PlayerClusterQueryData,
            &mut ambition_gameplay_core::player::PlayerAnimState,
            &mut ambition_gameplay_core::actor::BodyCombat,
            &mut ambition_gameplay_core::player::PlayerInteractionState,
            &mut ambition_gameplay_core::player::PlayerBlinkCameraState,
            &mut ambition_gameplay_core::player::ActivePlayerAttack,
            &mut ambition_gameplay_core::player::PlayerSafetyState,
        ),
        ambition_gameplay_core::player::PrimaryPlayerOnly,
    >,
) {
    if replay_requests.read().count() == 0 {
        return;
    }
    let cut_rope_placements: Vec<String> = cut_rope_bosses
        .iter()
        .filter(|config| ambition_content::bosses::is_cut_rope_boss(&config.behavior.id))
        .map(|config| config.id.clone())
        .collect();
    ambition_content::bosses::reset_cut_rope_boss_attempt(
        &boss_registry,
        save.as_deref_mut(),
        boss_music.as_deref_mut(),
        &cut_rope_placements,
    );

    let Ok((
        mut cluster_item,
        mut anim,
        mut combat,
        mut interaction,
        mut blink_cam,
        mut attack,
        mut safety,
    )) = player_q.single_mut()
    else {
        reset_room_features.write(features::ResetRoomFeaturesEvent {
            reason: features::RoomResetReason::Manual,
        });
        return;
    };

    let mut clusters = cluster_item.as_clusters_mut();
    super::world_flow::reset_sandbox(
        &world.0,
        &mut sfx_writer,
        &mut vfx_writer,
        &mut clusters,
        &mut sim_state,
        &mut clock,
        &mut safety,
        &mut attack.0,
        &mut anim,
        &mut combat,
        &mut interaction,
        &mut blink_cam,
        editable_tuning.as_engine(),
        *feel_tuning,
    );
    reset_room_features.write(features::ResetRoomFeaturesEvent {
        reason: features::RoomResetReason::Manual,
    });
}

/// Decay presentation-only animation and flash timers.
///
/// Runs every frame (including paused/dialogue) so visual flash and
/// animation pose timers wind down continuously, not just during
/// gameplay. Owns: real-time decay of `hit_flash`, `preset_flash`,
/// `slash_anim_timer`, `blink_in_timer`, `camera_snap_timer`. New
/// presentation-flash timers belong here; gameplay timers belong in
/// `input_timer_system`.
pub fn cleanup_timers_system(
    time: Res<Time>,
    mut dev_state: ResMut<ambition_gameplay_core::SandboxDevState>,
    mut player_q: Query<
        (
            &ambition_gameplay_core::player::BodyKinematics,
            &ambition_gameplay_core::player::PlayerGroundState,
            &ambition_gameplay_core::player::PlayerDashState,
            &mut ambition_gameplay_core::player::PlayerAnimState,
            &mut ambition_gameplay_core::actor::BodyCombat,
            &mut ambition_gameplay_core::player::PlayerBlinkCameraState,
        ),
        ambition_gameplay_core::player::PrimaryPlayerOnly,
    >,
) {
    let frame_dt = time.delta_secs();
    let Ok((kinematics, ground, dash, mut anim, mut combat, mut blink_cam)) = player_q.single_mut()
    else {
        return;
    };
    combat.hit_flash = (combat.hit_flash - frame_dt).max(0.0);
    dev_state.preset_flash = (dev_state.preset_flash - frame_dt).max(0.0);
    anim.slash_anim_timer = (anim.slash_anim_timer - frame_dt).max(0.0);
    anim.shoot_anim_timer = (anim.shoot_anim_timer - frame_dt).max(0.0);
    anim.wall_jump_anim_timer = (anim.wall_jump_anim_timer - frame_dt).max(0.0);
    anim.interact_anim_timer = (anim.interact_anim_timer - frame_dt).max(0.0);
    blink_cam.blink_in_timer = (blink_cam.blink_in_timer - frame_dt).max(0.0);
    blink_cam.camera_snap_timer = (blink_cam.camera_snap_timer - frame_dt).max(0.0);
    update_anim_signal_timers(
        ground.on_ground,
        kinematics.vel.y,
        dash.timer,
        &mut anim,
        frame_dt,
    );
}

/// Drive the presentation-only landing + dash-startup timers and capture
/// the per-frame state needed for edge detection.
///
/// The sprite picker (`pick_player_anim`) reads these from the
/// `PlayerAnimState` component. Detection lives here so all presentation
/// timers decay in one phase and so the "previous frame" snapshot is
/// the one immediately before the next gameplay tick.
fn update_anim_signal_timers(
    on_ground: bool,
    vel_y: f32,
    dash_timer: f32,
    anim: &mut ambition_gameplay_core::player::PlayerAnimState,
    frame_dt: f32,
) {
    // Hard-landing threshold: pre-touchdown downward speed (px/s) above
    // which we play `LandHard` instead of `LandRecovery`. Tuned by the
    // sandbox's terminal-fall feel; raise if normal jump landings start
    // reading as hard impacts.
    const HARD_LAND_SPEED: f32 = 520.0;
    // Time the landing pose holds after touchdown.
    const LAND_HARD_HOLD_SECS: f32 = 0.34;
    const LAND_SOFT_HOLD_SECS: f32 = 0.16;
    // Brief pre-roll for the dash startup pose. Falls below the dash's
    // own duration so the streaking dash row still gets airtime.
    const DASH_STARTUP_SECS: f32 = 0.05;

    // Landing edge: airborne last frame, grounded this frame.
    if on_ground && !anim.anim_prev_on_ground {
        let impact_speed = anim.anim_prev_vel_y;
        let hard = impact_speed >= HARD_LAND_SPEED;
        anim.land_anim_hard = hard;
        anim.land_anim_timer = if hard {
            LAND_HARD_HOLD_SECS
        } else {
            LAND_SOFT_HOLD_SECS
        };
    } else if !on_ground {
        // Stay airborne: the landing pose only plays on the ground.
        anim.land_anim_timer = 0.0;
    } else {
        anim.land_anim_timer = (anim.land_anim_timer - frame_dt).max(0.0);
    }

    // Dash rising edge: previous frame had no dash, this frame has one.
    if dash_timer > 0.0 && anim.anim_prev_dash_timer <= 0.0 {
        anim.dash_startup_timer = DASH_STARTUP_SECS;
    } else {
        anim.dash_startup_timer = (anim.dash_startup_timer - frame_dt).max(0.0);
    }

    // Snapshot for the next frame. Sample vel.y BEFORE any further
    // physics so the landing detector sees the pre-touchdown speed
    // (engine zeroes vertical velocity on contact); this system runs
    // at the end of the gameplay loop, so the player state here is
    // already post-integration but still reflects the speed that produced
    // this frame's `on_ground`.
    anim.anim_prev_on_ground = on_ground;
    anim.anim_prev_vel_y = vel_y;
    anim.anim_prev_dash_timer = dash_timer;
}

#[cfg(test)]
mod suspended_time_tests {
    use super::*;
    use ambition_gameplay_core::game_mode::{gameplay_suspended, GameMode};
    use ambition_gameplay_core::time::time_control::RequestedClockScale;
    use ambition_gameplay_core::WorldTime;
    use bevy::state::app::StatesPlugin;

    /// Regression: when gameplay is suspended (pause / dialogue /
    /// cutscene / room transition), `apply_suspended_time_scale_system`
    /// must zero both `SandboxSimState::time_scale` AND
    /// `RequestedClockScale::sim_clock` BEFORE `refresh_world_time`
    /// snapshots them — otherwise `WorldTime::scaled_dt` stays
    /// non-zero on the first suspended frame and any presentation
    /// system multiplying by it ticks one extra frame after pause
    /// lands.
    #[test]
    fn suspended_frame_zeros_world_time_scaled_dt() {
        let mut app = App::new();
        app.add_plugins(StatesPlugin);
        app.insert_state(GameMode::Paused);
        app.insert_resource(ambition_gameplay_core::time::clock_state::ClockState {
            time_scale: 1.0,
        });
        app.insert_resource(RequestedClockScale {
            sim_clock: 1.0,
            ..Default::default()
        });
        app.insert_resource(WorldTime {
            raw_dt: 0.016,
            scaled_dt: 0.016,
        });
        app.insert_resource(Time::<()>::default());

        // Mirror the new ordering from `register_player_input_systems`:
        // suspended-zero FIRST, then refresh.
        app.add_systems(
            Update,
            (
                apply_suspended_time_scale_system.run_if(gameplay_suspended),
                ambition_gameplay_core::refresh_world_time,
            )
                .chain(),
        );

        // Pump one wall-clock tick so refresh_world_time has a real dt.
        let frame = std::time::Duration::from_millis(16);
        app.world_mut().resource_mut::<Time>().advance_by(frame);
        app.update();

        let clock = app
            .world()
            .resource::<ambition_gameplay_core::time::clock_state::ClockState>();
        let target = app.world().resource::<RequestedClockScale>();
        let wt = app.world().resource::<WorldTime>();
        assert_eq!(
            clock.time_scale, 0.0,
            "suspended frame must zero ClockState.time_scale"
        );
        assert_eq!(
            target.sim_clock, 0.0,
            "suspended frame must zero RequestedClockScale.sim_clock"
        );
        assert_eq!(
            wt.scaled_dt, 0.0,
            "suspended frame must zero WorldTime.scaled_dt (refresh_world_time \
             must see the zeroed time_scale, not last frame's 1.0)"
        );
        // wall_dt keeps ticking through pause — that's the contract.
        assert!(
            (wt.wall_dt() - 0.016).abs() < 1e-6,
            "wall clock must keep ticking through pause"
        );
    }

    /// Gameplay-allowed frames take the regular emit → apply → smooth
    /// path; the suspended fallback is short-circuited by `run_if`.
    /// `refresh_world_time` then sees `sim_state.time_scale = 1.0`
    /// (the default) and reports a non-zero `scaled_dt`.
    #[test]
    fn gameplay_frame_preserves_world_time_scaled_dt() {
        let mut app = App::new();
        app.add_plugins(StatesPlugin);
        app.insert_state(GameMode::Playing);
        app.insert_resource(ambition_gameplay_core::time::clock_state::ClockState::default());
        app.insert_resource(RequestedClockScale::default());
        app.insert_resource(WorldTime::default());
        app.insert_resource(Time::<()>::default());

        app.add_systems(
            Update,
            (
                apply_suspended_time_scale_system.run_if(gameplay_suspended),
                ambition_gameplay_core::refresh_world_time,
            )
                .chain(),
        );

        let frame = std::time::Duration::from_millis(16);
        app.world_mut().resource_mut::<Time>().advance_by(frame);
        app.update();

        let wt = app.world().resource::<WorldTime>();
        assert!(
            wt.scaled_dt > 0.0,
            "gameplay frame must produce a non-zero scaled_dt; got {}",
            wt.scaled_dt
        );
    }
}

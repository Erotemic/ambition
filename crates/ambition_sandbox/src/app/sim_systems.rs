//! Bevy systems extracted from `sandbox_update`.
//!
//! Each function here is a real Bevy system with a narrow query/resource
//! signature. They live in the [`SandboxSet::CoreSimulation`] chain
//! configured by [`super::schedule::configure_sandbox_sets`], and their
//! order inside that chain is expressed by the tuple `.chain()` in
//! `add_simulation_plugins` rather than by `.after(name)` on each system.

use ambition_engine as ae;
use bevy::prelude::*;

use crate::feel::SandboxFeelTuning;
use crate::input::ControlFrame;
use crate::SandboxSimState;

/// While gameplay is suspended (paused, dialogue, room transition,
/// cutscene), force `SandboxSimState::time_scale` to 0 so any
/// presentation system that scales an animation by `time_scale * dt`
/// freezes. The previous `mode_gate_phase` did the same thing at the
/// top of `sandbox_update`; now that `sandbox_update` is gated by
/// `run_if(gameplay_allowed)`, this needs to live in its own system
/// that runs only when gameplay is *not* allowed.
///
/// In gameplay mode `update_time_scale` (inside `sandbox_update`'s
/// `player_simulation_phase`) drives `time_scale` from hitstop /
/// slowmo / dev settings, so this system intentionally does nothing
/// when gameplay is allowed.
pub fn apply_suspended_time_scale_system(mut sim_state: ResMut<SandboxSimState>) {
    sim_state.time_scale = 0.0;
}

/// Tick per-frame gameplay timers and detect double-tap gestures.
///
/// Registered with `run_if(gameplay_allowed)` so it only runs in
/// `GameMode::Playing`. Writes `fast_fall_pressed` back to
/// `Res<ControlFrame>` so `sandbox_update` sees the updated flag.
/// Sets `PlayerInteractionState::double_tap_up_pending` so the
/// subsequent interaction phase inside `sandbox_update` can activate
/// doors/NPCs.
pub fn input_timer_system(
    time: Res<Time>,
    feel_tuning: Res<SandboxFeelTuning>,
    mut sim_state: ResMut<crate::SandboxSimState>,
    mut control_frame: ResMut<ControlFrame>,
    mut player_q: Query<
        (
            &mut crate::player::PlayerCombatState,
            &mut crate::player::PlayerInteractionState,
        ),
        With<crate::player::PlayerEntity>,
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
    let double_tap_down = interaction.register_down_tap(
        control_frame.down_pressed,
        frame_dt,
        feel.down_double_tap_window,
    );
    control_frame.fast_fall_pressed = double_tap_down;
    if double_tap_down {
        interaction.double_tap_down_pending = true;
    }
    let door_double_tap_up = interaction.register_up_tap(
        control_frame.up_pressed,
        frame_dt,
        feel.up_double_tap_window,
    );
    if door_double_tap_up {
        interaction.double_tap_up_pending = true;
    }
    combat.hitstop_timer = (combat.hitstop_timer - frame_dt).max(0.0);
}

/// Decay presentation-only animation and flash timers.
///
/// Runs every frame (including paused/dialogue) so visual flash and
/// animation pose timers wind down continuously, not just during
/// gameplay. Owns: real-time decay of `flash_timer`, `preset_flash`,
/// `slash_anim_timer`, `blink_in_timer`, `camera_snap_timer`. New
/// presentation-flash timers belong here; gameplay timers belong in
/// `input_timer_system`.
pub fn cleanup_timers_system(
    time: Res<Time>,
    mut dev_state: ResMut<crate::SandboxDevState>,
    mut player_q: Query<
        (
            &crate::player::PlayerMovementAuthority,
            &mut crate::player::PlayerAnimState,
            &mut crate::player::PlayerCombatState,
            &mut crate::player::PlayerBlinkCameraState,
        ),
        With<crate::player::PlayerEntity>,
    >,
) {
    let frame_dt = time.delta_secs();
    let Ok((authority, mut anim, mut combat, mut blink_cam)) = player_q.single_mut() else {
        return;
    };
    let player = &authority.player;
    combat.flash_timer = (combat.flash_timer - frame_dt).max(0.0);
    dev_state.preset_flash = (dev_state.preset_flash - frame_dt).max(0.0);
    anim.slash_anim_timer = (anim.slash_anim_timer - frame_dt).max(0.0);
    blink_cam.blink_in_timer = (blink_cam.blink_in_timer - frame_dt).max(0.0);
    blink_cam.camera_snap_timer = (blink_cam.camera_snap_timer - frame_dt).max(0.0);
    update_anim_signal_timers(player, &mut anim, frame_dt);
}

/// Drive the presentation-only landing + dash-startup timers and capture
/// the per-frame state needed for edge detection.
///
/// The sprite picker (`pick_player_anim`) reads these from the
/// `PlayerAnimState` component. Detection lives here so all presentation
/// timers decay in one phase and so the "previous frame" snapshot is
/// the one immediately before the next gameplay tick.
fn update_anim_signal_timers(
    player: &ae::Player,
    anim: &mut crate::player::PlayerAnimState,
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

    let on_ground = player.on_ground;
    let dash_timer = player.dash_timer;

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
    anim.anim_prev_vel_y = player.vel.y;
    anim.anim_prev_dash_timer = dash_timer;
}

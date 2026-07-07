//! Per-frame player input/timer systems.
//!
//! These publish the primary controller's slot gestures from the local device
//! and tick the home/player body's own reaction + presentation timers. They are
//! body-generic gameplay-sim logic (no render, no host-only types), so they live
//! beside the player state they mutate; the host schedule (`register_player_input_systems`)
//! owns their ordering + `run_if` gates and references these `pub fn`s.

use ambition_engine_core as ae;
use bevy::prelude::*;

use ambition_input::ControlFrame;

/// Tick per-frame gameplay timers and publish the primary controller's slot
/// gestures from the local device.
///
/// Two concerns, deliberately separated by ownership:
/// - **Home-body reaction timers** (`hitstun` / `hitstop` / `damage-invuln` /
///   `recoil`): the home/player body isn't in the actor tick, so it ticks its OWN
///   reaction timers here. This is the home body's own state, NOT authority over the
///   controlled subject — a possessed actor ticks its own timers in the actor path.
/// - **Slot gestures** (double-tap down/up): published from `Res<ControlFrame>` into
///   `SlotInteractionState` for the primary controller slot. Body mode / interaction
///   consume THAT (keyed by the controlled body's slot), never a per-body component.
///
/// The host registers this with `run_if(gameplay_allowed)` so it only runs in
/// `GameMode::Playing`. Writes `fast_fall_pressed` back to `Res<ControlFrame>`.
pub fn input_timer_system(
    time: Res<Time>,
    feel_tuning: Res<crate::time::feel::SandboxFeelTuning>,
    gravity_field: Option<Res<crate::physics::GravityField>>,
    user_settings: Option<Res<ambition_persistence::settings::UserSettings>>,
    mut sim_state: ResMut<crate::SandboxSimState>,
    mut control_frame: ResMut<ControlFrame>,
    mut slot_gestures: ResMut<crate::player::SlotInteractionState>,
    // Home/player bodies tick their OWN reaction timers here (they aren't in the
    // actor tick). Iterates every player body so a co-op / clone body ticks its own.
    mut home_feel_q: Query<
        &mut ambition_characters::actor::BodyCombat,
        With<crate::actor::PlayerEntity>,
    >,
) {
    let frame_dt = time.delta_secs();
    let feel = *feel_tuning;
    sim_state.room_transition_cooldown = (sim_state.room_transition_cooldown - frame_dt).max(0.0);
    for mut combat in &mut home_feel_q {
        combat.damage_invuln_timer = (combat.damage_invuln_timer - frame_dt).max(0.0);
        combat.hitstun_timer = (combat.hitstun_timer - frame_dt).max(0.0);
        combat.recoil_lock_timer = (combat.recoil_lock_timer - frame_dt).max(0.0);
        combat.hitstop_timer = (combat.hitstop_timer - frame_dt).max(0.0);
    }
    let interaction = slot_gestures.primary_mut();
    // Fast-fall = double-tap local-down for the controlled body. Raw cardinal
    // edges are resolved through the same input mapping policy as locomotion,
    // so ScreenDirected sideways gravity can map raw-right/raw-left into local
    // down/up without bespoke cases here.
    let gravity_dir = crate::physics::gravity_dir_or_default(gravity_field.as_deref());
    let movement_mode = user_settings
        .as_deref()
        .map_or(ae::InputFrameMode::DEFAULT_MOVEMENT, |s| {
            s.gameplay.movement_frame_mode
        });
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
}

/// Fold the explicit `Interact` action together with the
/// `double_tap_up_pending` gesture, gate the result on the CONTROLLED body's
/// hit-stun, and advance the per-frame interact buffer on the primary
/// controller's slot (`SlotInteractionState`).
///
/// Downstream consumers read the buffered signal from
/// `SlotInteractionState::primary().buffered()` (or the controlled body's slot).
/// The host gates this on `gameplay_allowed` so the buffer does not tick down
/// while paused, in dialogue, or mid-cutscene.
///
/// Ordering: must run after `input_timer_system` (which decrements the controlled
/// body's `combat.hitstun_timer` and sets `double_tap_up_pending` from
/// `register_up_tap`) and before `detect_room_transition_system` (which consumes
/// the buffered signal post-player-tick).
pub fn interaction_input_system(
    time: Res<Time>,
    feel_tuning: Res<crate::time::feel::SandboxFeelTuning>,
    control_frame: Res<ControlFrame>,
    gravity_field: Option<Res<crate::physics::GravityField>>,
    user_settings: Option<Res<ambition_persistence::settings::UserSettings>>,
    controlled: Option<Res<crate::abilities::traversal::possession::ControlledSubject>>,
    mut slot_gestures: ResMut<crate::player::SlotInteractionState>,
    // Hit-stun gate reads the CONTROLLED body's reaction state — the body actually
    // being driven, home avatar or possessed actor.
    combat_q: Query<&ambition_characters::actor::BodyCombat>,
    primary_q: Query<
        Entity,
        (
            With<crate::actor::PlayerEntity>,
            With<crate::actor::PrimaryPlayer>,
        ),
    >,
) {
    let frame_dt = time.delta_secs();
    let feel = *feel_tuning;
    let subject = controlled
        .and_then(|subject| subject.0)
        .or_else(|| primary_q.single().ok());
    let hitstun = subject
        .and_then(|subject| combat_q.get(subject).ok())
        .map_or(0.0, |combat| combat.hitstun_timer);
    let interaction = slot_gestures.primary_mut();
    let door_double_tap_up = std::mem::take(&mut interaction.double_tap_up_pending);
    // Down + Interact is the possession gesture (`abilities::traversal::possession`),
    // so a held-Down interact is CLAIMED by possession and must NOT also trigger a
    // normal interaction (open a door / start an NPC dialog) — otherwise the press
    // that begins a possession hold also opens whatever's adjacent. Suppress the
    // interact EDGE while Down is held, using the SAME gravity-resolved "down" the
    // possession trigger uses so they agree under any gravity. The double-tap-UP
    // door request is an Up gesture, so it is never suppressed.
    let gravity_dir = crate::physics::gravity_dir_or_default(gravity_field.as_deref());
    let movement_mode = user_settings
        .as_deref()
        .map_or(ae::InputFrameMode::DEFAULT_MOVEMENT, |s| {
            s.gameplay.movement_frame_mode
        });
    let down_held = crate::abilities::traversal::possession::holding_descend(
        control_frame.axis_x,
        control_frame.axis_y,
        gravity_dir,
        movement_mode,
    );
    // Reads `Res<ControlFrame>` directly (local input publication is exactly
    // where raw device state is allowed): this system runs mid-input-chain and
    // publishes the device's interact into the slot buffer. The hit-stun gate uses
    // the CONTROLLED body's `hitstun`, resolved above.
    let raw_interact_pressed = if hitstun > 0.0 {
        false
    } else {
        (control_frame.interact_pressed && !down_held) || door_double_tap_up
    };
    let _live =
        interaction.buffered_interact(raw_interact_pressed, frame_dt, feel.interaction_buffer_time);
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
    mut dev_state: ResMut<crate::SandboxDevState>,
    mut player_q: Query<
        (
            &crate::actor::BodyKinematics,
            &crate::actor::BodyGroundState,
            &crate::actor::BodyDashState,
            &mut crate::player::BodyAnimFacts,
            &mut ambition_characters::actor::BodyCombat,
            &mut crate::player::PlayerBlinkCameraState,
        ),
        crate::actor::PrimaryPlayerOnly,
    >,
) {
    let frame_dt = time.delta_secs();
    let Ok((kinematics, ground, dash, mut anim, mut combat, mut blink_cam)) = player_q.single_mut()
    else {
        return;
    };
    combat.hit_flash = (combat.hit_flash - frame_dt).max(0.0);
    dev_state.preset_flash = (dev_state.preset_flash - frame_dt).max(0.0);
    // Player-specific presentation timers (the blink-camera lerp) decay here; the
    // body-generic anim OVERLAYS advance through the shared helper the actor tick
    // also runs (fable review §A9).
    blink_cam.blink_in_timer = (blink_cam.blink_in_timer - frame_dt).max(0.0);
    blink_cam.camera_snap_timer = (blink_cam.camera_snap_timer - frame_dt).max(0.0);
    crate::player::advance_body_anim_overlays(
        ground.on_ground,
        kinematics.vel.y,
        dash.timer,
        &mut anim,
        frame_dt,
    );
}

#[cfg(test)]
mod interaction_suppression_tests {
    use super::*;
    use crate::actor::{PlayerEntity, PrimaryPlayer};
    use crate::player::SlotInteractionState;
    use crate::time::feel::SandboxFeelTuning;
    use ambition_characters::actor::BodyCombat;

    /// Build a minimal app with `interaction_input_system` and one primary
    /// player, set the control frame, run a frame, and report whether the
    /// primary controller's slot interaction buffer went live.
    fn buffered_after(interact: bool, axis_y: f32) -> bool {
        let mut app = App::new();
        app.insert_resource(Time::<()>::default());
        app.insert_resource(SandboxFeelTuning::default());
        app.init_resource::<SlotInteractionState>();
        app.insert_resource(ControlFrame {
            interact_pressed: interact,
            axis_y,
            ..Default::default()
        });
        app.world_mut()
            .spawn((PlayerEntity, PrimaryPlayer, BodyCombat::default()));
        app.add_systems(Update, interaction_input_system);
        app.update();
        app.world()
            .resource::<SlotInteractionState>()
            .primary()
            .buffered()
    }

    /// A plain Interact (no Down) registers a normal interaction.
    #[test]
    fn plain_interact_registers() {
        assert!(
            buffered_after(true, 0.0),
            "Interact with no Down must trigger a normal interaction"
        );
    }

    /// Down + Interact is the possession gesture and must NOT register a normal
    /// interaction (the in-game bug Jon hit: starting a possession hold next to
    /// an NPC opened its dialog). The Down-held interact edge is suppressed.
    #[test]
    fn down_interact_is_claimed_by_possession_not_a_normal_interact() {
        assert!(
            !buffered_after(true, 1.0),
            "Down+Interact must be claimed by possession, not open a door/NPC"
        );
    }

    /// Sanity: no interact press → nothing buffered, with or without Down.
    #[test]
    fn no_press_buffers_nothing() {
        assert!(!buffered_after(false, 0.0));
        assert!(!buffered_after(false, 1.0));
    }
}

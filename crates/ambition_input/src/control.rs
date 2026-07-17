//! Device adapters that build the engine-owned `ControlFrame` resource.
//!
//! The pure, brain-facing [`ControlFrame`] vocabulary lives in
//! `ambition_engine_core`; this module is the input adapter that translates
//! Leafwing `SandboxAction`s, control settings, and trigger hysteresis into that
//! frame. Headless/replay/netcode callers can construct `ControlFrame` directly
//! without depending on this crate.

use bevy::prelude::Resource;
#[cfg(feature = "input")]
use leafwing_input_manager::prelude::ActionState;

#[cfg(feature = "input")]
use ambition_engine_core::ControlFrame;

#[cfg(feature = "input")]
use crate::actions::SandboxAction;

/// Build a gameplay control frame, applying configurable deadzones,
/// trigger hysteresis, and the dash-input mode from
/// [`crate::settings::ControlSettings`].
///
/// `dash_state` is the persistent trigger edge tracker for the player; it must
/// outlive a single frame so the hysteretic press/release semantics work. The
/// function returns the next state so the caller can store it back into a Bevy
/// resource.
#[cfg(feature = "input")]
pub fn read_gameplay_control_frame_with_settings(
    actions: &ActionState<SandboxAction>,
    controls: &crate::settings::ControlSettings,
    dash_state: crate::settings::TriggerEdgeState,
) -> (ControlFrame, crate::settings::TriggerEdgeState) {
    let raw_move = actions.clamped_axis_pair(&SandboxAction::Move);
    // Apply the left-stick deadzone before any walk-modifier logic so analog
    // drift doesn't pollute the magnitude check.
    let (deadzoned_x, deadzoned_y) = crate::settings::ControlSettings::apply_deadzone(
        raw_move.x,
        raw_move.y,
        controls.left_stick_deadzone,
    );
    let mut axis = bevy::math::Vec2::new(deadzoned_x, deadzoned_y);

    // Walk modifier: Shift on keyboard, LT2 on gamepad. Cardinal / D-pad input
    // arrives at unit magnitude (run); the modifier caps the move vector so
    // digital input becomes walk speed. Analog sticks already pace via
    // magnitude — capping at WALK_FACTOR only kicks in when the stick is pushed
    // past walk speed, so LT2 acts as a "max-speed governor" for stick users
    // while letting them still creep slowly without the modifier.
    if actions.pressed(&SandboxAction::Modifier) {
        const WALK_FACTOR: f32 = 0.45;
        let magnitude = axis.length();
        if magnitude > WALK_FACTOR {
            axis *= WALK_FACTOR / magnitude;
        }
    }
    let left_pressed = actions.just_pressed(&SandboxAction::MoveLeft);
    let right_pressed = actions.just_pressed(&SandboxAction::MoveRight);
    let up_pressed = actions.just_pressed(&SandboxAction::MoveUp);
    let down_pressed = actions.just_pressed(&SandboxAction::MoveDown);

    // Dash hysteresis: read the analog right trigger value plus the binary RT2
    // button as the "press level". The settings-defined press / release
    // thresholds collapse trigger jitter into a single edge.
    let raw_trigger = actions.value(&SandboxAction::DashAnalog).clamp(0.0, 1.0);
    let dash_button_value = if actions.pressed(&SandboxAction::Dash) {
        1.0
    } else {
        0.0
    };
    let trigger_value = raw_trigger.max(dash_button_value);
    let (next_dash_state, trigger_edge_pressed) = crate::settings::update_trigger_edge(
        dash_state,
        trigger_value,
        controls.trigger_release_threshold,
        controls.trigger_press_threshold,
    );
    let dash_pressed = match controls.dash_input_mode {
        crate::settings::DashInputMode::Trigger => trigger_edge_pressed,
        // Button mode: ignore trigger hysteresis, only the configured Dash
        // button counts (e.g. RB on a 360 pad).
        crate::settings::DashInputMode::Button => actions.just_pressed(&SandboxAction::Dash),
        crate::settings::DashInputMode::Both => {
            trigger_edge_pressed || actions.just_pressed(&SandboxAction::Dash)
        }
    };

    // Aim deadzone — applied to the right stick before blink aim consumes it.
    // This is the fix for old-controller drift pushing the blink target upward.
    let raw_aim = actions.clamped_axis_pair(&SandboxAction::AimStick);
    let (aim_x_raw, aim_y_raw) = crate::settings::ControlSettings::apply_deadzone(
        raw_aim.x,
        raw_aim.y,
        controls.right_stick_deadzone,
    );
    let aim_y = if controls.invert_aim_y {
        -aim_y_raw
    } else {
        aim_y_raw
    };

    let frame = ControlFrame {
        axis_x: axis.x,
        // Ambition's simulation uses screen-space world coordinates: +Y is
        // downward. Leafwing's virtual D-pads use the usual +Y-up convention.
        axis_y: -axis.y,
        jump_pressed: actions.just_pressed(&SandboxAction::Jump),
        jump_held: actions.pressed(&SandboxAction::Jump),
        jump_released: actions.just_released(&SandboxAction::Jump),
        dash_pressed,
        left_pressed,
        right_pressed,
        up_pressed,
        down_pressed,
        fast_fall_pressed: false,
        blink_pressed: actions.just_pressed(&SandboxAction::Blink),
        blink_held: actions.pressed(&SandboxAction::Blink),
        blink_released: actions.just_released(&SandboxAction::Blink),
        special_pressed: actions.just_pressed(&SandboxAction::Special),
        attack_pressed: actions.just_pressed(&SandboxAction::Attack),
        pogo_pressed: actions.just_pressed(&SandboxAction::Pogo),
        fly_toggle_pressed: actions.just_pressed(&SandboxAction::Utility),
        interact_pressed: actions.just_pressed(&SandboxAction::Interact),
        interact_held: actions.pressed(&SandboxAction::Interact),
        reset_pressed: actions.just_pressed(&SandboxAction::Reset),
        start_pressed: actions.just_pressed(&SandboxAction::Start),
        projectile_pressed: actions.just_pressed(&SandboxAction::Projectile),
        projectile_held: actions.pressed(&SandboxAction::Projectile),
        projectile_released: actions.just_released(&SandboxAction::Projectile),
        shield_held: actions.pressed(&SandboxAction::QuickAction),
        aim_x: aim_x_raw,
        // Match the sim's +Y-down convention.
        aim_y: -aim_y,
    };
    (frame, next_dash_state)
}

/// Convenience for tests/headless-visible paths: gameplay frame with default
/// control settings and a fresh trigger state.
#[cfg(feature = "input")]
pub fn read_gameplay_control_frame(actions: &ActionState<SandboxAction>) -> ControlFrame {
    let (frame, _) = read_gameplay_control_frame_with_settings(
        actions,
        &crate::settings::ControlSettings::default(),
        crate::settings::TriggerEdgeState::default(),
    );
    frame
}

/// Read only the gameplay-side state that should still flow during pause/menu
/// mode. Today that's just `start_pressed` (which the pause toggle reads) —
/// every other gameplay action is suppressed.
#[cfg(feature = "input")]
pub fn read_menu_control_frame(actions: &ActionState<SandboxAction>) -> ControlFrame {
    ControlFrame {
        start_pressed: actions.just_pressed(&SandboxAction::Start),
        ..ControlFrame::default()
    }
}

/// Persistent dash-trigger edge state. Lives outside `ControlFrame` because
/// the hysteresis logic must remember the previous state across frames;
/// `ControlFrame` is stateless and rebuilt every frame.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlayerDashTriggerState {
    pub edge: crate::settings::TriggerEdgeState,
}

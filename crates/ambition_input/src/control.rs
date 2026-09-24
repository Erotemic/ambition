//! Device adapters that build the engine-owned `ControlFrame` resource.
//!
//! The brain-facing [`ControlFrame`] vocabulary lives in
//! `ambition_platformer2d_core`. This module translates Leafwing
//! `Platformer2dInputActionMonolith` actions, control settings, and trigger
//! hysteresis into that frame. Headless, replay, and netcode callers can build
//! `ControlFrame` directly without this crate.

#[cfg(feature = "input")]
use leafwing_input_manager::prelude::ActionState;

#[cfg(feature = "input")]
use ambition_platformer2d_core::ControlFrame;

#[cfg(feature = "input")]
use crate::actions::Platformer2dInputActionMonolith;

/// The largest movement magnitude the walk modifier permits.
///
/// Keep this below `LocomotionTuning::run_commit_frac` (default
/// [`ambition_platformer2d_core::movement::tuning::RUN_COMMIT_FRAC`], 0.55).
/// The simulation compares the magnitude with that value to select walk or
/// run, so a cap at or above it gives a "walk" that runs. A body that authors
/// a lower `run_commit_frac` runs while walk is held.
// Gated like its only use site. Without the gate, a standalone build of this
// crate (no default features) warns; the workspace build does not.
#[cfg(feature = "input")]
pub const WALK_AXIS_CAP: f32 = 0.5;

/// Build a gameplay control frame, applying configurable deadzones,
/// trigger hysteresis, and the burst-input mode from
/// [`crate::settings::ControlSettings`].
///
/// `burst_state` is the persistent trigger edge tracker for the player; it must
/// outlive a single frame so the hysteretic press/release semantics work. The
/// function returns the next state so the caller can store it back into a Bevy
/// resource.
#[cfg(feature = "input")]

pub fn read_gameplay_control_frame_with_settings(
    actions: &ActionState<Platformer2dInputActionMonolith>,
    controls: crate::settings::ControlFilters,
    edges: crate::settings::GameplayEdgeState,
) -> (ControlFrame, crate::settings::GameplayEdgeState) {
    let raw_move = actions.clamped_axis_pair(&Platformer2dInputActionMonolith::Move);
    // Deadzone first. The simulation reads stick magnitude as the gait
    // (`run * max_run_speed`, cut by `run_commit_frac`), so drift here would
    // make a body walk by itself.
    let (deadzoned_x, deadzoned_y) = crate::settings::ControlSettings::apply_deadzone(
        raw_move.x,
        raw_move.y,
        controls.left_stick_deadzone,
    );
    let axis = bevy::math::Vec2::new(deadzoned_x, deadzoned_y);

    // Report the modifier raw (held level and press edge). The adapter gives it
    // no meaning; each body's rules decide what holding it does.
    let modifier_held = actions.pressed(&Platformer2dInputActionMonolith::Modifier);
    let modifier_pressed = actions.just_pressed(&Platformer2dInputActionMonolith::Modifier);
    // Walk: the simulation reads stick magnitude as the gait, and a digital
    // source always gives 1.0, so without this cap a keyboard or D-pad body
    // cannot walk. Cap the magnitude; do not scale it, so an analog tilt that
    // is already a walk stays the same. The rule is the same for every source
    // bound to `Move`.
    //
    // Read `Walk`, not `Modifier`: Mary-O uses `modifier_held` as her run.
    let walk_held = actions.pressed(&Platformer2dInputActionMonolith::Walk);
    let axis = if walk_held {
        let magnitude = axis.length();
        if magnitude > WALK_AXIS_CAP {
            axis * (WALK_AXIS_CAP / magnitude)
        } else {
            axis
        }
    } else {
        axis
    };
    let left_pressed = actions.just_pressed(&Platformer2dInputActionMonolith::MoveLeft);
    let right_pressed = actions.just_pressed(&Platformer2dInputActionMonolith::MoveRight);
    let up_pressed = actions.just_pressed(&Platformer2dInputActionMonolith::MoveUp);
    let down_pressed = actions.just_pressed(&Platformer2dInputActionMonolith::MoveDown);

    // Burst press level: the larger of the analog right trigger and the binary
    // Burst button. The press/release thresholds from settings collapse
    // trigger jitter into one edge.
    //
    // Stored remaps use the action's `Debug` name, so
    // `settings::ControlSettings::migrate_renamed_actions` maps `"Dash"` to
    // `"Burst"`. The persisted `dash_input_mode` key stays pinned with
    // `#[serde(rename)]`.
    let raw_trigger = actions
        .value(&Platformer2dInputActionMonolith::BurstAnalog)
        .clamp(0.0, 1.0);
    let burst_button_value = if actions.pressed(&Platformer2dInputActionMonolith::Burst) {
        1.0
    } else {
        0.0
    };
    let trigger_value = raw_trigger.max(burst_button_value);
    let (next_burst_state, trigger_edge_pressed) = crate::settings::update_trigger_edge(
        edges.burst,
        trigger_value,
        controls.trigger_release_threshold,
        controls.trigger_press_threshold,
    );
    let burst_pressed = match controls.burst_input_mode {
        crate::settings::BurstInputMode::Trigger => trigger_edge_pressed,
        // Button mode: ignore trigger hysteresis, only the configured Burst
        // button counts (e.g. RB on a 360 pad).
        crate::settings::BurstInputMode::Button => {
            actions.just_pressed(&Platformer2dInputActionMonolith::Burst)
        }
        crate::settings::BurstInputMode::Both => {
            trigger_edge_pressed || actions.just_pressed(&Platformer2dInputActionMonolith::Burst)
        }
    };

    // Deadzone the aim stick so controller drift does not move the blink target.
    let raw_aim = actions.clamped_axis_pair(&Platformer2dInputActionMonolith::AimStick);
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

    // C-stick: in an attack mode, a right-stick deflection is a press, not an
    // aim. Crossing `AIM_STICK_ATTACK_THRESHOLD` from rest throws the authored
    // strength in that direction. The burst-trigger hysteresis stops a held
    // stick from firing again each frame.
    //
    // The direction uses its own pair (`attack_aim_x/y`), not `aim_x`/`aim_y`.
    // Aim values are levels (newest sample wins); the press and strength are
    // edges. A flick that recenters before the sim tick would otherwise give an
    // attack with zero aim, and `attack_axis` would fall back to the movement
    // axis and attack the wrong way.
    //
    // In this mode the stick does not aim.
    let (next_aim_stick_state, aim_stick_flicked) = crate::settings::update_trigger_edge(
        edges.aim_stick,
        bevy::math::Vec2::new(aim_x_raw, aim_y_raw).length(),
        controls.right_stick_deadzone,
        crate::settings::AIM_STICK_ATTACK_THRESHOLD,
    );
    let stick_attack = controls
        .right_stick_mode
        .attack_strength()
        .filter(|_| aim_stick_flicked);

    let frame = ControlFrame {
        // The device adapter does not know the seat's frame policy. That mode is
        // a settings fact that `populate_seat_control_frames` stamps on at
        // capture. The default here keeps a frame that skips that stage valid.
        control_frame_modes: ambition_platformer2d_core::ControlFrameModes::default(),
        axis_x: axis.x,
        // Ambition's simulation uses screen-space world coordinates: +Y is
        // downward. Leafwing's virtual D-pads use the usual +Y-up convention.
        axis_y: -axis.y,
        jump_pressed: actions.just_pressed(&Platformer2dInputActionMonolith::Jump),
        jump_held: actions.pressed(&Platformer2dInputActionMonolith::Jump),
        jump_released: actions.just_released(&Platformer2dInputActionMonolith::Jump),
        burst_pressed,
        left_pressed,
        right_pressed,
        up_pressed,
        down_pressed,
        fast_fall_pressed: false,
        blink_pressed: actions.just_pressed(&Platformer2dInputActionMonolith::Blink),
        blink_held: actions.pressed(&Platformer2dInputActionMonolith::Blink),
        blink_released: actions.just_released(&Platformer2dInputActionMonolith::Blink),
        special_pressed: actions.just_pressed(&Platformer2dInputActionMonolith::Special),
        special_held: actions.pressed(&Platformer2dInputActionMonolith::Special),
        attack_pressed: actions.just_pressed(&Platformer2dInputActionMonolith::Attack)
            || stick_attack.is_some(),
        attack_held: actions.pressed(&Platformer2dInputActionMonolith::Attack),
        attack_released: actions.just_released(&Platformer2dInputActionMonolith::Attack),
        // `RightStickMode::TiltAttack`/`SmashAttack` make the right stick force
        // `Tilt` or `Smash`. The stick outranks the button on the same frame,
        // because its purpose is to force a strength. A `StrongAttack` press
        // without a flick means `Smash`. Otherwise `Auto`: the interpreter
        // reads the stick, as an ordinary attack button asks.
        attack_strength_hint: stick_attack.unwrap_or(
            if actions.pressed(&Platformer2dInputActionMonolith::StrongAttack) {
                ambition_platformer2d_core::AttackStrengthHint::Smash
            } else {
                ambition_platformer2d_core::AttackStrengthHint::Auto
            },
        ),
        attack_from_aim_stick: stick_attack.is_some(),
        // The flick's own direction, latched with its edge. Zero when this frame
        // has no C-stick press.
        attack_aim_x: if stick_attack.is_some() {
            aim_x_raw
        } else {
            0.0
        },
        attack_aim_y: if stick_attack.is_some() { -aim_y } else { 0.0 },
        pogo_pressed: actions.just_pressed(&Platformer2dInputActionMonolith::Pogo),
        fly_toggle_pressed: actions.just_pressed(&Platformer2dInputActionMonolith::Utility),
        interact_pressed: actions.just_pressed(&Platformer2dInputActionMonolith::Interact),
        interact_held: actions.pressed(&Platformer2dInputActionMonolith::Interact),
        reset_pressed: actions.just_pressed(&Platformer2dInputActionMonolith::Reset),
        start_pressed: actions.just_pressed(&Platformer2dInputActionMonolith::Start),
        projectile_pressed: actions.just_pressed(&Platformer2dInputActionMonolith::Projectile),
        projectile_held: actions.pressed(&Platformer2dInputActionMonolith::Projectile),
        projectile_released: actions.just_released(&Platformer2dInputActionMonolith::Projectile),
        shield_held: actions.pressed(&Platformer2dInputActionMonolith::Shield),
        // Read the edge, not the level: a grab is one attempt, and the grab
        // move owns how long it stays active. Reading the level would retry
        // every tick the button is held and remove the cost of a miss.
        grab_pressed: actions.just_pressed(&Platformer2dInputActionMonolith::Grab),
        // A taunt is one press, one performance. Read the edge, as for grab.
        taunt_pressed: actions.just_pressed(&Platformer2dInputActionMonolith::Taunt),
        modifier_held,
        modifier_pressed,
        aim_x: aim_x_raw,
        // Match the sim's +Y-down convention.
        aim_y: -aim_y,
    };
    (
        frame,
        crate::settings::GameplayEdgeState {
            burst: next_burst_state,
            aim_stick: next_aim_stick_state,
        },
    )
}

/// Convenience for tests/headless-visible paths: gameplay frame with default
/// control settings and a fresh trigger state.
#[cfg(feature = "input")]
pub fn read_gameplay_control_frame(
    actions: &ActionState<Platformer2dInputActionMonolith>,
) -> ControlFrame {
    let defaults = crate::settings::ControlSettings::default();
    let (frame, _) = read_gameplay_control_frame_with_settings(
        actions,
        crate::settings::ControlFilters::from_settings(&defaults),
        crate::settings::GameplayEdgeState::default(),
    );
    frame
}

/// Read only the gameplay-side state that should still flow during pause/menu
/// mode. Today that's just `start_pressed` (which the pause toggle reads) —
/// every other gameplay action is suppressed.
#[cfg(feature = "input")]
pub fn read_menu_control_frame(
    actions: &ActionState<Platformer2dInputActionMonolith>,
) -> ControlFrame {
    ControlFrame {
        start_pressed: actions.just_pressed(&Platformer2dInputActionMonolith::Start),
        ..ControlFrame::default()
    }
}

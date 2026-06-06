use super::*;

/// Per-frame snapshot of player input feeding the simulation.
///
/// Stored as a Bevy resource so the visible binary can populate it from
/// `leafwing-input-manager` (presentation-side) and headless tests can
/// pre-populate it directly without an InputPlugin in scope. The
/// simulation reads `Res<ControlFrame>`; it never touches leafwing types
/// — that's the ADR 0012 sim/presentation seam for the input channel.
///
/// **Multiplayer caveat (primary-player-only):** there is exactly one
/// `ControlFrame` resource and it represents the local primary
/// player's input. Co-op / split-screen / network play will need
/// per-`PlayerSlot` input — likely an `InputFrame` *component* on the
/// player entity, with the visible binary writing per-device and
/// remote-player frames being driven by a netcode adapter. Until then,
/// do not add new "per-player" fields to this struct; instead expand
/// `PlayerInteractionState` or another per-player component.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct ControlFrame {
    pub axis_x: f32,
    pub axis_y: f32,
    pub jump_pressed: bool,
    pub jump_held: bool,
    pub jump_released: bool,
    pub dash_pressed: bool,
    /// Movement-up input was newly pressed this frame. The sandbox uses this
    /// to require a double-tap-up door activation while flying, so upward
    /// flight does not accidentally enter doors.
    pub up_pressed: bool,
    /// Movement-down input was newly pressed this frame. The sandbox uses this
    /// to recognize double-tap-down for fast-fall without making down+attack
    /// automatically fast-fall.
    pub down_pressed: bool,
    /// Double-tap-down recognized by the sandbox input gesture detector.
    pub fast_fall_pressed: bool,
    pub blink_pressed: bool,
    pub blink_held: bool,
    pub blink_released: bool,
    pub attack_pressed: bool,
    pub pogo_pressed: bool,
    pub fly_toggle_pressed: bool,
    /// Generic context interaction. This is a dedicated interact action plus
    /// the sandbox double-tap-up gesture, not raw held/up movement.
    pub interact_pressed: bool,
    pub reset_pressed: bool,
    pub start_pressed: bool,
    /// Player projectile / spell action — newly pressed this frame.
    pub projectile_pressed: bool,
    /// Player projectile button is currently held. Used by the
    /// fireball charge mechanic to accumulate hold time. Whenever
    /// the button is held, the charge timer ticks; release-edge
    /// (`projectile_released`) commits the charged shot.
    pub projectile_held: bool,
    /// Player projectile button was released this frame. Triggers
    /// the actual fireball spawn when a charge was in progress.
    pub projectile_released: bool,
    /// Shield button is currently held. Maps to `SandboxAction::QuickAction`.
    /// While held with the `shield` ability active, the engine deploys the
    /// bubble and tracks the parry window.
    pub shield_held: bool,
    /// Right stick / aim vector after deadzone is applied. Blink aim and
    /// any future twin-stick aiming should consume this instead of
    /// reading raw axes — the deadzone here is what fixes Xbox 360
    /// drift from gradually pushing the blink target upward.
    pub aim_x: f32,
    pub aim_y: f32,
}

impl ControlFrame {
    /// Build a gameplay control frame, applying configurable deadzones,
    /// trigger hysteresis, and the dash-input mode from
    /// `crate::settings::ControlSettings`.
    ///
    /// `dash_state` is the persistent trigger edge tracker for the
    /// player; it must outlive a single frame so the hysteretic press/
    /// release semantics work. The function returns the next state so
    /// the caller can store it back into a Bevy resource.
    #[cfg(feature = "input")]
    pub fn read_gameplay_with_settings(
        actions: &ActionState<SandboxAction>,
        controls: &crate::settings::ControlSettings,
        dash_state: crate::settings::TriggerEdgeState,
    ) -> (Self, crate::settings::TriggerEdgeState) {
        let raw_move = actions.clamped_axis_pair(&SandboxAction::Move);
        // Apply the left-stick deadzone before any walk-modifier logic
        // so analog drift doesn't pollute the magnitude check.
        let (deadzoned_x, deadzoned_y) = crate::settings::ControlSettings::apply_deadzone(
            raw_move.x,
            raw_move.y,
            controls.left_stick_deadzone,
        );
        let mut axis = bevy::math::Vec2::new(deadzoned_x, deadzoned_y);

        // Walk modifier: Shift on keyboard, LT2 on gamepad. Cardinal /
        // D-pad input arrives at unit magnitude (run); the modifier caps
        // the move vector so digital input becomes walk speed. Analog
        // sticks already pace via magnitude — capping at WALK_FACTOR
        // only kicks in when the stick is pushed past walk speed, so
        // LT2 acts as a "max-speed governor" for stick users while
        // letting them still creep slowly without the modifier.
        if actions.pressed(&SandboxAction::Modifier) {
            const WALK_FACTOR: f32 = 0.45;
            let magnitude = axis.length();
            if magnitude > WALK_FACTOR {
                axis *= WALK_FACTOR / magnitude;
            }
        }
        let up_pressed = actions.just_pressed(&SandboxAction::MoveUp);
        let down_pressed = actions.just_pressed(&SandboxAction::MoveDown);

        // Dash hysteresis: read the analog right trigger value plus the
        // binary RT2 button as the "press level". The settings-defined
        // press / release thresholds collapse trigger jitter into a
        // single edge.
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
            // Button mode: ignore trigger hysteresis, only the
            // configured Dash button counts (e.g. RB on a 360 pad).
            crate::settings::DashInputMode::Button => actions.just_pressed(&SandboxAction::Dash),
            crate::settings::DashInputMode::Both => {
                trigger_edge_pressed || actions.just_pressed(&SandboxAction::Dash)
            }
        };

        // Aim deadzone — applied to the right stick before blink aim
        // consumes it. This is the fix for old-controller drift
        // pushing the blink target upward.
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

        let frame = Self {
            axis_x: axis.x,
            // Ambition's simulation uses screen-space world coordinates: +Y is
            // downward. Leafwing's virtual D-pads use the usual +Y-up convention.
            axis_y: -axis.y,
            jump_pressed: actions.just_pressed(&SandboxAction::Jump),
            jump_held: actions.pressed(&SandboxAction::Jump),
            jump_released: actions.just_released(&SandboxAction::Jump),
            dash_pressed,
            up_pressed,
            down_pressed,
            fast_fall_pressed: false,
            blink_pressed: actions.just_pressed(&SandboxAction::Blink),
            blink_held: actions.pressed(&SandboxAction::Blink),
            blink_released: actions.just_released(&SandboxAction::Blink),
            attack_pressed: actions.just_pressed(&SandboxAction::Attack),
            pogo_pressed: actions.just_pressed(&SandboxAction::Pogo),
            fly_toggle_pressed: actions.just_pressed(&SandboxAction::Utility),
            interact_pressed: actions.just_pressed(&SandboxAction::Interact),
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

    /// Convenience for tests/headless: gameplay frame with default
    /// control settings and a fresh trigger state.
    #[cfg(feature = "input")]
    pub fn read_gameplay(actions: &ActionState<SandboxAction>) -> Self {
        let (frame, _) = Self::read_gameplay_with_settings(
            actions,
            &crate::settings::ControlSettings::default(),
            crate::settings::TriggerEdgeState::default(),
        );
        frame
    }

    /// Read only the gameplay-side state that should still flow during
    /// pause/menu mode. Today that's just `start_pressed` (which the
    /// pause toggle reads) — every other gameplay action is suppressed.
    #[cfg(feature = "input")]
    pub fn read_menu(actions: &ActionState<SandboxAction>) -> Self {
        Self {
            start_pressed: actions.just_pressed(&SandboxAction::Start),
            ..default()
        }
    }

    pub fn engine_input(self, control_dt: f32) -> ae::InputState {
        // Down held + jump just-pressed is the explicit "drop through one-way"
        // gesture. Holding down alone no longer drops the player through.
        let drop_through_pressed = self.axis_y > 0.35 && self.jump_pressed;
        ae::InputState {
            axis_x: self.axis_x,
            axis_y: self.axis_y,
            jump_pressed: self.jump_pressed,
            jump_held: self.jump_held,
            jump_released: self.jump_released,
            dash_pressed: self.dash_pressed,
            fly_toggle_pressed: self.fly_toggle_pressed,
            blink_pressed: self.blink_pressed,
            blink_held: self.blink_held,
            blink_released: self.blink_released,
            fast_fall_pressed: self.fast_fall_pressed,
            drop_through_pressed,
            attack_pressed: self.attack_pressed,
            pogo_pressed: self.pogo_pressed,
            interact_pressed: self.interact_pressed,
            reset_pressed: false,
            shield_held: self.shield_held,
            control_dt,
        }
    }
}

/// Persistent dash-trigger edge state. Lives outside `ControlFrame`
/// because the hysteresis logic must remember the previous state across
/// frames; `ControlFrame` is stateless and rebuilt every frame.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlayerDashTriggerState {
    pub edge: crate::settings::TriggerEdgeState,
}

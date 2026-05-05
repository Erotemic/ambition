//! Keyboard/gamepad semantic input model for the sandbox.
//!
//! Physical inputs are bound to `SandboxAction` with Leafwing Input Manager.
//! The engine still consumes a compact `ControlFrame`, which keeps movement
//! physics independent from keyboards, gamepads, UI rebinding, or replay input.

use ambition_engine as ae;
use bevy::prelude::*;
#[cfg(feature = "input")]
use leafwing_input_manager::prelude::*;

/// Logical player/sandbox inputs understood by the Bevy adapter layer.
///
/// `Move` is dual-axis so analog sticks and virtual D-pads can feed a single
/// movement vector. The cardinal `Move*` button actions intentionally duplicate
/// the directional bindings so systems can still detect edge-triggered gestures
/// such as double-tap-down fast fall and double-tap-up door activation.
///
/// Menu navigation lives on its own `MenuNavigate*` / `MenuSelect` /
/// `MenuBack` axis so confirming in a menu does not require pressing
/// "Jump", and so D-pad / arrow keys / Enter all flow through one
/// semantic seam. The renderer reads `MenuAxisFrame` (drained from
/// these actions) instead of touching `SandboxAction` directly.
///
/// Gated behind `input`: this type pulls in leafwing's `Actionlike` trait.
/// Sim-only builds use `ControlFrame` (always-available) on the seam instead.
#[cfg(feature = "input")]
#[derive(Actionlike, Clone, Copy, Debug, Hash, PartialEq, Eq, Reflect)]
pub enum SandboxAction {
    #[actionlike(DualAxis)]
    Move,
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    Jump,
    Attack,
    Dash,
    Blink,
    QuickAction,
    Interact,
    Modifier,
    Utility,
    Map,
    Inventory,
    Pogo,
    Reset,
    Start,
    /// Player projectile / spell action. Default binding: `F` (keyboard)
    /// and the gamepad West face button (with Attack on the same button
    /// when no projectile is unlocked yet — sandbox always-on for now).
    Projectile,
    /// Menu navigation seam. These are the only actions the pause /
    /// settings menu reads; gameplay never consumes them. Bindings:
    /// arrow keys, WASD, D-pad, left stick (with deadzone applied
    /// later), Enter / Space / South for select, Escape / Backspace /
    /// East for back.
    MenuNavigateUp,
    MenuNavigateDown,
    MenuNavigateLeft,
    MenuNavigateRight,
    MenuSelect,
    MenuBack,
    /// Analog left-stick read used to drive menu navigation with
    /// configurable deadzone + repeat. Renders into `MenuAxisFrame`.
    #[actionlike(DualAxis)]
    MenuStick,
    /// Analog right-trigger value (0..=1). Used together with
    /// configurable hysteresis thresholds to derive the dash-pressed
    /// edge so a worn trigger held above the threshold cannot retrigger
    /// dash repeatedly.
    #[actionlike(Axis)]
    DashAnalog,
    /// Analog right-stick / aim read. The aim deadzone is applied here
    /// before the value reaches blink aim, so a drifting Xbox 360
    /// controller does not gradually push the blink target upward.
    #[actionlike(DualAxis)]
    AimStick,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresetId {
    ArrowsZxc,
    WasdJkl,
    ArrowsQwer,
    WasdUipo,
}

#[derive(Clone, Copy, Debug)]
pub struct MovementKeys {
    pub left: KeyCode,
    pub right: KeyCode,
    pub up: KeyCode,
    pub down: KeyCode,
}

#[derive(Clone, Copy, Debug)]
pub struct ActionKeys {
    pub jump: KeyCode,
    pub attack: KeyCode,
    pub dash: KeyCode,
    pub secondary: Option<KeyCode>,
    pub quick_action: Option<KeyCode>,
    pub interact: Option<KeyCode>,
    pub modifier: Option<KeyCode>,
    pub utility: Option<KeyCode>,
    pub map: Option<KeyCode>,
    pub inventory: Option<KeyCode>,
    pub pause: KeyCode,
    pub select_reset: KeyCode,
    pub dedicated_pogo: Option<KeyCode>,
}

#[derive(Clone, Copy, Debug)]
pub struct KeyboardPreset {
    pub id: PresetId,
    pub name: &'static str,
    pub movement: MovementKeys,
    pub actions: ActionKeys,
}

impl KeyboardPreset {
    pub fn presets() -> [Self; 4] {
        [
            Self::arrows_zxc(),
            Self::wasd_jkl(),
            Self::arrows_qwer(),
            Self::wasd_uipo(),
        ]
    }

    pub fn arrows_zxc() -> Self {
        Self {
            id: PresetId::ArrowsZxc,
            name: "classic action: arrows + Z/X/C",
            movement: MovementKeys {
                left: KeyCode::ArrowLeft,
                right: KeyCode::ArrowRight,
                up: KeyCode::ArrowUp,
                down: KeyCode::ArrowDown,
            },
            actions: ActionKeys {
                jump: KeyCode::KeyZ,
                attack: KeyCode::KeyX,
                dash: KeyCode::KeyC,
                secondary: Some(KeyCode::KeyA),
                quick_action: Some(KeyCode::KeyE),
                interact: Some(KeyCode::KeyF),
                modifier: Some(KeyCode::KeyS),
                utility: Some(KeyCode::KeyD),
                map: Some(KeyCode::Tab),
                inventory: Some(KeyCode::KeyI),
                pause: KeyCode::Escape,
                select_reset: KeyCode::Delete,
                dedicated_pogo: None,
            },
        }
    }

    pub fn wasd_jkl() -> Self {
        Self {
            id: PresetId::WasdJkl,
            name: "custom PC: WASD + Space/J/K/L/I/U",
            movement: MovementKeys {
                left: KeyCode::KeyA,
                right: KeyCode::KeyD,
                up: KeyCode::KeyW,
                down: KeyCode::KeyS,
            },
            actions: ActionKeys {
                jump: KeyCode::Space,
                attack: KeyCode::KeyJ,
                dash: KeyCode::KeyK,
                secondary: Some(KeyCode::KeyL),
                quick_action: Some(KeyCode::KeyI),
                interact: Some(KeyCode::KeyE),
                modifier: Some(KeyCode::ShiftLeft),
                utility: Some(KeyCode::KeyU),
                map: Some(KeyCode::Tab),
                inventory: Some(KeyCode::KeyV),
                pause: KeyCode::Escape,
                select_reset: KeyCode::Delete,
                dedicated_pogo: None,
            },
        }
    }

    pub fn arrows_qwer() -> Self {
        Self {
            id: PresetId::ArrowsQwer,
            name: "chirality A: arrows + QWER",
            movement: MovementKeys {
                left: KeyCode::ArrowLeft,
                right: KeyCode::ArrowRight,
                up: KeyCode::ArrowUp,
                down: KeyCode::ArrowDown,
            },
            actions: ActionKeys {
                jump: KeyCode::KeyQ,
                dash: KeyCode::KeyW,
                attack: KeyCode::KeyE,
                secondary: Some(KeyCode::KeyR),
                quick_action: None,
                interact: Some(KeyCode::KeyF),
                modifier: Some(KeyCode::ShiftLeft),
                utility: None,
                map: Some(KeyCode::Tab),
                inventory: Some(KeyCode::KeyI),
                pause: KeyCode::Escape,
                select_reset: KeyCode::Delete,
                dedicated_pogo: None,
            },
        }
    }

    pub fn wasd_uipo() -> Self {
        Self {
            id: PresetId::WasdUipo,
            name: "chirality B: WASD + UIPO",
            movement: MovementKeys {
                left: KeyCode::KeyA,
                right: KeyCode::KeyD,
                up: KeyCode::KeyW,
                down: KeyCode::KeyS,
            },
            actions: ActionKeys {
                jump: KeyCode::KeyU,
                dash: KeyCode::KeyI,
                attack: KeyCode::KeyP,
                secondary: Some(KeyCode::KeyO),
                quick_action: None,
                interact: Some(KeyCode::KeyE),
                modifier: Some(KeyCode::ShiftLeft),
                utility: None,
                map: Some(KeyCode::Tab),
                inventory: Some(KeyCode::KeyV),
                pause: KeyCode::Escape,
                select_reset: KeyCode::Delete,
                dedicated_pogo: None,
            },
        }
    }

    pub fn movement_label(&self) -> &'static str {
        match self.id {
            PresetId::ArrowsZxc | PresetId::ArrowsQwer => "Arrow keys",
            PresetId::WasdJkl | PresetId::WasdUipo => "WASD",
        }
    }

    /// Build a fresh Leafwing `InputMap` for this preset.
    ///
    /// Preset cycling swaps this component on the player entity. Keeping the
    /// preset as data means later TOML/RON keybinding config can deserialize
    /// into the same shape instead of rewriting gameplay systems. Gated
    /// behind `input` because the return type is leafwing-owned.
    #[cfg(feature = "input")]
    pub fn input_map(&self) -> InputMap<SandboxAction> {
        let keyboard_move = match self.id {
            PresetId::ArrowsZxc | PresetId::ArrowsQwer => VirtualDPad::arrow_keys(),
            PresetId::WasdJkl | PresetId::WasdUipo => VirtualDPad::wasd(),
        };

        let mut map = InputMap::default()
            .with_dual_axis(SandboxAction::Move, keyboard_move)
            .with_dual_axis(SandboxAction::Move, VirtualDPad::dpad())
            .with_dual_axis(SandboxAction::Move, GamepadStick::LEFT)
            .with(SandboxAction::MoveLeft, self.movement.left)
            .with(SandboxAction::MoveRight, self.movement.right)
            .with(SandboxAction::MoveUp, self.movement.up)
            .with(SandboxAction::MoveDown, self.movement.down)
            .with(SandboxAction::Jump, self.actions.jump)
            .with(SandboxAction::Jump, GamepadButton::South)
            .with(SandboxAction::Attack, self.actions.attack)
            .with(SandboxAction::Attack, GamepadButton::West)
            .with(SandboxAction::Dash, self.actions.dash)
            .with(SandboxAction::Dash, GamepadButton::RightTrigger2)
            .with(SandboxAction::Reset, self.actions.select_reset)
            .with(SandboxAction::Reset, KeyCode::Delete)
            .with(SandboxAction::Reset, KeyCode::Backspace)
            .with(SandboxAction::Reset, GamepadButton::Select)
            .with(SandboxAction::Start, self.actions.pause)
            .with(SandboxAction::Start, GamepadButton::Start);

        insert_optional(&mut map, SandboxAction::Blink, self.actions.secondary);
        insert_optional(
            &mut map,
            SandboxAction::QuickAction,
            self.actions.quick_action,
        );
        insert_optional(&mut map, SandboxAction::Interact, self.actions.interact);
        insert_optional(&mut map, SandboxAction::Modifier, self.actions.modifier);
        insert_optional(&mut map, SandboxAction::Utility, self.actions.utility);
        insert_optional(&mut map, SandboxAction::Map, self.actions.map);
        insert_optional(&mut map, SandboxAction::Inventory, self.actions.inventory);
        insert_optional(&mut map, SandboxAction::Pogo, self.actions.dedicated_pogo);

        map.insert(SandboxAction::Blink, GamepadButton::East);
        map.insert(SandboxAction::QuickAction, GamepadButton::RightTrigger);
        map.insert(SandboxAction::Interact, GamepadButton::RightTrigger);
        map.insert(SandboxAction::Modifier, GamepadButton::LeftTrigger2);
        map.insert(SandboxAction::Utility, GamepadButton::North);
        map.insert(SandboxAction::Map, GamepadButton::LeftTrigger);
        map.insert(SandboxAction::Inventory, GamepadButton::Select);

        // Projectile (Hadouken / fireball) — keyboard `F`, gamepad West.
        // The keyboard binding only matters when the preset already
        // assigns something else to F (interact); leafwing tolerates
        // multi-action sharing.
        map.insert(SandboxAction::Projectile, KeyCode::KeyF);
        map.insert(SandboxAction::Projectile, GamepadButton::West);

        // Menu navigation seam. Cardinal/D-pad/arrow keys all hit the
        // same MenuNavigate* actions; analog stick provides MenuStick
        // for repeat handling, and Enter/Space/South map to MenuSelect.
        map.insert(SandboxAction::MenuNavigateUp, KeyCode::ArrowUp);
        map.insert(SandboxAction::MenuNavigateUp, KeyCode::KeyW);
        map.insert(SandboxAction::MenuNavigateUp, GamepadButton::DPadUp);
        map.insert(SandboxAction::MenuNavigateDown, KeyCode::ArrowDown);
        map.insert(SandboxAction::MenuNavigateDown, KeyCode::KeyS);
        map.insert(SandboxAction::MenuNavigateDown, GamepadButton::DPadDown);
        map.insert(SandboxAction::MenuNavigateLeft, KeyCode::ArrowLeft);
        map.insert(SandboxAction::MenuNavigateLeft, KeyCode::KeyA);
        map.insert(SandboxAction::MenuNavigateLeft, GamepadButton::DPadLeft);
        map.insert(SandboxAction::MenuNavigateRight, KeyCode::ArrowRight);
        map.insert(SandboxAction::MenuNavigateRight, KeyCode::KeyD);
        map.insert(SandboxAction::MenuNavigateRight, GamepadButton::DPadRight);

        map.insert(SandboxAction::MenuSelect, KeyCode::Enter);
        map.insert(SandboxAction::MenuSelect, KeyCode::NumpadEnter);
        map.insert(SandboxAction::MenuSelect, KeyCode::Space);
        map.insert(SandboxAction::MenuSelect, GamepadButton::South);
        // Also accept the player's configured Jump key as confirm so
        // existing muscle memory still works, but Enter is the
        // canonical menu confirmation.
        map.insert(SandboxAction::MenuSelect, self.actions.jump);

        map.insert(SandboxAction::MenuBack, KeyCode::Escape);
        map.insert(SandboxAction::MenuBack, KeyCode::Backspace);
        map.insert(SandboxAction::MenuBack, GamepadButton::East);

        map.insert_dual_axis(SandboxAction::MenuStick, GamepadStick::LEFT);
        map.insert_dual_axis(SandboxAction::AimStick, GamepadStick::RIGHT);
        // RIGHT_Z is the analog right-trigger axis on most pads.
        // Reading it as an axis lets us apply hysteresis ourselves
        // instead of relying on the binary just_pressed edge.
        map.insert_axis(SandboxAction::DashAnalog, GamepadControlAxis::RIGHT_Z);
        map
    }

    pub fn action_label(&self) -> String {
        let mut parts = vec![
            format!("Jump {}", key_name(self.actions.jump)),
            format!("Attack {}", key_name(self.actions.attack)),
            format!("Dash {}", key_name(self.actions.dash)),
        ];
        if let Some(k) = self.actions.dedicated_pogo {
            parts.push(format!("Pogo {}", key_name(k)));
        } else {
            parts.push("Pogo Down+Attack".to_string());
        }
        let optional = [
            ("Blink", self.actions.secondary),
            ("Quick", self.actions.quick_action),
            ("Interact", self.actions.interact),
            ("Modifier", self.actions.modifier),
            ("Fly", self.actions.utility),
            ("Map", self.actions.map),
            ("Inventory", self.actions.inventory),
            ("Select", Some(self.actions.select_reset)),
        ];
        for (label, key) in optional {
            if let Some(k) = key {
                parts.push(format!("{} {}", label, key_name(k)));
            }
        }
        parts.join("  |  ")
    }
}

/// Per-frame snapshot of player input feeding the simulation.
///
/// Stored as a Bevy resource so the visible binary can populate it from
/// `leafwing-input-manager` (presentation-side) and headless tests can
/// pre-populate it directly without an InputPlugin in scope. The
/// simulation reads `Res<ControlFrame>`; it never touches leafwing types
/// — that's the ADR 0012 sim/presentation seam for the input channel.
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
    /// Player projectile / spell action edge.
    pub projectile_pressed: bool,
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
                axis = axis * (WALK_FACTOR / magnitude);
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
            reset_pressed: false,
            control_dt,
        }
    }
}

/// Per-frame menu navigation snapshot. Decoded from `SandboxAction`'s
/// `Menu*` actions plus the analog left-stick (with deadzone + repeat)
/// so the pause-menu controller doesn't have to know about leafwing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MenuInputFrame {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub select: bool,
    pub back: bool,
    pub start: bool,
}

impl MenuInputFrame {
    pub fn any_directional(self) -> bool {
        self.up || self.down || self.left || self.right
    }
}

/// State the menu input system carries across frames so analog repeat
/// behaves predictably.
///
/// `held_dir` records the currently-held direction (or `None`).
/// `time_since_repeat` is the accumulated dt since the last emitted
/// repeat tick. When `held_dir` changes, both timers reset.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MenuInputState {
    pub held_dir: Option<MenuDir>,
    /// Time the current direction has been continuously held. Reset on
    /// new direction.
    held_for_centiseconds: u16,
    /// Time since the last repeat tick was emitted on this direction.
    repeat_accum_centiseconds: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuDir {
    Up,
    Down,
    Left,
    Right,
}

impl MenuInputState {
    /// Resolve a per-frame menu input given the analog stick + button
    /// edge state plus the user's repeat tuning.
    ///
    /// `analog_dir` is the discrete direction the analog stick is
    /// currently pushed toward (after deadzone), or None. `edge_*` are
    /// the discrete edge events from D-pad / arrow keys / WASD.
    pub fn step(
        &mut self,
        edge_up: bool,
        edge_down: bool,
        edge_left: bool,
        edge_right: bool,
        analog_dir: Option<MenuDir>,
        select_pressed: bool,
        back_pressed: bool,
        start_pressed: bool,
        dt_seconds: f32,
        initial_delay: f32,
        repeat_interval: f32,
    ) -> MenuInputFrame {
        // Cardinal edges (D-pad / keyboard) always emit on the press
        // edge regardless of the held analog state. Repeat is reserved
        // for the analog axis so users who hold a stick get predictable
        // pacing rather than cardinal-edge mashing.
        let mut frame = MenuInputFrame {
            up: edge_up,
            down: edge_down,
            left: edge_left,
            right: edge_right,
            select: select_pressed,
            back: back_pressed,
            start: start_pressed,
        };

        match analog_dir {
            Some(dir) if Some(dir) == self.held_dir => {
                // Continuing to hold the same direction: count time
                // toward the next repeat tick.
                self.held_for_centiseconds = self
                    .held_for_centiseconds
                    .saturating_add(centiseconds(dt_seconds));
                let initial_cs = centiseconds(initial_delay);
                if self.held_for_centiseconds >= initial_cs {
                    self.repeat_accum_centiseconds = self
                        .repeat_accum_centiseconds
                        .saturating_add(centiseconds(dt_seconds));
                    let interval_cs = centiseconds(repeat_interval).max(1);
                    if self.repeat_accum_centiseconds >= interval_cs {
                        self.repeat_accum_centiseconds = 0;
                        match dir {
                            MenuDir::Up => frame.up = true,
                            MenuDir::Down => frame.down = true,
                            MenuDir::Left => frame.left = true,
                            MenuDir::Right => frame.right = true,
                        }
                    }
                }
            }
            Some(dir) => {
                // New direction: emit immediately, then wait for the
                // initial delay before repeating.
                self.held_dir = Some(dir);
                self.held_for_centiseconds = 0;
                self.repeat_accum_centiseconds = 0;
                match dir {
                    MenuDir::Up => frame.up = true,
                    MenuDir::Down => frame.down = true,
                    MenuDir::Left => frame.left = true,
                    MenuDir::Right => frame.right = true,
                }
            }
            None => {
                // Analog stick released — reset so the next push fires
                // immediately again.
                self.held_dir = None;
                self.held_for_centiseconds = 0;
                self.repeat_accum_centiseconds = 0;
            }
        }
        frame
    }
}

fn centiseconds(seconds: f32) -> u16 {
    (seconds * 100.0).clamp(0.0, u16::MAX as f32) as u16
}

/// Persistent dash-trigger edge state. Lives outside `ControlFrame`
/// because the hysteresis logic must remember the previous state across
/// frames; `ControlFrame` is stateless and rebuilt every frame.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlayerDashTriggerState {
    pub edge: crate::settings::TriggerEdgeState,
}

/// Convert an analog stick vector (post-deadzone) into a single
/// discrete direction. Returns `None` when below `threshold`.
pub fn analog_to_dir(x: f32, y: f32, threshold: f32) -> Option<MenuDir> {
    let mag = (x * x + y * y).sqrt();
    if mag < threshold {
        return None;
    }
    if x.abs() > y.abs() {
        if x > 0.0 {
            Some(MenuDir::Right)
        } else {
            Some(MenuDir::Left)
        }
    } else if y > 0.0 {
        Some(MenuDir::Up)
    } else {
        Some(MenuDir::Down)
    }
}

pub const GAMEPAD_MAP: &[(&str, &str)] = &[
    ("L-stick / D-pad", "movement / aim"),
    ("A / Cross", "jump / confirm"),
    ("X / Square", "primary attack"),
    ("RT / R2", "dash"),
    ("B / Circle", "blink / special"),
    ("RB / R1", "interact / quick action"),
    ("LT / L2", "modifier placeholder"),
    ("Y / Triangle", "fly toggle / utility"),
    ("LB / L1", "map placeholder"),
    ("Back / Touchpad", "inventory or sandbox reset"),
    ("Start / Options", "pause / menu"),
];

#[cfg(feature = "input")]
fn insert_optional(map: &mut InputMap<SandboxAction>, action: SandboxAction, key: Option<KeyCode>) {
    if let Some(key) = key {
        map.insert(action, key);
    }
}

fn key_name(key: KeyCode) -> &'static str {
    match key {
        KeyCode::KeyA => "A",
        KeyCode::KeyB => "B",
        KeyCode::KeyC => "C",
        KeyCode::KeyD => "D",
        KeyCode::KeyE => "E",
        KeyCode::KeyF => "F",
        KeyCode::KeyG => "G",
        KeyCode::KeyH => "H",
        KeyCode::KeyI => "I",
        KeyCode::KeyJ => "J",
        KeyCode::KeyK => "K",
        KeyCode::KeyL => "L",
        KeyCode::KeyM => "M",
        KeyCode::KeyN => "N",
        KeyCode::KeyO => "O",
        KeyCode::KeyP => "P",
        KeyCode::KeyQ => "Q",
        KeyCode::KeyR => "R",
        KeyCode::KeyS => "S",
        KeyCode::KeyT => "T",
        KeyCode::KeyU => "U",
        KeyCode::KeyV => "V",
        KeyCode::KeyW => "W",
        KeyCode::KeyX => "X",
        KeyCode::KeyY => "Y",
        KeyCode::KeyZ => "Z",
        KeyCode::ArrowLeft => "Left",
        KeyCode::ArrowRight => "Right",
        KeyCode::ArrowUp => "Up",
        KeyCode::ArrowDown => "Down",
        KeyCode::Space => "Space",
        KeyCode::ShiftLeft => "LShift",
        KeyCode::Tab => "Tab",
        KeyCode::Escape => "Esc",
        KeyCode::Delete => "Delete",
        KeyCode::Backspace => "Backspace",
        _ => "?",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::ControlSettings;

    #[test]
    fn analog_drift_below_deadzone_zeros_movement() {
        // Simulated worn Xbox 360 controller with a small +Y bias.
        let (x, y) = ControlSettings::apply_deadzone(0.04, 0.06, 0.18);
        assert_eq!((x, y), (0.0, 0.0));
        // The same drift fed to analog_to_dir must not pick a direction.
        assert!(analog_to_dir(x, y, 0.5).is_none());
    }

    #[test]
    fn analog_to_dir_picks_dominant_axis() {
        assert_eq!(analog_to_dir(0.8, 0.1, 0.5), Some(MenuDir::Right));
        assert_eq!(analog_to_dir(-0.8, -0.1, 0.5), Some(MenuDir::Left));
        // +y is up in the leafwing convention used here.
        assert_eq!(analog_to_dir(0.1, 0.8, 0.5), Some(MenuDir::Up));
        assert_eq!(analog_to_dir(0.1, -0.8, 0.5), Some(MenuDir::Down));
    }

    #[test]
    fn menu_state_emits_first_press_then_waits_for_initial_delay() {
        let mut state = MenuInputState::default();
        // First frame holding Down: emit immediately.
        let f = state.step(
            false,
            false,
            false,
            false,
            Some(MenuDir::Down),
            false,
            false,
            false,
            0.016,
            0.30,
            0.10,
        );
        assert!(f.down);
        // Continuing to hold for less than the initial delay must not
        // re-emit.
        let mut emits = 0;
        for _ in 0..5 {
            let f = state.step(
                false,
                false,
                false,
                false,
                Some(MenuDir::Down),
                false,
                false,
                false,
                0.016,
                0.30,
                0.10,
            );
            if f.down {
                emits += 1;
            }
        }
        assert_eq!(emits, 0, "should not repeat before initial delay elapses");
    }

    #[test]
    fn menu_state_repeats_after_initial_delay() {
        let mut state = MenuInputState::default();
        // First push to start the hold.
        let _ = state.step(
            false,
            false,
            false,
            false,
            Some(MenuDir::Right),
            false,
            false,
            false,
            0.016,
            0.10,
            0.05,
        );
        let mut emits = 0;
        for _ in 0..40 {
            let f = state.step(
                false,
                false,
                false,
                false,
                Some(MenuDir::Right),
                false,
                false,
                false,
                0.016,
                0.10,
                0.05,
            );
            if f.right {
                emits += 1;
            }
        }
        assert!(emits >= 4, "expected several repeat ticks; got {emits}");
    }

    #[test]
    fn cardinal_edges_pass_through_without_repeat_state() {
        let mut state = MenuInputState::default();
        // D-pad / arrow keys edge fires on one frame but does not start
        // an analog hold.
        let f = state.step(
            true, false, false, false, None, false, false, false, 0.016, 0.30, 0.10,
        );
        assert!(f.up);
        let f = state.step(
            false, false, false, false, None, false, false, false, 0.016, 0.30, 0.10,
        );
        assert!(!f.any_directional());
    }

    #[test]
    fn menu_state_select_passes_through() {
        let mut state = MenuInputState::default();
        let f = state.step(
            false, false, false, false, None, true, false, false, 0.016, 0.30, 0.10,
        );
        assert!(f.select);
        assert!(!f.any_directional());
    }

    #[test]
    fn menu_state_back_passes_through() {
        let mut state = MenuInputState::default();
        let f = state.step(
            false, false, false, false, None, false, true, false, 0.016, 0.30, 0.10,
        );
        assert!(f.back);
    }
}

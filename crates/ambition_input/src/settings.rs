//! Controls / input settings.
//!
//! Holds controller deadzones, trigger thresholds, hysteresis, burst
//! input behavior, which keyboard / controller profile is active, and the
//! per-action binding OVERRIDES layered on top of that profile.
//! The values flow into input filtering before the engine-owned `ControlFrame`
//! is built so gameplay sees clean edges instead of analog jitter.

use bevy::prelude::{GamepadButton, KeyCode};
use serde::{Deserialize, Serialize};

/// How a tap or mouse click on a menu item should behave.
///
/// All three modes share the same hover semantic (pointer-over moves
/// the highlight); they differ only in what a *press* does.
///
/// - `SingleTapWithDestructiveGuard` (default on desktop): non-destructive items
///   activate on the first press. Destructive items (Quit, Reset
///   Sandbox) only highlight on the first press; a second press on the
///   same item activates. Matches the safety/expectation balance most
///   players want on touch.
/// - `SingleTap`: every press activates immediately. Faster, but a
///   stray touch on Quit will exit the game.
/// - `TapToSelectThenConfirm` (default on Android): first press only
///   highlights; a second press on the same item activates. Console-style;
///   fewer mistaps.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MenuTapMode {
    SingleTapWithDestructiveGuard,
    SingleTap,
    TapToSelectThenConfirm,
}

impl Default for MenuTapMode {
    fn default() -> Self {
        // One default on every platform. `SingleTapWithDestructiveGuard` already
        // guards the presses that need it, such as a stray touch on Quit. A press
        // that turns into a drag belongs to the gesture layer, which reads it as a
        // scroll.
        Self::SingleTapWithDestructiveGuard
    }
}

impl MenuTapMode {
    pub const ALL: [Self; 3] = [
        Self::SingleTapWithDestructiveGuard,
        Self::SingleTap,
        Self::TapToSelectThenConfirm,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::SingleTapWithDestructiveGuard => "single tap (guard quit)",
            Self::SingleTap => "single tap",
            Self::TapToSelectThenConfirm => "tap, then tap to confirm",
        }
    }

    pub fn next(self) -> Self {
        let idx = Self::ALL.iter().position(|p| p == &self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }

    pub fn prev(self) -> Self {
        let idx = Self::ALL.iter().position(|p| p == &self).unwrap_or(0);
        Self::ALL[(idx + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

/// Outcome of a pointer press on a menu row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuPointerPress {
    /// Move the highlight to this row only — do not confirm.
    SelectOnly,
    /// Move the highlight and confirm this row.
    Confirm,
}

impl MenuTapMode {
    /// Decide what a pointer press on `target` should do, given the current
    /// selection and whether the target is destructive.
    ///
    /// `Row` is an opaque identity, not an ordinate: this only asks whether two
    /// presses hit the same row. An index-addressed menu passes `usize`; the
    /// pointer bridge passes the row's action. Both callers share one
    /// destructive guard.
    ///
    /// `armed` tracks "this destructive row was selected by a prior
    /// press and is awaiting a confirm tap". The function may clear or
    /// set it.
    pub fn resolve_press<Row: Clone + PartialEq>(
        self,
        target: Row,
        currently_selected: &Row,
        is_destructive: bool,
        armed: &mut Option<Row>,
    ) -> MenuPointerPress {
        let armed_here = armed.as_ref() == Some(&target);
        match self {
            Self::SingleTap => {
                *armed = None;
                MenuPointerPress::Confirm
            }
            Self::TapToSelectThenConfirm => {
                if *currently_selected == target && armed_here {
                    *armed = None;
                    MenuPointerPress::Confirm
                } else {
                    *armed = Some(target);
                    MenuPointerPress::SelectOnly
                }
            }
            Self::SingleTapWithDestructiveGuard => {
                let confirm_now = !is_destructive || (*currently_selected == target && armed_here);
                if confirm_now {
                    *armed = None;
                    MenuPointerPress::Confirm
                } else {
                    *armed = Some(target);
                    MenuPointerPress::SelectOnly
                }
            }
        }
    }
}

/// Whether the burst press (the shared dodge/dash button) fires from the
/// right trigger, the right shoulder button, or both.
///
/// Stored in [`ControlSettings::burst_input_mode`], whose serde name is pinned.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BurstInputMode {
    /// Right trigger 2 (RT/R2). Default; matches prior behavior.
    #[default]
    Trigger,
    /// Right shoulder button (RB/R1).
    Button,
    /// Either input fires the burst.
    Both,
}

/// What the right stick does during gameplay.
///
/// This is a mode, not a second reader, because the right stick also aims the
/// blink. One deflection cannot both aim and attack.
///
/// The two attack modes are not cosmetic. A tilt stick throws a tilt at full
/// deflection, where a movement stick reads a smash. A smash stick throws a
/// smash at a gentle push. Each mode forces a strength the other cannot give,
/// so [`ambition_platformer2d_core::AttackStrengthHint`] carries both.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RightStickMode {
    /// Aim the blink. Default, and what the stick has always done.
    #[default]
    Aim,
    /// Flicking the stick throws a tilt in that direction, at any deflection.
    TiltAttack,
    /// Flicking the stick throws a smash in that direction, at any deflection.
    SmashAttack,
}

impl RightStickMode {
    pub const ALL: [Self; 3] = [Self::Aim, Self::TiltAttack, Self::SmashAttack];

    pub fn label(self) -> &'static str {
        match self {
            Self::Aim => "aim",
            Self::TiltAttack => "tilt attack",
            Self::SmashAttack => "smash attack",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Aim => Self::TiltAttack,
            Self::TiltAttack => Self::SmashAttack,
            Self::SmashAttack => Self::Aim,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Aim => Self::SmashAttack,
            Self::TiltAttack => Self::Aim,
            Self::SmashAttack => Self::TiltAttack,
        }
    }

    /// The strength a press from this stick asks for, or `None` when the stick
    /// is not an attack stick at all.
    pub fn attack_strength(self) -> Option<ambition_platformer2d_core::AttackStrengthHint> {
        match self {
            Self::Aim => None,
            Self::TiltAttack => Some(ambition_platformer2d_core::AttackStrengthHint::Tilt),
            Self::SmashAttack => Some(ambition_platformer2d_core::AttackStrengthHint::Smash),
        }
    }
}

impl BurstInputMode {
    pub const ALL: [Self; 3] = [Self::Trigger, Self::Button, Self::Both];

    pub fn label(self) -> &'static str {
        match self {
            Self::Trigger => "trigger",
            Self::Button => "button",
            Self::Both => "both",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Trigger => Self::Button,
            Self::Button => Self::Both,
            Self::Both => Self::Trigger,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Trigger => Self::Both,
            Self::Button => Self::Trigger,
            Self::Both => Self::Button,
        }
    }
}

/// Controller profile vocabulary. Today the sandbox doesn't switch
/// gamepad layouts dynamically, but the field is here so future
/// patches can add real per-pad profiles without restructuring.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControllerProfileId {
    #[default]
    Default,
    Xbox360,
    XboxOne,
    PlayStation,
    Generic,
}

/// Per-profile defaults (deadzone, trigger thresholds). Returned by
/// [`ControllerProfileId::filter_defaults`] and applied by
/// [`ControlSettings::apply_profile_defaults`].
///
/// Captures known per-pad characteristics:
/// - `Xbox360` ships with notoriously drifty thumbsticks; default
///   deadzones are ~50% wider than the generic baseline.
/// - `Xbox360` analog triggers also tend to rest at non-zero values
///   when slightly worn; the press threshold is bumped accordingly.
/// - `PlayStation` (DualShock 4 / DualSense) sticks are tighter from
///   the factory, so the default deadzone is slightly *smaller* than
///   the generic baseline.
/// - `XboxOne` / `Generic` use the same baseline as `Default`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProfileFilterDefaults {
    pub left_stick_deadzone: f32,
    pub right_stick_deadzone: f32,
    pub trigger_release_threshold: f32,
    pub trigger_press_threshold: f32,
}

impl ProfileFilterDefaults {
    /// The baseline stick/trigger filter values — single source of truth shared
    /// by `ControlSettings::default` and the Default/XboxOne/Generic profile arms
    /// of [`ControllerProfileId::filter_defaults`]. Calibrated profiles override.
    pub const BASELINE: Self = Self {
        left_stick_deadzone: 0.18,
        right_stick_deadzone: 0.20,
        trigger_release_threshold: 0.30,
        trigger_press_threshold: 0.55,
    };
}

impl ControllerProfileId {
    pub const ALL: [Self; 5] = [
        Self::Default,
        Self::Xbox360,
        Self::XboxOne,
        Self::PlayStation,
        Self::Generic,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Xbox360 => "xbox 360",
            Self::XboxOne => "xbox one",
            Self::PlayStation => "playstation",
            Self::Generic => "generic",
        }
    }

    /// Per-profile filter defaults. The `Default` baseline matches
    /// `ControlSettings::default()`; other profiles override with
    /// values calibrated to that pad's known drift characteristics.
    pub fn filter_defaults(self) -> ProfileFilterDefaults {
        match self {
            Self::Xbox360 => ProfileFilterDefaults {
                // 360 sticks drift; bump deadzones ~50% over baseline.
                left_stick_deadzone: 0.27,
                right_stick_deadzone: 0.30,
                // Worn triggers can rest at ~0.10; widen the
                // hysteresis band so a held trigger never re-fires.
                trigger_release_threshold: 0.20,
                trigger_press_threshold: 0.65,
            },
            Self::PlayStation => ProfileFilterDefaults {
                // DualShock 4 / DualSense sticks tighter than baseline.
                left_stick_deadzone: 0.14,
                right_stick_deadzone: 0.16,
                trigger_release_threshold: 0.30,
                trigger_press_threshold: 0.55,
            },
            // Default / XboxOne / Generic share the baseline.
            _ => ProfileFilterDefaults::BASELINE,
        }
    }

    pub fn next(self) -> Self {
        let idx = Self::ALL.iter().position(|p| p == &self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }

    pub fn prev(self) -> Self {
        let idx = Self::ALL.iter().position(|p| p == &self).unwrap_or(0);
        Self::ALL[(idx + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

/// The values the device reader filters with.
///
/// A deadzone is a property of the stick, not of the player.
///
/// `Copy` because it is rebuilt per seat per frame. Cloning `ControlSettings`
/// would allocate its binding-override `Vec` each time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ControlFilters {
    pub left_stick_deadzone: f32,
    pub right_stick_deadzone: f32,
    pub trigger_release_threshold: f32,
    pub trigger_press_threshold: f32,
    /// A preference, not a calibration. Which control means burst is a choice of
    /// the player, so it stays machine-wide even per pad.
    pub burst_input_mode: BurstInputMode,
    /// What the right stick is for — see [`RightStickMode`].
    pub right_stick_mode: RightStickMode,
    /// Also a preference. Inverted aim is a habit, not a hardware property.
    pub invert_aim_y: bool,
}

impl ControlFilters {
    /// The machine-wide values from the settings screen. The primary seat uses
    /// these.
    pub fn from_settings(settings: &ControlSettings) -> Self {
        Self {
            left_stick_deadzone: settings.left_stick_deadzone,
            right_stick_deadzone: settings.right_stick_deadzone,
            trigger_release_threshold: settings.trigger_release_threshold,
            trigger_press_threshold: settings.trigger_press_threshold,
            burst_input_mode: settings.burst_input_mode,
            right_stick_mode: settings.right_stick_mode,
            invert_aim_y: settings.invert_aim_y,
        }
    }

    /// Calibrated for a pad of this vendor style, keeping the machine-wide
    /// preferences.
    ///
    /// An explicit profile choice in settings wins over detection. Only
    /// `Default` ("nobody chose") defers to the pad.
    pub fn for_pad(settings: &ControlSettings, style: crate::GamepadStyle) -> Self {
        let mut filters = Self::from_settings(settings);
        if settings.controller_profile != ControllerProfileId::Default {
            return filters;
        }
        let calibrated = profile_for_pad(style).filter_defaults();
        filters.left_stick_deadzone = calibrated.left_stick_deadzone;
        filters.right_stick_deadzone = calibrated.right_stick_deadzone;
        filters.trigger_release_threshold = calibrated.trigger_release_threshold;
        filters.trigger_press_threshold = calibrated.trigger_press_threshold;
        filters
    }
}

/// Which calibration table a detected pad style gets.
///
/// Detection never gives `Xbox360`. `gamepad_style_of` reads Microsoft's
/// vendor id, which 360 and Series pads share. The 360 table (drifty sticks,
/// worn triggers) would widen the deadzone on a new pad. It stays available
/// as an explicit settings choice.
fn profile_for_pad(style: crate::GamepadStyle) -> ControllerProfileId {
    match style {
        crate::GamepadStyle::PlayStation => ControllerProfileId::PlayStation,
        crate::GamepadStyle::XboxLike => ControllerProfileId::XboxOne,
        crate::GamepadStyle::Switch | crate::GamepadStyle::Generic => ControllerProfileId::Generic,
    }
}

/// A control an override can name.
///
/// This is not `PhysicalControl`. That type reads out of a live `InputMap`,
/// so it must be total and has an `Other(String)` arm. This type is written
/// into a map, so every value must be bindable, and it has no `Other` arm.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OverrideControl {
    Key(KeyCode),
    Button(GamepadButton),
}

impl OverrideControl {
    /// Which half of a preset this override replaces. A preset binds most
    /// actions on a key and a pad button. Remapping Jump to `J` must not unbind
    /// the controller.
    pub fn device_class(self) -> OverrideDeviceClass {
        match self {
            Self::Key(_) => OverrideDeviceClass::Keyboard,
            Self::Button(_) => OverrideDeviceClass::Gamepad,
        }
    }
}

/// The half of an action's bindings an [`OverrideControl`] speaks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OverrideDeviceClass {
    Keyboard,
    Gamepad,
}

/// One persisted override: "this action is on this control instead".
///
/// The action is the `Debug` spelling that [`crate::ActionBindings`]
/// publishes, so settings files, trace lines and rebind rows share one id.
/// It is a `String` because this module compiles without the `input`
/// feature, where the leafwing action enum does not exist. The settings file
/// must deserialize the same in both builds.
///
/// An override that names an unknown action is ignored, not an error, so a
/// file from a newer build still loads.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BindingOverride {
    pub action: String,
    pub control: OverrideControl,
}

impl BindingOverride {
    pub fn key(action: impl Into<String>, key: KeyCode) -> Self {
        Self {
            action: action.into(),
            control: OverrideControl::Key(key),
        }
    }

    pub fn button(action: impl Into<String>, button: GamepadButton) -> Self {
        Self {
            action: action.into(),
            control: OverrideControl::Button(button),
        }
    }
}

/// Not `Copy`: it holds the binding overrides in a `Vec`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ControlSettings {
    /// Active keyboard preset index (matches `KeyboardPreset::presets()`).
    pub keyboard_preset_index: usize,
    pub controller_profile: ControllerProfileId,
    /// Radial deadzone for the left analog stick. Magnitudes below this
    /// are treated as zero before being fed to gameplay or the menu.
    pub left_stick_deadzone: f32,
    /// Radial deadzone for the right analog stick / aim.
    pub right_stick_deadzone: f32,
    /// Lower hysteresis bound for the right trigger (Burst). The trigger
    /// must drop below this to "release"; pulling back above
    /// `trigger_press_threshold` re-arms a press edge.
    pub trigger_release_threshold: f32,
    /// Upper hysteresis bound for the right trigger (Burst).
    pub trigger_press_threshold: f32,
    /// Whether the D-pad navigates menus (in addition to the analog
    /// stick / arrow keys).
    pub dpad_menu_navigation: bool,
    /// Invert vertical aim (right stick / aim binding).
    pub invert_aim_y: bool,
    /// Which control fires the shared dodge/dash burst press.
    ///
    /// The serde name is pinned to `dash_input_mode`. This field has no
    /// `#[serde(default)]` and the struct has no container default, so a missing
    /// key fails the whole struct. `load_settings` then discards the entire
    /// settings file and returns `UserSettings::default()`. Renaming the key
    /// would wipe the settings of every existing player.
    #[serde(rename = "dash_input_mode")]
    pub burst_input_mode: BurstInputMode,
    /// What the right stick is for; see [`RightStickMode`].
    ///
    /// `#[serde(default)]` is required. Older settings files do not have this
    /// key, and a missing key without a default discards the whole settings file
    /// (see `burst_input_mode`).
    #[serde(default)]
    pub right_stick_mode: RightStickMode,
    /// Initial repeat delay for held menu directions, in seconds.
    pub menu_repeat_initial_delay: f32,
    /// Repeat interval after the initial delay.
    pub menu_repeat_interval: f32,
    /// Whether the on-screen touch overlay (joystick and action buttons) is
    /// visible. Mirrors into the `TouchControlsVisible` resource from
    /// `TouchControlsPlugin`. This does not enable touch input; the plugin owns
    /// that (installed means enabled). Default true. Turn it off to test
    /// keyboard-only on desktop; touch input stays live.
    #[serde(default = "default_touch_controls_visible")]
    pub touch_controls_visible: bool,
    /// How a tap or mouse click on a menu item should behave. See
    /// [`MenuTapMode`] for semantics.
    #[serde(default)]
    pub menu_tap_mode: MenuTapMode,
    /// Per-action binding overrides on top of
    /// [`Self::keyboard_preset_index`]'s preset.
    ///
    /// The preset gives the start binding of every action; an override moves one
    /// action. Storing the whole rebuilt map would freeze a player at the preset
    /// they edited, so later preset changes could not reach them.
    ///
    /// `serde(default)` so older files load. An empty list means no overrides.
    #[serde(default)]
    pub binding_overrides: Vec<BindingOverride>,
}

fn default_touch_controls_visible() -> bool {
    true
}

impl Default for ControlSettings {
    fn default() -> Self {
        Self {
            keyboard_preset_index: 0,
            controller_profile: ControllerProfileId::default(),
            left_stick_deadzone: ProfileFilterDefaults::BASELINE.left_stick_deadzone,
            right_stick_deadzone: ProfileFilterDefaults::BASELINE.right_stick_deadzone,
            trigger_release_threshold: ProfileFilterDefaults::BASELINE.trigger_release_threshold,
            trigger_press_threshold: ProfileFilterDefaults::BASELINE.trigger_press_threshold,
            dpad_menu_navigation: true,
            invert_aim_y: false,
            burst_input_mode: BurstInputMode::default(),
            right_stick_mode: RightStickMode::default(),
            menu_repeat_initial_delay: 0.32,
            menu_repeat_interval: 0.12,
            touch_controls_visible: default_touch_controls_visible(),
            menu_tap_mode: MenuTapMode::default(),
            binding_overrides: Vec::new(),
        }
    }
}

impl ControlSettings {
    /// Apply the active controller profile's deadzones and trigger thresholds
    /// over the stored values, so a profile change takes effect at once.
    pub fn apply_profile_defaults(&mut self) {
        let p = self.controller_profile.filter_defaults();
        self.left_stick_deadzone = p.left_stick_deadzone;
        self.right_stick_deadzone = p.right_stick_deadzone;
        self.trigger_release_threshold = p.trigger_release_threshold;
        self.trigger_press_threshold = p.trigger_press_threshold;
    }

    /// Set a per-action binding override. Replace any earlier override for the
    /// same action and device class.
    ///
    /// Keep one override per (action, class), so the winner never depends on
    /// insertion order.
    pub fn set_binding_override(&mut self, over: BindingOverride) {
        let class = over.control.device_class();
        self.binding_overrides
            .retain(|held| held.action != over.action || held.control.device_class() != class);
        self.binding_overrides.push(over);
    }

    /// Drop the override for one action and device class, returning that
    /// action to whatever the preset binds it to.
    pub fn clear_binding_override(&mut self, action: &str, class: OverrideDeviceClass) {
        self.binding_overrides
            .retain(|held| held.action != action || held.control.device_class() != class);
    }

    /// Forget every override, so the active preset alone decides the bindings.
    pub fn reset_binding_overrides(&mut self) {
        self.binding_overrides.clear();
    }

    /// Restore the deadzone, trigger and repeat values to their defaults. Keep
    /// the controller and keyboard profile selection.
    ///
    /// The `ResetControlFiltering` row calls this. It keeps the preset and the
    /// binding overrides; [`Self::reset_binding_overrides`] forgets a remap.
    pub fn reset_filtering_to_defaults(&mut self) {
        let defaults = Self::default();
        self.left_stick_deadzone = defaults.left_stick_deadzone;
        self.right_stick_deadzone = defaults.right_stick_deadzone;
        self.trigger_release_threshold = defaults.trigger_release_threshold;
        self.trigger_press_threshold = defaults.trigger_press_threshold;
        self.dpad_menu_navigation = defaults.dpad_menu_navigation;
        self.invert_aim_y = defaults.invert_aim_y;
        self.burst_input_mode = defaults.burst_input_mode;
        self.menu_repeat_initial_delay = defaults.menu_repeat_initial_delay;
        self.menu_repeat_interval = defaults.menu_repeat_interval;
    }

    pub fn clamp_all(&mut self) {
        self.migrate_renamed_actions();
        self.left_stick_deadzone = self.left_stick_deadzone.clamp(0.0, 0.95);
        self.right_stick_deadzone = self.right_stick_deadzone.clamp(0.0, 0.95);
        self.trigger_release_threshold = self.trigger_release_threshold.clamp(0.0, 0.95);
        // Press threshold must be greater than release for usable hysteresis.
        let press_lower = (self.trigger_release_threshold + 0.05).min(0.95);
        self.trigger_press_threshold = self.trigger_press_threshold.clamp(press_lower, 1.0);
        self.menu_repeat_initial_delay = self.menu_repeat_initial_delay.clamp(0.05, 1.5);
        self.menu_repeat_interval = self.menu_repeat_interval.clamp(0.02, 1.0);
    }

    /// Carry a stored remap across an action rename.
    ///
    /// [`BindingOverride::action`] is the action's `Debug` spelling, and
    /// `bindings::apply_override` ignores names this build does not have, so
    /// files from newer builds load. Thus a renamed action's override stops
    /// applying, with no log. Add each rename to this table.
    ///
    /// [`Self::clamp_all`] runs this on every load, so the stored name is
    /// rewritten and a later rename needs no two-step chain.
    fn migrate_renamed_actions(&mut self) {
        const RENAMED_ACTIONS: &[(&str, &str)] = &[("QuickAction", "Shield"), ("Dash", "Burst")];

        for over in &mut self.binding_overrides {
            if let Some((_, now)) = RENAMED_ACTIONS
                .iter()
                .find(|(was, _)| *was == over.action.as_str())
            {
                (*now).clone_into(&mut over.action);
            }
        }
        // A file can hold both names for one action. Keep the last row per
        // (action, device class), as `set_binding_override` does.
        let mut seen = Vec::new();
        self.binding_overrides.reverse();
        self.binding_overrides.retain(|over| {
            let key = (over.action.clone(), over.control.device_class());
            if seen.contains(&key) {
                false
            } else {
                seen.push(key);
                true
            }
        });
        self.binding_overrides.reverse();
    }

    /// Apply a radial deadzone to a 2D stick vector.
    ///
    /// Below `deadzone` the output is zero; above the magnitude is
    /// rescaled into `[0, 1]` so the analog response is smooth.
    pub fn apply_deadzone(x: f32, y: f32, deadzone: f32) -> (f32, f32) {
        let mag = (x * x + y * y).sqrt();
        if mag <= deadzone || deadzone >= 1.0 {
            return (0.0, 0.0);
        }
        let scaled = ((mag - deadzone) / (1.0 - deadzone)).clamp(0.0, 1.0);
        let inv = scaled / mag;
        (x * inv, y * inv)
    }
}

/// Edge state for one analog input that gives "just pressed" events with
/// hysteresis. It has no Bevy types, so keyboard code, gamepad triggers and
/// tests can share it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TriggerEdgeState {
    #[default]
    Released,
    Pressed,
}

/// The per-device analog edges a gameplay frame remembers between ticks.
///
/// One carrier, because the caller stores one value back into its resource.
/// A C-stick flick uses the same mechanism as the burst trigger: an analog
/// value crosses a press threshold from rest, with hysteresis so a held
/// stick does not re-fire.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GameplayEdgeState {
    /// The burst trigger's hysteretic press.
    pub burst: TriggerEdgeState,
    /// The aim stick's deflection when it is an attack stick. Idle in
    /// [`RightStickMode::Aim`], the default.
    pub aim_stick: TriggerEdgeState,
}

/// How far the aim stick must be pushed to throw a C-stick attack.
///
/// Above the deadzone and below full deflection. Any deflection past this
/// throws the mode's strength, so this is an intent gate, not a strength
/// threshold. Release uses the stick deadzone, so the gesture re-arms when
/// the stick reads as centred.
pub const AIM_STICK_ATTACK_THRESHOLD: f32 = 0.5;

/// Update a hysteretic trigger edge.
///
/// Returns `(new_state, just_pressed)`. The "press" edge fires when the
/// previous state is `Released` and the current value rises above
/// `press`; the "release" edge fires when the value drops below
/// `release`. Values between the two thresholds preserve the previous
/// state — that's the hysteresis that prevents jitter from producing
/// repeated edges while a Burst trigger is held.
pub fn update_trigger_edge(
    previous: TriggerEdgeState,
    value: f32,
    release: f32,
    press: f32,
) -> (TriggerEdgeState, bool) {
    debug_assert!(release <= press, "release must be <= press");
    match previous {
        TriggerEdgeState::Released => {
            if value >= press {
                (TriggerEdgeState::Pressed, true)
            } else {
                (TriggerEdgeState::Released, false)
            }
        }
        TriggerEdgeState::Pressed => {
            if value <= release {
                (TriggerEdgeState::Released, false)
            } else {
                (TriggerEdgeState::Pressed, false)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_drift_zeros_out_under_deadzone() {
        let (x, y) = ControlSettings::apply_deadzone(0.05, -0.04, 0.18);
        assert_eq!((x, y), (0.0, 0.0));
    }

    #[test]
    fn deadzone_rescales_above_threshold() {
        let (x, y) = ControlSettings::apply_deadzone(0.5, 0.0, 0.2);
        assert!(x > 0.0);
        assert!((y).abs() < 1e-6);
        assert!(x < 0.5, "value should have lost its dead band");
    }

    #[test]
    fn deadzone_unit_input_stays_unit() {
        // `(1.0, 0.0)` magnitude is 1.0; rescale should hand back unit-magnitude
        // direction even with a substantial deadzone.
        let (x, y) = ControlSettings::apply_deadzone(1.0, 0.0, 0.3);
        assert!((x - 1.0).abs() < 1e-6);
        assert!(y.abs() < 1e-6);
    }

    #[test]
    fn trigger_jitter_does_not_repeat_press_edges() {
        // Mimic a worn trigger that crosses ~0.55 repeatedly while the
        // user holds it just above the threshold.
        let mut state = TriggerEdgeState::default();
        let mut press_edges = 0;
        let release = 0.30;
        let press = 0.55;
        for value in [0.40, 0.60, 0.70, 0.65, 0.58, 0.62, 0.59, 0.56, 0.61] {
            let (next, just_pressed) = update_trigger_edge(state, value, release, press);
            if just_pressed {
                press_edges += 1;
            }
            state = next;
        }
        assert_eq!(
            press_edges, 1,
            "hysteresis must collapse jitter into a single press edge"
        );
    }

    #[test]
    fn trigger_release_then_repress_fires_again() {
        let mut state = TriggerEdgeState::default();
        let mut press_edges = 0;
        for value in [0.0, 0.6, 0.0, 0.6, 0.0] {
            let (next, just_pressed) = update_trigger_edge(state, value, 0.30, 0.55);
            if just_pressed {
                press_edges += 1;
            }
            state = next;
        }
        assert_eq!(press_edges, 2);
    }

    #[test]
    fn clamp_keeps_press_above_release() {
        let mut s = ControlSettings::default();
        s.trigger_release_threshold = 0.9;
        s.trigger_press_threshold = 0.4;
        s.clamp_all();
        assert!(
            s.trigger_press_threshold > s.trigger_release_threshold,
            "press {} must end up above release {}",
            s.trigger_press_threshold,
            s.trigger_release_threshold
        );
    }

    #[test]
    fn burst_mode_cycles_through_all() {
        let mut visited = std::collections::HashSet::new();
        let mut cur = BurstInputMode::default();
        for _ in 0..BurstInputMode::ALL.len() {
            visited.insert(cur);
            cur = cur.next();
        }
        assert_eq!(visited.len(), BurstInputMode::ALL.len());
    }

    #[test]
    fn xbox360_profile_widens_deadzone_and_trigger_band() {
        let baseline = ControllerProfileId::Default.filter_defaults();
        let xbox360 = ControllerProfileId::Xbox360.filter_defaults();
        // Xbox 360 sticks drift more than baseline; deadzones must
        // be wider, never narrower, than the default.
        assert!(xbox360.left_stick_deadzone > baseline.left_stick_deadzone);
        assert!(xbox360.right_stick_deadzone > baseline.right_stick_deadzone);
        // Worn trigger compensation: hysteresis band wider than
        // baseline (release lower, press higher).
        assert!(xbox360.trigger_release_threshold < baseline.trigger_release_threshold);
        assert!(xbox360.trigger_press_threshold > baseline.trigger_press_threshold);
    }

    /// A seat's deadzone follows the pad in its hands, not the machine.
    #[test]
    fn a_seats_filtering_follows_its_own_pad() {
        let mut settings = ControlSettings::default();
        // The primary has hand-tuned their sticks wide open.
        settings.left_stick_deadzone = 0.40;
        settings.right_stick_deadzone = 0.40;

        let primary = ControlFilters::from_settings(&settings);
        assert_eq!(
            primary.left_stick_deadzone, 0.40,
            "the settings sliders are the primary's own, untouched"
        );

        let couch = ControlFilters::for_pad(&settings, crate::GamepadStyle::PlayStation);
        assert_ne!(
            couch.left_stick_deadzone, primary.left_stick_deadzone,
            "a DualSense on seat two does not inherit the primary's 0.40"
        );
        assert_eq!(
            couch.left_stick_deadzone,
            ControllerProfileId::PlayStation
                .filter_defaults()
                .left_stick_deadzone,
            "it gets the calibration table for the pad it actually is"
        );
        assert_eq!(
            couch.burst_input_mode, primary.burst_input_mode,
            "PREFERENCES stay machine-wide — which button bursts is about the \
             person, not the hardware"
        );
    }

    /// A profile somebody chose outranks one the game detected.
    #[test]
    fn an_explicit_controller_profile_is_not_overruled_by_detection() {
        let mut settings = ControlSettings::default();
        settings.controller_profile = ControllerProfileId::Xbox360;
        settings.apply_profile_defaults();
        let chosen = settings.left_stick_deadzone;

        let filters = ControlFilters::for_pad(&settings, crate::GamepadStyle::PlayStation);
        assert_eq!(
            filters.left_stick_deadzone, chosen,
            "the pad reads as a DualSense, but somebody picked the 360 table and \
             a settings screen the game silently rewrites is not a settings screen"
        );
    }

    #[test]
    fn playstation_profile_tightens_deadzone() {
        let baseline = ControllerProfileId::Default.filter_defaults();
        let ps = ControllerProfileId::PlayStation.filter_defaults();
        // DualShock / DualSense sticks calibrate tighter than baseline.
        assert!(ps.left_stick_deadzone < baseline.left_stick_deadzone);
        assert!(ps.right_stick_deadzone < baseline.right_stick_deadzone);
    }

    #[test]
    fn apply_profile_defaults_writes_filter_values() {
        let mut s = ControlSettings::default();
        s.controller_profile = ControllerProfileId::Xbox360;
        // Overwrite with other values so the apply is observable.
        s.left_stick_deadzone = 0.99;
        s.trigger_press_threshold = 0.10;
        s.apply_profile_defaults();
        let xbox360 = ControllerProfileId::Xbox360.filter_defaults();
        assert_eq!(s.left_stick_deadzone, xbox360.left_stick_deadzone);
        assert_eq!(s.trigger_press_threshold, xbox360.trigger_press_threshold);
        // After clamp_all the values must remain valid.
        s.clamp_all();
        assert!(s.trigger_press_threshold > s.trigger_release_threshold);
    }

    /// A player's remap survives an action rename. The compiler cannot catch this.
    #[test]
    fn a_stored_remap_survives_the_shield_action_rename() {
        use bevy::prelude::KeyCode;

        let mut settings = ControlSettings::default();
        settings.binding_overrides = vec![
            BindingOverride::key("QuickAction", KeyCode::KeyQ),
            BindingOverride::key("Jump", KeyCode::KeyJ),
        ];
        settings.clamp_all();

        assert_eq!(
            settings.binding_overrides,
            vec![
                BindingOverride::key("Shield", KeyCode::KeyQ),
                BindingOverride::key("Jump", KeyCode::KeyJ),
            ],
            "the stored shield remap did not carry across the rename (or an \
             untouched action was disturbed on the way past)"
        );
    }

    /// A file can hold both spellings for one action and device class.
    /// Migration keeps the last one, as a fresh remap does.
    #[test]
    fn both_spellings_of_one_action_collapse_to_the_latest() {
        use bevy::prelude::KeyCode;

        let mut settings = ControlSettings::default();
        settings.binding_overrides = vec![
            BindingOverride::key("QuickAction", KeyCode::KeyQ),
            BindingOverride::key("Shield", KeyCode::KeyR),
        ];
        settings.clamp_all();

        assert_eq!(
            settings.binding_overrides,
            vec![BindingOverride::key("Shield", KeyCode::KeyR)],
            "two rows for one action and device class survived migration, so the \
             override layer would apply them in file order"
        );
    }
}

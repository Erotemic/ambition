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
        // The stated reason for it was accidental activation when a press turns
        // into a small drag. That is a real hazard and it is not what a
        // whole-menu confirmation policy is for: `SingleTapWithDestructiveGuard`
        // already keeps the two-step for exactly the presses worth guarding (a
        // stray touch on Quit), and drag-cancellation belongs to the gesture
        // layer, where a press that moves past the drag threshold is a scroll.
        //
        // So every platform now shares one default, which also removes a
        // behaviour that could only be discovered by owning the device.
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
    /// `Row` is an opaque identity, never an ordinate: nothing here compares or
    /// orders rows, it only asks whether two presses landed on the SAME one. An
    /// index-addressed menu passes `usize`; the pointer bridge, which knows a row
    /// by its action rather than its position, passes that instead. One policy,
    /// both call shapes — the alternative was a second implementation of the
    /// destructive guard for whichever caller did not fit.
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

/// Whether the BURST press — the shared dodge/dash button — should fire from
/// the right trigger only, the right shoulder button only, or both.
///
/// The field that DOES carry it is [`ControlSettings::burst_input_mode`], and that one is
/// pinned.
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

/// What the RIGHT STICK does during gameplay.
///
/// ⭐⭐ THE GENRE'S C-STICK, and the reason it is a MODE rather than a second
/// reader of the same stick: the right stick already aims the blink, and a
/// deflection cannot mean "aim there" and "attack that way" at once. A player
/// picks which one their right stick is.
///
/// ⛔ THE TWO ATTACK MODES ARE NOT COSMETIC VARIANTS OF EACH OTHER. A tilt stick
/// exists so a full deflection throws a TILT — the flick that a movement stick
/// would read as a smash — and a smash stick exists so a gentle push still
/// throws a SMASH. Each forces the strength the other cannot reach, which is why
/// [`ambition_platformer2d_core::AttackStrengthHint`] had to stop being a
/// one-way bool before either could exist.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RightStickMode {
    /// Aim the blink. Default, and what the stick has always done.
    #[default]
    Aim,
    /// Flicking the stick throws a TILT in that direction, at any deflection.
    TiltAttack,
    /// Flicking the stick throws a SMASH in that direction, at any deflection.
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

/// The values the DEVICE READER filters with.
///
/// A deadzone is a fact about the stick in somebody's hands, not about the person holding it.
///
/// `Copy` on purpose: this is rebuilt per seat per frame, and cloning
/// `ControlSettings` would allocate its binding-override `Vec` every time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ControlFilters {
    pub left_stick_deadzone: f32,
    pub right_stick_deadzone: f32,
    pub trigger_release_threshold: f32,
    pub trigger_press_threshold: f32,
    /// a PREFERENCE, not a calibration — which trigger or button means BURST
    /// is a choice about the person, so it stays machine-wide even per pad.
    pub burst_input_mode: BurstInputMode,
    /// What the right stick is for — see [`RightStickMode`].
    pub right_stick_mode: RightStickMode,
    /// Also a preference. Inverted aim is a habit, not a hardware property.
    pub invert_aim_y: bool,
}

impl ControlFilters {
    /// The machine-wide values, exactly as the settings screen tuned them. What
    /// the PRIMARY seat uses — those sliders are theirs.
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
    /// PREFERENCES.
    ///
    /// an explicit profile choice still wins. If somebody picked a
    /// controller profile in the settings, that is a decision and detection does
    /// not get to overrule it; only `Default` — "nobody said" — defers to the
    /// pad. That keeps the settings screen meaningful instead of making it a
    /// value the game silently rewrites.
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

/// Which calibration table a DETECTED pad style gets.
///
/// `Xbox360` is deliberately unreachable from detection. Its table is the
/// drifty-stick / worn-trigger one, and `gamepad_style_of` reads Microsoft's
/// vendor id — which a 360 pad and a Series controller share. Guessing "old and
/// worn" from a vendor id would widen the deadzone on a brand-new pad. That
/// table stays available as an explicit settings choice, which is the only place
/// the information exists.
fn profile_for_pad(style: crate::GamepadStyle) -> ControllerProfileId {
    match style {
        crate::GamepadStyle::PlayStation => ControllerProfileId::PlayStation,
        crate::GamepadStyle::XboxLike => ControllerProfileId::XboxOne,
        crate::GamepadStyle::Switch | crate::GamepadStyle::Generic => ControllerProfileId::Generic,
    }
}

/// A control an override can NAME.
///
/// deliberately not `PhysicalControl`, which the binding projection uses,
/// and the difference is the direction of travel. That type reads OUT of a live
/// `InputMap` and so must be total — it carries an `Other(String)` arm rather
/// than dropping a binding it cannot classify. This one is authored INTO a map
/// and so must be constructible: there is no honest `Other` here, because a
/// settings file cannot ask for a control the binding layer cannot bind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OverrideControl {
    Key(KeyCode),
    Button(GamepadButton),
}

impl OverrideControl {
    /// Which half of a preset this override REPLACES. A preset binds most
    /// actions on both a key and a pad button, and remapping Jump to `J` must
    /// not silently unbind the controller.
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

/// One persisted "this action is on THIS control instead".
///
/// The action is named by the string [`crate::ActionBindings`] already
/// publishes (the `Debug` spelling of the action), because a settings file, a
/// trace line and a rebind row have to agree on one id and that is the one
/// already in use. It is a `String` rather than the action type itself for a
/// mechanical reason: this module compiles WITHOUT the `input` feature, where
/// the leafwing action enum does not exist — while the settings file must
/// deserialize identically in both builds.
///
/// An override naming an action this build does not have is IGNORED, not an
/// error: settings outlive the binary that wrote them, and a file from a build
/// with one more action must still load.
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

/// not `Copy`. It holds the binding overrides, which are a `Vec`. The
/// handful of sites that took a copy take a `.clone()`; every other reader
/// already went through a reference.
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
    /// Which control fires the shared dodge/dash BURST press.
    ///
    /// the SERDE NAME IS THE WIRE, and it stays `dash_input_mode`.
    /// This field has no `#[serde(default)]` and `ControlSettings` has no
    /// container default, so a missing key is not "fall back to the default for
    /// this one knob" — it is a deserialize error for the whole struct, and
    /// `load_settings` answers a parse error by discarding the ENTIRE settings
    /// file (video, audio, gameplay, presets and every binding override with
    /// it) and returning `UserSettings::default()`. Renaming the key without
    /// pinning it would have silently wiped every existing player's settings on
    /// the first launch after the rename.
    #[serde(rename = "dash_input_mode")]
    pub burst_input_mode: BurstInputMode,
    /// What the right stick is for — see [`RightStickMode`].
    ///
    /// ⛔⛔ `#[serde(default)]`, AND THE FIELD ABOVE EXPLAINS WHY IT IS NOT
    /// OPTIONAL. `ControlSettings` has no container default, so a key missing
    /// from a saved file is a deserialize error for the WHOLE struct — and
    /// `load_settings` answers a parse error by discarding the entire settings
    /// file, bindings and all. Every settings file written before 2026-08-31
    /// lacks this key, so without this attribute adding a right-stick mode would
    /// have wiped every existing player's settings on their next launch.
    #[serde(default)]
    pub right_stick_mode: RightStickMode,
    /// Initial repeat delay for held menu directions, in seconds.
    pub menu_repeat_initial_delay: f32,
    /// Repeat interval after the initial delay.
    pub menu_repeat_interval: f32,
    /// Whether the on-screen touch overlay (joystick + action buttons)
    /// is VISIBLE. Mirrors into the `TouchControlsVisible` resource from
    /// the `TouchControlsPlugin`. This controls only the overlay's
    /// visibility, NOT whether touch input is enabled — touch enablement
    /// is owned by the plugin (installed = enabled). Default true so the
    /// overlay shows whenever the plugin is installed; toggle off via the
    /// controls settings page to hide it while testing keyboard-only on
    /// desktop (touch input stays live, just invisible).
    #[serde(default = "default_touch_controls_visible")]
    pub touch_controls_visible: bool,
    /// How a tap or mouse click on a menu item should behave. See
    /// [`MenuTapMode`] for semantics.
    #[serde(default)]
    pub menu_tap_mode: MenuTapMode,
    /// Per-action binding overrides layered ON TOP of
    /// [`Self::keyboard_preset_index`]'s preset.
    ///
    /// A preset and an override are not rivals: the preset says what every
    /// action starts on, the override says which single action moved. Storing
    /// the whole rebuilt map instead would freeze a player's controls at the
    /// preset they were authored against, so a later preset revision could
    /// never reach anybody who had ever touched a binding.
    ///
    /// `serde(default)` because every settings file written before this field
    /// existed must keep loading — an empty list is exactly "no overrides".
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
    /// Apply the active controller profile's filter defaults
    /// (deadzones + trigger thresholds) over whatever is currently
    /// stored. Useful when the user changes the profile dropdown
    /// and wants the per-pad calibration to take effect immediately.
    pub fn apply_profile_defaults(&mut self) {
        let p = self.controller_profile.filter_defaults();
        self.left_stick_deadzone = p.left_stick_deadzone;
        self.right_stick_deadzone = p.right_stick_deadzone;
        self.trigger_release_threshold = p.trigger_release_threshold;
        self.trigger_press_threshold = p.trigger_press_threshold;
    }

    /// Set a per-action binding override, replacing any previous override for
    /// the SAME action and device class.
    ///
    /// One authority per (action, class): a list that accumulated two keyboard
    /// overrides for Jump would make "which one wins" a question about
    /// insertion order, and the answer would be invisible in the settings file.
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

    /// Restore the deadzone / trigger / repeat values to their defaults
    /// without disturbing controller/keyboard profile selection.
    ///
    /// filtering only — it is what the `ResetControlFiltering` row calls,
    /// and it leaves both the preset and the binding overrides alone. Forgetting
    /// a remap is [`Self::reset_binding_overrides`]; a row that did both would
    /// wipe a player's controls when they only wanted their deadzone back.
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

    /// Carry a stored remap across an action RENAME.
    ///
    /// a rename silently deletes a player's remap, and nothing reports it.
    /// [`BindingOverride::action`] is the action's `Debug` spelling, and
    /// `apply_override` deliberately ignores a name this build does not have —
    /// that tolerance is what lets a settings file from a newer build load at all
    /// (see `bindings::apply_override`). The same tolerance means a renamed
    /// action's override just stops applying: the player's shield goes back to
    /// the preset key and no log line says why.
    ///
    /// So a rename owes this table an entry. Run from [`Self::clamp_all`], which
    /// every load path already calls right after reading the file, so the stored
    /// name is HEALED rather than merely tolerated — a second rename later cannot
    /// then need a two-step chain.
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
        // A file that already carried BOTH names for one action (written either
        // side of the rename) would now hold two rows the override layer applies
        // in order; keep the last one per (action, device class), which is the
        // same precedence `set_binding_override` enforces on a fresh remap.
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

/// State machine for a single analog input that should produce edge
/// events ("just pressed") with hysteresis. Independent of Bevy types
/// so it can be shared between keyboard scaffolding, gamepad triggers,
/// and tests.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TriggerEdgeState {
    #[default]
    Released,
    Pressed,
}

/// The per-device analog edges a gameplay frame has to remember between ticks.
///
/// ⭐ ONE CARRIER, because the caller stores exactly one value back into its
/// resource. The burst trigger was the only member until the right stick became
/// an attack stick, and a C-STICK FLICK IS THE SAME MECHANISM — an analog value
/// crossing a press threshold from rest, with hysteresis so a stick held out
/// does not re-fire every frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GameplayEdgeState {
    /// The burst trigger's hysteretic press.
    pub burst: TriggerEdgeState,
    /// The aim stick's DEFLECTION, when the stick is an attack stick. Idle in
    /// [`RightStickMode::Aim`], which is what the stick does by default.
    pub aim_stick: TriggerEdgeState,
}

/// How far the aim stick must be pushed to throw a C-stick attack.
///
/// ⭐ ABOVE THE DEADZONE AND BELOW A FULL DEFLECTION. The point of an attack
/// stick is that ANY deflection past this throws the authored strength — a tilt
/// stick's full push is still a tilt — so this is a "did you mean it" gate, not
/// a strength threshold. The release side is the stick's own deadzone, so the
/// gesture re-arms exactly when the stick reads as centred.
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
    ///
    /// So player two's pad ran on player one's calibration.
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

    /// a profile somebody CHOSE outranks one the game detected.
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
        // Stomp existing values with random nonsense so the apply
        // is observably an overwrite, not a no-op.
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

    /// A PLAYER'S REMAP SURVIVES THE ACTION BEING RENAMED.
    ///
    /// this is the one failure mode a rename has that a compiler cannot see.
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

    /// A file written either side of the rename can hold BOTH spellings for one
    /// action and device class. Migration collapses them the way a fresh remap
    /// would: last write wins.
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

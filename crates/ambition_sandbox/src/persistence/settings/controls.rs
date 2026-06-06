//! Controls / input settings.
//!
//! Holds controller deadzones, trigger thresholds, hysteresis, dash
//! input behavior, and which keyboard / controller profile is active.
//! The values flow into `crate::input` filtering before the `ControlFrame`
//! is built so gameplay sees clean edges instead of analog jitter.

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
        // Touch-only Android testing is more tolerant when a row tap selects
        // first and requires an explicit second tap / Confirm. Desktop and
        // Steam Deck keep the faster single-tap behavior, while Android avoids
        // accidental activation when a finger press turns into a small drag.
        #[cfg(target_os = "android")]
        {
            Self::TapToSelectThenConfirm
        }
        #[cfg(not(target_os = "android"))]
        {
            Self::SingleTapWithDestructiveGuard
        }
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
    /// Decide what a pointer press on `target_index` should do, given
    /// the current selection and whether the target is destructive.
    ///
    /// `armed` tracks "this destructive row was selected by a prior
    /// press and is awaiting a confirm tap". The function may clear or
    /// set it.
    pub fn resolve_press(
        self,
        target_index: usize,
        currently_selected: usize,
        is_destructive: bool,
        armed: &mut Option<usize>,
    ) -> MenuPointerPress {
        match self {
            Self::SingleTap => {
                *armed = None;
                MenuPointerPress::Confirm
            }
            Self::TapToSelectThenConfirm => {
                if currently_selected == target_index && *armed == Some(target_index) {
                    *armed = None;
                    MenuPointerPress::Confirm
                } else {
                    *armed = Some(target_index);
                    MenuPointerPress::SelectOnly
                }
            }
            Self::SingleTapWithDestructiveGuard => {
                let confirm_now = !is_destructive
                    || (currently_selected == target_index && *armed == Some(target_index));
                if confirm_now {
                    *armed = None;
                    MenuPointerPress::Confirm
                } else {
                    *armed = Some(target_index);
                    MenuPointerPress::SelectOnly
                }
            }
        }
    }
}

/// Whether dash should fire from the right trigger only, the right
/// shoulder button only, or both.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DashInputMode {
    /// Right trigger 2 (RT/R2). Default; matches prior behavior.
    #[default]
    Trigger,
    /// Right shoulder button (RB/R1).
    Button,
    /// Either input fires dash.
    Both,
}

impl DashInputMode {
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
            _ => ProfileFilterDefaults {
                left_stick_deadzone: 0.18,
                right_stick_deadzone: 0.20,
                trigger_release_threshold: 0.30,
                trigger_press_threshold: 0.55,
            },
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

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ControlSettings {
    /// Active keyboard preset index (matches `KeyboardPreset::presets()`).
    pub keyboard_preset_index: usize,
    pub controller_profile: ControllerProfileId,
    /// Radial deadzone for the left analog stick. Magnitudes below this
    /// are treated as zero before being fed to gameplay or the menu.
    pub left_stick_deadzone: f32,
    /// Radial deadzone for the right analog stick / aim.
    pub right_stick_deadzone: f32,
    /// Lower hysteresis bound for the right trigger (Dash). The trigger
    /// must drop below this to "release"; pulling back above
    /// `trigger_press_threshold` re-arms a press edge.
    pub trigger_release_threshold: f32,
    /// Upper hysteresis bound for the right trigger (Dash).
    pub trigger_press_threshold: f32,
    /// Whether the D-pad navigates menus (in addition to the analog
    /// stick / arrow keys).
    pub dpad_menu_navigation: bool,
    /// Invert vertical aim (right stick / aim binding).
    pub invert_aim_y: bool,
    pub dash_input_mode: DashInputMode,
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
}

fn default_touch_controls_visible() -> bool {
    true
}

impl Default for ControlSettings {
    fn default() -> Self {
        Self {
            keyboard_preset_index: 0,
            controller_profile: ControllerProfileId::default(),
            left_stick_deadzone: 0.18,
            right_stick_deadzone: 0.20,
            trigger_release_threshold: 0.30,
            trigger_press_threshold: 0.55,
            dpad_menu_navigation: true,
            invert_aim_y: false,
            dash_input_mode: DashInputMode::default(),
            menu_repeat_initial_delay: 0.32,
            menu_repeat_interval: 0.12,
            touch_controls_visible: default_touch_controls_visible(),
            menu_tap_mode: MenuTapMode::default(),
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

    /// Restore the deadzone / trigger / repeat values to their defaults
    /// without disturbing controller/keyboard profile selection. The
    /// "Reset bindings" menu row calls this.
    pub fn reset_filtering_to_defaults(&mut self) {
        let defaults = Self::default();
        self.left_stick_deadzone = defaults.left_stick_deadzone;
        self.right_stick_deadzone = defaults.right_stick_deadzone;
        self.trigger_release_threshold = defaults.trigger_release_threshold;
        self.trigger_press_threshold = defaults.trigger_press_threshold;
        self.dpad_menu_navigation = defaults.dpad_menu_navigation;
        self.invert_aim_y = defaults.invert_aim_y;
        self.dash_input_mode = defaults.dash_input_mode;
        self.menu_repeat_initial_delay = defaults.menu_repeat_initial_delay;
        self.menu_repeat_interval = defaults.menu_repeat_interval;
    }

    pub fn clamp_all(&mut self) {
        self.left_stick_deadzone = self.left_stick_deadzone.clamp(0.0, 0.95);
        self.right_stick_deadzone = self.right_stick_deadzone.clamp(0.0, 0.95);
        self.trigger_release_threshold = self.trigger_release_threshold.clamp(0.0, 0.95);
        // Press threshold must be greater than release for usable hysteresis.
        let press_lower = (self.trigger_release_threshold + 0.05).min(0.95);
        self.trigger_press_threshold = self.trigger_press_threshold.clamp(press_lower, 1.0);
        self.menu_repeat_initial_delay = self.menu_repeat_initial_delay.clamp(0.05, 1.5);
        self.menu_repeat_interval = self.menu_repeat_interval.clamp(0.02, 1.0);
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

/// Update a hysteretic trigger edge.
///
/// Returns `(new_state, just_pressed)`. The "press" edge fires when the
/// previous state is `Released` and the current value rises above
/// `press`; the "release" edge fires when the value drops below
/// `release`. Values between the two thresholds preserve the previous
/// state — that's the hysteresis that prevents jitter from producing
/// repeated edges while a Dash trigger is held.
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
    fn dash_mode_cycles_through_all() {
        let mut visited = std::collections::HashSet::new();
        let mut cur = DashInputMode::default();
        for _ in 0..DashInputMode::ALL.len() {
            visited.insert(cur);
            cur = cur.next();
        }
        assert_eq!(visited.len(), DashInputMode::ALL.len());
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
}

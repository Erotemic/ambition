//! Battery / power policy primitives.
//!
//! Resources:
//!
//! - [`PowerProfile`] — user-selected aggressiveness tier
//!   (`Performance`, `Balanced`, `BatterySaver`). Affects whether
//!   nonessential per-frame work runs when the window is unfocused
//!   or when the platform is power-constrained (Android, low
//!   battery).
//! - [`WindowFocusState`] — current focus state of the primary
//!   window. Updated by the platform plugin from Bevy
//!   `WindowFocused` events.
//!
//! Today this module is a *scaffold*: the resources exist, the
//! focus-tracking system updates `WindowFocusState`, and a
//! `should_pause_nonessential_work` helper combines the two into a
//! single decision. Wiring nonessential systems to this gate (HUD
//! redraw, dust-particle update, ambient music ducking) is a
//! follow-up in the per-system plugins.

use bevy::prelude::*;
use bevy::window::WindowFocused;

/// User-selected power-aggressiveness tier. `Balanced` is the
/// default for desktop; `BatterySaver` ought to be the default for
/// Android once the gate is wired into the heavy systems.
///
/// The settings page exposes this as a controls/gameplay toggle —
/// see `dev/journals/lessons_learned.md` "Android size is a
/// separate profile and platform-composition problem" for the
/// reasoning behind keeping power policy distinct from feature
/// flags.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PowerProfile {
    /// Run every system every frame. Default on desktop where the
    /// user almost always has wall power.
    #[default]
    Performance,
    /// Skip work that's clearly nonessential when the window has
    /// lost focus (no input, audio ducks).
    Balanced,
    /// Aggressively pause anything that doesn't directly affect
    /// gameplay correctness. Targeted at Android.
    BatterySaver,
}

/// Focus state of the primary window. `Focused` is the default;
/// the platform plugin flips it on Bevy `WindowFocused` events.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum WindowFocusState {
    #[default]
    Focused,
    Unfocused,
}

impl WindowFocusState {
    pub fn is_focused(self) -> bool {
        matches!(self, Self::Focused)
    }
}

/// Update [`WindowFocusState`] from Bevy's `WindowFocused` event
/// stream. Registered by the [`super::PlatformPlugin`].
pub fn track_window_focus(
    mut events: MessageReader<WindowFocused>,
    mut state: ResMut<WindowFocusState>,
) {
    for event in events.read() {
        *state = if event.focused {
            WindowFocusState::Focused
        } else {
            WindowFocusState::Unfocused
        };
    }
}

/// Should nonessential per-frame work be skipped this frame?
///
/// `true` when the user-selected `PowerProfile` says we should pause
/// nonessential work AND the window is currently unfocused.
/// `Performance` mode never returns true. Run-condition shape so
/// callers can use it as a Bevy `.run_if(...)` predicate.
pub fn should_pause_nonessential_work(
    profile: Res<PowerProfile>,
    focus: Res<WindowFocusState>,
) -> bool {
    if focus.is_focused() {
        return false;
    }
    match *profile {
        PowerProfile::Performance => false,
        PowerProfile::Balanced | PowerProfile::BatterySaver => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn performance_never_pauses_even_when_unfocused() {
        // Performance profile keeps every system running regardless
        // of focus.
        let mut app = App::new();
        app.insert_resource(PowerProfile::Performance);
        app.insert_resource(WindowFocusState::Unfocused);
        app.add_systems(
            Update,
            |profile: Res<PowerProfile>, focus: Res<WindowFocusState>| {
                assert!(!should_pause_nonessential_work(profile, focus));
            },
        );
        app.update();
    }

    #[test]
    fn balanced_pauses_when_unfocused_only() {
        let mut app = App::new();
        app.insert_resource(PowerProfile::Balanced);
        app.insert_resource(WindowFocusState::Focused);
        app.add_systems(
            Update,
            |profile: Res<PowerProfile>, focus: Res<WindowFocusState>| {
                assert!(!should_pause_nonessential_work(profile, focus));
            },
        );
        app.update();

        let mut app = App::new();
        app.insert_resource(PowerProfile::Balanced);
        app.insert_resource(WindowFocusState::Unfocused);
        app.add_systems(
            Update,
            |profile: Res<PowerProfile>, focus: Res<WindowFocusState>| {
                assert!(should_pause_nonessential_work(profile, focus));
            },
        );
        app.update();
    }
}

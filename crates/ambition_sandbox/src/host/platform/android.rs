//! Android-specific platform setup.
//!
//! Owns the Android pause/resume handler: when the user backgrounds
//! the app (home button, screen off, app switcher, notification
//! shade), the game flips to `GameMode::Paused` and every audio
//! channel is paused so kira's audio thread stops mixing. On return
//! the audio channels resume; the game stays paused so the user must
//! tap Resume from the pause menu before play continues.
//!
//! We listen to three Bevy signals at once and treat any of them as
//! "backgrounded":
//!
//! - `AppLifecycle::WillSuspend` / `Suspended` from `bevy_winit` —
//!   the most authoritative signal but Android only fires it when
//!   the OS actually paused the activity, which can lag behind the
//!   user's "home button" press by a frame.
//! - `WindowFocused(focused: false)` — fires reliably when the user
//!   pulls down the notification shade or another app takes focus.
//! - `WindowOccluded(true)` — fires when the screen is fully covered
//!   (split-screen with another fullscreen app, lock screen).
//!
//! Listening to all three protects against platform versions /
//! launcher quirks that drop one of the events.

use bevy::prelude::*;
#[cfg(target_os = "android")]
use bevy::window::{AppLifecycle, WindowFocused, WindowOccluded};

use super::power::PowerProfile;
#[cfg(target_os = "android")]
use crate::game_mode::GameMode;

/// Pick a sensible default `PowerProfile` for the Android build.
///
/// Default is `BatterySaver`: phones run on battery, and the user
/// can flip to `Performance` if they want maximum FPS while plugged
/// in.
pub fn default_power_profile() -> PowerProfile {
    PowerProfile::BatterySaver
}

/// Bevy plugin for Android-only setup.
///
/// - Inserts the initial [`PowerProfile`] resource.
/// - Wires the suspend/resume handler that pauses the game + audio
///   when the OS backgrounds the app.
pub struct AndroidPlatformPlugin;

impl Plugin for AndroidPlatformPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(default_power_profile());
        #[cfg(target_os = "android")]
        {
            app.init_resource::<AndroidSuspendState>();
            app.add_systems(PreUpdate, detect_android_suspend_state);
            app.add_systems(Update, apply_android_suspend_to_game_mode);
            #[cfg(feature = "audio")]
            app.add_systems(Update, audio_lifecycle::apply_android_suspend_to_audio);
        }
    }
}

#[cfg(target_os = "android")]
#[derive(Resource, Default, Debug, Clone, Copy)]
struct AndroidSuspendState {
    /// `true` while the OS considers the app backgrounded for any
    /// of the three reasons (lifecycle / focus / occlusion).
    suspended: bool,
    /// Bumped each frame the suspended bit flips so downstream
    /// systems (audio, game mode) react on the edge instead of
    /// every frame.
    just_changed: bool,
}

/// Fold every "app is going to the background" signal into a single
/// `AndroidSuspendState.suspended` bit + edge flag. Runs in
/// `PreUpdate` so the gameplay/audio systems in `Update` see the
/// latest reading.
///
/// We treat the lifecycle / focus / occlusion events with **OR-pause,
/// AND-resume** semantics: any of the three claiming "backgrounded"
/// flips suspend on; coming back requires the lifecycle event to say
/// Running OR a focus regain. Without OR-pause we lost wake-ups on
/// devices that only emit `WindowOccluded` for the lock screen, and
/// without AND-resume the focus event sometimes flickered true for a
/// single frame mid-suspend on the Pixel test phone.
#[cfg(target_os = "android")]
fn detect_android_suspend_state(
    mut lifecycle: MessageReader<AppLifecycle>,
    mut focused: MessageReader<WindowFocused>,
    mut occluded: MessageReader<WindowOccluded>,
    mut state: ResMut<AndroidSuspendState>,
) {
    let was = state.suspended;
    let mut suspending = false;
    let mut resuming = false;

    for event in lifecycle.read() {
        match event {
            AppLifecycle::WillSuspend | AppLifecycle::Suspended => suspending = true,
            AppLifecycle::WillResume | AppLifecycle::Running => resuming = true,
            AppLifecycle::Idle => {}
        }
    }
    for event in focused.read() {
        if event.focused {
            resuming = true;
        } else {
            suspending = true;
        }
    }
    for event in occluded.read() {
        if event.occluded {
            suspending = true;
        } else {
            resuming = true;
        }
    }

    // OR-pause wins: if any signal said "suspending" this frame, we
    // suspend regardless of a contradicting resume from another
    // signal. Same-frame both means the user backgrounded and
    // refocused inside one tick, which we treat as "stay paused" so
    // the next confirmed resume reads as an edge.
    let next = if suspending {
        true
    } else if resuming {
        false
    } else {
        was
    };

    state.just_changed = next != was;
    state.suspended = next;
    if state.just_changed {
        bevy::log::info!(
            target: "ambition::android_lifecycle",
            "android suspend state changed: {} -> {}",
            was,
            next
        );
    }
}

/// On the suspend edge, force `GameMode::Paused`. On the resume edge,
/// leave the mode alone — Android convention is that the user
/// explicitly resumes from the pause menu when they return.
#[cfg(target_os = "android")]
fn apply_android_suspend_to_game_mode(
    state: Res<AndroidSuspendState>,
    mode: Res<State<GameMode>>,
    mut next_mode: ResMut<NextState<GameMode>>,
) {
    if !state.just_changed {
        return;
    }
    if state.suspended {
        // Only flip into Paused if gameplay was actually active.
        // Leaving Dialogue alone avoids stomping a mid-NPC
        // conversation when the user briefly checks notifications;
        // Paused / RoomTransition are already non-playing states.
        if matches!(mode.get(), GameMode::Playing | GameMode::Cutscene) {
            next_mode.set(GameMode::Paused);
        }
    }
}

#[cfg(all(target_os = "android", feature = "audio"))]
mod audio_lifecycle {
    use bevy::prelude::*;
    use bevy_kira_audio::prelude::{AudioChannel, AudioControl, AudioTween};
    use std::time::Duration;

    use super::AndroidSuspendState;

    /// Pause every audio channel on the suspend edge and resume on the
    /// resume edge. Uses a 40 ms tween so the cut isn't audibly clicky
    /// while still being fast enough that the user perceives the
    /// silence as immediate.
    pub(super) fn apply_android_suspend_to_audio(
        state: Res<AndroidSuspendState>,
        music: Option<Res<AudioChannel<crate::audio::MusicChannel>>>,
        sfx: Option<Res<AudioChannel<crate::audio::SfxChannel>>>,
    ) {
        if !state.just_changed {
            return;
        }
        let tween = AudioTween::linear(Duration::from_millis(40));
        if state.suspended {
            if let Some(ch) = music.as_deref() {
                ch.pause().fade_out(tween);
            }
            if let Some(ch) = sfx.as_deref() {
                ch.pause()
                    .fade_out(AudioTween::linear(Duration::from_millis(40)));
            }
            bevy::log::info!(
                target: "ambition::android_lifecycle",
                "android suspend: paused music + sfx channels"
            );
        } else {
            if let Some(ch) = music.as_deref() {
                ch.resume().fade_in(tween);
            }
            if let Some(ch) = sfx.as_deref() {
                ch.resume()
                    .fade_in(AudioTween::linear(Duration::from_millis(40)));
            }
            bevy::log::info!(
                target: "ambition::android_lifecycle",
                "android resume: resumed music + sfx channels"
            );
        }
    }
}

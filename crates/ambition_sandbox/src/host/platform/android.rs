//! Android-specific platform setup.
//!
//! Owns the Android `AppLifecycle` handler that pauses the game and
//! mutes audio when the user backgrounds the app (home button, screen
//! off, app switcher), and restores audio on resume. Without this
//! handler, `bevy_kira_audio` keeps the music thread running and the
//! simulation can advance one extra frame while suspended.
//!
//! Future Android-only systems (audio focus listener, doze handling,
//! internal-storage path resolution) live here too. The aim is to keep
//! `target_os = "android"` cfg guards inside this file rather than
//! scattered across gameplay code.

use bevy::prelude::*;
use bevy::window::AppLifecycle;

use super::power::PowerProfile;
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
/// - Listens for `AppLifecycle` events and pauses gameplay + audio
///   when the OS suspends the app, then resumes audio on return.
pub struct AndroidPlatformPlugin;

impl Plugin for AndroidPlatformPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(default_power_profile());
        app.add_systems(Update, handle_app_lifecycle);
        #[cfg(feature = "audio")]
        app.add_systems(Update, audio_lifecycle::pause_resume_audio_on_lifecycle);
    }
}

/// React to `AppLifecycle` events from `bevy_winit`.
///
/// On `WillSuspend` / `Suspended` (home button, screen off, app
/// switcher): force `GameMode::Paused` so the player isn't mid-jump
/// when the user returns. On `WillResume` / `Running`: leave
/// `GameMode` at `Paused` so the user explicitly resumes from the
/// pause menu (matches the convention of every other mobile game).
///
/// Audio pause/resume is handled by the companion system in
/// [`audio_lifecycle`] so the `bevy_kira_audio` dependency stays
/// behind the `audio` feature.
fn handle_app_lifecycle(
    mut events: MessageReader<AppLifecycle>,
    mode: Res<State<GameMode>>,
    mut next_mode: ResMut<NextState<GameMode>>,
) {
    for event in events.read() {
        if matches!(event, AppLifecycle::WillSuspend | AppLifecycle::Suspended) {
            // Only flip into Paused if gameplay was actually active.
            // Leaving Dialogue alone avoids stomping a mid-NPC
            // conversation when the user briefly checks notifications;
            // Paused / RoomTransition are already non-playing states.
            if matches!(mode.get(), GameMode::Playing | GameMode::Cutscene) {
                next_mode.set(GameMode::Paused);
            }
        }
    }
}

#[cfg(feature = "audio")]
mod audio_lifecycle {
    use bevy::prelude::*;
    use bevy::window::AppLifecycle;
    use bevy_kira_audio::prelude::{AudioChannel, AudioControl};

    /// Pause every audio channel on suspend so kira's audio thread
    /// stops mixing while the app is in the background, and resume on
    /// return so music picks up where it left off.
    pub(super) fn pause_resume_audio_on_lifecycle(
        mut events: MessageReader<AppLifecycle>,
        music: Option<Res<AudioChannel<crate::audio::MusicChannel>>>,
        sfx: Option<Res<AudioChannel<crate::audio::SfxChannel>>>,
    ) {
        for event in events.read() {
            match event {
                AppLifecycle::WillSuspend | AppLifecycle::Suspended => {
                    if let Some(ch) = music.as_deref() {
                        ch.pause();
                    }
                    if let Some(ch) = sfx.as_deref() {
                        ch.pause();
                    }
                }
                AppLifecycle::WillResume | AppLifecycle::Running => {
                    if let Some(ch) = music.as_deref() {
                        ch.resume();
                    }
                    if let Some(ch) = sfx.as_deref() {
                        ch.resume();
                    }
                }
                AppLifecycle::Idle => {}
            }
        }
    }
}

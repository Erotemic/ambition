//! Android-specific platform setup.
//!
//! Owns the Android pause/resume handler: when the user backgrounds
//! the app (home button, screen off, app switcher, notification
//! shade), the live game's pause menu opens ([`PauseRequested`]) and
//! every audio channel is paused so kira's audio thread stops mixing.
//! On return the audio resumes and the pause menu is still open: the
//! player resumes. While the window is unfocused the app updates at low
//! power instead of every display frame.
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

#![cfg_attr(not(target_os = "android"), allow(dead_code))]

use bevy::prelude::*;
#[cfg(target_os = "android")]
use bevy::window::{AppLifecycle, WindowFocused, WindowOccluded};

#[cfg(target_os = "android")]
use ambition_platformer2d::game_shell::PauseRequested;

// ⭐ THE ANDROID FONT SEEDING IS THE ENGINE'S JOB NOW, and it is a better job.
//
// Under Bevy 0.18 this module ran a startup system that reached into
// `bevy::text::CosmicFontSystem` and called `load_fonts_dir("/system/fonts")`,
// because cosmic-text's `fontdb` explicitly no-ops `load_system_fonts()` on
// Android and would otherwise panic with "no default font found".
//
// Bevy 0.19 replaced cosmic-text with Parley/fontique, and `CosmicFontSystem`
// does not exist to reach into. Fontique 0.9 ships a real Android system source
// (`fontique::backend::android`) that scans `$ANDROID_ROOT/fonts` — the same
// directory — AND parses the platform's `fonts.xml` to map generic families
// (sans-serif -> Roboto Flex / Roboto / Noto Sans, emoji -> Noto Color Emoji,
// and so on), which the hand-rolled directory load never did. It is enabled by
// bevy's `system_font_discovery` feature, which this app turns on for the
// `android_platform` composition ONLY.
//
// ⛔ NOT ON DESKTOP. Shipped product typography is the bundled Inter/JetBrains
// faces resolved through the asset catalog, and it must not vary with whatever
// a player has installed; on Linux the same feature would also drag in
// fontconfig. System discovery is here for FALLBACK GLYPH COVERAGE on a
// platform that ships no bundled fallback, which is what the old workaround was
// for too.

/// Bevy plugin for Android-only setup.
///
/// Wires the suspend/resume handler that pauses the game + audio
/// when the OS backgrounds the app.
pub struct AndroidPlatformPlugin;

impl Plugin for AndroidPlatformPlugin {
    fn build(&self, _app: &mut App) {
        #[cfg(target_os = "android")]
        {
            // An unfocused window (the screen off, the shade down, another
            // app over it) updates on its events and every few seconds, not
            // at the display rate: nothing is shown or played then.
            _app.insert_resource(bevy::winit::WinitSettings {
                focused_mode: bevy::winit::UpdateMode::Continuous,
                unfocused_mode: bevy::winit::UpdateMode::reactive_low_power(
                    std::time::Duration::from_secs(5),
                ),
            });
            _app.init_resource::<AndroidSuspendState>();
            _app.add_message::<PauseRequested>();
            _app.add_systems(PreUpdate, detect_android_suspend_state);
            _app.add_systems(Update, request_the_pause_menu_on_suspend);
            #[cfg(feature = "audio")]
            _app.add_systems(Update, audio_lifecycle::apply_android_suspend_to_audio);
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
    /// systems (audio, the pause request) react on the edge instead of
    /// every frame.
    just_changed: bool,
}

/// Fold every "app is going to the background" signal into a single
/// `AndroidSuspendState.suspended` bit + edge flag. Runs in
/// `PreUpdate` so the gameplay/audio systems in `Update` see the
/// latest reading.
///
/// We treat the lifecycle / focus / occlusion events with OR-pause,
/// AND-resume semantics: any of the three claiming "backgrounded"
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
            target: "ambition_platformer2d::android_lifecycle",
            "android suspend state changed: {} -> {}",
            was,
            next
        );
        // The same edge on the marker channel, carrying the FRAME.
        ambition_platformer2d::platformer::world_log::world_event(format_args!(
            "android-suspend-edge suspended {was} -> {next}"
        ));
    }
}

/// On the suspend edge, ask the live game's pause menu to open
/// ([`PauseRequested`]). The menu stays open on return, so the player resumes
/// by choice; nothing is restored automatically.
///
/// A mode with no pause menu (a cutscene, a room transition) is not forced
/// away from: while the app is unfocused it updates at low power (see
/// [`AndroidPlatformPlugin`]), and a backgrounded app does not update at all.
#[cfg(target_os = "android")]
fn request_the_pause_menu_on_suspend(
    state: Res<AndroidSuspendState>,
    mut requests: MessageWriter<PauseRequested>,
) {
    if state.just_changed && state.suspended {
        ambition_platformer2d::platformer::world_log::world_event(format_args!(
            "android-suspend: pause requested"
        ));
        requests.write(PauseRequested);
    }
}

#[cfg(all(target_os = "android", feature = "audio"))]
mod audio_lifecycle {
    use bevy::prelude::*;
    use bevy_kira_audio::prelude::{
        AudioChannel, AudioControl, AudioTween, DynamicAudioChannels, MainTrack,
    };
    use std::time::Duration;

    use super::AndroidSuspendState;

    /// Pause every audio channel on the suspend edge and resume each on the
    /// resume edge: the main track, the typed music and sound channels, and
    /// each dynamic channel (the music banks play there). A 40 ms tween keeps
    /// the cut from clicking.
    pub(super) fn apply_android_suspend_to_audio(
        state: Res<AndroidSuspendState>,
        main: Option<Res<AudioChannel<MainTrack>>>,
        music: Option<Res<AudioChannel<ambition_platformer2d::audio::library::MusicChannel>>>,
        sfx: Option<Res<AudioChannel<ambition_platformer2d::audio::library::SfxChannel>>>,
        dynamic: Option<Res<DynamicAudioChannels>>,
    ) {
        if !state.just_changed {
            return;
        }
        let suspended = state.suspended;
        if let Some(channel) = main.as_deref() {
            pause_or_resume(channel, suspended);
        }
        if let Some(channel) = music.as_deref() {
            pause_or_resume(channel, suspended);
        }
        if let Some(channel) = sfx.as_deref() {
            pause_or_resume(channel, suspended);
        }
        for (_, channel) in dynamic.iter().flat_map(|channels| channels.iter()) {
            pause_or_resume(channel, suspended);
        }
        bevy::log::info!(
            target: "ambition_platformer2d::android_lifecycle",
            "android {}: every audio channel",
            if suspended { "suspend: paused" } else { "resume: resumed" }
        );
    }

    fn pause_or_resume(channel: &impl AudioControl, suspended: bool) {
        let tween = AudioTween::linear(Duration::from_millis(40));
        if suspended {
            channel.pause().fade_out(tween);
        } else {
            channel.resume().fade_in(tween);
        }
    }
}

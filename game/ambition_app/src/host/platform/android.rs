//! Android-specific platform setup.
//!
//! Owns the Android pause/resume handler: when the user backgrounds
//! the app (home button, screen off, app switcher, notification
//! shade), the game flips to `GameMode::Paused` and every audio
//! channel is paused so kira's audio thread stops mixing. On return
//! the audio channels resume and the game mode is restored to whatever
//! it was before the suspend (unless the user navigated off the
//! forced-pause while backgrounded), so play continues automatically.
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

// Nothing about `GameMode` is platform-specific, and the suspend DECISION below is written in terms
// of it, so gating this `use` would gate the only part of this module a desktop test can reach.
use ambition_platformer2d::platformer::schedule::GameMode;

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
            _app.init_resource::<AndroidSuspendState>();
            _app.add_systems(PreUpdate, detect_android_suspend_state);
            _app.add_systems(Update, apply_android_suspend_to_game_mode);
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
    /// systems (audio, game mode) react on the edge instead of
    /// every frame.
    just_changed: bool,
    /// The `GameMode` we forced away from on the suspend edge, so the
    /// resume edge can restore it. `Some` only while a suspend-induced
    /// pause is in effect; cleared on the branch that restores it, and
    /// deliberately KEPT when the restore is refused — see
    /// [`decide_suspend`].
    mode_before_suspend: Option<GameMode>,
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

/// The variants carry only what the caller cannot already see: the observed
/// mode is an input, so it is never repeated here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuspendDecision {
    /// Suspend edge with gameplay active. `previous` has been written into the
    /// saved slot; the caller forces `GameMode::Paused`.
    Captured { previous: GameMode },
    /// Suspend edge in a mode we deliberately do not force away from
    /// (Dialogue, Paused, RoomTransition). Nothing saved, nothing to restore.
    CaptureSkipped,
    /// Resume edge, guard accepted. The saved slot has been CLEARED and the
    /// caller restores `restore_to`.
    Restored { restore_to: GameMode },
    /// Resume edge, guard refused because the forced pause is not the observed
    /// mode. `saved` is still in the slot for a later resume edge.
    RestoreRefused { saved: GameMode },
    /// Resume edge with nothing saved — no suspend-induced pause is in effect.
    NothingSaved,
}

/// Pure suspend/resume state decision, kept outside the Android glue for tests.
///
/// Restore only from the suspend-induced `Paused` state and retain the saved
/// mode when the guard refuses. Because `NextState` is deferred, a refused
/// restore is retried only on a later suspend/resume edge.
/// TODO(android-resume): record the forced-pause edge explicitly so resume does
/// not depend on observing the deferred `Paused` state.
pub fn decide_suspend(
    suspended: bool,
    observed: GameMode,
    saved: &mut Option<GameMode>,
) -> SuspendDecision {
    if suspended {
        // Only flip into Paused if gameplay was actually active.
        // Leaving Dialogue alone avoids stomping a mid-NPC
        // conversation when the user briefly checks notifications;
        // Paused / RoomTransition are already non-playing states.
        if matches!(observed, GameMode::Playing | GameMode::Cutscene) {
            *saved = Some(observed);
            SuspendDecision::Captured { previous: observed }
        } else {
            SuspendDecision::CaptureSkipped
        }
    } else if let Some(prev) = *saved {
        // Resume edge: restore the pre-suspend mode, but only if the
        // suspend-induced Paused is still the current mode. If the user opened a
        // menu or otherwise moved off it while backgrounded, leave their
        // navigation alone — and KEEP the saved mode rather than taking it.
        if matches!(observed, GameMode::Paused) {
            *saved = None;
            SuspendDecision::Restored { restore_to: prev }
        } else {
            SuspendDecision::RestoreRefused { saved: prev }
        }
    } else {
        SuspendDecision::NothingSaved
    }
}

/// On the resume edge, restore that mode — but only if we're still sitting in the `Paused` we
/// forced, so we never stomp a menu / dialogue / transition the user navigated into while the
/// app was backgrounded. This mirrors the audio channels, which already auto-resume on the same
/// edge; without it the game stays frozen with no visible affordance to un-pause.
///
/// GLUE ONLY. Every branch condition lives in [`decide_suspend`], which is
/// un-gated and unit-tested; this reads the world, delivers the decision, and
/// writes the world log. What is still unverifiable here is whether Android
/// delivers the lifecycle events in the order the decision assumes — that needs
/// a device, and no `adb` exists on this machine.
#[cfg(target_os = "android")]
fn apply_android_suspend_to_game_mode(
    mut state: ResMut<AndroidSuspendState>,
    mode: Res<State<GameMode>>,
    mut next_mode: ResMut<NextState<GameMode>>,
) {
    if !state.just_changed {
        return;
    }
    // The world-log outcomes below are the device-report instrument: a freeze
    // report is ordered against these lines, so each branch still says which one
    // happened and with what values.
    let observed = *mode.get();
    let suspended = state.suspended;
    match decide_suspend(suspended, observed, &mut state.mode_before_suspend) {
        SuspendDecision::Captured { previous } => {
            ambition_platformer2d::platformer::world_log::world_event(format_args!(
                "android-suspend captured={} (forcing paused)",
                previous.label()
            ));
            ambition_platformer2d::platformer::world_log::note_game_mode_request(
                GameMode::Paused,
                "android_suspend",
            );
            next_mode.set(GameMode::Paused);
        }
        SuspendDecision::CaptureSkipped => {
            ambition_platformer2d::platformer::world_log::world_event(format_args!(
                "android-suspend captured=none (mode={} is not Playing/Cutscene, so the \
                 resume edge will have nothing to restore)",
                observed.label()
            ));
        }
        SuspendDecision::Restored { restore_to } => {
            ambition_platformer2d::platformer::world_log::world_event(format_args!(
                "android-resume saved={} observed={} -> RESTORED",
                restore_to.label(),
                observed.label()
            ));
            ambition_platformer2d::platformer::world_log::note_game_mode_request(
                restore_to,
                "android_resume",
            );
            next_mode.set(restore_to);
        }
        SuspendDecision::RestoreRefused { saved } => {
            // THE LINE THIS WHOLE PROBE EXISTS FOR — and it no longer
            // reports a loss. The guard refused, and the saved mode is KEPT for
            // the next resume edge instead of being discarded. A freeze report
            // still ordered against this line answers the same question; it
            // just no longer answers it with an unrecoverable state.
            ambition_platformer2d::platformer::world_log::world_event(format_args!(
                "android-resume saved={} observed={} -> DEFERRED (guard refused; saved mode \
                 KEPT for a later resume edge)",
                saved.label(),
                observed.label()
            ));
        }
        SuspendDecision::NothingSaved => {
            ambition_platformer2d::platformer::world_log::world_event(format_args!(
                "android-resume saved=none observed={}",
                observed.label()
            ));
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
        music: Option<Res<AudioChannel<ambition_platformer2d::audio::library::MusicChannel>>>,
        sfx: Option<Res<AudioChannel<ambition_platformer2d::audio::library::SfxChannel>>>,
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
                target: "ambition_platformer2d::android_lifecycle",
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
                target: "ambition_platformer2d::android_lifecycle",
                "android resume: resumed music + sfx channels"
            );
        }
    }
}

/// The Android suspend decision, executed on a machine with no Android.
///
/// this proves nothing about Android and must not be reported as if it did. No NDK, no `adb`;
/// every `#[cfg(target_os = "android")]` item in this file is still uncompiled here. Whether
/// Android delivers the lifecycle events in that order is a separate claim and needs hardware.
#[cfg(test)]
mod tests {
    use super::{decide_suspend, GameMode, SuspendDecision};

    #[test]
    fn a_refused_restore_keeps_the_saved_mode_for_the_next_edge() {
        let mut saved = None;

        assert_eq!(
            decide_suspend(true, GameMode::Playing, &mut saved),
            SuspendDecision::Captured {
                previous: GameMode::Playing
            }
        );
        assert_eq!(saved, Some(GameMode::Playing));

        // The resume edge runs while the deferred Paused transition has not
        // applied yet, so `observed` is still the PRE-pause mode.
        assert_eq!(
            decide_suspend(false, GameMode::Playing, &mut saved),
            SuspendDecision::RestoreRefused {
                saved: GameMode::Playing
            }
        );
        assert_eq!(
            saved,
            Some(GameMode::Playing),
            "a refused restore must KEEP the saved mode; taking it here is the bug"
        );

        // …and the deferred Paused lands. The next resume edge recovers.
        assert_eq!(
            decide_suspend(false, GameMode::Paused, &mut saved),
            SuspendDecision::Restored {
                restore_to: GameMode::Playing
            }
        );
        assert_eq!(saved, None, "a restore consumes the saved mode");
    }

    /// A suspend from a mode we do not force away from saves nothing, so the
    /// resume edge has nothing to restore and says so.
    #[test]
    fn a_suspend_from_a_non_playing_mode_captures_nothing() {
        for observed in [
            GameMode::Paused,
            GameMode::Dialogue,
            GameMode::RoomTransition,
        ] {
            let mut saved = None;
            assert_eq!(
                decide_suspend(true, observed, &mut saved),
                SuspendDecision::CaptureSkipped,
                "{observed:?} is not a mode the suspend edge forces away from"
            );
            assert_eq!(saved, None);
            assert_eq!(
                decide_suspend(false, GameMode::Paused, &mut saved),
                SuspendDecision::NothingSaved
            );
        }
    }

    /// Cutscene is captured alongside Playing — a scripted set piece resumes
    /// rather than dumping the player into a paused cutscene.
    #[test]
    fn a_cutscene_is_captured_and_restored_like_playing() {
        let mut saved = None;
        assert_eq!(
            decide_suspend(true, GameMode::Cutscene, &mut saved),
            SuspendDecision::Captured {
                previous: GameMode::Cutscene
            }
        );
        assert_eq!(
            decide_suspend(false, GameMode::Paused, &mut saved),
            SuspendDecision::Restored {
                restore_to: GameMode::Cutscene
            }
        );
        assert_eq!(saved, None);
    }

    /// A second suspend while a mode is already saved OVERWRITES it, which is
    /// what makes keeping a refused value safe: a user who really did navigate
    /// away and then backgrounded again gets the mode they navigated TO.
    #[test]
    fn a_second_capture_overwrites_a_kept_mode() {
        let mut saved = None;
        decide_suspend(true, GameMode::Playing, &mut saved);
        decide_suspend(false, GameMode::Playing, &mut saved); // refused, kept
        assert_eq!(saved, Some(GameMode::Playing));

        assert_eq!(
            decide_suspend(true, GameMode::Cutscene, &mut saved),
            SuspendDecision::Captured {
                previous: GameMode::Cutscene
            }
        );
        assert_eq!(saved, Some(GameMode::Cutscene));
    }
}

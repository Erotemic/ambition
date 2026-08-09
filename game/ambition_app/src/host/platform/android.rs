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

#[cfg(target_os = "android")]
use ambition_platformer2d::platformer::schedule::GameMode;

// Bevy's CosmicFontSystem is initialized with an empty fontdb (no system
// fonts loaded). On Android, /system/fonts/ holds Roboto etc., which
// fontdb won't find on its own. We seed it at Startup so that cosmic-text
// always has at least one font face before text shaping can be triggered —
// otherwise it panics with "no default font found" during the first frame
// before the async asset server delivers the game's custom fonts.
#[cfg(target_os = "android")]
fn seed_android_system_fonts(mut font_system: ResMut<bevy::text::CosmicFontSystem>) {
    font_system.0.db_mut().load_fonts_dir("/system/fonts");
    bevy::log::info!(
        target: "ambition_platformer2d::android_platform",
        "android: seeded fontdb with {} face(s) from /system/fonts",
        font_system.0.db().faces().count()
    );
}

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
            _app.add_systems(Startup, seed_android_system_fonts);
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
    /// pause is in effect; cleared (taken) when we restore it.
    mode_before_suspend: Option<GameMode>,
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
            target: "ambition_platformer2d::android_lifecycle",
            "android suspend state changed: {} -> {}",
            was,
            next
        );
        // The same edge on the marker channel, carrying the FRAME. The tracing
        // line above is what a 2026-08-08 device log showed for a spurious
        // 0.6s suspend/resume pair, and it could not be ordered against the
        // menu close that happened between them because neither line knew what
        // frame it was on.
        ambition_platformer2d::platformer::world_log::world_event(format_args!(
            "android-suspend-edge suspended {was} -> {next}"
        ));
    }
}

/// On the suspend edge, force `GameMode::Paused` and remember the mode
/// we came from. On the resume edge, restore that mode — but only if
/// we're still sitting in the `Paused` we forced, so we never stomp a
/// menu / dialogue / transition the user navigated into while the app
/// was backgrounded. This mirrors the audio channels, which already
/// auto-resume on the same edge; without it the game stays frozen with
/// no visible affordance to un-pause.
#[cfg(target_os = "android")]
fn apply_android_suspend_to_game_mode(
    mut state: ResMut<AndroidSuspendState>,
    mode: Res<State<GameMode>>,
    mut next_mode: ResMut<NextState<GameMode>>,
) {
    if !state.just_changed {
        return;
    }
    // ✔ FIXED 2026-08-09; the instrumentation stays. The resume branch used to
    // `take()` the saved mode before the guard decided whether to accept it, so
    // a refused restore consumed the value with no second chance. It now peeks
    // and clears only on the branch that restores — see the comment there for
    // why the guard refuses more often than it looks (`NextState` is deferred).
    // The three world-log outcomes below are unchanged and still say which one
    // happened, because that is how a device report gets ordered.
    //
    // ⛔ BLIND FIX: written without a device. No `adb` here, so this is reasoned
    // from the 2026-08-08 log and the code, not observed. What it makes
    // unreachable is the unrecoverable state; whether it is the freeze Jon hit
    // needs a device run to confirm.
    let observed = *mode.get();
    if state.suspended {
        // Only flip into Paused if gameplay was actually active.
        // Leaving Dialogue alone avoids stomping a mid-NPC
        // conversation when the user briefly checks notifications;
        // Paused / RoomTransition are already non-playing states.
        if matches!(observed, GameMode::Playing | GameMode::Cutscene) {
            ambition_platformer2d::platformer::world_log::world_event(format_args!(
                "android-suspend captured={} (forcing paused)",
                observed.label()
            ));
            state.mode_before_suspend = Some(observed);
            ambition_platformer2d::platformer::world_log::note_game_mode_request(
                GameMode::Paused,
                "android_suspend",
            );
            next_mode.set(GameMode::Paused);
        } else {
            ambition_platformer2d::platformer::world_log::world_event(format_args!(
                "android-suspend captured=none (mode={} is not Playing/Cutscene, so the \
                 resume edge will have nothing to restore)",
                observed.label()
            ));
        }
    } else if let Some(prev) = state.mode_before_suspend {
        // Resume edge: restore the pre-suspend mode, but only if the
        // suspend-induced Paused is still the current mode. If the user
        // opened a menu or otherwise moved off it while backgrounded,
        // leave their navigation alone.
        //
        // ⛔ **PEEKED, NOT TAKEN — and that is the whole fix (2026-08-09).**
        // This was `.take()`, which consumed the saved mode before the guard
        // below decided whether to use it. A refused restore therefore threw
        // the value away and no later resume could recover it.
        //
        // ⚠ **and the guard refuses more often than it looks, because
        // `NextState` is DEFERRED.** The suspend edge calls
        // `next_mode.set(GameMode::Paused)`; the transition applies later. On a
        // spurious short suspend/resume pair — a 0.6 s one is in the
        // 2026-08-08 device log — the resume edge can run while `observed` is
        // still the PRE-pause mode, so `matches!(observed, Paused)` is false,
        // the restore is refused, and the deferred `Paused` lands immediately
        // afterwards. Under `.take()` that left the game paused with the only
        // thing that could unpause it already discarded, which matches Jon's
        // report exactly: *"I can still do the menu … but I can't move my
        // character."*
        //
        // Keeping the value is safe in the other direction: a genuinely
        // navigated-away user either suspends again (the capture branch
        // overwrites it) or eventually resumes from the forced `Paused`, which
        // is precisely when restoring is what they want.
        //
        // ⚠ **THIS MAKES THE FREEZE RECOVERABLE, NOT IMPOSSIBLE.** This whole
        // function early-returns unless `just_changed`, so a refused restore is
        // only retried on the NEXT suspend/resume edge — background and
        // foreground once more and the mode comes back. A player who never
        // backgrounds again is still stuck. Closing that needs the guard to stop
        // inferring "we forced this pause" from `observed`, either by consulting
        // the pending `NextState` or by recording the fact on the capture edge.
        // Both change behaviour on a platform with no test here, so they wait
        // for a device — see the queue row.
        if matches!(observed, GameMode::Paused) {
            state.mode_before_suspend = None;
            ambition_platformer2d::platformer::world_log::world_event(format_args!(
                "android-resume saved={} observed={} -> RESTORED",
                prev.label(),
                observed.label()
            ));
            ambition_platformer2d::platformer::world_log::note_game_mode_request(
                prev,
                "android_resume",
            );
            next_mode.set(prev);
        } else {
            // ⭐ THE LINE THIS WHOLE PROBE EXISTS FOR — and it no longer
            // reports a loss. The guard refused, and the saved mode is KEPT for
            // the next resume edge instead of being discarded. A freeze report
            // still ordered against this line answers the same question; it
            // just no longer answers it with an unrecoverable state.
            ambition_platformer2d::platformer::world_log::world_event(format_args!(
                "android-resume saved={} observed={} -> DEFERRED (guard refused; saved mode \
                 KEPT for a later resume edge)",
                prev.label(),
                observed.label()
            ));
        }
    } else {
        ambition_platformer2d::platformer::world_log::world_event(format_args!(
            "android-resume saved=none observed={}",
            observed.label()
        ));
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

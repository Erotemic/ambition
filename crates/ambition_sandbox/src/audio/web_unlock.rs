//! Browser AudioContext unlock detection + ECS readiness flag.
//!
//! Browsers create the Web Audio `AudioContext` in the `suspended`
//! state and only let `ctx.resume()` succeed when called from inside
//! a user-gesture event handler. Kira (via cpal's webaudio backend)
//! constructs the AudioContext at startup and calls `ctx.resume()`
//! lazily from `Stream::play()`; those play calls dispatch from
//! Bevy's RAF loop, *not* from a gesture handler, so the resume
//! silently fails and audio stays muted forever.
//!
//! The fix has two halves:
//!
//! 1. **JS-side resume** — `crates/ambition_sandbox/web/index.html`
//!    patches `window.AudioContext` to track every context cpal
//!    creates, then resumes them all from a real DOM gesture handler
//!    (`pointerdown` / `keydown` / `touchstart` / `click`). This is
//!    the half that actually unblocks playback.
//!
//! 2. **Rust-side gating** — this module observes the *first* Bevy
//!    input event and flips [`AudioUnlockState::unlocked`] to `true`.
//!    Music + SFX startup gates itself on that flag so we don't fire
//!    a `play()` against a context the JS hook hasn't had a chance
//!    to resume yet.
//!
//! On non-wasm targets the JS hook is irrelevant and the
//! `AudioUnlockState` flips on the first frame so behavior matches
//! the pre-deferred startup. Cross-platform call sites can read
//! `unlock.unlocked` uniformly.

#![cfg(feature = "audio")]

use bevy::input::touch::Touches;
use bevy::input::ButtonInput;
use bevy::log::info;
use bevy::prelude::{
    App, KeyCode, MouseButton, Plugin, Res, ResMut, Resource, Startup, Update,
};

pub const AUDIO_LOG_TARGET: &str = "ambition::audio";

/// ECS-visible readiness signal for "is it safe to start playback?".
///
/// - On wasm, flips to `true` the frame we observe the first user
///   gesture. The JS unlock shim in `web/index.html` resumes the
///   AudioContext from inside that same gesture event handler, so
///   by the time downstream `Update` systems see `unlocked == true`
///   the context is (or is in the middle of becoming) `running`.
/// - On desktop / Android, gestures are not required by the audio
///   backend, so this is force-flipped to `true` during Startup.
#[derive(Resource, Default, Clone, Copy, Debug)]
pub struct AudioUnlockState {
    pub unlocked: bool,
    /// Number of `Update` frames observed since startup. Lets the
    /// "we never saw a gesture" warning fire at a sensible moment
    /// without spamming.
    pub frames_since_startup: u64,
}

pub struct WebAudioUnlockPlugin;

impl Plugin for WebAudioUnlockPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AudioUnlockState>()
            .add_systems(Startup, (log_initial_lock_status, prime_unlock_for_native))
            .add_systems(Update, observe_unlock_gesture);
    }
}

fn log_initial_lock_status() {
    #[cfg(target_arch = "wasm32")]
    {
        info!(
            target: AUDIO_LOG_TARGET,
            "ambition audio: kira plugin installed; AudioContext is suspended until \
             first user gesture (click / key / touch). The JS shim in web/index.html \
             resumes the context on gesture and logs `[ambition-audio] resume() ...`."
        );
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        info!(
            target: AUDIO_LOG_TARGET,
            "ambition audio: kira plugin installed (native backend; no gesture gate)."
        );
    }
}

/// Native (desktop / Android) backends don't require a user gesture
/// to start audio. Flip the unlock flag in Startup so downstream
/// systems that gate on it behave identically to the pre-deferred
/// startup.
fn prime_unlock_for_native(mut state: ResMut<AudioUnlockState>) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        state.unlocked = true;
    }
    #[cfg(target_arch = "wasm32")]
    {
        // Wasm path: stay locked until `observe_unlock_gesture` sees
        // a real input. `state` is intentionally untouched here.
        let _ = state;
    }
}

/// Watch for the first input event and:
/// - emit a one-shot log line so devtools captures the unlock moment
/// - flip [`AudioUnlockState::unlocked`] so downstream playback
///   systems can fire their first `play()` call.
fn observe_unlock_gesture(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    touches: Res<Touches>,
    mut state: ResMut<AudioUnlockState>,
) {
    state.frames_since_startup = state.frames_since_startup.saturating_add(1);
    if state.unlocked {
        return;
    }
    let gesture = keys.get_just_pressed().next().is_some()
        || mouse.get_just_pressed().next().is_some()
        || touches.iter_just_pressed().next().is_some();
    if gesture {
        info!(
            target: AUDIO_LOG_TARGET,
            "ambition audio: first user gesture observed; flagging AudioUnlockState. \
             Music + SFX startup will now fire."
        );
        state.unlocked = true;
    }
}

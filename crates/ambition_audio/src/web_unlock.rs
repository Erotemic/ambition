//! Cross-platform audio-start readiness.
//!
//! On wasm, `web/index.html` resumes Web Audio contexts from a real DOM gesture;
//! this module publishes the corresponding first Bevy input event through
//! [`AudioUnlockState`] so music and SFX do not start before that gesture. Native
//! backends mark the state ready during startup.

use bevy::input::touch::Touches;
use bevy::input::ButtonInput;
use bevy::log::info;
use bevy::prelude::{App, KeyCode, MouseButton, Plugin, Res, ResMut, Resource, Startup, Update};

pub const AUDIO_LOG_TARGET: &str = "ambition_platformer2d::audio";

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

/// Native backends require no gesture, so publish readiness during startup.
fn prime_unlock_for_native(
    // Written only by the native arm below; the wasm build waits for a real
    // gesture instead, so there the binding is read-only.
    #[cfg_attr(target_arch = "wasm32", allow(unused_mut))] mut state: ResMut<AudioUnlockState>,
) {
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

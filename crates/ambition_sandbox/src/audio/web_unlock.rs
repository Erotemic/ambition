//! Browser AudioContext unlock detection.
//!
//! Browsers create the Web Audio `AudioContext` in the `suspended`
//! state and only resume it after a user gesture (click, key, touch).
//! Kira's `cpal` backend handles the actual resume call once a sound
//! tries to play after the gesture, so we don't need to poke the
//! context from Rust — but the user sees the game boot silent and
//! might assume audio is broken. This plugin logs a clear status line
//! at startup and a second line the first time we see a user gesture,
//! so devtools shows the unlock moment.
//!
//! On non-wasm targets the systems are still registered but the
//! "audio locked" startup line is suppressed (desktop audio is not
//! gesture-gated). The unlock log fires on the first input event on
//! every platform — harmless on desktop, useful for cross-checking.

#![cfg(feature = "audio")]

use bevy::input::touch::Touches;
use bevy::input::ButtonInput;
use bevy::log::info;
use bevy::prelude::{App, KeyCode, Local, MouseButton, Plugin, Res, Startup, Update};

pub const AUDIO_LOG_TARGET: &str = "ambition::audio";

pub struct WebAudioUnlockPlugin;

impl Plugin for WebAudioUnlockPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, log_initial_lock_status)
            .add_systems(Update, log_first_unlock_gesture);
    }
}

fn log_initial_lock_status() {
    #[cfg(target_arch = "wasm32")]
    {
        info!(
            target: AUDIO_LOG_TARGET,
            "audio locked until first user gesture (click / key / touch); \
             kira will start playback once the AudioContext resumes"
        );
    }
}

/// Watch for the first input event and emit a one-shot log so the
/// browser devtools captures the exact moment audio could start
/// playing. Uses `ButtonInput::get_just_pressed` rather than message
/// readers to keep the system param list tiny and let it coexist with
/// the rest of the input pipeline.
fn log_first_unlock_gesture(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    touches: Res<Touches>,
    mut logged: Local<bool>,
) {
    if *logged {
        return;
    }
    let gesture = keys.get_just_pressed().next().is_some()
        || mouse.get_just_pressed().next().is_some()
        || touches.iter_just_pressed().next().is_some();
    if gesture {
        info!(target: AUDIO_LOG_TARGET, "audio unlocked (user gesture detected)");
        *logged = true;
    }
}


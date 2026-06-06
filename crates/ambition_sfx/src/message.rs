//! SFX request message.
//!
//! [`SfxMessage`] is the gameplay-facing request enum: mechanics
//! (portal, gravity, abilities, …) write it; a consumer (the sandbox
//! audio runtime) reads it and plays the corresponding sound. It lives
//! here — next to [`crate::SfxId`] — so reusable mechanics can request
//! sound without naming any sandbox-internal audio module.
//!
//! The typed variants (`Jump`, `Dash`, …) are convenience cues that map
//! to a small fixed sound set; `Play { id }` is the open-ended path that
//! plays any [`SfxId`]. How a consumer turns a typed variant into an
//! actual sound is the consumer's concern (the sandbox keeps that
//! mapping next to its `SoundCue` table).
//!
//! The `Message` derive (a `bevy_ecs` trait) and `Vec2` (from
//! `bevy_math`) are behind the default `bevy` feature so this crate
//! stays usable from headless / RL / benchmarking contexts that don't
//! pull in Bevy: with `default-features = false` the rest of the crate
//! (ids, providers) compiles with no Bevy dependency.

use crate::SfxId;
use bevy_math::Vec2;

/// A request to play a sound effect. Written by gameplay mechanics,
/// consumed by the audio runtime.
#[derive(bevy_ecs::message::Message, Clone, Copy, Debug)]
pub enum SfxMessage {
    Jump { pos: Vec2 },
    DoubleJump { pos: Vec2 },
    Dash { pos: Vec2 },
    Blink { pos: Vec2, precision: bool },
    Pogo { pos: Vec2 },
    Slash { pos: Vec2 },
    Hit { pos: Vec2 },
    Death { pos: Vec2 },
    Reset { pos: Vec2 },
    Play { id: SfxId, pos: Vec2 },
}

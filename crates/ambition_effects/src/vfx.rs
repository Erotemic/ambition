//! The visual-effects MESSAGE vocabulary — the presentation-neutral data a
//! simulation system emits to ask for a cue, with NO renderer attached.
//!
//! This lives in the foundation crate (not in `presentation`) so a sim system
//! that only fires a one-shot effect ("spawn an impact here", "blink dust from
//! A to B") does not depend on the whole rendering module. The presentation
//! layer owns the subscriber that turns each [`VfxMessage`] into actual
//! particle / explosion / speech-bubble entities, plus the render/audio
//! mappings for [`ExplosionKind`] (which spritesheet row, which packed SFX).
//!
//! Headless builds simply omit the subscriber: messages accumulate and drain
//! without spawning anything, so gameplay stays ECS-native and testable.

use bevy::prelude::*;

use ambition_engine_core as ae;

/// Particle flavour for a [`VfxMessage::Burst`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParticleKind {
    Spark,
    Dust,
    Shard,
}

/// Which explosion to play. The variants are pure data; the render mapping
/// (`explosion_anim` → spritesheet row) and audio mapping (`explosion_sfx` →
/// packed-bank id) live in the presentation layer, keeping this enum free of a
/// render/audio dependency.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExplosionKind {
    ClassicBurst,
    BurstRound,
    Shockwave,
    SmokeBurst,
    Starburst,
}

/// Typed visual-effects message (Bevy 0.18 buffered Message API). Emitted by
/// simulation systems; the presentation-side subscriber spawns the actual
/// particle / impact / slash entities. See the module docs.
#[derive(Message, Clone, Debug)]
pub enum VfxMessage {
    Burst {
        pos: ae::Vec2,
        count: u32,
        speed: f32,
        color: [f32; 4],
        kind: ParticleKind,
    },
    Dust {
        pos: ae::Vec2,
        facing: f32,
    },
    Impact {
        pos: ae::Vec2,
    },
    Explosion {
        pos: ae::Vec2,
        kind: ExplosionKind,
        scale: f32,
    },
    BlinkEffects {
        from: ae::Vec2,
        to: ae::Vec2,
        precision: bool,
    },
    SlashPreview {
        hitbox: ae::Aabb,
    },
    ResetEffects {
        from: ae::Vec2,
        to: ae::Vec2,
    },
    SpeechBubble {
        pos: ae::Vec2,
        text: String,
    },
}

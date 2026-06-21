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

/// Which slash-effect row to play. Maps to the `robot_slash` sheet's
/// directional rows: `Side`/`Up` are energy-arc crescents (most attacks),
/// `Down` is a tapered lance/poke (down-tilt / pogo). The attacker picks one
/// from its `AttackIntent`; eventually each attack can carry a bespoke effect.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlashDir {
    Side,
    Up,
    Down,
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
    /// A melee slash effect (the `robot_slash` sheet) at `center`, drawn
    /// `size` wide/tall, flipped by `facing`, playing the `dir` row once.
    Slash {
        center: ae::Vec2,
        size: f32,
        dir: SlashDir,
        facing: f32,
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

/// Packed-bank SFX id an [`ExplosionKind`] plays. A pure id mapping (no bank
/// loading), so it lives in the foundation with the request vocab — only the
/// spritesheet-row mapping (`explosion_anim`) is render-specific and stays in
/// presentation.
pub fn explosion_sfx(kind: ExplosionKind) -> ambition_sfx::SfxId {
    match kind {
        ExplosionKind::ClassicBurst => ambition_sfx::ids::VFX_EXPLOSION_CLASSIC_BURST,
        ExplosionKind::BurstRound => ambition_sfx::ids::VFX_EXPLOSION_BURST_ROUND,
        ExplosionKind::Shockwave => ambition_sfx::ids::VFX_EXPLOSION_SHOCKWAVE,
        ExplosionKind::SmokeBurst => ambition_sfx::ids::VFX_EXPLOSION_SMOKE_BURST,
        ExplosionKind::Starburst => ambition_sfx::ids::VFX_EXPLOSION_STARBURST,
    }
}

/// A reusable explosion CUE request: a sim system writes this to ask for an
/// explosion's visual + paired sound, without depending on the renderer. The
/// presentation `process_explosion_requests` fans it out to [`VfxMessage`] + the
/// SFX channel.
#[derive(Message, Clone, Debug)]
pub struct ExplosionRequest {
    pub pos: ae::Vec2,
    pub kind: ExplosionKind,
    pub scale: f32,
    pub sfx: Option<ambition_sfx::SfxId>,
}

impl ExplosionRequest {
    pub fn new(pos: ae::Vec2, kind: ExplosionKind) -> Self {
        Self {
            pos,
            kind,
            scale: 1.0,
            sfx: Some(explosion_sfx(kind)),
        }
    }

    pub fn classic(pos: ae::Vec2) -> Self {
        Self::new(pos, ExplosionKind::ClassicBurst)
    }

    pub fn with_scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }

    #[allow(dead_code)]
    pub fn without_sfx(mut self) -> Self {
        self.sfx = None;
        self
    }

    #[allow(dead_code)]
    pub fn with_sfx(mut self, sfx: ambition_sfx::SfxId) -> Self {
        self.sfx = Some(sfx);
        self
    }
}

/// Request a short, spatially distributed sequence of explosion VFX/SFX. Higher
/// level than several [`ExplosionRequest`]s: callers say "fireworks here" and the
/// presentation `process_fireworks_requests` owns the temporal spread + variety.
#[derive(Message, Clone, Debug)]
pub struct FireworksRequest {
    pub origin: ae::Vec2,
    pub count: u32,
    pub spread: ae::Vec2,
    pub duration: f32,
}

impl FireworksRequest {
    pub fn around(origin: ae::Vec2) -> Self {
        Self {
            origin,
            count: 11,
            spread: ae::Vec2::new(360.0, 210.0),
            duration: 2.35,
        }
    }
}

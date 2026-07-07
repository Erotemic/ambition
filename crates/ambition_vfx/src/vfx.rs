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

/// High-level physics-debris recipe a gameplay event handler emits
/// (breakable shatter, ragdoll burst). Pure data — the physics adapter owns
/// the subscriber that spawns actual debris bodies (`ambition_actors::
/// world::physics::physics_spawn_debris_messages`); headless builds omit it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhysicsDebrisCue {
    Impact,
    Breakable,
    EnemyRagdoll,
    BossRagdoll,
}

/// Typed physics-debris message (the debris twin of [`VfxMessage`]).
/// Bundled into the same `SandboxEventWriters` SystemParam as `SfxMessage`
/// and `VfxMessage` to stay within Bevy's 16-system-param budget.
#[derive(Message, Clone, Copy, Debug)]
pub struct DebrisBurstMessage {
    pub pos: ae::Vec2,
    pub cue: PhysicsDebrisCue,
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

/// Which slash-effect ART to play, independent of which way it points.
/// `Arc` is the sweeping energy crescent (most swings); `Poke` is the tapered
/// lance/thrust (down-tilt). Orientation is carried separately as a world
/// `dir`, so one art serves every direction under any gravity — the effect is
/// oriented in the attacker's reference frame, not screen space.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlashKind {
    Arc,
    Poke,
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
    /// `size` square, playing `kind` once. `dir` is the WORLD direction from
    /// the attacker to the strike (player→hitbox) — already gravity-relative —
    /// so the renderer orients the art toward it, keeping the effect in the
    /// attacker's reference frame under any gravity.
    Slash {
        center: ae::Vec2,
        size: f32,
        kind: SlashKind,
        dir: ae::Vec2,
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

/// The content-registered COSMETIC vfx vocabulary a move's `Vfx { effect }`
/// event (CM5 per-move presentation) resolves against. A pure id→kind mapping
/// (no render/audio dep): the moveset dispatcher turns the resolved kind into a
/// [`VfxMessage::Explosion`] at the owner, and the startup validator
/// (`MoveSpec::presentation_problems`) rejects any move naming an id NOT in this
/// table — so a typo fails loudly at load, never as a silent missing effect.
/// The burst vocabulary is deliberately the shared [`ExplosionKind`] set: a
/// jab authors `"burst_round"`, a smash `"shockwave"`, a launcher `"starburst"`
/// — distinct looks, zero new art plumbing. Add a row here when a move needs a
/// look the current five can't express.
pub fn move_vfx_kind(effect: &str) -> Option<ExplosionKind> {
    Some(match effect {
        "classic_burst" => ExplosionKind::ClassicBurst,
        "burst_round" => ExplosionKind::BurstRound,
        "shockwave" => ExplosionKind::Shockwave,
        "smoke_burst" => ExplosionKind::SmokeBurst,
        "starburst" => ExplosionKind::Starburst,
        _ => return None,
    })
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

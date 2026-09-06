//! The visual-effects MESSAGE vocabulary — the presentation-neutral data a
//! simulation system emits to ask for a cue, with NO renderer attached.
//!
//! This lives in the foundation crate (not in `presentation`) so a sim system
//! that only fires a one-shot effect ("spawn an impact here", "blink dust from
//! A to B") does not depend on the whole rendering module. The presentation
//! layer owns the subscriber that turns each [`VfxMessage`] into actual
//! particle / effect / speech-bubble entities, and resolves an [`FxId`] to the
//! sheet row and packed cue the authored name already names.
//!
//! Headless builds simply omit the subscriber: messages accumulate and drain
//! without spawning anything, so gameplay stays ECS-native and testable.

use bevy::prelude::*;

use crate::fx::FxId;

// VFX depends on generic geometry only; the message vocabulary has no
// platformer dependency.
use ambition_geometry as ae;

/// Particle flavour for a [`VfxMessage::Burst`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParticleKind {
    Spark,
    Dust,
    Shard,
}

/// High-level physics-debris recipe a gameplay event handler emits
/// (breakable shatter, ragdoll burst). Pure data — the physics adapter owns
/// the subscriber that spawns actual debris bodies (`ambition_platformer2d_actor_monolith::
/// world::physics::physics_spawn_debris_messages`); headless builds omit it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhysicsDebrisCue {
    Impact,
    Breakable,
    EnemyRagdoll,
    BossRagdoll,
}

/// Typed physics-debris message (the debris twin of [`VfxMessage`]).
/// Bundled into the same `GameplayFeedbackWriters` SystemParam as `SfxMessage`
/// and `VfxMessage` to stay within Bevy's 16-system-param budget.
#[derive(Message, Clone, Copy, Debug)]
pub struct DebrisBurstMessage {
    pub pos: ae::Vec2,
    pub cue: PhysicsDebrisCue,
}

/// A particle burst thrown off when a strike lands on a body (CM8). `Copy` so it
/// rides the projected [`HurtFeedback`] with no allocation. Turn it into a real
/// [`VfxMessage::Burst`] with [`HitBurst::message`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HitBurst {
    pub count: u32,
    pub speed: f32,
    pub color: [f32; 4],
    pub kind: ParticleKind,
}

impl HitBurst {
    /// Reusable red shard spray for a solid body hit.
    pub const HURT: Self = Self {
        count: 14,
        speed: 300.0,
        color: [1.0, 0.34, 0.28, 0.88],
        kind: ParticleKind::Shard,
    };

    /// This burst as a positioned [`VfxMessage`].
    pub fn message(self, pos: ae::Vec2) -> VfxMessage {
        VfxMessage::Burst {
            pos,
            count: self.count,
            speed: self.speed,
            color: self.color,
            kind: self.kind,
        }
    }
}

/// Broad physical surface used only to resolve an attack-owned material
/// selector into a concrete contact sound. This is deliberately small: it is
/// not a general rendering material system, and the attack still owns whether
/// it asks for material-aware audio at all.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ImpactMaterial {
    /// Living / soft tissue: wet, dense contact.
    #[default]
    Flesh,
    /// Articulated machine body: hard shell plus internal crunch.
    Robot,
    /// Rigid world metal: bright ring over a hard impact.
    Metal,
}

/// How ONE body reacts to being struck (CM8): the VICTIM-owned half of hit
/// feedback — a default hurt sound plus the optional particle spray and physics
/// debris that body throws off. The victim owns these because they describe the
/// body being hurt, not the attack; an attack contributes only its own STRIKE
/// SOUND, which overrides `sfx` but never the spray. That split is the whole
/// point of CM8: an enemy struck by another enemy uses [`HurtFeedback::ENEMY`]
/// and so never borrows the player's red "you got hurt" burst (the old
/// `is_player`-keyed attacker-side payload, which fired for every victim of a
/// body-contact hit), while the player always keeps its hurt flash regardless of
/// what hit it.
///
/// `Copy`: projected onto the combat-owned `CombatTuning`, so it carries no
/// snapshot weight of its own (`SfxId` is a `u64`, [`ParticleKind`]/
/// [`PhysicsDebrisCue`] are small enums).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HurtFeedback {
    /// The sound this body makes when hit and the attack authored no strike
    /// sound of its own.
    pub sfx: ambition_sfx::SfxId,
    /// The physical body family used when an attack asks for a material-aware
    /// contact variant. Ordinary attacks ignore it.
    pub material: ImpactMaterial,
    /// The particle spray this body throws on a solid hit, if any.
    pub burst: Option<HitBurst>,
    /// The physics debris this body throws on a solid hit, if any.
    pub debris: Option<PhysicsDebrisCue>,
}

impl HurtFeedback {
    pub const PLAYER: Self = Self {
        sfx: ambition_sfx::ids::PLAYER_DAMAGE,
        material: ImpactMaterial::Robot,
        burst: Some(HitBurst::HURT),
        debris: Some(PhysicsDebrisCue::Impact),
    };

    /// An ordinary body's reaction: a plain `player.hit` tick, no spray, no
    /// debris. Every non-player body defaults to this, so enemy-vs-enemy contact
    /// no longer throws the player's hurt burst.
    pub const ENEMY: Self = Self {
        sfx: ambition_sfx::ids::PLAYER_HIT,
        material: ImpactMaterial::Flesh,
        burst: None,
        debris: None,
    };

    /// A mechanical actor: same conservative visual reaction as an ordinary
    /// enemy, but material-aware player attacks resolve to a crunchy machine hit.
    pub const ROBOT: Self = Self {
        sfx: ambition_sfx::ids::PLAYER_HIT,
        material: ImpactMaterial::Robot,
        burst: None,
        debris: None,
    };

    /// A rigid metal world surface. Used by breakables / props only when the
    /// striking attack explicitly requests material-aware contact audio.
    pub const METAL: Self = Self {
        sfx: ambition_sfx::ids::WORLD_ROCK_HIT,
        material: ImpactMaterial::Metal,
        burst: None,
        debris: None,
    };
}

impl Default for HurtFeedback {
    fn default() -> Self {
        Self::ENEMY
    }
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

/// Which directional ROW / pose to play for a slash effect.
/// `Side` is the broad forward sweep, `Up` the anti-air overhead arc, and
/// `Down` the downward cleave / poke.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlashPose {
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
    /// A coin popping out of a struck block.
    ///
    /// The block credits the wallet separately; this only draws the coin pop and
    /// never creates a collectible or collidable entity.
    ///
    ///  its own variant rather than a one-particle `Burst`, because a burst
    /// fans its particles around a circle: a single one leaves at whatever angle
    /// index zero lands on, which is sideways. "Out of the block" means UP.
    CoinPop {
        pos: ae::Vec2,
    },
    /// Draw the authored effect `fx`, at `pos`.
    ///
    ///  the whole vocabulary is the name: presentation resolves [`FxId`] to
    /// the sheet that holds that row and draws it. Any of the 189 shipped rows
    /// is reachable, and adding art adds looks with no engine edit — which is
    /// why this is not `kind: ExplosionKind` any more.
    Effect {
        pos: ae::Vec2,
        fx: FxId,
        scale: f32,
        /// Explicit orientation; use [`FxPose::UPRIGHT`] when no mirroring or
        /// rotation is intended.
        pose: FxPose,
    },
    BlinkEffects {
        from: ae::Vec2,
        to: ae::Vec2,
        precision: bool,
    },
    /// A melee slash effect, drawn for the swing `shape` — where the swing
    /// starts, which way it goes, how far, and how wide it is at each end. The
    /// renderer places, orients and sizes the art from that alone.
    ///
    /// `pose` chooses which authored row to use (`side` / `up` / `down`) so the
    /// presentation matches the move's real strike silhouette instead of
    /// rotating one generic arc for every attack.
    ///
    /// Preserve the swing shape through the message so presentation can fit the
    /// effect to the actual strike geometry. See [`ae::SwingShape`].
    Slash {
        /// The swing, in the STRIKING BODY'S frame — origin relative to the
        /// attacker, not to the world.
        ///
        /// Owner-local so presentation can follow the body for the full swing,
        /// matching the hitbox's owner anchoring.
        shape: ae::SwingShape,
        /// Who is swinging. Presentation re-places the effect on this body every
        /// frame, which is the same anchoring rule the hitbox already uses.
        owner: bevy::prelude::Entity,
        kind: SlashKind,
        pose: SlashPose,
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

/// A reusable effect CUE request: a sim system writes this to ask for an
/// effect's visual and its paired sound, without depending on the renderer.
/// The presentation `process_fx_requests` fans it out to [`VfxMessage`] + the
/// SFX channel.
///
/// `sfx` is an OVERRIDE, and `None` is the interesting case. The shipped bank carries one
/// `vfx.<family>.<row>` cue for every authored row, so the effect's own sound is a property of the
/// NAME and presentation resolves it — there is nothing for a caller to remember. Set `sfx` only to
/// say something other than what the art already says. Effect orientation in the emitter's frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FxPose {
    /// Mirror horizontally: the move's committed facing pointed left.
    pub mirror: bool,
    /// Rotation in radians, from the owner's gravity frame — the same angle the
    /// SPRITE renderer stands a body up with, so a body and the effect hanging
    /// off it cannot disagree about which way is up.
    pub angle: f32,
}

impl FxPose {
    /// Unmirrored and unrotated — what every emitter that never had an opinion
    /// was already getting.
    pub const UPRIGHT: Self = Self {
        mirror: false,
        angle: 0.0,
    };

    /// A pose from the two authorities a move's authored offset already uses.
    pub fn of(facing: f32, angle: f32) -> Self {
        Self {
            mirror: facing < 0.0,
            angle,
        }
    }
}

#[derive(Message, Clone, Debug)]
pub struct FxRequest {
    pub pos: ae::Vec2,
    pub fx: FxId,
    pub scale: f32,
    /// Play THIS cue instead of the effect's own paired one.
    pub sfx: Option<ambition_sfx::SfxId>,
    /// Presentation source that owns the effect and its paired audio.
    /// [`PresentationSourceId::unscoped`] delegates to the active context's
    /// primary source.
    pub source: ambition_sfx::PresentationSourceId,
    /// How the art is oriented — see [`FxPose`]. [`FxPose::UPRIGHT`] by default.
    pub pose: FxPose,
}

impl FxRequest {
    pub fn new(pos: ae::Vec2, fx: FxId) -> Self {
        Self {
            pos,
            fx,
            scale: 1.0,
            sfx: None,
            source: ambition_sfx::PresentationSourceId::unscoped(),
            pose: FxPose::UPRIGHT,
        }
    }

    /// The same request, drawn in `pose`.
    pub fn with_pose(mut self, pose: FxPose) -> Self {
        self.pose = pose;
        self
    }

    /// The same request, attributed to a specific presentation source.
    pub fn from_source(mut self, source: ambition_sfx::PresentationSourceId) -> Self {
        self.source = source;
        self
    }

    pub fn classic(pos: ae::Vec2) -> Self {
        Self::new(pos, crate::fx::ids::CLASSIC_BURST)
    }

    pub fn with_scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }

    /// Say something other than what the art says. The override arm of
    /// [`sfx`](Self::sfx) — a sustained effect asking for the looping variant of
    /// its own row's cue is the case that makes it real, and the only one the
    /// shipped fighter tables use.
    pub fn with_sfx(mut self, sfx: ambition_sfx::SfxId) -> Self {
        self.sfx = Some(sfx);
        self
    }
}

/// A fighter left play: draw the knockout beat where it went out.
///
/// ⛔⛔ A MESSAGE, AND IT USED TO BE A REBUILT READ-MODEL. `KnockoutsView` was a
/// `Resource` cleared and refilled on every SIMULATION ADVANCE and sampled once
/// per RENDER FRAME, which is two different clocks: a rollback host can run
/// several advances before one frame, so a knockout on an intermediate advance
/// was erased before presentation ever saw it — and a knockout on the latest
/// SPECULATIVE advance was drawn immediately, walking straight past the
/// confirmed-effect quarantine every other cue goes through.
///
/// ⭐ AS AN INTENT IT RIDES THE SAME PATH AS THE SFX AND THE CAMERA SHAKE THAT
/// ACCOMPANY IT: journalled by producing frame, replaced on resimulation,
/// released only when the frame is confirmed, and discarded outright if the
/// branch is abandoned. That is what the view was imitating badly.
///
/// ⭐⭐ AND IT CARRIES THE POSITION RATHER THAN AN `Entity`, which is what
/// retired the view's other defect. The view kept a `LastSeenBodies` cache in a
/// non-rollback `Local` because the KO position was destroyed before any
/// consumer could look — a respawn teleported the body on the same tick. D201
/// changed that: a body waiting out its death beat is not placed until the
/// window closes, so its position is simply READABLE where the stock is spent.
/// The cache had outlived the problem it existed for, and a `Local` that a
/// rewind does not restore was answering "where did the body leave play" from
/// the abandoned branch.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct KnockoutBeatRequested {
    /// Where the body was when it left play, in world space.
    pub pos: ae::Vec2,
    /// Whether that was its LAST stock. The simulation's own answer, never a
    /// comparison of remaining against zero on the presentation side.
    pub eliminated: bool,
    /// How fast it was going when it went out — the launch trail's own band, so
    /// the plume and the burst that ends it agree about the same flight.
    pub speed: f32,
}

/// Request a short, spatially distributed sequence of explosion VFX/SFX. Higher
/// level than several [`FxRequest`]s: callers say "fireworks here" and the
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

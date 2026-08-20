//! Entity-contract + moveset vocabulary — the gameplay-truth schema.
//!
//! This crate is the typed spine of the `EntityCatalog` target in
//! `docs/archive/reviews/sprite-pipeline-2026-07/data-driven-sprites-and-characters.md`: entities as
//! **contract bundles** (not categories), and abilities as **Smash-model move
//! timelines** that every actor plays through the same system.
//!
//! Two rules carry the design:
//!
//! - **One clock per move: the owner's proper time.** Every duration in a
//!   [`MoveSpec`] is seconds of the *owning actor's* clock — its entity dt
//!   (sim dt × whatever dilation that actor experiences: bullet-time, a time
//!   bubble, a relativistic zone). The bound clip's playback is slaved to the
//!   move's normalized phase, so a dilated actor's picture and hit windows
//!   slow together and can never desync. Dilation is a property of the
//!   actor's clock, never of this data — the schema stays
//!   frame-of-reference-free.
//! - **Entity-local logical space.** Move volumes are authored in the
//!   entity's local coordinates (+x = facing, y = up, origin = body center),
//!   never atlas pixels. Quality tiers rescale render textures; they cannot
//!   touch this data.
//!
//! The engine owns the *primitives* here (window, volume, event, gate,
//! cancel edge); content composes them into moves. A move is data — giving
//! the goblin the player's slash is a re-binding, not a Rust change.
//!
//! Authored as RON (this is Rust/hand-authored data; only Python-authored
//! interchange uses JSON). Headless by construction: no Bevy, no assets —
//! a simulation can parse, validate, and play a move without loading a PNG.

use std::collections::{BTreeMap, HashSet};

use serde::{Deserialize, Serialize};

pub mod action_scheme;
pub mod brain_profile_ref;
pub mod placements;

pub use brain_profile_ref::{BrainProfileId, BrainProfileRef};

/// The reward/effect represented by a pickup, a chest, or a defeated boss.
///
/// ⚠ **this lived in `ambition_interaction` until 2026-08-03, and moving it DOWN
/// is what let the boss-profile vocabulary move at all.** A boss authors its
/// post-defeat reward in `boss_profiles.ron`, so that vocabulary names this
/// type; the vocabulary belongs in `ambition_characters` (light, and what the
/// content compiler can link), but `ambition_interaction` DEPENDS on
/// `ambition_characters` — so naming it from there was a dependency cycle.
///
/// The type itself is a leaf noun: `i32` and `String`, no behaviour, no Bevy. It
/// was never interaction-specific, it was merely first needed there.
/// `ambition_interaction` re-exports it, so every existing path still resolves.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum PickupKind {
    Health { amount: i32 },
    Currency { amount: i32 },
    Ability { ability_id: String },
    StoryFlag { flag: String },
    Custom(String),
}

// ---------------------------------------------------------------------------
// Ability vocabulary: the ONE effect reference + its opaque params.
// ---------------------------------------------------------------------------

/// Opaque, structured parameters for a technique or prefab. Wraps a parsed
/// `ron::Value`; the consuming effect hydrates its OWN typed struct via
/// [`ParamValue::hydrate`], so this crate stays ignorant of every
/// content-owned param shape (fable review AJ1, option A). The authored RON is
/// byte-identical to a `Reflect`-typed form, so if a visual move editor ever
/// lands, swapping hydration to the type registry is a mechanical migration —
/// the data survives.
///
/// `Default` is the empty table `{}` (not `Unit`): a paramless `EffectRef`
/// hydrates cleanly into a technique's all-defaults `#[derive(Deserialize)]`
/// param struct.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParamValue(pub ron::Value);

impl Default for ParamValue {
    fn default() -> Self {
        ParamValue(ron::Value::Map(ron::Map::new()))
    }
}

impl ParamValue {
    /// Parse authored RON param text (`"(rise: 320.0)"`) into a value.
    pub fn parse(ron_text: &str) -> Result<Self, ron::error::SpannedError> {
        Ok(ParamValue(ron::from_str(ron_text)?))
    }

    /// Build params FROM a technique's own typed struct — the inverse of
    /// [`hydrate`](Self::hydrate), for the case where code composes an effect
    /// that an author could equally have written by hand. Round-trips through
    /// the authored RON text so the stored value is byte-identical to the
    /// hand-written form.
    pub fn from_typed<T: Serialize>(value: &T) -> Result<Self, ron::Error> {
        let text = ron::ser::to_string(value)?;
        ron::from_str(&text)
            .map(ParamValue)
            .map_err(|spanned| spanned.code)
    }

    /// Hydrate these params into a technique/prefab's own `Deserialize` type.
    /// The concrete type is declared AT the consumer — this crate never names
    /// it. A missing required field or a type mismatch fails here (the basis of
    /// the install-time param-schema check, R2.2). Enum-valued params are
    /// unsupported by `ron::Value`'s deserializer — model those as string tags.
    pub fn hydrate<T: serde::de::DeserializeOwned>(&self) -> Result<T, ron::Error> {
        self.0.clone().into_rust()
    }
}

/// A reference to a content-defined technique/effect by string key, carrying
/// its opaque [`ParamValue`] payload. This is the ONE ability-vocabulary
/// reference: timed events ([`MoveEventKind::Effect`]), sustained windows
/// ([`MoveWindow::sustain_effect`]), and on-hit volume payloads
/// ([`HitVolume::on_hit`]) all name an `EffectRef`. The engine never matches a
/// key; a content-owned technique recognizes it and hydrates its own params
/// (fable review AJ1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EffectRef {
    pub key: String,
    #[serde(default)]
    pub params: ParamValue,
}

impl EffectRef {
    /// A keyed effect with empty params — the common paramless case.
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            params: ParamValue::default(),
        }
    }
}

/// A param-schema check for one technique/prefab key: does an authored
/// [`ParamValue`] satisfy the technique's contract?
pub type ParamCheck = fn(&ParamValue) -> Result<(), String>;

/// A check that authored params HYDRATE into the technique's own `T` — the
/// common case. Register it as `registry.register("glider", check_hydrates::<GliderParams>)`;
/// a missing required field or a type mismatch becomes a startup error instead
/// of a mid-fight silent default.
pub fn check_hydrates<T: serde::de::DeserializeOwned>(params: &ParamValue) -> Result<(), String> {
    params.hydrate::<T>().map(|_| ()).map_err(|e| e.to_string())
}

/// Install-time param-schema validation registry (fable AJ1 / A1). Each
/// content-owned technique/prefab MAY register a [`ParamCheck`] under its
/// effect key; the content-validation pass runs every authored [`EffectRef`]
/// through [`validate`](Self::validate), so a param typo fails at startup, not
/// mid-fight. The engine matches no key, so an unregistered key always passes
/// (a paramless content-const technique needs no schema).
#[derive(Default)]
pub struct ParamSchemaRegistry {
    checks: BTreeMap<String, ParamCheck>,
}

impl ParamSchemaRegistry {
    /// Register a technique's param check. Last registration for a key wins
    /// (a re-register overrides — content install is the single caller).
    pub fn register(&mut self, key: impl Into<String>, check: ParamCheck) {
        self.checks.insert(key.into(), check);
    }

    /// True once at least one technique has registered a check.
    pub fn is_empty(&self) -> bool {
        self.checks.is_empty()
    }

    /// Validate one authored effect ref. Unknown keys pass (see the type doc).
    pub fn validate(&self, effect: &EffectRef) -> Result<(), String> {
        match self.checks.get(&effect.key) {
            Some(check) => {
                check(&effect.params).map_err(|e| format!("effect '{}': {e}", effect.key))
            }
            None => Ok(()),
        }
    }

    /// Validate a batch of authored refs; collect every failure (the content
    /// pass reports all typos at once rather than failing on the first).
    pub fn validate_all<'a, I>(&self, refs: I) -> Vec<String>
    where
        I: IntoIterator<Item = &'a EffectRef>,
    {
        refs.into_iter()
            .filter_map(|effect| self.validate(effect).err())
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Moves: the Smash-model timeline.
// ---------------------------------------------------------------------------

/// What a span of a move's timeline means, gameplay-wise.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WindowTag {
    /// Windup — no hits yet; the tell.
    Startup,
    /// The window's volumes are live hitboxes.
    Active,
    /// Follow-through — vulnerable, no hits.
    Recovery,
    /// The owner cannot be hit.
    Invuln,
    /// The owner takes hits without hitstun.
    Armor,
    /// The move may be canceled into the named moves (CM4). `into` entries
    /// share one namespace: literal move ids (`"jab2"`), verbs (`"special"`,
    /// `"attack"`), and classes (`"any_attack"`, `"jump"`, `"dash"`). The
    /// timeline IS the cancel table — combo/chain design is authored as
    /// windows, like everything else about a move.
    Cancelable {
        into: Vec<String>,
        /// When the escape is legal. Default `Always` — the pre-CM4 meaning
        /// of an authored `Cancelable` window (serde-default keeps existing
        /// RON rows parsing unchanged).
        #[serde(default)]
        condition: CancelCondition,
    },
}

/// The cancel-target CLASS namespace (CM4): names an authored `into` entry may
/// use besides a literal move id. Verbs + classes the trigger seam resolves.
pub const CANCEL_CLASS_NAMES: [&str; 6] =
    ["any_attack", "attack", "special", "ranged", "jump", "dash"];

/// When a [`WindowTag::Cancelable`] escape is legal (CM4).
///
/// `OnBlock` deliberately does NOT exist yet: the victim-shield-contact fact
/// lands with CM6 (shield-stun); adding the variant now would parse and then
/// silently never fire — an authoring trap.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum CancelCondition {
    /// Any time the window is open.
    #[default]
    Always,
    /// Only after this move CONNECTED with a victim (combo confirm — jab
    /// chains into jab2 on hit).
    OnHit,
    /// Only while the move has NOT connected (whiff escape — bail out of a
    /// missed heavy's recovery).
    OnWhiff,
}

/// An axis-aligned or circular hit volume in ENTITY-LOCAL logical space
/// (+x = facing; the runtime mirrors x for a left-facing actor).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum VolumeShape {
    /// Centered at `offset`, extending `half_extents` each way.
    Rect {
        offset: (f32, f32),
        half_extents: (f32, f32),
    },
    /// Centered at `offset` with radius `radius`.
    Circle { offset: (f32, f32), radius: f32 },
}

/// One hit volume carried by an [`WindowTag::Active`] window, with its hit
/// payload. Volumes live on their window — where the timeline says they are —
/// not in a parallel list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HitVolume {
    pub shape: VolumeShape,
    /// Damage dealt on contact.
    pub damage: i32,
    /// Knockback impulse magnitude (engine units; direction is derived from
    /// facing + contact by the combat runtime).
    #[serde(default)]
    pub knockback: f32,
    /// Knockback GROWTH per point of the victim's accumulated damage (CM1, the
    /// smash-percent axis): the applied knockback becomes
    /// `knockback + knockback_growth * victim.damage_taken() / victim.weight`. Default
    /// `0.0` == today's flat knockback exactly (parity by construction); content
    /// opts a row into growth to get percent-scaling launches.
    #[serde(default)]
    pub knockback_growth: f32,
    /// Body-local launch direction override `(+x = facing, +y = gravity-down)`.
    /// `None` = today's facing+contact derivation. The runtime mirrors x by
    /// facing and rotates into the owner's gravity frame (frame-correct under any
    /// gravity), then applies DI (CM2). Authored on strong-directional smash
    /// volumes that want a fixed launch angle instead of a contact-derived one.
    #[serde(default)]
    pub launch_dir: Option<(f32, f32)>,
    /// A conditional technique that fires WHEN this volume lands a hit, with
    /// the hit context (owner, victim, contact). The missing conditional
    /// primitive: pogo, lifesteal, on-hit status, launch modifiers. `None` for
    /// an ordinary damage volume (fable review AJ1). Down-air pogo authors
    /// `on_hit: Some(EffectRef { key: "pogo_bounce", .. })`.
    #[serde(default)]
    pub on_hit: Option<EffectRef>,
    /// Presentation tag for this volume's strike (§7.1/§7.2): a bladed swing
    /// authors `"slash_arc"` / `"slash_poke"` and the move runtime (a) draws the
    /// slash VFX from the SAME spawned volume (hitbox and slash can never point
    /// different ways) and (b) treats the volume as the character's BLADE —
    /// resolving the sprite-manifest's authored per-animation hit polygon (keyed
    /// by the move's clip name) in place of this synthetic shape when the owner
    /// authors one. `None` = a silent, data-shaped volume (boss geometry
    /// strikes, hazards) — no VFX, no manifest override. Unknown tags draw the
    /// default arc; the tag set is engine presentation vocabulary, not content.
    #[serde(default)]
    pub vfx: Option<String>,
    /// Authored STRIKE SOUND id (CM8): the sound THIS attack makes on contact,
    /// e.g. `"player.slash"` for a blade or `"world.rock.hit"` for a bludgeon —
    /// so a sword and a goblin swipe are heard apart even when they land on the
    /// same body. The string is the `SfxId` name (lowered via `SfxId::new` at
    /// spawn); an id the bank never rendered simply plays nothing, so authoring
    /// one is always safe. `None` = the victim's own default hurt sound. This is
    /// the ATTACK's contribution to hit feedback; the spray/debris a solid hit
    /// throws are the VICTIM's, carried on its `HurtFeedback`.
    #[serde(default)]
    pub hit_sfx: Option<String>,
}

// ---------------------------------------------------------------------------
// Hurtboxes: body-state and move-clock authored timelines.
// ---------------------------------------------------------------------------

/// One damageable body volume in entity-local logical space.
///
/// Kept distinct from [`HitVolume`] even though both reuse [`VolumeShape`]: a
/// hurtbox carries no attack payload. The wrapper leaves room for future
/// per-region hurt behavior without changing the timeline container.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HurtboxVolume {
    pub shape: VolumeShape,
}

/// One piecewise-constant hurtbox keyframe.
///
/// Its volumes remain active from `at_s` until the next keyframe. The clock is
/// supplied by authoritative simulation state: move elapsed time for a move
/// override, hitstun/tumble elapsed time for those pose profiles, or a
/// deterministic locomotion phase. Rendering and decoded sprite frames never
/// participate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HurtboxKeyframe {
    pub at_s: f32,
    pub volumes: Vec<HurtboxVolume>,
}

/// A deterministic, piecewise-constant hurtbox timeline.
///
/// Validation requires a first keyframe at `0.0`, strictly increasing finite
/// times, and at least one non-degenerate volume per keyframe. Those rules make
/// sampling total for every non-negative authoritative clock value and avoid
/// implicit interpolation or fallback gaps.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct HurtboxTimeline {
    pub keyframes: Vec<HurtboxKeyframe>,
}

impl HurtboxTimeline {
    /// Sample the most recent keyframe at or before `elapsed_s`.
    ///
    /// A malformed/unvalidated empty timeline or non-finite time returns
    /// `None`. Negative values clamp to the first keyframe so a tiny numerical
    /// underflow at state entry cannot select a different profile.
    pub fn volumes_at(&self, elapsed_s: f32) -> Option<&[HurtboxVolume]> {
        if self.keyframes.is_empty() || !elapsed_s.is_finite() {
            return None;
        }
        let elapsed_s = elapsed_s.max(0.0);
        let index = self
            .keyframes
            .partition_point(|keyframe| keyframe.at_s <= elapsed_s)
            .saturating_sub(1);
        Some(self.keyframes[index].volumes.as_slice())
    }
}

/// Authored hurtbox sources for one body.
///
/// Selection precedence is the settled character rule:
///
/// 1. active move override,
/// 2. current body pose/status profile,
/// 3. default authored body timeline,
/// 4. no authored answer (the runtime may use its sprite-derived bbox
///    compatibility fallback).
///
/// Pose ids intentionally remain an engine/content vocabulary rather than an
/// enum here: idle, run, crouch, shield, hitstun, tumble, airborne, ledge-hang,
/// and future deterministic body states all use the same data shape.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct HurtboxDoc {
    #[serde(default)]
    pub default: Option<HurtboxTimeline>,
    #[serde(default)]
    pub poses: BTreeMap<String, HurtboxTimeline>,
    #[serde(default)]
    pub moves: BTreeMap<String, HurtboxTimeline>,
}

impl HurtboxDoc {
    /// Resolve authored volumes using move -> pose -> default precedence.
    pub fn volumes_for(
        &self,
        active_move: Option<(&str, f32)>,
        pose: Option<(&str, f32)>,
    ) -> Option<&[HurtboxVolume]> {
        if let Some((move_id, elapsed_s)) = active_move {
            if let Some(volumes) = self
                .moves
                .get(move_id)
                .and_then(|timeline| timeline.volumes_at(elapsed_s))
            {
                return Some(volumes);
            }
        }
        if let Some((pose_id, elapsed_s)) = pose {
            if let Some(volumes) = self
                .poses
                .get(pose_id)
                .and_then(|timeline| timeline.volumes_at(elapsed_s))
            {
                return Some(volumes);
            }
        }
        self.default
            .as_ref()
            .and_then(|timeline| timeline.volumes_at(0.0))
    }

    /// Validate every authored profile without consulting rendering or assets.
    pub fn validate(&self) -> Vec<HurtboxError> {
        let mut errors = Vec::new();
        if let Some(default) = self.default.as_ref() {
            validate_hurtbox_timeline(HurtboxSource::Default, default, &mut errors);
        }
        for (pose_id, timeline) in &self.poses {
            let source = HurtboxSource::Pose(pose_id.clone());
            if pose_id.trim().is_empty() {
                errors.push(HurtboxError::EmptySourceId {
                    source: source.clone(),
                });
            }
            validate_hurtbox_timeline(source, timeline, &mut errors);
        }
        for (move_id, timeline) in &self.moves {
            let source = HurtboxSource::Move(move_id.clone());
            if move_id.trim().is_empty() {
                errors.push(HurtboxError::EmptySourceId {
                    source: source.clone(),
                });
            }
            validate_hurtbox_timeline(source, timeline, &mut errors);
        }
        errors
    }
}

/// Which authored clock/source owns a malformed hurtbox timeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HurtboxSource {
    Default,
    Pose(String),
    Move(String),
}

impl std::fmt::Display for HurtboxSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Default => f.write_str("default"),
            Self::Pose(id) => write!(f, "pose `{id}`"),
            Self::Move(id) => write!(f, "move `{id}`"),
        }
    }
}

/// Structural hurtbox-authoring failures detected before publication.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HurtboxError {
    EmptySourceId {
        source: HurtboxSource,
    },
    EmptyTimeline {
        source: HurtboxSource,
    },
    FirstKeyframeNotZero {
        source: HurtboxSource,
    },
    InvalidKeyframeTime {
        source: HurtboxSource,
        index: usize,
    },
    NonIncreasingKeyframeTime {
        source: HurtboxSource,
        index: usize,
    },
    EmptyKeyframe {
        source: HurtboxSource,
        index: usize,
    },
    DegenerateVolume {
        source: HurtboxSource,
        keyframe: usize,
        volume: usize,
    },
}

impl std::fmt::Display for HurtboxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptySourceId { source } => write!(f, "{source}: empty profile id"),
            Self::EmptyTimeline { source } => write!(f, "{source}: empty hurtbox timeline"),
            Self::FirstKeyframeNotZero { source } => {
                write!(f, "{source}: first hurtbox keyframe must start at 0")
            }
            Self::InvalidKeyframeTime { source, index } => {
                write!(f, "{source}: keyframe[{index}] has an invalid time")
            }
            Self::NonIncreasingKeyframeTime { source, index } => write!(
                f,
                "{source}: keyframe[{index}] does not follow a strictly increasing time"
            ),
            Self::EmptyKeyframe { source, index } => {
                write!(f, "{source}: keyframe[{index}] has no hurt volumes")
            }
            Self::DegenerateVolume {
                source,
                keyframe,
                volume,
            } => write!(
                f,
                "{source}: keyframe[{keyframe}] volume[{volume}] is degenerate"
            ),
        }
    }
}

fn validate_hurtbox_timeline(
    source: HurtboxSource,
    timeline: &HurtboxTimeline,
    errors: &mut Vec<HurtboxError>,
) {
    let Some(first) = timeline.keyframes.first() else {
        errors.push(HurtboxError::EmptyTimeline { source });
        return;
    };
    if first.at_s != 0.0 {
        errors.push(HurtboxError::FirstKeyframeNotZero {
            source: source.clone(),
        });
    }
    for (index, keyframe) in timeline.keyframes.iter().enumerate() {
        if !keyframe.at_s.is_finite() || keyframe.at_s < 0.0 {
            errors.push(HurtboxError::InvalidKeyframeTime {
                source: source.clone(),
                index,
            });
        }
        if index > 0 && keyframe.at_s <= timeline.keyframes[index - 1].at_s {
            errors.push(HurtboxError::NonIncreasingKeyframeTime {
                source: source.clone(),
                index,
            });
        }
        if keyframe.volumes.is_empty() {
            errors.push(HurtboxError::EmptyKeyframe {
                source: source.clone(),
                index,
            });
        }
        for (volume, hurtbox) in keyframe.volumes.iter().enumerate() {
            if !valid_volume_shape(hurtbox.shape) {
                errors.push(HurtboxError::DegenerateVolume {
                    source: source.clone(),
                    keyframe: index,
                    volume,
                });
            }
        }
    }
}

fn valid_volume_shape(shape: VolumeShape) -> bool {
    match shape {
        VolumeShape::Rect {
            offset,
            half_extents,
        } => {
            offset.0.is_finite()
                && offset.1.is_finite()
                && half_extents.0.is_finite()
                && half_extents.1.is_finite()
                && half_extents.0 > 0.0
                && half_extents.1 > 0.0
        }
        VolumeShape::Circle { offset, radius } => {
            offset.0.is_finite() && offset.1.is_finite() && radius.is_finite() && radius > 0.0
        }
    }
}

/// One span of a move's timeline. Times are seconds of the owner's proper
/// time, relative to move start.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoveWindow {
    pub start_s: f32,
    pub end_s: f32,
    pub tag: WindowTag,
    /// Hit volumes live during this window (meaningful for `Active`).
    #[serde(default)]
    pub volumes: Vec<HitVolume>,
    /// How much of the OWNER'S steering intent survives while its clock is
    /// inside this window — the move's authored MOTION LOCK. `1.0` (the
    /// default, every ordinary move) leaves steering untouched; `< 1.0` damps
    /// it (a committed heavy strike the body mustn't outrun — the boss
    /// strike-speed throttle authors this on its Active window); `0.0` roots
    /// the body for the window. Enforced BODY-side at integration
    /// ([`MoveSpec::motion_scale_at`]), so it holds for any controller —
    /// autonomous brain or possessing player alike (controller attempts, body
    /// enforces). Frame-agnostic: it scales intent magnitude, never a world
    /// direction.
    #[serde(default = "default_motion_scale")]
    pub motion_scale: f32,
    /// A SUSTAINED content effect: while this window is active, an `Effect { key }`
    /// is emitted EVERY frame (not one-shot like a `MoveEvent`). This is how a move
    /// expresses a HELD/continuous special — a beam that lingers, a rain that keeps
    /// falling — where the consuming technique times its own cadence off the
    /// per-frame "active this tick" signal (the shape the boss `apple_rain`-style
    /// specials need; the boss fold rides this). `None` for ordinary windows.
    #[serde(default)]
    pub sustain_effect: Option<EffectRef>,
}

/// A timed one-shot on the move timeline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MoveEventKind {
    /// Play a sound cue by key.
    Sfx { cue: String },
    /// Emit a purely COSMETIC visual effect by id (CM5 per-move presentation).
    /// Unlike [`Effect`](Self::Effect) (a gameplay technique) this changes only
    /// what the move LOOKS like — the sim emits the fact, presentation resolves
    /// the id against the rows the shipped FX spritesheets carry
    /// (`ambition_sprite_sheet::fx`) and draws that clip at the owner. A typo is
    /// a validation error where a validator is available
    /// (`MoveSpec::presentation_problems`) and a counted miss at draw time
    /// otherwise — never a silent no-op. This is how a jab, a smash, and a
    /// launcher look distinct with zero code — each authors its own
    /// `Vfx { effect }`.
    ///
    /// ⭐ **it also says WHERE and HOW BIG, because a move that could not was
    /// the whole of Jon's 2026-08-16 report**: *"right now we are seeing crazy
    /// upscaled vfx and very tiny hitboxes"*. Every authored effect drew as a
    /// fixed square at the owner's CENTRE, so a jab's spark bloomed out of the
    /// fighter's chest at the size of a super. Both fields are serde-defaulted
    /// to exactly the old behaviour, so no existing authored event moves.
    Vfx {
        effect: String,
        /// WHERE, body-local — `+x` toward the facing the move committed to,
        /// `+y` gravity-down — the same convention
        /// [`Impulse`](Self::Impulse) and every [`HitVolume`] offset use, and
        /// mirrored and rotated by the same two steps. `(0.0, 0.0)` is the
        /// owner's centre, which is where every effect drew before this existed.
        ///
        /// ⭐ **so an effect can sit on the box that throws it.** A move authors
        /// its strike volume's offset and its burst's offset in the same numbers.
        #[serde(default)]
        at: (f32, f32),
        /// HOW BIG, as a multiple of the presentation's default effect size.
        /// `1.0` is that default; a flourish asks for less and a screen-filling
        /// super asks for more.
        #[serde(default = "default_vfx_scale")]
        scale: f32,
        /// **WHAT IT SOUNDS LIKE, when that is not what it looks like.**
        ///
        /// ⭐⭐ `None` — the default and the overwhelming case — means *the cue
        /// the effect's own name addresses*. The shipped bank carries one
        /// `vfx.<family>.<row>` cue per authored row, so a burst that wants its
        /// own sound has already said which one by naming the art; presentation
        /// resolves it and the author remembers nothing.
        ///
        /// ⛔ **this field is why the ceremony could go.** Fourteen fighter
        /// tables hand-wrote a `Sfx` event beside every `Vfx` one — 74 of 145
        /// authored cues did nothing but restate the default — because the only
        /// way to say "a looping variant of this row's sound" was a second
        /// event. A sustained burst is ONE authored thing now, and the pair is
        /// not a thing an author can get half-right.
        #[serde(default)]
        sfx: Option<String>,
    },
    /// Emit a content-defined effect (the `Effect` vocabulary / technique seam
    /// resolves it), carrying its opaque params.
    Effect(EffectRef),
    /// FIRE the owner's ranged weapon now, sampling its LIVE aim at this frame.
    /// Content-free on purpose (mirrors [`Effect`](Self::Effect)): the move names
    /// "shoot", and the dispatcher reads the owner's `ActionSet.ranged` slot + its
    /// current aim/facing to build the concrete shot — so a `"fire"` move gets real
    /// startup/recovery windows while its projectile still tracks a strafing target
    /// (fable review: ranged subsumption, option A — dynamic aim, not facing-lock).
    Ranged,
    /// **A TIMED authored self-displacement**, body-local (`+x = facing,
    /// `+y = gravity-down`) — the move moving its own owner, at a moment the
    /// timeline chooses.
    ///
    /// ⭐ **why this exists when [`MoveSpec::start_impulse`] already did.** That
    /// field is a velocity ADD applied at TRIGGER, and both halves fail the one
    /// move a platform fighter cannot do without. A recovery special has to fire
    /// its burst AFTER its startup — that windup is the tell the whole move is
    /// balanced around — and it has to SET the rise rather than add to it,
    /// because a body that is falling at terminal velocity when it presses the
    /// button is exactly the body that needs it: `vel += -1000` while falling at
    /// 900 climbs at 100, so an additive recovery is strongest when it is least
    /// needed and useless when it is most. [`ImpulseMode::Set`] is the whole
    /// difference between an Up-B and a hop.
    ///
    /// ⛔ **not a character mechanic.** It is authored self-motion on a
    /// timeline, and its second and third customers are already obvious: a dive
    /// that commits downward mid-aerial, a lunge that travels on the active
    /// frame instead of the press.
    Impulse {
        /// Body-local `(side toward facing, gravity-down)` in engine units/s.
        local: (f32, f32),
        #[serde(default)]
        mode: ImpulseMode,
    },
}

/// How a [`MoveEventKind::Impulse`] meets the velocity the body already had.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImpulseMode {
    /// ADD to the current velocity — [`MoveSpec::start_impulse`]'s meaning, and
    /// the default so an authored impulse that says nothing behaves like the
    /// field it grew out of. A lunge, a drift nudge.
    #[default]
    Add,
    /// REPLACE the velocity outright. The move COMMANDS a speed rather than
    /// contributing to one, so its result does not depend on how fast the body
    /// happened to be falling. This is what makes a recovery move a recovery
    /// move, and it is the only mode a static reader can price
    /// ([`MoveFrameData::lift_speed`]).
    Set,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoveEvent {
    /// Seconds (owner's proper time) from move start.
    pub at_s: f32,
    pub kind: MoveEventKind,
}

/// Which semantic clip presents this move, with a declared fallback chain
/// (e.g. `tilt_up → slash → idle`). Resolution happens against the entity's
/// visual (pack or sheet) at bind time; a missing clip degrades presentation,
/// never simulation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClipBinding {
    pub clip: String,
    #[serde(default)]
    pub fallbacks: Vec<String>,
}

/// Activation gates for a move. Narrow on purpose — add knobs when real
/// moves need them.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MoveGates {
    /// `Some(true)` = grounded only; `Some(false)` = airborne only;
    /// `None` = either.
    #[serde(default)]
    pub grounded: Option<bool>,
}

impl MoveGates {
    /// Whether these gates permit activation in the given grounded state. A
    /// grounded-only move is skipped for an airborne body (and vice versa) so
    /// directional resolution falls through to a permitted fallback.
    pub fn permits(&self, grounded: bool) -> bool {
        match self.grounded {
            Some(required) => required == grounded,
            None => true,
        }
    }
}

/// One ability activation: a clip binding plus the full gameplay meaning of
/// the ability on one timeline. **The move timeline is authoritative for both
/// gameplay and presentation** — windows advance on the owner's proper time
/// and the bound clip is sampled by normalized move phase.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoveSpec {
    /// Stable move id (`"jab"`, `"tilt_up"`, `"sandbag_swat"`).
    pub id: String,
    pub clip: ClipBinding,
    /// Total move time, seconds of the owner's proper time.
    pub duration_s: f32,
    pub windows: Vec<MoveWindow>,
    #[serde(default)]
    pub events: Vec<MoveEvent>,
    #[serde(default)]
    pub gates: MoveGates,
    /// A one-shot body-local velocity ADD applied when the move is triggered —
    /// the move's self-motion (a jab's forward lunge, a dash-attack's slide, a
    /// back-air's drift). `(+x = facing, +y = gravity-down)`; the runtime mirrors
    /// x by facing and rotates it into the owner's gravity frame, so it stays
    /// frame-correct under any gravity. `None` = no self-motion (the identity
    /// case for every actor/boss move that doesn't lunge).
    #[serde(default)]
    pub start_impulse: Option<(f32, f32)>,
    /// Smash-charge payoff (CM3): the multiplier a FULLY-charged release applies
    /// to this move's damage and knockback. The applied scale interpolates
    /// `1.0 → smash_charge_mult` by the charge fraction reached at release (how
    /// far the owner's clock advanced through the leading Startup window). DEFAULT
    /// `1.0` = no charge scaling (every non-charge move, and Ambition's charge
    /// moves until a game opts in) — byte-parity. A smash roster authors e.g.
    /// `2.0` so a held smash lands twice as hard as a tap.
    #[serde(default = "default_charge_mult")]
    pub smash_charge_mult: f32,
    /// **Landing lag: the recovery this move owes if the body touches down
    /// before the move ended.** Seconds of the owner's proper time, spent as a
    /// hard control lock.
    ///
    /// The platform-fighter rule this expresses: an aerial is a COMMITMENT. You
    /// throw it knowing that landing mid-move costs you, which is what makes
    /// spacing and timing an aerial a decision rather than a free action.
    ///
    /// `None` = an aerial that lands is an ordinary landing, which is the
    /// behaviour of every move that has not opted in.
    #[serde(default)]
    pub landing_lag_s: Option<f32>,
    /// **Auto-cancel: land after this point in the move and pay NO landing
    /// lag.** Seconds of proper time from the move's start.
    ///
    /// The other half of the commitment: a move thrown early enough that its
    /// dangerous part is over by touchdown lands clean. Authoring the pair is
    /// how a designer says "rise with this one, do not fall with it".
    ///
    /// `None` = no auto-cancel window; [`Self::landing_lag_s`] applies whenever
    /// the move is still running. Ignored if no landing lag is authored.
    #[serde(default)]
    pub autocancel_after_s: Option<f32>,
}

/// Serde default for [`MoveSpec::smash_charge_mult`]: the multiplicative
/// identity, so every existing move is unscaled (parity).
fn default_charge_mult() -> f32 {
    1.0
}

/// Serde default for [`MoveEventKind::Vfx::scale`]: the presentation default
/// size, so every effect authored before the field existed draws exactly as it
/// did.
fn default_vfx_scale() -> f32 {
    1.0
}

/// Serde default for [`MoveWindow::motion_scale`]: the multiplicative
/// identity, so every existing window leaves steering untouched (parity).
fn default_motion_scale() -> f32 {
    1.0
}

impl MoveSpec {
    /// The player-facing label for this move — used by the action scheme to
    /// name the slot this move occupies. Today a title-cased `id`
    /// (`"sandbag_swat"` → `"Sandbag Swat"`); P6 adds an authored
    /// `display_name: Option<String>` field that this reads first, filled in
    /// the same commit that touches the move construction sites.
    pub fn display(&self) -> String {
        crate::action_scheme::title_case_id(&self.id)
    }

    /// CM5: validate this move's PRESENTATION event ids so a typo fails loudly
    /// at load, never as a silent missing sound/effect. `vfx_known` is the
    /// injected cosmetic-vfx vocabulary oracle — this crate does not depend on
    /// presentation, and the honest answer lives with the ART: pass
    /// `ambition_sprite_sheet::fx::is_authored_effect`, which reads the rows of
    /// the shipped FX sheets out of their baked manifests (pure; no App, no
    /// loaded assets). Returns one human-readable problem per bad id:
    /// - a `Vfx { effect }` whose id is not in the cosmetic vocabulary, and
    /// - a `Sfx { cue }` with an empty cue (a blank cue resolves to silence).
    /// Empty result = the move's presentation is resolvable.
    pub fn presentation_problems(&self, vfx_known: impl Fn(&str) -> bool) -> Vec<String> {
        let mut problems = Vec::new();
        for ev in &self.events {
            match &ev.kind {
                MoveEventKind::Vfx { effect, .. } if !vfx_known(effect) => {
                    problems.push(format!(
                        "move '{}': Vfx event names unknown cosmetic effect '{}' (no \
                         shipped FX spritesheet has a row by that name)",
                        self.id, effect
                    ));
                }
                MoveEventKind::Sfx { cue } if cue.is_empty() => {
                    problems.push(format!(
                        "move '{}': Sfx event has an empty cue (resolves to silence)",
                        self.id
                    ));
                }
                _ => {}
            }
        }
        problems
    }

    /// The windows carrying `tag`, in declaration order.
    pub fn windows_tagged(
        &self,
        want: fn(&WindowTag) -> bool,
    ) -> impl Iterator<Item = &MoveWindow> {
        self.windows.iter().filter(move |w| want(&w.tag))
    }

    /// The steering-intent scale in force at proper-time `t` — the MOST
    /// RESTRICTIVE (minimum) [`MoveWindow::motion_scale`] among the windows
    /// containing `t`, `1.0` outside every window. The body integrator
    /// multiplies the controller's steering intent by this each tick, so a
    /// move's motion lock binds every controller of the body uniformly.
    pub fn motion_scale_at(&self, t: f32) -> f32 {
        self.windows
            .iter()
            .filter(|w| w.start_s <= t && t < w.end_s)
            .map(|w| w.motion_scale.clamp(0.0, 1.0))
            .fold(1.0, f32::min)
    }

    /// The active hit volumes at proper-time `t` seconds into the move.
    pub fn active_volumes_at(&self, t: f32) -> impl Iterator<Item = &HitVolume> {
        self.windows
            .iter()
            .filter(move |w| matches!(w.tag, WindowTag::Active) && w.start_s <= t && t < w.end_s)
            .flat_map(|w| w.volumes.iter())
    }

    /// Normalized phase (`0..=1`) at proper-time `t` — what presentation
    /// samples the bound clip by.
    pub fn phase_at(&self, t: f32) -> f32 {
        if self.duration_s <= 0.0 {
            return 1.0;
        }
        (t / self.duration_s).clamp(0.0, 1.0)
    }

    /// The charge fraction (`0..=1`) reached at proper-time `t`: how far the
    /// owner's clock advanced through the leading Startup (charge) window. A
    /// move with no Startup window is "fully charged" instantly. This is the
    /// smash-charge state — it lives on the move's clock (`MovePlayback.t`), not
    /// a parallel component (CM3).
    pub fn charge_fraction_at(&self, t: f32) -> f32 {
        let charge_end = self
            .windows
            .iter()
            .find(|w| matches!(w.tag, WindowTag::Startup))
            .map(|w| w.end_s)
            .unwrap_or(0.0);
        if charge_end <= 0.0 {
            return 1.0;
        }
        (t / charge_end).clamp(0.0, 1.0)
    }

    /// The damage/knockback scale a release at proper-time `t` applies (CM3):
    /// `1.0 → smash_charge_mult` interpolated by the charge fraction. Returns
    /// `1.0` exactly when `smash_charge_mult == 1.0` (parity: no charge scaling).
    pub fn charge_scale_at(&self, t: f32) -> f32 {
        if self.smash_charge_mult == 1.0 {
            return 1.0;
        }
        1.0 + self.charge_fraction_at(t) * (self.smash_charge_mult - 1.0)
    }

    /// CM4: may this move, at proper-time `t` with the given hit state, be
    /// canceled into a candidate answering to any of `names`? The caller
    /// supplies every name the candidate answers to — its verb (`"attack"`,
    /// `"special"`, `"ranged"`), its resolved move id, and its classes
    /// (`"any_attack"` for the attack family; `"jump"`/`"dash"` for the
    /// locomotion escapes) — and an authored `into` entry matches any of
    /// them. One namespace, no enum: content authors strings, the runtime
    /// answers membership. An empty `cancels` timeline (no `Cancelable`
    /// window) refuses everything — the pre-CM4 status quo, which is the
    /// parity pin.
    pub fn cancel_permits(&self, t: f32, landed_hit: bool, names: &[&str]) -> bool {
        self.windows.iter().any(|w| match &w.tag {
            WindowTag::Cancelable { into, condition } => {
                w.start_s <= t
                    && t < w.end_s
                    && match condition {
                        CancelCondition::Always => true,
                        CancelCondition::OnHit => landed_hit,
                        CancelCondition::OnWhiff => !landed_hit,
                    }
                    && into.iter().any(|entry| names.contains(&entry.as_str()))
            }
            _ => false,
        })
    }

    /// Derive this move's frame data (CM7): the startup / active / recovery /
    /// cancel windows and the strike's reach, as a queryable table. A PURE
    /// derivation from `windows` + `duration_s` — no storage, no new state. The
    /// fighter brain reads it to time punishes and spacing; the boss validator
    /// reads it to assert telegraph/recovery budgets. Proper-time seconds
    /// throughout (the owner's clock), like every `MoveSpec` duration.
    pub fn frame_data(&self) -> MoveFrameData {
        let active_spans: Vec<(f32, f32)> = self
            .windows
            .iter()
            .filter(|w| matches!(w.tag, WindowTag::Active))
            .map(|w| (w.start_s, w.end_s))
            .collect();
        let cancel_windows: Vec<CancelWindow> = self
            .windows
            .iter()
            .filter_map(|w| match &w.tag {
                WindowTag::Cancelable { into, condition } => Some(CancelWindow {
                    start_s: w.start_s,
                    end_s: w.end_s,
                    into: into.clone(),
                    condition: *condition,
                }),
                _ => None,
            })
            .collect();
        // Startup = time until the first live hit; a move with no Active window
        // is pure recovery/utility, so its "startup" is its whole duration.
        let first_active = active_spans
            .iter()
            .map(|(s, _)| *s)
            .fold(f32::MAX, f32::min);
        let startup_s = if active_spans.is_empty() {
            self.duration_s
        } else {
            first_active
        };
        // Recovery = from the last Active edge to the move's end.
        let last_active_end = active_spans.iter().map(|(_, e)| *e).fold(0.0_f32, f32::max);
        let recovery_s = (self.duration_s - last_active_end).max(0.0);
        // Reach = the farthest body-local +x extent any Active volume reaches
        // (offset toward facing + the volume's half-width / radius). Zero when
        // the move lands no volume (a pure-motion or effect-only move).
        let reach = self
            .windows
            .iter()
            .filter(|w| matches!(w.tag, WindowTag::Active))
            .flat_map(|w| w.volumes.iter())
            .map(|v| match v.shape {
                VolumeShape::Rect {
                    offset,
                    half_extents,
                } => offset.0 + half_extents.0,
                VolumeShape::Circle { offset, radius } => offset.0 + radius,
            })
            .fold(0.0_f32, f32::max);
        // **WHERE THIS MOVE CAN HIT** — the union of every Active volume, in the
        // same body-local frame `reach` is measured in. See [`MoveCoverage`].
        let coverage = self
            .windows
            .iter()
            .filter(|w| matches!(w.tag, WindowTag::Active))
            .flat_map(|w| w.volumes.iter())
            .map(|v| match v.shape {
                VolumeShape::Rect {
                    offset,
                    half_extents,
                } => MoveCoverage {
                    min: (offset.0 - half_extents.0, offset.1 - half_extents.1),
                    max: (offset.0 + half_extents.0, offset.1 + half_extents.1),
                },
                VolumeShape::Circle { offset, radius } => MoveCoverage {
                    min: (offset.0 - radius, offset.1 - radius),
                    max: (offset.0 + radius, offset.1 + radius),
                },
            })
            .reduce(|a, b| MoveCoverage {
                min: (a.min.0.min(b.min.0), a.min.1.min(b.min.1)),
                max: (a.max.0.max(b.max.0), a.max.1.max(b.max.1)),
            });
        // Power = the strongest Active volume, derived exactly like `reach`.
        let max_damage = self
            .windows
            .iter()
            .filter(|w| matches!(w.tag, WindowTag::Active))
            .flat_map(|w| w.volumes.iter())
            .map(|v| v.damage)
            .max()
            .unwrap_or(0);
        let max_knockback = self
            .windows
            .iter()
            .filter(|w| matches!(w.tag, WindowTag::Active))
            .flat_map(|w| w.volumes.iter())
            .map(|v| v.knockback)
            .fold(0.0_f32, f32::max);
        // **LIFT: the against-gravity speed this move COMMANDS of its owner.**
        //
        // ⭐ the whole point of deriving it here is that a policy layer can then
        // recognise a recovery move by its GEOMETRY instead of by its name. `+y`
        // is gravity-down, so lift is `-y`, and it rotates with gravity for free
        // because it never leaves the body frame.
        //
        // ⛔ **only [`ImpulseMode::Set`] counts, and that is not a shortcut.** An
        // additive impulse commands nothing — its outcome is whatever the body
        // was already doing plus a number — so no static reader can say what
        // speed it produces. A `Set` states one. That distinction is exactly why
        // a jab with a small upward lunge cannot be mistaken here for a recovery
        // special.
        //
        // ⭐⭐ **AND THE SIDE COMPONENT COMES WITH IT.** The commanded velocity is
        // a VECTOR and this used to keep only its projection onto the
        // gravity axis, which is exactly the half a vertical Up-B happens to
        // consist of. A move that hauls its owner mostly SIDEWAYS — a grapple
        // line, a boarding charge, a slingshot — was reported as the small rise
        // left over after the useful half was discarded, and every reader of
        // this table then planned a route the body would never take. Both
        // halves are read off the SAME winning event, so `lift_side` is never
        // some other move's number.
        let (lift_speed, lift_at_s, lift_side) = self
            .events
            .iter()
            .filter_map(|ev| match &ev.kind {
                MoveEventKind::Impulse {
                    local,
                    mode: ImpulseMode::Set,
                } if local.1 < 0.0 => Some((-local.1, ev.at_s, local.0)),
                _ => None,
            })
            // Ties on speed break on the EARLIER moment, so the answer does not
            // depend on declaration order (ADR 0023).
            .fold((0.0_f32, 0.0_f32, 0.0_f32), |best, (speed, at, side)| {
                if speed > best.0 || (speed == best.0 && speed > 0.0 && at < best.1) {
                    (speed, at, side)
                } else {
                    best
                }
            });
        MoveFrameData {
            total_s: self.duration_s,
            startup_s,
            active_spans,
            recovery_s,
            cancel_windows,
            reach,
            // ⚠ **a derivation cannot answer this one.** A capture is recognised
            // by its effect KEY, which belongs to the ruleset that authors it,
            // not to this catalog — so the layer that builds a fighter's option
            // kit sets it and nothing here guesses.
            ignores_guard: false,
            coverage,
            max_damage,
            max_knockback,
            start_impulse: self.start_impulse.unwrap_or((0.0, 0.0)),
            lift_speed,
            lift_at_s,
            lift_side,
        }
    }
}

/// A move's cancel window (CM7): the proper-time span during which the move may
/// be canceled into the named move classes/ids, under [`CancelCondition`]
/// (CM4). Derived from a [`WindowTag::Cancelable`] window.
#[derive(Debug, Clone, PartialEq)]
pub struct CancelWindow {
    pub start_s: f32,
    pub end_s: f32,
    pub into: Vec<String>,
    pub condition: CancelCondition,
}

/// **The body-local box a move's Active volumes cover**, in the same frame the
/// volumes author themselves in: `+x` toward the owner's facing, `+y` toward its
/// feet (so an anti-air's box has a NEGATIVE `min.1`).
///
/// ⚠ a union, not a list. A move with three volumes is described by the region
/// they span, which is what a *"can this reach where they are"* question needs;
/// a consumer that wanted each volume separately would read the windows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MoveCoverage {
    pub min: (f32, f32),
    pub max: (f32, f32),
}

impl MoveCoverage {
    /// **HOW FAR THIS MOVE REACHES IN ONE DIRECTION** — the distance from the
    /// owner's origin to the far side of the box along `toward`, or `0.0` when
    /// the box does not lie that way at all.
    ///
    /// ⭐⭐ **this is [`MoveFrameData::reach`] generalised, and it collapses back
    /// to it exactly.** For a foe straight ahead of a forward volume the answer
    /// IS `reach`; for a foe overhead it is how far the move reaches UP, which is
    /// the number an anti-air is authored for and the number no scalar could
    /// carry. A move that covers nothing in the asked direction answers `0.0`,
    /// which is the honest *"this cannot touch them from here"*.
    ///
    /// `inflate` grows the box on every side — a hitbox catches a HURTBOX, so a
    /// caller passes the target's half-extent rather than pretending the target
    /// is a point.
    ///
    /// ⚠ a slab intersection from the ORIGIN, so a box that does not span the
    /// ray returns `0.0` rather than a nearest-approach consolation prize. Two
    /// moves that both fail to point at the opponent are equally useless, which
    /// is the same judgement `reach` made about a whiff.
    pub fn extent_toward(&self, toward: (f32, f32), inflate: (f32, f32)) -> f32 {
        let len = (toward.0 * toward.0 + toward.1 * toward.1).sqrt();
        if !(len > 0.0) {
            return 0.0;
        }
        let (dx, dy) = (toward.0 / len, toward.1 / len);
        let (lo_x, hi_x) = (self.min.0 - inflate.0, self.max.0 + inflate.0);
        let (lo_y, hi_y) = (self.min.1 - inflate.1, self.max.1 + inflate.1);
        // Per-axis entry/exit of the ray `t · d` through each slab. A zero
        // component means the ray never leaves that slab, so it either lies
        // inside it for all `t` or misses outright.
        let slab = |lo: f32, hi: f32, d: f32| -> Option<(f32, f32)> {
            if d.abs() < f32::EPSILON {
                return (lo <= 0.0 && 0.0 <= hi).then_some((f32::NEG_INFINITY, f32::INFINITY));
            }
            let (a, b) = (lo / d, hi / d);
            Some((a.min(b), a.max(b)))
        };
        let (Some((nx, fx)), Some((ny, fy))) = (slab(lo_x, hi_x, dx), slab(lo_y, hi_y, dy)) else {
            return 0.0;
        };
        let far = fx.min(fy);
        if nx.max(ny) > far || far <= 0.0 {
            return 0.0;
        }
        far
    }
}

/// The queryable frame data of a move (CM7) — the introspection the fighter
/// brain and boss validators consume. A pure derivation of [`MoveSpec::frame_data`]
/// (no storage). All times are the owner's proper-time seconds.
#[derive(Debug, Clone, PartialEq)]
pub struct MoveFrameData {
    /// Total move length.
    pub total_s: f32,
    /// Time until the first Active window opens — the tell the opponent reads.
    pub startup_s: f32,
    /// Every Active window's `(start, end)`, in declaration order.
    pub active_spans: Vec<(f32, f32)>,
    /// Time from the last Active window's end to the move's end — the punish
    /// window.
    pub recovery_s: f32,
    /// Cancel windows (`WindowTag::Cancelable`), for combo/chain reasoning.
    pub cancel_windows: Vec<CancelWindow>,
    /// Farthest body-local reach of any Active volume (`+x` toward facing).
    ///
    /// ⚠ **a PROJECTION of [`Self::coverage`], kept because it is what a lunge's
    /// effective range is measured against** (`start_impulse` adds travel along
    /// the same axis). ⛔ it is NOT the move's hittable region, and reading it as
    /// one is what made every vertical move in every kit look like a short poke —
    /// see [`MoveCoverage`].
    pub reach: f32,
    /// **A guard does not stop this move.**
    ///
    /// ⭐ derived by nobody and set by the caller that knows: a hit volume is
    /// blockable and a CAPTURE is not, and only the layer that recognises a
    /// capture effect can say which this is. Default `false`, so every ordinary
    /// move keeps the answer it always had.
    ///
    /// ⚠ genre-neutral on purpose. Unblockables, command grabs and armour
    /// breaks are the same fact to a planner: *the shield is not the answer to
    /// this one*.
    pub ignores_guard: bool,
    /// **The region this move can hit**, body-local, `None` when it lands no
    /// Active volume at all (a buff, a summon, a pure-motion recovery).
    ///
    /// ⭐⭐ **[`Self::reach`] is one face of this box, and a scorer that had only
    /// that face could not see the stage in two dimensions.** Measured
    /// 2026-08-15 in a CPU-versus-CPU match: an up-tilt whose volume sits ABOVE
    /// the shoulder projects onto `+x` as a ~30px poke, indistinguishable from a
    /// jab, so the option scorer priced the anti-air and the jab identically and
    /// then broke the tie on speed — every time, in every kit. George Booul
    /// authors sixteen moves and started five distinct ones per match; the whole
    /// vertical game (anti-air, juggle, spike) was never selected for the reason
    /// it exists, because nothing downstream knew the opponent was ABOVE.
    ///
    /// ⭐ the same lesson [`Self::lift_side`] records one field down: a 2-D
    /// authored shape summarised by a 1-D scalar describes a move that does not
    /// exist. This is the datum — the union of the authored volumes — not another
    /// summary of it.
    pub coverage: Option<MoveCoverage>,
    /// Highest `damage` any Active volume deals — the move's POWER, so an
    /// option scorer can price a smash above a jab (FB6a; §9 of
    /// fighter-brain.md recorded that nothing could). `0` for a move that
    /// lands no volume.
    pub max_damage: i32,
    /// Highest flat `knockback` any Active volume applies (the `knockback_growth`
    /// percent-scaling term is the victim's business, not the table's).
    pub max_knockback: f32,
    /// The move's authored self-motion at trigger, body-local (`+x` toward
    /// facing, `+y` per the authoring convention), `(0, 0)` when none. A
    /// lunge's EFFECTIVE range is `reach` plus the travel this buys — the
    /// fighter brain's shadow model learned that from its own fidelity
    /// instrument on day one (FB6e: a 51px-reach swing landed from 102px).
    pub start_impulse: (f32, f32),
    /// **The against-gravity speed this move COMMANDS**, from its strongest
    /// [`ImpulseMode::Set`] impulse. `0.0` for every move that does not lift its
    /// owner outright, which is almost all of them.
    ///
    /// ⭐ this is the semantic affordance a recovery policy reads. A move is a
    /// recovery because of what it DOES to the body, not because of what it is
    /// called — so a brain, an authoring validator and a recovery probe all
    /// recognise one from the same number, and no layer needs a table of which
    /// character's special is the Up-B.
    pub lift_speed: f32,
    /// When [`Self::lift_speed`] arrives, proper-time seconds from move start —
    /// the windup a body has to survive before the burst fires. `0.0` when there
    /// is no lift.
    pub lift_at_s: f32,
    /// **The along-facing half of the SAME commanded velocity**, body-local
    /// (`+x` toward facing, so a move that hauls its owner backwards is
    /// negative). `0.0` when there is no lift, and `0.0` for a purely vertical
    /// one.
    ///
    /// ⭐⭐ **this exists because [`Self::lift_speed`] is a PROJECTION, and a
    /// projection is only lossless for the shape it was derived from.** The
    /// first fighter to author a recovery authored a vertical one, so a scalar
    /// described it exactly; the second authored a grapple line that trades
    /// almost all of its energy for lateral distance, and the scalar described
    /// it as a small hop. Every consumer — the option scorer, the recovery
    /// probe, an authoring validator — was then reasoning about a move that
    /// does not exist.
    ///
    /// ⭐⭐ **AND THIS IS WHERE THE SCALARS STOP. There is no third one coming,
    /// and the reason is structural rather than a resolution.** `lift_side` and
    /// [`Self::lift_speed`] are not two summaries of a recovery: together they
    /// are the authored [`MoveEventKind::Impulse`]'s own `local` pair, copied
    /// into this table with its sign convention flipped on one axis. A 2-D
    /// impulse has exactly two components, so the pair is LOSSLESS for
    /// velocity-shaped self-motion — it is the datum, not a description of it —
    /// and a fourth number could only describe something a velocity is not.
    ///
    /// ⛔ **so if the next recovery does not fit, the answer is NOT another
    /// field here.** A teleport is the case that already does not fit: it
    /// commands a POSITION, not a velocity, and whether the destination is
    /// standable is a question about the world that no static table can hold.
    /// The honest response to that move is a new affordance it exposes for
    /// itself, and a probe that can spend a validated displacement — never a
    /// `lift_warp_x` beside these two.
    ///
    /// ⛔ **and this pair is not "the recovery affordance" even for the moves it
    /// does describe.** A commanded velocity is a STATIC property; whether
    /// throwing it from where the body is right now gets it home is a question
    /// about the current state, and the only thing that can answer it is the
    /// movement kernel. `RecoveryLens::best_route` uses these numbers to
    /// PROPOSE routes and lets the kernel dispose of them. Ranking moves by
    /// [`Self::lift_speed`] and calling the winner "the recovery" is the failure
    /// this pair exists inside of, not the thing it fixes.
    pub lift_side: f32,
}

// ---------------------------------------------------------------------------
// Entities: contract bundles.
// ---------------------------------------------------------------------------

/// Physics body contract: entity-local collision half-extents.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Body2dContract {
    pub half_extents: (f32, f32),
}

/// Presentation contract: which visual this entity binds to. `visual_id` is
/// the packer/sheet target name resolved through the sprite pack (or the
/// per-target sheet compatibility path).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PresentationContract {
    pub visual_id: String,
}

/// A discrete attack aim direction, reduced from the body-local input axis by
/// the caller (the engine-coordinate threshold stays in the runtime, where the
/// gravity/input frames live). Drives directional move selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttackDir {
    /// No directional aim — the plain neutral attack / jab.
    Neutral,
    /// Aimed in the body's facing direction (Smash forward tilt / forward air).
    Forward,
    Up,
    Down,
    /// Aimed away from facing (Smash "back air"). +x is facing, so the runtime
    /// maps `axis.x < 0` here.
    Back,
}

/// The verb-id fallback chain for a directional attack, most-specific first.
/// A moveset that authors only `base` still answers every direction; adding
/// `{base}_air_down` (a pogo down-air) is purely additive data — never a schema
/// fork (fable review AJ1: smash-style tilt/smash variants are MORE VERBS).
///
/// Examples (`base = "attack"`):
/// - aerial, `Forward`: `attack_air_forward` → `attack_forward` → `attack_air` → `attack`
/// - grounded, `Forward`: `attack_forward` → `attack`
/// - aerial, `Down`:   `attack_air_down` → `attack_down` → `attack_air` → `attack`
/// - grounded, `Down`: `attack_down` → `attack`
/// - grounded, `Neutral`: `attack`
/// **The RUNNING-stance verb for an attack base** — the one place the suffix is
/// spelled, so the runtime's verb vocabulary and the selector cannot disagree
/// about the word. Named for the genre's move ("dash attack"), keyed off the
/// body's gait and not off `AbilitySet::dash`.
pub fn dash_stance_verb(base: &str) -> String {
    format!("{base}_dash")
}

pub fn directional_verb_chain(base: &str, dir: AttackDir, grounded: bool) -> Vec<String> {
    let dir_suffix = match dir {
        AttackDir::Neutral => None,
        AttackDir::Forward => Some("forward"),
        AttackDir::Up => Some("up"),
        AttackDir::Down => Some("down"),
        AttackDir::Back => Some("back"),
    };
    let mut chain = Vec::with_capacity(4);
    if !grounded {
        if let Some(s) = dir_suffix {
            chain.push(format!("{base}_air_{s}"));
        }
    }
    if let Some(s) = dir_suffix {
        chain.push(format!("{base}_{s}"));
    }
    if !grounded {
        chain.push(format!("{base}_air"));
    }
    chain.push(base.to_string());
    chain
}

/// **Which reusable character template an actor instantiates.**
///
/// A character is an authored template, not a singleton person: `spawn Goblin`
/// three times and `spawn Fretjaw` twice are the same engine operation, one
/// definition and many runtime actors. This id names the definition; the actor's
/// runtime identity is its `SimId`, and the two are never the same question.
///
/// ⛔ **not a display name and not a sheet id.** Presentation is a projection of
/// a character, so which sprite a body wears must never be used to work out
/// which character it is. A newtype so that confusion cannot survive a
/// signature — the same reason `BrainPresetId` exists next door, and the
/// confusion this one prevents is the more expensive of the two.
///
/// Lives here, beside the placement schemas, because character identity is
/// content vocabulary that authoring, the character domain and the runtime all
/// need to name — and `#[serde(transparent)]` so authored data encodes exactly
/// as the bare string it always was.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CharacterId(pub String);

impl CharacterId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for CharacterId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for CharacterId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl std::fmt::Display for CharacterId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// ⭐ **so a `BTreeMap<CharacterId, _>` can still be looked up by `&str`.** The
/// registry key becomes honest without every caller having to mint an id to ask
/// a question — `Borrow` is the standard way a newtype key stays ergonomic, and
/// it holds the required invariant: `Ord`/`Eq`/`Hash` on `CharacterId` delegate
/// to the same `String`, so borrowed and owned comparisons cannot disagree.
impl std::borrow::Borrow<str> for CharacterId {
    fn borrow(&self) -> &str {
        &self.0
    }
}

/// The canonical verb id a body's basic melee swing binds to in its moveset.
///
/// ⭐ **the four verb ids live BESIDE the contract they are keys into**, not in
/// the runtime that plays it. A verb NAME is part of the moveset contract's
/// authoring vocabulary — content types one of these strings into a `verbs`
/// map — while `ambition_combat` is where a bound move is *executed*. They sat
/// in the runtime for historical reasons, and that placement was one of the
/// couplings keeping `CharacterDefinition` out of the character domain: a
/// definition cannot name the verb its moveset binds without reaching up into
/// the runtime crate. Re-exported from `ambition_combat::moveset`, so every
/// existing path still resolves.
///
/// See `docs/systems/actors-brains-and-character-content.md`; historical D73
/// rationale is archived under `docs/archive/planning-superseded/2026-08-13/`.
pub const ATTACK_VERB: &str = "attack";
/// Strong directional attacks use the same authored verb machinery under the
/// distinct `smash` base. A moveset that authors no smash verb falls back to its
/// ordinary attack repertoire.
pub const SMASH_VERB: &str = "smash";
/// The canonical verb id a body's ranged shot binds to in its moveset.
pub const RANGED_VERB: &str = "ranged";
/// The canonical verb id a body's signature special binds to in its moveset.
/// `special_pressed` resolves the facing-relative directional `special` chain;
/// a body only has a real special when its moveset authors a matching
/// directional verb or the base verb.
pub const SPECIAL_VERB: &str = "special";
/// The canonical verb id a body's taunt binds to, and the base of its
/// directional chain. Unlike every verb above it, this one is not a threat —
/// which is why a body needs no permission to carry it.
pub const TAUNT_VERB: &str = "taunt";
/// **The capture verbs.** The grab that establishes a hold, and the moves a
/// captor selects while one exists.
///
/// ⚠ **they sit beside [`SMASH_VERB`] because they are the same kind of thing
/// and it is worth being honest about what that kind is.** This crate holds the
/// verb NAMES a press can resolve to; content holds what each one DOES. `smash`
/// was already platform-fighter taxonomy living here, so a throw is not a new
/// concession — but it does make the pile of it bigger.
///
/// ⇒ **the restitch point is the first character-owned `smash.fighter` facet**
/// (queue D166). When a Smash capability owns its own schema, these move with
/// it and the generic catalog stops naming a throw. Until then one definition
/// here beats the same strings copied into a selector and an authoring module.
pub const GRAB_VERB: &str = "grab";
/// Neutral Attack inside a capture. Repeatable; the hold survives it.
pub const CAPTURE_PUMMEL_VERB: &str = "capture_pummel";
/// Forward Attack inside a capture. Ends the hold at its authored release.
pub const CAPTURE_THROW_FORWARD_VERB: &str = "capture_throw_forward";
/// Back Attack inside a capture.
pub const CAPTURE_THROW_BACK_VERB: &str = "capture_throw_back";
/// Up Attack inside a capture.
pub const CAPTURE_THROW_UP_VERB: &str = "capture_throw_up";
/// Down Attack inside a capture.
pub const CAPTURE_THROW_DOWN_VERB: &str = "capture_throw_down";

/// Moveset contract: the entity's moves plus which input verb activates
/// which move. `moves` is the composition surface — re-binding an existing
/// move onto a different actor is a data edit here.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MovesetContract {
    /// Input verb → move id (e.g. `"attack" → "sandbag_swat"`). BTreeMap for
    /// deterministic iteration (query-order discipline).
    #[serde(default)]
    pub verbs: BTreeMap<String, String>,
    pub moves: Vec<MoveSpec>,
}

impl MovesetContract {
    pub fn move_by_id(&self, id: &str) -> Option<&MoveSpec> {
        self.moves.iter().find(|m| m.id == id)
    }

    /// Resolve an input verb to its move.
    pub fn move_for_verb(&self, verb: &str) -> Option<&MoveSpec> {
        self.move_by_id(self.verbs.get(verb)?)
    }

    /// Resolve a directional attack to its move: the first verb in the
    /// most-specific → least-specific chain ([`directional_verb_chain`]) that is
    /// both authored AND whose gates permit the current grounded state (a
    /// grounded-only `attack_down` is skipped for an airborne body, falling
    /// through to `attack`). A moveset that authors only `base` answers every
    /// direction with the same move.
    /// **What an ATTACK press produces, stance included.** The dash attack is a
    /// STANCE and not a direction, so it is asked BEFORE the directional chain
    /// rather than added to [`AttackDir`] — a dashing body pressing forward and
    /// a standing one pressing forward want different moves, and `AttackDir` has
    /// no vocabulary for the difference.
    ///
    /// ⚠ **composes with [`Self::move_for_directional_verb`] rather than
    /// replacing it**, and that is not a wrapper to unpick later: every OTHER
    /// verb — special, smash, taunt — has no dash stance to ask about, and
    /// giving them one would be a question with a constant answer. A fighter
    /// that authors no `{base}_dash` resolves exactly what it did before.
    pub fn move_for_attack(
        &self,
        base: &str,
        dir: AttackDir,
        grounded: bool,
        running: bool,
    ) -> Option<&MoveSpec> {
        if grounded && running {
            if let Some(mv) = self
                .move_for_verb(&dash_stance_verb(base))
                .filter(|mv| mv.gates.permits(grounded))
            {
                return Some(mv);
            }
        }
        self.move_for_directional_verb(base, dir, grounded)
    }

    pub fn move_for_directional_verb(
        &self,
        base: &str,
        dir: AttackDir,
        grounded: bool,
    ) -> Option<&MoveSpec> {
        directional_verb_chain(base, dir, grounded)
            .into_iter()
            .find_map(|verb| {
                let mv = self.move_for_verb(&verb)?;
                mv.gates.permits(grounded).then_some(mv)
            })
    }
}

/// The contracts an entity exposes. All optional: the engine asks "does this
/// entity expose the contract this system consumes?", never "what category
/// is it?". Narrow seed set — grow per real consumer.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EntityContracts {
    #[serde(default)]
    pub body: Option<Body2dContract>,
    /// Simulation-authored damageable body shapes. Absent means the runtime may
    /// use its visible sprite-bounds compatibility fallback.
    #[serde(default)]
    pub hurtboxes: Option<HurtboxDoc>,
    #[serde(default)]
    pub presentation: Option<PresentationContract>,
    #[serde(default)]
    pub moveset: Option<MovesetContract>,
}

/// One catalog entity: a stable id plus its contract bundle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntityDef {
    pub id: String,
    pub contracts: EntityContracts,
}

/// An authored entity-catalog document (one or many entities).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntityCatalogDoc {
    pub schema_version: u32,
    pub entities: Vec<EntityDef>,
}

// ---------------------------------------------------------------------------
// Validation: headless, structural, exhaustive.
// ---------------------------------------------------------------------------

/// A structural problem in an authored catalog. Every violation is reported
/// (not just the first) so an author fixes a file in one pass.
#[derive(Debug, Clone, PartialEq)]
pub enum CatalogError {
    DuplicateEntityId {
        id: String,
    },
    DuplicateMoveId {
        entity: String,
        id: String,
    },
    /// A window lies outside `[0, duration_s]` or is inverted.
    ///
    /// A ZERO-WIDTH window (`start_s == end_s`) is legal: a move with no windup
    /// still authors its Startup phase, and the phase readers (`phase_at`, the
    /// synthesized read-model swing) want the window to exist so the timeline
    /// stays three-phase. `simple_melee` with `windup_s: 0.0` emits exactly that.
    /// Nothing fires inside one — every window predicate is the half-open
    /// `start_s <= t < end_s`, which is empty here — so it costs nothing.
    WindowOutOfRange {
        entity: String,
        mv: String,
        index: usize,
    },
    /// A non-Active window carries hit volumes (they would never fire).
    VolumesOnInactiveWindow {
        entity: String,
        mv: String,
        index: usize,
    },
    /// A `Cancelable { into }` edge names an undeclared move.
    UnknownCancelTarget {
        entity: String,
        mv: String,
        target: String,
    },
    /// A verb maps to an undeclared move.
    UnknownVerbMove {
        entity: String,
        verb: String,
        target: String,
    },
    /// An event fires outside the move's duration.
    EventOutOfRange {
        entity: String,
        mv: String,
        index: usize,
    },
    /// Non-positive move duration.
    NonPositiveDuration {
        entity: String,
        mv: String,
    },
    /// Degenerate volume (non-positive extent/radius).
    DegenerateVolume {
        entity: String,
        mv: String,
        window: usize,
    },
    /// An entity declares a moveset but no presentation clip could ever bind.
    /// (Warning-grade in spirit, but structural: an empty clip name is a typo.)
    EmptyClipBinding {
        entity: String,
        mv: String,
    },
    /// A body-state or move-clock hurtbox profile is structurally malformed.
    Hurtbox {
        entity: String,
        problem: HurtboxError,
    },
    /// A move-clock hurtbox override names no move on the same entity.
    UnknownHurtboxMove {
        entity: String,
        move_id: String,
    },
    /// A move-clock hurtbox keyframe cannot be reached before the move ends.
    HurtboxKeyframeOutOfMoveRange {
        entity: String,
        move_id: String,
        index: usize,
    },
}

impl std::fmt::Display for CatalogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CatalogError::DuplicateEntityId { id } => write!(f, "duplicate entity id `{id}`"),
            CatalogError::DuplicateMoveId { entity, id } => {
                write!(f, "{entity}: duplicate move id `{id}`")
            }
            CatalogError::WindowOutOfRange { entity, mv, index } => {
                write!(f, "{entity}/{mv}: window[{index}] outside [0, duration]")
            }
            CatalogError::VolumesOnInactiveWindow { entity, mv, index } => {
                write!(
                    f,
                    "{entity}/{mv}: window[{index}] carries volumes but is not Active"
                )
            }
            CatalogError::UnknownCancelTarget { entity, mv, target } => {
                write!(
                    f,
                    "{entity}/{mv}: cancel target `{target}` is not a declared move"
                )
            }
            CatalogError::UnknownVerbMove {
                entity,
                verb,
                target,
            } => {
                write!(
                    f,
                    "{entity}: verb `{verb}` maps to undeclared move `{target}`"
                )
            }
            CatalogError::EventOutOfRange { entity, mv, index } => {
                write!(
                    f,
                    "{entity}/{mv}: event[{index}] fires outside the move duration"
                )
            }
            CatalogError::NonPositiveDuration { entity, mv } => {
                write!(f, "{entity}/{mv}: non-positive duration")
            }
            CatalogError::DegenerateVolume { entity, mv, window } => {
                write!(f, "{entity}/{mv}: window[{window}] has a degenerate volume")
            }
            CatalogError::EmptyClipBinding { entity, mv } => {
                write!(f, "{entity}/{mv}: empty clip binding")
            }
            CatalogError::Hurtbox { entity, problem } => {
                write!(f, "{entity}: {problem}")
            }
            CatalogError::UnknownHurtboxMove { entity, move_id } => {
                write!(
                    f,
                    "{entity}: hurtbox override names undeclared move `{move_id}`"
                )
            }
            CatalogError::HurtboxKeyframeOutOfMoveRange {
                entity,
                move_id,
                index,
            } => write!(
                f,
                "{entity}/{move_id}: hurtbox keyframe[{index}] lies after the move duration"
            ),
        }
    }
}

impl EntityCatalogDoc {
    /// Parse a catalog document from RON text.
    pub fn parse(ron_text: &str) -> Result<Self, ron::error::SpannedError> {
        ron::from_str(ron_text)
    }

    /// Serialize to pretty RON (authoring round-trips).
    pub fn to_ron(&self) -> Result<String, ron::Error> {
        ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())
    }

    /// Structural validation. Empty ⇒ sound. Filesystem-free: clip bindings
    /// are checked for shape here; whether a clip resolves in the bound
    /// visual is the publish-time validator's job (it has the visual data).
    pub fn validate(&self) -> Vec<CatalogError> {
        let mut errors = Vec::new();
        let mut seen_entities = HashSet::new();
        for entity in &self.entities {
            if !seen_entities.insert(entity.id.as_str()) {
                errors.push(CatalogError::DuplicateEntityId {
                    id: entity.id.clone(),
                });
            }
            if let Some(hurtboxes) = entity.contracts.hurtboxes.as_ref() {
                errors.extend(hurtboxes.validate().into_iter().map(|problem| {
                    CatalogError::Hurtbox {
                        entity: entity.id.clone(),
                        problem,
                    }
                }));
            }
            let Some(moveset) = &entity.contracts.moveset else {
                if let Some(hurtboxes) = entity.contracts.hurtboxes.as_ref() {
                    errors.extend(hurtboxes.moves.keys().cloned().map(|move_id| {
                        CatalogError::UnknownHurtboxMove {
                            entity: entity.id.clone(),
                            move_id,
                        }
                    }));
                }
                continue;
            };
            let declared: HashSet<&str> = moveset.moves.iter().map(|m| m.id.as_str()).collect();
            if let Some(hurtboxes) = entity.contracts.hurtboxes.as_ref() {
                errors.extend(
                    hurtboxes
                        .moves
                        .keys()
                        .filter(|move_id| !declared.contains(move_id.as_str()))
                        .cloned()
                        .map(|move_id| CatalogError::UnknownHurtboxMove {
                            entity: entity.id.clone(),
                            move_id,
                        }),
                );
                for (move_id, timeline) in &hurtboxes.moves {
                    let Some(mv) = moveset.move_by_id(move_id) else {
                        continue;
                    };
                    errors.extend(
                        timeline
                            .keyframes
                            .iter()
                            .enumerate()
                            .filter(|(_, keyframe)| keyframe.at_s > mv.duration_s)
                            .map(|(index, _)| CatalogError::HurtboxKeyframeOutOfMoveRange {
                                entity: entity.id.clone(),
                                move_id: move_id.clone(),
                                index,
                            }),
                    );
                }
            }
            let mut seen_moves = HashSet::new();
            for mv in &moveset.moves {
                if !seen_moves.insert(mv.id.as_str()) {
                    errors.push(CatalogError::DuplicateMoveId {
                        entity: entity.id.clone(),
                        id: mv.id.clone(),
                    });
                }
                if mv.duration_s <= 0.0 {
                    errors.push(CatalogError::NonPositiveDuration {
                        entity: entity.id.clone(),
                        mv: mv.id.clone(),
                    });
                }
                if mv.clip.clip.is_empty() {
                    errors.push(CatalogError::EmptyClipBinding {
                        entity: entity.id.clone(),
                        mv: mv.id.clone(),
                    });
                }
                for (index, w) in mv.windows.iter().enumerate() {
                    if !(0.0..=mv.duration_s).contains(&w.start_s)
                        || !(0.0..=mv.duration_s).contains(&w.end_s)
                        || w.start_s > w.end_s
                    {
                        errors.push(CatalogError::WindowOutOfRange {
                            entity: entity.id.clone(),
                            mv: mv.id.clone(),
                            index,
                        });
                    }
                    if !w.volumes.is_empty() && !matches!(w.tag, WindowTag::Active) {
                        errors.push(CatalogError::VolumesOnInactiveWindow {
                            entity: entity.id.clone(),
                            mv: mv.id.clone(),
                            index,
                        });
                    }
                    for v in &w.volumes {
                        let degenerate = match v.shape {
                            VolumeShape::Rect { half_extents, .. } => {
                                half_extents.0 <= 0.0 || half_extents.1 <= 0.0
                            }
                            VolumeShape::Circle { radius, .. } => radius <= 0.0,
                        };
                        if degenerate {
                            errors.push(CatalogError::DegenerateVolume {
                                entity: entity.id.clone(),
                                mv: mv.id.clone(),
                                window: index,
                            });
                        }
                    }
                    if let WindowTag::Cancelable { into, .. } = &w.tag {
                        for target in into {
                            if !declared.contains(target.as_str())
                                && !CANCEL_CLASS_NAMES.contains(&target.as_str())
                            {
                                errors.push(CatalogError::UnknownCancelTarget {
                                    entity: entity.id.clone(),
                                    mv: mv.id.clone(),
                                    target: target.clone(),
                                });
                            }
                        }
                    }
                }
                for (index, ev) in mv.events.iter().enumerate() {
                    if !(0.0..=mv.duration_s).contains(&ev.at_s) {
                        errors.push(CatalogError::EventOutOfRange {
                            entity: entity.id.clone(),
                            mv: mv.id.clone(),
                            index,
                        });
                    }
                }
            }
            for (verb, target) in &moveset.verbs {
                if !declared.contains(target.as_str()) {
                    errors.push(CatalogError::UnknownVerbMove {
                        entity: entity.id.clone(),
                        verb: verb.clone(),
                        target: target.clone(),
                    });
                }
            }
        }
        errors
    }

    pub fn entity(&self, id: &str) -> Option<&EntityDef> {
        self.entities.iter().find(|e| e.id == id)
    }
}

#[cfg(test)]
mod tests;

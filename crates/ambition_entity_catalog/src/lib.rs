//! Entity-contract + moveset vocabulary — the gameplay-truth schema.
//!
//! Two rules carry the design:
//!
//! - One clock per move: the owner's proper time. Every duration in a
//!   [`MoveSpec`] is seconds of the *owning actor's* clock — its entity dt
//!   (sim dt × whatever dilation that actor experiences: bullet-time, a time
//!   bubble, a relativistic zone). The bound clip's playback is slaved to the
//!   move's normalized phase, so a dilated actor's picture and hit windows
//!   slow together and can never desync. Dilation is a property of the
//!   actor's clock, never of this data — the schema stays
//!   frame-of-reference-free.
//! - Entity-local logical space. Move volumes are authored in the
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

/// Opaque, structured parameters for a technique or prefab. The authored RON is byte-identical
/// to a `Reflect`-typed form, so if a visual move editor ever lands, swapping hydration to the
/// type registry is a mechanical migration — the data survives.
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
    /// [`hydrate`](Self::hydrate), for the case where code composes an effect that an author
    /// could equally have written by hand.
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

/// Content-defined technique/effect reference with opaque parameters. Timed
/// events, sustained windows, and on-hit payloads share this vocabulary; the
/// content-owned technique recognizes the key and hydrates its own params.
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

impl CancelCondition {
    /// Is this escape legal given whether the move has CONNECTED?
    ///
    /// One place, because [`MoveSpec::cancel_permits`] and
    /// [`MoveSpec::cancel_successors`] both ask it, and a chain that answered
    /// it differently from the permission would nominate a successor the
    /// cancel then refused.
    pub fn permits(self, landed_hit: bool) -> bool {
        match self {
            Self::Always => true,
            Self::OnHit => landed_hit,
            Self::OnWhiff => !landed_hit,
        }
    }
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

/// A per-volume override of what a hit DOES to the body it lands on.
///
/// The two arms are the two ways a volume can decline the ordinary
/// launch-and-hurt reaction, and they are opposites: [`Self::Autolink`] pulls
/// the victim IN, [`Self::Windbox`] pushes it AWAY. Targeting, faction and
/// contact resolve identically for both and for an ordinary hit.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum VolumeReaction {
    /// AUTOLINK: this volume HOLDS its victim near the attacker instead of
    /// launching it away.
    ///
    /// The genre's multi-hit moves work because their intermediate pulses keep
    /// the victim inside the next hitbox and only the LAST one launches, so a
    /// move authors this on its intermediate volumes and leaves its final volume
    /// alone. ⛔ it is not a capture: no relationship, no hold clock, no escape,
    /// and the victim keeps every verb it has.
    Autolink(AutolinkVolume),
    /// WINDBOX: this volume PUSHES its victim and does nothing else — no damage,
    /// no hitstun, no shield.
    Windbox(WindboxVolume),
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
    /// `knockback + knockback_growth * victim.damage_taken() / victim.weight`.
    ///
    /// `None` = **this stage decides** — the ruleset's own growth scales this
    /// hit, which is what an unauthored row has always got.
    ///
    /// `Some(g)` = **exactly this**, including `Some(0.0)`: FIXED KNOCKBACK, a
    /// hit that launches the same at 0% and at 200%. That is the mechanic
    /// multi-hit moves are built on — a pulse whose carry stops working once
    /// the victim's percent grows is a combo that dissolves exactly when it
    /// matters — and it was unauthorable.
    ///
    /// ⛔ AN `f32` HERE MAKES ZERO MEAN BOTH THINGS: flat knockback to an author,
    /// and "unspecified, use the stage's" to
    /// `ambition_platformer2d::combat::hitbox::resolved_hitbox_knockback_magnitude`,
    /// which substitutes — so the documented behaviour is the one you cannot get.
    /// One value with two meanings is the shape
    /// this repository keeps paying for; an `Option` is what tells "the author
    /// said zero" from "the author said nothing".
    #[serde(default)]
    pub knockback_growth: Option<f32>,
    /// Body-local launch direction override `(+x = facing, +y = gravity-down)`. `None` =
    /// today's facing+contact derivation. The runtime mirrors x by facing and rotates into the
    /// owner's gravity frame (frame-correct under any gravity), then applies DI (CM2).
    #[serde(default)]
    pub launch_dir: Option<(f32, f32)>,
    /// How this volume's REACTION differs from an ordinary hit. `None` is an
    /// ordinary hit, which is nearly every volume in the game.
    ///
    /// ⛔⛔ A SUM, NOT TWO `Option`s, and that is the whole point. It was
    /// `autolink: Option<_>` beside `windbox: Option<_>`, documented as
    /// *"mutually exclusive in meaning, not in type"* — so a volume asking to
    /// HOLD and to SHOVE at once was writable, and the runtime picked one and
    /// said so in a comment. A comment is not a schema. Fixed while it was still
    /// latent: no authored volume has ever set both.
    #[serde(default)]
    pub reaction: Option<VolumeReaction>,
    /// Optional technique fired when this volume lands, with owner/victim/contact
    /// context. `None` is an ordinary damage-only volume.
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

impl HitVolume {
    /// The autolink this volume authors, if its reaction is one.
    pub fn autolink(&self) -> Option<AutolinkVolume> {
        match self.reaction {
            Some(VolumeReaction::Autolink(link)) => Some(link),
            _ => None,
        }
    }

    /// The windbox this volume authors, if its reaction is one.
    pub fn windbox(&self) -> Option<WindboxVolume> {
        match self.reaction {
            Some(VolumeReaction::Windbox(wind)) => Some(wind),
            _ => None,
        }
    }
}

/// A WINDBOX: this volume MOVES its victim without hurting it.
///
/// ⭐⭐ IT CARRIES NO PUSH OF ITS OWN, and that is the design. A volume already
/// says where and how hard it throws — `knockback` and `launch_dir` — and a
/// gust is thrown the same way a punch is. Adding a second push vector here
/// would be a second way to author one thing, and the two would disagree the
/// first time somebody set both.
///
/// ⭐ HALF OF THIS MECHANIC ALREADY EXISTED. `damage: 0` is authorable and the
/// damage floor deliberately keeps it at zero (see `damage_floor` — *"a volume
/// that authors NO damage is a WINDBOX, and flooring it to one turned a push
/// into a hit"*). What a damageless volume still did was STUN its victim and
/// spend its hit-once slot, so it was a punch that dealt nothing rather than a
/// gust. This field is the remaining difference and nothing more.
///
/// ⛔ IT IS STILL AN ORDINARY VOLUME EVERYWHERE ELSE. Targeting, faction and
/// contact resolve exactly as they do for a hit, because "who is standing in
/// this" is the same question whether the answer hurts or not.
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct WindboxVolume {
    /// May this volume move the SAME body again while it stands in the gust?
    ///
    /// ⭐ A GUST PUSHES FOR AS LONG AS YOU ARE IN IT, which an ordinary strike
    /// must not: the hit-once set exists so a long active window cannot re-hit a
    /// stationary target every frame. `true` opts out of that set — correct for
    /// a sustained wind, wrong for a one-shot shove, so it is authored rather
    /// than assumed.
    #[serde(default)]
    pub repeating: bool,
}

/// The authored half of an autolink pulse: where it holds, how hard, and how
/// much of the attacker's own motion the victim inherits.
///
/// ⚠ THE ATTACKER'S VELOCITY IS NOT AUTHORED — the runtime samples it at the
/// pulse, because it is a fact about the moment and not about the move.
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct AutolinkVolume {
    /// Follow point in the ATTACKER'S local frame: `x` forward along its facing,
    /// `y` toward its feet — the same local convention `launch_dir` uses.
    pub anchor: (f32, f32),
    /// Share of the attacker's own velocity handed to the victim, `0..=1`.
    /// A rising move needs this: the correction below only closes a gap, and a
    /// fighter climbing fast outruns any gap-closing term.
    #[serde(default = "one")]
    pub carry: f32,
    /// Spring gain on the remaining gap, in 1/s. How HARD this move grabs.
    #[serde(default = "default_autolink_pull")]
    pub pull: f32,
    /// Ceiling on the corrective term, engine units/s. The carry is not clamped.
    #[serde(default = "default_autolink_max_speed")]
    pub max_speed: f32,
}

fn one() -> f32 {
    1.0
}

/// A 30 px gap asks for 600 px/s — firm enough to hold through a spin, gentle
/// enough that a victim at arm's length is not snapped.
fn default_autolink_pull() -> f32 {
    20.0
}

/// Above a fast fighter's own run, below anything that would read as a yank.
fn default_autolink_max_speed() -> f32 {
    900.0
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
    Vfx {
        effect: String,
        /// WHERE, body-local — `+x` toward the facing the move committed to, `+y` gravity-down
        /// — the same convention [`Impulse`](Self::Impulse) and every [`HitVolume`] offset use,
        /// and mirrored and rotated by the same two steps.
        ///
        ///  so an effect can sit on the box that throws it. A move authors
        /// its strike volume's offset and its burst's offset in the same numbers.
        #[serde(default)]
        at: (f32, f32),
        /// HOW BIG, as a multiple of the presentation's default effect size.
        /// `1.0` is that default; a flourish asks for less and a screen-filling
        /// super asks for more.
        #[serde(default = "default_vfx_scale")]
        scale: f32,
        /// Optional sound override. `None` resolves the cue addressed by the VFX
        /// row itself; use `Some` only when presentation should sound different.
        #[serde(default)]
        sfx: Option<String>,
    },
    /// Emit a content-defined effect (the `Effect` vocabulary / technique seam
    /// resolves it), carrying its opaque params.
    Effect(EffectRef),
    /// Fire the owner's ranged action using live aim/facing at this frame. The
    /// move supplies timing; the dispatcher's `ActionSet.ranged` supplies the shot.
    Ranged,
    /// Timed authored self-displacement in body-local axes (`+x = facing`,
    /// `+y = gravity-down`). Unlike `MoveSpec::start_impulse`, this fires at the
    /// timeline event and may set rather than add velocity, which is required for
    /// recovery moves whose burst follows startup.
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

/// What a move does to its owner's once-per-airtime recovery.
///
/// ⭐⭐ THREE STATES BECAUSE THE GENRE HAS THREE, and they were two booleans
/// until 2026-08-26 — `spends_recovery` and `recovery_without_freefall`, the
/// second documented as *ignored* unless the first was set. That is an invalid
/// combination spelled in valid data: `(false, true)` is representable, means
/// nothing, and every reader had to correlate two fields to find out which of
/// three things an author meant.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecoveryUse {
    /// Not a recovery: costs nothing and is refused by nothing.
    #[default]
    None,
    /// THE GENRE'S ORDINARY UP-B. Spends the airtime's one recovery and leaves
    /// its owner helpless once the move ends — the trade is your ability to act
    /// for height, which is what makes freefall its price rather than a penalty.
    SpendAndFreefall,
    /// Spends the airtime's one recovery and leaves its owner able to act.
    ///
    /// ⭐ THE DIFFERENCE IS WHAT THE RECOVERY BUYS. A recovery that hands you a
    /// VEHICLE has already given the height and the control together, so the
    /// price is the once-per-airtime budget alone — the pirate is aboard a shark
    /// and can still swing from the saddle, and it would be incoherent for the
    /// same move to also say it cannot act.
    ///
    /// ⛔ NOT "this move is free". The charge is still spent and still refreshes
    /// only on a re-seating cause or a flinching hit, so its owner gets ONE of
    /// these per airtime exactly like everybody else. What it declines is the
    /// helpless EPISODE, not the budget — see
    /// `BodyJumpState::post_recovery_helpless`, whose whole point is that those
    /// are different things.
    SpendWithoutFreefall,
}

impl RecoveryUse {
    /// Does starting this move cost a recovery charge?
    pub const fn spends(self) -> bool {
        !matches!(self, Self::None)
    }

    /// Does spending the LAST charge on this move arm the helpless episode?
    pub const fn arms_freefall(self) -> bool {
        matches!(self, Self::SpendAndFreefall)
    }
}

/// WHAT KIND OF WAY HOME THIS MOVE OFFERS, as its author states it.
///
/// ⭐⭐ THE GENRE HAS MORE THAN ONE, AND THE PLANNER KNEW ONE. Recovery
/// reasoning modelled every route as a `RecoveryLift`: one commanded velocity,
/// thrown once. That is the genre's ordinary up-B and it is not the only shape —
/// a fighter can teleport, and the pirate's up-B summons a steerable flying
/// shark and hands its rider SECONDS OF MOVEMENT AUTHORITY. Neither is a burst,
/// so `lift_speed` reads `0.0` for both and the CPU saw no way home at all
/// (D250).
///
/// ⛔ AND THE ANSWER IS NOT TO FAKE A LIFT. A fabricated impulse would make the
/// planner certify a rise the move does not throw, and the search would then be
/// wrong in the confident direction. ⛔ Nor a name check: what makes a move a
/// recovery is what it DOES, which is the rule `RecoveryUse` already states one
/// field up.
///
/// `None` here means *"whatever this move's frame data implies"*, which is a
/// burst when it commands one and nothing when it does not — every move authored
/// before route kinds existed still means what it meant.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum AuthoredRecoveryRoute {
    /// SECONDS OF MOVEMENT AUTHORITY, from something the move summons or mounts.
    ///
    /// ⛔ `reach` IS AUTHORED, and it is the claim the planner spends: *"this
    /// gets you home from within this far"*. Deriving it would mean reading the
    /// summoned body's own locomotion out of a registry the planner cannot see,
    /// and guessing it would make the search confidently wrong.
    SustainedAuthority { seconds: f32, reach: f32 },
    /// A DISCONTINUITY: the body is somewhere else, up to `distance` away.
    Teleport { distance: f32 },
}

/// The way home a move offers, RESOLVED — the authored statement above folded
/// with what the move's own frame data implies. See
/// [`MoveSpec::frame_data`], which is the only place the fold happens.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub enum RecoveryRoute {
    /// This move is no way home.
    #[default]
    None,
    /// ONE COMMANDED VELOCITY, thrown `at_s` into the move. The genre's ordinary
    /// up-B, and what every route was before there were kinds.
    Burst { speed: f32, side: f32, at_s: f32 },
    /// See [`AuthoredRecoveryRoute::SustainedAuthority`].
    SustainedAuthority { seconds: f32, reach: f32 },
    /// See [`AuthoredRecoveryRoute::Teleport`].
    Teleport { distance: f32 },
}

impl RecoveryRoute {
    pub fn offers_a_way_home(self) -> bool {
        !matches!(self, Self::None)
    }

    /// How far toward home this route CARRIES the body before it is an ordinary
    /// falling body again. `0.0` for a burst, whose whole effect is a velocity
    /// the kernel already simulates.
    pub fn carry(self) -> f32 {
        match self {
            Self::SustainedAuthority { reach, .. } => reach,
            Self::Teleport { distance } => distance,
            Self::None | Self::Burst { .. } => 0.0,
        }
    }
}

/// Activation gates for a move. Narrow on purpose — add knobs when real
/// moves need them.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MoveGates {
    /// `Some(true)` = grounded only; `Some(false)` = airborne only;
    /// `None` = either.
    #[serde(default)]
    pub grounded: Option<bool>,
    /// WHAT THIS MOVE COSTS THE AIRTIME'S ONE RECOVERY, and what it costs its
    /// owner afterwards.
    ///
    /// ⛔⛔ WITHOUT A RECOVERY BUDGET A PLATFORM FIGHTER HAS NO BOTTOM
    /// BLASTZONE. Nothing else limits a repeated special — there is no cooldown,
    /// no cost and no per-airtime rule on a move — and `grounded` cannot tell the
    /// second use in one airtime from the first. A fighter that authored a rising
    /// special could press it forever and only die to a launch that outran it.
    ///
    /// ⭐ AUTHORED, not inferred from a name or an impulse. What makes a move a
    /// recovery is that somebody said it is one — an up-special that does not
    /// lift, or a side-special that does, are both ordinary statements this way
    /// and neither is a special case in input code. ⚠ for a SMASH fighter the
    /// somebody is the repertoire slot rather than the moveset: see
    /// `SmashRepertoire`'s `UpSpecial`, which applies the genre's default so that
    /// fourteen authors do not each have to remember it.
    ///
    /// `#[serde(default)]` on a `Default`-`None` enum, so authored content from
    /// before this existed still means what it meant.
    #[serde(default)]
    pub recovery: RecoveryUse,
    /// This move REFUSES TO START while something else owns the body's pose —
    /// a saddle, a lift, a grab.
    ///
    /// ⛔⛔ IT IS A START CONDITION, NOT A LATE VETO, AND THAT DISTINCTION IS THE
    /// WHOLE REASON IT EXISTS. The pirate's shark up-B first enforced "no recast
    /// from the saddle" downstream, where the summon effect was translated: by
    /// then the move had been accepted, the recovery use spent, and the startup
    /// cues played, and all that happened was that no shark appeared. A mounted
    /// pirate who got flinched — which refunds the recovery — could press up-B
    /// and simply lose the use to nothing.
    ///
    /// ⭐ ONCE A MOVE STARTS, ITS AUTHORED EVENTS ARE OWED. Anything that can
    /// refuse the move has to say so before `start_move` spends what starting it
    /// costs; a rule enforced after acceptance is not a rule, it is a silent
    /// failure with a comment.
    ///
    /// ⚠ NOT A [`RecoveryUse`] ARM. What a move costs and whether it may begin
    /// are different questions: this one is asked of moves that are not
    /// recoveries at all, and a recovery can be perfectly castable from a
    /// saddle if its author says so.
    #[serde(default)]
    pub forbidden_while_held: bool,
    /// While this move plays, its owner has NO STEERING AUTHORITY: the
    /// controller's locomotion intent is zeroed and the body keeps only the
    /// motion the move itself gives it.
    ///
    /// ⭐ THE GENRE'S RULE FOR A GROUNDED ATTACK, and it is a fact about the
    /// STANCE rather than about any one move — which is why it is a gate beside
    /// `grounded` and not a per-window number every author would have to
    /// remember. In a platform fighter you cannot walk out of a jab, a tilt or a
    /// smash; a dash attack slides on its own impulse and steers no more than
    /// the rest. ⛔⛔ measured 2026-08-24: a human forward smash travelled 64
    /// world px — more than a body width — accelerating to the full run cap
    /// through its own startup, because nothing said this.
    ///
    /// Default FALSE, because the engine hosts more than fighters: an
    /// action-adventure protagonist that keeps walking through a slash is a
    /// legitimate feel, and a blanket engine rule would take it away.
    #[serde(default)]
    pub roots_steering: bool,
    /// The way home this move offers, when its frame data cannot say. See
    /// [`AuthoredRecoveryRoute`]; `None` leaves the answer to the frame data,
    /// which is where every burst still comes from.
    #[serde(default)]
    pub recovery_route: Option<AuthoredRecoveryRoute>,
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
/// the ability on one timeline. The move timeline is authoritative for both
/// gameplay and presentation — windows advance on the owner's proper time
/// and the bound clip is sampled by normalized move phase.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoveSpec {
    /// Stable move id (`"jab"`, `"tilt_up"`, `"sandbag_swat"`).
    pub id: String,
    /// Optional player-facing label. [`MoveSpec::display`] title-cases
    /// [`Self::id`] when absent. Shared move prefabs therefore share this label.
    #[serde(default)]
    pub display_name: Option<String>,
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
    /// How a chargeable use of this move holds and releases its charge.
    ///
    /// `None` = the derived policy: the hold sits a fraction into the leading
    /// Startup window and lasts [`SmashChargeSpec::DEFAULT_MAX_HOLD_S`].
    /// Authoring one is how a move differs — a slower windup that pays off
    /// sooner, or a charge that cannot be held at all.
    ///
    /// ⭐ AND AUTHORING ONE IS ALSO HOW A MOVE SAYS IT CHARGES AT ALL, which it
    /// did not used to be. The rule was *"only a move whose
    /// [`Self::smash_charge_mult`] pays for it"*, and that is exactly wrong for
    /// a charged SHOT: its payoff is the projectile it releases, and it lands
    /// no melee volume for a multiplier to scale. Either statement now counts —
    /// see [`Self::charge_policy`].
    ///
    /// WHICH press holds it is [`Self::charge_gesture`], and a use reached
    /// through any other verb is never chargeable.
    #[serde(default)]
    pub smash_charge: Option<SmashChargeSpec>,
    /// WHICH PRESS holds this move's charge.
    ///
    /// The charge mechanic was built for smash attacks and hardcoded to the
    /// smash gesture, which is right for every smash and wrong for the genre's
    /// other chargeable move: the held neutral special. Both freeze a timeline
    /// while a button is down and release when it comes up; they differ only in
    /// WHICH button, so the move says which rather than the runtime assuming.
    ///
    /// DEFAULT [`ChargeGesture::Smash`] — every move authored before this field
    /// existed, and every smash after it.
    #[serde(default)]
    pub charge_gesture: ChargeGesture,
    /// The stretch of this move's timeline that REPEATS while its button is
    /// held — the rapid jab, the drill, the flurry.
    ///
    /// `None` = a move that plays once, which is every move that has not opted
    /// in. The loop belongs to PLAYBACK and not to a fighter: a move says which
    /// of its own windows repeat and for how long, and the runtime does the
    /// same thing to all of them.
    #[serde(default)]
    pub repeat: Option<MoveLoop>,
    /// Landing lag: the recovery this move owes if the body touches down
    /// before the move ended. Seconds of the owner's proper time, spent as a
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
    /// Auto-cancel: land after this point in the move and pay NO landing
    /// lag. Seconds of proper time from the move's start.
    ///
    /// The other half of the commitment: a move thrown early enough that its
    /// dangerous part is over by touchdown lands clean. Authoring the pair is
    /// how a designer says "rise with this one, do not fall with it".
    ///
    /// `None` = no auto-cancel window; [`Self::landing_lag_s`] applies whenever
    /// the move is still running. Ignored if no landing lag is authored.
    #[serde(default)]
    pub autocancel_after_s: Option<f32>,
    /// How many times a second this move MIRRORS the body's drawn sprite while
    /// it plays — a spin, cheaply.
    ///
    /// ⭐ PRESENTATION ONLY. It flips the published pose's facing and touches
    /// nothing else: the body's own `facing` is unchanged, so its hitboxes, its
    /// launch directions and every rule that reads which way it is looking are
    /// exactly as they were. A spin that MOVED a hitbox would be a different
    /// move, not a different drawing of one.
    ///
    /// ⭐ AND IT IS A CRUDE ANSWER ON PURPOSE. Jon, W8 playtest, on Pointed's
    /// Up-B: *"it is acceptable to fake the spin by repeatedly flipping the
    /// sprite horizontally if that gives the basic rotational read... Do not
    /// spend a lot of time producing beautiful spin animation yet."* Real
    /// rotation is a rig problem; this is one number an author can put on a move
    /// today and take off when the art exists.
    ///
    /// `None` (and zero) = drawn the way every other move is.
    #[serde(default)]
    pub sprite_spin_hz: Option<f32>,
    /// The held item this move BRANDISHES while it plays.
    ///
    /// ⭐ THE GENRE'S DRAW-AND-SWING: a move whose whole read is "he pulls the
    /// gun-sword out and fires it" is one move, not an equip plus a shot the
    /// player has to sequence. The item is worn for exactly as long as the
    /// move's own clock runs and the body's authored item comes back after —
    /// so a fighter who carries nothing carries nothing again, and one who
    /// carries a sword gets its sword back.
    ///
    /// ⛔ IT IS NOT A PICKUP. Nothing enters or leaves an inventory, the item
    /// cannot be dropped or thrown, and a body that picked something up keeps
    /// it: the brandish REMEMBERS what it displaced and restores exactly that.
    ///
    /// `None` = every move that has not opted in, which is all of them but one.
    #[serde(default)]
    pub equips: Option<String>,
}

/// Serde default for [`MoveSpec::smash_charge_mult`]: the multiplicative
/// identity, so every existing move is unscaled (parity).
fn default_charge_mult() -> f32 {
    1.0
}

/// The stretch of a move that repeats while its button is held.
///
/// Authored in the move's own proper time, like every other clock on a
/// [`MoveSpec`]. The loop runs while the button stays down and ends on the
/// release or at [`Self::max_s`], whichever comes first; what the move authors
/// AFTER [`Self::to_s`] is the finisher the loop exits into, so a flurry that
/// ends in a launcher is one timeline rather than two moves.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MoveLoop {
    /// Where the clock jumps BACK to.
    pub from_s: f32,
    /// Where it jumps back FROM.
    pub to_s: f32,
    /// The longest the loop may run before it exits on its own, in seconds of
    /// looped time. A flurry nobody can end is a stall.
    pub max_s: f32,
}

impl MoveLoop {
    /// Is this loop authored coherently — a non-empty stretch with an end?
    pub fn is_live(&self) -> bool {
        self.to_s > self.from_s && self.max_s > 0.0
    }
}

/// How a chargeable move HOLDS: where on its own timeline the charge waits,
/// and how long it may wait before it fires itself.
///
/// Both values are seconds of the OWNER'S proper time, like every other clock
/// on a [`MoveSpec`] — a dilated fighter charges as slowly as it swings.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SmashChargeSpec {
    /// The instant the timeline freezes while Attack is held.
    pub hold_at_s: f32,
    /// The longest that freeze may last. Reaching it releases the move whether
    /// or not the button is still down — UNLESS [`Self::stores`], for which see
    /// there. That auto-release is what stops a held smash from being a stall.
    pub max_hold_s: f32,
    /// Does a charge SURVIVE being interrupted, and resume the next use?
    ///
    /// ⭐⭐ THE GENRE'S STORED SHOT. Jon, 2026-08-27, on the Projectile
    /// Polygon's neutral-B: *"This should have parity with samus / mewtwo 'b',
    /// so that means it needs to be able to store a charge and fire at different
    /// sizes."* Firing at different sizes was already there; STORING was not —
    /// the charge died with the move, so the only way to reach a full one was to
    /// stand still for the whole hold with nobody hitting you.
    ///
    /// `true` changes two things and nothing else:
    ///
    /// - reaching [`Self::max_hold_s`] no longer fires the move. A full charge
    ///   is LOADED and stays loaded until the button comes up or the move is
    ///   interrupted, which is what makes "charge it now, throw it later" a plan
    ///   rather than a race;
    /// - a use interrupted while still charging banks what it had, and the next
    ///   use of the SAME move resumes from there.
    ///
    /// ⛔ IT IS NOT A RESOURCE THE FIGHTER SPENDS. Firing consumes the charge
    /// because the shot IS the payoff; nothing else refunds it, and it does not
    /// leak between moves — the bank is keyed by move id, so a stored power ball
    /// cannot come out of a forward smash.
    ///
    /// DEFAULT `false` — every smash in the game, and byte-parity for each of
    /// them.
    #[serde(default)]
    pub stores: bool,
    /// Does the freeze ROOT the body?
    ///
    /// ⭐⭐ `true` FOR EVERY SMASH IN THE GAME, and Jon's rule is why: *"when
    /// the character is charging their smash attack, they should not be able to
    /// walk or move."* A windup you can stroll out of is not a commitment.
    ///
    /// ⛔⛔ AND IT IS A PROPERTY OF THE POLICY, NOT OF CHARGING. The Performer's
    /// trapdoor freezes its timeline through this exact mechanic — hold the
    /// button, hold the beat — and the beat it holds is TRAVEL: she is under
    /// the stage steering, and Jon asked for that in the same breath as the
    /// move itself (*"I do want the player to be able to control where they
    /// move"*). Rooting that freeze would delete the move. So the two uses of
    /// one mechanic say which they are instead of the runtime guessing from
    /// the gesture.
    ///
    /// DEFAULT `true`, which is byte-parity for every policy authored before
    /// this field existed.
    #[serde(default = "charge_roots_by_default")]
    pub roots: bool,
    /// WHAT KEEPS THE FREEZE — the button, or the move.
    #[serde(default)]
    pub sustain: ChargeSustain,
}

/// What holds a frozen timeline frozen.
///
/// ⭐⭐ ONE MECHANIC, TWO SHAPES, and the second one is not a smash. A charge is
/// *freeze the timeline, resume it later*; what differs is what decides
/// "later". A smash is paid for by KEEPING THE BUTTON DOWN — let go and it
/// swings — because the hold is the commitment you are being charged for.
///
/// ⛔ THE ACTOR'S TRAPDOOR IS THE OTHER ONE, and authoring it as a held charge
/// shipped a regression Jon caught in a day: *"The latest main the actor doesn't
/// spend any time under the stage."* He is not holding B while he steers, and
/// nobody would — the beat being held is a SECOND OF TRAVEL, which he asked for
/// as a duration (*"Give them 1 second under the stage"*) and then asked to be
/// able to cut short (*"she should be able to pop up at any time from it"*). A
/// hold-to-sustain reading of that gives a fighter three ticks under the boards
/// unless she keeps a finger down, which is the opposite of both sentences.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub enum ChargeSustain {
    /// The button stays DOWN, and releasing it resumes the move. Every smash in
    /// the game, and the default for a policy that says nothing.
    #[default]
    WhileHeld,
    /// The freeze holds ITSELF, up to the maximum, and a NEW press ends it.
    UntilPressedAgain,
}

/// Which press holds a move's charge.
///
/// A charge is one mechanic — freeze the timeline while a button is down,
/// release when it comes up — and the genre binds it to two different buttons.
/// The move says which; the runtime does the same thing to both.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub enum ChargeGesture {
    /// The SMASH gesture. Every smash attack, and the default for a move that
    /// says nothing.
    #[default]
    Smash,
    /// The SPECIAL press that started the move — the held neutral-B.
    Special,
}

impl SmashChargeSpec {
    /// A full charge takes one second — the platform-fighter house number
    /// (60 frames at 60Hz). A knob, not a measurement: a move that wants a
    /// different commitment authors its own policy.
    pub const DEFAULT_MAX_HOLD_S: f32 = 1.0;

    /// Is this policy capable of holding at all? A zero (or negative) maximum
    /// is how a move says "this smash does not charge".
    pub fn holds(&self) -> bool {
        self.max_hold_s > 0.0
    }

    /// The fraction of a full charge `held_s` of hold time buys, `0..=1`.
    pub fn fraction_for(&self, held_s: f32) -> f32 {
        if self.max_hold_s <= 0.0 {
            return 0.0;
        }
        (held_s / self.max_hold_s).clamp(0.0, 1.0)
    }
}

/// Serde default for [`SmashChargeSpec::roots`]: a charge roots its body, which
/// is what every policy authored before the field meant.
fn charge_roots_by_default() -> bool {
    true
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
    /// name the slot this move occupies: the authored [`Self::display_name`],
    /// else a title-cased `id` (`"sandbag_swat"` → `"Sandbag Swat"`).
    pub fn display(&self) -> String {
        self.display_name
            .clone()
            .unwrap_or_else(|| crate::action_scheme::title_case_id(&self.id))
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

    /// Does a window the predicate accepts COVER proper-time `t`?
    ///
    /// The timeline question every defensive window asks — an authored
    /// [`WindowTag::Invuln`] or [`WindowTag::Armor`] is in force for exactly the
    /// span it declares, on the owner's own clock, like every other window.
    pub fn tagged_window_covers(&self, t: f32, want: fn(&WindowTag) -> bool) -> bool {
        self.windows
            .iter()
            .any(|w| want(&w.tag) && w.start_s <= t && t < w.end_s)
    }

    /// The successors this move's live cancel window NAMES at proper-time `t`,
    /// in authored order.
    ///
    /// The chain is the cancel table read forwards: a `Cancelable` window that
    /// says `into: ["jab2"]` is not only permitting jab2, it is nominating it.
    /// A follow-up press inside that window takes the nomination instead of
    /// restarting the move that is playing, which is the whole of a jab chain
    /// and needs no successor field of its own.
    pub fn cancel_successors(&self, t: f32, landed_hit: bool) -> impl Iterator<Item = &str> {
        self.windows
            .iter()
            .filter(move |w| w.start_s <= t && t < w.end_s)
            .filter_map(move |w| match &w.tag {
                WindowTag::Cancelable { into, condition } => {
                    condition.permits(landed_hit).then_some(into)
                }
                _ => None,
            })
            .flatten()
            .map(String::as_str)
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

    /// Where a charge freezes when the move authors no policy of its own: a
    /// fraction into the leading Startup window, and never at or past the first
    /// Active instant.
    ///
    /// ⛔ THE CLAMP IS NOT DEFENSIVE, it is the invariant. A move may author a
    /// zero-width Startup, may lay Active before Startup ends, or may have no
    /// Startup at all; in each case a fraction of the windup is the wrong answer
    /// and the first live volume is the line that must not be crossed.
    fn derived_charge_hold_at_s(&self) -> f32 {
        let leading = self
            .windows
            .iter()
            .find(|w| matches!(w.tag, WindowTag::Startup));
        let Some(leading) = leading else {
            // No windup to hold inside. The move freezes at its own first
            // instant, which is what it did before any of this existed.
            return 0.0;
        };
        let pose = leading.start_s + (leading.end_s - leading.start_s) * CHARGE_POSE_FRACTION;
        let first_active = self
            .windows
            .iter()
            .filter(|w| matches!(w.tag, WindowTag::Active))
            .map(|w| w.start_s)
            .fold(f32::MAX, f32::min);
        if first_active == f32::MAX {
            return pose.max(0.0);
        }
        // Strictly before: a freeze ON the first Active instant is the defect.
        pose.clamp(0.0, (first_active - CHARGE_POSE_EPSILON_S).max(0.0))
    }

    /// The charge policy a SMASH-gesture use of this move plays under, or
    /// `None` when this move does not charge.
    ///
    /// The multiplier is what says a move charges: a smash with no payoff is a
    /// timeline that would freeze for nothing. The hold point is DERIVED from
    /// the timeline the move already authors rather than duplicated beside it,
    /// which is why every shipped fighter became chargeable without touching a
    /// single moveset.
    pub fn charge_policy(&self) -> Option<SmashChargeSpec> {
        // ⭐ EITHER PAYOFF SAYS THIS MOVE CHARGES, and the second one had to be
        // added for a move whose payoff is not a damage multiplier at all. A
        // charged SHOT pays in the projectile it releases — bigger, faster,
        // and worth more the longer it was held — and it lands no melee volume
        // for a multiplier to scale, so `smash_charge_mult` on it would be a
        // number that multiplies nothing.
        //
        // An explicit `smash_charge` is therefore its own statement of intent:
        // a move that authors a hold point and a maximum is a move that holds.
        // This is a strict widening — every previously-chargeable move still
        // charges — and the shipped roster is unmoved by it, because every
        // authored policy today sits beside a multiplier that already paid.
        let Some(policy) = self.smash_charge.or_else(|| {
            (self.smash_charge_mult > 1.0).then_some(SmashChargeSpec {
                // ⭐⭐ THE CHARGE POSE IS IN THE WINDUP, NOT AT THE HITBOX. Jon,
                // 2026-08-23: *"it needs to hold on the first frames of the smash
                // animation, before letting the rest of the animation, which
                // actually has the hitboxes, play."* That is what the genre does:
                // a charged smash freezes in its windup and releases into the swing.
                //
                // ⛔⛔ THIS DERIVED FROM THE STARTUP WINDOW'S `end_s`, AND ACTIVE
                // MEMBERSHIP IS `start_s <= t < end_s`. Ordinary smash authoring
                // lays Active directly against Startup, so the freeze landed on the
                // FIRST ACTIVE INSTANT — a fighter holding a charge with a live
                // strike volume already spawned. `rooted_by_charge` is true there,
                // so the hold was legal, indefinite, and armed.
                //
                // ⇒ the hold sits a fraction into the leading windup, strictly
                // before the first Active window. The rest of the windup plays on
                // release, which is the beat that makes a charge readable.
                hold_at_s: self.derived_charge_hold_at_s(),
                max_hold_s: SmashChargeSpec::DEFAULT_MAX_HOLD_S,
                // ⛔ A DERIVED POLICY NEVER STORES. This arm exists for the
                // smashes, whose charge is a commitment inside one swing; a
                // smash you could bank and throw later is a different mechanic
                // and would arrive here by accident rather than by authoring.
                stores: false,
                roots: true,
                sustain: ChargeSustain::WhileHeld,
            })
        }) else {
            return None;
        };
        policy.holds().then_some(policy)
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
                    && condition.permits(landed_hit)
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
        // LIFT: the against-gravity speed this move COMMANDS of its owner.
        //
        //  the whole point of deriving it here is that a policy layer can then
        // recognise a recovery move by its GEOMETRY instead of by its name. `+y`
        // is gravity-down, so lift is `-y`, and it rotates with gravity for free
        // because it never leaves the body frame.
        //
        //  only [`ImpulseMode::Set`] counts, and that is not a shortcut. An
        // additive impulse commands nothing — its outcome is whatever the body
        // was already doing plus a number — so no static reader can say what
        // speed it produces. A `Set` states one. That distinction is exactly why
        // a jab with a small upward lunge cannot be mistaken here for a recovery
        // special.
        //
        // A move that hauls its owner mostly SIDEWAYS — a grapple line, a boarding charge, a
        // slingshot — was reported as the small rise left over after the useful half was discarded,
        // and every reader of this table then planned a route the body would never take. Both
        // halves are read off the SAME winning event, so `lift_side` is never some other move's
        // number.
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
            charge_hold_at_s: self.charge_policy().map(|policy| policy.hold_at_s),
            total_s: self.duration_s,
            startup_s,
            active_spans,
            recovery_s,
            cancel_windows,
            reach,
            //  a derivation cannot answer this one. A capture is recognised
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
            // ⭐⭐ THE ONE PLACE THE FOLD HAPPENS. An author who stated a route
            // kind gets it; everybody else gets what their frame data implies,
            // which is a burst when the move commands a rise against gravity and
            // nothing when it does not. That second arm IS the rule the planner
            // has always used (`lift_speed > 0.0`), moved here so no consumer
            // has to spell it a second time.
            recovery_route: match self.gates.recovery_route {
                Some(AuthoredRecoveryRoute::SustainedAuthority { seconds, reach }) => {
                    RecoveryRoute::SustainedAuthority { seconds, reach }
                }
                Some(AuthoredRecoveryRoute::Teleport { distance }) => {
                    RecoveryRoute::Teleport { distance }
                }
                None if lift_speed > 0.0 => RecoveryRoute::Burst {
                    speed: lift_speed,
                    side: lift_side,
                    at_s: lift_at_s,
                },
                None => RecoveryRoute::None,
            },
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

/// The body-local box a move's Active volumes cover, in the same frame the
/// volumes author themselves in: `+x` toward the owner's facing, `+y` toward its
/// feet (so an anti-air's box has a NEGATIVE `min.1`).
///
///  a union, not a list. A move with three volumes is described by the region
/// they span, which is what a *"can this reach where they are"* question needs;
/// a consumer that wanted each volume separately would read the windows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MoveCoverage {
    pub min: (f32, f32),
    pub max: (f32, f32),
}

impl MoveCoverage {
    /// HOW FAR THIS MOVE REACHES IN ONE DIRECTION — the distance from the
    /// owner's origin to the far side of the box along `toward`, or `0.0` when
    /// the box does not lie that way at all.
    ///
    ///  this is [`MoveFrameData::reach`] generalised, and it collapses back
    /// to it exactly. For a foe straight ahead of a forward volume the answer
    /// IS `reach`; for a foe overhead it is how far the move reaches UP, which is
    /// the number an anti-air is authored for and the number no scalar could
    /// carry. A move that covers nothing in the asked direction answers `0.0`,
    /// which is the honest *"this cannot touch them from here"*.
    ///
    /// `inflate` grows the box on every side — a hitbox catches a HURTBOX, so a
    /// caller passes the target's half-extent rather than pretending the target
    /// is a point.
    ///
    ///  a slab intersection from the ORIGIN, so a box that does not span the
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

/// How far into a smash's leading windup the charge pose sits, as a fraction of
/// that window.
///
/// ⭐ EARLY ON PURPOSE — a brief windup, then the freeze. The genre reads a
/// charge as "the swing started and stopped", which needs some windup to have
/// played; it does not read as "the swing is about to land and stopped", which
/// is what a hold at the end of the window looks like. Jon, 2026-08-23: *"it
/// needs to hold on the first frames of the smash animation."*
///
/// ⛔⛔ TRANSITIONAL, AND NOT THE AUTHORING CONTRACT. A charge pose is an
/// ANIMATION fact — where in this move's windup this fighter holds — so it
/// belongs on the move, as an explicit `smash_charge.hold_at_s` inside its
/// leading Startup. This global exists so the shipped roster charges at all
/// while that authoring is done, and every smash currently leans on it.
///
/// ⛔ DO NOT TUNE IT FROM AN EMERGENT CPU MATCH. Jon's reviewer, 2026-08-23:
/// *"Charge-pose location is an animation/move-authoring fact; George's
/// offstage/recovery trajectory is an emergent balance result."* Swept against
/// one matchup (George Booul vs the duelist, 3600 ticks), `0.25` left George
/// off the stage 394 ticks pressing his route home 0 times where `0.50` left
/// him out 169 ticks pressing it 5 times — which is a fact about that match,
/// not about where a swing should freeze. ⚠ AND I OVERSTATED THE FOLLOW-UP:
/// the decision-log guard (`the_decision_log`, `--features causal`) was run at
/// `0.50` only, so "his recovery still works at both" was never measured.
///
/// ⇒ THE SHIPPED ROSTER NO LONGER USES THIS. Both smash tables author an
/// explicit `hold_at_s` of four frames, and `fighter_moveset`'s own contract
/// test refuses a smash that derives its pose instead. What is left here is the
/// answer for a move that says nothing — a boss swing, a fixture, a table
/// written before charge existed — and for those a fraction of the windup is a
/// reasonable guess rather than a feel decision.
pub const CHARGE_POSE_FRACTION: f32 = 0.50;

/// The margin that keeps a derived charge pose STRICTLY before the first live
/// volume, for a move whose windup is so short that the fraction lands on it.
const CHARGE_POSE_EPSILON_S: f32 = 1.0 / 240.0;

/// The queryable frame data of a move (CM7) — the introspection the fighter
/// brain and boss validators consume. A pure derivation of [`MoveSpec::frame_data`]
/// (no storage). All times are the owner's proper-time seconds.
#[derive(Debug, Clone, PartialEq)]
pub struct MoveFrameData {
    /// Total move length.
    pub total_s: f32,
    /// Where on this move's own timeline a SMASH-gesture use freezes, or `None`
    /// when the move does not charge at all.
    ///
    /// ⛔ NOT `startup_s`, and the difference is the whole reason this exists. A
    /// reader that wants to know when a charge BEGINS was deriving it from when
    /// the move's first hit LANDS, which is only the same number while the hold
    /// point happens to sit at the end of the leading Startup window. Moving the
    /// charge pose earlier — which is what the genre does — silently made every
    /// such reader early or late.
    pub charge_hold_at_s: Option<f32>,
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
    pub reach: f32,
    /// A guard does not stop this move.
    ///
    ///  derived by nobody and set by the caller that knows: a hit volume is
    /// blockable and a CAPTURE is not, and only the layer that recognises a
    /// capture effect can say which this is. Default `false`, so every ordinary
    /// move keeps the answer it always had.
    ///
    ///  genre-neutral on purpose. Unblockables, command grabs and armour
    /// breaks are the same fact to a planner: *the shield is not the answer to
    /// this one*.
    pub ignores_guard: bool,
    /// The region this move can hit, body-local, `None` when it lands no
    /// Active volume at all (a buff, a summon, a pure-motion recovery).
    ///
    /// George Booul authors sixteen moves and started five distinct ones per match; the whole
    /// vertical game (anti-air, juggle, spike) was never selected for the reason it exists, because
    /// nothing downstream knew the opponent was ABOVE.
    ///
    ///  the same lesson [`Self::lift_side`] records one field down: a 2-D
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
    /// The move's authored self-motion at trigger, body-local (`+x` toward facing, `+y` per the
    /// authoring convention), `(0, 0)` when none.
    pub start_impulse: (f32, f32),
    /// The against-gravity speed this move COMMANDS, from its strongest
    /// [`ImpulseMode::Set`] impulse. `0.0` for every move that does not lift its
    /// owner outright, which is almost all of them.
    ///
    ///  this is the semantic affordance a recovery policy reads. A move is a
    /// recovery because of what it DOES to the body, not because of what it is
    /// called — so a brain, an authoring validator and a recovery probe all
    /// recognise one from the same number, and no layer needs a table of which
    /// character's special is the Up-B.
    pub lift_speed: f32,
    /// When [`Self::lift_speed`] arrives, proper-time seconds from move start —
    /// the windup a body has to survive before the burst fires. `0.0` when there
    /// is no lift.
    pub lift_at_s: f32,
    /// Along-facing component of the commanded lift velocity, body-local
    /// (`+x` toward facing). Together with [`Self::lift_speed`] this is the
    /// complete 2-D velocity-shaped recovery proposal.
    pub lift_side: f32,
    /// THE WAY HOME THIS MOVE OFFERS, resolved — a burst from the two fields
    /// above, or whatever its author stated instead.
    ///
    /// ⭐ ONE ANSWER, so a planner never has to correlate `lift_speed` against a
    /// gate to find out whether a move is a recovery at all. The fields above
    /// remain what a BURST is made of; this is which kind of route it is.
    pub recovery_route: RecoveryRoute,
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

/// **The RUNNING-stance verb for an attack base** — the one place the suffix is
/// spelled, so the runtime's verb vocabulary and the selector cannot disagree
/// about the word. Named for the genre's move ("dash attack"), keyed off the
/// body's gait and not off `AbilitySet::dash`.
pub fn dash_stance_verb(base: &str) -> String {
    format!("{base}_dash")
}

/// Verb-id fallback chain for a directional attack, most-specific first. A
/// moveset that authors only `base` still answers every direction; directional
/// and aerial verbs are additive entries in the same vocabulary.
///
/// Examples (`base = "attack"`):
/// - aerial, `Forward`: `attack_air_forward` → `attack_forward` → `attack_air` → `attack`
/// - grounded, `Forward`: `attack_forward` → `attack`
/// - aerial, `Down`:   `attack_air_down` → `attack_down` → `attack_air` → `attack`
/// - grounded, `Down`: `attack_down` → `attack`
/// - grounded, `Neutral`: `attack`
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

/// Which reusable character template an actor instantiates.
///
/// A character is an authored template, not a singleton person: `spawn Goblin`
/// three times and `spawn Fretjaw` twice are the same engine operation, one
/// definition and many runtime actors. This id names the definition; the actor's
/// runtime identity is its `SimId`, and the two are never the same question.
///
/// A newtype so that confusion cannot survive a signature — the same reason `BrainPresetId` exists
/// next door, and the confusion this one prevents is the more expensive of the two.
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

///  so a `BTreeMap<CharacterId, _>` can still be looked up by `&str`. The
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
///  the four verb ids live BESIDE the contract they are keys into, not in
/// the runtime that plays it. A verb NAME is part of the moveset contract's
/// authoring vocabulary — content types one of these strings into a `verbs`
/// map — while `ambition_combat` is where a bound move is *executed*. They sat
/// in the runtime for historical reasons, and that placement was one of the
/// couplings keeping `CharacterDefinition` out of the character domain: a
/// definition cannot name the verb its moveset binds without reaching up into
/// the runtime crate. Re-exported from `ambition_platformer2d::combat::moveset`, so every
/// existing path still resolves.
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
/// The capture verbs. The grab that establishes a hold, and the moves a
/// captor selects while one exists.
///
///  they sit beside [`SMASH_VERB`] because they are the same kind of thing
/// and it is worth being honest about what that kind is. This crate holds the
/// verb NAMES a press can resolve to; content holds what each one DOES. `smash`
/// was already platform-fighter taxonomy living here, so a throw is not a new
/// concession — but it does make the pile of it bigger.
///
///  the restitch point is the first character-owned `smash.fighter` facet
/// . When a Smash capability owns its own schema, these move with
/// it and the generic catalog stops naming a throw. Until then one definition
/// here beats the same strings copied into a selector and an authoring module.
pub const GRAB_VERB: &str = "grab";
/// **The RUNNING grab.** The same reach-out performed out of a run, which in
/// this genre trades endlag for the range the run already carries — so unlike
/// the dash ATTACK (a different move entirely, which each fighter authors) this
/// one is DERIVED from the fighter's own standing grab. Spelled here because
/// [`MovesetContract::move_for_flat_verb`] needs a `&'static str`;
/// `a_running_grabs_verb_is_the_dash_stance_of_the_grab` pins it to
/// [`dash_stance_verb`] so the two spellings cannot drift.
pub const GRAB_DASH_VERB: &str = "grab_dash";
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
    /// Rename every move this table defines, and every reference to one.
    ///
    /// ⭐⭐ THE TRAVERSAL BELONGS TO THE SCHEMA, not to the caller that wants a
    /// borrowed fighter. A move id appears in THREE places inside a contract —
    /// `moves[].id`, `verbs`, and a `Cancelable` window's `into` list when it
    /// names a move rather than a verb class — and a caller that knew about two
    /// of them produced a table with one dead button. Every future field that
    /// carries a move id is this function's obligation, and the compiler shows it
    /// to whoever adds the field.
    ///
    /// ⛔ A VERB CLASS IS NOT A MOVE ID. `"attack"`, `"any_attack"` and the rest
    /// survive untouched: only a name this table itself defines is renamed with
    /// it, which is why the old→new map is built first.
    ///
    /// ⛔ AND `rename` IS NOT ASKED ABOUT ANYTHING BUT A MOVE THIS TABLE OWNS,
    /// so a caller may panic on an id it does not recognise without having to
    /// know which of the three places it came from.
    pub fn remap_move_ids(&mut self, rename: impl Fn(&str) -> String) {
        let by_old: BTreeMap<String, String> = self
            .moves
            .iter()
            .map(|mv| (mv.id.clone(), rename(&mv.id)))
            .collect();
        for mv in &mut self.moves {
            mv.id = by_old
                .get(mv.id.as_str())
                .cloned()
                .unwrap_or_else(|| rename(&mv.id));
            for window in &mut mv.windows {
                if let WindowTag::Cancelable { into, .. } = &mut window.tag {
                    for target in into.iter_mut() {
                        if let Some(new) = by_old.get(target.as_str()) {
                            *target = new.clone();
                        }
                    }
                }
            }
        }
        for target in self.verbs.values_mut() {
            *target = by_old
                .get(target.as_str())
                .cloned()
                .unwrap_or_else(|| rename(target));
        }
    }

    pub fn move_by_id(&self, id: &str) -> Option<&MoveSpec> {
        self.moves.iter().find(|m| m.id == id)
    }

    /// Resolve an input verb to its move, IGNORING its gates.
    ///
    /// This answers "does this fighter author the verb at all" — what a display
    /// row or an authorship check wants. A SELECTOR deciding what a press starts
    /// wants [`Self::move_for_verb_in_stance`], because an authored move whose
    /// gates refuse the body's stance is not a move that press may start.
    pub fn move_for_verb(&self, verb: &str) -> Option<&MoveSpec> {
        self.move_by_id(self.verbs.get(verb)?)
    }

    /// The move an input verb names, when the body's stance PERMITS it.
    ///
    /// The gated exact-verb lookup, and the one a selector should reach for. Its
    /// siblings [`Self::move_for_flat_verb`] and
    /// [`Self::move_for_directional_verb`] already refuse a move whose gates
    /// disagree with the stance; a bare `move_for_verb` in a selector is that
    /// same check written as "remembered to", and it was forgotten in seven
    /// places — every capture verb among them, which is how an AIRBORNE press
    /// started a grab the capture kit declares grounded-only.
    pub fn move_for_verb_in_stance(&self, verb: &str, grounded: bool) -> Option<&MoveSpec> {
        self.move_for_verb(verb)
            .filter(|mv| mv.gates.permits(grounded))
    }

    /// Resolve a directional attack to its move: the first verb in the
    /// most-specific → least-specific chain ([`directional_verb_chain`]) that is
    /// both authored AND whose gates permit the current grounded state (a
    /// grounded-only `attack_down` is skipped for an airborne body, falling
    /// through to `attack`). A moveset that authors only `base` answers every
    /// direction with the same move.
    /// What an ATTACK press produces, stance included. The dash attack is a
    /// STANCE and not a direction, so it is asked BEFORE the directional chain
    /// rather than added to [`AttackDir`] — a dashing body pressing forward and
    /// a standing one pressing forward want different moves, and `AttackDir` has
    /// no vocabulary for the difference.
    ///
    /// Composes with [`Self::move_for_directional_verb`] rather than replacing
    /// it: the remaining verbs — special, smash, taunt — have no dash stance to
    /// ask about, and giving them one would be a question with a constant
    /// answer. A fighter that authors no `{base}_dash` resolves exactly what it
    /// did before.
    ///
    /// GRAB is the other verb that does have one, and it goes through
    /// [`Self::move_for_flat_verb`] instead, because the capture kit is flat.
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

    /// **A FLAT verb's move, preferring its RUNNING-stance variant.** The
    /// sibling of [`Self::move_for_attack`] for a verb with no directional
    /// family — the capture kit, whose own doc is emphatic that a throw is not
    /// `grab_forward`.
    ///
    /// ⚠ **conditioned on the variant being BOUND**, exactly as the dash attack
    /// is: a contract without `{base}_dash` resolves its press to `base`, byte
    /// for byte, so this cannot change what an existing moveset does.
    pub fn move_for_flat_verb(
        &self,
        base: &str,
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
        // GATED, like the running variant above and like every candidate the
        // directional chain considers. A standing move whose gates refuse this
        // stance is not the answer to the press -- there simply is no answer.
        self.move_for_verb_in_stance(base, grounded)
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
    /// An AUTHORED smash charge freezes the timeline where a strike is already
    /// live, or outside the move's leading windup entirely.
    ///
    /// ⛔⛔ THE HOLD POINT IS NOT A FREE NUMBER. `rooted_by_charge` is true from
    /// the freeze onward and the button may hold it indefinitely, so a hold at
    /// or past the first Active instant is a fighter standing still with a live
    /// hitbox out — the strike volume spawns from the clock, and the clock has
    /// stopped inside the window. Active membership is `start_s <= t < end_s`,
    /// so "at" is already inside.
    ChargeHoldOutsideWindup {
        entity: String,
        mv: String,
        hold_at_s: f32,
        first_active_s: f32,
    },
    /// A WINDBOX volume authors damage, which its own contract forbids.
    ///
    /// ⛔⛔ THE CONTRACT LIVED IN A COMMENT AND IN EVERY FIXTURE'S GOOD MANNERS.
    /// `VolumeReaction::Windbox` says it *"pushes its victim and does nothing
    /// else — no damage, no hitstun, no shield"*, the runtime honours the last
    /// two, and `damage` was published normally: the type permitted
    /// `damage: 10` beside a windbox and every existing fixture merely
    /// remembered to write zero.
    ///
    /// ⭐ REJECTED, NOT SILENTLY ZEROED. Discarding a number somebody
    /// deliberately typed is how a content error becomes a mystery about why a
    /// move does nothing; saying which move and which volume is the whole value
    /// of catching it here. And it is caught NOW because no shipped move
    /// authors a windbox yet — the moment before content starts depending on
    /// the ambiguity is the only cheap one.
    WindboxWithDamage {
        entity: String,
        mv: String,
        window: usize,
        damage: i32,
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
            CatalogError::ChargeHoldOutsideWindup {
                entity,
                mv,
                hold_at_s,
                first_active_s,
            } => {
                write!(
                    f,
                    "{entity}/{mv}: the smash charge freezes at {hold_at_s}s but \
                     a strike goes live at {first_active_s}s — a held charge \
                     would stand inside it"
                )
            }
            CatalogError::WindboxWithDamage {
                entity,
                mv,
                window,
                damage,
            } => {
                write!(
                    f,
                    "{entity}/{mv}: window[{window}] authors a WINDBOX with \
                     damage {damage}. A windbox pushes its victim and does \
                     nothing else — author `damage: 0`, or use an ordinary hit \
                     volume if the contact is meant to hurt"
                )
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

    /// Structural validation. Empty  sound. Filesystem-free: clip bindings
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
                        if v.damage != 0 && v.windbox().is_some() {
                            errors.push(CatalogError::WindboxWithDamage {
                                entity: entity.id.clone(),
                                mv: mv.id.clone(),
                                window: index,
                                damage: v.damage,
                            });
                        }
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
                // ⛔ THE AUTHORED policy is what needs checking. The DERIVED one
                // is clamped strictly before the first Active instant by
                // `derived_charge_hold_at_s` and cannot land here; authoring
                // OVERRIDES that clamp, so this is what refuses a bad override
                // instead of letting it put a live hitbox inside a held charge.
                if let Some(policy) = mv.charge_policy().filter(|_| mv.smash_charge.is_some()) {
                    let first_active = mv
                        .windows
                        .iter()
                        .filter(|w| matches!(w.tag, WindowTag::Active))
                        .map(|w| w.start_s)
                        .fold(f32::MAX, f32::min);
                    if first_active < f32::MAX && policy.hold_at_s >= first_active {
                        errors.push(CatalogError::ChargeHoldOutsideWindup {
                            entity: entity.id.clone(),
                            mv: mv.id.clone(),
                            hold_at_s: policy.hold_at_s,
                            first_active_s: first_active,
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

//! Entity-contract + moveset vocabulary — the gameplay-truth schema.
//!
//! This crate is the typed spine of the `EntityCatalog` target in
//! `docs/planning/engine/data-driven-sprites-and-characters.md`: entities as
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
    /// The move may be canceled into the named moves.
    Cancelable { into: Vec<String> },
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
}

impl MoveSpec {
    /// The windows carrying `tag`, in declaration order.
    pub fn windows_tagged(
        &self,
        want: fn(&WindowTag) -> bool,
    ) -> impl Iterator<Item = &MoveWindow> {
        self.windows.iter().filter(move |w| want(&w.tag))
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
    /// No directional aim — the plain forward/neutral attack.
    Neutral,
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
/// - aerial, `Down`:   `attack_air_down` → `attack_down` → `attack_air` → `attack`
/// - grounded, `Down`: `attack_down` → `attack`
/// - grounded, `Neutral`: `attack`
pub fn directional_verb_chain(base: &str, dir: AttackDir, grounded: bool) -> Vec<String> {
    let dir_suffix = match dir {
        AttackDir::Neutral => None,
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
    /// A window lies outside `[0, duration_s]` or is inverted/empty.
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
            let Some(moveset) = &entity.contracts.moveset else {
                continue;
            };
            let declared: HashSet<&str> = moveset.moves.iter().map(|m| m.id.as_str()).collect();
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
                        || w.start_s >= w.end_s
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
                    if let WindowTag::Cancelable { into } = &w.tag {
                        for target in into {
                            if !declared.contains(target.as_str()) {
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
mod tests {
    use super::*;

    #[derive(serde::Deserialize)]
    struct GliderParams {
        #[allow(dead_code)]
        rise: f32,
    }

    #[test]
    fn param_schema_registry_catches_typos_at_validate_time() {
        // AJ1 / A1: a technique registers a hydrate check; the content pass
        // runs every authored EffectRef through it. A good ref passes; a
        // missing/mistyped field fails at validate time, not mid-fight.
        let mut reg = ParamSchemaRegistry::default();
        assert!(reg.is_empty());
        reg.register("glider", check_hydrates::<GliderParams>);

        let good = EffectRef {
            key: "glider".into(),
            params: ParamValue::parse("(rise: 320.0)").unwrap(),
        };
        assert!(reg.validate(&good).is_ok());

        // Wrong type for `rise` — fails, naming the offending key.
        let bad = EffectRef {
            key: "glider".into(),
            params: ParamValue::parse("(rise: \"fast\")").unwrap(),
        };
        let err = reg.validate(&bad).expect_err("bad params must fail");
        assert!(err.contains("glider"), "error names the effect key: {err}");

        // An unregistered key always passes — the engine matches no key.
        let unknown = EffectRef::new("some_content_const_technique");
        assert!(reg.validate(&unknown).is_ok());

        // Batch validation collects every failure at once.
        let errs = reg.validate_all([&good, &bad, &unknown]);
        assert_eq!(errs.len(), 1, "only the mistyped ref fails: {errs:?}");
    }

    /// The seed catalog: one actor-like entity (a moveset + body +
    /// presentation) and one prop-like entity (body + presentation only).
    /// The actor's `swat` is the SwipeSpec shape as data: three windows,
    /// the active one carrying one rect hit volume.
    const SEED: &str = r#"
    (
        schema_version: 1,
        entities: [
            (
                id: "sandbag_seed",
                contracts: (
                    body: Some((half_extents: (15.0, 24.0))),
                    presentation: Some((visual_id: "sandbag")),
                    moveset: Some((
                        verbs: { "attack": "swat" },
                        moves: [
                            (
                                id: "swat",
                                clip: (clip: "slash", fallbacks: ["idle"]),
                                duration_s: 0.68,
                                windows: [
                                    (start_s: 0.0, end_s: 0.28, tag: Startup, volumes: []),
                                    (start_s: 0.28, end_s: 0.36, tag: Active, volumes: [
                                        (shape: Rect(offset: (28.0, 0.0), half_extents: (14.0, 10.0)),
                                         damage: 1, knockback: 40.0),
                                    ]),
                                    (start_s: 0.36, end_s: 0.68, tag: Recovery, volumes: []),
                                    (start_s: 0.5, end_s: 0.68, tag: Cancelable(into: ["swat"]), volumes: []),
                                ],
                                events: [
                                    (at_s: 0.28, kind: Sfx(cue: "swing_light")),
                                ],
                                gates: (grounded: Some(true)),
                            ),
                        ],
                    )),
                ),
            ),
            (
                id: "crate_seed",
                contracts: (
                    body: Some((half_extents: (16.0, 16.0))),
                    presentation: Some((visual_id: "intro_cart")),
                ),
            ),
        ],
    )
    "#;

    #[test]
    fn seed_catalog_parses_and_validates() {
        let doc = EntityCatalogDoc::parse(SEED).unwrap();
        assert!(doc.validate().is_empty(), "{:?}", doc.validate());
        assert_eq!(doc.entities.len(), 2);
        let actor = doc.entity("sandbag_seed").unwrap();
        let moveset = actor.contracts.moveset.as_ref().unwrap();
        let swat = moveset.move_for_verb("attack").unwrap();
        assert_eq!(swat.id, "swat");
        // Prop exposes body+presentation, no moveset — contracts, not
        // categories: nothing marks it "a prop".
        let prop = doc.entity("crate_seed").unwrap();
        assert!(prop.contracts.moveset.is_none());
        assert!(prop.contracts.presentation.is_some());
    }

    /// A bare move (no windows) with the given id and grounded gate.
    fn bare_move(id: &str, grounded: Option<bool>) -> MoveSpec {
        MoveSpec {
            id: id.to_string(),
            clip: ClipBinding {
                clip: id.to_string(),
                fallbacks: vec![],
            },
            duration_s: 0.3,
            windows: vec![],
            events: vec![],
            gates: MoveGates { grounded },
            start_impulse: None,
        }
    }

    /// The full R2 ability vocabulary, authored entirely as RON: directional
    /// verbs, a move-start `start_impulse` lunge, and an `on_hit` pogo volume.
    /// The I7 acceptance — a fighter's whole kit is DATA, not code.
    const R2_FIGHTER: &str = r#"
    (
        schema_version: 1,
        entities: [(
            id: "data_fighter",
            contracts: (
                moveset: Some((
                    verbs: {
                        "attack": "jab",
                        "attack_air_down": "dair",
                    },
                    moves: [
                        (
                            id: "jab",
                            clip: (clip: "jab", fallbacks: ["idle"]),
                            duration_s: 0.30,
                            windows: [
                                (start_s: 0.04, end_s: 0.14, tag: Active, volumes: [
                                    (shape: Rect(offset: (28.0, 0.0), half_extents: (20.0, 14.0)),
                                     damage: 2, knockback: 120.0),
                                ]),
                            ],
                            start_impulse: Some((30.0, 0.0)),
                        ),
                        (
                            id: "dair",
                            clip: (clip: "dair", fallbacks: ["idle"]),
                            duration_s: 0.28,
                            gates: (grounded: Some(false)),
                            windows: [
                                (start_s: 0.03, end_s: 0.14, tag: Active, volumes: [
                                    (shape: Rect(offset: (0.0, 26.0), half_extents: (18.0, 18.0)),
                                     damage: 3, knockback: 0.0,
                                     on_hit: Some((key: "pogo_bounce"))),
                                ]),
                            ],
                        ),
                    ],
                )),
            ),
        )],
    )
    "#;

    #[test]
    fn the_full_r2_vocabulary_is_authorable_as_ron() {
        let doc = EntityCatalogDoc::parse(R2_FIGHTER).unwrap();
        assert!(doc.validate().is_empty(), "{:?}", doc.validate());
        let ms = doc
            .entity("data_fighter")
            .unwrap()
            .contracts
            .moveset
            .as_ref()
            .unwrap();
        // Directional resolution off authored verbs: aerial + down → the dair,
        // grounded neutral → the jab (the aerial-only dair is gate-skipped).
        let dair = ms
            .move_for_directional_verb("attack", AttackDir::Down, false)
            .unwrap();
        assert_eq!(dair.id, "dair");
        let jab = ms
            .move_for_directional_verb("attack", AttackDir::Down, true)
            .unwrap();
        assert_eq!(jab.id, "jab", "grounded skips the aerial-only dair");
        // The jab carries its authored move-start lunge.
        assert_eq!(jab.start_impulse, Some((30.0, 0.0)));
        // The dair's Active volume carries the pogo on-hit technique.
        let vol = dair
            .windows
            .iter()
            .flat_map(|w| &w.volumes)
            .next()
            .expect("dair has an active volume");
        assert_eq!(
            vol.on_hit.as_ref().expect("dair volume authors on_hit").key,
            "pogo_bounce",
        );
    }

    #[test]
    fn directional_verb_chain_orders_most_specific_first() {
        assert_eq!(
            directional_verb_chain("attack", AttackDir::Down, false),
            vec!["attack_air_down", "attack_down", "attack_air", "attack"],
        );
        assert_eq!(
            directional_verb_chain("attack", AttackDir::Down, true),
            vec!["attack_down", "attack"],
        );
        assert_eq!(
            directional_verb_chain("attack", AttackDir::Neutral, true),
            vec!["attack"],
        );
        assert_eq!(
            directional_verb_chain("attack", AttackDir::Neutral, false),
            vec!["attack_air", "attack"],
        );
    }

    #[test]
    fn directional_resolution_falls_back_and_respects_gates() {
        // Only `attack` authored: every direction resolves to it.
        let base_only = MovesetContract {
            verbs: BTreeMap::from([("attack".to_string(), "attack".to_string())]),
            moves: vec![bare_move("attack", None)],
        };
        assert_eq!(
            base_only
                .move_for_directional_verb("attack", AttackDir::Down, false)
                .unwrap()
                .id,
            "attack",
        );

        // An aerial-only down-air (a pogo host): aerial+down picks it; the
        // grounded chain skips it (gate) and falls through to `attack`.
        let with_dair = MovesetContract {
            verbs: BTreeMap::from([
                ("attack".to_string(), "attack".to_string()),
                ("attack_air_down".to_string(), "dair".to_string()),
            ]),
            moves: vec![bare_move("attack", None), bare_move("dair", Some(false))],
        };
        assert_eq!(
            with_dair
                .move_for_directional_verb("attack", AttackDir::Down, false)
                .unwrap()
                .id,
            "dair",
        );
        assert_eq!(
            with_dair
                .move_for_directional_verb("attack", AttackDir::Down, true)
                .unwrap()
                .id,
            "attack",
        );

        // A grounded-only `attack_down` (a down-tilt) is chosen grounded but
        // skipped for an airborne body — gate-respecting fallthrough.
        let with_dtilt = MovesetContract {
            verbs: BTreeMap::from([
                ("attack".to_string(), "attack".to_string()),
                ("attack_down".to_string(), "dtilt".to_string()),
            ]),
            moves: vec![bare_move("attack", None), bare_move("dtilt", Some(true))],
        };
        assert_eq!(
            with_dtilt
                .move_for_directional_verb("attack", AttackDir::Down, true)
                .unwrap()
                .id,
            "dtilt",
        );
        assert_eq!(
            with_dtilt
                .move_for_directional_verb("attack", AttackDir::Down, false)
                .unwrap()
                .id,
            "attack",
        );
    }

    #[test]
    fn round_trips_through_ron() {
        let doc = EntityCatalogDoc::parse(SEED).unwrap();
        let text = doc.to_ron().unwrap();
        let back = EntityCatalogDoc::parse(&text).unwrap();
        assert_eq!(doc, back);
    }

    #[test]
    fn move_timeline_queries_answer_the_sim() {
        let doc = EntityCatalogDoc::parse(SEED).unwrap();
        let moveset = doc
            .entity("sandbag_seed")
            .unwrap()
            .contracts
            .moveset
            .as_ref()
            .unwrap();
        let swat = moveset.move_by_id("swat").unwrap();
        // Proper-time queries: nothing live during startup, one volume
        // mid-active, nothing during recovery.
        assert_eq!(swat.active_volumes_at(0.1).count(), 0);
        assert_eq!(swat.active_volumes_at(0.30).count(), 1);
        assert_eq!(swat.active_volumes_at(0.5).count(), 0);
        // Phase is normalized move progress — what the clip samples by.
        assert!((swat.phase_at(0.34) - 0.5).abs() < 1e-6);
        assert_eq!(swat.phase_at(2.0), 1.0);
    }

    #[test]
    fn validators_catch_structural_violations() {
        let bad = r#"
        (
            schema_version: 1,
            entities: [
                (
                    id: "bad",
                    contracts: (
                        moveset: Some((
                            verbs: { "attack": "missing" },
                            moves: [
                                (
                                    id: "broken",
                                    clip: (clip: ""),
                                    duration_s: 0.5,
                                    windows: [
                                        (start_s: 0.4, end_s: 0.9, tag: Startup, volumes: []),
                                        (start_s: 0.0, end_s: 0.2, tag: Recovery, volumes: [
                                            (shape: Circle(offset: (0.0, 0.0), radius: 0.0),
                                             damage: 1, knockback: 0.0),
                                        ]),
                                        (start_s: 0.2, end_s: 0.4, tag: Cancelable(into: ["nowhere"]), volumes: []),
                                    ],
                                    events: [ (at_s: 0.9, kind: Effect((key: "boom"))) ],
                                ),
                            ],
                        )),
                    ),
                ),
                ( id: "bad", contracts: () ),
            ],
        )
        "#;
        let doc = EntityCatalogDoc::parse(bad).unwrap();
        let errors = doc.validate();
        let has = |f: &dyn Fn(&CatalogError) -> bool| errors.iter().any(|e| f(e));
        assert!(has(&|e| matches!(
            e,
            CatalogError::DuplicateEntityId { .. }
        )));
        assert!(has(&|e| matches!(e, CatalogError::WindowOutOfRange { .. })));
        assert!(has(&|e| matches!(
            e,
            CatalogError::VolumesOnInactiveWindow { .. }
        )));
        assert!(has(&|e| matches!(e, CatalogError::DegenerateVolume { .. })));
        assert!(has(&|e| matches!(
            e,
            CatalogError::UnknownCancelTarget { .. }
        )));
        assert!(has(&|e| matches!(e, CatalogError::UnknownVerbMove { .. })));
        assert!(has(&|e| matches!(e, CatalogError::EventOutOfRange { .. })));
        assert!(has(&|e| matches!(e, CatalogError::EmptyClipBinding { .. })));
    }

    /// The relativity contract, pinned as behavior: the timeline is queried
    /// in the OWNER'S proper time, so a dilated actor advancing at 0.25×
    /// world rate reaches its active window after 4× the world time — by
    /// construction, because the caller integrates proper time from the
    /// owner's dt. The schema carries no world-time anywhere.
    #[test]
    fn proper_time_integration_is_callers_dt_sum() {
        let doc = EntityCatalogDoc::parse(SEED).unwrap();
        let moveset = doc
            .entity("sandbag_seed")
            .unwrap()
            .contracts
            .moveset
            .as_ref()
            .unwrap();
        let swat = moveset.move_by_id("swat").unwrap();
        // Simulate a 0.25×-dilated owner: 60 world frames of 16ms reach only
        // 0.24s proper — still in startup. An undilated owner is active.
        let dilated: f32 = (0..60).map(|_| 0.016 * 0.25).sum();
        let undilated: f32 = (0..60).map(|_| 0.016).sum();
        assert_eq!(swat.active_volumes_at(dilated).count(), 0);
        assert_eq!(swat.active_volumes_at(undilated - 0.65).count(), 1);
    }
}

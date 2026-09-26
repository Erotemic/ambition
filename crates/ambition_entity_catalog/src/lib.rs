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

/// The primitives a character's move table is written with.
///
/// This module does not link Bevy: authoring a move is a pure value
/// computation.
pub mod authoring;
/// The hazard-travel family: see the module doc for why it is its own file.
pub mod hazard;

/// The move family's artifact section: its kind, its own version, and the codec
/// that turns a move table into a payload the content envelope can carry.
pub mod launch;
/// The main game's Mana: identity, pool and rate. See the module doc.
pub mod mana;
pub mod move_section;

// Re-exported at the crate root because many call sites in other crates use
// the crate-root names.
pub(crate) use hazard::hazard_of;
pub use hazard::{MoveHazard, ThreatTravel, RANGED_ACTION_REACH};

/// The platform-fighter authoring vocabulary: captures, repertoires, counters,
/// tethers, portals and the other technique families, as pure value
/// constructors.
///
/// None of these modules link Bevy. Every shipped moveset calls
/// [`smash_repertoire`] and [`smash_capture`], so an offline builder can author
/// a real move without an engine. Runtime hold state lives in
/// `ambition_characters::smash_hold_state`.
pub mod smash_bolt;
pub mod smash_bomb;
pub mod smash_capture;
pub mod smash_counter;
pub mod smash_flyline;
pub mod smash_homing;
pub mod smash_limit;
pub mod smash_mark;
pub mod smash_mine;
pub mod smash_portal;
pub mod smash_repertoire;
pub mod smash_ride;
pub mod smash_riposte;
pub mod smash_sleep;
pub mod smash_spring;
pub mod smash_teleport;
pub mod smash_tether;
pub mod smash_time_dilation;
pub mod smash_trapdoor;
pub mod smash_vitality;


use std::collections::{BTreeMap, HashSet};

use serde::{Deserialize, Serialize};

pub mod action_scheme;
pub mod brain_profile_ref;
pub mod placements;

pub use brain_profile_ref::{BrainProfileId, BrainProfileRef};

/// The reward/effect represented by a pickup, a chest, or a defeated boss.
///
/// A leaf noun type: `i32` and `String`, no behavior, no Bevy.
/// `ambition_interaction` re-exports it.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum PickupKind {
    Health { amount: i32 },
    Currency { amount: i32 },
    Ability { ability_id: String },
    StoryFlag { flag: String },
    Custom(String),
}

// ---------------------------------------------------------------------------
// Ability vocabulary: the one effect reference + its opaque params.
// ---------------------------------------------------------------------------

/// Opaque, structured parameters for a technique or prefab. The authored RON is byte-identical
/// to a `Reflect`-typed form, so if a visual move editor ever lands, swapping hydration to the
/// type registry is a mechanical migration — the data survives.
///
/// `Default` is the empty table `{}` (not `Unit`): a paramless `EffectRef`
/// hydrates cleanly into a technique's all-defaults `#[derive(Deserialize)]`
/// param struct.
///
/// A params struct must not contain an enum, and the failure is silent. A
/// Rust enum (also a unit-variant enum) does not survive the `ron::Value`
/// round trip: `from_typed` succeeds, but `hydrate` returns
/// `InvalidValueForType`. A consumer that treats a hydrate failure as "no
/// params" then stops working with only a `warn!`. Use a `bool`, a number, or
/// a string. For a closed set, use a string and validate it in the params
/// type's own `problems()`.
///
/// Write a round-trip probe for each new params type (see
/// `smash_counter::round_trip_probe`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParamValue(pub ron::Value);

impl Default for ParamValue {
    fn default() -> Self {
        ParamValue(ron::Value::Map(ron::Map::new()))
    }
}

impl ParamValue {
    /// Every authored float field that is not finite, by the path an author reads.
    ///
    /// `NaN`, `inf` and `-inf` are valid RON and hydrate cleanly, so
    /// `check_hydrates::<T>` admits them. A non-finite value in gameplay state
    /// stays there: for example, `ResourceMeter::refill` clamps, and `f32::clamp`
    /// returns `NaN` for a `NaN` input. `body.mana` is rollback-canonical, so the
    /// bad value is also snapshotted and restored.
    ///
    /// The check walks the `ron::Value`, not a typed struct, so it covers every
    /// technique without a list of keys. Integers always map to a finite `f64`, so
    /// they never fail.
    pub fn nonfinite_fields(&self) -> Vec<String> {
        fn walk(value: &ron::Value, path: &str, out: &mut Vec<String>) {
            match value {
                ron::Value::Number(number) => {
                    if !number.into_f64().is_finite() {
                        out.push(if path.is_empty() {
                            "<the value itself>".to_string()
                        } else {
                            path.to_string()
                        });
                    }
                }
                ron::Value::Map(map) => {
                    for (key, child) in map.iter() {
                        let name = match key {
                            ron::Value::String(name) => name.clone(),
                            other => format!("{other:?}"),
                        };
                        let next = if path.is_empty() {
                            name
                        } else {
                            format!("{path}.{name}")
                        };
                        walk(child, &next, out);
                    }
                }
                ron::Value::Seq(items) => {
                    for (index, child) in items.iter().enumerate() {
                        walk(child, &format!("{path}[{index}]"), out);
                    }
                }
                // Walk through an `Option`: `Some(NaN)` is as bad as a bare `NaN`.
                ron::Value::Option(Some(inner)) => walk(inner, path, out),
                ron::Value::Bool(_)
                | ron::Value::Char(_)
                | ron::Value::Option(None)
                | ron::Value::String(_)
                | ron::Value::Bytes(_)
                | ron::Value::Unit => {}
            }
        }
        let mut out = Vec::new();
        walk(&self.0, "", &mut out);
        out
    }

    /// Parse authored RON param text (`"(rise: 320.0)"`) into a value.
    pub fn parse(ron_text: &str) -> Result<Self, ron::error::SpannedError> {
        Ok(ParamValue(ron::from_str(ron_text)?))
    }

    /// Build params from a technique's own typed struct. This is the inverse
    /// of [`hydrate`](Self::hydrate).
    pub fn from_typed<T: Serialize>(value: &T) -> Result<Self, ron::Error> {
        let text = ron::ser::to_string(value)?;
        ron::from_str(&text)
            .map(ParamValue)
            .map_err(|spanned| spanned.code)
    }

    /// Hydrate these params into a technique/prefab's own `Deserialize` type.
    /// The consumer declares the concrete type; this crate never names it. A
    /// missing required field or a type mismatch fails here (the install-time
    /// param-schema check). `ron::Value` cannot deserialize enums; use string
    /// tags.
    pub fn hydrate<T: serde::de::DeserializeOwned>(&self) -> Result<T, ron::Error> {
        self.0.clone().into_rust()
    }
    /// Did the author write no parameters at all?
    ///
    /// An empty map and a non-map both count as empty: neither supplies a
    /// field. The refusal names the technique, so the author knows which key
    /// dropped the value.
    pub fn is_empty(&self) -> bool {
        match &self.0 {
            ron::Value::Map(map) => map.is_empty(),
            ron::Value::Unit => true,
            _ => false,
        }
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

/// A check that authored params hydrate into the technique's own `T`.
/// Register it as `registry.register("glider", check_hydrates::<GliderParams>)`.
/// A missing field or a type mismatch then fails at startup, not mid-fight.
pub fn check_hydrates<T: serde::de::DeserializeOwned>(params: &ParamValue) -> Result<(), String> {
    params.hydrate::<T>().map(|_| ()).map_err(|e| e.to_string())
}

/// Install-time param-schema validation registry. A content-owned
/// technique/prefab may register a [`ParamCheck`] under its effect key. The
/// content-validation pass runs every authored [`EffectRef`] through
/// [`validate`](Self::validate), so a param typo fails at startup. An
/// unregistered key always passes.
#[derive(Default)]
pub struct ParamSchemaRegistry {
    checks: BTreeMap<String, ParamCheck>,
}

impl ParamSchemaRegistry {
    /// Register a technique's param check. The last registration for a key
    /// wins; content install is the only caller.
    ///
    /// Replacement is deliberate. `ambition_registry_core::classify` cannot be
    /// used: it compares entries with `PartialEq`, and a [`ParamCheck`] is a
    /// function pointer, which must not be part of a registration's identity.
    /// So no idempotent or conflict case can be detected.
    ///
    /// If a second caller appears, an override silently replaces a validator,
    /// and a param typo the first check caught then passes at startup.
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

/// Where in a move an authored [`EffectRef`] was found.
///
/// It records the path, not only the key, so an author knows which volume or
/// node names the refused technique.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EffectSite {
    /// `windows[w].volumes[v].on_hit`
    VolumeOnHit { window: usize, volume: usize },
    /// `windows[w].sustain_effect`
    WindowSustain { window: usize },
    /// `events[e].kind = Effect(..)`
    Event { event: usize },
    /// `flow.nodes[n] = Emit { effect, .. }`
    FlowEmit { node: usize },
}

impl std::fmt::Display for EffectSite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EffectSite::VolumeOnHit { window, volume } => {
                write!(f, "windows[{window}].volumes[{volume}].on_hit")
            }
            EffectSite::WindowSustain { window } => write!(f, "windows[{window}].sustain_effect"),
            EffectSite::Event { event } => write!(f, "events[{event}].kind"),
            EffectSite::FlowEmit { node } => write!(f, "flow.nodes[{node}]"),
        }
    }
}

/// The message road a technique's handler listens on.
///
/// Admission must use the [`EffectSite`] that `MoveSpec::effect_refs` reports.
/// Example: `pogo_bounce` is consumed only from `OnHitEffectMessage`. Authored
/// at a timeline event, a sustain slot or a flow `Emit`, it goes through
/// `MoveEventKind::Effect` into `ActorActionMessage::Special`, which the pogo
/// handler never reads. The move then plays and the technique does nothing.
///
/// The four sites collapse into two deliveries. A handler chooses its delivery
/// when it picks a `MessageReader`, so a declaration of the road stays true
/// when a new site is added.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TechniqueDelivery {
    /// Answered from `OnHitEffectMessage` — authored at a volume's `on_hit`.
    OnHit,
    /// Answered from `ActorActionMessage::Special` — authored at a timeline
    /// event, a window's sustain slot, or a flow `Emit` node.
    Action,
    /// Answered on both roads. Rare, and a claim: a handler that reads only one
    /// reader may not declare this.
    Either,
}

impl TechniqueDelivery {
    /// Does an effect authored at `site` reach a handler on this road?
    pub fn reaches(self, site: &EffectSite) -> bool {
        let authored = match site {
            EffectSite::VolumeOnHit { .. } => Self::OnHit,
            EffectSite::WindowSustain { .. }
            | EffectSite::Event { .. }
            | EffectSite::FlowEmit { .. } => Self::Action,
        };
        matches!(self, Self::Either) || self == authored
    }

    /// How an author would name this road in a diagnostic.
    pub fn describe(self) -> &'static str {
        match self {
            Self::OnHit => "a volume's `on_hit`",
            Self::Action => "a timeline event, a window's sustain slot, or a flow `Emit`",
            Self::Either => "any authored effect site",
        }
    }
}

/// What other authored definitions a technique's params name.
///
/// "Names nothing" is a contract, the same as `Paramless`. Some technique
/// params carry the id of another authored definition (for example
/// `SummonRideParams::character_id` and the `item_id`s). Preparation checks
/// them, so an unknown id fails at preparation and not at fire time (where
/// `preflight_planned_bodies` only logs an error).
///
/// The declaration carries the extractor, so preparation never matches on a
/// technique name. Every variant here is checked.
#[derive(Clone, Copy)]
pub enum NestedReferences {
    /// Names no other authored definition.
    None,
    /// The character ids this effect's params name.
    Characters(fn(&EffectRef) -> Vec<String>),
    /// The held-item ids this effect's params name.
    /// The held-item ids this effect's params name. They are checked against
    /// `held_item_by_id` in `ambition_characters::brain::action_set`.
    HeldItems(fn(&EffectRef) -> Vec<String>),
}

impl std::fmt::Debug for NestedReferences {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => f.write_str("None"),
            Self::Characters(_) => f.write_str("Characters(..)"),
            Self::HeldItems(_) => f.write_str("HeldItems(..)"),
        }
    }
}

impl PartialEq for NestedReferences {
    fn eq(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (Self::None, Self::None)
                | (Self::Characters(_), Self::Characters(_))
                | (Self::HeldItems(_), Self::HeldItems(_))
        )
    }
}

impl Eq for NestedReferences {}

impl NestedReferences {
    /// The character ids this effect names, or empty when it names none.
    pub fn characters(&self, effect: &EffectRef) -> Vec<String> {
        match self {
            Self::Characters(extract) => extract(effect),
            _ => Vec::new(),
        }
    }

    /// The held-item ids this effect names, or empty when it names none.
    pub fn held_items(&self, effect: &EffectRef) -> Vec<String> {
        match self {
            Self::HeldItems(extract) => extract(effect),
            _ => Vec::new(),
        }
    }
}

/// What one capability declares when it installs a technique handler.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TechniqueOffer {
    /// Who claims this key — a module path, for the conflict diagnostic.
    pub owner: &'static str,
    /// Whether the key takes authored parameters at all.
    /// Whether the key takes authored parameters at all.
    ///
    /// `Paramless` is a contract, not an absent schema. A technique that takes
    /// nothing must refuse a non-empty map, because silently dropping authored
    /// params hides an author error.
    pub params: TechniqueParams,
    /// What other authored definitions this key's params name. See
    /// [`NestedReferences`]: `None` is a claim, not an omission.
    pub references: NestedReferences,
    /// Which road this technique's handler listens on. See
    /// [`TechniqueDelivery`]: an effect authored at a site this road does not
    /// carry reaches nothing.
    pub delivery: TechniqueDelivery,
}

/// How a technique treats authored parameters.
#[derive(Clone, Copy)]
pub enum TechniqueParams {
    /// Takes none. A non-empty authored map is an error.
    None,
    /// Takes parameters, checked by this function.
    Checked(ParamCheck),
}

impl std::fmt::Debug for TechniqueParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TechniqueParams::None => f.write_str("None"),
            TechniqueParams::Checked(_) => f.write_str("Checked(..)"),
        }
    }
}

impl PartialEq for TechniqueParams {
    /// Two checked declarations are never equal. A [`ParamCheck`] is a
    /// function pointer, and a registration's identity must not contain
    /// process-local values (addresses, `TypeId`s, allocation order): two builds
    /// of the same content must fingerprint equal. So
    /// [`TechniqueSupport::declare`] reports a conflict by key and owner, not by
    /// comparing checks.
    fn eq(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (TechniqueParams::None, TechniqueParams::None)
        )
    }
}

impl Eq for TechniqueParams {}

/// Why an authored [`EffectRef`] is not admissible.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TechniqueRefusal {
    // There is no `Disabled` variant ("declared by a capability this
    // composition did not install"). `TechniqueSupport` stores only installed
    // offers, so nothing could construct it. Telling a typo from a key that
    // another composition installs needs a workspace-wide registry of declared
    // keys, which does not exist yet. Add the variant only with that source.
    /// No installed capability declares this key (for example a typo such as
    /// `smash.teleprot`). This is a data error, known at install time.
    Unknown { key: String },
    /// The technique takes no parameters and the author wrote some.
    UnexpectedParams { key: String, owner: &'static str },
    /// The technique's own check refused these parameters.
    BadParams {
        key: String,
        owner: &'static str,
        detail: String,
    },
    /// The key is installed, but authored somewhere its handler never reads.
    /// The key is installed, but authored at a site its handler never reads.
    /// Example: `pogo_bounce` authored at a timeline event becomes an
    /// `ActorActionMessage::Special`, which the pogo handler never consumes.
    WrongSite {
        key: String,
        owner: &'static str,
        site: String,
        accepts: &'static str,
    },
}

impl std::fmt::Display for TechniqueRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TechniqueRefusal::WrongSite {
                key,
                owner,
                site,
                accepts,
            } => write!(
                f,
                "effect '{key}' is authored at {site}, which its handler ({owner}) \
                 never reads — it answers {accepts}. The move plays and the \
                 technique does nothing"
            ),
            TechniqueRefusal::Unknown { key } => write!(
                f,
                "effect '{key}' names a technique nothing installed declares — a \
                 misspelled key reaches the runtime, matches no handler, and the \
                 move plays and does nothing"
            ),
            TechniqueRefusal::UnexpectedParams { key, owner } => write!(
                f,
                "effect '{key}' ('{owner}') takes no parameters, but parameters \
                 were authored for it — they would be silently dropped"
            ),
            TechniqueRefusal::BadParams { key, owner, detail } => {
                write!(f, "effect '{key}' ('{owner}'): {detail}")
            }
        }
    }
}

/// Two capabilities claiming one technique key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TechniqueConflict {
    pub key: String,
    pub held_by: &'static str,
    pub claimed_by: &'static str,
}

impl std::fmt::Display for TechniqueConflict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "technique '{}' is declared by '{}' and again by '{}'; one key, one \
             owner — the second declaration would silently replace the first's \
             parameter contract",
            self.key, self.held_by, self.claimed_by
        )
    }
}

/// Which techniques this composition actually installed, and what each one's
/// parameters must look like.
///
/// It replaces [`ParamSchemaRegistry`], which admitted unknown keys and
/// silently overwrote duplicates.
///
/// The capability that installs the handler makes the declaration. A key here
/// means a capability installs the thing that answers it. An absent key means
/// nothing answers it.
#[derive(Default, Clone)]
pub struct TechniqueSupport {
    offers: BTreeMap<String, TechniqueOffer>,
}

impl TechniqueSupport {
    /// Declare that this capability installs the handler for `key`.
    ///
    /// Refuses a second claim on one key instead of replacing it.
    pub fn declare(
        &mut self,
        key: impl Into<String>,
        offer: TechniqueOffer,
    ) -> Result<(), TechniqueConflict> {
        let key = key.into();
        if let Some(held) = self.offers.get(&key) {
            return Err(TechniqueConflict {
                key,
                held_by: held.owner,
                claimed_by: offer.owner,
            });
        }
        self.offers.insert(key, offer);
        Ok(())
    }

    /// Every declared key, in a deterministic order.
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.offers.keys().map(String::as_str)
    }

    /// What declares this key, if anything does.
    pub fn offer(&self, key: &str) -> Option<&TechniqueOffer> {
        self.offers.get(key)
    }

    /// Is this authored reference admissible against what is installed?
    pub fn admit(&self, effect: &EffectRef) -> Result<(), TechniqueRefusal> {
        self.admit_at(None, effect)
    }

    /// The same, told where the effect was authored.
    ///
    /// Pass the site from `MoveSpec::effect_refs`. Without it, an effect
    /// authored at a site its handler never reads passes (see
    /// [`TechniqueDelivery`]).
    ///
    /// `None` means "site unknown", for a caller that validates a bare
    /// `EffectRef`. Delivery is then not checked.
    pub fn admit_at(
        &self,
        site: Option<&EffectSite>,
        effect: &EffectRef,
    ) -> Result<(), TechniqueRefusal> {
        let Some(offer) = self.offers.get(&effect.key) else {
            return Err(TechniqueRefusal::Unknown {
                key: effect.key.clone(),
            });
        };
        if let Some(site) = site {
            if !offer.delivery.reaches(site) {
                return Err(TechniqueRefusal::WrongSite {
                    key: effect.key.clone(),
                    owner: offer.owner,
                    site: site.to_string(),
                    accepts: offer.delivery.describe(),
                });
            }
        }
        // Check finiteness before the declaration's own predicate. No authored
        // field may hold a non-finite float, so each declaration must not have
        // to remember this. Most declarations use only `check_hydrates::<T>`,
        // and serde builds `NaN` without error.
        let nonfinite = effect.params.nonfinite_fields();
        if !nonfinite.is_empty() {
            return Err(TechniqueRefusal::BadParams {
                key: effect.key.clone(),
                owner: offer.owner,
                detail: format!(
                    "not a finite number: {}. NaN and infinity are valid RON and \
                     hydrate cleanly, so nothing downstream refuses them — and a \
                     non-finite value reaching gameplay state poisons it \
                     permanently rather than misbehaving once",
                    nonfinite.join(", ")
                ),
            });
        }
        match offer.params {
            TechniqueParams::None => {
                if effect.params.is_empty() {
                    Ok(())
                } else {
                    Err(TechniqueRefusal::UnexpectedParams {
                        key: effect.key.clone(),
                        owner: offer.owner,
                    })
                }
            }
            TechniqueParams::Checked(check) => {
                check(&effect.params).map_err(|detail| TechniqueRefusal::BadParams {
                    key: effect.key.clone(),
                    owner: offer.owner,
                    detail,
                })
            }
        }
    }

    /// Every refusal across a batch, so an author sees all of them at once.
    pub fn admit_all<'a, I>(&self, refs: I) -> Vec<TechniqueRefusal>
    where
        I: IntoIterator<Item = &'a EffectRef>,
    {
        refs.into_iter()
            .filter_map(|effect| self.admit(effect).err())
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
    /// The owner takes hits without hitstun. Super armor: every hit.
    Armor,
    /// Threshold armor: the owner takes hits that deal less than `damage`
    /// without hitstun. A hit at or above `damage` breaks through.
    ///
    /// This is a separate tag, not a field on [`Self::Armor`], so shipped
    /// super-armor moves do not get a threshold they did not choose.
    ///
    /// The comparison is `>=`: to absorb 9 and break on 10, author
    /// `damage: 10`. Each hit is judged alone; small hits do not add up.
    ArmorUnder {
        /// The damage at which a hit breaks through.
        damage: i32,
    },
    /// The move may be canceled into the named moves. `into` entries share
    /// one namespace: literal move ids (`"jab2"`), verbs (`"special"`,
    /// `"attack"`), and classes (`"any_attack"`, `"jump"`, `"dash"`). The
    /// timeline is the cancel table.
    Cancelable {
        into: Vec<String>,
        /// When the escape is legal. The default is `Always`, so older RON
        /// rows parse unchanged.
        #[serde(default)]
        condition: CancelCondition,
    },
}

/// The cancel-target class namespace: names an authored `into` entry may use
/// besides a literal move id.
///
/// Derived from [`cancel_names_for`], not restated, so the validator and the
/// runtime accept the same names. The locomotion escapes are added here
/// because `trigger_moveset_moves` passes them directly and not through
/// `cancel_names_for`.
pub fn cancel_class_names() -> Vec<&'static str> {
    let mut names: Vec<&'static str> = [
        ATTACK_VERB,
        SMASH_VERB,
        SPECIAL_VERB,
        RANGED_VERB,
        GRAB_VERB,
        TAUNT_VERB,
    ]
    .iter()
    .flat_map(|verb| cancel_names_for(verb, false).iter().copied())
    .chain(cancel_names_for(ATTACK_VERB, true).iter().copied())
    // The locomotion escapes are not verbs. The trigger road passes them
    // directly to `cancel_permits(.., &[name])`.
    .chain(["jump", "dash"])
    .collect();
    names.sort_unstable();
    names.dedup();
    names
}

/// What one use of a move has done to a body so far.
///
/// Three facts, because a strike has three outcomes. Overlap alone (what the
/// hitbox sweep publishes) cannot tell a blocked hit from a connect.
///
/// `overlapped` without either of the others is a hit still being resolved:
/// the actor road decides on the overlap frame, the player road on the next.
/// A consumer that wants a decision must read `connected` or `blocked`, not
/// the absence of one.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MoveContact {
    /// The move's volume overlapped a hurtbox. The staling fact — a move that
    /// hits a shield has been used.
    pub overlapped: bool,
    /// The overlap resolved to a connect: damage or knockback reached a body.
    pub connected: bool,
    /// The overlap resolved to a block: a guard consumed it.
    pub blocked: bool,
}

/// When a [`WindowTag::Cancelable`] escape is legal.
///
/// The overlap marker stales the move on overlap (a move that hits a shield
/// has been used). The resolved facts (connect, block) arrive on their own
/// channels, so nothing is written and then retracted. On the actor road the
/// connect-frame cancel is unchanged, because that road resolves on the
/// overlap frame. Against a player victim, an `OnHit` cancel opens one frame
/// later.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum CancelCondition {
    /// Any time the window is open.
    #[default]
    Always,
    /// Only after this move connected with a victim (combo confirm: jab
    /// chains into jab2 on hit).
    OnHit,
    /// Only while the move touched nothing (whiff escape from a missed
    /// heavy's recovery). A blocked move is not a whiff.
    OnWhiff,
    /// Only after a guard absorbed this move (the safe-on-block follow-up).
    /// It reads `BlockedBodyHit`, which the damage resolver publishes.
    OnBlock,
}

impl CancelCondition {
    /// Is this escape legal given the move's contact so far?
    ///
    /// [`MoveSpec::cancel_permits`] and [`MoveSpec::cancel_successors`] both
    /// ask this, so a chain cannot nominate a successor the cancel refuses.
    pub fn permits(self, contact: MoveContact) -> bool {
        match self {
            Self::Always => true,
            Self::OnHit => contact.connected,
            // Test the absence of an overlap, not of a connect. A blocked move
            // must not get the whiff escape.
            Self::OnWhiff => !contact.overlapped,
            Self::OnBlock => contact.blocked,
        }
    }
}

/// An axis-aligned or circular hit volume in entity-local logical space
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

impl VolumeShape {
    /// How far in front of the body this volume reaches, body-local.
    ///
    /// Call this; do not write `offset + half_extent` at a call site, which
    /// can drift. `test_the_grab_reach_is_one_formula` guards the same rule
    /// for capture params.
    pub fn leading_edge_x(&self) -> f32 {
        self.coverage_box().max.0
    }

    /// The body-local box this volume occupies.
    ///
    /// The single spelling of `offset ± half_extent`. [`leading_edge_x`] is a
    /// projection of it; do not write the box out by hand elsewhere.
    ///
    /// [`leading_edge_x`]: VolumeShape::leading_edge_x
    pub fn coverage_box(&self) -> MoveCoverage {
        match *self {
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
        }
    }
}

/// A per-volume override of what a hit does to the body it lands on.
///
/// The two arms are opposites: [`Self::Autolink`] pulls the victim in,
/// [`Self::Windbox`] pushes it away. Targeting, faction and contact resolve
/// the same for both and for an ordinary hit.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum VolumeReaction {
    /// Autolink: this volume holds its victim near the attacker and does not
    /// launch it.
    ///
    /// A multi-hit move authors this on its intermediate volumes, so the
    /// victim stays in the next hitbox, and leaves the final volume alone. It
    /// is not a capture: no relationship, no hold clock, no escape, and the
    /// victim keeps every verb.
    Autolink(AutolinkVolume),
    /// Windbox: this volume pushes its victim and does nothing else (no
    /// damage, no hitstun, no shield).
    Windbox(WindboxVolume),
}

/// One hit volume carried by an [`WindowTag::Active`] window, with its hit
/// payload. Volumes live on their window — where the timeline says they are —
/// not in a parallel list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HitVolume {
    pub shape: VolumeShape,
    /// Damage dealt on contact.
    pub damage: i32,
    /// Knockback impulse magnitude (engine units; direction is derived from
    /// facing + contact by the combat runtime).
    #[serde(default)]
    pub knockback: f32,
    /// Knockback growth per point of the victim's accumulated damage: the
    /// applied knockback is
    /// `knockback + knockback_growth * victim.damage_taken() / victim.weight`.
    ///
    /// `None`: the stage's ruleset growth applies.
    ///
    /// `Some(g)`: exactly `g`. `Some(0.0)` is fixed knockback, the same at 0%
    /// and at 200%, which multi-hit moves need.
    ///
    /// An `Option`, not an `f32`, because
    /// `ambition_platformer2d::combat::hitbox::resolved_hitbox_knockback_magnitude`
    /// treats zero as "unspecified" and substitutes the stage's value.
    #[serde(default)]
    pub knockback_growth: Option<f32>,
    /// Body-local launch direction override `(+x = facing, +y = gravity-down)`.
    /// `None` uses the facing+contact derivation. The runtime mirrors x by
    /// facing, rotates into the owner's gravity frame, then applies DI.
    #[serde(default)]
    pub launch_dir: Option<(f32, f32)>,
    /// How this volume's reaction differs from an ordinary hit. `None` is an
    /// ordinary hit.
    ///
    /// One sum type, not two `Option`s, so a volume cannot ask to hold and to
    /// push at once.
    #[serde(default)]
    pub reaction: Option<VolumeReaction>,
    /// Optional technique fired when this volume lands, with owner/victim/contact
    /// context. `None` is an ordinary damage-only volume.
    #[serde(default)]
    pub on_hit: Option<EffectRef>,
    /// Presentation tag for this volume's strike. A bladed swing authors
    /// `"slash_arc"` / `"slash_poke"`. The move runtime then (a) draws the
    /// slash VFX from the same spawned volume, so hitbox and slash always
    /// agree, and (b) treats the volume as the character's blade: it uses the
    /// sprite manifest's per-animation hit polygon (keyed by the move's clip
    /// name) in place of this shape when the owner authors one. `None` is a
    /// silent volume (boss strikes, hazards): no VFX, no manifest override.
    /// Unknown tags draw the default arc.
    #[serde(default)]
    pub vfx: Option<String>,
    /// Authored strike sound id: the sound this attack makes on contact, for
    /// example `"player.slash"` or `"world.rock.hit"`. The string is the
    /// `SfxId` name (lowered via `SfxId::new` at spawn). An id the bank does
    /// not have plays nothing, so it is always safe. `None` uses the victim's
    /// default hurt sound. Spray and debris belong to the victim's
    /// `HurtFeedback`.
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

/// A windbox: this volume moves its victim without hurting it.
///
/// It carries no push of its own. The volume's `knockback` and `launch_dir`
/// already say where and how hard it throws, and a second push vector would
/// disagree with them.
///
/// `damage: 0` is authorable, and `damage_floor` keeps it at zero. Without
/// this field, a damageless volume still stuns its victim and spends its
/// hit-once slot. This field removes that difference.
///
/// Targeting, faction and contact resolve the same as for a hit.
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct WindboxVolume {
    /// May this volume move the same body again while it stays in the gust?
    ///
    /// `true` opts out of the hit-once set, which stops a long active window
    /// from re-hitting a target every frame. Use it for a sustained wind, not
    /// a one-shot shove.
    #[serde(default)]
    pub repeating: bool,
}

/// The authored half of an autolink pulse: where it holds, how hard, and how
/// much of the attacker's own motion the victim inherits.
///
/// The runtime samples the attacker's velocity at the pulse; it is not
/// authored.
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct AutolinkVolume {
    /// Follow point in the attacker's local frame: `x` forward along its facing,
    /// `y` toward its feet — the same local convention `launch_dir` uses.
    pub anchor: (f32, f32),
    /// Share of the attacker's own velocity handed to the victim, `0..=1`.
    /// A rising move needs this: the correction below only closes a gap, and a
    /// fighter climbing fast outruns any gap-closing term.
    #[serde(default = "one")]
    pub carry: f32,
    /// Spring gain on the remaining gap, in 1/s. How hard this move grabs.
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
#[serde(deny_unknown_fields)]
pub struct MoveWindow {
    pub start_s: f32,
    pub end_s: f32,
    pub tag: WindowTag,
    /// Hit volumes live during this window (meaningful for `Active`).
    #[serde(default)]
    pub volumes: Vec<HitVolume>,
    /// The move's motion lock: how much of the owner's steering intent
    /// survives inside this window. `1.0` (the default) leaves steering
    /// unchanged. A value below `1.0` damps it (for example a committed heavy
    /// strike; the boss strike-speed throttle authors this on its Active
    /// window). `0.0` roots the body. The body enforces it at integration
    /// ([`MoveSpec::motion_scale_at`]), so it holds for a brain or a player
    /// controller. It scales intent magnitude, never a world direction.
    #[serde(default = "default_motion_scale")]
    pub motion_scale: f32,
    /// A sustained content effect: while this window is active, an
    /// `Effect { key }` is emitted every frame (not one-shot like a
    /// `MoveEvent`). Use it for a held special such as a lingering beam. The
    /// technique times its own cadence from the per-frame signal. The boss
    /// `apple_rain`-style specials use this. `None` for ordinary windows.
    #[serde(default)]
    pub sustain_effect: Option<EffectRef>,
}

/// A timed one-shot on the move timeline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MoveEventKind {
    /// Play a sound cue by key.
    Sfx { cue: String },
    /// Emit a cosmetic visual effect by id. Unlike [`Effect`](Self::Effect)
    /// (a gameplay technique), this changes only how the move looks. The sim
    /// emits the fact; presentation resolves the id against the rows of the
    /// shipped FX spritesheets (`ambition_sprite_sheet::fx`) and draws that
    /// clip at the owner. A typo is a validation error where a validator runs
    /// (`MoveSpec::presentation_problems`) and a counted miss at draw time
    /// otherwise.
    Vfx {
        effect: String,
        /// Where, body-local: `+x` toward the committed facing, `+y`
        /// gravity-down. This is the convention of
        /// [`Impulse`](Self::Impulse) and every [`HitVolume`] offset, so an
        /// effect can sit on the volume that throws it.
        #[serde(default)]
        at: (f32, f32),
        /// Size as a multiple of the presentation's default effect size
        /// (`1.0`).
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
    /// Put the owner under a timed gravity multiplier: a parasol, a float, a
    /// slow-fall. `scale` multiplies the body's gravity for `seconds`, after
    /// which the movement domain restores it with no second call.
    ///
    /// The move asks and the movement domain owns the regime. So this is a
    /// duration, not an on/off pair: a move that owes an "off" leaks the regime
    /// when it is interrupted, canceled, or rolled back.
    ///
    /// The regime outlives the move on purpose. `seconds` runs from the beat
    /// that fires it, so a parasol opened in a 0.3s animation can hold a body
    /// up for two seconds. A `WindowTag` cannot say this, because a window ends
    /// with its move.
    GravityModifier {
        /// Multiplier on the body's gravity. `1.0` is no change, `0.25` is a
        /// parasol, `0.0` is a hover and is legal.
        scale: f32,
        /// How long it lasts, from this beat. A non-positive value clears the
        /// body's current modifier, so a move can end one early.
        seconds: f32,
    },
    /// Leave the owner's side speed to the move for `seconds`: the movement
    /// domain neither steers nor brakes it until the clock runs out, as for an
    /// accepted roll. Paired with an [`Impulse`](Self::Impulse) at the same
    /// beat, it is a timed step of `speed x seconds` that ground friction does
    /// not eat. A duration for the reason [`GravityModifier`](Self::GravityModifier)
    /// is one; `0.0` hands the side speed back.
    HoldVelocity {
        seconds: f32,
    },
}

/// How a [`MoveEventKind::Impulse`] meets the velocity the body already had.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImpulseMode {
    /// Add to the current velocity. This is the meaning of
    /// [`MoveSpec::start_impulse`] and the default. Use it for a lunge or a
    /// drift nudge.
    #[default]
    Add,
    /// Replace the velocity. The result does not depend on how fast the body
    /// was falling, which a recovery move needs. It is the only mode a static
    /// reader can price ([`MoveFrameData::lift_speed`]).
    Set,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
pub struct ClipBinding {
    pub clip: String,
    #[serde(default)]
    pub fallbacks: Vec<String>,
}

/// What a move does to its owner's once-per-airtime recovery.
///
/// One enum and not two booleans, so no combination is meaningless.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecoveryUse {
    /// Not a recovery: costs nothing and is refused by nothing.
    #[default]
    None,
    /// The ordinary up-B. Spends the airtime's one recovery and leaves its
    /// owner helpless once the move ends: freefall is the price of the height.
    SpendAndFreefall,
    /// Spends the airtime's one recovery and leaves its owner able to act.
    /// Spends the airtime's one recovery and leaves its owner able to act.
    ///
    /// Use this when the recovery gives a vehicle: the height and control come
    /// together, so the price is the budget alone (the pirate can still swing
    /// from the shark's saddle).
    ///
    /// This is not free. The charge is still spent and refreshes only on a
    /// re-seating cause or a flinching hit. It declines the helpless episode,
    /// not the budget (see `BodyJumpState::post_recovery_helpless`).
    SpendWithoutFreefall,
}

impl RecoveryUse {
    /// Does starting this move cost a recovery charge?
    pub const fn spends(self) -> bool {
        !matches!(self, Self::None)
    }

    /// Does spending the last charge on this move arm the helpless episode?
    pub const fn arms_freefall(self) -> bool {
        matches!(self, Self::SpendAndFreefall)
    }
}

/// The kind of way home this move offers, as its author states it.
///
/// Not every recovery is one commanded burst. A fighter can teleport, and the
/// pirate's up-B summons a steerable shark that gives its rider seconds of
/// movement authority. For both, `lift_speed` reads `0.0`, so the planner needs
/// this authored statement. Do not fake a lift (the planner would certify a
/// rise the move does not give) and do not match on names.
///
/// `None` means "what the frame data implies": a burst when the move commands
/// one, otherwise nothing.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum AuthoredRecoveryRoute {
    /// Seconds of movement authority, from something the move summons or
    /// mounts.
    ///
    /// `reach` is authored: "this gets you home from within this far". The
    /// planner cannot see the summoned body's locomotion to derive it.
    SustainedAuthority { seconds: f32, reach: f32 },
    /// A discontinuity: the body is somewhere else, up to `distance` away.
    Teleport { distance: f32 },
}

/// The resolved way home: the authored statement folded with what the move's
/// frame data implies. The fold happens only in [`MoveSpec::frame_data`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub enum RecoveryRoute {
    /// This move is no way home.
    #[default]
    None,
    /// One commanded velocity, thrown `at_s` into the move. The ordinary up-B.
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

    /// How far toward home this route carries the body before it falls
    /// normally again. `0.0` for a burst, whose effect is a velocity the kernel
    /// already simulates.
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
#[serde(deny_unknown_fields)]
pub struct MoveGates {
    /// `Some(true)` = grounded only; `Some(false)` = airborne only;
    /// `None` = either.
    #[serde(default)]
    pub grounded: Option<bool>,
    /// What this move costs the airtime's one recovery, and what it costs its
    /// owner afterwards.
    ///
    /// Without a recovery budget nothing limits a repeated rising special, and
    /// a fighter could press it forever. `grounded` cannot tell the second use
    /// in one airtime from the first.
    ///
    /// Authored, not inferred from a name or an impulse. For a smash fighter,
    /// the repertoire slot applies the default: see `SmashRepertoire`'s
    /// `UpSpecial`.
    #[serde(default)]
    pub recovery: RecoveryUse,
    /// This move refuses to start while something else owns the body's pose
    /// (a saddle, a lift, a grab).
    ///
    /// It is a start condition, not a late veto. Once a move starts, its
    /// authored events are owed, so any refusal must happen before
    /// `start_move` spends the costs. A downstream veto (for example in the
    /// summon translation) lets the move start, spend the recovery and play
    /// its cues, and then do nothing.
    ///
    /// Not a [`RecoveryUse`] arm: cost and permission to begin are separate.
    /// This applies to non-recovery moves too, and a recovery can be castable
    /// from a saddle.
    #[serde(default)]
    pub forbidden_while_held: bool,
    /// While this move plays, its owner has no steering authority: the
    /// controller's locomotion intent is zeroed and the body keeps only the
    /// motion the move gives it.
    ///
    /// This is the platform-fighter rule for a grounded attack: you cannot
    /// walk out of a jab, a tilt or a smash, and a dash attack slides only on
    /// its own impulse. It is a gate beside `grounded`, not a per-window
    /// number, because it is a fact about the stance.
    ///
    /// Default `false`: an action-adventure protagonist that keeps walking
    /// through a slash is a valid feel.
    #[serde(default)]
    pub roots_steering: bool,
    /// The way home this move offers, when its frame data cannot say. See
    /// [`AuthoredRecoveryRoute`]; `None` leaves the answer to the frame data,
    /// which is where every burst still comes from.
    #[serde(default)]
    pub recovery_route: Option<AuthoredRecoveryRoute>,
    /// What this move costs, as terms that each name the resource they spend.
    /// Empty is free.
    ///
    /// This is the third cost beside [`Self::recovery`] and the weapon
    /// recharge. Each term names its resource: a body that does not hold that
    /// resource cannot pay, so the ruleset cannot silently choose which meter
    /// pays.
    ///
    /// The check is not here. [`Self::permits`] runs in this data crate, which
    /// must not read body state. Refusal and payment happen at acceptance in
    /// `ambition_combat`, beside `afford_recovery`, on both the trigger and
    /// cancel roads. Every term is paid or none is.
    #[serde(default)]
    pub costs: Vec<ambition_resource_spec::ResourceCost>,
    /// The move to run in place of this one when acceptance refuses it, so a
    /// priced press is not a dead button.
    ///
    /// A named move, not a fall through the directional chain: the chain is
    /// keyed by verb, so a refused `special_down` would resolve to the neutral
    /// `special`.
    ///
    /// One hop only. The named move's own `when_refused` is not followed. This
    /// bounds cycles (`a -> b -> a`) without a visited set. If the named move
    /// is also refused, the press is refused.
    ///
    /// The fallback goes through the same acceptance filters as a pressed
    /// move; it is not a way around the gates.
    ///
    /// Names a move id in the same moveset (`MovesetContract::move_by_id`). An
    /// id that matches nothing refuses, like no fallback.
    /// `every_named_move_variant_resolves` guards against typos.
    #[serde(default)]
    pub when_refused: Option<String>,
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

/// What a move does next, based on what happened before.
///
/// A `MoveSpec` timeline says when something happens on a fixed clock. It
/// cannot say "wait until the strike connects; if it did, continue, else
/// recover". A flow adds that sequence.
///
/// It is deliberately not a language: no variables, arithmetic, expressions,
/// queries or blackboard (see
/// `docs/planning/engine/authored-gameplay-logic-and-orchestration.md`). Four
/// node kinds; transitions are indices into one list.
///
/// The flow owns no state except its cursor. Each operation belongs to another
/// authority: an effect key reaches a technique, and a signal is read from
/// facts the combat road publishes. From `expressive-move-capabilities.md`: a
/// complex move may coordinate many authorities, but it must not become the
/// authority for their state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TechniqueFlow {
    /// The nodes, in authored order. Execution starts at index 0.
    ///
    /// Indices, not names, validated by [`TechniqueFlow::problems`]. The
    /// authored form is small, so indices stay readable.
    pub nodes: Vec<FlowNode>,
}

/// One step of a [`TechniqueFlow`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FlowNode {
    /// Fire a semantic effect and move on in the same tick.
    ///
    /// The effect is an ordinary [`EffectRef`], the same value a window's
    /// `sustain_effect` or an event's `Effect` carries, so a flow can reach
    /// every technique.
    ///
    /// The edge is a `u16` because the cursor (`MovePlayback::flow_node`) is a
    /// `u16`. An `as u16` narrowing would silently wrap node 65,536 to node 0.
    /// With the cursor's own width, the conversion happens once, at the
    /// authoring or deserialization boundary, where an index that does not fit
    /// is a hard error that names the field.
    Emit { effect: EffectRef, then: u16 },
    /// Hold here until a signal arrives, or until the patience runs out.
    ///
    /// The timeout is mandatory: an authored "wait forever" is always a bug.
    ///
    /// The fighter does not get stuck. `MovePlayback::finished()` is
    /// `t >= spec.duration_s`, so the timeline ends the move whatever the flow
    /// is doing. `Finish` stops flow activity but does not remove the move's
    /// recovery, and an unfinished `Wait` does not extend the move. The cost of
    /// an unbounded wait is that every later `Emit` never fires, so the
    /// sequence silently does half its job.
    Wait {
        on: FlowSignal,
        timeout_s: f32,
        /// See [`FlowNode::Emit::then`] for why the edge is the cursor's width.
        then: u16,
        on_timeout: u16,
    },
    /// Take one road or the other, deciding immediately on a fact that is
    /// already true.
    ///
    /// Not a shorthand for `Wait`. A wait suspends and resumes; a branch
    /// resolves in the tick it is reached.
    Branch {
        on: FlowSignal,
        /// See [`FlowNode::Emit::then`] for why the edge is the cursor's width.
        then: u16,
        otherwise: u16,
    },
    /// The flow is over. The move plays out whatever timeline it has left.
    Finish,
}

/// Something a flow can wait for or branch on.
///
/// Every variant must be a fact that another authority already publishes. A
/// signal that needs new state would make the flow the authority for that
/// state. The current signals are the contact facts that `MovePlayback`
/// carries and the damage road decides.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FlowSignal {
    /// This move's own strike overlapped a hurtbox (the staling fact). It is
    /// also true for a blocked strike; to mean "it worked", use
    /// [`Self::Connected`].
    Overlapped,
    /// The overlap resolved to a connect: damage or knockback reached a body.
    Connected,
    /// The overlap resolved to a block: a guard consumed it.
    ///
    /// A parry is not a block. A perfect shield publishes `ParriedBodyHit`,
    /// which reaches no contact field, so a flow that waits here misses a
    /// parry.
    Blocked,
}

impl FlowSignal {
    /// Is this signal true of `contact` right now?
    pub fn satisfied_by(self, contact: MoveContact) -> bool {
        match self {
            FlowSignal::Overlapped => contact.overlapped,
            FlowSignal::Connected => contact.connected,
            FlowSignal::Blocked => contact.blocked,
        }
    }
}

/// The widest version-1 [`TechniqueFlow`] an author may publish.
///
/// This is the version's stated budget; a later version may raise it. Edges
/// are `u16`, the cursor's own width, so a large graph cannot wrap the cursor.
///
/// It bounds dispatch, not cost. It says nothing about what a native handler
/// reached by an `Emit` does; see the owner document's trust boundary.
pub const MAX_TECHNIQUE_FLOW_NODES: usize = 256;

impl TechniqueFlow {
    /// Everything wrong with this flow, as sentences an author can act on.
    ///
    /// Validated where it is authored, because each of these failures is
    /// silent at runtime: an edge past the end of the list, no reachable
    /// `Finish`, a `Wait` that never times out, a node nothing reaches. Each
    /// gives a move that plays and does only part of what it says.
    ///
    /// None of them traps a fighter: the move ends on its own timeline
    /// (`MovePlayback::finished()` is `t >= spec.duration_s`). The harm is
    /// authored intent that never runs, or, for a cycle, an `Emit` fired many
    /// times where the author wrote one.
    ///
    /// Cycles are rejected. Version 1 is a bounded acyclic move-local program;
    /// a repeat belongs in the move's timeline. The per-tick node budget only
    /// limits how fast a loop spins. `reaches_finish` is existential, so it
    /// admits a graph that ends on one branch and loops on the other, and the
    /// branch taken depends on whether the strike connected.
    pub fn problems(&self) -> Vec<String> {
        let mut problems = Vec::new();
        if self.nodes.is_empty() {
            problems.push("the flow has no nodes, so the move it is on does nothing".to_string());
            return problems;
        }
        let len = self.nodes.len();
        // Bound the graph by the version's budget (see
        // [`MAX_TECHNIQUE_FLOW_NODES`]).
        if len > MAX_TECHNIQUE_FLOW_NODES {
            problems.push(format!(
                "the flow has {len} nodes; version 1 admits at most \
                 {MAX_TECHNIQUE_FLOW_NODES}. A move-local program is meant to be \
                 readable in one screen; a graph this wide is a system that wants \
                 to be authored somewhere else"
            ));
            // Every later check indexes this list; reporting them all against an
            // oversized graph buries the one problem the author has to fix.
            return problems;
        }
        // Take edges from [`Self::successors`], so every check sees the same
        // edges for every `FlowNode` variant.
        for (index, node) in self.nodes.iter().enumerate() {
            for (what, target) in Self::successors(node).into_iter().flatten() {
                if usize::from(target) >= len {
                    problems.push(format!(
                        "node {index}'s `{what}` goes to {target}, past the last node ({})",
                        len - 1
                    ));
                }
            }
            // Require a finite timeout, not only a positive one:
            // `f32::INFINITY > 0.0` is true, and infinity is a "wait forever".
            // `NaN` already fails the positive test; it is named here so the
            // diagnostic shows the value.
            if let FlowNode::Wait { timeout_s, .. } = node {
                if !timeout_s.is_finite() || !(*timeout_s > 0.0) {
                    problems.push(format!(
                        "node {index} waits with a {timeout_s}s timeout, which never \
                     expires — a signal that never comes parks the flow here for \
                     the move's whole remaining window, so every step after this \
                     one silently never runs"
                    ));
                }
            }
        }
        // Check reachability, not presence: a `Finish` that no transition
        // reaches is the same as no `Finish`.
        if !self.reaches_finish() {
            problems.push(
                "no `Finish` is reachable from node 0, so the flow never stops stepping \
                 and keeps acting for the move's whole remaining window — the move still \
                 ends on its own timeline, but nothing an author wrote after the \
                 unreachable `Finish` marks the end of it"
                    .to_string(),
            );
        }
        // Reject cycles. `reaches_finish` is existential, so a branch whose
        // `then` reaches `Finish` and whose `otherwise` loops back passes it.
        if let Some(cycle) = self.first_cycle() {
            problems.push(format!(
                "the flow loops: {} returns to node {}. Version 1 is acyclic — a \
                 repeat belongs in the move's own timeline, not in a program that \
                 can take the looping road on one branch and terminate on the other",
                cycle
                    .iter()
                    .map(|index| format!("node {index}"))
                    .collect::<Vec<_>>()
                    .join(" -> "),
                cycle[0],
            ));
        }
        // A node nothing reaches is authored intent that never runs. Report
        // each one so the diagnostic names the site.
        let reachable = self.reachable_from_start();
        for index in 0..len {
            if !reachable[index] {
                problems.push(format!(
                    "node {index} is unreachable from node 0, so nothing it does can ever happen"
                ));
            }
        }
        problems
    }

    /// The successors of one node, in the order the author wrote them, each
    /// paired with the authored field name it was written under.
    ///
    /// This is the only place the edges are enumerated. Every edge walk in
    /// this type uses it, so a new `FlowNode` variant is seen by every check.
    /// The field name is included because the dangling report needs it.
    fn successors(node: &FlowNode) -> [Option<(&'static str, u16)>; 2] {
        match node {
            FlowNode::Emit { then, .. } => [Some(("then", *then)), None],
            FlowNode::Wait {
                then, on_timeout, ..
            } => [Some(("then", *then)), Some(("on_timeout", *on_timeout))],
            FlowNode::Branch {
                then, otherwise, ..
            } => [Some(("then", *then)), Some(("otherwise", *otherwise))],
            FlowNode::Finish => [None, None],
        }
    }

    /// Which nodes execution can arrive at, starting at node 0.
    fn reachable_from_start(&self) -> Vec<bool> {
        let mut seen = vec![false; self.nodes.len()];
        let mut stack = vec![0usize];
        while let Some(index) = stack.pop() {
            let Some(node) = self.nodes.get(index) else {
                continue;
            };
            if seen[index] {
                continue;
            }
            seen[index] = true;
            for (_, target) in Self::successors(node).into_iter().flatten() {
                stack.push(usize::from(target));
            }
        }
        seen
    }

    /// One cycle reachable from node 0, as the path that closes it.
    ///
    /// Returns the nodes from the repeated one round to itself, so the
    /// diagnostic can print the loop an author has to break rather than just
    /// asserting that one exists.
    fn first_cycle(&self) -> Option<Vec<usize>> {
        #[derive(Clone, Copy, PartialEq)]
        enum Mark {
            New,
            OnPath,
            Done,
        }
        let mut mark = vec![Mark::New; self.nodes.len()];
        let mut path: Vec<usize> = Vec::new();
        // Explicit stack of `(node, which successor to try next)`, so a deep
        // chain cannot overflow the call stack.
        let mut stack: Vec<(usize, usize)> = vec![(0, 0)];
        mark[0] = Mark::OnPath;
        path.push(0);
        while let Some((index, edge)) = stack.pop() {
            let Some(node) = self.nodes.get(index) else {
                continue;
            };
            let next = Self::successors(node)
                .into_iter()
                .flatten()
                .nth(edge)
                .map(|(_, target)| usize::from(target))
                .filter(|target| *target < self.nodes.len());
            match next {
                Some(target) => {
                    stack.push((index, edge + 1));
                    match mark[target] {
                        Mark::OnPath => {
                            let start = path
                                .iter()
                                .position(|node| *node == target)
                                .expect("a node marked on-path is on the path");
                            let mut cycle = path[start..].to_vec();
                            cycle.push(target);
                            return Some(cycle);
                        }
                        Mark::Done => {}
                        Mark::New => {
                            mark[target] = Mark::OnPath;
                            path.push(target);
                            stack.push((target, 0));
                        }
                    }
                }
                None => {
                    mark[index] = Mark::Done;
                    if path.last() == Some(&index) {
                        path.pop();
                    }
                }
            }
        }
        None
    }

    /// Can execution starting at node 0 arrive at a [`FlowNode::Finish`]?
    fn reaches_finish(&self) -> bool {
        let mut seen = vec![false; self.nodes.len()];
        let mut stack = vec![0usize];
        while let Some(index) = stack.pop() {
            let Some(node) = self.nodes.get(index) else {
                continue;
            };
            if seen[index] {
                continue;
            }
            seen[index] = true;
            if matches!(node, FlowNode::Finish) {
                return true;
            }
            for (_, target) in Self::successors(node).into_iter().flatten() {
                stack.push(usize::from(target));
            }
        }
        false
    }
}

/// One ability activation: a clip binding plus the full gameplay meaning of
/// the ability on one timeline. The move timeline is authoritative for both
/// gameplay and presentation — windows advance on the owner's proper time
/// and the bound clip is sampled by normalized move phase.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
    /// A one-shot body-local velocity add applied when the move is triggered:
    /// the move's self-motion (a jab's lunge, a dash-attack's slide, a
    /// back-air's drift). `(+x = facing, +y = gravity-down)`; the runtime
    /// mirrors x by facing and rotates it into the owner's gravity frame.
    /// `None` is no self-motion.
    #[serde(default)]
    pub start_impulse: Option<(f32, f32)>,
    /// Smash-charge payoff: the multiplier a fully charged release applies to
    /// this move's damage and knockback. The applied scale interpolates
    /// `1.0 → smash_charge_mult` by the charge fraction reached at release
    /// (how far the owner's clock advanced through the leading Startup
    /// window). The default `1.0` is no charge scaling. A smash roster authors
    /// for example `2.0`, so a held smash lands twice as hard as a tap.
    #[serde(default = "default_charge_mult")]
    pub smash_charge_mult: f32,
    /// How a chargeable use of this move holds and releases its charge.
    ///
    /// `None` = the derived policy: the hold sits a fraction into the leading
    /// Startup window and lasts [`SmashChargeSpec::DEFAULT_MAX_HOLD_S`].
    /// Authoring one is how a move differs — a slower windup that pays off
    /// sooner, or a charge that cannot be held at all.
    ///
    /// Authoring one also declares that the move charges. A charged shot needs
    /// this: its payoff is the projectile, and it has no melee volume for
    /// [`Self::smash_charge_mult`] to scale. Either statement counts; see
    /// [`Self::charge_policy`].
    ///
    /// [`Self::charge_gesture`] says which press holds it. A use reached
    /// through another verb is never chargeable.
    #[serde(default)]
    pub smash_charge: Option<SmashChargeSpec>,
    /// Which press holds this move's charge.
    ///
    /// A smash attack and a held neutral special both freeze a timeline while
    /// a button is down and release when it comes up. They differ only in the
    /// button, so the move names it.
    ///
    /// The default is [`ChargeGesture::Smash`].
    #[serde(default)]
    pub charge_gesture: ChargeGesture,
    /// The stretch of this move's timeline that repeats while its button is
    /// held (the rapid jab, the drill, the flurry).
    ///
    /// `None` is a move that plays once. The loop belongs to playback: a move
    /// says which of its windows repeat and for how long, and the runtime
    /// treats all of them the same.
    #[serde(default)]
    pub repeat: Option<MoveLoop>,
    /// Landing lag: the recovery this move owes if the body touches down
    /// before the move ended. Seconds of the owner's proper time, spent as a
    /// hard control lock.
    ///
    /// An aerial is a commitment: landing mid-move costs recovery.
    ///
    /// `None`: an aerial that lands is an ordinary landing.
    #[serde(default)]
    pub landing_lag_s: Option<f32>,
    /// Auto-cancel: land after this point in the move and pay no landing lag.
    /// Seconds of proper time from the move's start.
    ///
    /// A move thrown early enough that its dangerous part is over by touchdown
    /// lands clean.
    ///
    /// `None`: no auto-cancel window; [`Self::landing_lag_s`] applies whenever
    /// the move is still running. Ignored if no landing lag is authored.
    #[serde(default)]
    pub autocancel_after_s: Option<f32>,
    /// How many times a second this move mirrors the body's drawn sprite while
    /// it plays: a cheap spin.
    ///
    /// Presentation only. It flips the published pose's facing. The body's own
    /// `facing` is unchanged, so hitboxes, launch directions and facing rules
    /// are not affected.
    ///
    /// This is a deliberate placeholder until real rotation art exists.
    ///
    /// `None` (and zero): drawn normally.
    #[serde(default)]
    pub sprite_spin_hz: Option<f32>,
    /// The held item this move brandishes while it plays.
    ///
    /// A draw-and-swing move ("he pulls the gun-sword out and fires it") is one
    /// move, not an equip plus a shot. The item is worn for as long as the
    /// move's clock runs. Then the body's previous item comes back.
    ///
    /// It is not a pickup. Nothing enters or leaves an inventory, and the item
    /// cannot be dropped or thrown. The brandish remembers what it displaced
    /// and restores exactly that.
    ///
    /// `None` for almost every move.
    #[serde(default)]
    pub equips: Option<String>,
    /// What happens next, based on what happened before; see
    /// [`TechniqueFlow`].
    ///
    /// `None` for a move whose meaning is its timeline. A flow does not replace
    /// the timeline: windows open, volumes live and events fire as usual. The
    /// flow runs beside them.
    #[serde(default)]
    pub flow: Option<TechniqueFlow>,
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
/// release or at [`Self::max_s`], whichever comes first. What the move
/// authors after [`Self::to_s`] is the finisher the loop exits into, so a
/// flurry that ends in a launcher is one timeline.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MoveLoop {
    /// Where the clock jumps back to.
    pub from_s: f32,
    /// Where it jumps back from.
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

/// How a chargeable move holds: where on its own timeline the charge waits,
/// and how long it may wait before it fires itself.
///
/// Both values are seconds of the owner's proper time, like every clock on a
/// [`MoveSpec`]: a dilated fighter charges as slowly as it swings.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SmashChargeSpec {
    /// The instant the timeline freezes while Attack is held.
    pub hold_at_s: f32,
    /// The longest that freeze may last. Reaching it releases the move whether
    /// or not the button is still down, unless [`Self::stores`]. The
    /// auto-release stops a held smash from being a stall.
    pub max_hold_s: f32,
    /// Does a charge survive an interruption and resume on the next use?
    ///
    /// This is the stored shot (the neutral-B charge shot of the genre).
    ///
    /// `true` changes two things:
    ///
    /// - Reaching [`Self::max_hold_s`] does not fire the move. A full charge
    ///   stays loaded until the button comes up or the move is interrupted.
    /// - A use interrupted while still charging banks its charge, and the next
    ///   use of the same move resumes from there.
    ///
    /// Firing consumes the charge; nothing else refunds it. The bank is keyed
    /// by move id, so a stored charge cannot come out of a different move.
    ///
    /// The default is `false` (every smash).
    #[serde(default)]
    pub stores: bool,
    /// Does the freeze root the body?
    ///
    /// `true` for every smash: a charging fighter must not walk or move.
    ///
    /// It is a property of the policy, not of charging. The Performer's
    /// trapdoor freezes its timeline through this mechanic, and the held beat
    /// is travel: she steers under the stage. Rooting that freeze would
    /// remove the move. So each use states its choice.
    ///
    /// The default is `true`.
    #[serde(default = "charge_roots_by_default")]
    pub roots: bool,
    /// What keeps the freeze: the button, or the move.
    #[serde(default)]
    pub sustain: ChargeSustain,
}

/// What holds a frozen timeline frozen.
///
/// One mechanic, two shapes. A charge freezes the timeline and resumes it
/// later; what differs is what decides "later". A smash stays frozen while the
/// button is held; release it and it swings.
///
/// The Performer's trapdoor is the other shape. The held beat is about one
/// second of travel under the stage, which the player can cut short with a new
/// press. The player does not hold the button while steering, so a
/// hold-to-sustain freeze would end after a few ticks.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub enum ChargeSustain {
    /// The button stays down, and releasing it resumes the move. Every smash,
    /// and the default.
    #[default]
    WhileHeld,
    /// The freeze holds itself, up to the maximum, and a new press ends it.
    UntilPressedAgain,
}

/// Which press holds a move's charge.
///
/// A charge is one mechanic (freeze the timeline while a button is down,
/// release when it comes up) bound to two different buttons. The move says
/// which; the runtime treats both the same.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub enum ChargeGesture {
    /// The smash gesture. Every smash attack, and the default.
    #[default]
    Smash,
    /// The special press that started the move (the held neutral-B).
    Special,
}

impl SmashChargeSpec {
    /// A full charge takes one second (60 frames at 60Hz). A move that wants a
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
    /// Every authored [`EffectRef`] this move carries, with the path it sits on.
    ///
    /// Every authored [`EffectRef`] this move carries, with the path it sits on.
    ///
    /// A move names techniques from four places: a volume's `on_hit`, a
    /// window's `sustain_effect`, a timeline event's `Effect`, and a flow
    /// node's `Emit`. Validators must see all of them.
    ///
    /// Every level destructures without `..`, so a new field on [`MoveSpec`],
    /// [`MoveWindow`], [`HitVolume`] or [`MoveEventKind`] is a compile error
    /// here. The author of the field must then decide whether it can carry a
    /// technique.
    ///
    /// This walks the expanded move (after prefab expansion and authoring
    /// overrides), so an override that adds a reference is covered.
    pub fn effect_refs(&self) -> Vec<(EffectSite, &EffectRef)> {
        let MoveSpec {
            id: _,
            display_name: _,
            clip: _,
            duration_s: _,
            windows,
            events,
            gates: _,
            start_impulse: _,
            smash_charge_mult: _,
            smash_charge: _,
            charge_gesture: _,
            repeat: _,
            landing_lag_s: _,
            autocancel_after_s: _,
            sprite_spin_hz: _,
            equips: _,
            flow,
        } = self;
        let mut found: Vec<(EffectSite, &EffectRef)> = Vec::new();
        for (window, w) in windows.iter().enumerate() {
            let MoveWindow {
                start_s: _,
                end_s: _,
                tag: _,
                volumes,
                motion_scale: _,
                sustain_effect,
            } = w;
            for (volume, v) in volumes.iter().enumerate() {
                let HitVolume {
                    shape: _,
                    damage: _,
                    knockback: _,
                    knockback_growth: _,
                    launch_dir: _,
                    reaction: _,
                    on_hit,
                    vfx: _,
                    hit_sfx: _,
                } = v;
                if let Some(effect) = on_hit {
                    found.push((EffectSite::VolumeOnHit { window, volume }, effect));
                }
            }
            if let Some(effect) = sustain_effect {
                found.push((EffectSite::WindowSustain { window }, effect));
            }
        }
        for (event, e) in events.iter().enumerate() {
            let MoveEvent { at_s: _, kind } = e;
            // Exhaustive on purpose: a `_ => {}` arm would silently admit a
            // future variant that names a technique.
            match kind {
                MoveEventKind::Effect(effect) => {
                    found.push((EffectSite::Event { event }, effect));
                }
                MoveEventKind::Vfx { .. }
                | MoveEventKind::Sfx { .. }
                | MoveEventKind::Ranged
                | MoveEventKind::Impulse { .. }
                | MoveEventKind::GravityModifier { .. }
                | MoveEventKind::HoldVelocity { .. } => {}
            }
        }
        if let Some(TechniqueFlow { nodes }) = flow {
            for (node, n) in nodes.iter().enumerate() {
                match n {
                    FlowNode::Emit { effect, then: _ } => {
                        found.push((EffectSite::FlowEmit { node }, effect));
                    }
                    FlowNode::Wait { .. } | FlowNode::Branch { .. } | FlowNode::Finish => {}
                }
            }
        }
        found
    }

    /// The player-facing label for this move, used by the action scheme to
    /// name the slot: the authored [`Self::display_name`], else a title-cased
    /// `id` (`"sandbag_swat"` → `"Sandbag Swat"`).
    pub fn display(&self) -> String {
        self.display_name
            .clone()
            .unwrap_or_else(|| crate::action_scheme::title_case_id(&self.id))
    }

    /// Validate this move's presentation event ids, so a typo fails at load
    /// and not as a silent missing sound or effect. `vfx_known` is the
    /// injected cosmetic-vfx vocabulary; this crate does not depend on
    /// presentation. Pass `ambition_sprite_sheet::fx::is_authored_effect`,
    /// which reads the shipped FX sheets' baked manifests (pure; no App, no
    /// loaded assets). Returns one problem per bad id:
    /// - a `Vfx { effect }` whose id is not in the cosmetic vocabulary, and
    /// - a `Sfx { cue }` with an empty cue (a blank cue resolves to silence).
    /// An empty result means the presentation is resolvable.
    pub fn presentation_problems(&self, vfx_known: impl Fn(&str) -> bool) -> Vec<String> {
        let mut problems = Vec::new();
        for effect in self.vfx_effects() {
            if !vfx_known(effect) {
                problems.push(format!(
                    "move '{}': Vfx event names unknown cosmetic effect '{}' (no \
                     shipped FX spritesheet has a row by that name)",
                    self.id, effect
                ));
            }
        }
        for ev in &self.events {
            match &ev.kind {
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

    /// Every cosmetic effect this move can fire, by name. The install-time
    /// validator and the runtime FX demand both read this (a character-owned
    /// FX sheet is decoded when a realized moveset names one of its rows). Add
    /// any new place a move names an effect here.
    pub fn vfx_effects(&self) -> impl Iterator<Item = &str> {
        self.events.iter().filter_map(|ev| match &ev.kind {
            MoveEventKind::Vfx { effect, .. } => Some(effect.as_str()),
            _ => None,
        })
    }

    /// Does a window the predicate accepts cover proper-time `t`?
    ///
    /// Every defensive window uses this: an authored [`WindowTag::Invuln`] or
    /// [`WindowTag::Armor`] is in force for exactly its span, on the owner's
    /// clock.
    pub fn tagged_window_covers(&self, t: f32, want: fn(&WindowTag) -> bool) -> bool {
        self.windows
            .iter()
            .any(|w| want(&w.tag) && w.start_s <= t && t < w.end_s)
    }

    /// Every window tag in force at proper-time `t`, in authored order.
    ///
    /// Use this, not [`Self::tagged_window_covers`], when the tag carries a
    /// value the caller needs (for example `ArmorUnder`'s threshold).
    pub fn tagged_windows_covering(&self, t: f32) -> impl Iterator<Item = &WindowTag> {
        self.windows
            .iter()
            .filter(move |w| w.start_s <= t && t < w.end_s)
            .map(|w| &w.tag)
    }

    /// The successors this move's live cancel window names at proper-time `t`,
    /// in authored order.
    ///
    /// A `Cancelable` window with `into: ["jab2"]` permits jab2 and also
    /// nominates it. A follow-up press inside that window takes the
    /// nomination and does not restart the playing move. This is a jab chain.
    pub fn cancel_successors(&self, t: f32, contact: MoveContact) -> impl Iterator<Item = &str> {
        self.windows
            .iter()
            .filter(move |w| w.start_s <= t && t < w.end_s)
            .filter_map(move |w| match &w.tag {
                WindowTag::Cancelable { into, condition } => {
                    condition.permits(contact).then_some(into)
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

    /// The steering-intent scale in force at proper-time `t`: the minimum
    /// [`MoveWindow::motion_scale`] among the windows that contain `t`, and
    /// `1.0` outside every window. The body integrator multiplies the
    /// controller's steering intent by this each tick, so the motion lock
    /// binds every controller of the body.
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

    /// Normalized phase (`0..=1`) at proper-time `t`. Presentation samples the
    /// bound clip by this.
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
    /// The clamp is the invariant, not a defensive check. A move may author a
    /// zero-width Startup, lay Active before Startup ends, or have no Startup.
    /// In each case the first live volume is the line the freeze must not
    /// cross.
    fn derived_charge_hold_at_s(&self) -> f32 {
        let leading = self
            .windows
            .iter()
            .find(|w| matches!(w.tag, WindowTag::Startup));
        let Some(leading) = leading else {
            // No windup to hold inside: freeze at the move's first instant.
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
        // Strictly before: a freeze on the first Active instant is a defect.
        pose.clamp(0.0, (first_active - CHARGE_POSE_EPSILON_S).max(0.0))
    }

    /// The charge policy a smash-gesture use of this move plays under, or
    /// `None` when this move does not charge.
    ///
    /// The derived hold point comes from the timeline the move already
    /// authors, so every fighter with a charge multiplier is chargeable
    /// without moveset changes.
    pub fn charge_policy(&self) -> Option<SmashChargeSpec> {
        // Either payoff says this move charges. A charged shot pays in the
        // projectile it releases and has no melee volume for
        // `smash_charge_mult` to scale, so an explicit `smash_charge` is its
        // own statement of intent.
        let Some(policy) = self.smash_charge.or_else(|| {
            (self.smash_charge_mult > 1.0).then_some(SmashChargeSpec {
                // The charge pose is in the windup, not at the hitbox: a
                // charged smash freezes in its windup and releases into the
                // swing.
                //
                // Active membership is `start_s <= t < end_s`, and smash
                // authoring usually puts Active right after Startup. A freeze
                // at Startup's `end_s` would hold a charge with a live strike
                // volume spawned. So the hold sits a fraction into the windup,
                // strictly before the first Active window. The rest of the
                // windup plays on release.
                hold_at_s: self.derived_charge_hold_at_s(),
                max_hold_s: SmashChargeSpec::DEFAULT_MAX_HOLD_S,
                // A derived policy never stores. A smash charge is a
                // commitment inside one swing; a stored charge must be
                // authored.
                stores: false,
                roots: true,
                sustain: ChargeSustain::WhileHeld,
            })
        }) else {
            return None;
        };
        policy.holds().then_some(policy)
    }

    /// May this move, at proper-time `t` with the given contact, be canceled
    /// into a candidate that answers to any of `names`? The caller supplies
    /// every name the candidate answers to: its verb (`"attack"`,
    /// `"special"`, `"ranged"`), its resolved move id, and its classes
    /// (`"any_attack"` for the attack family; `"jump"`/`"dash"` for the
    /// locomotion escapes). An authored `into` entry matches any of them. A
    /// move with no `Cancelable` window refuses everything.
    pub fn cancel_permits(&self, t: f32, contact: MoveContact, names: &[&str]) -> bool {
        self.windows.iter().any(|w| match &w.tag {
            WindowTag::Cancelable { into, condition } => {
                w.start_s <= t
                    && t < w.end_s
                    && condition.permits(contact)
                    && into.iter().any(|entry| names.contains(&entry.as_str()))
            }
            _ => false,
        })
    }

    /// Derive this move's frame data: the startup / active / recovery /
    /// cancel windows and the strike's reach. A pure derivation from
    /// `windows` + `duration_s`, with no stored state. The fighter brain
    /// reads it to time punishes and spacing; the boss validator reads it to
    /// check telegraph/recovery budgets. All times are proper-time seconds.
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
        // A push is not a reach. [`authoring::wake`] requires the push to
        // reach further than the hit, so a union over all Active volumes would
        // report the dust's extent as the move's reach (for example
        // `goblin::dirt_kick` would read as an 82px poke whose hit stops at
        // 48px). A brain then presses the push move from a range where it
        // cannot hit, and the push holds that range open. So hittable volumes
        // and [`VolumeReaction::Windbox`] volumes are separate fields.
        let hittable = |v: &HitVolume| !matches!(v.reaction, Some(VolumeReaction::Windbox(_)));
        let active_volumes = || {
            self.windows
                .iter()
                .filter(|w| matches!(w.tag, WindowTag::Active))
                .flat_map(|w| w.volumes.iter())
        };
        let extent_x = |v: &HitVolume| v.shape.leading_edge_x();
        let box_of = |v: &HitVolume| v.shape.coverage_box();
        let union = |volumes: &mut dyn Iterator<Item = MoveCoverage>| {
            volumes.reduce(|a, b| MoveCoverage {
                min: (a.min.0.min(b.min.0), a.min.1.min(b.min.1)),
                max: (a.max.0.max(b.max.0), a.max.1.max(b.max.1)),
            })
        };
        // Reach: the farthest body-local +x extent of any hittable Active
        // volume. Zero when the move has no hittable volume.
        let reach = active_volumes()
            .filter(|v| hittable(v))
            .map(extent_x)
            .fold(0.0_f32, f32::max);
        // A capture is a reach, and it is not in `volumes`.
        // [`smash_capture::CAPTURE_ATTEMPT`] sustains on an Active window's
        // `sustain_effect`. Without this, a grab has no region, and an option
        // scorer reads `coverage: None` as "this move cannot miss". The
        // derivation is here, where the params and key are declared, so a
        // command grab on an ordinary attack verb gets the same answer as the
        // neutral grab.
        let captures = || {
            self.windows
                .iter()
                .filter(|w| matches!(w.tag, WindowTag::Active))
                .filter_map(|w| w.sustain_effect.as_ref())
                .filter(|effect| effect.key == crate::smash_capture::CAPTURE_ATTEMPT)
                .filter_map(|effect| {
                    effect
                        .params
                        .hydrate::<crate::smash_capture::CaptureAttemptParams>()
                        .ok()
                })
        };
        let capture_box = |p: &crate::smash_capture::CaptureAttemptParams| {
            let (min, max) = p.coverage();
            MoveCoverage { min, max }
        };
        let reach = reach.max(
            captures()
                .map(|p| p.reach_x())
                .fold(0.0_f32, f32::max),
        );
        let coverage = union(
            &mut active_volumes()
                .filter(|v| hittable(v))
                .map(box_of)
                .chain(captures().map(|p| capture_box(&p))),
        );
        let push_coverage = union(&mut active_volumes().filter(|v| !hittable(v)).map(box_of));
        // A shield does not stop a grab. `ignores_guard` stays a
        // caller-settable field for unblockables this code cannot recognize.
        let ignores_guard = captures().next().is_some();
        // Reach through something the move spawns: see [`hazard_of`]. Ask
        // every authored effect, one-shot events and window sustains alike,
        // because a technique may use either.
        let hazard = self
            .events
            .iter()
            .filter_map(|event| match &event.kind {
                MoveEventKind::Effect(effect) => Some(effect),
                _ => None,
            })
            .chain(self.windows.iter().filter_map(|w| w.sustain_effect.as_ref()))
            .filter_map(hazard_of)
            .chain(self.events.iter().filter_map(|event| {
                matches!(event.kind, MoveEventKind::Ranged)
                    .then_some(MoveHazard::OwnersRangedAction)
            }))
            // A `SustainedAuthority` summon is not a hazard. Recovery authority
            // answers "what movement does this give me"; hazard reach answers
            // "what can this do to them". Example: the admiral's shark is a
            // hitless mobility special, and its `reach` is travel distance.
            // A summon that also threatens must state that as an effect. The
            // travel half is read by [`RecoveryRoute::carry`] on the motion
            // road (see `brain::fighter::options::motion_options`).
            //
            // If a move has two hazards, the farthest-reaching one wins. No
            // shipped move has two; a list would be the general answer.
            .max_by(|a, b| a.reach().total_cmp(&b.reach()));
        // When the threat goes live. This is not `startup_s` for a move that
        // reaches through something it spawns: `startup_s` falls back to the
        // whole duration when there is no Active window, which is the shape
        // of every projectile move. (Example: `polygon_projectile_charge_shot`
        // fires at 0.26s but has `startup_s` 0.58s.) A consumer that leads a
        // moving target by `startup_s` then aims past it.
        //
        // Each threatening road reports its own time: an Active window opens
        // at its `start_s`, a `Ranged` trigger and a hazardous `Effect` fire
        // at their event `at_s`, and a sustained hazard is live for its
        // window. `None` means the move offers the opponent nothing (the same
        // set as `hazard.is_none() && coverage.is_none() &&
        // push_coverage.is_none()`).
        let threat_live_at_s = (coverage.is_some() || push_coverage.is_some())
            .then_some(startup_s)
            .into_iter()
            .chain(self.events.iter().filter_map(|event| match &event.kind {
                MoveEventKind::Ranged => Some(event.at_s),
                MoveEventKind::Effect(effect) => hazard_of(effect).map(|_| event.at_s),
                _ => None,
            }))
            .chain(
                self.windows
                    .iter()
                    .filter(|w| matches!(w.tag, WindowTag::Active))
                    .filter(|w| {
                        w.sustain_effect
                            .as_ref()
                            .is_some_and(|effect| hazard_of(effect).is_some())
                    })
                    .map(|w| w.start_s),
            )
            .fold(None::<f32>, |acc, t| {
                Some(acc.map_or(t, |best: f32| best.min(t)))
            });
        // Push direction. It is authored (`launch_dir`), not derived from
        // geometry, because wind blows one way whichever side the victim came
        // from. A scorer can ask whether the push sends the victim toward a
        // blast line. Body-local, `+x` toward facing, the same frame as
        // `push_coverage`. `None` when the move pushes nobody.
        let push_dir = active_volumes()
            .filter(|v| !hittable(v))
            .find_map(|v| v.launch_dir)
            .map(|(x, y)| {
                let len = (x * x + y * y).sqrt();
                if len > 0.0 {
                    (x / len, y / len)
                } else {
                    (0.0, 0.0)
                }
            });
        // Power: the strongest Active volume, derived like `reach`.
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
        // Keep the launch line, not one scalar. A `max(knockback)` fold drops
        // the growth magnitude, which decides a finisher. Example: the
        // Pugnacious Polygon's forward smash `(162, 3.25)` and up smash
        // `(158, 5.83)` cross at about 2 damage. `LaunchEnvelope::at`
        // evaluates the line against the actual opponent.
        //
        // Only hittable volumes count: a windbox moves a body and finishes
        // nobody.
        let launch = active_volumes()
            .filter(|v| hittable(v))
            .fold(LaunchEnvelope::default(), |envelope, v| {
                envelope.with_volume(v.knockback, v.knockback_growth)
            });
        // Lift: the against-gravity speed this move commands of its owner.
        //
        // A policy layer can then recognize a recovery move by its geometry,
        // not its name. `+y` is gravity-down, so lift is `-y`, and it rotates
        // with gravity because it stays in the body frame.
        //
        // Only [`ImpulseMode::Set`] counts. An additive impulse commands no
        // speed (its result depends on the body's velocity), so a jab with a
        // small upward lunge is not a recovery.
        //
        // The sideways half is kept too, because a move that pulls its owner
        // mostly sideways (a grapple line, a slingshot) is not a small rise.
        // Both halves come from the same winning event.
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
            // Ties on speed break on the earlier moment, so the answer does not
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
            ignores_guard,
            hazard,
            threat_live_at_s,
            coverage,
            push_coverage,
            push_dir,
            max_damage,
            max_knockback,
            launch,
            start_impulse: self.start_impulse.unwrap_or((0.0, 0.0)),
            lift_speed,
            lift_at_s,
            lift_side,
            // The only place the fold happens. An authored route kind wins;
            // otherwise a move that commands a rise against gravity
            // (`lift_speed > 0.0`) is a burst, and any other move is none.
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

/// A move's cancel window: the proper-time span during which the move may be
/// canceled into the named move classes/ids, under [`CancelCondition`].
/// Derived from a [`WindowTag::Cancelable`] window.
#[derive(Debug, Clone, PartialEq)]
pub struct CancelWindow {
    pub start_s: f32,
    pub end_s: f32,
    pub into: Vec<String>,
    pub condition: CancelCondition,
}

/// The body-local box a move's Active volumes cover, in the same frame the
/// volumes author themselves in: `+x` toward the owner's facing, `+y` toward its
/// feet (so an anti-air's box has a negative `min.1`).
///
/// A union, not a list: a "can this reach where they are" question needs the
/// region the volumes span. For each volume separately, read the windows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MoveCoverage {
    pub min: (f32, f32),
    pub max: (f32, f32),
}

impl MoveCoverage {
    /// How far this move reaches in one direction: the distance from the
    /// owner's origin to the far side of the box along `toward`, or `0.0` when
    /// the box does not lie that way.
    ///
    /// This generalizes [`MoveFrameData::reach`]: for a foe straight ahead of
    /// a forward volume, it equals `reach`. For a foe overhead it is how far
    /// the move reaches up, which an anti-air needs.
    ///
    /// `inflate` grows the box on every side. A hitbox catches a hurtbox, so
    /// pass the target's half-extent.
    ///
    /// A slab intersection from the origin: a box that does not span the ray
    /// returns `0.0`, not a nearest-approach value.
    pub fn extent_toward(&self, toward: (f32, f32), inflate: (f32, f32)) -> f32 {
        self.span_toward(toward, inflate).map_or(0.0, |(_, far)| far)
    }

    /// Where this move's region starts and stops along `toward`: the near and
    /// far sides of the box, or `None` when the box does not lie that way.
    ///
    /// Use the near side too, not only [`extent_toward`]. A strike box hangs
    /// out from the body (for example `pointed_polygon`'s thrust spans
    /// x ∈ [20, 76]), so `gap <= far` alone admits a foe who stands in the gap
    /// between body and box.
    ///
    /// [`extent_toward`]: Self::extent_toward
    pub fn span_toward(&self, toward: (f32, f32), inflate: (f32, f32)) -> Option<(f32, f32)> {
        let len = (toward.0 * toward.0 + toward.1 * toward.1).sqrt();
        if !(len > 0.0) {
            return None;
        }
        let (dx, dy) = (toward.0 / len, toward.1 / len);
        let (lo_x, hi_x) = (self.min.0 - inflate.0, self.max.0 + inflate.0);
        let (lo_y, hi_y) = (self.min.1 - inflate.1, self.max.1 + inflate.1);
        // Per-axis entry/exit of the ray `t · d` through each slab. A zero
        // component means the ray never leaves that slab: it is inside it for
        // all `t` or misses.
        let slab = |lo: f32, hi: f32, d: f32| -> Option<(f32, f32)> {
            if d.abs() < f32::EPSILON {
                return (lo <= 0.0 && 0.0 <= hi).then_some((f32::NEG_INFINITY, f32::INFINITY));
            }
            let (a, b) = (lo / d, hi / d);
            Some((a.min(b), a.max(b)))
        };
        let (Some((nx, fx)), Some((ny, fy))) = (slab(lo_x, hi_x, dx), slab(lo_y, hi_y, dy)) else {
            return None;
        };
        let far = fx.min(fy);
        let near = nx.max(ny);
        if near > far || far <= 0.0 {
            return None;
        }
        // If the origin is inside the box, the region opens at the origin. A
        // negative "near" would read as a gap behind the body.
        Some((near.max(0.0), far))
    }
}

/// How far into a smash's leading windup the charge pose sits, as a fraction of
/// that window.
///
/// Early on purpose: a brief windup, then the freeze. A charge must read as
/// "the swing started and stopped", not "the swing is about to land and
/// stopped".
///
/// This is a fallback, not the authoring contract. A charge pose is an
/// animation fact, so it belongs on the move as an explicit
/// `smash_charge.hold_at_s` inside its leading Startup. The shipped smash
/// tables author that, and `fighter_moveset`'s contract test refuses a smash
/// that derives its pose. This value is for a move that says nothing (a boss
/// swing, a fixture, an old table).
///
/// Do not tune it from CPU match results. Charge-pose location is an
/// animation fact; recovery outcomes in a match are emergent balance.
pub const CHARGE_POSE_FRACTION: f32 = 0.50;

/// The margin that keeps a derived charge pose strictly before the first live
/// volume, for a move whose windup is so short that the fraction lands on it.
const CHARGE_POSE_EPSILON_S: f32 = 1.0 / 240.0;

/// What a move launches for, as a function of the victim's accumulated damage.
///
/// The launch law is a line ([`launch::launch_speed`]), so one number cannot
/// hold it. This holds the two `(base, growth)` pairs it is evaluated at. A
/// base-only summary ranks a high-base/low-growth move above a
/// low-base/high-growth one at every damage, which is wrong past the crossover
/// (the Pugnacious Polygon's forward smash `(162, 3.25)` and up smash
/// `(158, 5.83)` cross at about 2 damage).
///
/// `growth_scale`, the victim's weight and rage multiply the percent term, not
/// `base`, so they move the crossover. They are arguments: [`Self::at`] takes
/// a [`launch::LaunchConditions`], which the fighter brain fills from the
/// stage's view.
///
/// `knockback_growth: None` is not a set launch. `Some(0.0)` is fixed
/// knockback; `None` means "the ruleset decides". The `Option` is kept here,
/// and [`launch::launch_speed`] resolves it once the conditions are known.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct LaunchEnvelope {
    /// The volume with the largest base: the line that wins against a fresh
    /// opponent. Its growth is `None` when that volume authors none, which
    /// means the ruleset's fallback, not a set launch.
    pub flat: (f32, Option<f32>),
    /// The `(base, growth)` of the volume with the steepest authored growth:
    /// the line that wins once the opponent is worn.
    ///
    /// A `None` volume cannot claim this line: its growth is
    /// `base * ruleset_growth`, and no ruleset is in scope at fold time. This
    /// loses a smaller `None` volume that would out-grow an authored one in
    /// the same move. No shipped move mixes the two; each `None` volume is
    /// its move's only volume, so [`Self::flat`] carries it.
    pub steep: (f32, f32),
}

impl LaunchEnvelope {
    /// Fold one Active volume's authored launch in.
    ///
    /// Keeps two lines, not all of them. The true envelope of `n` volumes is
    /// the upper hull of `n` lines. The flattest and steepest reproduce it
    /// exactly for one or two volumes and under-report a third volume that
    /// wins only in a middle band. Under-reporting is the safe direction for a
    /// kill question, and it needs no allocation in a per-frame scorer.
    pub fn with_volume(self, base: f32, growth: Option<f32>) -> Self {
        Self {
            flat: if base > self.flat.0 {
                (base, growth)
            } else {
                self.flat
            },
            // Only an authored growth competes for the steep line (see the
            // field). A `None` stays on the flat line.
            steep: match growth {
                Some(g) if g > self.steep.1 => (base, g),
                _ => self.steep,
            },
        }
    }

    /// The launch speed this move produces under `conditions`: the upper hull
    /// of its two lines, each evaluated by [`launch::launch_speed`]. It takes
    /// the full conditions because they move the crossover between lines (see
    /// the [`launch`] module doc).
    pub fn at(self, conditions: launch::LaunchConditions) -> f32 {
        launch::launch_speed(self.flat.0, self.flat.1, conditions)
            .max(launch::launch_speed(
                self.steep.0,
                Some(self.steep.1),
                conditions,
            ))
    }

    /// Does this move's launch get better as the opponent takes damage?
    ///
    /// A set launch answers `false`: a windbox goes the same distance at 0%
    /// and at 200%, and [`launch::launch_speed`] declines rage for it.
    ///
    /// It takes the conditions because the answer depends on them. A volume
    /// with `knockback_growth: None` is a set launch without a declared
    /// ruleset, and scales with percent on a stage that declares a fallback
    /// growth.
    pub fn grows_under(self, conditions: launch::LaunchConditions) -> bool {
        let resolved = |base: f32, growth: Option<f32>| {
            growth.unwrap_or_else(|| base * conditions.ruleset_growth.max(0.0))
        };
        resolved(self.flat.0, self.flat.1) > 0.0 || self.steep.1 > 0.0
    }
}

/// The queryable frame data of a move, read by the fighter brain and boss
/// validators. A pure derivation by [`MoveSpec::frame_data`] (no storage).
/// All times are the owner's proper-time seconds.
#[derive(Debug, Clone, PartialEq)]
pub struct MoveFrameData {
    /// Total move length.
    pub total_s: f32,
    /// Where on this move's timeline a smash-gesture use freezes, or `None`
    /// when the move does not charge.
    ///
    /// Not `startup_s`. The hold point is in the windup, before the first hit,
    /// so a reader that wants the charge start must read this.
    pub charge_hold_at_s: Option<f32>,
    /// Time until the first Active window opens: the tell the opponent reads.
    pub startup_s: f32,
    /// Every Active window's `(start, end)`, in declaration order.
    pub active_spans: Vec<(f32, f32)>,
    /// Time from the last Active window's end to the move's end: the punish
    /// window.
    pub recovery_s: f32,
    /// Cancel windows (`WindowTag::Cancelable`), for combo/chain reasoning.
    pub cancel_windows: Vec<CancelWindow>,
    /// Farthest body-local reach of any Active volume (`+x` toward facing).
    pub reach: f32,
    /// A guard does not stop this move.
    ///
    /// Derived for a capture ([`smash_capture::CAPTURE_ATTEMPT`] is declared
    /// here). A ruleset that authors another unblockable (armor break,
    /// command strike) sets it itself. An ordinary move keeps `false`.
    ///
    /// Genre-neutral: to a planner, unblockables, command grabs and armor
    /// breaks are the same fact: the shield is not the answer.
    pub ignores_guard: bool,
    /// What this move puts into the world that can hurt somebody. `None` for
    /// most moves.
    ///
    /// The other half of [`Self::coverage`]. A launcher has no Active volume
    /// on its owner's body, so `coverage` is `None` even though the move
    /// reaches far.
    ///
    /// A [`MoveHazard`], not a distance: the hazard's speed lets a consumer
    /// ask when it arrives, and [`MoveHazard::OwnersRangedAction`] is a
    /// request to the layer that can see the body.
    ///
    /// A key that `hazard_of` does not know answers `None`. The admission rule
    /// reads that as "this move offers the opponent nothing" and keeps the
    /// move off the attack menu. The symptom is a new projectile that is never
    /// thrown.
    pub hazard: Option<MoveHazard>,

    /// When this move first offers the opponent anything, on the move's own
    /// timeline. `None` when it offers nothing (a pure-motion move, a buff, a
    /// recovery summon).
    ///
    /// Not `startup_s`, which falls back to the whole move duration when there
    /// is no Active window (every projectile move). Leading a moving target by
    /// `startup_s` aims too far ahead. See the derivation in
    /// [`MoveSpec::frame_data`].
    ///
    /// For an ordinary strike it equals `startup_s`. The two differ only
    /// where the move reaches through something it spawns.
    pub threat_live_at_s: Option<f32>,
    /// The region this move can hit from its own body, body-local. `None`
    /// when the move cannot touch anybody from its own body.
    ///
    /// `None` covers several unrelated cases: a counter (reaches nobody until
    /// struck), a buff or taunt (reaches nobody), a launcher whose damage is a
    /// projectile (may cross the stage), and a pure-motion recovery. Do not
    /// read `None` as "this move cannot miss". Captures are included (see
    /// [`MoveSpec::frame_data`]).
    ///
    /// This is the union of the authored volumes, a 2-D datum, not a 1-D
    /// summary: the vertical game (anti-air, juggle, spike) needs to know the
    /// opponent is above. See also [`Self::lift_side`].
    pub coverage: Option<MoveCoverage>,
    /// The region this move can push without hitting: the union of its
    /// [`VolumeReaction::Windbox`] volumes. `None` for most moves.
    ///
    /// Separate from [`Self::coverage`] because "can I hit them from here" and
    /// "can I push them from here" have different answers. `wake` guarantees
    /// the push reaches further, so a merged union would overstate hit reach.
    pub push_coverage: Option<MoveCoverage>,
    /// Which way the shove blows, body-local and unit-length (`+x` toward
    /// facing, `+y` toward the feet), from the windbox volumes' authored
    /// `launch_dir`. `None` when the move shoves nobody.
    ///
    /// A push's value is signed. Coverage says the push reaches the opponent,
    /// not whether it sends them toward the blast line or saves them from it.
    pub push_dir: Option<(f32, f32)>,
    /// Highest `damage` any Active volume deals: the move's power, so an
    /// option scorer can price a smash above a jab. `0` for a move with no
    /// volume.
    ///
    /// Only the move's own volumes. A scorer should use
    /// [`Self::strongest_hit`], because a launcher's damage is in what it
    /// spawns. This field stays volume-only because the fighter rollout
    /// applies it when the foe is inside [`Self::reach`], where a volume
    /// lands.
    pub max_damage: i32,
    /// Highest flat `knockback` any Active volume applies (the `knockback_growth`
    /// percent-scaling term is the victim's business, not the table's).
    pub max_knockback: f32,
    /// What this move launches for, against an opponent at a given damage.
    ///
    /// A kill question reads this. It is a line because the launch law is
    /// `base + growth * damage`: a set launch never improves, and the largest
    /// `base` is often not the largest launch on a worn opponent.
    pub launch: LaunchEnvelope,
    /// The move's authored self-motion at trigger, body-local (`+x` toward
    /// facing, `+y` per the authoring convention). `(0, 0)` when none.
    pub start_impulse: (f32, f32),
    /// The against-gravity speed this move commands, from its strongest
    /// [`ImpulseMode::Set`] impulse. `0.0` for moves that do not lift their
    /// owner.
    ///
    /// A recovery policy reads this. A move is a recovery because of what it
    /// does to the body, so a brain, a validator and a recovery probe all use
    /// the same number and need no table of each character's Up-B.
    pub lift_speed: f32,
    /// When [`Self::lift_speed`] arrives, proper-time seconds from move start:
    /// the windup a body must survive before the burst. `0.0` when there is no
    /// lift.
    pub lift_at_s: f32,
    /// Along-facing component of the commanded lift velocity, body-local
    /// (`+x` toward facing). Together with [`Self::lift_speed`] this is the
    /// complete 2-D velocity-shaped recovery proposal.
    pub lift_side: f32,
    /// The resolved way home this move offers: a burst from the fields above,
    /// or the author's stated route.
    ///
    /// One answer, so a planner does not correlate `lift_speed` with a gate.
    /// The fields above describe a burst; this says which kind of route it is.
    pub recovery_route: RecoveryRoute,
}

impl MoveFrameData {
    /// The most one use of this move can take off somebody: its own volumes
    /// or what it puts in the world, whichever is larger.
    ///
    /// An option scorer must use this, not [`Self::max_damage`], which prices
    /// every projectile at zero.
    ///
    /// A `max`, not a sum: a move that swings and throws connects with one of
    /// them on a given body, and the payoff prices one press.
    ///
    /// An unresolved [`MoveHazard::OwnersRangedAction`] contributes nothing,
    /// because its damage is on the body. The kit builder joins the two and
    /// replaces the variant.
    pub fn strongest_hit(&self) -> i32 {
        self.max_damage
            .max(self.hazard.map_or(0, MoveHazard::damage))
    }
}

// ---------------------------------------------------------------------------
// Entities: contract bundles.
// ---------------------------------------------------------------------------

/// Physics body contract: entity-local collision half-extents.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Body2dContract {
    pub half_extents: (f32, f32),
}

/// Presentation contract: which visual this entity binds to. `visual_id` is
/// the packer/sheet target name resolved through the sprite pack (or the
/// per-target sheet compatibility path).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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

/// The running-stance verb for an attack base. This is the only place the
/// suffix is spelled. Named for the "dash attack", keyed off the body's gait
/// and not off `AbilitySet::dash`.
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

/// The base verb a composed verb id was built from: the inverse of
/// [`directional_verb_chain`] and [`dash_stance_verb`].
///
/// It lives beside the composers because it inverts the same suffix table. Do
/// not split on the first underscore; a base verb may contain one.
///
/// `attack_air_forward` → `attack`, `smash_dash` → `smash`, `special` →
/// `special`. An id with no known suffix is its own base.
pub fn base_verb_of(verb: &str) -> &str {
    // Longest first: `attack_air_forward` must not reduce to `attack_air` by
    // matching `_forward`.
    const SUFFIXES: [&str; 10] = [
        "_air_forward",
        "_air_back",
        "_air_up",
        "_air_down",
        "_air",
        "_dash",
        "_forward",
        "_back",
        "_up",
        "_down",
    ];
    SUFFIXES
        .iter()
        .find_map(|suffix| verb.strip_suffix(suffix))
        .unwrap_or(verb)
}

/// The cancel namespace a move answers to when it is reached by `base`.
///
/// This is the only place the list lives. A `Cancelable` window names verbs
/// and classes as well as move ids; the trigger road and any cancel-graph
/// exporter both read this.
///
/// A running attack answers to the attack family whatever gesture asked for
/// it: the namespace follows the move that ran, not the button.
///
/// The empty answer means "its own full name only". Grabs, captures, taunts
/// and `ranged` each pass exactly one name on the trigger road. Putting them
/// in the attack family would let `any_attack` cancel into a throw.
pub fn cancel_names_for(base: &str, running_attack: bool) -> &'static [&'static str] {
    match base {
        SPECIAL_VERB => &[SPECIAL_VERB],
        SMASH_VERB if !running_attack => &[SMASH_VERB, ATTACK_VERB, "any_attack"],
        ATTACK_VERB | SMASH_VERB => &[ATTACK_VERB, "any_attack"],
        // Each of these passes exactly one name on its own arm of the trigger
        // road. `grab_dash` reduces to `grab`, which is correct (the grab road
        // takes a running stance). A capture verb does not reduce, so it falls
        // to the empty answer below.
        GRAB_VERB => &[GRAB_VERB],
        RANGED_VERB => &[RANGED_VERB],
        TAUNT_VERB => &[TAUNT_VERB],
        // Empty means "its own full name", not "the attack family". The
        // capture road passes `capture_throw_forward` verbatim, and
        // `base_verb_of` would reduce that to `capture_throw`, which names
        // nothing.
        _ => &[],
    }
}

/// Which reusable character template an actor instantiates.
///
/// A character is an authored template, not a singleton person: `spawn Goblin`
/// three times and `spawn Fretjaw` twice are the same engine operation, one
/// definition and many runtime actors. This id names the definition; the actor's
/// runtime identity is its `SimId`, and the two are never the same question.
///
/// A newtype so the template/instance confusion cannot survive a signature
/// (the same reason `BrainPresetId` exists).
///
/// It lives beside the placement schemas because authoring, the character
/// domain and the runtime all name it. `#[serde(transparent)]` keeps the
/// authored encoding a bare string.
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

/// Lets a `BTreeMap<CharacterId, _>` be looked up by `&str`. This is sound
/// because `Ord`/`Eq`/`Hash` on `CharacterId` delegate to the same `String`,
/// so borrowed and owned comparisons agree.
impl std::borrow::Borrow<str> for CharacterId {
    fn borrow(&self) -> &str {
        &self.0
    }
}

/// The canonical verb id a body's basic melee swing binds to in its moveset.
///
/// The verb ids live beside the moveset contract they key into, not in the
/// runtime that plays moves, so a `CharacterDefinition` can name its verbs
/// without depending on the runtime crate. Re-exported from
/// `ambition_platformer2d::combat::moveset`.

pub const ATTACK_VERB: &str = "attack";
/// Strong directional attacks use the same authored verb machinery under the
/// distinct `smash` base. A moveset that authors no smash verb falls back to its
/// ordinary attack repertoire.
pub const SMASH_VERB: &str = "smash";
/// The canonical verb id a body's ranged shot binds to in its moveset.
pub const RANGED_VERB: &str = "ranged";

/// The melee verb family: the attack and smash bases and every variant of them
/// (directional, aerial, dash).
pub fn is_melee_verb(verb: &str) -> bool {
    verb == ATTACK_VERB
        || verb.starts_with("attack_")
        || verb == SMASH_VERB
        || verb.starts_with("smash_")
}

/// The ranged verb family: the base and any variant of it.
pub fn is_ranged_verb(verb: &str) -> bool {
    verb == RANGED_VERB || verb.starts_with("ranged_")
}
/// The canonical verb id a body's signature special binds to in its moveset.
/// `special_pressed` resolves the facing-relative directional `special` chain;
/// a body only has a real special when its moveset authors a matching
/// directional verb or the base verb.
pub const SPECIAL_VERB: &str = "special";
/// The canonical verb id a body's taunt binds to, and the base of its
/// directional chain. Unlike the verbs above, it is not a threat, so a body
/// needs no permission to carry it.
pub const TAUNT_VERB: &str = "taunt";
/// The capture verbs. The grab that establishes a hold, and the moves a
/// captor selects while one exists.
///
/// This crate holds the verb names a press can resolve to; content holds what
/// each one does. When a Smash capability owns its own schema (a
/// character-owned `smash.fighter` facet), these move with it. Until then,
/// one definition here is better than strings copied into a selector and an
/// authoring module.
pub const GRAB_VERB: &str = "grab";
/// The running grab: the standing reach-out performed from a run. Unlike the
/// dash attack (a separate move each fighter authors), it is derived from the
/// fighter's own standing grab. Spelled here because
/// [`MovesetContract::move_for_flat_verb`] needs a `&'static str`;
/// `a_running_grabs_verb_is_the_dash_stance_of_the_grab` pins it to
/// [`dash_stance_verb`].
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
/// which move. Re-binding an existing move onto a different actor is a data
/// edit here.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MovesetContract {
    /// Input verb → move id (e.g. `"attack" → "sandbag_swat"`). BTreeMap for
    /// deterministic iteration (query-order discipline).
    #[serde(default)]
    pub verbs: BTreeMap<String, String>,
    pub moves: Vec<MoveSpec>,
}

impl MovesetContract {
    /// The moves a `Cancelable` window's `into` list actually admits.
    ///
    /// An authored list such as `["attack", "smash", "any_attack"]` resolves
    /// to moves only against this table, because the answer is the
    /// character's own repertoire. Do not resolve it elsewhere.
    ///
    /// The rule is the trigger road's: a candidate is admitted when the list
    /// names its move id, or any verb-class name it answers to
    /// ([`cancel_names_for`], keyed by the base of every verb that binds it).
    ///
    /// Timing and condition are not checked; see
    /// [`MoveSpec::cancel_permits`]. This answers only "who could this list
    /// mean".
    pub fn cancel_targets(&self, into: &[String]) -> Vec<&MoveSpec> {
        self.moves
            .iter()
            .filter(|candidate| {
                if into.iter().any(|entry| entry == &candidate.id) {
                    return true;
                }
                // Every verb that binds this move, reduced to its base: a move
                // bound as both `attack_forward` and `smash_forward` answers
                // to both namespaces.
                self.verbs
                    .iter()
                    .filter(|(_, target)| *target == &candidate.id)
                    .flat_map(|(verb, _)| {
                        let family = cancel_names_for(base_verb_of(verb), false);
                        if family.is_empty() {
                            // No family means its own full name (not a reduced
                            // base): `capture_throw_forward` answers to
                            // `capture_throw_forward`.
                            vec![verb.as_str()]
                        } else {
                            family.to_vec()
                        }
                    })
                    .any(|name| into.iter().any(|entry| entry == name))
            })
            .collect()
    }

    /// Rename every move this table defines, and every reference to one.
    ///
    /// The traversal belongs to the schema. A move id appears in several
    /// places: `moves[].id`, `verbs`, a `Cancelable` window's `into` list,
    /// and `gates.when_refused`. A new field that carries a move id must be
    /// handled here.
    ///
    /// A verb class is not a move id. `"attack"`, `"any_attack"` and the other
    /// classes are not changed: only names this table defines are renamed, so
    /// the old→new map is built first.
    ///
    /// `rename` is called only for moves this table owns, so a caller may
    /// panic on an id it does not recognize.
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
            // The refusal fallback carries a move id too. A cloned fighter
            // renames every move, and a fallback left on the original id would
            // resolve to the other copy's move or, silently, to nothing.
            //
            // `get` only, like the cancel targets: an id this table does not
            // own is left as written.
            if let Some(target) = mv.gates.when_refused.as_mut() {
                if let Some(new) = by_old.get(target.as_str()) {
                    *target = new.clone();
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

    /// This table under another fighter's name.
    ///
    /// Each move id's longest matching prefix is replaced by `owner`. The
    /// borrowed table keeps the archetype's frame data and gets the borrower's
    /// move ids, because a move id is what a causal log, a cue table and a
    /// cancel window name: two fighters answering to one id cannot be told
    /// apart. Longest prefix first, so `polygon` cannot claim a
    /// `polygon_brawler` id.
    ///
    /// An id that carries none of the prefixes is an error, and so is a rename
    /// that makes two moves one: renaming only some moves would leave two
    /// fighters sharing a name.
    pub fn under_own_name(mut self, prefixes: &[String], owner: &str) -> Result<Self, String> {
        let mut sorted: Vec<&str> = prefixes.iter().map(String::as_str).collect();
        sorted.sort_by_key(|prefix| std::cmp::Reverse(prefix.len()));
        let renamed = |id: &str| -> Option<String> {
            sorted
                .iter()
                .find_map(|prefix| id.strip_prefix(prefix).map(|rest| format!("{owner}{rest}")))
        };
        let stray = self
            .moves
            .iter()
            .map(|mv| mv.id.as_str())
            .chain(self.verbs.values().map(String::as_str))
            .find(|id| renamed(id).is_none());
        if let Some(stray) = stray {
            return Err(format!(
                "move id `{stray}` carries none of the prefixes {prefixes:?}, so it \
                 cannot be renamed for `{owner}`"
            ));
        }
        self.remap_move_ids(|id| renamed(id).unwrap_or_else(|| id.to_string()));
        let distinct: std::collections::BTreeSet<&str> =
            self.moves.iter().map(|mv| mv.id.as_str()).collect();
        if distinct.len() != self.moves.len() {
            return Err(format!(
                "renaming the table for `{owner}` with prefixes {prefixes:?} makes two \
                 moves one"
            ));
        }
        Ok(self)
    }

    /// This table with a borrower's own table laid over it.
    ///
    /// A move whose id this table already has replaces it where it stands. A
    /// new move is added at the end, in the order `own` lists it. Each verb
    /// `own` binds is rebound, and a move that the rebinding leaves bound to no
    /// verb is dropped, because the borrower replaced it.
    pub fn overlaid_with(mut self, own: &MovesetContract) -> Self {
        for mv in &own.moves {
            match self.moves.iter_mut().find(|slot| slot.id == mv.id) {
                Some(slot) => *slot = mv.clone(),
                None => self.moves.push(mv.clone()),
            }
        }
        for (verb, target) in &own.verbs {
            let Some(displaced) = self.verbs.insert(verb.clone(), target.clone()) else {
                continue;
            };
            if displaced != *target && !self.verbs.values().any(|id| *id == displaced) {
                self.moves.retain(|mv| mv.id != displaced);
            }
        }
        self
    }

    pub fn move_by_id(&self, id: &str) -> Option<&MoveSpec> {
        self.moves.iter().find(|m| m.id == id)
    }

    /// Resolve an input verb to its move, ignoring its gates.
    ///
    /// This answers "does this fighter author the verb at all" (a display row
    /// or an authorship check). A selector that decides what a press starts
    /// must use [`Self::move_for_verb_in_stance`].
    pub fn move_for_verb(&self, verb: &str) -> Option<&MoveSpec> {
        self.move_by_id(self.verbs.get(verb)?)
    }

    /// The move an input verb names, when the body's stance permits it.
    ///
    /// Use this gated lookup in a selector, like
    /// [`Self::move_for_flat_verb`] and [`Self::move_for_directional_verb`].
    /// A bare `move_for_verb` lets an airborne press start a grounded-only
    /// move (for example a grab).
    pub fn move_for_verb_in_stance(&self, verb: &str, grounded: bool) -> Option<&MoveSpec> {
        self.move_for_verb(verb)
            .filter(|mv| mv.gates.permits(grounded))
    }

    /// What an attack press produces, stance included. The dash attack is a
    /// stance, not a direction, so it is checked before the directional chain
    /// and not added to [`AttackDir`].
    ///
    /// Other verbs (special, smash, taunt) have no dash stance. A fighter that
    /// authors no `{base}_dash` resolves through
    /// [`Self::move_for_directional_verb`] only.
    ///
    /// Grab also has a running stance, but it goes through
    /// [`Self::move_for_flat_verb`] because the capture kit is flat.
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

    /// A flat verb's move, preferring its running-stance variant. The sibling
    /// of [`Self::move_for_attack`] for a verb with no directional family (the
    /// capture kit: a throw is not `grab_forward`).
    ///
    /// Only when the variant is bound, as for the dash attack: a contract
    /// without `{base}_dash` resolves its press to `base`.
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
        // Gated, like the running variant and the directional chain. A
        // standing move whose gates refuse this stance is not an answer.
        self.move_for_verb_in_stance(base, grounded)
    }

    /// Resolve a directional attack to its move: the first verb in the
    /// most-specific → least-specific chain ([`directional_verb_chain`]) that
    /// is authored and whose gates permit the grounded state (a grounded-only
    /// `attack_down` is skipped for an airborne body, falling through to
    /// `attack`). A moveset that authors only `base` answers every direction.
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
#[serde(deny_unknown_fields)]
pub struct EntityContracts {
    #[serde(default)]
    pub body: Option<Body2dContract>,
    /// Simulation-authored damageable body shapes. Absent means the runtime may
    /// use its visible sprite-bounds compatibility fallback.
    #[serde(default)]
    pub hurtboxes: Option<HurtboxDoc>,
    #[serde(default)]
    pub presentation: Option<PresentationContract>,
    /// Another entity's move table, borrowed under this entity's name.
    /// [`Self::moveset`] is then laid over it (see
    /// [`MovesetContract::overlaid_with`]) rather than standing alone. It is
    /// declared before the table so a file states what it borrows first.
    #[serde(default)]
    pub borrows: Option<MovesetBorrow>,
    #[serde(default)]
    pub moveset: Option<MovesetContract>,
}

/// A move table borrowed from another entity under the borrower's own name.
///
/// A fighter drawn on an archetype's rig plays the archetype's timings. A
/// copy of the table would drift when the archetype is tuned, so the borrower
/// names the archetype and states only what it changes.
///
/// The borrowed moves take the borrower's name and are not shared verbatim.
/// A move id is what a causal log attributes a hit to, what a cue table
/// addresses and what a cancel window names: two fighters that answer to
/// `polygon_jab` cannot be told apart in a trace.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MovesetBorrow {
    /// The entity whose move table is borrowed. It authors its own table.
    pub archetype: String,
    /// The prefixes the archetype's move ids carry. See
    /// [`MovesetContract::under_own_name`].
    pub prefixes: Vec<String>,
}

/// One catalog entity: a stable id plus its contract bundle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EntityDef {
    pub id: String,
    pub contracts: EntityContracts,
}

/// The `schema_version` an authored entity-catalog document must declare.
///
/// Readers must compare it. `ambition_characters::moveset_content_schema`
/// refuses a document from another version and does not read its fields as
/// the current shape.
///
/// It lives with the document, so a schema handler, an exporter and a test do
/// not each spell the number.
pub const ENTITY_CATALOG_SCHEMA_VERSION: u32 = 1;

/// An authored entity-catalog document (one or many entities).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
    /// A zero-width window (`start_s == end_s`) is legal: a move with no
    /// windup still authors its Startup phase, and the phase readers
    /// (`phase_at`, the synthesized read-model swing) need the window to keep
    /// the timeline three-phase. `simple_melee` with `windup_s: 0.0` emits
    /// one. Every window predicate is the half-open `start_s <= t < end_s`,
    /// so nothing fires inside it.
    WindowOutOfRange {
        entity: String,
        mv: String,
        index: usize,
    },
    /// An authored smash charge freezes the timeline where a strike is already
    /// live, or outside the move's leading windup.
    ///
    /// `rooted_by_charge` is true from the freeze onward and the button may
    /// hold it indefinitely. A hold at or past the first Active instant leaves
    /// a fighter standing still with a live hitbox. Active membership is
    /// `start_s <= t < end_s`, so "at" is already inside.
    ChargeHoldOutsideWindup {
        entity: String,
        mv: String,
        hold_at_s: f32,
        first_active_s: f32,
    },
    /// A windbox volume authors damage, which its contract forbids
    /// (`VolumeReaction::Windbox` pushes and does nothing else).
    ///
    /// Rejected, not silently zeroed: discarding a typed number turns a
    /// content error into a mystery. The error names the move and window.
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
    /// (A typo-level error, but structural: an empty clip name is a typo.)
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

    /// Structural validation. An empty result means the catalog is sound.
    /// Filesystem-free: clip bindings are checked for shape here. Whether a
    /// clip resolves in the bound visual is the publish-time validator's job.
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
                                && !cancel_class_names().contains(&target.as_str())
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
                // Check only an authored policy. The derived one is clamped
                // before the first Active instant by `derived_charge_hold_at_s`.
                // Authoring overrides that clamp, so this refuses a bad
                // override.
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

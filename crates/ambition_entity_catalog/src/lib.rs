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

pub mod placements;

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
    /// `knockback + kb_growth * victim.damage_taken() / victim.weight`. Default
    /// `0.0` == today's flat knockback exactly (parity by construction); content
    /// opts a row into growth to get percent-scaling launches.
    #[serde(default)]
    pub kb_growth: f32,
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
    /// the `effect` id against the content-registered cosmetic vocabulary
    /// (`ambition_vfx::move_vfx_kind`) and spawns the burst at the owner. A typo
    /// is a startup validation error (`MoveSpec::presentation_problems`), never
    /// a silent no-op. This is how a jab, a smash, and a launcher look distinct
    /// with zero code — each authors its own `Vfx { effect }`.
    Vfx { effect: String },
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
    /// Smash-charge payoff (CM3): the multiplier a FULLY-charged release applies
    /// to this move's damage and knockback. The applied scale interpolates
    /// `1.0 → smash_charge_mult` by the charge fraction reached at release (how
    /// far the owner's clock advanced through the leading Startup window). DEFAULT
    /// `1.0` = no charge scaling (every non-charge move, and Ambition's charge
    /// moves until a game opts in) — byte-parity. A smash roster authors e.g.
    /// `2.0` so a held smash lands twice as hard as a tap.
    #[serde(default = "default_charge_mult")]
    pub smash_charge_mult: f32,
}

/// Serde default for [`MoveSpec::smash_charge_mult`]: the multiplicative
/// identity, so every existing move is unscaled (parity).
fn default_charge_mult() -> f32 {
    1.0
}

/// Serde default for [`MoveWindow::motion_scale`]: the multiplicative
/// identity, so every existing window leaves steering untouched (parity).
fn default_motion_scale() -> f32 {
    1.0
}

impl MoveSpec {
    /// CM5: validate this move's PRESENTATION event ids so a typo fails loudly
    /// at load, never as a silent missing sound/effect. `vfx_known` is the
    /// injected cosmetic-vfx vocabulary oracle (this crate does not depend on
    /// `ambition_vfx`, so gameplay_core passes `|id| move_vfx_kind(id).is_some()`
    /// at expansion time). Returns one human-readable problem per bad id:
    /// - a `Vfx { effect }` whose id is not in the cosmetic vocabulary, and
    /// - a `Sfx { cue }` with an empty cue (a blank cue resolves to silence).
    /// Empty result = the move's presentation is resolvable.
    pub fn presentation_problems(&self, vfx_known: impl Fn(&str) -> bool) -> Vec<String> {
        let mut problems = Vec::new();
        for ev in &self.events {
            match &ev.kind {
                MoveEventKind::Vfx { effect } if !vfx_known(effect) => {
                    problems.push(format!(
                        "move '{}': Vfx event names unknown cosmetic effect '{}' (not in \
                         the move_vfx_kind vocabulary)",
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
        MoveFrameData {
            total_s: self.duration_s,
            startup_s,
            active_spans,
            recovery_s,
            cancel_windows,
            reach,
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
    pub reach: f32,
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

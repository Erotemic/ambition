//! The boss-profile AUTHORING VOCABULARY — the types `boss_profiles.ron` is
//! deserialized into.
//!
//! ## Why this is here and not in the actor crate
//!
//! A schema must be registered by the crate that owns its type, and the validator has to link that
//! crate to install the schema — so a boss-profile schema meant the CLI linking the monolith: **708
//! crates against the validator's 239, and a renderer**. That would have destroyed the one property
//! justifying the compiler (build in seconds, validate in milliseconds).
//!
//! Nothing here ever needed the actor crate. Every field resolves against
//! `ambition_platformer2d_core`, `ambition_entity_catalog`, and this crate's own
//! boss-pattern vocabulary; the coupling was locational, not real. What DID need
//! the actor crate — the `BossCatalog` lookups, `ActorSpriteMetrics`, the
//! animation-key table — stayed there, and the lookups became
//! `BossBehaviorProfileExt` because the orphan rule does not let an inherent
//! `impl` follow a type across a crate boundary.
//!
//! ⚠ `PickupKind` moved DOWN to `ambition_entity_catalog` in the same change:
//! `BossRewardProfile` names it, and `ambition_interaction` (its old home)
//! depends on THIS crate, so naming it from here was a dependency cycle.

use super::{BossAttackPattern, BossAttackProfile, BossMovementProfile};
use crate::actor::limb::LimbSlot;
use ambition_platformer2d_core as ae;

/// `serde`-ready so a content boss can eventually AUTHOR its strike geometry (in the boss
/// roster RON) instead of a core enum variant — the "second game adds a boss without editing
/// core" oracle. Today the built-in per-profile tables below supply it; an authored override is
/// the next slice.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StrikeRect {
    /// Center offset from the strike origin, as a fraction of the body size.
    pub offset_factor: ae::Vec2,
    #[serde(default)]
    pub offset_const: ae::Vec2,
    /// Half-extent, as a fraction of the body size.
    pub half_factor: ae::Vec2,
    #[serde(default)]
    pub half_const: ae::Vec2,
}

impl StrikeRect {
    /// A rect whose center offset and half-extent are PURE fractions of the body
    /// size (no fixed-pixel term) — the common case for every profile but FloorSlam.
    pub const fn scaled(offset_factor: ae::Vec2, half_factor: ae::Vec2) -> Self {
        Self {
            offset_factor,
            offset_const: ae::Vec2::ZERO,
            half_factor,
            half_const: ae::Vec2::ZERO,
        }
    }

    /// Resolve this data rect to a world-space AABB for a body of `size` whose strike
    /// origin is `origin`.
    pub fn to_aabb(&self, origin: ae::Vec2, size: ae::Vec2) -> ae::Aabb {
        ae::Aabb::new(
            origin + self.offset_factor * size + self.offset_const,
            self.half_factor * size + self.half_const,
        )
    }
}

/// Live sandbox-side behavior tuning for a boss. This is deliberately separate
/// from the actor crate's `BossEncounterSpec`: the engine spec owns phase progression and HP
/// thresholds, while this profile owns sandbox movement, contact size, damage,
/// and hitbox shapes.
///
/// Every field here is authored in a provider's `boss_profiles.ron` fragment.
/// [`BossProfileRegistry`] remains a pure parser for focused tests and tools;
/// production providers assemble those rows into the App-local
/// `BossCatalog`. Adding a new boss is a new key + row in provider
/// data when it needs custom behavior; unknown authored bosses fall back
/// to the generic profile.
#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BossBehaviorProfile {
    pub id: String,
    /// Sprite-registry target id: which sheet the boss draws from and keys its
    /// per-animation hit/hurtboxes against. `None` (the default) = the boss's
    /// own `id` IS the target (the common case). Author it only when the sheet
    /// target diverges from the id — the gradient sentinel / clockwork warden
    /// share the generic `"boss"` sheet, GNU-ton draws `"gnu_ton_boss"`, the
    /// mockingbird `"mockingbird_boss"`. Data-driven so the engine's sprite
    /// lookup names no boss (was a hardcoded id->target match in `sync.rs`).
    #[serde(default)]
    pub sprite_target: Option<String>,
    #[serde(default, with = "boss_vec2_option")]
    pub combat_size: Option<ae::Vec2>,
    pub movement: BossMovementProfile,
    /// Optional per-phase movement overrides. `None` means "use
    /// `movement` during this phase." Lets a boss escalate its
    /// movement personality across phases without changing the
    /// profile enum itself.
    #[serde(default)]
    pub movement_phase2: Option<BossMovementProfile>,
    #[serde(default)]
    pub movement_enrage: Option<BossMovementProfile>,
    /// Multiplier applied to movement speed while a strike is committed.
    /// `< 1.0` keeps the boss roughly anchored so World-space special hitboxes
    /// (saddle cross, minima pit) don't slide out from under the visual
    /// telegraph, and so a melee FollowOwner strike can actually land. `1.0`
    /// leaves steering untouched. CONSUMED at moveset-bake time
    /// (`boss_attack_moveset`): it becomes each strike move's Active-window
    /// `MoveWindow::motion_scale`, the
    /// move's authored motion lock, enforced body-side at integration for any
    /// controller of the body.
    pub strike_speed_scale: f32,
    /// Optional self-dodge: while a strike is committed, the boss side-steps
    /// with the authored `(amplitude_px, frequency_hz)` so it can weave out of
    /// its OWN attack — GNU-ton dodges its apple rain. `None` (the default when
    /// the RON row omits it) = the boss holds its ground during strikes.
    /// Data-driven by design: the engine spawn path reads this instead of
    /// naming a specific boss, so the dependency points content -> core.
    #[serde(default)]
    pub self_dodge: Option<(f32, f32)>,
    /// Macro state machine tuning — when enabled, the boss runs an
    /// Engage / Approach / Retreat dance on top of the scripted
    /// attack schedule. See [`crate::brain::BossMacroTuning`].
    /// Use `BossMacroTuning::disabled()` for legacy "stand and
    /// fight" behavior.
    pub macro_tuning: crate::brain::BossMacroTuning,
    pub attacks: Vec<BossAttackProfile>,
    pub attack_cooldown: f32,
    pub attack_windup: f32,
    pub attack_active: f32,
    pub attack_damage: i32,
    pub body_damage: i32,
    /// How attack hitboxes are selected. `Cycle` (default for legacy bosses)
    /// rotates through `attacks` using the flat durations above. `Scripted`
    /// runs an authored phase-keyed timeline of telegraph / strike / rest
    /// beats and ignores `attacks` / `attack_cooldown` / `attack_windup` /
    /// `attack_active`.
    pub attack_pattern: BossAttackPattern,
    /// World-space anchor offset (in pixels) from the boss center where
    /// "hand"-class attacks should originate. For body-centered giants
    /// (GNU-ton) the entity transform sits at the scholar on the shoulder,
    /// not the giant's body — without this offset, hand hitboxes would
    /// hover near the scholar instead of where the giant's arms are. Y is
    /// world-space positive-down; leave at `Vec2::ZERO` for ordinary bosses.
    #[serde(default, with = "boss_vec2_required")]
    pub attack_origin_offset: ae::Vec2,
    /// World-space anchor offset (in pixels) from the boss center for
    /// projectile-like specials. Smirking Behemoth uses this to fire
    /// MemorizedVolley eye beams from its eye instead of its body center.
    #[serde(default, with = "boss_vec2_required")]
    pub projectile_origin_offset: ae::Vec2,
    /// Authored post-defeat reward. `None` (the default when the RON
    /// row omits `reward:`) means the boss drops nothing; `DropChest`
    /// spawns a reward chest at the given offset/size on defeat. This
    /// rides on the behavior profile so the whole boss is authored in
    /// one RON row.
    #[serde(default)]
    pub reward: BossRewardProfile,
    /// Catalog ability this boss grants on defeat (`"blink"`, `"fireball"`, …),
    /// dropped as a collectible pickup — "every boss a failed objective
    /// function; defeating it teaches its theorem." `None` (the default) = no
    /// ability drop. Content data: each boss embodies an ability, authored in
    /// `boss_profiles.ron`, so the engine's drop logic names no boss.
    #[serde(default)]
    pub reward_ability: Option<String>,
    /// The boss's signature wielded gauntlet (a held-item id like `"shockwave"`
    /// / `"meteor"`), dropped as a ground item the player picks up to wield the
    /// boss's own attack. `None` (the default) = no gauntlet drop. Content data
    /// (authored in `boss_profiles.ron`), distinct from [`Self::reward_ability`]
    /// which grants a catalog ability.
    #[serde(default)]
    pub signature_gauntlet: Option<String>,
    /// When `true`, ordinary player hits (slash / projectile) never damage
    /// this boss — they only give honest local impact feedback when they
    /// overlap the body hurtbox. The only kill condition is an environmental
    /// rule authored elsewhere (e.g. the Smirking Behemoth's LDtk-authored
    /// rope/anvil trap in the content crate's `bosses::cut_rope`).
    ///
    /// Data-driven by design: core asks the boss's *data* whether it is
    /// invulnerable to ordinary hits, instead of naming a specific boss, so the
    /// dependency points content -> core, never the reverse.
    #[serde(default)]
    pub environmental_kill_only: bool,
    /// Where a combat-banter speech bubble anchors relative to the
    /// boss: `pos + (dx_px, dy_half_h * combat_half_height + dy_px)`.
    /// The default hangs the bubble just above the body; multi-part
    /// bosses (GNU-ton's shoulder scholar) author their own.
    #[serde(default)]
    pub bark_anchor: BarkAnchorSpec,
    /// Authored strike-geometry OVERRIDES, keyed by the attack's `move_id`
    /// (`"floor_slam"`, `"hand_sweep"`, …, or a `Special` key). When a profile's
    /// move_id is present here, its [`StrikeRect`] list REPLACES the built-in
    /// the built-in `strike_geometry` table for that
    /// strike — so a boss (a second game's especially) authors its OWN strike rects
    /// in `boss_profiles.ron` with NO edit to core's geometry table. Empty (the
    /// `#[serde(default)]`) = use the built-in per-profile geometry, unchanged. The
    /// "second game adds a boss without editing core" oracle, for strike shapes.
    #[serde(default)]
    /// ⛔ **`BTreeMap`, and the ordering is load-bearing.** This is part of the
    /// profile's CANONICAL form, and canonicalization is derived `Debug`, which
    /// follows iteration order. A `HashMap` randomises that per instance — six
    /// constructions of the same four-key map gave six different orders in one
    /// process — so two identical rosters produced two different pack
    /// fingerprints as soon as any boss authored a second strike override.
    /// The same rule ADR 0023 already states for
    /// every ordered read.
    pub strike_geometry: std::collections::BTreeMap<String, Vec<StrikeRect>>,
    /// ADR 0020: mount classes a boss authored as a would-be RIDER may pilot. A
    /// boss that rides a mount (GNU-ton the scholar aboard the `giant_gnu` mount)
    /// authors e.g. `["giant"]`; `spawn_boss` then attaches a [`CanPilot`] tag —
    /// the SAME mount-role the enemy path attaches in `attach_mount_role`, so the
    /// boss and enemy spawn paths stay symmetric. Empty (the default) ⇒ the boss
    /// pilots nothing (every boss today). The `RidingOn`/`MountSlot` link itself is
    /// installed later from the room's authored `mounted_on` refs.
    ///
    /// `CanPilot` lives in the actor integration crate.
    #[serde(default)]
    pub pilotable_mount_classes: Vec<String>,
    /// Q18 (G3): the profile→limb routing seam. Keyed by the strike's move id
    /// (`"hand_slam"`, `"hand_sweep"`, …), each entry names which of the mount's
    /// limb slots a strike drives and how ([`LimbRoute`]). When this boss is a
    /// RIDER whose linked mount carries a `LimbRig`,
    /// `route_boss_strikes_to_limbs` turns the ACTIVE strike's route into per-limb
    /// `velocity_target` arcs + a `melee_pressed` edge, written onto the mount's
    /// `LimbIntents`. A strike move id NOT present here stays a
    /// host-body strike (no limb intent) — exactly as today. Empty (the
    /// `#[serde(default)]`) ⇒ no strike drives limbs (every boss but the gnu-ton
    /// rider). Authored in `boss_profiles.ron`, so a second game's mounted boss
    /// wires its own limbs with no edit to core.
    #[serde(default)]
    pub limb_routing: Vec<(String, LimbRoute)>,
    /// G5 (R10.6): the POSSESSED-VERB map — controller verb → this boss's move
    /// key, consulted when a human possesses the boss (`DrivingParticipant`). The
    /// possession arm reduces the controller's aim to a directional attack verb
    /// through the SAME chain every actor melee resolves
    /// ([`ambition_entity_catalog::directional_verb_chain`]: `attack_down` →
    /// `attack`), then looks the winning verb up HERE; `"special"` maps the
    /// special button. Combined with [`Self::limb_routing`], this is the
    /// controller→limb map: possess GNU-ton, aim down + attack → `hand_slam` →
    /// both giant hands slam. Empty (the `#[serde(default)]`) ⇒ the legacy
    /// possession mapping (primary strike / signature special) — every boss
    /// today except the gnu-ton rider. Authored in `boss_profiles.ron`; verbs
    /// are data, so a second game's possessable boss maps its own controls with
    /// no edit to core.
    #[serde(default)]
    pub possessed_verbs: Vec<(String, String)>,
}

/// Q18 (G3): one motion verb the limb router turns into a `velocity_target` arc
/// across a strike's Startup/Active phases. A tiny closed set — anything richer
/// is authored later as per-limb `MoveSpec`s. Data-driven on the boss profile;
/// the router (`route_boss_strikes_to_limbs`) owns the arc math.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
pub enum LimbMotion {
    /// Lift the limb toward `-gravity` (wind up / hold high).
    Raise,
    /// Sweep the limb laterally along the host's facing.
    SweepAcross,
    /// Arc the limb down along `+gravity` to a strike depth (an overhead slam).
    SlamDown,
    /// Station-keep — the limb holds its home pose while others strike.
    Hold,
}

/// Limb slots and motion driven by a strike, keyed by move id in
/// [`BossBehaviorProfile::limb_routing`].
#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LimbRoute {
    /// The slots the strike drives. Slots absent from the host rig are inert.
    pub slots: Vec<LimbSlot>,
    /// How each named slot moves during the strike.
    pub motion: LimbMotion,
}

/// Authored speech-bubble anchor for a boss (see
/// [`BossBehaviorProfile::bark_anchor`]).
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BarkAnchorSpec {
    pub dx_px: f32,
    pub dy_half_h: f32,
    pub dy_px: f32,
}

impl Default for BarkAnchorSpec {
    fn default() -> Self {
        Self {
            dx_px: 0.0,
            dy_half_h: -1.0,
            dy_px: -20.0,
        }
    }
}

/// Authored post-defeat reward for a boss. Parsed from the optional
/// `reward:` field of each row in `assets/data/boss_profiles.ron`
/// (defaults to `None` when the field is absent). The drop-chest
/// geometry (`offset`, `size`) is in world pixels; `pickup` names the
/// `PickupKind` granted on open.
///
/// Lives in the RON alongside the rest of the boss's behavior tuning, so
/// adding/retuning a reward is a content edit, not a code change.
#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub enum BossRewardProfile {
    None,
    DropChest {
        pickup: ambition_entity_catalog::PickupKind,
        #[serde(with = "boss_vec2_required")]
        offset: ae::Vec2,
        #[serde(with = "boss_vec2_required")]
        size: ae::Vec2,
    },
}

impl Default for BossRewardProfile {
    fn default() -> Self {
        Self::None
    }
}

/// Vec2 (de)serialization shims for `BossBehaviorProfile`. `bevy_math::Vec2`
/// doesn't implement `Deserialize` under the features the sandbox compiles
/// with, so we route through tuple shims.
mod boss_vec2_option {
    use ambition_platformer2d_core as ae;
    use serde::Deserialize;

    pub fn deserialize<'de, D>(de: D) -> Result<Option<ae::Vec2>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw: Option<(f32, f32)> = Option::deserialize(de)?;
        Ok(raw.map(|(x, y)| ae::Vec2::new(x, y)))
    }
}

mod boss_vec2_required {
    use ambition_platformer2d_core as ae;
    use serde::Deserialize;

    pub fn deserialize<'de, D>(de: D) -> Result<ae::Vec2, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let (x, y) = <(f32, f32)>::deserialize(de)?;
        Ok(ae::Vec2::new(x, y))
    }
}

/// Parsed boss-behavior rows used by content tooling and focused tests. Runtime
/// authority lives in the App-local `BossCatalog`.
#[derive(Clone, Debug, Default)]
pub struct BossProfileRegistry {
    /// `BTreeMap` for the same determinism reason as `strike_geometry` above:
    /// a parsed roster is iterated, and ADR 0023 says an iteration order must not
    /// depend on a hash seed.
    by_id: std::collections::BTreeMap<String, BossBehaviorProfile>,
}

impl BossProfileRegistry {
    /// Parse a boss-profile RON document (`BTreeMap<id, BossBehaviorProfile>`).
    pub fn from_ron(ron: &str) -> Self {
        let by_id = ron::from_str(ron).unwrap_or_else(|err| {
            panic!("boss_profiles.ron failed to deserialize as BTreeMap<String, BossBehaviorProfile>: {err}")
        });
        Self { by_id }
    }

    pub fn get(&self, id: &str) -> Option<&BossBehaviorProfile> {
        self.by_id.get(id)
    }
}

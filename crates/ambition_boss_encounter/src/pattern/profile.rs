//! Serde authoring vocabulary for `boss_profiles.ron`.
//! Runtime lookups and actor-specific extensions live outside this schema module.

use ambition_characters::actor::limb::LimbSlot;
use ambition_characters::brain::boss_pattern::{
    BossAttackPattern, BossAttackProfile, BossMovementProfile,
};
use ambition_platformer2d_core as ae;

/// Authored strike rectangle composed from body-scaled and fixed-pixel terms.
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
    /// Construct a rectangle using only body-scaled terms.
    pub const fn scaled(offset_factor: ae::Vec2, half_factor: ae::Vec2) -> Self {
        Self {
            offset_factor,
            offset_const: ae::Vec2::ZERO,
            half_factor,
            half_const: ae::Vec2::ZERO,
        }
    }

    /// Resolve the authored terms to a world-space AABB.
    pub fn to_aabb(&self, origin: ae::Vec2, size: ae::Vec2) -> ae::Aabb {
        ae::Aabb::new(
            origin + self.offset_factor * size + self.offset_const,
            self.half_factor * size + self.half_const,
        )
    }
}

/// Authored boss behavior: movement, contact/damage, attack geometry, and rewards.
/// Encounter HP and phase thresholds are owned separately. Providers assemble rows
/// into the App-local catalog; unknown authored bosses use the generic profile.
#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BossBehaviorProfile {
    pub id: String,
    /// Sprite-registry target. `None` uses this profile's `id`.
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
    /// Movement multiplier while a strike is committed. At moveset bake time this
    /// becomes the strike Active-window `motion_scale`.
    pub strike_speed_scale: f32,
    /// Optional `(amplitude_px, frequency_hz)` self-dodge while striking.
    #[serde(default)]
    pub self_dodge: Option<(f32, f32)>,
    /// Optional Engage/Approach/Retreat macro behavior; disabled means no macro dance.
    pub macro_tuning: ambition_characters::brain::BossMacroTuning,
    pub attacks: Vec<BossAttackProfile>,
    pub attack_cooldown: f32,
    pub attack_windup: f32,
    pub attack_active: f32,
    pub attack_damage: i32,
    pub body_damage: i32,
    /// `Cycle` uses the flat attack fields above; `Scripted` uses the phase-keyed
    /// timeline and ignores the flat cycle durations.
    pub attack_pattern: BossAttackPattern,
    /// World-space pixel offset for body-relative melee attack origins.
    #[serde(default, with = "boss_vec2_required")]
    pub attack_origin_offset: ae::Vec2,
    /// World-space pixel offset for projectile-like special origins.
    #[serde(default, with = "boss_vec2_required")]
    pub projectile_origin_offset: ae::Vec2,
    /// Optional post-defeat reward.
    #[serde(default)]
    pub reward: BossRewardProfile,
    /// Catalog ability dropped as a collectible on defeat.
    #[serde(default)]
    pub reward_ability: Option<String>,
    /// Held-item id dropped as the boss's signature gauntlet on defeat.
    #[serde(default)]
    pub signature_gauntlet: Option<String>,
    /// If true, ordinary hits provide impact feedback but cannot damage the boss;
    /// an authored environmental rule must defeat it.
    #[serde(default)]
    pub environmental_kill_only: bool,
    /// Speech-bubble anchor: `pos + (dx_px, dy_half_h * combat_half_height + dy_px)`.
    #[serde(default)]
    pub bark_anchor: BarkAnchorSpec,
    /// Per-move strike rectangles. A present move id replaces the built-in geometry;
    /// absent entries use it. `BTreeMap` order is canonical and fingerprint-load-bearing.
    #[serde(default)]
    pub strike_geometry: std::collections::BTreeMap<String, Vec<StrikeRect>>,
    /// Mount classes this boss may pilot. Room `mounted_on` refs install the
    /// actual rider/mount link.
    #[serde(default)]
    pub pilotable_mount_classes: Vec<String>,
    /// Move id to limb route. Missing moves remain host-body strikes with no limb intent.
    #[serde(default)]
    pub limb_routing: Vec<(String, LimbRoute)>,
    /// Controller verb to boss move key while possessed. Directional verbs use the
    /// shared verb chain; an empty map uses the default primary/signature mapping.
    #[serde(default)]
    pub possessed_verbs: Vec<(String, String)>,
}

/// Closed motion vocabulary used by the limb router during strike Startup/Active.
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

/// Authored post-defeat reward. Chest geometry is in world pixels.
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

/// Vec2 serde shim used by the boss-profile schema.
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

/// Parsed boss profiles for tooling and focused tests; runtime authority is the App-local catalog.
#[derive(Clone, Debug, Default)]
pub struct BossProfileRegistry {
    /// Deterministic iteration order is part of canonical profile handling.
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

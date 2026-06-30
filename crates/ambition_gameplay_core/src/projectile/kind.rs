//! Named projectile kinds + their authored stat tables (Ambition's basic kit).
//!
//! This is *named game content* — the Fireball / Hadouken tier and the numbers
//! behind it. It lives here (not in the foundation `ambition_platformer_primitives`,
//! which stays generic, and not yet in `ambition_content`, which the player
//! fire/charge/gesture systems below can't be reached from without the §4
//! ControlFrame→actor-intent extraction). The kind *lowers* into the engine's
//! generic [`ProjectileSpec`]; the engine never matches on a kind.
//!
//! `ProjectileKind` is also an ECS [`Component`]: player projectile entities
//! carry it so the combat-attribution (`HitSource::PlayerProjectile { kind }`),
//! the feel-trace, and the renderer (tint / sprite name) can read the named kind
//! without the engine knowing kinds exist.

use ambition_engine_core::Vec2;
use bevy::prelude::Component;
use serde::{Deserialize, Serialize};

use ambition_platformer_primitives::projectile::ProjectileSpec;

/// What kind of projectile to spawn.
///
/// The three variants form a "tier" of motion-input difficulty:
///   - `Fireball`: tap (or release-from-charge) the fire button. No
///     motion required. Bouncing, mild arc. Charges into bigger
///     variants via `ProjectileSpec.charge_tier`.
///   - `Hadouken`: a 2-step grace quarter-circle
///     (`Down → Right`) plus fire. Easier on keyboard than the
///     traditional 3-step gesture; the trade-off is a weaker
///     projectile than `HadoukenSuper`.
///   - `HadoukenSuper`: the traditional 3-step quarter-circle
///     (`Down → DownRight → Right`) plus fire. Strongest
///     projectile in the basic kit, costs the most resource.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Component)]
pub enum ProjectileKind {
    /// Cheap, bouncing fireball with a mild arc. Charges via hold-
    /// then-release; tier scales size and damage on `ProjectileSpec`.
    Fireball,
    /// Grace-input Hadouken. Travels horizontally, expires on first
    /// solid contact (no bounces).
    Hadouken,
    /// Full-input Hadouken. Same trajectory shape as `Hadouken` but
    /// chunkier hitbox, more damage, longer cooldown.
    HadoukenSuper,
}

impl ProjectileKind {
    /// Resource cost (mana / ammo / charge units) to fire one.
    pub fn cost(self) -> f32 {
        match self {
            Self::Fireball => 1.0,
            Self::Hadouken => 3.0,
            Self::HadoukenSuper => 5.0,
        }
    }

    /// Damage dealt on hit. Fireball charge tiers scale on top of
    /// this baseline via [`ProjectileKind::charged_spec`].
    pub fn damage(self) -> i32 {
        match self {
            Self::Fireball => 1,
            Self::Hadouken => 3,
            Self::HadoukenSuper => 5,
        }
    }

    /// Cooldown after firing, in seconds. Higher-tier projectiles
    /// have longer cooldowns so the player can't bypass the cost by
    /// spamming.
    pub fn cooldown(self) -> f32 {
        match self {
            Self::Fireball => 0.30,
            Self::Hadouken => 0.55,
            Self::HadoukenSuper => 0.85,
        }
    }

    /// Initial speed in pixels-per-second.
    pub fn speed(self) -> f32 {
        match self {
            Self::Fireball => 360.0,
            Self::Hadouken => 520.0,
            Self::HadoukenSuper => 640.0,
        }
    }

    /// Maximum lifetime in seconds. A projectile that hasn't hit
    /// anything by this time despawns and emits `ProjectileExpired`.
    pub fn max_lifetime(self) -> f32 {
        match self {
            Self::Fireball => 1.20,
            Self::Hadouken => 1.60,
            Self::HadoukenSuper => 1.80,
        }
    }

    /// Hitbox half-extent (pixels). Higher tiers are chunkier so the
    /// player can see (and aim) the difference at a glance.
    pub fn half_extent(self) -> Vec2 {
        match self {
            // Fireball was 8×6, bumped to 12×9 so the basic
            // projectile reads at a glance even before charging.
            Self::Fireball => Vec2::new(12.0, 9.0),
            Self::Hadouken => Vec2::new(16.0, 12.0),
            Self::HadoukenSuper => Vec2::new(22.0, 16.0),
        }
    }

    /// Per-second downward acceleration. Fireballs arc; Hadoukens fly straight.
    pub fn gravity(self) -> f32 {
        match self {
            Self::Fireball => 360.0,
            Self::Hadouken | Self::HadoukenSuper => 0.0,
        }
    }

    /// How many support-face bounces before expiring on a solid. Fireballs
    /// bounce twice (Mario-like / arcade-style); Hadoukens don't bounce.
    pub fn bounces(self) -> u8 {
        match self {
            Self::Fireball => 2,
            Self::Hadouken | Self::HadoukenSuper => 0,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Fireball => "fireball",
            Self::Hadouken => "hadouken",
            Self::HadoukenSuper => "hadouken_super",
        }
    }

    /// Lower this named kind into the engine's generic [`ProjectileSpec`].
    /// `damage_multiplier` is the firer's outgoing-damage scaling.
    pub fn spec(self, origin: Vec2, direction: Vec2, damage_multiplier: f32) -> ProjectileSpec {
        ProjectileSpec {
            origin,
            direction: direction.normalize_or(Vec2::new(1.0, 0.0)),
            damage: ((self.damage() as f32) * damage_multiplier)
                .round()
                .max(1.0) as i32,
            speed: self.speed(),
            max_lifetime: self.max_lifetime(),
            half_extent: self.half_extent(),
            gravity: self.gravity(),
            bounces: self.bounces(),
            // Ambition's player kinds are arcing/bouncing shots (pass through
            // one-ways, bounce off supports per budget). Authored here, not
            // derived from who fires — so the player-robot boss firing this
            // same kind behaves identically.
            world_hit: crate::projectile::WorldHitPolicy::Bouncing,
            charge_tier: 0,
        }
    }

    /// Apply a fireball charge tier (0–2) to a freshly-built spec. Multiplies
    /// damage and hitbox half-extent so a "heavy" charge is visibly larger and
    /// hits harder. Tier 0 is the no-charge baseline (no change). Non-Fireball
    /// kinds ignore the tier — they don't charge.
    pub fn charged_spec(self, mut spec: ProjectileSpec, tier: u8) -> ProjectileSpec {
        if !matches!(self, Self::Fireball) {
            return spec;
        }
        spec.charge_tier = tier.min(2);
        let size_mult = match spec.charge_tier {
            0 => 1.0,
            1 => 1.4,
            _ => 1.8,
        };
        // Exponential damage ramp for fast boss-battle test loops:
        // tier 0 = 1x, tier 1 = 4x, tier 2 = 16x. This intentionally
        // makes a fully charged fireball dramatically stronger than
        // the old linear-ish 1x/2x/3x table.
        let damage_mult = 4_i32.pow(spec.charge_tier as u32).max(1);
        spec.half_extent *= size_mult;
        spec.damage = spec.damage.saturating_mul(damage_mult).max(1);
        spec
    }
}

/// Fireball charge mechanic tuning. The sandbox samples hold time on
/// the fire button and quantizes into one of three tiers via
/// [`FireballChargeTuning::tier_for_hold`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FireballChargeTuning {
    /// Hold time threshold (seconds) for tier 1 (medium charge).
    pub medium_after: f32,
    /// Hold time threshold (seconds) for tier 2 (heavy charge).
    pub heavy_after: f32,
}

impl FireballChargeTuning {
    /// Default thresholds: light <0.35s, medium 0.35–0.85s, heavy
    /// 0.85s+. Tuned to feel like a brief hold for medium and a
    /// noticeable wind-up for heavy.
    pub const DEFAULT: Self = Self {
        medium_after: 0.35,
        heavy_after: 0.85,
    };

    pub fn tier_for_hold(self, hold_seconds: f32) -> u8 {
        if hold_seconds >= self.heavy_after {
            2
        } else if hold_seconds >= self.medium_after {
            1
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn charge_tier_thresholds_are_inclusive_at_the_boundary() {
        let t = FireballChargeTuning::DEFAULT;
        assert_eq!(t.tier_for_hold(0.0), 0);
        assert_eq!(t.tier_for_hold(0.349), 0);
        assert_eq!(t.tier_for_hold(0.35), 1, "exact medium threshold charges");
        assert_eq!(t.tier_for_hold(0.849), 1);
        assert_eq!(t.tier_for_hold(0.85), 2, "exact heavy threshold charges");
        assert_eq!(t.tier_for_hold(10.0), 2);
    }

    #[test]
    fn fireball_charge_tier_ramps_damage_x4_per_tier_and_grows_hitbox() {
        let base = ProjectileKind::Fireball.spec(Vec2::ZERO, Vec2::new(1.0, 0.0), 1.0);
        let base_dmg = base.damage;
        let base_he = base.half_extent;

        let t1 = ProjectileKind::Fireball.charged_spec(base, 1);
        assert_eq!(t1.damage, base_dmg * 4);
        assert!((t1.half_extent.x - base_he.x * 1.4).abs() < 1e-3);

        let t2 = ProjectileKind::Fireball.charged_spec(base, 2);
        assert_eq!(t2.damage, base_dmg * 16);
        assert!((t2.half_extent.x - base_he.x * 1.8).abs() < 1e-3);

        // Out-of-range tier clamps to 2.
        let clamped = ProjectileKind::Fireball.charged_spec(base, 7);
        assert_eq!(clamped.charge_tier, 2);
        assert_eq!(clamped.damage, base_dmg * 16);
    }

    #[test]
    fn non_fireball_kinds_ignore_the_charge_tier() {
        let h = ProjectileKind::Hadouken.spec(Vec2::ZERO, Vec2::new(1.0, 0.0), 1.0);
        let charged = ProjectileKind::Hadouken.charged_spec(h, 2);
        assert_eq!(charged.damage, h.damage, "Hadouken does not charge");
        assert_eq!(charged.charge_tier, 0);
        assert_eq!(charged.half_extent, h.half_extent);
    }

    #[test]
    fn fireball_bounces_twice_hadouken_never() {
        assert_eq!(ProjectileKind::Fireball.bounces(), 2);
        assert_eq!(ProjectileKind::Hadouken.bounces(), 0);
        assert_eq!(ProjectileKind::HadoukenSuper.bounces(), 0);
    }

    #[test]
    fn spec_normalizes_direction_and_floors_damage() {
        let s = ProjectileKind::Fireball.spec(Vec2::ZERO, Vec2::new(3.0, 0.0), 1.0);
        let v = s.initial_velocity();
        assert!((v.x - s.speed).abs() < 1e-3);
        assert!(v.y.abs() < 1e-3);
        assert!(s.damage >= 1, "damage floors at 1");
    }
}

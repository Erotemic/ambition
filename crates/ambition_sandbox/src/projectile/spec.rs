//! Authored-intent types for projectiles: kind enum + per-shot spec
//! plus the fireball charge tuning tier table.

use crate::engine_core::Vec2;
use serde::{Deserialize, Serialize};

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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
    /// this baseline via `ProjectileSpec::with_charge_tier`.
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

    pub fn label(self) -> &'static str {
        match self {
            Self::Fireball => "fireball",
            Self::Hadouken => "hadouken",
            Self::HadoukenSuper => "hadouken_super",
        }
    }
}

/// Authored intent for a single new projectile. Sandbox spawns an
/// entity carrying this spec plus its current pos / vel; `ProjectileBody`
/// is the per-frame state it advances.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProjectileSpec {
    pub kind: ProjectileKind,
    /// Initial center position.
    pub origin: Vec2,
    /// Unit-length direction vector. (1, 0) fires right.
    pub direction: Vec2,
    /// Damage to apply on hit.
    pub damage: i32,
    /// Initial speed in px/s.
    pub speed: f32,
    /// Maximum lifetime.
    pub max_lifetime: f32,
    /// Half-extent of the hitbox.
    pub half_extent: Vec2,
    /// Vertical acceleration applied each frame (px/s^2). Mario-like /
    /// arcade-style arc: positive value pulls down (recall +Y is down
    /// in the sandbox simulation).
    pub gravity: f32,
    /// Fireball charge tier (0 = light tap, 1 = medium hold, 2 = heavy
    /// charge). Hadouken / HadoukenSuper ignore this — their stats
    /// come from the kind. Stored so the trace and the visual layer
    /// can read which tier was fired.
    pub charge_tier: u8,
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

impl ProjectileSpec {
    pub fn new(
        kind: ProjectileKind,
        origin: Vec2,
        direction: Vec2,
        damage_multiplier: f32,
    ) -> Self {
        Self {
            kind,
            origin,
            direction: direction.normalize_or(Vec2::new(1.0, 0.0)),
            damage: ((kind.damage() as f32) * damage_multiplier)
                .round()
                .max(1.0) as i32,
            speed: kind.speed(),
            max_lifetime: kind.max_lifetime(),
            half_extent: kind.half_extent(),
            gravity: match kind {
                ProjectileKind::Fireball => 360.0,
                ProjectileKind::Hadouken | ProjectileKind::HadoukenSuper => 0.0,
            },
            charge_tier: 0,
        }
    }

    /// Apply a fireball charge tier (0–2). Multiplies damage and
    /// hitbox half-extent so a "heavy" charge is visibly larger and
    /// hits harder. Tier 0 is the no-charge baseline (no change).
    /// Non-Fireball kinds ignore the tier — they don't charge.
    pub fn with_charge_tier(mut self, tier: u8) -> Self {
        if !matches!(self.kind, ProjectileKind::Fireball) {
            return self;
        }
        self.charge_tier = tier.min(2);
        let size_mult = match self.charge_tier {
            0 => 1.0,
            1 => 1.4,
            _ => 1.8,
        };
        // Exponential damage ramp for fast boss-battle test loops:
        // tier 0 = 1x, tier 1 = 4x, tier 2 = 16x. This intentionally
        // makes a fully charged fireball dramatically stronger than
        // the old linear-ish 1x/2x/3x table.
        let damage_mult = 4_i32.pow(self.charge_tier as u32).max(1);
        self.half_extent *= size_mult;
        self.damage = self.damage.saturating_mul(damage_mult).max(1);
        self
    }

    pub fn initial_velocity(&self) -> Vec2 {
        self.direction * self.speed
    }
}

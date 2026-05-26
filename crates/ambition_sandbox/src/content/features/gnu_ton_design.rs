//! GNU-ton's per-attack-profile design-space geometry.
//!
//! Today these coordinates live as Rust constants because GNU-ton's
//! sprite generator (`tools/ambition_sprite2d_renderer/.../gnu_ton_boss/sprite_generator.py`)
//! doesn't go through the standard adapter pipeline that emits
//! `body_metrics.animations[name].hitbox.parts` into the spritesheet
//! RON — the way the gradient sentinel publishes its hitboxes.
//!
//! **Migration plan** to fully data-driven:
//!
//! 1. Update the gnu_ton_boss sprite generator to emit a
//!    `body_metrics` block in its spritesheet RON, with one entry
//!    per animation name keyed by `BossAttackProfile`. The pixel
//!    coordinates would come from the same design-space constants
//!    referenced below (`REST_HEAD_Y`, `REST_HAND_X`, `SLAM_STRIKE_Y`,
//!    etc. in the Python source).
//! 2. Add `gnu_ton_boss` to `sprite_target_for_boss` and
//!    `sprite_render_size_for` so the existing `BossSpriteMetrics`
//!    derivation path runs for GNU-ton.
//! 3. Extend `boss_animation_for_profile` so the Gnu* profiles
//!    resolve to the per-profile animation names in the RON.
//! 4. Delete this module, the `is_gnu_ton` branches in
//!    `boss_attack_geometry::damageable_volumes` /
//!    `volumes_for_profile`, and `gnu_ton_part_aabb`.
//!
//! The intermediate state (this module) achieves the *architectural*
//! goal of removing inline magic numbers from the volume math while
//! making the migration path concrete. The data is structured as a
//! lookup table that maps 1-to-1 to the JSON / RON shape we'd write.

use ambition_engine as ae;

use crate::brain::BossAttackProfile;

/// One hand-tuned design-space part — center + half-extent — that
/// `gnu_ton_part_aabb` consumes to produce a world AABB.
pub struct GnuPartSpec {
    pub center: ae::Vec2,
    pub half_size: ae::Vec2,
}

// =================================================================
// Per-attack-profile parts. Coordinates mirror the design-space
// values the sprite generator's `_draw_*` functions render at;
// keep both files in sync.
// =================================================================

/// Hands lift then slam down to floor level. Two strike volumes —
/// one per hand — at design y=195 (just below the leg hooves at
/// design +175). Reach is the hand silhouette half-extent.
pub const HAND_SLAM_PARTS: &[GnuPartSpec] = &[
    GnuPartSpec {
        center: ae::Vec2::new(-235.0, 195.0),
        half_size: ae::Vec2::new(78.0, 60.0),
    },
    GnuPartSpec {
        center: ae::Vec2::new(235.0, 195.0),
        half_size: ae::Vec2::new(78.0, 60.0),
    },
];

/// Hands sweep in from the far edges. Two arc volumes — one per
/// side — covering the swept arm reach at mid-body height.
pub const HAND_SWEEP_PARTS: &[GnuPartSpec] = &[
    GnuPartSpec {
        center: ae::Vec2::new(-185.0, 20.0),
        half_size: ae::Vec2::new(140.0, 60.0),
    },
    GnuPartSpec {
        center: ae::Vec2::new(185.0, 20.0),
        half_size: ae::Vec2::new(140.0, 60.0),
    },
];

/// Head descends to player level — the vulnerability window. The
/// head AABB also damages the player while it's down.
pub const HEAD_DESCENT_PARTS: &[GnuPartSpec] = &[GnuPartSpec {
    center: ae::Vec2::new(0.0, 30.0),
    half_size: ae::Vec2::new(92.0, 74.0),
}];

/// Ground-level shockwave bar that fires after a slam impact.
pub const SHOCKWAVE_PARTS: &[GnuPartSpec] = &[GnuPartSpec {
    center: ae::Vec2::new(0.0, 195.0),
    half_size: ae::Vec2::new(300.0, 18.0),
}];

/// Empty fallback for profiles GNU-ton doesn't author (apple rain
/// damage routes through projectile bodies; the gradient sentinel
/// profiles never land on this boss).
const NO_PARTS: &[GnuPartSpec] = &[];

/// Look up the per-profile parts in design space. Always returns a
/// slice — empty when the profile has no body-mounted hitbox.
pub fn parts_for_profile(profile: &BossAttackProfile) -> &'static [GnuPartSpec] {
    match profile {
        BossAttackProfile::GnuHandSlam => HAND_SLAM_PARTS,
        BossAttackProfile::GnuHandSweep => HAND_SWEEP_PARTS,
        BossAttackProfile::GnuHeadDescent => HEAD_DESCENT_PARTS,
        BossAttackProfile::GnuShockwave => SHOCKWAVE_PARTS,
        // Apple rain damages via spawned projectile bodies, not a
        // body-mounted AABB. Gradient Sentinel profiles never land
        // on this boss but the empty arm keeps the match coverage
        // explicit.
        BossAttackProfile::GnuAppleRain
        | BossAttackProfile::OverfitVolley
        | BossAttackProfile::MinimaTrap
        | BossAttackProfile::SaddlePoint
        | BossAttackProfile::GradientCascade
        | BossAttackProfile::GradientLane => NO_PARTS,
        // Non-gnu_ton profiles never land on this boss either.
        _ => NO_PARTS,
    }
}

// =================================================================
// Damageable head — split-out from the attack profiles because the
// head moves between the rest and descent positions across the
// boss's animation timeline, not per attack profile.
// =================================================================

/// Head's design-space half-extent (shared between rest + descent).
pub const HEAD_HALF_SIZE: ae::Vec2 = ae::Vec2::new(92.0, 74.0);

/// Head's idle position above the shoulder ridge. Matches the
/// generator's `REST_HEAD_Y = -75.0`.
pub const HEAD_REST_CENTER: ae::Vec2 = ae::Vec2::new(0.0, -75.0);

/// Head's held-low position during the descent vulnerability
/// window. Matches the generator's `_draw_head_down` target y=30.
pub const HEAD_DESCENT_CENTER: ae::Vec2 = ae::Vec2::new(0.0, 30.0);

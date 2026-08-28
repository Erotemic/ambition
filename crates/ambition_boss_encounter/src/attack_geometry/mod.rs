//! Pure authored attack/body volume math; no ECS access or mutation.
//!
//! [`CombatGeometry`] is shared body geometry. [`BossVolumeContext`] adds the
//! boss-specific state needed to derive strike, telegraph, hurtbox, and contact
//! volumes. Sprite-authored hit/hurt boxes are preferred; profile geometry is
//! the fallback.

use ambition_platformer2d_core as ae;
use ambition_sprite_sheet::ActorSpriteMetrics;

// The volumes this module returns are shaped now, so the LIB stopped measuring
// boxes with this trait — but the sibling test modules still do, and they
// inherit their imports from here through `use super::*`.
#[cfg_attr(not(test), allow(unused_imports))]
use ambition_platformer2d_core::AabbExt;

use bevy::prelude::Component;

use ambition_characters::brain::{BossAttackProfile, BossAttackState};

use super::behavior::BossBehaviorProfile;

mod frame;
// ⭐⭐ THE UNIVERSAL HALF LEFT FOR `ambition_combat::body_geometry` (2026-08-28,
// D117): `CombatGeometry`, `AnimationSelection`, `SimpleActorGeometry`, the
// hurtbox/collision derivation and the pixel-rect → world AABB math. What is left
// here is the BOSS half — the context a boss feeds that math, its impl of the
// trait, and the per-profile strike geometry.
//
// ⛔ RE-EXPORTED rather than repointed at ~30 in-crate call sites, and that is the
// ONE republication this crate keeps on purpose: the names are re-exported here
// because the boss half's own signatures speak them. ⚠ A CONSUMER OUTSIDE THIS
// CRATE SHOULD NAME `ambition_combat::body_geometry` — republishing a peer
// domain's vocabulary under this crate's address is the defect four carves went
// looking for today.
pub use ambition_combat::body_geometry::*;
use frame::*;

/// All the per-tick inputs the volume helpers need. Owned by the
/// caller so the helpers themselves stay pure.
pub struct BossVolumeContext<'a> {
    /// App-local authored boss authority used for special animation aliases.
    pub boss_catalog: &'a super::BossCatalog,
    pub pos: ae::Vec2,
    pub size: ae::Vec2,
    pub combat_size: ae::Vec2,
    pub behavior: &'a BossBehaviorProfile,
    pub attack_state: &'a BossAttackState,
    /// Sprite-driven body metrics. `Some` for bosses whose sprite
    /// RON carries `body_metrics` and the derivation system has
    /// snapshotted it. `damageable_volumes` prefers multi-rect
    /// hurtboxes from here over the legacy single-AABB fallback.
    pub sprite_metrics: Option<&'a ambition_sprite_sheet::ActorSpriteMetrics>,
    /// Optional frame sample from the live boss sprite animator.
    /// When present and its profile matches the requested attack,
    /// sprite-authored hit/hurt boxes use this exact frame index
    /// instead of re-deriving a frame from attack timers. That keeps
    /// gameplay/debug boxes locked to the rendered animation frame.
    pub animation_frame: Option<&'a BossAnimationFrameSample>,
    /// Boss facing (sign of x). The sprite flips horizontally to face the
    /// player, so an off-center body's hurtboxes must mirror too — otherwise
    /// they land on the wrong side when the boss faces left. `1.0` = right
    /// (no mirror), `< 0.0` = flipped. See [`mirror_x_if_flipped`].
    pub facing: f32,
}

/// Live sprite-animation frame for a boss attack profile.
///
/// The renderer writes this component onto the boss simulation
/// entity when the currently rendered boss row is directly driven by
/// a `BossAttackProfile`. Gameplay/debug volume helpers read it
/// opportunistically and fall back to elapsed-time sampling in
/// headless tests or before sprites have upgraded.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct BossAnimationFrameSample {
    /// Gameplay profile that selected the currently-rendered boss row,
    /// or `None` when the rendered row is the idle/rest pose (which is
    /// not driven by any attack profile). An idle sample still carries
    /// the live `frame_index` so the rest-pose hurtbox bobs with the
    /// breathing animation instead of locking to frame 0.
    pub profile: Option<BossAttackProfile>,
    /// Frame index in the currently-rendered boss row.
    pub frame_index: usize,
    /// Runtime sprite-metadata key that should be sampled with
    /// `frame_index`, when the renderer can resolve it. This is
    /// redundant with `profile` for most rows, but keeping the key on
    /// the sample makes the bridge explicit and prevents future
    /// profile↔row alias drift from silently selecting a fallback box.
    pub animation_key: Option<String>,
}

impl<'a> BossVolumeContext<'a> {
    /// Build the context from a live boss view + its attack-state component.
    /// The boss contributes only body fields, not policy; volume selection is
    /// data-driven via `sprite_metrics`.
    pub fn from_ref(
        boss_catalog: &'a super::BossCatalog,
        boss: crate::BossRef<'a>,
        attack_state: &'a BossAttackState,
    ) -> Self {
        Self {
            boss_catalog,
            pos: boss.kin.pos,
            // The sprite render-BASIS (AS4b) — the world scale sprite-metric hurtboxes
            // derive from. Was `kin.size`; that's now the COLLISION envelope, so read
            // the render basis explicitly to keep hurtbox scaling byte-identical.
            size: boss.render_size(),
            combat_size: boss.combat_size(),
            behavior: &boss.config.behavior,
            attack_state,
            sprite_metrics: boss.status.sprite_metrics.as_ref(),
            animation_frame: None,
            facing: boss.kin.facing,
        }
    }

    pub fn with_animation_frame(
        mut self,
        animation_frame: Option<&'a BossAnimationFrameSample>,
    ) -> Self {
        self.animation_frame = animation_frame;
        self
    }
}

impl CombatGeometry for BossVolumeContext<'_> {
    fn body_pos(&self) -> ae::Vec2 {
        self.pos
    }
    fn body_size(&self) -> ae::Vec2 {
        self.size
    }
    fn facing(&self) -> f32 {
        self.facing
    }
    fn combat_size(&self) -> ae::Vec2 {
        self.combat_size
    }
    fn sprite_metrics(&self) -> Option<&ActorSpriteMetrics> {
        self.sprite_metrics
    }
    fn hurtbox_selection(&self) -> AnimationSelection {
        // The current animation is the live strike's, else the windup's, else
        // rest. Matches the visible sprite pose so a side-sweep's extended arms
        // register as damageable while the rest pose's tight bbox wins idle.
        let active_profile = self
            .attack_state
            .active_profile
            .as_ref()
            .or(self.attack_state.telegraph_profile.as_ref());
        let keys = runtime_animation_keys(self, active_profile, &["rest"]).in_lookup_order();
        let elapsed_s = if self.attack_state.active_profile.is_some() {
            self.attack_state.active_elapsed
        } else if self.attack_state.telegraph_profile.is_some() {
            self.attack_state.telegraph_elapsed
        } else {
            0.0
        };
        // A live frame sample overrides elapsed derivation only when it matches
        // the pose being sampled (same profile, or an idle sample for rest).
        let live_frame_index = self
            .animation_frame
            .and_then(|sample| match active_profile {
                Some(profile) => {
                    (sample.profile.as_ref() == Some(profile)).then_some(sample.frame_index)
                }
                None => sample.profile.is_none().then_some(sample.frame_index),
            });
        AnimationSelection {
            keys,
            elapsed_s,
            live_frame_index,
        }
    }
}

/// Active strike volumes — drawn red in the debug overlay and tested
/// against the player body by the damage system. Returns empty when
/// no strike is live (`attack_state.active_profile == None`).
///
/// Priority: sprite-author-declared per-animation hitbox (from
/// `ActorSpriteMetrics::animations[animation_name].hitbox`) wins
/// over the hardcoded `volumes_for_profile` math. So when an
/// adapter declares the FloorSlam hitbox as `(4, 88, 120, 30)` in
/// pixel-frame coords, that's what damages the player — scaled to
/// world by the boss's render size. Falls back to
/// `volumes_for_profile` when the sprite has no per-animation
/// hitbox for this profile.
pub fn active_attack_volumes(ctx: &BossVolumeContext) -> Vec<ae::CombatVolume> {
    let Some(profile) = ctx.attack_state.active_profile.as_ref() else {
        return Vec::new();
    };
    if let Some(volumes) = sprite_authored_volumes(ctx, profile, ctx.attack_state.active_elapsed) {
        return volumes;
    }
    // The hardcoded strike-geometry table is rectangles by authorship, so it
    // stays rectangles here — a box that says it is a box costs nothing and
    // keeps the cheap overlap path.
    volumes_for_profile(profile, ctx.pos, ctx.combat_size, ctx.behavior)
        .into_iter()
        .map(ae::CombatVolume::aabb)
        .collect()
}

/// Pull sprite-author-declared hitbox rectangles for the given
/// attack profile from `ctx.sprite_metrics.animations`. Returns
/// `None` (not empty) when the sprite has no hitbox for this
/// animation; the caller falls back to the hardcoded
/// `volumes_for_profile` math. Returns an empty `Vec` when the
/// sprite has an entry but no usable rects (defensive).

/// Damageable hurtbox volumes — where the player's attacks register
/// as hits. Single-piece bosses use one AABB derived from
/// combat_size; multi-part bosses (sprite RON carrying
/// `body_pixel_parts`) emit one AABB per piece so head/body/arms
/// hit independently. Animation boxes may also carry per-frame
/// samples so large moving parts like GNU-ton's head can track the
/// drawn pose instead of one coarse per-animation rectangle.

// No bespoke boss damage poll.
//
// NOTE: `active_attack_volumes` / `volumes_for_profile` below are now consumed only by
// the DEBUG overlay (telegraph/strike gizmos) and the hurtbox-pose selection — the
// gameplay strike geometry is authored into each boss move's `HitVolume`s at spawn
// (`boss_attack_moveset`). The sprite-frame-tracking multi-part geometry those helpers
// still express is the fidelity the static move volumes approximate (bulk-review).

/// One body-local strike rectangle, as DATA.
///
/// Pure data over `ae::Vec2`; nothing about it needed to be here.
pub use crate::pattern::profile::StrikeRect;

// Built-in per-profile strike geometry, as DATA. Each was a hardcoded `vec![Aabb::new
// (..)]` arm in `volumes_for_profile`; the numbers are IDENTICAL (pinned byte-for-byte
// by `strike_geometry_is_byte_identical_to_the_old_hardcoded_match`). A content boss's
// authored geometry would slot in beside these.
const FLOOR_SLAM: &[StrikeRect] = &[StrikeRect {
    offset_factor: ae::Vec2::new(0.0, 0.5),
    offset_const: ae::Vec2::new(0.0, 22.0),
    half_factor: ae::Vec2::new(0.75, 0.0),
    half_const: ae::Vec2::new(0.0, 18.0),
}];
const SIDE_SWEEP: &[StrikeRect] = &[
    StrikeRect::scaled(ae::Vec2::new(-0.50, 0.0), ae::Vec2::new(0.25, 0.72)),
    StrikeRect::scaled(ae::Vec2::new(0.50, 0.0), ae::Vec2::new(0.25, 0.72)),
];
const FULL_BODY_PULSE: &[StrikeRect] = &[StrikeRect::scaled(
    ae::Vec2::new(0.0, 0.0),
    ae::Vec2::new(0.70, 0.70),
)];
const HAZARD_COLUMN: &[StrikeRect] = &[StrikeRect::scaled(
    ae::Vec2::new(0.0, 0.0),
    ae::Vec2::new(0.30, 1.80),
)];
const WING_SWEEP: &[StrikeRect] = &[StrikeRect::scaled(
    ae::Vec2::new(0.0, 0.08),
    ae::Vec2::new(0.56, 0.42),
)];
const DIVE_LANE: &[StrikeRect] = &[StrikeRect::scaled(
    ae::Vec2::new(0.0, 0.42),
    ae::Vec2::new(0.22, 0.72),
)];
const BROADSIDE: &[StrikeRect] = &[
    StrikeRect::scaled(ae::Vec2::new(-0.34, 0.0), ae::Vec2::new(0.18, 0.84)),
    StrikeRect::scaled(ae::Vec2::new(0.34, 0.0), ae::Vec2::new(0.18, 0.84)),
];
const HAND_SLAM: &[StrikeRect] = &[
    StrikeRect::scaled(ae::Vec2::new(-0.40, 0.25), ae::Vec2::new(0.14, 0.60)),
    StrikeRect::scaled(ae::Vec2::new(0.40, 0.25), ae::Vec2::new(0.14, 0.60)),
];
const HAND_SWEEP: &[StrikeRect] = &[StrikeRect::scaled(
    ae::Vec2::new(0.0, 0.15),
    ae::Vec2::new(0.85, 0.28),
)];
const HEAD_DESCENT: &[StrikeRect] = &[StrikeRect::scaled(
    ae::Vec2::new(0.0, 0.05),
    ae::Vec2::new(0.32, 0.38),
)];
const CONVERGING_SHOCKWAVE: &[StrikeRect] = &[StrikeRect::scaled(
    ae::Vec2::new(0.0, 0.48),
    ae::Vec2::new(0.90, 0.08),
)];

/// The body-local strike rectangles for a profile, as DATA. `Special(_)` carries no
/// body-mounted volume (its damage flows through the content Technique's own effects),
/// so it returns an empty slice. This is the single per-profile geometry table both the
/// gameplay path (`boss_attack_moveset` → `HitVolume`s) and the debug/pose fallback
/// (`volumes_for_profile`) read.
pub fn strike_geometry(move_id: &str) -> &'static [StrikeRect] {
    // Keyed by the profile's `move_id` (the strike key). The built-in geometry
    // vocabulary is `BossAttackProfile::BUILTIN_STRIKE_KEYS`; any other key
    // (a content-technique `Special`, or a geometry strike a boss authors ONLY
    // via its RON `strike_geometry` override) has no built-in rects here.
    match move_id {
        "floor_slam" => FLOOR_SLAM,
        "side_sweep" => SIDE_SWEEP,
        "full_body_pulse" => FULL_BODY_PULSE,
        "hazard_column" => HAZARD_COLUMN,
        "wing_sweep" => WING_SWEEP,
        "dive_lane" => DIVE_LANE,
        "broadside" => BROADSIDE,
        "hand_slam" => HAND_SLAM,
        "hand_sweep" => HAND_SWEEP,
        "head_descent" => HEAD_DESCENT,
        "converging_shockwave" => CONVERGING_SHOCKWAVE,
        _ => &[],
    }
}

/// World-space hitbox volumes for a specific attack profile — the DATA-driven resolve
/// of [`strike_geometry`] at this body's origin/size (fable §C6: the geometry is now a
/// declarative [`StrikeRect`] table, not a hardcoded per-variant `match`). Pure
/// function of the profile + body fields. Used as the fallback path when the boss has
/// no `sprite_metrics`-driven per-animation hitbox. The gradient sentinel and (since
/// ) GNU-ton route through `sprite_authored_volumes` instead — the geometry
/// table here is still required for bosses whose sprite RONs don't yet carry
/// per-animation hitbox.parts, AND is the source `boss_attack_moveset` derives each
/// boss move's `HitVolume`s from at spawn.
pub fn volumes_for_profile(
    attack: &BossAttackProfile,
    pos: ae::Vec2,
    combat_size: ae::Vec2,
    behavior: &BossBehaviorProfile,
) -> Vec<ae::Aabb> {
    // The strike origin: the boss body position shifted by its authored attack
    // offset. Each profile's DATA rects resolve against it.
    let origin = pos + behavior.attack_origin_offset;
    // A boss may AUTHOR its own rects for this move (§C6 "out of core"): an override
    // in `behavior.strike_geometry` (RON, keyed by `move_id`) REPLACES the built-in
    // table — so a content boss supplies its strike shapes with no core edit. Empty =
    // the built-in per-profile geometry. This one resolve feeds BOTH the debug/pose
    // path AND `boss_attack_moveset`'s gameplay `HitVolume`s (its single source).
    let move_id = attack.move_id();
    let rects: &[StrikeRect] = behavior
        .strike_geometry
        .get(&move_id)
        .map(Vec::as_slice)
        .unwrap_or_else(|| strike_geometry(&move_id));
    rects
        .iter()
        .map(|rect| rect.to_aabb(origin, combat_size))
        .collect()
}

// `gnu_ton_part_aabb` / `gnu_ton_sprite_scale` /
// GNU-ton's per-animation hit/hurt-box geometry lives in
// `gnu_ton_boss_spritesheet.ron`'s `body_metrics.animations` map, derived via
// the generic `world_aabb_from_pixel_rect` pixel→world transform (the same one
// the gradient sentinel uses).

#[cfg(test)]
mod sprite_metadata_derivation_tests;

#[cfg(test)]
mod simple_geometry_tests;
#[cfg(test)]
mod strike_geometry_data_tests;

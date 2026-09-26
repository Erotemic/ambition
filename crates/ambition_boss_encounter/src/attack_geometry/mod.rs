//! Pure authored attack/body volume math; no ECS access or mutation.
//!
//! [`CombatGeometry`] is shared body geometry. [`BossVolumeContext`] adds the
//! boss-specific state needed to derive strike, telegraph, hurtbox, and contact
//! volumes. Sprite-authored hit/hurt boxes are preferred; profile geometry is
//! the fallback.

use ambition_platformer2d_core as ae;
use ambition_sprite_sheet::ActorSpriteMetrics;

// The library no longer uses this trait, but the sibling test modules do, and
// they get their imports from here through `use super::*`.
#[cfg_attr(not(test), allow(unused_imports))]
use ambition_platformer2d_core::AabbExt;

use bevy::prelude::Component;

use ambition_characters::brain::{BossAttackProfile, BossAttackState};

use super::behavior::BossBehaviorProfile;

mod frame;
// The universal half is in `ambition_combat::body_geometry`:
// `CombatGeometry`, `AnimationSelection`, `SimpleActorGeometry`, the
// hurtbox/collision derivation and the pixel-rect → world AABB math. This
// module has the boss half: the context a boss feeds that math, its impl of
// the trait, and the per-profile strike geometry.
//
// Re-exported `pub(crate)` for this crate's own signatures only. Consumers
// outside this crate name `ambition_combat::body_geometry` directly.
pub(crate) use ambition_combat::body_geometry::*;
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
    /// Sprite-driven body metrics. `Some` for bosses whose sprite RON carries
    /// `body_metrics` and the derivation system has captured it.
    /// `damageable_volumes` prefers multi-rect hurtboxes from here over the
    /// single-AABB fallback.
    pub sprite_metrics: Option<&'a ambition_sprite_sheet::ActorSpriteMetrics>,
    /// Optional frame sample from the live boss sprite animator.
    /// When present and its profile matches the requested attack,
    /// sprite-authored hit/hurt boxes use this exact frame index, not a frame
    /// derived from attack timers, so gameplay and debug boxes follow the
    /// rendered frame.
    pub animation_frame: Option<&'a BossAnimationFrameSample>,
    /// Boss facing (sign of x). The sprite flips horizontally to face the
    /// player, so an off-center body's hurtboxes must mirror too, or they land
    /// on the wrong side when the boss faces left. `1.0` = right (no mirror),
    /// `< 0.0` = flipped. See [`mirror_x_if_flipped`].
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
            // The sprite render basis: the world scale sprite-metric hurtboxes
            // derive from. `kin.size` is the collision envelope, so read the
            // render basis explicitly.
            size: boss.render_size(),
            combat_size: boss.combat_size(),
            behavior: &boss.config.behavior,
            attack_state,
            sprite_metrics: boss.status.sprite_metrics.as_ref(),
            animation_frame: None,
            // The volumes mirror with the drawn sprite, so by its side.
            facing: boss.drawn_side(),
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
/// Priority: the sprite author's per-animation hitbox (from
/// `ActorSpriteMetrics::animations[animation_name].hitbox`) wins over the
/// `volumes_for_profile` math. For example, a FloorSlam hitbox declared as
/// `(4, 88, 120, 30)` in pixel-frame coords is what damages the player,
/// scaled to world by the boss's render size. Falls back to
/// `volumes_for_profile` when the sprite has no per-animation hitbox for this
/// profile.
pub fn active_attack_volumes(ctx: &BossVolumeContext) -> Vec<ae::CombatVolume> {
    let Some(profile) = ctx.attack_state.active_profile.as_ref() else {
        return Vec::new();
    };
    if let Some(volumes) = sprite_authored_volumes(ctx, profile, ctx.attack_state.active_elapsed) {
        return volumes;
    }
    // The strike-geometry table is rectangles by authorship, so it stays
    // rectangles here and keeps the cheap overlap path.
    volumes_for_profile(profile, ctx.pos, ctx.combat_size, ctx.behavior)
        .into_iter()
        .map(ae::CombatVolume::aabb)
        .collect()
}

// How sprite-declared geometry is read.
//
// A sprite-author-declared hitbox for an attack profile comes from
// `ctx.sprite_metrics.animations`. `None` (not empty) means the sprite has no
// hitbox for that animation, and the caller falls back to
// `volumes_for_profile`; an empty `Vec` means an entry with no usable rects.
//
// Damageable hurtbox volumes (where the player's attacks register) are one
// AABB from `combat_size` for a single-piece boss, and one per piece for a
// multi-part boss (sprite RON with `body_pixel_parts`), so head/body/arms hit
// independently. Animation boxes may carry per-frame samples, so a large
// moving part such as GNU-ton's head follows the drawn pose. The live spawner
// is `boss_spawn_hurtboxes` in `ecs/sync.rs`.

// No bespoke boss damage poll.
//
// `active_attack_volumes` / `volumes_for_profile` are used only by the debug
// overlay (telegraph/strike gizmos) and the hurtbox-pose selection. Gameplay
// strike geometry is authored into each boss move's `HitVolume`s at spawn
// (`boss_attack_moveset`).

/// One body-local strike rectangle, as data.
pub use crate::pattern::profile::StrikeRect;

// Built-in per-profile strike geometry, as data. Pinned by
// `strike_geometry_is_byte_identical_to_the_old_hardcoded_match`. A content
// boss's authored geometry goes beside these.
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

/// The body-local strike rectangles for a profile, as data. `Special(_)` has
/// no body-mounted volume (its damage comes from the content technique's own
/// effects), so it returns an empty slice. Both the gameplay path
/// (`boss_attack_moveset` → `HitVolume`s) and the debug/pose fallback
/// (`volumes_for_profile`) read this one table.
pub fn strike_geometry(move_id: &str) -> &'static [StrikeRect] {
    // Keyed by the profile's `move_id` (the strike key). The built-in
    // vocabulary is `BossAttackProfile::BUILTIN_STRIKE_KEYS`; any other key (a
    // content-technique `Special`, or a strike a boss authors only through its
    // RON `strike_geometry` override) has no built-in rects here.
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

/// World-space hitbox volumes for an attack profile: [`strike_geometry`]
/// resolved at this body's origin and size. A pure function of the profile and
/// body fields. It is the fallback when the boss has no
/// `sprite_metrics`-driven per-animation hitbox (the gradient sentinel and
/// GNU-ton use `sprite_authored_volumes`), and it is the source
/// `boss_attack_moveset` derives each boss move's `HitVolume`s from at spawn.
pub fn volumes_for_profile(
    attack: &BossAttackProfile,
    pos: ae::Vec2,
    combat_size: ae::Vec2,
    behavior: &BossBehaviorProfile,
) -> Vec<ae::Aabb> {
    // The strike origin: the boss body position shifted by its authored
    // attack offset. Each profile's rects resolve against it.
    let origin = pos + behavior.attack_origin_offset;
    // A boss may author its own rects for this move: an override in
    // `behavior.strike_geometry` (RON, keyed by `move_id`) replaces the
    // built-in table, so a content boss needs no core edit. Empty means the
    // built-in geometry. This one resolve feeds both the debug/pose path and
    // `boss_attack_moveset`'s gameplay `HitVolume`s.
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

// GNU-ton's per-animation hit/hurt-box geometry is in
// `gnu_ton_boss_spritesheet.ron`'s `body_metrics.animations` map, converted by
// the generic `world_aabb_from_pixel_rect` pixel→world transform (as for the
// gradient sentinel).

#[cfg(test)]
mod sprite_metadata_derivation_tests;

#[cfg(test)]
mod simple_geometry_tests;
#[cfg(test)]
mod strike_geometry_data_tests;

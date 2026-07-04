//! Pure boss-attack volume math (no ECS, no mutation).
//!
//! Free functions that derive world-space AABBs for a boss's active strike,
//! telegraph, damageable hurtbox, and body-contact zone, then resolve the
//! per-tick boss -> player `HitEvent`. Inputs are bundled in
//! [`BossVolumeContext`] (body fields + `BossAttackState` + optional
//! [`ActorSpriteMetrics`] + an optional [`BossAnimationFrameSample`]); helpers
//! prefer sprite-author-declared hit/hurtboxes and fall back to
//! `volumes_for_profile`'s hardcoded geometry per `BossAttackProfile`.
//! Submodules: `aabb` (pixel-rect -> world-AABB derivation), `frame`
//! (animation-frame sampling). Distinct from the engine's collision system —
//! this is boss-attack-specific geometry only.

use ambition_engine_core as ae;
use ambition_engine_core::AabbExt;

use bevy::prelude::Component;

use ambition_characters::brain::{BossAttackProfile, BossAttackState};
use ambition_sprite_sheet::{AnimationBox, BodyMetrics, PixelRect};

use super::behavior::{ActorSpriteMetrics, BossBehaviorProfile};

mod aabb;
mod frame;
pub use aabb::*;
use frame::*;

/// All the per-tick inputs the volume helpers need. Owned by the
/// caller so the helpers themselves stay pure.
pub struct BossVolumeContext<'a> {
    pub pos: ae::Vec2,
    pub size: ae::Vec2,
    pub combat_size: ae::Vec2,
    pub behavior: &'a BossBehaviorProfile,
    pub attack_state: &'a BossAttackState,
    /// Sprite-driven body metrics. `Some` for bosses whose sprite
    /// RON carries `body_metrics` and the derivation system has
    /// snapshotted it. `damageable_volumes` prefers multi-rect
    /// hurtboxes from here over the legacy single-AABB fallback.
    pub sprite_metrics: Option<&'a crate::boss_encounter::behavior::ActorSpriteMetrics>,
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
    pub animation_key: Option<&'static str>,
}

impl<'a> BossVolumeContext<'a> {
    /// Build the context from a live boss view + its attack-state component.
    /// The boss contributes only body fields, not policy; volume selection is
    /// data-driven via `sprite_metrics`.
    pub fn from_ref(boss: crate::features::BossRef<'a>, attack_state: &'a BossAttackState) -> Self {
        Self {
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

/// The currently-playing animation row, resolved for hit/hurt-box sampling:
/// the ordered candidate row keys to try, the elapsed time within that row
/// (for deriving a frame from `frame_duration`), and an optional exact frame
/// index from a live animator that overrides the elapsed derivation.
///
/// This is the ONE actor-specific input the shared hurtbox math needs — each
/// actor knows how it picks its current pose (a boss maps an attack profile to
/// rows; a player/enemy just reports its current animation), but once resolved
/// the world-space volume derivation is identical for all of them.
pub struct AnimationSelection {
    pub keys: Vec<&'static str>,
    pub elapsed_s: f32,
    pub live_frame_index: Option<usize>,
}

/// Actor-neutral surface the shared combat-geometry math reads to derive an
/// actor's collision box and damageable hurtbox. Player, NPC, Enemy, and Boss
/// each implement it; the boss is just the richest impl (its `hurtbox_selection`
/// folds in the attack-profile → animation-row mapping). Engine-first: another
/// platformer's actor type unifies onto the same volume math by implementing
/// this trait.
pub trait CombatGeometry {
    fn body_pos(&self) -> ae::Vec2;
    /// LDtk spawn size — the fallback world scale when no sprite render size
    /// was captured, and the size of the legacy single-AABB hurtbox.
    fn body_size(&self) -> ae::Vec2;
    fn facing(&self) -> f32;
    /// Collision envelope; defaults to the body size.
    fn combat_size(&self) -> ae::Vec2 {
        self.body_size()
    }
    /// World offset from `body_pos` to the collision-box center (off-center
    /// bodies). Mirrored with facing by the implementor. Defaults to zero.
    fn combat_offset(&self) -> ae::Vec2 {
        ae::Vec2::ZERO
    }
    /// The actor's reference-frame "down": gravity at its position, or a clung
    /// surface normal for a wall-walker. The body/hurt box orients to this so a
    /// sideways-gravity body's footprint lies along the wall — the relativity
    /// principle. Defaults to screen-down `(0, 1)`; the box is identity under
    /// vertical gravity, so upright play is byte-for-byte unchanged.
    fn frame_down(&self) -> ae::Vec2 {
        ae::Vec2::new(0.0, 1.0)
    }
    fn sprite_metrics(&self) -> Option<&ActorSpriteMetrics>;
    /// The current pose for hurtbox sampling (rest/idle when not attacking).
    fn hurtbox_selection(&self) -> AnimationSelection;
}

/// An actor's collision AABB — its combat-size body box oriented to its
/// reference frame and shifted by any off-center `combat_offset`. The single
/// way to ask "where is this actor's body" across player / NPC / enemy / boss.
pub fn collision_aabb(g: &impl CombatGeometry) -> ae::Aabb {
    let half = ae::AccelerationFrame::new(g.frame_down()).to_world_half(g.combat_size() * 0.5);
    ae::Aabb::new(g.body_pos() + g.combat_offset(), half)
}

/// A minimal [`CombatGeometry`] for an actor whose hurtbox is just its
/// frame-oriented collision box — no per-animation sprite metrics. This is the
/// player and ordinary enemies today: build it from a body's pos / size /
/// facing and its reference-frame down, and `damageable_volumes` /
/// [`collision_aabb`] yield the same box they used before, now through the one
/// shared path. (Sprite metrics — pose-accurate, multi-part hurtboxes — are a
/// later opt-in: populate them and the same call lights up automatically.)
pub struct SimpleActorGeometry {
    pub pos: ae::Vec2,
    pub size: ae::Vec2,
    pub facing: f32,
    pub frame_down: ae::Vec2,
}

impl CombatGeometry for SimpleActorGeometry {
    fn body_pos(&self) -> ae::Vec2 {
        self.pos
    }
    fn body_size(&self) -> ae::Vec2 {
        self.size
    }
    fn facing(&self) -> f32 {
        self.facing
    }
    fn frame_down(&self) -> ae::Vec2 {
        self.frame_down
    }
    fn sprite_metrics(&self) -> Option<&ActorSpriteMetrics> {
        None
    }
    fn hurtbox_selection(&self) -> AnimationSelection {
        AnimationSelection {
            keys: Vec::new(),
            elapsed_s: 0.0,
            live_frame_index: None,
        }
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
        let keys = runtime_animation_keys(self, active_profile, &["rest"]);
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
pub fn active_attack_volumes(ctx: &BossVolumeContext) -> Vec<ae::Aabb> {
    let Some(profile) = ctx.attack_state.active_profile.as_ref() else {
        return Vec::new();
    };
    if let Some(volumes) = sprite_authored_volumes(ctx, profile, ctx.attack_state.active_elapsed) {
        return volumes;
    }
    volumes_for_profile(profile, ctx.pos, ctx.combat_size, ctx.behavior)
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
/// Reflect each AABB's center across the vertical line `axis_x` when `facing`
/// is leftward (`< 0`), leaving sizes unchanged. The boss sprite mirrors to
/// face the player, so an off-center body's hit/hurt boxes must mirror with it;
/// for a centered body this is a no-op (center already on the axis).
pub(crate) fn mirror_x_if_flipped(
    mut aabbs: Vec<ae::Aabb>,
    axis_x: f32,
    facing: f32,
) -> Vec<ae::Aabb> {
    if facing >= 0.0 {
        return aabbs;
    }
    for aabb in &mut aabbs {
        let c = aabb.center();
        let half = aabb.half_size();
        *aabb = ae::Aabb::new(ae::Vec2::new(2.0 * axis_x - c.x, c.y), half);
    }
    aabbs
}

pub fn damageable_volumes(g: &impl CombatGeometry) -> Vec<ae::Aabb> {
    mirror_x_if_flipped(damageable_volumes_unmirrored(g), g.body_pos().x, g.facing())
}

/// Body hurtbox volumes in the sprite's UNFLIPPED frame. `damageable_volumes`
/// mirrors these to the actor's current facing. Actor-neutral: every input is
/// read through the [`CombatGeometry`] trait, so player / enemy / boss share
/// one hurtbox derivation.
fn damageable_volumes_unmirrored(g: &impl CombatGeometry) -> Vec<ae::Aabb> {
    // Priority:
    //   1. Per-animation hurtbox for the currently-playing animation
    //      (attack frames with extended arms get a wider hurtbox than the
    //      rest pose; a multi-part actor's per-pose rows carve out body
    //      pieces — e.g. GNU-ton's head-only descent hurtbox).
    //   2. Static `body_pixel_parts` (multi-rect body for disjointed actors).
    //   3. Static `body_pixel_bbox` (single-rect alpha bbox).
    //   4. `combat_size`-driven fallback (actors without sprite metadata).
    if let Some(metrics) = g.sprite_metrics() {
        // Scale pixel rects to the visible sprite size, not the smaller LDtk
        // spawn AABB. See `sprite_world_size` for the rationale.
        let world_size = sprite_world_size(metrics, g.body_size());
        let pos = g.body_pos();
        // (1) Per-animation hurtbox for the actor's current pose. The actor
        // resolves which row(s) it is showing; the sampling is uniform.
        let sel = g.hurtbox_selection();
        for active_anim in &sel.keys {
            let Some(entry) = metrics.animations.get(*active_anim) else {
                continue;
            };
            let Some(box_) = entry.hurtbox.as_ref() else {
                continue;
            };
            if !box_.is_populated() {
                continue;
            }
            // A live animator frame wins; otherwise derive from elapsed.
            let frame_index = sel
                .live_frame_index
                .or_else(|| animation_frame_index(entry, sel.elapsed_s));
            let aabbs = world_space_animation_box_aabbs(
                box_,
                frame_index,
                metrics.frame_width,
                metrics.frame_height,
                pos,
                world_size,
            );
            if !aabbs.is_empty() {
                return aabbs;
            }
        }
        // (2) Static multi-part body.
        if !metrics.body_pixel_parts.is_empty() {
            let mut parts = Vec::with_capacity(metrics.body_pixel_parts.len());
            for part in &metrics.body_pixel_parts {
                parts.push(world_aabb_from_pixel_rect(
                    part.rect(),
                    metrics.frame_width,
                    metrics.frame_height,
                    pos,
                    world_size,
                ));
            }
            return parts;
        }
        // (3) Static single-rect body.
        if let Some(bbox) = metrics.body_pixel_bbox {
            return vec![world_aabb_from_pixel_rect(
                bbox,
                metrics.frame_width,
                metrics.frame_height,
                pos,
                world_size,
            )];
        }
    }
    // (4) Fallback: combat_size-driven single AABB, oriented to the actor's
    // reference frame (identity under vertical gravity, so bosses — which keep
    // the default screen-down frame — are unchanged).
    let half = ae::AccelerationFrame::new(g.frame_down()).to_world_half(g.combat_size() * 0.5);
    vec![ae::Aabb::new(g.body_pos(), half)]
}

/// Body-contact damage AABB at the boss's combat envelope — body contact is
/// "you ran into the boss", not a discrete strike.
pub fn body_damage_aabb(pos: ae::Vec2, combat_size: ae::Vec2) -> ae::Aabb {
    ae::Aabb::new(pos, combat_size * 0.5)
}

// `boss_attack_damage` is GONE (fable AD2): a boss's offense flows through the ONE
// set of systems every actor uses — STRIKE damage via the boss's own moveset moves
// (`trigger_boss_attack_moves` → `advance_move_playback` → `apply_hitbox_damage`'s Boss
// branch; fable review §A1 retired the per-tick `sync_boss_strike_hitboxes` poll), and
// BODY-CONTACT damage via the shared `apply_actor_contact_damage` (the boss's contact
// tuning is driven from `behavior.body_damage` at spawn). No bespoke boss damage poll.
//
// NOTE: `active_attack_volumes` / `volumes_for_profile` below are now consumed only by
// the DEBUG overlay (telegraph/strike gizmos) and the hurtbox-pose selection — the
// gameplay strike geometry is authored into each boss move's `HitVolume`s at spawn
// (`boss_attack_moveset`). The sprite-frame-tracking multi-part geometry those helpers
// still express is the fidelity the static move volumes approximate (bulk-review).

/// One body-local strike rectangle, expressed as DATA rather than an imperative
/// `match` arm (fable review §C6: "collapse the named-boss geometry toward authored
/// rect DATA"). Both the center offset and the half-extent are an AFFINE function of
/// the boss's combat size — `factor * size + const` — so the shape scales with any
/// boss body while a fixed pixel margin (a floor-slam's 22px reach past the feet, its
/// 18px slab thickness) stays fixed. `serde`-ready so a content boss can eventually
/// AUTHOR its strike geometry (in the boss roster RON) instead of a core enum variant —
/// the "second game adds a boss without editing core" oracle. Today the built-in
/// per-profile tables below supply it; an authored override is the next slice.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct StrikeRect {
    /// Center offset from the strike origin, as a fraction of the body size.
    pub offset_factor: ae::Vec2,
    /// Center offset from the strike origin, as a fixed pixel amount (added).
    #[serde(default)]
    pub offset_const: ae::Vec2,
    /// Half-extent, as a fraction of the body size.
    pub half_factor: ae::Vec2,
    /// Half-extent, as a fixed pixel amount (added).
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
/// 2026-05-26) GNU-ton route through `sprite_authored_volumes` instead — the geometry
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
mod simple_geometry_tests {
    use super::*;

    fn geom(frame_down: ae::Vec2) -> SimpleActorGeometry {
        SimpleActorGeometry {
            pos: ae::Vec2::new(10.0, 20.0),
            size: ae::Vec2::new(30.0, 48.0),
            facing: 1.0,
            frame_down,
        }
    }

    #[test]
    fn upright_gravity_is_the_plain_centered_box() {
        // Under screen-down gravity the oriented body box is identity — upright
        // play stays byte-for-byte the old `kin.aabb()` / CenteredAabb.
        let aabb = collision_aabb(&geom(ae::Vec2::new(0.0, 1.0)));
        assert_eq!(aabb.center(), ae::Vec2::new(10.0, 20.0));
        assert_eq!(aabb.half_size(), ae::Vec2::new(15.0, 24.0));
        // The single damageable volume agrees with the collision box.
        let vols = damageable_volumes(&geom(ae::Vec2::new(0.0, 1.0)));
        assert_eq!(vols.len(), 1);
        assert_eq!(vols[0].half_size(), ae::Vec2::new(15.0, 24.0));
    }

    #[test]
    fn sideways_gravity_lays_the_body_along_the_wall() {
        // Under sideways gravity the footprint rotates: width<->height swap so
        // the body lies along the wall (the relativity principle). Same box the
        // gizmo's `aabb_oriented` draws.
        let aabb = collision_aabb(&geom(ae::Vec2::new(1.0, 0.0)));
        assert_eq!(aabb.center(), ae::Vec2::new(10.0, 20.0));
        assert_eq!(aabb.half_size(), ae::Vec2::new(24.0, 15.0));
    }
}

#[cfg(test)]
mod strike_geometry_data_tests {
    use super::*;
    use ambition_characters::brain::BossAttackProfile;

    /// The ORIGINAL hardcoded `volumes_for_profile` arms, verbatim — the reference the
    /// `StrikeRect` DATA table (fable §C6) must reproduce byte-for-byte.
    fn reference(attack: &BossAttackProfile, origin: ae::Vec2, size: ae::Vec2) -> Vec<ae::Aabb> {
        match attack.move_id().as_str() {
            "floor_slam" => vec![ae::Aabb::new(
                origin + ae::Vec2::new(0.0, size.y * 0.5 + 22.0),
                ae::Vec2::new(size.x * 0.75, 18.0),
            )],
            "side_sweep" => vec![
                ae::Aabb::new(
                    origin + ae::Vec2::new(-size.x * 0.50, 0.0),
                    ae::Vec2::new(size.x * 0.25, size.y * 0.72),
                ),
                ae::Aabb::new(
                    origin + ae::Vec2::new(size.x * 0.50, 0.0),
                    ae::Vec2::new(size.x * 0.25, size.y * 0.72),
                ),
            ],
            "full_body_pulse" => vec![ae::Aabb::new(origin, size * 0.70)],
            "hazard_column" => vec![ae::Aabb::new(
                origin + ae::Vec2::new(0.0, 0.0),
                ae::Vec2::new(size.x * 0.30, size.y * 1.80),
            )],
            "wing_sweep" => vec![ae::Aabb::new(
                origin + ae::Vec2::new(0.0, size.y * 0.08),
                ae::Vec2::new(size.x * 0.56, size.y * 0.42),
            )],
            "dive_lane" => vec![ae::Aabb::new(
                origin + ae::Vec2::new(0.0, size.y * 0.42),
                ae::Vec2::new(size.x * 0.22, size.y * 0.72),
            )],
            "broadside" => vec![
                ae::Aabb::new(
                    origin + ae::Vec2::new(-size.x * 0.34, 0.0),
                    ae::Vec2::new(size.x * 0.18, size.y * 0.84),
                ),
                ae::Aabb::new(
                    origin + ae::Vec2::new(size.x * 0.34, 0.0),
                    ae::Vec2::new(size.x * 0.18, size.y * 0.84),
                ),
            ],
            "hand_slam" => vec![
                ae::Aabb::new(
                    origin + ae::Vec2::new(-size.x * 0.40, size.y * 0.25),
                    ae::Vec2::new(size.x * 0.14, size.y * 0.60),
                ),
                ae::Aabb::new(
                    origin + ae::Vec2::new(size.x * 0.40, size.y * 0.25),
                    ae::Vec2::new(size.x * 0.14, size.y * 0.60),
                ),
            ],
            "hand_sweep" => vec![ae::Aabb::new(
                origin + ae::Vec2::new(0.0, size.y * 0.15),
                ae::Vec2::new(size.x * 0.85, size.y * 0.28),
            )],
            "head_descent" => vec![ae::Aabb::new(
                origin + ae::Vec2::new(0.0, size.y * 0.05),
                ae::Vec2::new(size.x * 0.32, size.y * 0.38),
            )],
            "converging_shockwave" => vec![ae::Aabb::new(
                origin + ae::Vec2::new(0.0, size.y * 0.48),
                ae::Vec2::new(size.x * 0.90, size.y * 0.08),
            )],
            // Special (or any non-geometry key) carries no body-mounted volume.
            _ => Vec::new(),
        }
    }

    #[test]
    fn strike_geometry_is_byte_identical_to_the_old_hardcoded_match() {
        let profiles = [
            BossAttackProfile::Strike("floor_slam".to_string()),
            BossAttackProfile::Strike("side_sweep".to_string()),
            BossAttackProfile::Strike("full_body_pulse".to_string()),
            BossAttackProfile::Strike("wing_sweep".to_string()),
            BossAttackProfile::Strike("dive_lane".to_string()),
            BossAttackProfile::Strike("broadside".to_string()),
            BossAttackProfile::Strike("hand_slam".to_string()),
            BossAttackProfile::Strike("hand_sweep".to_string()),
            BossAttackProfile::Strike("head_descent".to_string()),
            BossAttackProfile::Strike("converging_shockwave".to_string()),
            BossAttackProfile::Strike("hazard_column".to_string()),
            BossAttackProfile::Special("overfit_volley".to_string()),
        ];
        // Sweep a couple of origins + body sizes so the affine `factor*size + const`
        // resolve is checked across scales (FloorSlam's fixed 22/18 px terms must NOT
        // scale; every other factor must).
        for origin in [ae::Vec2::ZERO, ae::Vec2::new(120.0, -40.0)] {
            for size in [ae::Vec2::new(30.0, 48.0), ae::Vec2::new(64.0, 96.0)] {
                for p in &profiles {
                    let got: Vec<ae::Aabb> = strike_geometry(&p.move_id())
                        .iter()
                        .map(|r| r.to_aabb(origin, size))
                        .collect();
                    let want = reference(p, origin, size);
                    assert_eq!(got.len(), want.len(), "{p:?} volume count");
                    for (g, w) in got.iter().zip(want.iter()) {
                        assert_eq!(g.center(), w.center(), "{p:?} center @ size {size:?}");
                        assert_eq!(g.half_size(), w.half_size(), "{p:?} half @ size {size:?}");
                    }
                }
            }
        }
    }

    /// §C6 "out of core": a boss AUTHORS its own strike rects in its behavior profile
    /// (RON-loaded here from the fixture), and that override REPLACES the built-in
    /// geometry for exactly that move — while every other profile keeps the built-in
    /// table. This is the seam a second game's boss uses to supply strike shapes with
    /// no edit to core's `strike_geometry`.
    #[test]
    fn an_authored_override_replaces_the_built_in_geometry_for_that_move() {
        use crate::boss_encounter::behavior::BossBehaviorProfile;

        let mut behavior = BossBehaviorProfile::from_data("clockwork_warden");
        let size = ae::Vec2::new(80.0, 80.0);
        let pos = ae::Vec2::new(200.0, 100.0);
        let origin = pos + behavior.attack_origin_offset;

        // Author a single bespoke rect for the floor_slam move — deliberately unlike
        // the built-in FloorSlam so the swap is unambiguous.
        let authored = StrikeRect::scaled(ae::Vec2::new(0.0, 1.0), ae::Vec2::new(0.40, 0.40));
        behavior
            .strike_geometry
            .insert("floor_slam".to_string(), vec![authored]);

        // FloorSlam now resolves to the AUTHORED rect, not the built-in slab.
        let slam = volumes_for_profile(
            &BossAttackProfile::Strike("floor_slam".to_string()),
            pos,
            size,
            &behavior,
        );
        assert_eq!(slam.len(), 1);
        assert_eq!(slam[0].center(), authored.to_aabb(origin, size).center());
        assert_eq!(
            slam[0].half_size(),
            authored.to_aabb(origin, size).half_size()
        );
        assert_ne!(
            slam[0].half_size(),
            FLOOR_SLAM[0].to_aabb(origin, size).half_size(),
            "the override must NOT equal the built-in FloorSlam geometry"
        );

        // A profile with NO authored override still uses the built-in table.
        let sweep = volumes_for_profile(
            &BossAttackProfile::Strike("side_sweep".to_string()),
            pos,
            size,
            &behavior,
        );
        assert_eq!(
            sweep.len(),
            2,
            "SideSweep keeps its built-in two-box geometry"
        );
        assert_eq!(
            sweep[0].center(),
            SIDE_SWEEP[0].to_aabb(origin, size).center()
        );
    }
}

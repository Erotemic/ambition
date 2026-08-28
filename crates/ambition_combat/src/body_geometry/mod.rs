//! ACTOR-NEUTRAL COMBAT GEOMETRY: how a body's collision box and its damageable
//! hurtbox are derived from its pose.
//!
//! ⭐⭐ IT LIVED IN `ambition_boss_encounter` BECAUSE A BOSS WAS THE RICHEST
//! IMPLEMENTATION, not because any of it is about bosses. `CombatGeometry`'s own
//! doc says so: *"Player, NPC, Enemy, and Boss each implement it … another
//! platformer's actor type unifies onto the same volume math by implementing this
//! trait."* A trait that says that, in a crate named for one of its four
//! implementors, is a boundary drawn by whoever needed it first.
//!
//! ⛔ WHAT STAYED BEHIND, and it is the whole boss half: `BossVolumeContext` (the
//! boss's own `CombatGeometry` impl — an impl belongs with the type it is FOR),
//! `BossAnimationFrameSample`, `active_attack_volumes`, `volumes_for_profile` and
//! the per-profile strike-geometry table. The split is clean because the universal
//! half named none of them.
//!
//! Moved 2026-08-28 (D117), after `ActorSpriteMetrics` left for
//! `ambition_sprite_sheet` — which is what made the rest of it nameable here.

use ambition_platformer2d_core as ae;
use ambition_sprite_sheet::{ActorSpriteMetrics, AnimationBox};

pub use aabb::*;

mod aabb;

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
    pub keys: Vec<String>,
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
/// THE body-footprint publish — write a body's oriented collision box into
/// the [`ae::CenteredAabb`] every consumer reads (the debug overlay, hurtbox
/// resolution, target volumes).
///
/// Two spellings of a rule that names itself universal is the shape was, one layer over.
///
/// `footprint` is the body's collision size, or a boss's render envelope where
/// one is carried. `frame_down` is the body's reference-frame down — gravity at
/// its position, or a clung surface normal for a wall-walker — so a
/// sideways-gravity body's box lies along the wall.
pub fn publish_body_footprint(
    out: &mut ae::CenteredAabb,
    pos: ae::Vec2,
    footprint: ae::Vec2,
    facing: f32,
    frame_down: ae::Vec2,
) {
    use ae::AabbExt;
    let body = collision_aabb(&SimpleActorGeometry {
        pos,
        size: footprint,
        facing,
        frame_down,
    });
    out.center = body.center();
    out.half_size = body.half_size();
}

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

/// Reflect each AABB's center across the vertical line `axis_x` when `facing`
/// is leftward (`< 0`), leaving sizes unchanged. The boss sprite mirrors to
/// face the player, so an off-center body's hit/hurt boxes must mirror with it;
/// for a centered body this is a no-op (center already on the axis).
// ⭐ `pub` for the same reason `world_aabb_from_pixel_rect` is: it was
// `pub(crate)` where the only caller was in-crate, and the crate boundary moved.
pub fn mirror_x_if_flipped(
    mut volumes: Vec<ae::CombatVolume>,
    axis_x: f32,
    facing: f32,
) -> Vec<ae::CombatVolume> {
    if facing >= 0.0 {
        return volumes;
    }
    for volume in &mut volumes {
        *volume = volume.mirrored_x(axis_x);
    }
    volumes
}

pub fn damageable_volumes(g: &impl CombatGeometry) -> Vec<ae::CombatVolume> {
    mirror_x_if_flipped(damageable_volumes_unmirrored(g), g.body_pos().x, g.facing())
}

/// Body hurtbox volumes in the sprite's UNFLIPPED frame. `damageable_volumes`
/// mirrors these to the actor's current facing. Actor-neutral: every input is
/// read through the [`CombatGeometry`] trait, so player / enemy / boss share
/// one hurtbox derivation.
fn damageable_volumes_unmirrored(g: &impl CombatGeometry) -> Vec<ae::CombatVolume> {
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
            let Some(entry) = metrics.animations.get(active_anim) else {
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
            let volumes = world_space_animation_box_volumes(
                box_,
                frame_index,
                metrics.frame_width,
                metrics.frame_height,
                pos,
                world_size,
            );
            if !volumes.is_empty() {
                return volumes;
            }
        }
        // (2) Static multi-part body.
        if !metrics.body_pixel_parts.is_empty() {
            let mut parts = Vec::with_capacity(metrics.body_pixel_parts.len());
            for part in &metrics.body_pixel_parts {
                parts.push(ae::CombatVolume::aabb(world_aabb_from_pixel_rect(
                    part.rect(),
                    metrics.frame_width,
                    metrics.frame_height,
                    pos,
                    world_size,
                )));
            }
            return parts;
        }
        // (3) Static single-rect body.
        if let Some(bbox) = metrics.body_pixel_bbox {
            return vec![ae::CombatVolume::aabb(world_aabb_from_pixel_rect(
                bbox,
                metrics.frame_width,
                metrics.frame_height,
                pos,
                world_size,
            ))];
        }
    }
    // (4) Fallback: combat_size-driven single AABB, oriented to the actor's
    // reference frame (identity under vertical gravity, so bosses — which keep
    // the default screen-down frame — are unchanged).
    let half = ae::AccelerationFrame::new(g.frame_down()).to_world_half(g.combat_size() * 0.5);
    vec![ae::CombatVolume::aabb(ae::Aabb::new(g.body_pos(), half))]
}

/// Body-contact damage AABB at the boss's combat envelope — body contact is
/// "you ran into the boss", not a discrete strike.
pub fn body_damage_aabb(pos: ae::Vec2, combat_size: ae::Vec2) -> ae::Aabb {
    ae::Aabb::new(pos, combat_size * 0.5)
}

// ⭐ THESE TWO CAME WITH THE MATH, and they were `pub(super)` in the boss crate's
// `frame` module: choosing a world size to scale pixel rects against, and asking
// a sheet which frame is showing. Neither mentions a boss. The universal half
// called them, which is the only reason they looked boss-side.

/// Choose the world-space size to scale sprite-pixel rects against.
/// Prefer the metrics-captured render size (set by
/// `derive_boss_sprite_metrics` from the sheet spec's
/// `collision_scale`). Fall back to `ctx.size` when the snapshot
/// didn't capture one — test fixtures that build `ActorSpriteMetrics`
/// by hand can leave `sprite_render_size = Vec2::ZERO` to opt out.
pub fn sprite_world_size(
    metrics: &ambition_sprite_sheet::ActorSpriteMetrics,
    fallback: ae::Vec2,
) -> ae::Vec2 {
    if metrics.sprite_render_size.x > 0.0 && metrics.sprite_render_size.y > 0.0 {
        metrics.sprite_render_size
    } else {
        fallback
    }
}

pub fn animation_frame_index(
    entry: &ambition_sprite_sheet::AnimationMetrics,
    elapsed_s: f32,
) -> Option<usize> {
    ambition_sprite_sheet::frame_at(entry, elapsed_s)
}

/// World-space volumes for one authored animation box, at the frame being shown.
///
/// Per-frame data still outranks the coarse per-animation box, so a large moving
/// part (GNU-ton's head) tracks the drawn pose rather than one average.
pub fn world_space_animation_box_volumes(
    box_: &AnimationBox,
    frame_index: Option<usize>,
    frame_width: u32,
    frame_height: u32,
    world_center: ae::Vec2,
    world_size: ae::Vec2,
) -> Vec<ae::CombatVolume> {
    let hull = |poly: &[(f32, f32)]| {
        ae::CombatVolume::convex(
            poly.iter()
                .map(|(x, y)| {
                    world_point_from_pixel(
                        *x,
                        *y,
                        frame_width,
                        frame_height,
                        world_center,
                        world_size,
                    )
                })
                .collect(),
        )
    };
    let boxes = |parts: &[ambition_sprite_sheet::NamedPixelRect], bbox| {
        // A part that authored a hull IS that hull; the rest fall back to their
        // rects. Mixing the two in one silhouette is normal — a hooded head is
        // shaped and a shoulder pad is honestly a box.
        if parts.iter().any(|part| !part.poly.is_empty()) {
            return parts
                .iter()
                .map(|part| {
                    if part.poly.is_empty() {
                        ae::CombatVolume::aabb(world_aabb_from_pixel_rect(
                            part.rect(),
                            frame_width,
                            frame_height,
                            world_center,
                            world_size,
                        ))
                    } else {
                        hull(&part.poly)
                    }
                })
                .collect::<Vec<_>>();
        }
        world_space_body_aabbs_from_parts(
            parts,
            bbox,
            frame_width,
            frame_height,
            world_center,
            world_size,
        )
        .into_iter()
        .map(ae::CombatVolume::aabb)
        .collect::<Vec<_>>()
    };
    // WHICH authored shape this frame shows is `frame_space::sample`'s call, and
    // the character attack path asks the same question through the same
    // function — the precedence (per-frame outranks per-animation, hull
    // outranks rectangles) has one home rather than a copy per consumer.
    match ambition_sprite_sheet::SampledBox::sample(box_, frame_index) {
        None => Vec::new(),
        Some(ambition_sprite_sheet::SampledBox::Poly(poly)) => vec![hull(poly)],
        Some(ambition_sprite_sheet::SampledBox::Rects(parts, bbox)) => boxes(parts, bbox),
    }
}

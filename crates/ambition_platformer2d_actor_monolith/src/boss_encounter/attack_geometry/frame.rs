//! Sprite animation-frame sampling — picks the authored/runtime frame and its
//! world-space animation-box AABBs that the volume queries read.
//!
//! Resolves which animation row + frame a boss is showing (preferring a live
//! `BossAnimationFrameSample` over elapsed-time sampling), the world size to
//! scale against (`sprite_world_size`), and the per-frame hit/hurtbox AABBs
//! (`sprite_authored_volumes`, `world_space_animation_box_aabbs`). Consumed by
//! `mod`'s `active_attack_volumes` / `telegraph_volumes` / `damageable_volumes`.

use super::*;

pub(super) fn sprite_authored_volumes(
    ctx: &BossVolumeContext,
    profile: &BossAttackProfile,
    animation_elapsed_s: f32,
) -> Option<Vec<ae::CombatVolume>> {
    let metrics = ctx.sprite_metrics?;
    // Use the SPRITE RENDER SIZE (not `ctx.size`) — that's the
    // world-space extent of the visible sprite quad. `ctx.size` is
    // the LDtk spawn AABB which is smaller than the rendered sprite
    // (collision_scale > 1.0 in every sheet spec). Using ctx.size
    // would render hitboxes at half the visible size of the attack.
    let world_size = sprite_world_size(metrics, ctx.size);
    for animation in
        crate::boss_encounter::behavior::boss_animation_keys_for_profile(ctx.boss_catalog, profile)
    {
        let Some(entry) = metrics.animations.get(&animation) else {
            continue;
        };
        let Some(hitbox) = entry.hitbox.as_ref() else {
            continue;
        };
        if !hitbox.is_populated() {
            continue;
        }
        let selected_frame =
            authored_animation_frame_index(ctx, profile, entry, animation_elapsed_s);
        let volumes = world_space_animation_box_volumes(
            hitbox,
            selected_frame,
            metrics.frame_width,
            metrics.frame_height,
            ctx.pos,
            world_size,
        );
        if !volumes.is_empty() {
            return Some(volumes);
        }
    }
    None
}

/// Choose the world-space size to scale sprite-pixel rects against.
/// Prefer the metrics-captured render size (set by
/// `derive_boss_sprite_metrics` from the sheet spec's
/// `collision_scale`). Fall back to `ctx.size` when the snapshot
/// didn't capture one — test fixtures that build `ActorSpriteMetrics`
/// by hand can leave `sprite_render_size = Vec2::ZERO` to opt out.
pub(super) fn sprite_world_size(
    metrics: &crate::boss_encounter::behavior::ActorSpriteMetrics,
    fallback: ae::Vec2,
) -> ae::Vec2 {
    if metrics.sprite_render_size.x > 0.0 && metrics.sprite_render_size.y > 0.0 {
        metrics.sprite_render_size
    } else {
        fallback
    }
}

pub(super) fn animation_frame_index(
    entry: &ambition_sprite_sheet::AnimationMetrics,
    elapsed_s: f32,
) -> Option<usize> {
    let frame_duration = entry.frame_duration_secs?;
    if frame_duration <= 0.0 {
        return None;
    }
    Some((elapsed_s.max(0.0) / frame_duration).floor() as usize)
}

pub(super) fn authored_animation_frame_index(
    ctx: &BossVolumeContext,
    profile: &BossAttackProfile,
    entry: &ambition_sprite_sheet::AnimationMetrics,
    elapsed_s: f32,
) -> Option<usize> {
    if let Some(sample) = ctx.animation_frame {
        if sample.profile.as_ref() == Some(profile) {
            return Some(sample.frame_index);
        }
    }
    animation_frame_index(entry, elapsed_s)
}

pub(super) fn push_unique_animation_key(keys: &mut Vec<String>, key: &str) {
    if !key.is_empty() && !keys.iter().any(|existing| *existing == key) {
        keys.push(key.to_string());
    }
}

pub(super) fn runtime_animation_keys(
    ctx: &BossVolumeContext,
    active_profile: Option<&BossAttackProfile>,
    rest_keys: &[&'static str],
) -> Vec<String> {
    let mut keys: Vec<String> = Vec::new();
    if let (Some(sample), Some(profile)) = (ctx.animation_frame, active_profile) {
        if sample.profile.as_ref() == Some(profile) {
            if let Some(animation_key) = sample.animation_key.as_deref() {
                push_unique_animation_key(&mut keys, animation_key);
            }
        }
    }
    let mapped_keys = active_profile
        .map(|profile| {
            crate::boss_encounter::behavior::boss_animation_keys_for_profile(
                ctx.boss_catalog,
                profile,
            )
        })
        .unwrap_or_else(|| rest_keys.iter().map(|key| (*key).to_string()).collect());
    for key in mapped_keys {
        push_unique_animation_key(&mut keys, &key);
    }
    keys
}

/// World-space volumes for one authored animation box, at the frame being shown.
///
/// **Precedence: `poly` wins.** An authored hull is the shape; `parts`/`bbox`
/// are what a body without one publishes, and what a consumer that could not
/// express a hull used to read INSTEAD of one. That was the split this campaign
/// closed: the same authored attack resolved to a cone on the player path and
/// to a differently-sized rectangle on this one, silently, because this returned
/// `Vec<Aabb>` and could not say anything else.
///
/// Per-frame data still outranks the coarse per-animation box, so a large moving
/// part (GNU-ton's head) tracks the drawn pose rather than one average.
pub(super) fn world_space_animation_box_volumes(
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
    if let Some(index) = frame_index {
        if let Some(frame) = box_
            .frames
            .get(index.min(box_.frames.len().saturating_sub(1)))
        {
            if frame.is_populated() {
                if !frame.poly.is_empty() {
                    return vec![hull(&frame.poly)];
                }
                return boxes(&frame.parts, frame.bbox);
            }
        }
    }
    if !box_.poly.is_empty() {
        return vec![hull(&box_.poly)];
    }
    boxes(&box_.parts, box_.bbox)
}

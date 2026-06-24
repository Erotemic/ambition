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
) -> Option<Vec<ae::Aabb>> {
    let metrics = ctx.sprite_metrics?;
    // Use the SPRITE RENDER SIZE (not `ctx.size`) — that's the
    // world-space extent of the visible sprite quad. `ctx.size` is
    // the LDtk spawn AABB which is smaller than the rendered sprite
    // (collision_scale > 1.0 in every sheet spec). Using ctx.size
    // would render hitboxes at half the visible size of the attack.
    let world_size = sprite_world_size(metrics, ctx.size);
    for animation in crate::boss_encounter::behavior::boss_animation_keys_for_profile(profile) {
        let Some(entry) = metrics.animations.get(*animation) else {
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
        let aabbs = world_space_animation_box_aabbs(
            hitbox,
            selected_frame,
            metrics.frame_width,
            metrics.frame_height,
            ctx.pos,
            world_size,
        );
        if !aabbs.is_empty() {
            return Some(aabbs);
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

/// Idle-pose frame index. Mirrors [`authored_animation_frame_index`]
/// for the rest pose: prefer the live rendered frame carried by an
/// idle (`profile: None`) sample so the rest-pose hurtbox bobs with
/// the breathing animation, falling back to elapsed-time sampling
/// (which, with the idle elapsed of 0, would lock to frame 0).
pub(super) fn idle_animation_frame_index(
    ctx: &BossVolumeContext,
    entry: &ambition_sprite_sheet::AnimationMetrics,
    elapsed_s: f32,
) -> Option<usize> {
    if let Some(sample) = ctx.animation_frame {
        if sample.profile.is_none() {
            return Some(sample.frame_index);
        }
    }
    animation_frame_index(entry, elapsed_s)
}

pub(super) fn push_unique_animation_key<'a>(keys: &mut Vec<&'a str>, key: &'a str) {
    if !key.is_empty() && !keys.iter().any(|existing| *existing == key) {
        keys.push(key);
    }
}

pub(super) fn runtime_animation_keys<'a>(
    ctx: &'a BossVolumeContext<'a>,
    active_profile: Option<&'a BossAttackProfile>,
    rest_keys: &'a [&'a str],
) -> Vec<&'a str> {
    let mut keys = Vec::new();
    if let (Some(sample), Some(profile)) = (ctx.animation_frame, active_profile) {
        if sample.profile.as_ref() == Some(profile) {
            if let Some(animation_key) = sample.animation_key {
                push_unique_animation_key(&mut keys, animation_key);
            }
        }
    }
    let mapped_keys = active_profile
        .map(crate::boss_encounter::behavior::boss_animation_keys_for_profile)
        .unwrap_or(rest_keys);
    for key in mapped_keys {
        push_unique_animation_key(&mut keys, key);
    }
    keys
}

pub(super) fn world_space_animation_box_aabbs(
    box_: &AnimationBox,
    frame_index: Option<usize>,
    frame_width: u32,
    frame_height: u32,
    world_center: ae::Vec2,
    world_size: ae::Vec2,
) -> Vec<ae::Aabb> {
    if let Some(index) = frame_index {
        if let Some(frame) = box_
            .frames
            .get(index.min(box_.frames.len().saturating_sub(1)))
        {
            if frame.is_populated() {
                return world_space_body_aabbs_from_parts(
                    &frame.parts,
                    frame.bbox,
                    frame_width,
                    frame_height,
                    world_center,
                    world_size,
                );
            }
        }
    }
    world_space_body_aabbs_from_parts(
        &box_.parts,
        box_.bbox,
        frame_width,
        frame_height,
        world_center,
        world_size,
    )
}

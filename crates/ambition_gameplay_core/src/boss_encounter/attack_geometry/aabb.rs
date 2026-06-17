//! Pure AABB geometry derivation for boss attack volumes.
//!
//! Transforms sprite-frame pixel rects (`body_pixel_bbox` /
//! `body_pixel_parts`) into world-space AABBs given a render position + size
//! (`world_aabb_from_pixel_rect`, `world_space_body_aabbs_from_parts` /
//! `_from_metrics`). `bounding_aabb` collapses multi-part bodies into one box
//! for movement/clamping. Consumed by `mod` and `frame`.

use super::*;

// =================================================================
// Sprite-metadata-driven body AABB derivation
// =================================================================
//
// The sprite generator emits per-sheet `body_metrics` carrying
// `body_pixel_bbox` (single overall body) and/or `body_pixel_parts`
// (named multi-rect for disjointed-piece characters like a giant
// boss with head + body + arms).
//
// These helpers turn that pixel-space metadata into world-space
// AABBs given the rendered position + render size, so gameplay
// systems (combat_size derivation, damageable_volumes, contact
// damage) can read a single source of truth — the sprite — instead
// of duplicating hardcoded numbers per boss.

/// Derive a single world-space AABB from one pixel rectangle in the
/// sprite-frame coordinate system, given the rendered size and
/// frame dimensions.
///
/// Sprite-frame coords: origin at top-left, y growing downward (the
/// image-space convention the generator emits).
///
/// World coords here: origin at the *center* of the rendered
/// sprite; y also grows downward in Ambition's world.
pub(super) fn world_aabb_from_pixel_rect(
    bbox: PixelRect,
    frame_width: u32,
    frame_height: u32,
    world_center: ae::Vec2,
    world_size: ae::Vec2,
) -> ae::Aabb {
    let fw = frame_width.max(1) as f32;
    let fh = frame_height.max(1) as f32;
    let scale = ae::Vec2::new(world_size.x / fw, world_size.y / fh);
    let frame_center_x = fw * 0.5;
    let frame_center_y = fh * 0.5;
    let center_offset = ae::Vec2::new(
        (bbox.x as f32 + bbox.w as f32 * 0.5) - frame_center_x,
        (bbox.y as f32 + bbox.h as f32 * 0.5) - frame_center_y,
    );
    let center = world_center + ae::Vec2::new(center_offset.x * scale.x, center_offset.y * scale.y);
    let half = ae::Vec2::new(
        (bbox.w as f32 * 0.5 * scale.x).abs(),
        (bbox.h as f32 * 0.5 * scale.y).abs(),
    );
    ae::Aabb::new(center, half)
}

/// Build the full list of world-space body AABBs for a sprite-driven
/// actor from raw metadata parts. Both the registry's `BodyMetrics`
/// and the gameplay snapshot `BossSpriteMetrics` flow through here
/// — pass `body_pixel_parts` (preferred) and `body_pixel_bbox`
/// (fallback) directly.
///
/// Multi-part input emits one AABB per part; single-piece input
/// emits one AABB from the bbox; empty input returns `Vec::new()`.
/// Callers should treat empty-result as a signal to fall back to
/// the legacy `world_size`-driven AABB rather than the sprite
/// path.
pub fn world_space_body_aabbs_from_parts(
    body_pixel_parts: &[ambition_sprite_sheet::NamedPixelRect],
    body_pixel_bbox: Option<PixelRect>,
    frame_width: u32,
    frame_height: u32,
    world_center: ae::Vec2,
    world_size: ae::Vec2,
) -> Vec<ae::Aabb> {
    if !body_pixel_parts.is_empty() {
        return body_pixel_parts
            .iter()
            .map(|p| {
                world_aabb_from_pixel_rect(
                    p.rect(),
                    frame_width,
                    frame_height,
                    world_center,
                    world_size,
                )
            })
            .collect();
    }
    if let Some(bbox) = body_pixel_bbox {
        return vec![world_aabb_from_pixel_rect(
            bbox,
            frame_width,
            frame_height,
            world_center,
            world_size,
        )];
    }
    Vec::new()
}

/// Convenience wrapper that accepts the registry's `BodyMetrics`
/// struct directly. Equivalent to calling
/// [`world_space_body_aabbs_from_parts`] with the metrics' fields
/// expanded.
pub fn world_space_body_aabbs_from_metrics(
    metrics: &BodyMetrics,
    frame_width: u32,
    frame_height: u32,
    world_center: ae::Vec2,
    world_size: ae::Vec2,
) -> Vec<ae::Aabb> {
    world_space_body_aabbs_from_parts(
        &metrics.body_pixel_parts,
        metrics.body_pixel_bbox,
        frame_width,
        frame_height,
        world_center,
        world_size,
    )
}

/// Tight bounding box around a list of AABBs. Used to collapse
/// multi-part body AABBs into a single `combat_size` for movement
/// + soft world-bounds clamping. `None` for empty input.
pub fn bounding_aabb(parts: &[ae::Aabb]) -> Option<ae::Aabb> {
    let mut iter = parts.iter();
    let first = iter.next()?;
    let mut min = first.min;
    let mut max = first.max;
    for part in iter {
        if part.min.x < min.x {
            min.x = part.min.x;
        }
        if part.min.y < min.y {
            min.y = part.min.y;
        }
        if part.max.x > max.x {
            max.x = part.max.x;
        }
        if part.max.y > max.y {
            max.y = part.max.y;
        }
    }
    let center = (min + max) * 0.5;
    let half = (max - min) * 0.5;
    Some(ae::Aabb::new(center, half))
}

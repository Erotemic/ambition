//! Debug-overlay drawing toolkit: the palette + low-level gizmo primitives
//! (AABB/arrow draws, world<->bevy coord helpers) the overlay layers share.
//!
//! Split out of the former 1001-line `debug_overlay.rs` (2026-06-15).

use super::*;

pub(crate) fn cyan() -> Color {
    Color::srgba(0.30, 0.92, 1.00, 0.92)
}
pub(crate) fn blue() -> Color {
    Color::srgba(0.30, 0.55, 1.00, 0.90)
}
pub(crate) fn green() -> Color {
    Color::srgba(0.25, 1.00, 0.45, 0.90)
}
pub(crate) fn yellow() -> Color {
    Color::srgba(1.00, 0.92, 0.22, 0.95)
}
pub(crate) fn orange() -> Color {
    Color::srgba(1.00, 0.55, 0.16, 0.90)
}
pub(crate) fn magenta() -> Color {
    Color::srgba(1.00, 0.32, 0.92, 0.88)
}
pub(crate) fn red() -> Color {
    Color::srgba(1.00, 0.18, 0.22, 0.82)
}
pub(crate) fn white_dim() -> Color {
    Color::srgba(0.90, 0.95, 1.00, 0.40)
}
pub(crate) fn gray() -> Color {
    Color::srgba(0.62, 0.66, 0.75, 0.46)
}

pub(crate) fn draw_aabb(gizmos: &mut Gizmos, world: &ae::World, aabb: ae::Aabb, color: Color) {
    let min = aabb.min;
    let max = aabb.max;
    let tl = w2(world, ae::Vec2::new(min.x, min.y));
    let tr = w2(world, ae::Vec2::new(max.x, min.y));
    let br = w2(world, ae::Vec2::new(max.x, max.y));
    let bl = w2(world, ae::Vec2::new(min.x, max.y));
    gizmos.line_2d(tl, tr, color);
    gizmos.line_2d(tr, br, color);
    gizmos.line_2d(br, bl, color);
    gizmos.line_2d(bl, tl, color);
}

/// Outline + optional translucent fill. Fills are controlled directly by the
/// current debug view mode instead of being coupled to sprite hiding; choose
/// Collision/Combat/Triggers when the filled volume view is useful.
pub(crate) fn draw_aabb_styled(
    gizmos: &mut Gizmos,
    world: &ae::World,
    aabb: ae::Aabb,
    color: Color,
    developer_tools: &DeveloperTools,
) {
    draw_aabb(gizmos, world, aabb, color);
    if !developer_tools.fill_debug_boxes {
        return;
    }
    let size = aabb.half_size() * 2.0;
    let center = w2(world, aabb.center());
    let fill = with_alpha(color, 0.22);
    // Bevy gizmos' `rect_2d` draws the outline by default. We want a
    // filled appearance, so draw a stack of horizontal lines spaced
    // 2px apart — works on every Bevy gizmo backend without needing a
    // separate mesh path. The cost is bounded (each AABB is small in
    // pixel terms and we only call this when the toggle is on).
    let step = 2.0;
    let half_h = (size.y * 0.5).max(0.5);
    let mut y = -half_h;
    while y < half_h {
        let a = BVec2::new(center.x - size.x * 0.5, center.y + y);
        let b = BVec2::new(center.x + size.x * 0.5, center.y + y);
        gizmos.line_2d(a, b, fill);
        y += step;
    }
}

pub(crate) fn with_alpha(color: Color, alpha: f32) -> Color {
    let srgba = color.to_srgba();
    Color::srgba(srgba.red, srgba.green, srgba.blue, alpha.clamp(0.0, 1.0))
}

pub(crate) fn draw_arrow(gizmos: &mut Gizmos, start: BVec2, end: BVec2, color: Color) {
    gizmos.line_2d(start, end, color);
    let delta = end - start;
    let len = delta.length();
    if len <= 1.0 {
        return;
    }
    let dir = delta / len;
    let side = BVec2::new(-dir.y, dir.x);
    let head = 9.0_f32.min(len * 0.28);
    gizmos.line_2d(end, end - dir * head + side * head * 0.55, color);
    gizmos.line_2d(end, end - dir * head - side * head * 0.55, color);
}

pub(crate) fn w2(world: &ae::World, p: ae::Vec2) -> BVec2 {
    world_to_bevy(world, p, 0.0).truncate()
}

pub(crate) fn engine_delta_to_bevy(delta: ae::Vec2) -> BVec2 {
    BVec2::new(delta.x, -delta.y)
}

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

/// Draw a [`CombatVolume`] outline — a box, rotated box, disc, or convex
/// polygon. Lets the overlay show the ACTUAL shaped hitbox (a blade-arc poly)
/// instead of its bounding box.
pub(crate) fn draw_combat_volume(
    gizmos: &mut Gizmos,
    world: &ae::World,
    vol: &ae::CombatVolume,
    color: Color,
) {
    let outline = |gizmos: &mut Gizmos, pts: &[ae::Vec2]| {
        let n = pts.len();
        for i in 0..n {
            gizmos.line_2d(w2(world, pts[i]), w2(world, pts[(i + 1) % n]), color);
        }
    };
    match vol {
        ae::CombatVolume::Aabb(a) => draw_aabb(gizmos, world, *a, color),
        ae::CombatVolume::Obb {
            center,
            half,
            rotation,
        } => {
            let (s, c) = rotation.sin_cos();
            let rot = |x: f32, y: f32| *center + ae::Vec2::new(x * c - y * s, x * s + y * c);
            outline(
                gizmos,
                &[
                    rot(-half.x, -half.y),
                    rot(half.x, -half.y),
                    rot(half.x, half.y),
                    rot(-half.x, half.y),
                ],
            );
        }
        ae::CombatVolume::Circle { center, radius } => {
            const N: usize = 24;
            let pts: Vec<ae::Vec2> = (0..N)
                .map(|i| {
                    let a = i as f32 / N as f32 * std::f32::consts::TAU;
                    *center + ae::Vec2::new(a.cos() * radius, a.sin() * radius)
                })
                .collect();
            outline(gizmos, &pts);
        }
        ae::CombatVolume::Convex { points, .. } => {
            if points.len() >= 2 {
                outline(gizmos, points);
            }
        }
    }
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

// ───────────────────────────── debug box labels ─────────────────────────────
//
// Gizmos draw lines, not text, so each debug box's *identity* (which color is
// the hurtbox vs the contact zone vs the collision envelope) is invisible. The
// label layer fixes that: draw calls push a `DebugLabel` per box, and
// `render_debug_overlay_labels` materializes them as world-space `Text2d`.

/// Font size (world units) for every debug-box label. **THIS IS THE SIZE KNOB**
/// — bump it up for bigger text, down for smaller, no other change needed.
/// Labels are world-space `Text2d`, so the size scales with camera zoom (this
/// default reads well at the usual boss-fight zoom).
pub const DEBUG_LABEL_FONT_PX: f32 = 7.0;

/// Bevy Z for label text — well above gameplay sprites (player=20, fx=30) so
/// labels never hide behind the art they annotate.
pub(crate) const DEBUG_LABEL_Z: f32 = 200.0;

/// Where a box's label sits relative to its rect. Each debug box *type* gets a
/// distinct spot so the labels for overlapping boxes (a boss's collision +
/// hurtbox + contact zones all share roughly one center) fan out to different
/// corners instead of stacking illegibly.
#[derive(Clone, Copy)]
pub(crate) enum LabelSpot {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Center,
}

/// One queued debug-box label: world position + text + color (matched to the
/// box color so the association is read at a glance).
pub(crate) struct DebugLabel {
    pub world_pos: ae::Vec2,
    pub text: String,
    pub color: Color,
}

/// Per-frame scratch buffer of debug-box labels. Filled by the overlay draw
/// calls, drained by [`render_debug_overlay_labels`].
#[derive(Resource, Default)]
pub(crate) struct DebugOverlayLabels(pub Vec<DebugLabel>);

/// Queue a label for `aabb`, anchored at the `spot` corner (inset a few px so
/// it sits just inside the line). World y is DOWN, so "top" = the smaller y.
pub(crate) fn label_box(
    labels: &mut DebugOverlayLabels,
    aabb: ae::Aabb,
    text: impl Into<String>,
    color: Color,
    spot: LabelSpot,
) {
    let c = aabb.center();
    let h = aabb.half_size();
    let pad = 3.0;
    let pos = match spot {
        LabelSpot::TopLeft => ae::Vec2::new(c.x - h.x + pad, c.y - h.y + pad),
        LabelSpot::TopRight => ae::Vec2::new(c.x + h.x - pad, c.y - h.y + pad),
        LabelSpot::BottomLeft => ae::Vec2::new(c.x - h.x + pad, c.y + h.y - pad),
        LabelSpot::BottomRight => ae::Vec2::new(c.x + h.x - pad, c.y + h.y - pad),
        LabelSpot::Center => c,
    };
    labels.0.push(DebugLabel {
        world_pos: pos,
        text: text.into(),
        color: with_alpha(color, 1.0),
    });
}

pub(crate) fn engine_delta_to_bevy(delta: ae::Vec2) -> BVec2 {
    BVec2::new(delta.x, -delta.y)
}

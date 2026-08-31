//! Debug-overlay drawing toolkit — the app-side remainder.
//!
//! What stays here is the LABEL machinery — world-space identities for the debug boxes — which
//! the shared shapes-only layer deliberately does not carry.

use super::*;

// ───────────────────────────── debug box labels ─────────────────────────────
//
// A debug box's *identity* (which colour is the hurtbox vs the contact zone vs
// the collision envelope) is invisible from its outline alone. The label layer
// fixes that: draw calls push a `DebugLabel` per box, and
// `render_debug_overlay_labels` draws them as TEXT GIZMOS on Bevy 0.19's stroke
// font — no entity, no font asset, no lifetime.

/// Font size (world units) for every debug-box label. THIS IS THE SIZE KNOB
/// — bump it up for bigger text, down for smaller, no other change needed.
/// ⭐ Text gizmos take a WORLD-space isometry, so this stayed world units across
/// the 0.19 move off `Text2d` and still scales with camera zoom (this default
/// reads well at the usual boss-fight zoom).
pub const DEBUG_LABEL_FONT_PX: f32 = 7.0;


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

//! What a generated sheet says about the BODY inside its frames: frame size,
//! the per-animation pixel rectangles, and the render size those imply.

use ambition_platformer2d_core as ae;

// This type is sprite metrics, so it lives in this crate. It cannot live in
// `ambition_characters`: that crate has no `sprite_sheet` dependency, and
// `sprite_sheet` already depends on `characters`.

/// Snapshot of the sprite generator's `body_metrics` for a boss, taken once
/// at sprite-registry lookup so per-tick damage and hurtbox math does not
/// query the SheetRegistry resource again.
///
/// `body_pixel_bbox` is one overall body bbox (single-piece bosses).
/// `body_pixel_parts` is the multi-rect form for bosses with separate pieces
/// (head, body, arms). Either or both may be set; the consumer uses parts when
/// present, else the bbox.
///
/// `sprite_render_size` is the world-space size of the rendered sprite quad
/// (`BossSheetSpec::render_size(boss.size)`). Hurtbox and hitbox math uses it,
/// not `boss.size`, as the world scale, so the boxes line up with the visible
/// sprite. The boss spawns at LDtk size but can render larger.
#[derive(Clone, Debug, Default)]
pub struct ActorSpriteMetrics {
    pub frame_width: u32,
    pub frame_height: u32,
    pub body_pixel_bbox: Option<crate::PixelRect>,
    pub body_pixel_parts: Vec<crate::NamedPixelRect>,
    /// World-space size of the rendered sprite quad
    /// (`BossSheetSpec::render_size(boss.size)` at derivation time). Test
    /// fixtures without a spec use `(boss.size, boss.size)`; consumers treat
    /// zero as "no render size yet, use ctx.size".
    pub sprite_render_size: ae::Vec2,
    /// World-space offset from `boss.pos` to the body bbox center. The body is
    /// not always at the frame center, so without this offset the pogo zone,
    /// debug box, and body-contact zone sit below the visible body.
    pub combat_offset: ae::Vec2,
    /// Per-animation `{hurtbox, hitbox}` data keyed by animation name (the
    /// sheet rows: `"rest"`, `"floor_slam"`, `"side_sweep"`, …). The renderer
    /// fills `hurtbox` from each animation's alpha bbox; the adapter declares
    /// `hitbox` rects for attacks. `damageable_volumes` and
    /// `volumes_for_profile` look up the current animation.
    ///
    /// A `BTreeMap`, like [`crate::BodyMetrics::animations`], because it is
    /// cloned from that map and must keep the same order.
    pub animations: std::collections::BTreeMap<String, crate::AnimationMetrics>,
}

impl ActorSpriteMetrics {
    /// True iff this snapshot carries at least one rectangle the
    /// derivation can use.
    pub fn has_body(&self) -> bool {
        !self.body_pixel_parts.is_empty() || self.body_pixel_bbox.is_some()
    }

    /// Per-animation hurtbox lookup. `damageable_volumes` uses it to size the
    /// hurtbox to the current animation (attack frames with extended arms get
    /// a wider hurtbox). `None` if the animation has no override; the caller
    /// then uses `body_pixel_parts` / `body_pixel_bbox`.
    pub fn hurtbox_for_animation(&self, animation: &str) -> Option<&crate::AnimationBox> {
        self.animations.get(animation)?.hurtbox.as_ref()
    }

    /// Per-animation hitbox lookup. `volumes_for_profile` uses it to read the
    /// authored damage geometry for an attack animation. `None` if the
    /// animation has no authored hitbox; the caller then uses its own volume
    /// math.
    pub fn hitbox_for_animation(&self, animation: &str) -> Option<&crate::AnimationBox> {
        self.animations.get(animation)?.hitbox.as_ref()
    }
}

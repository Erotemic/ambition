//! Sprite render geometry: per-target render size, feet anchoring, and the
//! Bevy `Sprite` construction helpers the renderers call.

use super::*;

impl CharacterSheetSpec {
    /// Where this sheet's character actually is inside its frame, in frame
    /// pixels — the rectangle the generator measured (or the target authored)
    /// for `anim`, `None` for a sheet that publishes no body at all.
    ///
    /// Asked of the one reader ([`crate::BodyMetrics::body_pixel_extent`]), so
    /// the quad, the collision box and the sheet-authored actor route cannot
    /// disagree about what the sheet says.
    pub fn body_pixel_extent(&self, anim: CharacterAnim) -> Option<Vec2> {
        let (w, h) = self.record.body_metrics.as_ref()?.body_pixel_extent(anim)?;
        let extent = Vec2::new(w, h);
        (extent.x > 0.0 && extent.y > 0.0).then_some(extent)
    }

    /// The sheet's frame, in pixels, floored at 1 so it is never a divisor of
    /// zero.
    pub fn frame_pixels(&self) -> Vec2 {
        Vec2::new(
            self.frame_width.max(1) as f32,
            self.frame_height.max(1) as f32,
        )
    }
}

/// Per-target sprite render size: the sheet's frame drawn at the scale that
/// puts the character's own body rectangle on the collision box.
pub fn sprite_render_size(spec: &CharacterSheetSpec, collision: Vec2) -> Vec2 {
    sprite_render_size_scaled(spec, collision, 1.0)
}

/// The quad is the FRAME at the scale that fits the sheet's BODY into the
/// collision box — one uniform scale, so the art is never stretched and the
/// drawn character is the size of the thing it collides with.
///
/// the scale is computable, which is why nothing authors it any more. A sheet publishes
/// `body_pixel_bbox` (184 of 190 do — the generator measures the alpha bbox on every
/// regeneration), so `world_per_pixel = fit(collision, body)` is arithmetic.
///
/// * the height came off the collision box's LARGER axis, so a long flat animal was drawn as tall as it is wide;
/// * the width came off the PADDED FRAME's aspect, which its own comment named as the intent — so the drawn body's size depended on how much empty space the generator's crop happened to leave around it;
/// * `collision_scale` corrected frame padding per sheet. The resulting figure scale varied by
///   10.9x across baked sheets; deriving scale from the body bbox makes it 1.0 by construction.
///
/// the whole frame is still drawn, not a crop of it. `Sprite::custom_size`
/// scales the entire atlas frame into the quad per axis, so sizing the quad to
/// the BODY while still sampling the frame divides the padding into the
/// character — measured, first try, at a 2.20x vertical squash on the snake and
/// 0.65x horizontal on Mary-O. Keeping the quad frame-shaped and scaling it
/// uniformly is what `posed_body_geometry` already does for sheet-authored
/// bodies, and it needs no atlas surgery: the padding is transparent.
///
/// `visual_scale` is presentation-only and is a deliberate deviation from "the
/// picture is the body" — it stays 1.0 unless somebody looking at the running
/// game wants a character drawn off its own box on purpose.
///
/// Sheets with no published body (2 of 183 baked: `creator_lab_props`,
/// `weird_hermit`) keep the old arithmetic, because there is nothing else to
/// ask.
pub fn sprite_render_size_scaled(
    spec: &CharacterSheetSpec,
    collision: Vec2,
    visual_scale: f32,
) -> Vec2 {
    let frame = spec.frame_pixels();
    let scale = visual_scale.max(0.05);
    if let Some(body) = spec.body_pixel_extent(CharacterAnim::Idle) {
        // Fit rather than match-one-axis: the box and the art can disagree about
        // aspect (an LDtk rectangle is a placement, not a claim about a
        // silhouette), and the honest reading of a disagreement is that the
        // drawn body stays INSIDE the box, touching on the axis that binds.
        // Where the box was derived from this same rectangle — the common case,
        // because `sprite_body_collision_for_character_id` derives it — both
        // axes bind and the fit is exact.
        let fit = (collision.x.max(1.0) / body.x).min(collision.y.max(1.0) / body.y);
        if fit.is_finite() && fit > 0.0 {
            return frame * fit * scale;
        }
    }
    // No published body: height off the collision box's larger axis, width off
    // the frame aspect, corrected by the sheet's hand-tuned `collision_scale`.
    let height = collision.x.max(collision.y).max(8.0) * spec.collision_scale * scale;
    Vec2::new(height * frame.x / frame.y, height)
}

/// Sprite anchor that places the rendered character's feet on the bottom
/// of the collision box (rather than at its centre).
pub fn feet_anchor_for(spec: &CharacterSheetSpec, collision: Vec2) -> Anchor {
    feet_anchor_for_render_size(spec, collision, sprite_render_size(spec, collision))
}

/// Sprite anchor for an explicit render size. This keeps the feet planted when
/// presentation-only scaling makes the sprite larger than its collider.
pub fn feet_anchor_for_render_size(
    spec: &CharacterSheetSpec,
    collision: Vec2,
    render_size: Vec2,
) -> Anchor {
    let render_height = render_size.y.max(1.0);
    let half_collision_y = collision.y * 0.5;
    let ay = spec.feet_anchor_y + half_collision_y / render_height;
    Anchor(Vec2::new(0.0, ay))
}

/// Build the textured sprite for a character given its collision-box size.
pub fn build_character_sprite(asset: &CharacterSpriteAsset, collision: Vec2) -> Sprite {
    build_character_sprite_with_render_size(asset, sprite_render_size(&asset.spec, collision))
}

/// Build the textured sprite with an explicit presentation render size.
pub fn build_character_sprite_with_render_size(
    asset: &CharacterSpriteAsset,
    render_size: Vec2,
) -> Sprite {
    let mut sprite = Sprite::from_atlas_image(
        asset.texture.clone(),
        bevy::image::TextureAtlas {
            layout: asset.layout.clone(),
            index: asset.spec.flat_index(CharacterAnim::Idle, 0),
        },
    );
    sprite.custom_size = Some(render_size);
    sprite
}

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

/// Scale the entire atlas frame uniformly so its published body bbox fits inside
/// the collision box. Transparent frame padding remains part of the quad; only
/// the body's measured extent determines world scale, so the art is not
/// stretched. `visual_scale` is an explicit presentation-only multiplier.
///
/// TODO(compat-remove): once every sheet publishes a body bbox, delete the
/// `collision_scale` fallback for sheets without one.
pub fn sprite_render_size_scaled(
    spec: &CharacterSheetSpec,
    collision: Vec2,
    visual_scale: f32,
) -> Vec2 {
    let frame = spec.frame_pixels();
    let scale = visual_scale.max(0.05);
    if let Some(body) = spec.body_pixel_extent(CharacterAnim::Idle) {
        // Preserve aspect ratio and keep the published body inside the collision box.
        let fit = (collision.x.max(1.0) / body.x).min(collision.y.max(1.0) / body.y);
        if fit.is_finite() && fit > 0.0 {
            return frame * fit * scale;
        }
    }
    // Compatibility fallback for sheets without a published body bbox.
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
    // ⛔⛔ THE X WAS `0.0`, AND `0.0` IS A CLAIM ABOUT THE FRAME rather than about
    // the character: it centres the art on the packed cell, and the art sits
    // wherever the crop left it inside that cell. Every sheet already measures
    // the difference — `body_metrics.feet_anchor_norm.x` IS the body's centre as
    // a fraction of the frame — and it was read for `y` and dropped for `x`.
    //
    // ⭐ SO A BODY IS DRAWN ON ITS OWN BOX NOW. `projectile_polygon` is packed
    // 17% of a 377px frame left of centre and `officer` 25% of a 326px one, which
    // is why they read as a collision box standing NEXT TO a fighter; a sheet
    // that authors no body metrics still answers `0.0` and is byte-identical.
    Anchor(Vec2::new(spec.feet_anchor_x, ay))
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

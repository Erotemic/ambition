//! Sprite render geometry: per-target render size, feet anchoring, and the
//! Bevy `Sprite` construction helpers the renderers call.
//!
//! Split out of the former 780-line `sheets/mod.rs` (2026-06-15).

use super::*;

/// Per-target sprite render size. The generator's character occupies only
/// part of the 128×128 frame, so the rendered quad must be larger than
/// the collision box for the visible body to roughly match the hitbox.
///
/// TODO(gen2d-collision-aware): teach the generator to write
/// `body_pixel_extent` + `feet_y_pixel` into the spritesheet YAML and
/// load them at runtime, replacing these per-spec constants with values
/// derived from each sheet's actual rendered body. The per-spec tuning
/// already isolates the override per target so the migration is local.
pub fn sprite_render_size(spec: &CharacterSheetSpec, collision: Vec2) -> Vec2 {
    sprite_render_size_scaled(spec, collision, 1.0)
}

/// Render-size helper with an additional presentation-only scale.
///
/// The collision box remains gameplay authority; this scale is only for
/// placeholder sprites while final art is still in flux.
pub fn sprite_render_size_scaled(
    spec: &CharacterSheetSpec,
    collision: Vec2,
    visual_scale: f32,
) -> Vec2 {
    // Height is collision-driven; width preserves the cropped frame's
    // aspect ratio so the character isn't horizontally squashed when the
    // generator crop produces non-square frames (e.g. robot 120×128).
    let height =
        collision.x.max(collision.y).max(8.0) * spec.collision_scale * visual_scale.max(0.05);
    let width = height * (spec.frame_width as f32 / spec.frame_height as f32);
    Vec2::new(width, height)
}

/// Presentation-only scale for the temporary player sprite.
///
/// The robot sheet's `collision_scale` compensates for transparent/cropped
/// frame space; this extra factor gives the placeholder a slightly more
/// heroic read against the tuned 30×48 movement body without changing
/// gameplay collision.
pub const PLAYER_PLACEHOLDER_VISUAL_SCALE: f32 = 1.16;

pub fn player_placeholder_render_size(spec: &CharacterSheetSpec, collision: Vec2) -> Vec2 {
    sprite_render_size_scaled(spec, collision, PLAYER_PLACEHOLDER_VISUAL_SCALE)
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

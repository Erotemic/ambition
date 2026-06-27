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

#[cfg(test)]
mod trim_tests {
    use super::*;

    /// World position of a logical-frame point when a sprite draws the sub-rect
    /// `[ox,oy,tw,th]` of the logical frame at size `S` with anchor `A`, placed
    /// at the origin. Mirrors Bevy's `sprite_center = -anchor * size`.
    fn world_of_logical_point(
        px: f32,
        py: f32, // logical pixel, py from top
        logical: Vec2,
        sub: (f32, f32, f32, f32), // ox, oy, tw, th
        size: Vec2,
        anchor: Vec2,
    ) -> Vec2 {
        let (ox, oy, tw, th) = sub;
        let _ = logical;
        let nx = (px - ox) / tw - 0.5;
        let ny = 0.5 - (py - oy) / th;
        let center = -anchor * size;
        center + Vec2::new(nx * size.x, ny * size.y)
    }

    #[test]
    fn trimmed_render_is_identity_for_untrimmed_frame() {
        let logical = UVec2::new(384, 529);
        let trim = FrameTrim { offset: IVec2::ZERO, trimmed: logical, logical };
        let base_size = Vec2::new(120.0, 165.0);
        let base_anchor = Vec2::new(0.0, -0.3);
        let (size, anchor) = trimmed_render(&trim, base_size, base_anchor);
        assert!((size - base_size).length() < 1e-3, "{size:?}");
        assert!((anchor - base_anchor).length() < 1e-4, "{anchor:?}");
    }

    #[test]
    fn trimmed_render_keeps_logical_points_fixed() {
        // A frame trimmed to an off-center sub-rect must still map every logical
        // point to the same world position the full frame would.
        let logical = UVec2::new(384, 529);
        let (ox, oy, tw, th) = (100i32, 80i32, 180u32, 360u32);
        let trim = FrameTrim {
            offset: IVec2::new(ox, oy),
            trimmed: UVec2::new(tw, th),
            logical,
        };
        let base_size = Vec2::new(120.0, 165.0);
        let base_anchor = Vec2::new(0.0, -0.3);
        let (size, anchor) = trimmed_render(&trim, base_size, base_anchor);

        let lw = logical.x as f32;
        let lh = logical.y as f32;
        // Sample several logical points that lie inside the trimmed sub-rect.
        for &(px, py) in &[
            (ox as f32 + 1.0, oy as f32 + 1.0),
            (ox as f32 + tw as f32 / 2.0, oy as f32 + th as f32 / 2.0),
            (ox as f32 + tw as f32 - 1.0, oy as f32 + th as f32 - 1.0),
        ] {
            let full = world_of_logical_point(
                px,
                py,
                Vec2::new(lw, lh),
                (0.0, 0.0, lw, lh),
                base_size,
                base_anchor,
            );
            let trimmed = world_of_logical_point(
                px,
                py,
                Vec2::new(lw, lh),
                (ox as f32, oy as f32, tw as f32, th as f32),
                size,
                anchor,
            );
            assert!(
                (full - trimmed).length() < 1e-2,
                "logical point ({px},{py}) moved: full={full:?} trimmed={trimmed:?}"
            );
        }
    }
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

/// Per-frame trim geometry (see [`CharacterSheetSpec::frame_trim`]).
#[derive(Clone, Copy, Debug)]
pub struct FrameTrim {
    /// Offset of the trimmed rect within the logical frame, in logical pixels.
    pub offset: IVec2,
    /// Size of the trimmed rect (== the atlas rect size), in logical pixels.
    pub trimmed: UVec2,
    /// Logical (untrimmed) frame size.
    pub logical: UVec2,
}

impl FrameTrim {
    /// True when this frame carries no trim (drawn the legacy uniform way).
    pub fn is_identity(&self) -> bool {
        self.offset == IVec2::ZERO && self.trimmed == self.logical
    }
}

/// Given a frame's trim geometry plus the base (untrimmed) render size and feet
/// anchor, return the `(custom_size, anchor)` that draws the trimmed sub-rect so
/// the logical frame's anchor point lands at the SAME world position the
/// untrimmed frame would have used.
///
/// Derivation: the full logical sprite has size `base_render_size` and anchor
/// `base_anchor`; render only the trimmed sub-region at the proportional size
/// and solve for the anchor that keeps the logical-frame mapping fixed. The
/// formula reduces to `(base_render_size, base_anchor)` for an untrimmed frame
/// (`offset == 0`, `trimmed == logical`), so untrimmed sheets are unchanged.
/// Pinned by `trimmed_anchor_*` unit tests.
pub fn trimmed_render(
    trim: &FrameTrim,
    base_render_size: Vec2,
    base_anchor: Vec2,
) -> (Vec2, Vec2) {
    let fw = trim.logical.x.max(1) as f32;
    let fh = trim.logical.y.max(1) as f32;
    let tw = trim.trimmed.x.max(1) as f32;
    let th = trim.trimmed.y.max(1) as f32;
    let ox = trim.offset.x as f32;
    let oy = trim.offset.y as f32;
    let (ax, ay) = (base_anchor.x, base_anchor.y);
    let custom = Vec2::new(tw / fw * base_render_size.x, th / fh * base_render_size.y);
    let anchor = Vec2::new(
        ((ax + 0.5) * fw - ox - tw * 0.5) / tw,
        ((ay - 0.5) * fh + oy + th * 0.5) / th,
    );
    (custom, anchor)
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

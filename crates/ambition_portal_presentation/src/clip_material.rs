//! The portal-clip material: a `Material2d` that draws one texture-accurate
//! **piece** of a sprite mid-portal-transit, discarding every fragment behind
//! a world-space clip half-plane.
//!
//! This is the render-side realization of the Core invariant in
//! [`ambition_portal::pieces`]: a body straddling a portal pair is ONE logical
//! object with TWO spatial pieces. [`crate::sync_portal_body_pieces`] hides
//! the real sprite while a `through` piece exists and draws both charts as
//! sibling mesh quads running this material — the `here` slice clipped to the
//! front of the entry plane, the `through` slice clipped to the front of the
//! exit plane (plus the exit aperture span). Because the portal map is an
//! isometry, the two slices tile continuously across the seam: nothing pops
//! when the authoritative position snaps at the centroid crossing, and the
//! sunk slice never draws over the far side of a thin wall.
//!
//! Clipping runs in the fragment shader against final render-world positions,
//! so it is exact for any anchor, trim rect, flip, roll, or scale — the
//! `Sprite.rect` alternative would have to re-derive all of those per frame.
//! The quad + atlas-frame UV mapping follows the hit-flash overlay pattern
//! (`ambition_render::rendering::hit_flash`), the established way to draw "the
//! sprite's current frame" as a mesh.

use bevy::asset::embedded_asset;
use bevy::image::TextureAtlasLayout;
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;
use bevy::sprite_render::{AlphaMode2d, Material2d, Material2dPlugin};

use crate::PortalWorldFrame;

/// A clip half-plane that rejects nothing (zero normal = disabled in-shader).
pub const CLIP_PLANE_OFF: Vec4 = Vec4::ZERO;

/// Material2d for one portal-clipped sprite piece.
///
/// Bindings follow the WebGL2-friendly convention of the hit-flash /
/// deep-dream overlays: plain `vec4` uniforms, no struct UBOs, no arrays.
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct PortalClipMaterial {
    /// Current atlas frame as a UV rect on the sprite sheet:
    /// `(min.x, min.y, max.x, max.y)` normalized.
    #[uniform(0)]
    pub uv_rect: Vec4,
    /// `(flip_x, _, _, _)`. `flip_x > 0.5` mirrors the frame horizontally.
    #[uniform(1)]
    pub control: Vec4,
    /// Sprite tint (linear RGBA), multiplied into the sample.
    #[uniform(2)]
    pub tint: Vec4,
    /// Clip half-planes `(point.xy, normal.xy)` in render-world space;
    /// fragments with `dot(p - point, normal) < 0` are discarded. Zero
    /// normal disables a plane ([`CLIP_PLANE_OFF`]).
    #[uniform(3)]
    pub clip0: Vec4,
    #[uniform(4)]
    pub clip1: Vec4,
    #[uniform(5)]
    pub clip2: Vec4,
    #[texture(6)]
    #[sampler(7)]
    pub color_texture: Handle<Image>,
}

impl Material2d for PortalClipMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://ambition_portal_presentation/shaders/portal_clip.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

/// Register the embedded shader + material pipeline. Called by
/// [`crate::PortalPresentationPlugin`] when its `body_pieces` visual is on and
/// the host runs a real asset/render stack; headless hosts skip it and
/// [`crate::sync_portal_body_pieces`] falls back to the unclipped sprite copy
/// (its asset params are `Option`al).
pub(crate) fn add_portal_clip_material_plugin(app: &mut App) {
    // `embedded_asset!` needs the AssetPlugin's registry; a headless test app
    // without assets simply doesn't get the material path.
    if app
        .world()
        .get_resource::<bevy::asset::io::embedded::EmbeddedAssetRegistry>()
        .is_none()
    {
        return;
    }
    embedded_asset!(app, "shaders/portal_clip.wgsl");
    app.add_plugins(Material2dPlugin::<PortalClipMaterial>::default());
}

/// An engine-space half-plane (point + outward normal) as the shader's
/// render-space `(point.xy, normal.xy)` uniform. Positions go through the one
/// canonical engine→Bevy adapter ([`PortalWorldFrame::to_render`]); directions
/// only flip y (engine is y-down, render is y-up).
pub fn clip_plane_render(frame: &PortalWorldFrame, point: Vec2, normal: Vec2) -> Vec4 {
    let p = frame.to_render(point, 0.0);
    Vec4::new(p.x, p.y, normal.x, -normal.y)
}

/// The render basis of a sprite's CURRENT frame: the normalized UV rect of the
/// frame on its texture, and the world-space quad size the sprite draws at.
/// `None` while the texture / atlas layout hasn't loaded — callers fall back
/// to the unclipped sprite-copy path for that frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpriteFrameBasis {
    /// `(min.x, min.y, max.x, max.y)` normalized on the sprite's texture.
    pub uv_rect: Vec4,
    /// Drawn quad size: `custom_size` when set (trimmed sheets update it per
    /// frame), else the frame's native pixel size.
    pub size: Vec2,
}

/// Resolve the [`SpriteFrameBasis`] for `sprite` (atlas frame or whole image).
/// Mirrors the hit-flash overlay's UV resolution so the mesh piece samples
/// exactly the pixels the sprite renderer would.
pub fn sprite_frame_basis(
    sprite: &Sprite,
    layouts: &Assets<TextureAtlasLayout>,
    images: &Assets<Image>,
) -> Option<SpriteFrameBasis> {
    let image = images.get(&sprite.image)?;
    let texture_size = image.texture_descriptor.size;
    let tex = Vec2::new(
        texture_size.width.max(1) as f32,
        texture_size.height.max(1) as f32,
    );
    let (uv_rect, frame_px) = if let Some(atlas) = sprite.texture_atlas.as_ref() {
        let layout = layouts.get(&atlas.layout)?;
        let rect = layout.textures.get(atlas.index)?;
        let min = Vec2::new(rect.min.x as f32, rect.min.y as f32);
        let max = Vec2::new(rect.max.x as f32, rect.max.y as f32);
        (
            Vec4::new(min.x / tex.x, min.y / tex.y, max.x / tex.x, max.y / tex.y),
            max - min,
        )
    } else {
        (Vec4::new(0.0, 0.0, 1.0, 1.0), tex)
    };
    Some(SpriteFrameBasis {
        uv_rect,
        size: sprite.custom_size.unwrap_or(frame_px),
    })
}

/// Pose a piece quad so it draws exactly where the source sprite would: the
/// unit rect is scaled to the drawn size, and the sprite's anchor becomes a
/// rotated world offset (a mesh quad is center-origin; a sprite pivots on its
/// anchor). `base` carries translation (incl. z), rotation, and any extra
/// sprite scale.
pub fn clip_piece_transform(base: &Transform, anchor: Vec2, size: Vec2) -> Transform {
    let offset = base.rotation * (-anchor * size * base.scale.truncate()).extend(0.0);
    Transform {
        translation: base.translation + offset,
        rotation: base.rotation,
        scale: (size * base.scale.truncate()).extend(1.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame_1000x600() -> PortalWorldFrame {
        PortalWorldFrame {
            size: Vec2::new(1000.0, 600.0),
        }
    }

    /// Engine y-down → render y-up: the plane point goes through the canonical
    /// adapter and the normal's y flips.
    #[test]
    fn clip_plane_render_flips_y() {
        let frame = frame_1000x600();
        // A floor plane at engine (500, 300) with up (engine -y) normal.
        let plane = clip_plane_render(&frame, Vec2::new(500.0, 300.0), Vec2::new(0.0, -1.0));
        assert_eq!(Vec2::new(plane.x, plane.y), Vec2::new(0.0, 0.0));
        // Engine "up" (-y) is render "+y".
        assert_eq!(Vec2::new(plane.z, plane.w), Vec2::new(0.0, 1.0));
        // A point above the floor (engine y < 300 → render y > 0) is in front.
        let p = frame.to_render(Vec2::new(500.0, 280.0), 0.0);
        let d =
            (Vec2::new(p.x, p.y) - Vec2::new(plane.x, plane.y)).dot(Vec2::new(plane.z, plane.w));
        assert!(d > 0.0, "above-floor point must be front-of-plane, got {d}");
    }

    /// The anchor offset moves the quad center the same way a sprite anchor
    /// moves pixels: a bottom-center anchor (0, -0.5) shifts the quad UP by
    /// half its height (the sprite hangs its pixels above the pivot).
    #[test]
    fn piece_transform_applies_anchor_and_size() {
        let base = Transform::from_translation(Vec3::new(10.0, 20.0, 5.0));
        let t = clip_piece_transform(&base, Vec2::new(0.0, -0.5), Vec2::new(32.0, 48.0));
        assert_eq!(t.translation, Vec3::new(10.0, 20.0 + 24.0, 5.0));
        assert_eq!(t.scale, Vec3::new(32.0, 48.0, 1.0));
    }

    /// Rotation rotates the anchor offset with the quad (a rolled sprite's
    /// anchor stays glued to the same sprite corner).
    #[test]
    fn piece_transform_rotates_anchor_offset() {
        let base = Transform::from_translation(Vec3::ZERO)
            .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2));
        let t = clip_piece_transform(&base, Vec2::new(0.0, -0.5), Vec2::new(10.0, 10.0));
        // Offset (0, +5) rotated 90° CCW → (-5, 0).
        assert!((t.translation.x - -5.0).abs() < 1e-4, "{:?}", t.translation);
        assert!(t.translation.y.abs() < 1e-4, "{:?}", t.translation);
    }

    /// Trimmed frames prefer `custom_size`; untrimmed atlas frames fall back
    /// to native pixel size; a plain image spans the whole texture.
    #[test]
    fn frame_basis_resolves_atlas_and_plain() {
        let mut images = Assets::<Image>::default();
        let mut layouts = Assets::<TextureAtlasLayout>::default();
        let mut image = Image::default(); // 1x1 white
        image.texture_descriptor.size.width = 64;
        image.texture_descriptor.size.height = 32;
        let image_handle = images.add(image);
        let mut layout = TextureAtlasLayout::new_empty(bevy::math::UVec2::new(64, 32));
        layout.add_texture(bevy::math::URect {
            min: bevy::math::UVec2::new(16, 0),
            max: bevy::math::UVec2::new(48, 32),
        });
        let layout_handle = layouts.add(layout);

        let mut sprite = Sprite::from_atlas_image(
            image_handle.clone(),
            bevy::image::TextureAtlas {
                layout: layout_handle,
                index: 0,
            },
        );
        let basis = sprite_frame_basis(&sprite, &layouts, &images).expect("atlas basis");
        assert_eq!(basis.uv_rect, Vec4::new(0.25, 0.0, 0.75, 1.0));
        assert_eq!(basis.size, Vec2::new(32.0, 32.0));

        sprite.custom_size = Some(Vec2::new(100.0, 50.0));
        let basis = sprite_frame_basis(&sprite, &layouts, &images).expect("atlas basis");
        assert_eq!(basis.size, Vec2::new(100.0, 50.0));

        let plain = Sprite::from_image(image_handle);
        let basis = sprite_frame_basis(&plain, &layouts, &images).expect("plain basis");
        assert_eq!(basis.uv_rect, Vec4::new(0.0, 0.0, 1.0, 1.0));
        assert_eq!(basis.size, Vec2::new(64.0, 32.0));
    }

    /// An unloaded texture yields no basis (callers fall back to the sprite
    /// copy instead of drawing a broken quad).
    #[test]
    fn frame_basis_none_when_texture_missing() {
        let images = Assets::<Image>::default();
        let layouts = Assets::<TextureAtlasLayout>::default();
        let sprite = Sprite::from_image(Handle::default());
        assert!(sprite_frame_basis(&sprite, &layouts, &images).is_none());
    }

    /// The embedded WGSL parses and validates under the same naga wgpu runs at
    /// runtime — a shader typo fails here instead of on first launch. Bevy's
    /// preprocessor directives are stubbed the way Bevy resolves them (the
    /// mesh2d vertex-output import, the bind-group substitution).
    #[test]
    fn portal_clip_wgsl_parses_and_validates() {
        let vertex_output = r#"
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}
"#;
        let source = include_str!("shaders/portal_clip.wgsl")
            .replace(
                "#import bevy_sprite::mesh2d_vertex_output::VertexOutput",
                vertex_output,
            )
            .replace("#{MATERIAL_BIND_GROUP}", "2");
        let module = naga::front::wgsl::parse_str(&source).expect("portal_clip.wgsl parses");
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .expect("portal_clip.wgsl validates");
    }
}

//! The portal-clip material: a `Material2d` that draws one texture-accurate
//! piece of a sprite mid-portal-transit, discarding every fragment behind
//! a world-space clip half-plane.
//!
//! This is the render-side realization of the Core invariant in
//! [`ambition_portal2d::pieces`]: a body straddling a portal pair is ONE logical
//! object with TWO spatial pieces. While a `through` piece exists, the real
//! sprite is hidden and [`crate::sync_portal_body_pieces`] draws both charts as
//! sibling mesh quads with this material: the `here` slice clipped to the front
//! of the entry plane, the `through` slice clipped to the front of the exit
//! plane (plus the exit aperture span). The portal map is an isometry, so the
//! slices tile continuously across the seam: nothing pops when the position
//! snaps at the centroid crossing, and the sunk slice never draws over the far
//! side of a thin wall.
//!
//! The piece builder only states a hide reason (`PortalTransitHidden`);
//! [`crate::source_visibility::resolve_portal_source_visibility`] writes
//! `Visibility`.
//!
//! Clipping runs in the fragment shader against final render-world positions,
//! so it is exact for any anchor, trim rect, flip, roll, or scale — the
//! `Sprite.rect` alternative would have to re-derive all of those per frame.
//! The quad + atlas-frame UV mapping follows the hit-flash overlay pattern
//! (`ambition_render::rendering::hit_flash`), the established way to draw "the
//! sprite's current frame" as a mesh.

use bevy::asset::embedded_asset;
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
    /// `(flip_x, flip_y, silhouette, _)`. `flip_x > 0.5` mirrors the frame
    /// horizontally; `silhouette > 0.5` paints `tint.rgb` masked by the sample's
    /// alpha and `tint.a` -- the look a declared non-sprite drawable asks for.
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
        "embedded://ambition_portal2d_presentation/shaders/portal_clip.wgsl".into()
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
    // Ensure, do not add: `add_plugins` panics on a duplicate, and both the
    // transit pieces and the far-side compositor call this (both on by
    // default), without knowing about each other.
    if app.is_plugin_added::<Material2dPlugin<PortalClipMaterial>>() {
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

// `SpriteFrameBasis` / `sprite_frame_basis` live in `ambition_sprite_fx` (they
// are not portal concepts) and are re-exported so callers are unchanged.
pub use ambition_sprite_fx::{sprite_frame_basis, SpriteFrameBasis};

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
mod tests;

#[cfg(test)]
mod idempotency_tests {
    use super::*;
    use bevy::asset::AssetPlugin;

    /// Two callers, one material, and `add_plugins` panics on a duplicate. The test
    /// installs `AssetPlugin`, because without the embedded registry the function
    /// returns early and never reaches the duplicate path.
    #[test]
    fn adding_the_clip_material_twice_is_safe() {
        let mut app = App::new();
        app.add_plugins(AssetPlugin::default());
        assert!(
            app.world()
                .get_resource::<bevy::asset::io::embedded::EmbeddedAssetRegistry>()
                .is_some(),
            "without the embedded registry this test cannot reach the duplicate path"
        );
        add_portal_clip_material_plugin(&mut app);
        // The second call is the one that used to panic.
        add_portal_clip_material_plugin(&mut app);
        assert!(app.is_plugin_added::<Material2dPlugin<PortalClipMaterial>>());
    }
}

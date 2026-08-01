//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

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
    let d = (Vec2::new(p.x, p.y) - Vec2::new(plane.x, plane.y)).dot(Vec2::new(plane.z, plane.w));
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
    let source = include_str!("../shaders/portal_clip.wgsl")
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

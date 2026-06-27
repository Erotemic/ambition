// ---------------------------------------------------------------------------
// Bubble shield sprite (procedural)
// ---------------------------------------------------------------------------
//
// The shield is a thin luminous ring that appears around the player while
// the shield button is held. The first `PARRY_WINDOW_TIME` seconds glow
// bright gold (full invulnerability parry window); after that it fades to a
// soft cyan (visible but no damage reduction in the v1 implementation).
//
// The texture is a white anti-aliased ring generated once at startup.
// `sync_bubble_shield_visual` tints it with `Sprite.color` each frame so
// no new image upload is needed for the parry-vs-held color switch.

use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

/// Procedural ring texture. Built once at startup.
#[derive(Resource, Clone, Default)]
pub struct BubbleShieldSprite {
    pub handle: Handle<Image>,
}

/// Marker on the shield sibling sprite.
#[derive(Component)]
pub struct BubbleShieldVisual;

const SHIELD_TEXTURE_SIZE: u32 = 64;

/// Generate a 64x64 RGBA ring with anti-aliased inner and outer edges.
/// White pixels so `Sprite.color` can drive the parry vs. held tint.
pub fn build_bubble_shield_image() -> Image {
    let size = SHIELD_TEXTURE_SIZE;
    let cx = (size as f32 - 1.0) * 0.5;
    let cy = cx;
    let inner_r = size as f32 * 0.32; // inner edge of ring
    let outer_r = size as f32 * 0.46; // outer edge of ring
    let aa = 1.8_f32; // anti-alias band width in pixels

    let mut data = vec![0u8; (size * size * 4) as usize];
    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let dist = (dx * dx + dy * dy).sqrt();
            // Outer edge fade: 1 inside, 0 outside.
            let outer_alpha = ((outer_r - dist) / aa).clamp(0.0, 1.0);
            // Inner edge fade: 0 inside hole, 1 outside hole.
            let inner_alpha = ((dist - inner_r) / aa).clamp(0.0, 1.0);
            let alpha = outer_alpha * inner_alpha;
            let i = ((y * size + x) * 4) as usize;
            // Full white so Sprite.color provides the actual hue.
            data[i] = 255;
            data[i + 1] = 255;
            data[i + 2] = 255;
            data[i + 3] = (alpha * 255.0) as u8;
        }
    }
    Image::new(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

/// Startup system: build the procedural bubble shield image and stash its handle.
pub fn build_bubble_shield_sprite(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let handle = images.add(build_bubble_shield_image());
    commands.insert_resource(BubbleShieldSprite { handle });
}

/// Spawn the shield sibling sprite. Runs each frame until one exists.
pub fn spawn_bubble_shield_visual(
    mut commands: Commands,
    sprite: Option<Res<BubbleShieldSprite>>,
    existing: Query<(), With<BubbleShieldVisual>>,
) {
    if !existing.is_empty() {
        return;
    }
    let Some(sprite) = sprite else { return };
    if sprite.handle == Handle::default() {
        return;
    }
    commands.spawn((
        Sprite {
            image: sprite.handle.clone(),
            custom_size: Some(bevy::math::Vec2::new(48.0, 64.0)),
            color: Color::srgba(0.5, 0.8, 1.0, 0.0),
            ..default()
        },
        // Render behind the player sprite so the ring feels like a field
        // around the body rather than a foreground overlay.
        Transform::from_xyz(
            0.0,
            0.0,
            ambition_gameplay_core::config::WORLD_Z_PLAYER - 0.05,
        ),
        Visibility::Hidden,
        BubbleShieldVisual,
        Name::new("Bubble Shield Visual"),
    ));
}

/// Show / hide and tint the shield ring based on `BodyShieldState`.
/// Position and scale track the player body size.
pub fn sync_bubble_shield_visual(
    world: Res<ambition_gameplay_core::RoomGeometry>,
    player_q: Query<
        (
            &ambition_gameplay_core::player::BodyKinematics,
            &ambition_gameplay_core::actor::BodyShieldState,
        ),
        ambition_gameplay_core::player::PrimaryPlayerOnly,
    >,
    mut shield_q: Query<(&mut Transform, &mut Sprite, &mut Visibility), With<BubbleShieldVisual>>,
) {
    let Ok((kin, shield)) = player_q.single() else {
        return;
    };
    let Ok((mut transform, mut sprite, mut vis)) = shield_q.single_mut() else {
        return;
    };

    if !shield.active {
        *vis = Visibility::Hidden;
        return;
    }

    // Position centered on the player body.
    transform.translation = ambition_gameplay_core::config::world_to_bevy(
        &world.0,
        kin.pos,
        ambition_gameplay_core::config::WORLD_Z_PLAYER - 0.05,
    );

    // Scale the ring to be slightly larger than the player collider so it
    // reads as surrounding the body without clipping into it.
    let render = bevy::math::Vec2::new(kin.size.x * 1.55, kin.size.y * 1.25);
    sprite.custom_size = Some(render);

    // Parry window: gold glow. Held but expired: soft cyan.
    sprite.color = if shield.parrying() {
        Color::srgba(1.0, 0.95, 0.40, 0.90)
    } else {
        Color::srgba(0.50, 0.80, 1.0, 0.55)
    };

    *vis = Visibility::Visible;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_bubble_shield_image_is_correct_size() {
        let img = build_bubble_shield_image();
        assert_eq!(img.width(), SHIELD_TEXTURE_SIZE);
        assert_eq!(img.height(), SHIELD_TEXTURE_SIZE);
    }

    #[test]
    fn build_bubble_shield_image_center_is_transparent() {
        let img = build_bubble_shield_image();
        let data = img.data.as_ref().expect("image data");
        // Center pixel should be inside the hole (alpha ~ 0).
        let size = SHIELD_TEXTURE_SIZE as usize;
        let cx = size / 2;
        let cy = size / 2;
        let i = (cy * size + cx) * 4;
        let alpha = data[i + 3];
        assert!(
            alpha < 20,
            "center should be transparent, got alpha={alpha}"
        );
    }

    #[test]
    fn build_bubble_shield_image_ring_is_opaque() {
        let img = build_bubble_shield_image();
        let data = img.data.as_ref().expect("image data");
        // A pixel halfway between inner and outer radius should be opaque.
        let size = SHIELD_TEXTURE_SIZE as usize;
        let cx = (size as f32 - 1.0) * 0.5;
        let mid_r = (SHIELD_TEXTURE_SIZE as f32 * (0.32 + 0.46) * 0.5) as usize;
        let px = (cx + mid_r as f32) as usize;
        let py = cx as usize;
        let i = (py * size + px) * 4;
        let alpha = data[i + 3];
        assert!(alpha > 200, "ring should be opaque, got alpha={alpha}");
    }
}

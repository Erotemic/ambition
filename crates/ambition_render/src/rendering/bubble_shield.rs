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

use ambition_platformer_primitives::lifecycle::{
    ActiveSessionScope, SessionSpawnScope, SpawnSessionScopedExt,
};
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

/// One pooled bubble-ring sprite (hidden until `sync` assigns it to a shielder).
fn new_ring_sprite(handle: Handle<Image>) -> impl Bundle {
    (
        Sprite {
            image: handle,
            custom_size: Some(bevy::math::Vec2::new(48.0, 64.0)),
            color: Color::srgba(0.5, 0.8, 1.0, 0.0),
            ..default()
        },
        // Render behind the body sprite so the ring feels like a field around the
        // body rather than a foreground overlay.
        Transform::from_xyz(
            0.0,
            0.0,
            ambition_engine_core::config::WORLD_Z_PLAYER - 0.05,
        ),
        Visibility::Hidden,
        BubbleShieldVisual,
        Name::new("Bubble Shield Visual"),
    )
}

/// Seed the pool with one ring. `sync` grows it on demand when several bodies
/// shield at once.
pub fn spawn_bubble_shield_visual(
    mut commands: Commands,
    sprite: Option<Res<BubbleShieldSprite>>,
    active_session: Option<Res<ActiveSessionScope>>,
    existing: Query<(), With<BubbleShieldVisual>>,
) {
    if !existing.is_empty() {
        return;
    }
    let Some(sprite) = sprite else { return };
    if sprite.handle == Handle::default() {
        return;
    }
    let Some(session_scope) =
        SessionSpawnScope::for_optional_active_session(active_session.as_deref())
    else {
        return;
    };
    commands.spawn_session_scoped(session_scope, new_ring_sprite(sprite.handle.clone()));
}

/// Parry window: gold glow. Held but expired: soft cyan.
fn shield_ring_color(parrying: bool) -> Color {
    if parrying {
        Color::srgba(1.0, 0.95, 0.40, 0.90)
    } else {
        Color::srgba(0.50, 0.80, 1.0, 0.55)
    }
}

/// Show / hide + tint a bubble ring around EVERY body whose shield is up — the
/// player AND any brain-controlled actor (the duel fighters). One pooled ring per
/// active shielder; unused rings hide, and the pool grows on demand. So an AI
/// shield now reads IDENTICALLY to the player's (it previously drew nothing for
/// actors — the ring was `PrimaryPlayer`-only). Scale tracks each body's size.
pub fn sync_bubble_shield_visual(
    mut commands: Commands,
    sprite: Option<Res<BubbleShieldSprite>>,
    active_session: Option<Res<ActiveSessionScope>>,
    world: ambition_platformer_primitives::lifecycle::SessionWorldRef<ambition_engine_core::RoomGeometry>,
    // Every raised shield, resolved sim-side into the pooled-ring read-model
    // (E4): render positions rings, it no longer queries the live clusters.
    active: Res<ambition_sim_view::ShieldRingsView>,
    mut rings: Query<(&mut Transform, &mut Sprite, &mut Visibility), With<BubbleShieldVisual>>,
) {
    let active = &active.0;
    let ring_count = rings.iter().count();
    let mut assigned = 0usize;
    for (mut transform, mut sprite, mut vis) in &mut rings {
        if let Some(ring) = active.get(assigned).copied() {
            transform.translation = ambition_engine_core::config::world_to_bevy(
                &world.0,
                ring.pos,
                ambition_engine_core::config::WORLD_Z_PLAYER - 0.05,
            );
            // Slightly larger than the collider so it surrounds the body.
            sprite.custom_size = Some(bevy::math::Vec2::new(
                ring.size.x * 1.55,
                ring.size.y * 1.25,
            ));
            sprite.color = shield_ring_color(ring.parrying);
            *vis = Visibility::Visible;
            assigned += 1;
        } else {
            *vis = Visibility::Hidden;
        }
    }

    // More bodies shielding than rings in the pool → grow it (the new rings get
    // positioned next frame). Spawn-on-demand keeps the common 0-1 shielder case at
    // a single sprite.
    if active.len() > ring_count {
        if let Some(sprite) = sprite {
            if sprite.handle != Handle::default() {
                let Some(session_scope) =
                    SessionSpawnScope::for_optional_active_session(active_session.as_deref())
                else {
                    return;
                };
                for _ in ring_count..active.len() {
                    commands.spawn_session_scoped(
                        session_scope,
                        new_ring_sprite(sprite.handle.clone()),
                    );
                }
            }
        }
    }
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

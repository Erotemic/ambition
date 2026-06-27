// ---------------------------------------------------------------------------
// Morph ball sprite (procedural)
// ---------------------------------------------------------------------------
//
// The shipped player spritesheet has no `MorphBall` row, but we still want
// the morph ball to look distinct from a crouched robot mid-game. Generating
// a small RGBA circle at startup avoids a parallel "render Morph row" task
// in the gen2d toolchain and keeps the mechanic playable today. Future art
// can replace this with a real spritesheet row by setting the
// `MorphBallSprite` handle to a loaded asset and the same toggle logic
// applies.

use ambition_engine_core as ae;
use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

/// Procedural sphere texture. Built once at startup, shown on a sibling
/// of the player while `Player::body_mode == MorphBall`.
#[derive(Resource, Clone, Default)]
pub struct MorphBallSprite {
    pub handle: Handle<Image>,
}

/// Marker on the morph-ball sibling sprite. The sprite is hidden by
/// default and mirrored to the player's position by
/// `sync_morph_ball_visual` when active.
#[derive(Component)]
pub struct MorphBallVisual;

const MORPH_BALL_TEXTURE_SIZE: u32 = 64;

/// Generate a 64x64 RGBA circle with a soft anti-aliased rim and a
/// top-left highlight so the ball reads as a sphere even at small render
/// sizes. Color matches the steel-blue palette of the player robot's
/// fallback rectangle (`Color::srgba(0.80, 0.95, 1.0, 1.0)`) so the
/// visual ties back to the standing body.
pub fn build_morph_ball_image() -> Image {
    let size = MORPH_BALL_TEXTURE_SIZE;
    let mut data = vec![0u8; (size * size * 4) as usize];
    let cx = (size as f32 - 1.0) * 0.5;
    let cy = cx;
    let radius = size as f32 * 0.5;
    // Anti-alias band width (pixels): edge fades from 1.0 → 0.0 alpha
    // across this many pixels at the sphere boundary.
    let edge = 1.5_f32;
    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let dist = (dx * dx + dy * dy).sqrt();
            let alpha = ((radius - dist) / edge).clamp(0.0, 1.0);
            // Top-left highlight: dot product with (-0.7, -0.7) direction.
            let nx = if dist > 0.001 { dx / radius } else { 0.0 };
            let ny = if dist > 0.001 { dy / radius } else { 0.0 };
            let highlight_dot = (-nx * 0.7 - ny * 0.7).clamp(0.0, 1.0);
            let highlight = highlight_dot.powf(2.5) * 0.55;
            // Rim shading: darker near the edge for spherical depth.
            let rim_factor = (1.0 - (dist / radius).powf(3.0)).clamp(0.0, 1.0);
            let base = 0.35 + 0.40 * rim_factor;
            let value = (base + highlight).clamp(0.0, 1.0);
            // Steel-blue tint: r=0.80, g=0.95, b=1.0 multiplied by value.
            let r = (value * 0.80 * 255.0) as u8;
            let g = (value * 0.95 * 255.0) as u8;
            let b = (value * 1.00 * 255.0) as u8;
            let a = (alpha * 255.0) as u8;
            let i = ((y * size + x) * 4) as usize;
            data[i] = r;
            data[i + 1] = g;
            data[i + 2] = b;
            data[i + 3] = a;
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

/// Startup system: build the procedural morph ball image and stash its
/// handle. The sibling visual is spawned by
/// `spawn_morph_ball_visual` once `SceneEntities` is populated.
pub fn build_morph_ball_sprite(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let handle = images.add(build_morph_ball_image());
    commands.insert_resource(MorphBallSprite { handle });
}

/// Spawn the morph-ball sibling sprite tied to the live `SceneEntities`.
/// Runs each frame but inserts only when the `MorphBallVisual` query is
/// empty — equivalent to a one-shot "after SceneEntities is ready"
/// guard that handles the visible-binary boot order without needing a
/// dedicated state.
pub fn spawn_morph_ball_visual(
    mut commands: Commands,
    sprite: Option<Res<MorphBallSprite>>,
    existing: Query<(), With<MorphBallVisual>>,
) {
    if !existing.is_empty() {
        return;
    }
    let Some(sprite) = sprite else {
        return;
    };
    if sprite.handle == Handle::default() {
        return;
    }
    commands.spawn((
        Sprite {
            image: sprite.handle.clone(),
            custom_size: Some(bevy::math::Vec2::new(16.0, 16.0)),
            ..default()
        },
        Transform::from_xyz(
            0.0,
            0.0,
            ambition_gameplay_core::config::WORLD_Z_PLAYER + 0.05,
        ),
        Visibility::Hidden,
        MorphBallVisual,
        Name::new("Morph Ball Visual"),
    ));
}

/// Toggle the morph-ball visual on / off based on `Player::body_mode`,
/// mirror its position to the player, and scale it to the morph-ball
/// AABB. Hides the regular player sprite while the ball is active so
/// the standing-rig animation doesn't show through.
pub fn sync_morph_ball_visual(
    world: Res<ambition_gameplay_core::RoomGeometry>,
    entities: Res<ambition_gameplay_core::platformer_runtime::lifecycle::SceneEntities>,
    player_q: Query<
        (
            &ambition_gameplay_core::actor::BodyKinematics,
            &ambition_gameplay_core::actor::BodyModeState,
        ),
        ambition_gameplay_core::actor::PrimaryPlayerOnly,
    >,
    mut player_query: Query<
        &mut Visibility,
        (
            With<ambition_gameplay_core::platformer_runtime::lifecycle::PlayerVisual>,
            Without<MorphBallVisual>,
        ),
    >,
    mut ball_query: Query<(&mut Transform, &mut Sprite, &mut Visibility), With<MorphBallVisual>>,
) {
    let Ok((mut transform, mut sprite, mut ball_visibility)) = ball_query.single_mut() else {
        return;
    };
    let Ok((kin, body_mode)) = player_q.single() else {
        return;
    };
    let in_morph = body_mode.body_mode == ae::BodyMode::MorphBall;
    if in_morph {
        transform.translation = ambition_gameplay_core::config::world_to_bevy(
            &world.0,
            kin.pos,
            ambition_gameplay_core::config::WORLD_Z_PLAYER + 0.05,
        );
        // Slightly larger than the AABB so the soft anti-aliased rim
        // reads as the ball's outline rather than as background.
        let render = bevy::math::Vec2::new(kin.size.x * 1.10, kin.size.y * 1.10);
        sprite.custom_size = Some(render);
        *ball_visibility = Visibility::Visible;
        if let Ok(mut player_vis) = player_query.get_mut(entities.player) {
            *player_vis = Visibility::Hidden;
        }
    } else {
        *ball_visibility = Visibility::Hidden;
        if let Ok(mut player_vis) = player_query.get_mut(entities.player) {
            // Inherited visibility lets the parent / overlay control
            // hiding (death overlay, room transition fade); we only
            // override to Visible when leaving morph ball, then drop
            // back to Inherited so we don't fight other systems.
            if matches!(*player_vis, Visibility::Hidden) {
                *player_vis = Visibility::Inherited;
            }
        }
    }
}

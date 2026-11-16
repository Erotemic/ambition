//! The darkness of a dark room (DR1, `docs/planning/engine/dark-rooms-and-tunnels.md`).
//!
//! A room says how dark it is with its level fields `darkness` (percent) and
//! `ambient` (its colour). The place gives it: a tunnel under the hill, a
//! cellar with its door shut. It is presentation: no simulation fact reads
//! light, so a replay and a peer agree whatever their quality.
//!
//! Drawn as a veil over the world, under the UI, like the daylight veil, by
//! the tier's [`DarknessQuality`]:
//! - `Flat` (Potato): one flat veil, never more than half dark, so what must
//!   be seen is seen.
//! - `AroundThePlayer`: the veil with a soft hole of light round the player:
//!   a sprite of a round falloff on the player, and four flat pieces round it.
//!
//! The light map that lamps and glows draw into is DR2's.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use ambition_persistence::settings::{DarknessQuality, ResolvedVisualQuality};
use ambition_platformer2d_shared_tangle::body::PrimaryLiveRoom;
use ambition_platformer2d_shared_tangle::camera_layers::MainCamera;
use ambition_platformer2d_shared_tangle::lifecycle::PlayerVisual;
use ambition_platformer2d_world::rooms::LiveRoomSpecs;

use super::daylight::DAYLIGHT_VEIL_Z;

/// Over the daylight veil, under the UI.
pub const DARKNESS_VEIL_Z: f32 = DAYLIGHT_VEIL_Z + 0.5;
/// Far past any view, as the daylight veil.
const FAR: f32 = 20_000.0;
/// The player's light: its radius in world units, and where its falloff
/// starts, as a part of the radius.
const LIGHT_RADIUS: f32 = 200.0;
const LIGHT_CORE: f32 = 0.35;
/// The cheapest tier is never more than half dark.
const FLAT_CAP: f32 = 0.5;
/// The darkness when a room names no `ambient`.
const DEFAULT_AMBIENT: Color = Color::srgb(0.02, 0.02, 0.06);
/// The side of the falloff texture, in texels.
const HOLE_TEXELS: u32 = 128;

/// One piece of the veil: the hole round the player, or a flat piece.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum DarknessVeil {
    Hole,
    Left,
    Right,
    Above,
    Below,
}

impl DarknessVeil {
    const ALL: [DarknessVeil; 5] = [Self::Hole, Self::Left, Self::Right, Self::Above, Self::Below];
}

/// The falloff texture of the hole: clear in its core, dark at its edge and
/// in its corners.
#[derive(Resource, Clone)]
pub struct DarknessHoleTexture(pub Handle<Image>);

fn hole_image() -> Image {
    let mut data = Vec::with_capacity((HOLE_TEXELS * HOLE_TEXELS * 4) as usize);
    let half = HOLE_TEXELS as f32 / 2.0;
    for y in 0..HOLE_TEXELS {
        for x in 0..HOLE_TEXELS {
            let dx = (x as f32 + 0.5 - half) / half;
            let dy = (y as f32 + 0.5 - half) / half;
            let r = (dx * dx + dy * dy).sqrt();
            let t = ((r - LIGHT_CORE) / (1.0 - LIGHT_CORE)).clamp(0.0, 1.0);
            let alpha = t * t * (3.0 - 2.0 * t);
            data.extend_from_slice(&[255, 255, 255, (alpha * 255.0).round() as u8]);
        }
    }
    Image::new(
        Extent3d { width: HOLE_TEXELS, height: HOLE_TEXELS, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}

/// The darkness of the primary live room, as a veil colour, or `None` when
/// the room is not dark.
fn room_darkness(rooms: Option<LiveRoomSpecs>, primary: Option<PrimaryLiveRoom>) -> Option<Color> {
    let (rooms, primary) = (rooms?, primary?);
    let definition = rooms.definition_named(primary.get())?;
    let profile = &rooms.rooms().spec(definition).metadata.visual_profile;
    let pct = profile.darkness_pct.filter(|pct| *pct > 0)?;
    let ambient = profile
        .ambient
        .as_deref()
        .and_then(|hex| Srgba::hex(hex).ok())
        .map(Color::from)
        .unwrap_or(DEFAULT_AMBIENT);
    Some(ambient.with_alpha(f32::from(pct) / 100.0))
}

/// The veil's alpha that makes the room look `perceived` dark. The veil is
/// blended in linear light, where an alpha of 0.7 takes a bright stone only
/// about 40% darker to the eye; the authored percent is what the eye sees.
fn veil_alpha(perceived: f32) -> f32 {
    1.0 - (1.0 - perceived.clamp(0.0, 1.0)).powf(2.2)
}

/// Where each piece of the veil goes, and how big it is (bevy world units),
/// for a veil round `light` (the player) under `camera`; `None` hides it.
fn layout(piece: DarknessVeil, quality: DarknessQuality, camera: Vec2, light: Option<Vec2>) -> Option<(Vec2, Vec2)> {
    let r = LIGHT_RADIUS;
    match (quality, light) {
        // Flat, or no player to light: one veil over all.
        (DarknessQuality::Flat, _) | (DarknessQuality::AroundThePlayer, None) => {
            (piece == DarknessVeil::Left).then_some((camera, Vec2::splat(2.0 * FAR)))
        }
        (DarknessQuality::AroundThePlayer, Some(p)) => Some(match piece {
            DarknessVeil::Hole => (p, Vec2::splat(2.0 * r)),
            DarknessVeil::Left => (Vec2::new(p.x - r - FAR / 2.0, p.y), Vec2::new(FAR, 2.0 * FAR)),
            DarknessVeil::Right => (Vec2::new(p.x + r + FAR / 2.0, p.y), Vec2::new(FAR, 2.0 * FAR)),
            DarknessVeil::Above => (Vec2::new(p.x, p.y + r + FAR / 2.0), Vec2::new(2.0 * r, FAR)),
            DarknessVeil::Below => (Vec2::new(p.x, p.y - r - FAR / 2.0), Vec2::new(2.0 * r, FAR)),
        }),
    }
}

/// Darken a dark room by the tier's darkness quality.
#[allow(clippy::too_many_arguments)]
pub fn darken_the_room(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    hole: Option<Res<DarknessHoleTexture>>,
    quality: Option<Res<ResolvedVisualQuality>>,
    rooms: Option<LiveRoomSpecs>,
    primary: Option<PrimaryLiveRoom>,
    cameras: Query<&Transform, (With<MainCamera>, Without<DarknessVeil>)>,
    players: Query<&GlobalTransform, (With<PlayerVisual>, Without<DarknessVeil>)>,
    mut veils: Query<(&DarknessVeil, &mut Transform, &mut Sprite, &mut Visibility)>,
) {
    let darkness = room_darkness(rooms, primary);
    if veils.is_empty() {
        if darkness.is_none() {
            return;
        }
        let texture = match hole {
            Some(hole) => hole.0.clone(),
            None => {
                let handle = images.add(hole_image());
                commands.insert_resource(DarknessHoleTexture(handle.clone()));
                handle
            }
        };
        for piece in DarknessVeil::ALL {
            let image = if piece == DarknessVeil::Hole { texture.clone() } else { Handle::default() };
            commands.spawn((
                piece,
                Sprite { image, color: Color::NONE, custom_size: Some(Vec2::ONE), ..Default::default() },
                Transform::from_translation(Vec3::new(0.0, 0.0, DARKNESS_VEIL_Z)),
                Visibility::Hidden,
                // The daylight veil's layer: the main cameras draw it, and no
                // portal capture does (which would darken its window twice).
                bevy::camera::visibility::RenderLayers::layer(
                    ambition_platformer2d_shared_tangle::camera_layers::PARALLAX_BACKGROUND_LAYER,
                ),
                Name::new("Darkness veil"),
            ));
        }
        return;
    }
    let Some(darkness) = darkness else {
        for (_, _, _, mut visibility) in &mut veils {
            visibility.set_if_neq(Visibility::Hidden);
        }
        return;
    };
    let quality = quality.map_or(DarknessQuality::default(), |q| q.budget.glow.darkness);
    let perceived = match quality {
        DarknessQuality::Flat => darkness.alpha().min(FLAT_CAP),
        DarknessQuality::AroundThePlayer => darkness.alpha(),
    };
    let darkness = darkness.with_alpha(veil_alpha(perceived));
    let camera = cameras.iter().next().map_or(Vec2::ZERO, |camera| camera.translation.truncate());
    let light = players.iter().next().map(|player| player.translation().truncate());
    for (piece, mut transform, mut sprite, mut visibility) in &mut veils {
        let Some((at, size)) = layout(*piece, quality, camera, light) else {
            visibility.set_if_neq(Visibility::Hidden);
            continue;
        };
        transform.translation = at.extend(DARKNESS_VEIL_Z);
        sprite.custom_size = Some(size);
        sprite.color = darkness;
        visibility.set_if_neq(Visibility::Inherited);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_veil_is_one_piece_when_flat_and_five_round_the_player() {
        let camera = Vec2::new(10.0, 20.0);
        let player = Some(Vec2::new(100.0, 50.0));
        let flat: Vec<_> = DarknessVeil::ALL.iter().filter_map(|p| layout(*p, DarknessQuality::Flat, camera, player)).collect();
        assert_eq!(flat.len(), 1);
        let round: Vec<_> = DarknessVeil::ALL
            .iter()
            .filter_map(|p| layout(*p, DarknessQuality::AroundThePlayer, camera, player))
            .collect();
        assert_eq!(round.len(), 5);
        // The hole is on the player, and the pieces meet its edges.
        assert_eq!(round[0], (Vec2::new(100.0, 50.0), Vec2::splat(2.0 * LIGHT_RADIUS)));
        let (left_at, left_size) = round[1];
        assert!((left_at.x + left_size.x / 2.0 - (100.0 - LIGHT_RADIUS)).abs() < 1e-3);
    }

    #[test]
    fn the_authored_percent_is_what_the_eye_sees() {
        // A bright stone (sRGB 0.8) under a black veil, blended in linear light.
        let seen = |perceived: f32| {
            let linear = Color::srgb(0.8, 0.8, 0.8).to_linear().red * (1.0 - veil_alpha(perceived));
            Color::LinearRgba(LinearRgba::rgb(linear, linear, linear)).to_srgba().red
        };
        assert!((seen(0.7) - 0.8 * 0.3).abs() < 0.03, "{}", seen(0.7));
        assert!((seen(0.5) - 0.8 * 0.5).abs() < 0.03, "{}", seen(0.5));
        assert_eq!(veil_alpha(0.0), 0.0);
    }

    #[test]
    fn the_hole_is_clear_at_its_middle_and_dark_at_its_corners() {
        let image = hole_image();
        let data = image.data.expect("pixels");
        let at = |x: u32, y: u32| data[((y * HOLE_TEXELS + x) * 4 + 3) as usize];
        assert_eq!(at(HOLE_TEXELS / 2, HOLE_TEXELS / 2), 0);
        assert_eq!(at(0, 0), 255);
    }
}

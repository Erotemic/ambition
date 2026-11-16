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
//! - `LightMap` (High, Ultra; DR2): one sprite over the view whose small
//!   texture is the darkness left where every light reaches: each player's,
//!   each lamp the room places (`LightSource`), and each thing that casts
//!   light ([`CastsLight`]: actor glows, fires). Computed on the CPU each
//!   frame at [`MAP_TEXELS`], each light over only the texels it reaches;
//!   drawn smooth. Where a lamp's light fades the veil takes its colour, so
//!   a lantern's pool has a warm rim; a player's light keeps the ambient.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use ambition_persistence::settings::{DarknessQuality, ResolvedVisualQuality};
use ambition_platformer2d_shared_tangle::body::PrimaryLiveRoom;
use ambition_platformer2d_shared_tangle::gameplay_presentation::{CastsLight, PlayerLight};
use ambition_platformer2d_shared_tangle::camera_layers::MainCamera;
use ambition_platformer2d_shared_tangle::lifecycle::PlayerVisual;
use ambition_platformer2d_world::rooms::LiveRoomSpecs;

use super::daylight::DAYLIGHT_VEIL_Z;
use super::occlusion::{lit_part, SolidGrid};

/// Which room's geometry a solid grid was built from: its name, size and
/// how many blocks and lines it has (a reloaded room that changed is new).
type GridKey = (String, Vec2, usize, usize);

/// A dark room's solid ground, and the reach of each of its lamps over it,
/// in the room's order.
pub struct RoomShadows {
    key: GridKey,
    grid: SolidGrid,
    lamps: Vec<Vec<f32>>,
}

/// Over the daylight veil, under the UI.
pub const DARKNESS_VEIL_Z: f32 = DAYLIGHT_VEIL_Z + 0.5;
/// Far past any view, as the daylight veil.
const FAR: f32 = 20_000.0;
/// Where a light's falloff starts, as a part of its radius. A player's
/// radius is the game's [`PlayerLight`].
const LIGHT_CORE: f32 = 0.35;
/// The cheapest tier is never more than half dark.
const FLAT_CAP: f32 = 0.5;
/// The darkness when a room names no `ambient`.
const DEFAULT_AMBIENT: Color = Color::srgb(0.02, 0.02, 0.06);
/// The side of the falloff texture, in texels.
const HOLE_TEXELS: u32 = 128;
/// The light map's texels over the view.
pub const MAP_TEXELS: UVec2 = UVec2::new(160, 90);
/// How much more than the view the light map covers, so the camera's own
/// motion inside a frame shows no edge.
const MAP_MARGIN: f32 = 1.25;
/// How much of a lamp's colour its rim leaves in the veil. The veil adds
/// its colour at its alpha, so a full lamp colour lays a haze over the scene;
/// a third of it is a warm edge to the dark.
const RIM_WARMTH: f32 = 0.3;

/// One piece of the veil: the hole round the player, or a flat piece.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum DarknessVeil {
    Hole,
    Left,
    Right,
    Above,
    Below,
    /// The light map over the view (`LightMap`).
    Map,
}

impl DarknessVeil {
    const ALL: [DarknessVeil; 6] = [Self::Hole, Self::Left, Self::Right, Self::Above, Self::Below, Self::Map];
}

/// The light map's texture.
#[derive(Resource, Clone)]
pub struct DarknessLightMap(pub Handle<Image>);

/// One light in the light map, in bevy world units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lamp {
    pub at: Vec2,
    pub radius: f32,
    pub intensity: f32,
    /// sRGB, 0 to 1.
    pub color: [f32; 3],
}

/// How much of the darkness `lamp` takes away at `p`: all of its intensity
/// in its core, none past its radius, a smooth fall between.
fn lamp_light(lamp: &Lamp, p: Vec2) -> f32 {
    let r = p.distance(lamp.at) / lamp.radius.max(1.0);
    let t = ((r - LIGHT_CORE) / (1.0 - LIGHT_CORE)).clamp(0.0, 1.0);
    lamp.intensity * (1.0 - t * t * (3.0 - 2.0 * t))
}

/// The light map's texels, row by row from the top: the veil over `size`
/// about `center`, `perceived` dark in `ambient` (sRGB), less what each lamp
/// lights. A texel's colour is the ambient and the lamps' colours, each as
/// much as it is there (the darkness left, each lamp's light).
/// With `reaches` (`ShadowedLightMap`; each lamp's, [`SolidGrid::reach`]),
/// a lamp's light stops at the ground.
fn light_map(
    texels: UVec2,
    center: Vec2,
    size: Vec2,
    perceived: f32,
    ambient: [f32; 3],
    lamps: &[Lamp],
    reaches: Option<&[Vec<f32>]>,
) -> Vec<u8> {
    let count = (texels.x * texels.y) as usize;
    // Per texel: the darkness left, the light, and the light's colour.
    let mut texel_light = vec![[1.0f32, 0.0, 0.0, 0.0, 0.0]; count];
    let texel = size / texels.as_vec2();
    let top_left = Vec2::new(center.x - size.x / 2.0, center.y + size.y / 2.0);
    let span = |low: f32, high: f32, n: u32| -> (u32, u32) {
        (low.floor().clamp(0.0, n as f32) as u32, high.ceil().clamp(0.0, n as f32) as u32)
    };
    for (n, lamp) in lamps.iter().enumerate().filter(|(_, lamp)| lamp.intensity > 0.0) {
        let r = lamp.radius.max(1.0);
        let (x0, x1) = span((lamp.at.x - r - top_left.x) / texel.x, (lamp.at.x + r - top_left.x) / texel.x, texels.x);
        let (y0, y1) = span((top_left.y - lamp.at.y - r) / texel.y, (top_left.y - lamp.at.y + r) / texel.y, texels.y);
        if x0 >= x1 || y0 >= y1 {
            continue;
        }
        let reach = reaches.and_then(|reaches| reaches.get(n));
        for y in y0..y1 {
            for x in x0..x1 {
                let p = Vec2::new(top_left.x + (x as f32 + 0.5) * texel.x, top_left.y - (y as f32 + 0.5) * texel.y);
                let mut light = lamp_light(lamp, p);
                if let (Some(reach), true) = (reach, light > 0.0) {
                    light *= lit_part(reach, lamp.at, p);
                }
                if light <= 0.0 {
                    continue;
                }
                let t = &mut texel_light[(y * texels.x + x) as usize];
                t[0] *= 1.0 - light;
                t[1] += light;
                let warmth = RIM_WARMTH * light;
                t[2] += lamp.color[0] * warmth;
                t[3] += lamp.color[1] * warmth;
                t[4] += lamp.color[2] * warmth;
            }
        }
    }
    // The veil's alpha by the darkness left, once per frame, not per texel.
    let alpha: Vec<u8> = (0..=255).map(|k| (veil_alpha(perceived * k as f32 / 255.0) * 255.0 + 0.5) as u8).collect();
    let mut data = Vec::with_capacity(count * 4);
    for &[dark, lit, r, g, b] in &texel_light {
        let scale = 255.0 / (dark + lit);
        let rgb = |ambient: f32, tint: f32| ((ambient * dark + tint) * scale + 0.5).min(255.0) as u8;
        data.extend_from_slice(&[
            rgb(ambient[0], r),
            rgb(ambient[1], g),
            rgb(ambient[2], b),
            alpha[(dark * 255.0 + 0.5) as usize],
        ]);
    }
    data
}

fn light_map_image() -> Image {
    let mut image = Image::new(
        Extent3d { width: MAP_TEXELS.x, height: MAP_TEXELS.y, depth_or_array_layers: 1 },
        TextureDimension::D2,
        vec![255; (MAP_TEXELS.x * MAP_TEXELS.y * 4) as usize],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );
    image.sampler = bevy::image::ImageSampler::linear();
    image
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

/// The darkness of the primary live room, as a veil colour, and its lamps;
/// or `None` when the room is not dark. With `solid`, the room's solid grid
/// is kept there, built again only for another room.
fn room_darkness(
    rooms: Option<LiveRoomSpecs>,
    primary: Option<PrimaryLiveRoom>,
    solid: Option<&mut Option<RoomShadows>>,
) -> Option<(Color, Vec<Lamp>)> {
    let (rooms, primary) = (rooms?, primary?);
    let definition = rooms.definition_named(primary.get())?;
    let spec = rooms.rooms().spec(definition);
    let profile = &spec.metadata.visual_profile;
    let pct = profile.darkness_pct.filter(|pct| *pct > 0)?;
    if let Some(solid) = solid {
        let world = &spec.world;
        let key = (spec.id.to_string(), world.size, world.blocks.iter().count(), world.chains.iter().count());
        if solid.as_ref().map(|built| &built.key) != Some(&key) {
            let grid = SolidGrid::of_world(world);
            // The room's lamps do not move: their reach is measured once.
            let lamps = spec
                .lights
                .iter()
                .map(|light| {
                    let at = ambition_platformer2d_core::config::world_to_bevy(world, light.payload.position, 0.0);
                    grid.reach(at.truncate(), light.payload.radius.max(1.0))
                })
                .collect();
            *solid = Some(RoomShadows { key, grid, lamps });
        }
    }
    let ambient = profile
        .ambient
        .as_deref()
        .and_then(|hex| Srgba::hex(hex).ok())
        .map(Color::from)
        .unwrap_or(DEFAULT_AMBIENT);
    let lamps = spec
        .lights
        .iter()
        .map(|light| Lamp {
            at: ambition_platformer2d_core::config::world_to_bevy(&spec.world, light.payload.position, 0.0).truncate(),
            radius: light.payload.radius,
            intensity: light.payload.intensity,
            color: light.payload.color,
        })
        .collect();
    Some((ambient.with_alpha(f32::from(pct) / 100.0), lamps))
}

/// The veil's alpha that makes the room look `perceived` dark. The veil is
/// blended in linear light, where an alpha of 0.7 takes a bright stone only
/// about 40% darker to the eye; the authored percent is what the eye sees.
fn veil_alpha(perceived: f32) -> f32 {
    1.0 - (1.0 - perceived.clamp(0.0, 1.0)).powf(2.2)
}

/// Where each piece of the veil goes, and how big it is (bevy world units),
/// for a veil round `light` (the player) under `camera`; `None` hides it.
fn layout(piece: DarknessVeil, quality: DarknessQuality, camera: Vec2, light: Option<Vec2>, r: f32) -> Option<(Vec2, Vec2)> {
    match (quality, light) {
        // The light map is one piece; the others' place is its own.
        (DarknessQuality::LightMap | DarknessQuality::ShadowedLightMap, _) => None,
        _ if piece == DarknessVeil::Map => None,
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
            DarknessVeil::Map => unreachable!("the map is placed by the light map"),
        }),
    }
}

/// Darken a dark room by the tier's darkness quality.
#[allow(clippy::too_many_arguments)]
pub fn darken_the_room(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    hole: Option<Res<DarknessHoleTexture>>,
    map: Option<Res<DarknessLightMap>>,
    quality: Option<Res<ResolvedVisualQuality>>,
    rooms: Option<LiveRoomSpecs>,
    primary: Option<PrimaryLiveRoom>,
    cameras: Query<(&Transform, Option<&Projection>), (With<MainCamera>, Without<DarknessVeil>)>,
    players: Query<&GlobalTransform, (With<PlayerVisual>, Without<DarknessVeil>)>,
    casters: Query<(&GlobalTransform, &CastsLight, &InheritedVisibility)>,
    player_light: Option<Res<PlayerLight>>,
    mut veils: Query<(&DarknessVeil, &mut Transform, &mut Sprite, &mut Visibility)>,
    // What the light map was last drawn from: a still frame draws nothing.
    mut drawn: Local<Option<(Vec2, Vec2, f32, [f32; 3], Vec<Lamp>, Option<GridKey>)>>,
    // The room's solid ground, when the light map is shadowed.
    mut solid: Local<Option<RoomShadows>>,
) {
    let quality = quality.map_or(DarknessQuality::default(), |q| q.budget.glow.darkness);
    let shadowed = quality == DarknessQuality::ShadowedLightMap;
    let darkness = room_darkness(rooms, primary, shadowed.then_some(&mut *solid));
    let room_lamps = darkness.as_ref().map_or(0, |(_, lamps)| lamps.len());
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
        let map_texture = match &map {
            Some(map) => map.0.clone(),
            None => {
                let handle = images.add(light_map_image());
                commands.insert_resource(DarknessLightMap(handle.clone()));
                handle
            }
        };
        for piece in DarknessVeil::ALL {
            let image = match piece {
                DarknessVeil::Hole => texture.clone(),
                DarknessVeil::Map => map_texture.clone(),
                _ => Handle::default(),
            };
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
    let Some((darkness, lamps)) = darkness else {
        for (_, _, _, mut visibility) in &mut veils {
            visibility.set_if_neq(Visibility::Hidden);
        }
        return;
    };
    let perceived = match quality {
        DarknessQuality::Flat => darkness.alpha().min(FLAT_CAP),
        DarknessQuality::AroundThePlayer | DarknessQuality::LightMap | DarknessQuality::ShadowedLightMap => {
            darkness.alpha()
        }
    };
    let (camera, view) = cameras.iter().next().map_or((Vec2::ZERO, Vec2::new(1280.0, 720.0)), |(transform, projection)| {
        let view = match projection {
            Some(Projection::Orthographic(o)) => o.area.size(),
            _ => Vec2::new(1280.0, 720.0),
        };
        (transform.translation.truncate(), view)
    });
    let light = players.iter().next().map(|player| player.translation().truncate());
    let player_light = player_light.map_or(PlayerLight::default(), |light| *light);
    if matches!(quality, DarknessQuality::LightMap | DarknessQuality::ShadowedLightMap) {
        let size = view * MAP_MARGIN;
        let ambient = darkness.to_srgba();
        let ambient = [ambient.red, ambient.green, ambient.blue];
        let mut all = lamps;
        all.extend(players.iter().map(|player| Lamp {
            at: player.translation().truncate(),
            radius: player_light.radius,
            intensity: 1.0,
            color: player_light.color.unwrap_or(ambient),
        }));
        all.extend(casters.iter().filter(|(_, _, shown)| shown.get()).map(|(at, light, _)| Lamp {
            at: at.translation().truncate(),
            radius: light.radius,
            intensity: light.intensity.clamp(0.0, 1.0),
            color: light.color,
        }));
        let shadows = solid.as_ref().filter(|_| shadowed);
        let inputs = (camera, size, perceived, ambient, all, shadows.map(|shadows| shadows.key.clone()));
        if drawn.as_ref() != Some(&inputs) {
            if let Some(mut image) = map.as_ref().and_then(|map| images.get_mut(&map.0)) {
                // The room's lamps' reach was measured with the room; the
                // lights that move are measured now.
                let reaches = shadows.map(|shadows| {
                    let mut reaches = shadows.lamps.clone();
                    reaches.extend(inputs.4[room_lamps..].iter().map(|lamp| shadows.grid.reach(lamp.at, lamp.radius.max(1.0))));
                    reaches
                });
                image.data = Some(light_map(MAP_TEXELS, camera, size, perceived, ambient, &inputs.4, reaches.as_deref()));
                *drawn = Some(inputs);
            }
        }
        for (piece, mut transform, mut sprite, mut visibility) in &mut veils {
            if *piece != DarknessVeil::Map {
                visibility.set_if_neq(Visibility::Hidden);
                continue;
            }
            transform.translation = camera.extend(DARKNESS_VEIL_Z);
            sprite.custom_size = Some(size);
            // The colour is in the texture.
            sprite.color = Color::WHITE;
            visibility.set_if_neq(Visibility::Inherited);
        }
        return;
    }
    let darkness = darkness.with_alpha(veil_alpha(perceived));
    for (piece, mut transform, mut sprite, mut visibility) in &mut veils {
        let Some((at, size)) = layout(*piece, quality, camera, light, player_light.radius) else {
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
        let r = 200.0;
        let flat: Vec<_> = DarknessVeil::ALL.iter().filter_map(|p| layout(*p, DarknessQuality::Flat, camera, player, r)).collect();
        assert_eq!(flat.len(), 1);
        let round: Vec<_> = DarknessVeil::ALL
            .iter()
            .filter_map(|p| layout(*p, DarknessQuality::AroundThePlayer, camera, player, r))
            .collect();
        assert_eq!(round.len(), 5);
        // The hole is on the player, and the pieces meet its edges.
        assert_eq!(round[0], (Vec2::new(100.0, 50.0), Vec2::splat(2.0 * r)));
        let (left_at, left_size) = round[1];
        assert!((left_at.x + left_size.x / 2.0 - (100.0 - r)).abs() < 1e-3);
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
    fn the_light_map_is_clear_at_each_lamp_and_dark_between() {
        let lamps = [
            Lamp { at: Vec2::new(-400.0, 0.0), radius: 200.0, intensity: 1.0, color: [1.0, 0.75, 0.4] },
            Lamp { at: Vec2::new(400.0, 0.0), radius: 200.0, intensity: 0.5, color: [1.0, 0.75, 0.4] },
        ];
        let texels = UVec2::new(160, 90);
        let size = Vec2::new(1600.0, 900.0);
        let map = light_map(texels, Vec2::ZERO, size, 0.7, [0.0, 0.0, 0.05], &lamps, None);
        // The texel whose middle is at world `p`, its channel `c`.
        let texel = |p: Vec2, c: u32| {
            let x = ((p.x / size.x + 0.5) * texels.x as f32) as u32;
            let y = ((0.5 - p.y / size.y) * texels.y as f32) as u32;
            map[((y * texels.x + x) * 4 + c) as usize]
        };
        let at = |p: Vec2| texel(p, 3);
        let full = (veil_alpha(0.7) * 255.0).round() as u8;
        assert_eq!(at(Vec2::new(-400.0, 0.0)), 0, "a full lamp clears its core");
        assert_eq!(at(Vec2::ZERO), full, "between the lamps the room is as dark as it says");
        let half = at(Vec2::new(400.0, 0.0));
        assert!(half > 0 && half < full, "a half lamp takes half the darkness: {half} of {full}");
        let rim = Vec2::new(-400.0 + 150.0, 0.0);
        assert!(texel(rim, 0) > texel(rim, 2), "a warm lamp's rim is warm");
        assert!(texel(Vec2::ZERO, 2) > texel(Vec2::ZERO, 0), "the dark between is the ambient");
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

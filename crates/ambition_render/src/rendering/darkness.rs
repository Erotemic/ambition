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
//! - `LightMap` (High, Ultra; DR2): the darkness left where every light
//!   reaches: each player's, each lamp the room places (`LightSource`), and
//!   each thing that casts light ([`CastsLight`]: actor glows, fires). Where
//!   a lamp's light fades the veil takes its colour, so a lantern's pool has
//!   a warm rim; a player's light keeps the ambient. Summed per pixel on the
//!   GPU ([`DarknessMaterial`], [`super::lamps`]), so a light's falloff and
//!   its shadow's edge are as fine as the screen. Where no 2D material can be
//!   built (no renderer, Android), a sprite over the view whose small texture
//!   is the same sum computed on the CPU each frame at [`MAP_TEXELS`], each
//!   light over only the texels it reaches, drawn smooth.

use std::collections::HashMap;

use bevy::asset::RenderAssetUsages;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_resource::{AsBindGroup, Extent3d, TextureDimension, TextureFormat};
use bevy::shader::ShaderRef;
use bevy::sprite_render::{AlphaMode2d, Material2d, Material2dPlugin};

use ambition_characters::control::DrivingParticipant;
use ambition_persistence::settings::{DarknessQuality, ResolvedVisualQuality};
use ambition_platformer2d_core::config::world_to_bevy;
use ambition_platformer2d_shared_tangle::gameplay_presentation::{CastsLight, PlayerLight};
use ambition_platformer2d_shared_tangle::lifecycle::{LiveRoomInstance, LiveRooms};
use ambition_platformer2d_world::rooms::{LiveRoomSpecs, RoomSpec};
use ambition_sim_view::presented_pose::draw_pos;
use ambition_sim_view::{BodyPoseView, PresentedForView, PresentedPose, ResolvedViewSubject};

use super::camera::coverage::GameplayOutputs;
use super::daylight::DAYLIGHT_VEIL_Z;
use super::lamps::{pack, reach_image, LampsPlugin, LampsUniform, ReachWrites};
use super::occlusion::{lit_part, SolidGrid};
use super::planar_mesh::{install_planar_mesh_2d, PreparedPlanarMesh};
use super::view_isolation::lighting_layers;

/// Which room's geometry a solid grid was built from: its name, size and
/// how many blocks and lines it has (a reloaded room that changed is new).
pub(crate) type GridKey = (String, Vec2, usize, usize);

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
#[require(crate::rendering::AnimatedWorldSpriteTint)]
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

/// The veil at High and Ultra, summed per pixel: the room's ambient (sRGB)
/// and how dark it looks, its lights, and how far each reaches (Ultra).
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone, PartialEq)]
pub struct DarknessMaterial {
    #[uniform(0)]
    pub veil: Vec4,
    #[uniform(1)]
    pub lamps: LampsUniform,
    #[texture(2, sample_type = "float", filterable = false)]
    pub reach: Option<Handle<Image>>,
}

impl Material2d for DarknessMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://ambition_render/rendering/darkness.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

/// The geometry shared by the shader light maps.
#[derive(Resource)]
struct DarknessMesh(Handle<Mesh>);

/// Installs the per-pixel veil's material. Android draws the light map on the CPU.
pub struct DarknessPlugin;

impl Plugin for DarknessPlugin {
    fn build(&self, app: &mut App) {
        if app.get_sub_app(bevy::render::RenderApp).is_none() || cfg!(target_os = "android") {
            return;
        }
        ambition_sprite_fx::install_mesh_fragment_shader(app);
        bevy::asset::embedded_asset!(app, "darkness.wgsl");
        if !app.is_plugin_added::<LampsPlugin>() {
            app.add_plugins(LampsPlugin);
        }
        app.add_plugins(Material2dPlugin::<DarknessMaterial>::default());
        install_planar_mesh_2d::<DarknessMaterial>(app);
        let world = app.world_mut();
        if let Some(mut meshes) = world.get_resource_mut::<Assets<Mesh>>() {
            let mesh = meshes.add(Rectangle::new(2.0 * FAR, 2.0 * FAR));
            world.insert_resource(DarknessMesh(mesh));
        }
    }
}

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

/// What the lamps give each texel of a map over `size` about `center`, row by
/// row from the top: `[the darkness left, the light, and the light's colour
/// (each lamp's colour by its light, summed)]`. The darkness left is the
/// product of what each lamp leaves; the light is their sum. With `reaches`
/// (each lamp's, [`SolidGrid::reach`]) a lamp's light stops at the ground.
pub(crate) fn lamp_texels(
    texels: UVec2,
    center: Vec2,
    size: Vec2,
    lamps: &[Lamp],
    reaches: Option<&[Vec<f32>]>,
) -> Vec<[f32; 5]> {
    let count = (texels.x * texels.y) as usize;
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
                t[2] += lamp.color[0] * light;
                t[3] += lamp.color[1] * light;
                t[4] += lamp.color[2] * light;
            }
        }
    }
    texel_light
}

/// The light map's texels, row by row from the top: the veil over `size`
/// about `center`, `perceived` dark in `ambient` (sRGB), less what each lamp
/// lights ([`lamp_texels`]). A texel's colour is the ambient and the lamps'
/// colours, each as much as it is there (the darkness left, each lamp's
/// light, at [`RIM_WARMTH`]).
fn light_map(
    texels: UVec2,
    center: Vec2,
    size: Vec2,
    perceived: f32,
    ambient: [f32; 3],
    lamps: &[Lamp],
    reaches: Option<&[Vec<f32>]>,
) -> Vec<u8> {
    let texel_light = lamp_texels(texels, center, size, lamps, reaches);
    // The veil's alpha by the darkness left, once per frame, not per texel.
    let alpha: Vec<u8> = (0..=255).map(|k| (veil_alpha(perceived * k as f32 / 255.0) * 255.0 + 0.5) as u8).collect();
    let mut data = Vec::with_capacity(texel_light.len() * 4);
    for &[dark, lit, r, g, b] in &texel_light {
        let scale = 255.0 / (dark + lit);
        let rgb = |ambient: f32, tint: f32| ((ambient * dark + RIM_WARMTH * tint) * scale + 0.5).min(255.0) as u8;
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

/// The lamps a room places (`LightSource`), in the room's order.
pub(crate) fn room_lamps(spec: &RoomSpec) -> Vec<Lamp> {
    spec.lights
        .iter()
        .map(|light| Lamp {
            at: ambition_platformer2d_core::config::world_to_bevy(&spec.world, light.payload.position, 0.0).truncate(),
            radius: light.payload.radius,
            intensity: light.payload.intensity,
            color: light.payload.color,
        })
        .collect()
}

impl RoomShadows {
    /// Keep `kept` the shadows of `spec`'s room, built again only for another
    /// room (or the same room changed).
    pub(crate) fn keep_for(kept: &mut Option<RoomShadows>, spec: &RoomSpec) {
        let world = &spec.world;
        let key = (spec.id.to_string(), world.size, world.blocks.iter().count(), world.chains.iter().count());
        if kept.as_ref().map(|built| &built.key) == Some(&key) {
            return;
        }
        let grid = SolidGrid::of_world(world);
        // The room's lamps do not move: their reach is measured once.
        let lamps = room_lamps(spec).iter().map(|lamp| grid.reach(lamp.at, lamp.radius.max(1.0))).collect();
        *kept = Some(RoomShadows { key, grid, lamps });
    }

    /// The reach of the room's lamps, then of `moving` (lights that move,
    /// measured now), in that order.
    pub(crate) fn reaches(&self, moving: &[Lamp]) -> Vec<Vec<f32>> {
        let mut reaches = self.lamps.clone();
        reaches.extend(moving.iter().map(|lamp| self.grid.reach(lamp.at, lamp.radius.max(1.0))));
        reaches
    }

    pub(crate) fn key(&self) -> &GridKey {
        &self.key
    }
}

fn room_darkness(spec: &RoomSpec) -> Option<Color> {
    let profile = &spec.metadata.visual_profile;
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

#[derive(PartialEq)]
struct MapSample {
    room: LiveRoomInstance,
    bounds: Rect,
    ambient: Vec4,
    lamps: Vec<Lamp>,
    shadowed: bool,
}

enum ViewLightMap {
    Texture(Handle<Image>),
    Shader(PreparedPlanarMesh<DarknessMaterial>, Handle<Image>),
}

struct DarknessForView {
    root: Entity,
    pieces: [Option<Entity>; DarknessVeil::ALL.len()],
    map: Option<ViewLightMap>,
    drawn: Option<MapSample>,
    solid: Option<RoomShadows>,
}

/// Darkness retained for each live view.
#[derive(Default)]
pub struct ViewDarkness {
    views: HashMap<Entity, DarknessForView>,
    hole: Option<Handle<Image>>,
}

#[derive(SystemParam)]
pub struct DarknessAssets<'w> {
    images: ResMut<'w, Assets<Image>>,
    mesh: Option<Res<'w, DarknessMesh>>,
    materials: Option<ResMut<'w, Assets<DarknessMaterial>>>,
    writes: Option<ResMut<'w, ReachWrites>>,
}

impl DarknessAssets<'_> {
    fn light_map(&mut self) -> ViewLightMap {
        if let (Some(mesh), Some(materials)) = (self.mesh.as_deref(), self.materials.as_deref_mut()) {
            let reach = self.images.add(reach_image());
            let material = materials.add(DarknessMaterial {
                veil: Vec4::ZERO, lamps: LampsUniform::default(), reach: Some(reach.clone()),
            });
            return ViewLightMap::Shader(PreparedPlanarMesh { mesh: mesh.0.clone(), material }, reach);
        }
        ViewLightMap::Texture(self.images.add(light_map_image()))
    }

    fn draw_map(&mut self, map: &ViewLightMap, sample: &MapSample, reaches: Option<&[Vec<f32>]>) {
        let center = sample.bounds.center();
        let size = sample.bounds.size() * MAP_MARGIN;
        match map {
            ViewLightMap::Texture(handle) => {
                if let Some(mut image) = self.images.get_mut(handle) {
                    let ambient = sample.ambient.truncate().to_array();
                    image.data = Some(light_map(MAP_TEXELS, center, size, sample.ambient.w, ambient, &sample.lamps, reaches));
                }
            }
            ViewLightMap::Shader(mesh, reach) => {
                let (lamps, rows) = pack(&sample.lamps, reaches, center, size);
                if let (Some(rows), Some(writes)) = (rows, self.writes.as_deref_mut()) {
                    writes.0.insert(reach.id(), rows);
                }
                if let Some(materials) = self.materials.as_deref_mut() {
                    if let Some(mut material) = materials.get_mut(&mesh.material) {
                        material.veil = sample.ambient;
                        material.lamps = lamps;
                    }
                }
            }
        }
    }
}

type VeilPictures<'w, 's> = Query<'w, 's,
    (&'static mut Transform, Option<&'static mut Sprite>, &'static mut Visibility),
    (With<DarknessVeil>, Without<Camera2d>),
>;

impl DarknessForView {
    fn show(
        &mut self,
        commands: &mut Commands,
        pictures: &mut VeilPictures,
        piece: DarknessVeil,
        at: Vec2,
        size: Vec2,
        color: Color,
        image: Handle<Image>,
    ) {
        let slot = &mut self.pieces[piece as usize];
        let at = at.extend(DARKNESS_VEIL_Z);
        if let Some((mut transform, sprite, mut visibility)) = slot.and_then(|entity| pictures.get_mut(entity).ok()) {
            if transform.translation != at {
                transform.translation = at;
            }
            if let Some(mut sprite) = sprite {
                if sprite.image != image {
                    sprite.image = image;
                }
                if sprite.color != color {
                    sprite.color = color;
                }
                if sprite.custom_size != Some(size) {
                    sprite.custom_size = Some(size);
                }
            }
            visibility.set_if_neq(Visibility::Inherited);
            return;
        }
        let mut entity = commands.spawn((
            piece, ChildOf(self.root), lighting_layers(), Transform::from_translation(at), Name::new("Darkness veil"),
        ));
        if let (DarknessVeil::Map, Some(ViewLightMap::Shader(mesh, _))) = (piece, &self.map) {
            entity.insert(PreparedPlanarMesh { mesh: mesh.mesh.clone(), material: mesh.material.clone() });
        } else {
            entity.insert(Sprite { image, color, custom_size: Some(size), ..Default::default() });
        }
        *slot = Some(entity.id());
    }
}

type DarknessPlayers<'w, 's> = Query<
    'w,
    's,
    (Entity, &'static BodyPoseView, Option<&'static PresentedPose>),
    With<DrivingParticipant>,
>;

fn player_hole(
    players: impl Iterator<Item = (Entity, Vec2)>,
    subject: Option<Entity>,
    center: Vec2,
) -> Option<Vec2> {
    players
        .min_by(|(a, left), (b, right)| {
            (subject == Some(*b)).cmp(&(subject == Some(*a)))
                .then_with(|| left.distance_squared(center).total_cmp(&right.distance_squared(center)))
                .then(a.cmp(b))
        })
        .map(|(_, at)| at)
}

/// Draw each view's room darkness across its gameplay outputs.
#[allow(clippy::too_many_arguments)]
pub fn darken_the_room(
    mut commands: Commands,
    quality: Option<Res<ResolvedVisualQuality>>,
    rooms: Option<LiveRoomSpecs>,
    outputs: GameplayOutputs,
    views: Query<&ResolvedViewSubject>,
    roots: Query<(), With<PresentedForView>>,
    live: LiveRooms,
    players: DarknessPlayers,
    casters: Query<(Entity, &GlobalTransform, &CastsLight, &InheritedVisibility)>,
    player_light: Option<Res<PlayerLight>>,
    mut pictures: VeilPictures,
    mut assets: DarknessAssets,
    mut retained: Local<ViewDarkness>,
) {
    let mut dark = outputs.view_bounds();
    dark.retain(|_, (frame, _)| rooms.as_ref()
        .and_then(|rooms| rooms.definition_in(frame.room).map(|definition| rooms.rooms().spec(definition)))
        .and_then(room_darkness).is_some());
    retained.views.retain(|view, slot| {
        let keep = dark.contains_key(view) && roots.contains(slot.root);
        if !keep {
            commands.entity(slot.root).try_despawn();
        }
        keep
    });
    let Some(rooms) = rooms else {
        return;
    };
    let quality = quality.map_or(DarknessQuality::default(), |q| q.budget.glow.darkness);
    let shadowed = quality == DarknessQuality::ShadowedLightMap;
    let use_map = matches!(quality, DarknessQuality::LightMap | DarknessQuality::ShadowedLightMap);
    let player_light = player_light.map_or(PlayerLight::default(), |light| *light);
    let ViewDarkness { views: retained, hole } = &mut *retained;

    for (view, (frame, bounds)) in dark {
        let Some(spec) = rooms.definition_in(frame.room).map(|definition| rooms.rooms().spec(definition)) else {
            continue;
        };
        let Some(tint) = room_darkness(spec) else {
            continue;
        };
        let slot = retained.entry(view).or_insert_with(|| DarknessForView {
            root: commands.spawn((PresentedForView(view), Transform::default(), Visibility::Inherited, Name::new("View darkness"))).id(),
            pieces: [None; DarknessVeil::ALL.len()], map: None, drawn: None, solid: None,
        });
        if rooms.set_is_changed() {
            slot.drawn = None;
            slot.solid = None;
        }

        let player_positions = || players.iter()
            .filter(|(entity, _, _)| live.of(*entity) == Some(frame.room))
            .map(|(entity, pose, presented)| {
                (entity, world_to_bevy(&spec.world, draw_pos(pose, presented), 0.0).truncate())
            });
        let light = if quality == DarknessQuality::AroundThePlayer {
            let subject = views.get(view).ok().and_then(|subject| subject.0);
            player_hole(player_positions(), subject, frame.plane_center)
        } else {
            None
        };

        if use_map {
            if shadowed {
                RoomShadows::keep_for(&mut slot.solid, spec);
            }
            let rgba = tint.to_srgba();
            let ambient = [rgba.red, rgba.green, rgba.blue];
            let mut lamps = room_lamps(spec);
            let room_count = lamps.len();
            lamps.extend(player_positions()
                .map(|(_, at)| Lamp { at, radius: player_light.radius,
                    intensity: 1.0, color: player_light.color.unwrap_or(ambient) }));
            lamps.extend(casters.iter().filter(|(entity, _, _, shown)| shown.get() && live.of(*entity) == Some(frame.room))
                .map(|(_, at, light, _)| Lamp { at: at.translation().truncate(), radius: light.radius,
                    intensity: light.intensity.clamp(0.0, 1.0), color: light.color }));
            let sample = MapSample { room: frame.room, bounds, ambient: Vec3::from_array(ambient).extend(rgba.alpha), lamps, shadowed };
            let map = slot.map.get_or_insert_with(|| assets.light_map());
            if slot.drawn.as_ref() != Some(&sample) {
                let reaches = slot.solid.as_ref().filter(|_| shadowed).map(|grid| grid.reaches(&sample.lamps[room_count..]));
                assets.draw_map(map, &sample, reaches.as_deref());
                slot.drawn = Some(sample);
            }
        }

        let perceived = if quality == DarknessQuality::Flat { tint.alpha().min(FLAT_CAP) } else { tint.alpha() };
        for piece in DarknessVeil::ALL {
            if piece == DarknessVeil::Map && use_map {
                let image = match &slot.map {
                    Some(ViewLightMap::Texture(image)) => image.clone(),
                    _ => Handle::default(),
                };
                slot.show(&mut commands, &mut pictures, piece, bounds.center(), bounds.size() * MAP_MARGIN, Color::WHITE, image);
            } else if let Some((at, size)) = layout(piece, quality, frame.plane_center, light, player_light.radius) {
                let image = if piece == DarknessVeil::Hole {
                    hole.get_or_insert_with(|| assets.images.add(hole_image())).clone()
                } else {
                    Handle::default()
                };
                slot.show(&mut commands, &mut pictures, piece, at, size, tint.with_alpha(veil_alpha(perceived)), image);
            } else if let Some((_, _, mut visibility)) = slot.pieces[piece as usize].and_then(|entity| pictures.get_mut(entity).ok()) {
                visibility.set_if_neq(Visibility::Hidden);
            }
        }
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

    #[test]
    fn overlapping_rooms_keep_their_own_player_lights_and_veils() {
        use ambition_characters::control::PlayerSlot;
        use ambition_persistence::settings::{VisualQualityBudget, VisualQualityProfile};
        use ambition_platformer2d_core as ae;
        use ambition_platformer2d_shared_tangle::camera_layers::{MainCamera, PARALLAX_BACKGROUND_LAYER};
        use ambition_platformer2d_shared_tangle::lifecycle::{spawn_live_room, InRoomInstance};
        use ambition_platformer2d_world::rooms::{insert_room_set, LiveRoomDefinition, RoomSet};
        use ambition_sim_view::{LocalView, LocalViewId, PresentsView};
        use bevy::camera::{visibility::RenderLayers, RenderTargetInfo};
        use crate::rendering::{PresentedViewFrame, PresentedViewFrames};

        let mut app = App::new();
        let quality = |profile| ResolvedVisualQuality { profile, budget: VisualQualityBudget::for_profile(profile) };
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(std::time::Duration::ZERO))
            .insert_resource(quality(VisualQualityProfile::High))
            .insert_resource(DarknessMesh(Handle::default()))
            .init_resource::<Assets<Image>>()
            .init_resource::<Assets<DarknessMaterial>>()
            .add_systems(Update, (darken_the_room, super::super::view_isolation::isolate_per_view_projections).chain());
        let world = app.world_mut();
        let room = |id, darkness| {
            let mut spec = RoomSpec::new(id, ae::World::new(id, ae::Vec2::splat(1000.0), ae::Vec2::ZERO, Vec::new()));
            spec.metadata.visual_profile.darkness_pct = darkness;
            spec
        };
        insert_room_set(world, RoomSet::from_parts_or_panic("east", vec![room("east", Some(25)), room("west", Some(75)), room("yard", None)], Vec::new()));
        let east = LiveRoomInstance::ACTIVATION;
        let west = east.next();
        let yard = west.next();
        spawn_live_room(world, west, LiveRoomDefinition::from_index(1));
        spawn_live_room(world, yard, LiveRoomDefinition::from_index(2));
        let frame = |room| PresentedViewFrame { room, plane_center: Vec2::ZERO, roll_radians: 0.0, orthographic_scale: 1.0 };
        let [(first, first_camera), (second, second_camera)] = [(0, east, -80.0), (1, west, 120.0)].map(|(id, room, x)| {
            let pose = BodyPoseView { pos: Vec2::new(500.0 + x, 500.0), ..default() };
            let player = world.spawn((DrivingParticipant(PlayerSlot(id)), InRoomInstance(room), pose)).id();
            let view = world.spawn((LocalView, LocalViewId(id), ResolvedViewSubject(Some(player)))).id();
            let mut camera = Camera::default();
            camera.computed.target_info = Some(RenderTargetInfo { physical_size: UVec2::new(640, 360), scale_factor: 1.0 });
            let output = world.spawn((Camera2d, MainCamera, camera, PresentsView(view), RenderLayers::layer(PARALLAX_BACKGROUND_LAYER))).id();
            (view, output)
        });
        world.insert_resource(PresentedViewFrames::of_views(vec![(first, frame(east)), (second, frame(west))]));

        app.update();
        let world = app.world_mut();
        let outputs = [first_camera, second_camera].map(|id| world.get::<RenderLayers>(id).unwrap().clone());
        let material_rows = world.query::<(&RenderLayers, &PreparedPlanarMesh<DarknessMaterial>)>()
            .iter(world).map(|(layers, source)| (layers.clone(), source.material.clone())).collect::<Vec<_>>();
        assert_eq!(material_rows.len(), 2);
        let materials = world.resource::<Assets<DarknessMaterial>>();
        for (slot, (dark, x)) in [(0.25, -80.0), (0.75, 120.0)].into_iter().enumerate() {
            let (layers, material) = material_rows.iter().find(|(layers, _)| layers.intersects(&outputs[slot])).unwrap();
            assert!(!layers.intersects(&outputs[1 - slot]));
            let material = materials.get(material).unwrap();
            assert_eq!(material.veil.w, dark);
            assert_eq!(material.lamps.count, 1);
            assert_eq!(material.lamps.at[0].x, x);
        }

        app.insert_resource(quality(VisualQualityProfile::Medium));
        *app.world_mut().resource_mut::<PresentedViewFrames>().of_mut(second).unwrap() = frame(yard);
        app.update();
        let world = app.world_mut();
        assert!(world.query::<&PresentedForView>().iter(world).all(|projection| projection.0 == first));
        let mut pieces = world.query::<(&DarknessVeil, &Visibility, &Transform, &RenderLayers)>();
        let visible = pieces.iter(world).filter(|(_, vis, ..)| **vis != Visibility::Hidden).collect::<Vec<_>>();
        assert_eq!(visible.len(), 5);
        assert!(visible.iter().all(|(_, _, _, layers)| layers.intersects(&outputs[0]) && !layers.intersects(&outputs[1])));
        let hole = visible.iter().find(|(piece, ..)| **piece == DarknessVeil::Hole).unwrap();
        assert_eq!(hole.2.translation.x, -80.0);
    }
}

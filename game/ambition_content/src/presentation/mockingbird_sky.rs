//! The Mockingbird's anger, and where its fight goes.
//!
//! Angry, it lights itself on fire: flames come off its body, red in its
//! second phase and cold blue in its third. In the third the sky climbs into
//! space: the dark comes down from the top, the stars come out, the planet's
//! limb falls away below, and the moon crosses the room on its lane. Space is
//! authored art in parts, posed here; no shader draws it.
//!
//! All of it reads the conductor's record ([`view_of`]) and decides nothing:
//! the phase, the climb and the moon are the simulation's. A rewind takes the
//! look back with the record. The flames ease between two states on the
//! presentation clock, which is a look only.

use std::collections::HashMap;

use bevy::{
    image::TextureAtlasLayout,
    prelude::*,
    reflect::TypePath,
    render::render_resource::AsBindGroup,
    shader::ShaderRef,
    sprite::{Anchor, SpriteImageMode},
    sprite_render::{AlphaMode2d, Material2d, Material2dPlugin, MeshMaterial2d},
};

use ambition_boss_encounter::BossConfig;
use ambition_combat::FeatureId;
use ambition_content_modules::mockingbird::{moon_lane, MOCKINGBIRD_ID, MOON_RADIUS};
use ambition_extension_host::BodyRecords;
use ambition_platformer2d::characters::actor::BodyHealth;
use ambition_platformer2d_core::config::world_to_bevy;
use ambition_platformer2d_shared_tangle::lifecycle::{
    ActiveSessionScope, InRoomInstance, SessionScopeSet, SessionScopedEntity, SessionSpawnScope, SpawnSessionScopedExt,
};
use ambition_platformer2d_world::rooms::LiveRoomSpecs;
use ambition_render::rendering::{BossOverlaySet, FeatureVisual, RoomVisual};
use ambition_sprite_sheet::boss::BossAnimator;

use crate::bosses::mockingbird::{view_of, MockingbirdView};

/// The flames' quad, as a multiple of the body's sprite: the room the flames
/// have to rise and trail in.
const FLAME_SPREAD: f32 = 1.9;
/// In front of the body's sprite, behind the renderer's hit-flash overlay
/// (1.5), so a hit still flashes over the flames.
const FLAME_Z_BIAS: f32 = 0.7;
/// How fast the flames come up when it ignites and go out when it dies, per
/// second.
const FLAME_EASE: f32 = 1.6;
/// How fast red fire turns to cold fire, per second.
const COLD_EASE: f32 = 1.1;
/// Above the parallax panels (`-18.0..=-15.0`) and the room look's backdrop
/// (`-14.5`), below the blocks: the parts of space are between this and
/// 0.2 above it.
const SPACE_Z: f32 = -14.4;

pub fn install(app: &mut App) {
    // `content_asset!` needs the asset registries.
    if app.world().get_resource::<bevy::asset::io::embedded::EmbeddedAssetRegistry>().is_none() {
        return;
    }
    ambition_platformer2d::content_asset!(app, "src/presentation/shaders/mockingbird_fire.wgsl");
    // The material plugins install render-world state: they need an app that
    // renders.
    if app.get_sub_app(bevy::render::RenderApp).is_some() {
        app.add_plugins(Material2dPlugin::<FlameMaterial>::default());
    }
    app.add_systems(
        Update,
        (
            // Each one acts on a boss visual or on its own overlay: with
            // neither (no boss in the room, e.g. in another game), it has no work.
            (attach_flames, sync_flames, cleanup_flames)
                .chain()
                .in_set(BossOverlaySet)
                .run_if(resource_exists::<Assets<FlameMaterial>>)
                .run_if(any_with_component::<BossAnimator>.or_else(any_with_component::<FlameOverlay>)),
            present_space
                .in_set(SessionScopeSet::Presentation)
                .run_if(resource_exists::<AssetServer>)
                .run_if(any_with_component::<BossConfig>.or_else(any_with_component::<SpaceSky>)),
        ),
    );
}

// ── The fire on its body ──

/// `uv_rect`: the body's cell in its page. `control`: `x` how high the flames
/// are (0 none), `y` the x-flip, `z` how cold the fire is (0 red, 1 blue),
/// `w` the time. `shape`: `x` the quad as a multiple of the sprite, `y` the
/// sprite's width over its height.
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct FlameMaterial {
    #[uniform(0)]
    pub uv_rect: Vec4,
    #[uniform(1)]
    pub control: Vec4,
    #[uniform(2)]
    pub shape: Vec4,
    #[texture(3)]
    #[sampler(4)]
    pub body_texture: Handle<Image>,
}

impl Material2d for FlameMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://ambition_content/presentation/shaders/mockingbird_fire.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

/// On the Mockingbird's sprite: its flame sibling.
#[derive(Component, Debug, Clone, Copy)]
pub struct FlameSource {
    overlay: Entity,
}

/// On the sibling quad: whose flames they are, and where their two eases are.
#[derive(Component, Debug, Clone, Copy)]
pub struct FlameOverlay {
    source: Entity,
    /// How high the flames are: 0 none, 1 its second phase, more in its third.
    pub height: f32,
    /// How cold the fire is: 0 red, 1 blue.
    pub cold: f32,
}

/// How high the flames of a Mockingbird in `phase` are, and how cold: none
/// before it is angry, and none on a dead one.
pub fn flames_of(phase: u32, alive: bool) -> (f32, f32) {
    match (alive, phase) {
        (false, _) | (true, 0) => (0.0, 0.0),
        (true, 1) => (1.0, 0.0),
        (true, _) => (1.35, 1.0),
    }
}

/// Each Mockingbird's view and whether it lives, by its feature id.
fn birds_by_id<'a>(
    birds: impl Iterator<Item = (&'a FeatureId, &'a BossConfig, &'a BodyRecords, Option<&'a BodyHealth>)>,
) -> HashMap<&'a str, (Option<MockingbirdView>, bool)> {
    birds
        .filter(|(_, config, ..)| config.behavior.id == MOCKINGBIRD_ID)
        .map(|(id, _, records, health)| (id.0.as_str(), (view_of(records), health.is_none_or(|h| h.alive()))))
        .collect()
}

/// A flame sibling for each Mockingbird's sprite.
pub fn attach_flames(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<FlameMaterial>>,
    birds: Query<(&FeatureId, &BossConfig, &BodyRecords, Option<&BodyHealth>)>,
    candidates: Query<
        (Entity, &FeatureVisual, &Transform, &Sprite, Option<&SessionScopedEntity>, Option<&InRoomInstance>),
        (With<BossAnimator>, Without<FlameSource>),
    >,
) {
    if candidates.is_empty() {
        return;
    }
    let birds = birds_by_id(birds.iter());
    for (source, visual, transform, sprite, session_owner, room) in &candidates {
        if !birds.contains_key(visual.id.as_str()) {
            continue;
        }
        let material = materials.add(FlameMaterial {
            uv_rect: Vec4::ZERO,
            control: Vec4::ZERO,
            shape: Vec4::new(FLAME_SPREAD, 1.0, 0.0, 0.0),
            body_texture: sprite.image.clone(),
        });
        let overlay = commands
            .spawn_session_scoped(
                super::overlay_scope_of(session_owner, room),
                (
                    Mesh2d(meshes.add(Rectangle::default())),
                    MeshMaterial2d(material),
                    *transform,
                    // Hidden until it is angry.
                    Visibility::Hidden,
                    FlameOverlay { source, height: 0.0, cold: 0.0 },
                    RoomVisual,
                    Name::new(format!("Flames: {}", visual.id)),
                ),
            )
            .id();
        commands.entity(source).insert(FlameSource { overlay });
    }
}

/// Draw the flames off the cell the Mockingbird is drawn at, as high and as
/// cold as its phase says.
pub fn sync_flames(
    time: ambition_time::PresentationTime,
    mut elapsed: Local<f32>,
    layouts: Res<Assets<TextureAtlasLayout>>,
    birds: Query<(&FeatureId, &BossConfig, &BodyRecords, Option<&BodyHealth>)>,
    sources: Query<
        (&FeatureVisual, &Transform, &Sprite, Option<&Anchor>, &FlameSource, Option<&Visibility>),
        Without<FlameOverlay>,
    >,
    mut overlays: Query<(&mut Transform, &mut Visibility, &MeshMaterial2d<FlameMaterial>, &mut FlameOverlay)>,
    mut materials: ResMut<Assets<FlameMaterial>>,
) {
    let dt = time.scaled_dt();
    *elapsed = (*elapsed + dt).rem_euclid(3600.0);
    if sources.is_empty() {
        return;
    }
    let birds = birds_by_id(birds.iter());
    for (visual, transform, sprite, anchor, source, source_visibility) in &sources {
        let Ok((mut overlay_transform, mut overlay_visibility, material, mut flames)) = overlays.get_mut(source.overlay)
        else {
            continue;
        };
        let (phase, alive) = birds
            .get(visual.id.as_str())
            .map_or((0, false), |(view, alive)| (view.map_or(0, |view| view.phase), *alive));
        let (height, cold) = flames_of(phase, alive);
        flames.height = ease(flames.height, height, FLAME_EASE * dt);
        flames.cold = ease(flames.cold, cold, COLD_EASE * dt);
        let hidden = matches!(source_visibility, Some(v) if *v == Visibility::Hidden);
        let (Some(render_size), Some(uv_rect), false, true) =
            (sprite.custom_size, sprite_uv_rect(sprite, &layouts), hidden, flames.height > 0.01)
        else {
            *overlay_visibility = Visibility::Hidden;
            continue;
        };
        *overlay_transform = super::deep_dream::sibling_quad_transform(
            transform,
            anchor.map(|a| a.0),
            render_size,
            FLAME_Z_BIAS,
        );
        overlay_transform.scale = (render_size * FLAME_SPREAD).extend(1.0);
        *overlay_visibility = Visibility::Visible;
        if let Some(mut material) = materials.get_mut(&material.0) {
            material.uv_rect = uv_rect;
            material.control = Vec4::new(flames.height, if sprite.flip_x { 1.0 } else { 0.0 }, flames.cold, *elapsed);
            material.shape = Vec4::new(FLAME_SPREAD, render_size.x / render_size.y.max(1.0), 0.0, 0.0);
            if material.body_texture != sprite.image {
                material.body_texture = sprite.image.clone();
            }
        }
    }
}

/// A flame sibling whose Mockingbird is gone goes with it.
pub fn cleanup_flames(mut commands: Commands, sources: Query<(), With<FlameSource>>, overlays: Query<(Entity, &FlameOverlay)>) {
    for (overlay, of) in &overlays {
        if sources.get(of.source).is_err() {
            commands.entity(overlay).despawn();
        }
    }
}

/// `from` moved toward `to` by at most `step`.
fn ease(from: f32, to: f32, step: f32) -> f32 {
    from + (to - from).clamp(-step, step)
}

/// The sprite's cell in its page, as `min.xy, max.xy` in 0 to 1. A sprite with
/// no atlas is drawn whole.
fn sprite_uv_rect(sprite: &Sprite, layouts: &Assets<TextureAtlasLayout>) -> Option<Vec4> {
    let Some(atlas) = sprite.texture_atlas.as_ref() else {
        return Some(Vec4::new(0.0, 0.0, 1.0, 1.0));
    };
    let layout = layouts.get(&atlas.layout)?;
    let rect = layout.textures.get(atlas.index)?;
    let size = Vec2::new(layout.size.x.max(1) as f32, layout.size.y.max(1) as f32);
    Some(Vec4::new(
        rect.min.x as f32 / size.x,
        rect.min.y as f32 / size.y,
        rect.max.x as f32 / size.x,
        rect.max.y as f32 / size.y,
    ))
}

// ── Space, and the moon ──
//
// Space is authored art in parts (`backgrounds/sky_parts.py` in the art
// submodule), posed here as the fight climbs. The dark and its stars hang
// from one place that comes down from above the room; the planet goes down
// and gets smaller; the moon is where the record says; the dashes walk its
// lane. No part is drawn by a shader, so space looks the same with each
// shader off.

const SPACE_ART: &str = "backgrounds/sky_parts/mockingbird_space.png";
const STARS_FAR_ART: &str = "backgrounds/sky_parts/mockingbird_stars_far.png";
const STARS_NEAR_ART: &str = "backgrounds/sky_parts/mockingbird_stars_near.png";
const STARS_EDGE_ART: &str = "backgrounds/sky_parts/mockingbird_stars_edge.png";
const GLINT_ART: &str = "backgrounds/sky_parts/mockingbird_star_glint.png";
const PLANET_ART: &str = "backgrounds/sky_parts/mockingbird_planet.png";
const MOON_ART: &str = "backgrounds/sky_parts/mockingbird_moon.png";
const HALO_ART: &str = "backgrounds/sky_parts/mockingbird_moon_halo.png";
const DASH_ART: &str = "backgrounds/sky_parts/mockingbird_lane_dash.png";

// The geometry the art is drawn for. `sky_parts.py` has the same numbers:
// change the two together.
/// The dark at rest (the fight in space), in parts of the room: across and down.
const SPACE_U: [f32; 2] = [-0.75, 1.75];
const SPACE_V: [f32; 2] = [-1.0, 2.3 / 1.3];
/// Above this the dark is whole; below it, it fades to clear.
const DARK_TO_V: f32 = 1.0;
/// How much higher the dark is, in room heights, for each step of the climb
/// still to come.
const CLIMB_RISE: f32 = 2.3 / 1.3;
/// The planet's picture, in radii from the planet's centre.
const PLANET_X: f32 = 0.68;
const PLANET_Y: [f32; 2] = [-1.14, -0.74];
/// The moon's picture and its halo's, in radii from its centre.
const MOON_SPAN: f32 = 1.04;
const HALO_SPAN: f32 = 2.2;
/// The world size of a dash's picture.
const DASH_SIZE: Vec2 = Vec2::new(32.0, 8.0);
/// Each star picture is drawn one pixel to a world unit: these are their sizes.
const STARS_FAR_TILE: f32 = 512.0;
const STARS_NEAR_TILE: f32 = 768.0;
const STARS_EDGE_HEIGHT: f32 = 384.0;

/// The dashes of the lane: one each `DASH_PERIOD` px, running the way the
/// moon goes at `DASH_SPEED` periods a second.
const DASH_PERIOD: f32 = 56.0;
const DASH_SPEED: f32 = 0.6;
const DASHES: u8 = 32;
/// How fast each layer of stars drifts, as a part of the sky's scroll.
const FAR_DRIFT: f32 = 0.004;
const NEAR_DRIFT: f32 = 0.010;
/// The bright stars that twinkle, in parts of the room (at rest), with
/// their size in world units.
const GLINTS: [(f32, f32, f32); 9] = [
    (0.08, 0.12, 18.0),
    (0.27, 0.31, 14.0),
    (0.43, 0.07, 16.0),
    (0.58, 0.22, 20.0),
    (0.71, 0.41, 14.0),
    (0.86, 0.15, 18.0),
    (0.95, 0.58, 15.0),
    (0.18, 0.55, 16.0),
    (0.36, 0.71, 13.0),
];

/// The root of a room's space: its parts hang from it. Its room is its stamp
/// ([`InRoomInstance`]).
#[derive(Component, Debug, Clone, Copy)]
pub struct SpaceSky;

/// One part of a room's space, which [`pose_of`] places.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpacePart {
    /// The dark, with its dust and its soft lower edge.
    Dark,
    StarsFar,
    StarsNear,
    /// The stars that thin out in the soft edge of the dark.
    StarsEdge,
    /// One of [`GLINTS`].
    Glint(u8),
    Planet,
    MoonHalo,
    Moon,
    /// One dash of the moon's lane.
    Dash(u8),
}

impl SpacePart {
    fn all() -> impl Iterator<Item = SpacePart> {
        [Self::Dark, Self::StarsFar, Self::StarsNear, Self::StarsEdge, Self::Planet, Self::MoonHalo, Self::Moon]
            .into_iter()
            .chain((0..GLINTS.len() as u8).map(Self::Glint))
            .chain((0..DASHES).map(Self::Dash))
    }

    fn art(self) -> &'static str {
        match self {
            Self::Dark => SPACE_ART,
            Self::StarsFar => STARS_FAR_ART,
            Self::StarsNear => STARS_NEAR_ART,
            Self::StarsEdge => STARS_EDGE_ART,
            Self::Glint(_) => GLINT_ART,
            Self::Planet => PLANET_ART,
            Self::MoonHalo => HALO_ART,
            Self::Moon => MOON_ART,
            Self::Dash(_) => DASH_ART,
        }
    }

    /// In front of the part before it; all of them behind the blocks.
    fn depth(self) -> f32 {
        match self {
            Self::Dark => 0.0,
            Self::StarsFar => 0.02,
            Self::StarsEdge => 0.025,
            Self::StarsNear => 0.03,
            Self::Glint(_) => 0.04,
            Self::Planet => 0.10,
            Self::Dash(_) => 0.15,
            Self::MoonHalo => 0.18,
            Self::Moon => 0.20,
        }
    }

    /// The stars repeat; the others are one picture each.
    fn image_mode(self) -> SpriteImageMode {
        match self {
            Self::StarsFar | Self::StarsNear => SpriteImageMode::Tiled { tile_x: true, tile_y: true, stretch_value: 1.0 },
            Self::StarsEdge => SpriteImageMode::Tiled { tile_x: true, tile_y: false, stretch_value: 1.0 },
            _ => SpriteImageMode::Auto,
        }
    }
}

/// What the parts are posed from: the record's climb and moon, the sky's
/// scroll and the presentation clock.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkyMoment {
    pub ascent: f32,
    pub moon: Option<Vec2>,
    pub scroll_px_s: f32,
    pub time_s: f32,
}

/// Where a part is: its centre and size in room coordinates (y down), its
/// turn (radians, clockwise on screen) and its alpha.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PartPose {
    pub centre: Vec2,
    pub size: Vec2,
    pub turn: f32,
    pub alpha: f32,
}

fn smoothstep(lo: f32, hi: f32, x: f32) -> f32 {
    let t = ((x - lo) / (hi - lo)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// How far up the dark and its stars hang at `ascent` (negative: above where
/// they rest).
fn rise(ascent: f32, room: Vec2) -> f32 {
    -room.y * CLIMB_RISE * (1.0 - ascent.clamp(0.0, 1.0))
}

/// The planet's centre and radius at `ascent`: its top falls from the middle
/// of the room to near its bottom, and it curves more.
pub fn planet_disc(ascent: f32, room: Vec2) -> (Vec2, f32) {
    let climb = smoothstep(0.25, 1.0, ascent);
    let radius = room.x * (4.0 + (1.25 - 4.0) * climb);
    let top = room.y * (0.62 + (0.90 - 0.62) * climb);
    (Vec2::new(room.x * 0.5, top + radius), radius)
}

/// A field of tiled stars that drifts at `drift` of the sky's scroll: as
/// wide as the dark and one tile more, so the drift never shows its edge.
fn star_field(moment: &SkyMoment, room: Vec2, tile: f32, drift: f32, top: f32, height: f32) -> (Vec2, Vec2) {
    let shift = (moment.time_s * moment.scroll_px_s * drift).rem_euclid(tile);
    let size = Vec2::new(room.x * (SPACE_U[1] - SPACE_U[0]) + tile, height);
    (Vec2::new(room.x * SPACE_U[0] - tile + shift + size.x * 0.5, top + height * 0.5), size)
}

/// Where `part` is at `moment` in a room of size `room`, or `None` when it
/// is not seen.
pub fn pose_of(part: SpacePart, moment: &SkyMoment, room: Vec2) -> Option<PartPose> {
    let ascent = moment.ascent.clamp(0.0, 1.0);
    let rise = rise(ascent, room);
    let stars = smoothstep(0.15, 0.7, ascent);
    let pose = |centre: Vec2, size: Vec2, alpha: f32| (alpha > 0.002).then_some(PartPose { centre, size, turn: 0.0, alpha });
    let dark_top = room.y * SPACE_V[0] + rise;
    let dark_whole = room.y * DARK_TO_V + rise;
    match part {
        SpacePart::Dark => pose(
            Vec2::new(room.x * (SPACE_U[0] + SPACE_U[1]) * 0.5, room.y * (SPACE_V[0] + SPACE_V[1]) * 0.5 + rise),
            Vec2::new(room.x * (SPACE_U[1] - SPACE_U[0]), room.y * (SPACE_V[1] - SPACE_V[0])),
            if ascent > 0.0 { 1.0 } else { 0.0 },
        ),
        SpacePart::StarsFar | SpacePart::StarsNear => {
            let (tile, drift) =
                if part == SpacePart::StarsFar { (STARS_FAR_TILE, FAR_DRIFT) } else { (STARS_NEAR_TILE, NEAR_DRIFT) };
            let (centre, size) = star_field(moment, room, tile, drift, dark_top, dark_whole - dark_top);
            pose(centre, size, stars)
        }
        SpacePart::StarsEdge => {
            let (centre, size) = star_field(moment, room, STARS_FAR_TILE, FAR_DRIFT, dark_whole, STARS_EDGE_HEIGHT);
            pose(centre, size, stars)
        }
        SpacePart::Glint(index) => {
            let (u, v, side) = GLINTS[index as usize];
            let phase = index as f32;
            let twinkle = 0.55 + 0.45 * (moment.time_s * (0.7 + 0.13 * phase) + phase * 2.4).sin();
            pose(Vec2::new(room.x * u, room.y * v + rise), Vec2::splat(side), stars * twinkle)
        }
        SpacePart::Planet => {
            let (centre, radius) = planet_disc(ascent, room);
            pose(
                Vec2::new(centre.x, centre.y + radius * (PLANET_Y[0] + PLANET_Y[1]) * 0.5),
                Vec2::new(2.0 * PLANET_X * radius, (PLANET_Y[1] - PLANET_Y[0]) * radius),
                smoothstep(0.30, 0.75, ascent),
            )
        }
        SpacePart::Moon | SpacePart::MoonHalo => {
            let span = if part == SpacePart::Moon { MOON_SPAN } else { HALO_SPAN };
            moment.moon.and_then(|moon| pose(moon, Vec2::splat(2.0 * span * MOON_RADIUS), 1.0))
        }
        SpacePart::Dash(index) => {
            let [from, to, lane, arc] = moon_lane(room);
            let first = (to / DASH_PERIOD).floor() * DASH_PERIOD;
            let x = first + DASH_PERIOD * (index as f32 + 0.75 - (moment.time_s * DASH_SPEED).fract());
            let u = (from - x) / (from - to);
            if !(0.0..1.0).contains(&u) {
                return None;
            }
            let pi = std::f32::consts::PI;
            let slope = arc * pi * (u * pi).cos() / (from - to);
            let mut dash = pose(Vec2::new(x, lane - arc * (u * pi).sin()), DASH_SIZE, smoothstep(0.92, 1.0, ascent))?;
            dash.turn = slope.atan();
            Some(dash)
        }
    }
}

/// Give each live room whose Mockingbird has taken its fight toward space
/// its space, and pose each part as far up as the fight is.
#[allow(clippy::too_many_arguments)]
pub fn present_space(
    mut commands: Commands,
    time: ambition_time::PresentationTime,
    mut elapsed: Local<f32>,
    rooms: LiveRoomSpecs,
    birds: Query<(&BossConfig, &BodyRecords, Option<&InRoomInstance>), Without<SpaceSky>>,
    mut skies: Query<(Entity, &InRoomInstance, &mut Visibility), With<SpaceSky>>,
    mut parts: Query<(&SpacePart, &ChildOf, &mut Transform, &mut Sprite, &mut Visibility), Without<SpaceSky>>,
    asset_server: Option<Res<AssetServer>>,
    active_session: Option<Res<ActiveSessionScope>>,
) {
    *elapsed = (*elapsed + time.scaled_dt()).rem_euclid(3600.0);
    let Some(session_scope) = SessionSpawnScope::for_optional_active_session(active_session.as_deref()) else {
        return;
    };
    let mut posed: HashMap<Entity, (SkyMoment, Vec2)> = HashMap::new();
    for (room, definition) in rooms.live_rooms() {
        let view = birds
            .iter()
            .filter(|(config, _, stamp)| {
                config.behavior.id == MOCKINGBIRD_ID && stamp.is_none_or(|stamp| stamp.0 == room)
            })
            .find_map(|(_, records, _)| view_of(records))
            .filter(|view| view.ascent > 0.0 || view.moon.is_some());
        let sky = skies.iter_mut().find(|(_, stamp, _)| stamp.0 == room);
        let Some(view) = view else {
            if let Some((_, _, mut visibility)) = sky {
                visibility.set_if_neq(Visibility::Hidden);
            }
            continue;
        };
        let spec = rooms.rooms().spec(definition);
        let world = &spec.world;
        let size = Vec2::new(world.size.x, world.size.y);
        let moment = SkyMoment {
            ascent: view.ascent,
            moon: view.moon,
            scroll_px_s: spec.metadata.visual_profile.sky_scroll_px_s.unwrap_or(0) as f32,
            time_s: *elapsed,
        };
        match sky {
            Some((root, _, mut visibility)) => {
                visibility.set_if_neq(Visibility::Visible);
                posed.insert(root, (moment, size));
            }
            None => {
                let Some(asset_server) = asset_server.as_deref() else {
                    continue;
                };
                // The root is at the room's corner, so a part's place is in
                // room coordinates with y turned up.
                let root = commands
                    .spawn_session_scoped(
                        session_scope.in_room(Some(room)),
                        (
                            Transform::from_translation(world_to_bevy(world, world.size * 0.0, SPACE_Z)),
                            Visibility::Visible,
                            Name::new("Mockingbird: space"),
                            RoomVisual,
                            SpaceSky,
                        ),
                    )
                    .id();
                // Each part is posed from the next frame on.
                for part in SpacePart::all() {
                    commands.spawn((
                        Sprite { image: asset_server.load(part.art()), image_mode: part.image_mode(), ..default() },
                        Transform::from_xyz(0.0, 0.0, part.depth()),
                        Visibility::Hidden,
                        part,
                        ChildOf(root),
                    ));
                }
            }
        }
    }
    if posed.is_empty() {
        return;
    }
    for (part, parent, mut transform, mut sprite, mut visibility) in &mut parts {
        let Some((moment, size)) = posed.get(&parent.parent()) else {
            continue;
        };
        match pose_of(*part, moment, *size) {
            None => {
                visibility.set_if_neq(Visibility::Hidden);
            }
            Some(pose) => {
                transform.set_if_neq(
                    Transform::from_xyz(pose.centre.x, -pose.centre.y, part.depth())
                        .with_rotation(Quat::from_rotation_z(-pose.turn)),
                );
                // A tiled sprite cuts its tiles again each time it changes:
                // it is written only when its size or its alpha does.
                let (size, colour) = (Some(pose.size), Color::srgba(1.0, 1.0, 1.0, pose.alpha));
                if sprite.custom_size != size || sprite.color != colour {
                    sprite.custom_size = size;
                    sprite.color = colour;
                }
                visibility.set_if_neq(Visibility::Inherited);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// It burns only while it lives and is angry, and its fire is cold only
    /// in its third phase, where it burns higher.
    #[test]
    fn it_burns_red_in_its_second_phase_and_higher_and_cold_in_its_third() {
        assert_eq!(flames_of(0, true), (0.0, 0.0));
        let (red, cold) = (flames_of(1, true), flames_of(2, true));
        assert!(red.0 > 0.0 && red.1 == 0.0, "{red:?}");
        assert!(cold.0 > red.0 && cold.1 == 1.0, "{cold:?}");
        for phase in 0..3 {
            assert_eq!(flames_of(phase, false).0, 0.0, "a dead one burns in phase {phase}");
        }
    }

    const ROOM: Vec2 = Vec2::new(1280.0, 720.0);

    fn at(ascent: f32, time_s: f32) -> SkyMoment {
        SkyMoment { ascent, moon: None, scroll_px_s: -800.0, time_s }
    }

    /// On the ground the dark is above the room; in space it is whole over
    /// all of the room.
    #[test]
    fn the_dark_comes_down_from_above_the_room_until_it_covers_all_of_it() {
        assert_eq!(pose_of(SpacePart::Dark, &at(0.0, 1.0), ROOM), None);
        let whole_edge = |ascent: f32| {
            let dark = pose_of(SpacePart::Dark, &at(ascent, 1.0), ROOM).unwrap();
            let top = dark.centre.y - dark.size.y * 0.5;
            assert!(top <= 0.0, "the top of the dark shows at {ascent}: {top}");
            (top + dark.size.y * (DARK_TO_V - SPACE_V[0]) / (SPACE_V[1] - SPACE_V[0]), top + dark.size.y)
        };
        let (_, clear_edge) = whole_edge(0.005);
        assert!(clear_edge < ROOM.y * 0.02, "the dark is in the room before the climb: {clear_edge}");
        let (whole, _) = whole_edge(1.0);
        assert!(whole >= ROOM.y, "the dark is not whole over the room in space: {whole}");
        let (half_way, _) = whole_edge(0.5);
        assert!(half_way < whole, "the dark does not come down as the fight climbs");
    }

    /// A field of stars drifts, and at no time does its edge show in the room.
    #[test]
    fn a_drifting_star_field_always_covers_the_room() {
        for part in [SpacePart::StarsFar, SpacePart::StarsNear, SpacePart::StarsEdge] {
            for step in 0..200 {
                let field = pose_of(part, &at(1.0, step as f32 * 3.7), ROOM).unwrap();
                let (left, right) = (field.centre.x - field.size.x * 0.5, field.centre.x + field.size.x * 0.5);
                assert!(left <= 0.0 && right >= ROOM.x, "{part:?} at step {step}: {left}..{right}");
            }
        }
    }

    /// The planet's top falls toward the bottom of the room and it curves
    /// more, and its limb stays in the room.
    #[test]
    fn the_planet_falls_away_and_curves_more_as_the_fight_climbs() {
        let mut last = (f32::NEG_INFINITY, f32::INFINITY);
        for step in 0..=20 {
            let (centre, radius) = planet_disc(step as f32 / 20.0, ROOM);
            let top = centre.y - radius;
            assert!(top >= last.0 && radius <= last.1, "step {step}: top {top}, radius {radius}");
            assert!(top < ROOM.y, "the limb leaves the room at step {step}");
            last = (top, radius);
        }
        assert!(last.0 > planet_disc(0.0, ROOM).0.y - planet_disc(0.0, ROOM).1);
    }

    /// Each dash is on the moon's lane, and only in space.
    #[test]
    fn each_dash_is_on_the_moons_lane() {
        let [from, to, lane, arc] = moon_lane(ROOM);
        assert!((0..DASHES).all(|k| pose_of(SpacePart::Dash(k), &at(0.5, 3.0), ROOM).is_none()));
        for step in 0..50 {
            let moment = at(1.0, step as f32 * 0.13);
            let mut seen = 0;
            for dash in (0..DASHES).filter_map(|k| pose_of(SpacePart::Dash(k), &moment, ROOM)) {
                let u = (from - dash.centre.x) / (from - to);
                let on_lane = lane - arc * (u * std::f32::consts::PI).sin();
                assert!((dash.centre.y - on_lane).abs() < 1e-3, "{dash:?} is off the lane ({on_lane})");
                seen += 1;
            }
            assert!(seen as f32 >= (from - to) / DASH_PERIOD - 1.0, "only {seen} dashes at step {step}");
        }
    }

    #[test]
    fn an_ease_moves_by_its_step_and_stops_at_its_goal() {
        assert_eq!(ease(0.0, 1.0, 0.25), 0.25);
        assert_eq!(ease(0.9, 1.0, 0.25), 1.0);
        assert_eq!(ease(1.0, 0.0, 0.25), 0.75);
    }
}

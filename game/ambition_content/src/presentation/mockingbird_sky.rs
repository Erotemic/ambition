//! The Mockingbird's anger, and where its fight goes.
//!
//! Angry, it lights itself on fire: flames come off its body, red in its
//! second phase and cold blue in its third. In the third the sky climbs into
//! space: the dark comes down from the top, the stars come out, the planet's
//! limb falls away below, and the moon crosses the room on its lane.
//!
//! All of it reads the conductor's record ([`view_of`]) and decides nothing:
//! the phase, the climb and the moon are the simulation's. A rewind takes the
//! look back with the record. The flames ease between two states on the
//! presentation clock, which is a look only.

use std::collections::HashMap;

use bevy::{
    asset::embedded_asset,
    image::TextureAtlasLayout,
    prelude::*,
    reflect::TypePath,
    render::render_resource::AsBindGroup,
    shader::ShaderRef,
    sprite::Anchor,
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
/// (`-14.5`), below the blocks.
const SPACE_Z: f32 = -14.2;
/// How far the space quad reaches past its room, so no view sees its edge.
const SPACE_PAD: f32 = 2400.0;

pub fn install(app: &mut App) {
    // `embedded_asset!` needs the asset registries.
    if app.world().get_resource::<bevy::asset::io::embedded::EmbeddedAssetRegistry>().is_none() {
        return;
    }
    embedded_asset!(app, "shaders/mockingbird_fire.wgsl");
    embedded_asset!(app, "shaders/mockingbird_space.wgsl");
    // The material plugins install render-world state: they need an app that
    // renders.
    if app.get_sub_app(bevy::render::RenderApp).is_some() {
        app.add_plugins((Material2dPlugin::<FlameMaterial>::default(), Material2dPlugin::<SpaceMaterial>::default()));
    }
    app.add_systems(
        Update,
        (
            (attach_flames, sync_flames, cleanup_flames)
                .chain()
                .in_set(BossOverlaySet)
                .run_if(resource_exists::<Assets<FlameMaterial>>),
            present_space
                .in_set(SessionScopeSet::Presentation)
                .run_if(resource_exists::<Assets<Mesh>>)
                .run_if(resource_exists::<Assets<SpaceMaterial>>),
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

/// All in engine world coordinates (y down). `rect`: the quad, `xy` its
/// corner and `zw` its size. `room`: `xy` the room's size, `z` how far the sky
/// has become space (0 to 1), `w` the time. `moon`: `xy` its centre, `z` its
/// radius, `w` 1 while it is in the room. `lane`: the moon's lane
/// (`moon_lane`). `sky`: `x` the sky's scroll in px/s.
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct SpaceMaterial {
    #[uniform(0)]
    pub rect: Vec4,
    #[uniform(1)]
    pub room: Vec4,
    #[uniform(2)]
    pub moon: Vec4,
    #[uniform(3)]
    pub lane: Vec4,
    #[uniform(4)]
    pub sky: Vec4,
}

impl Material2d for SpaceMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://ambition_content/presentation/shaders/mockingbird_space.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

/// The space quad of a room. Its room is its stamp ([`InRoomInstance`]).
#[derive(Component, Debug, Clone, Copy)]
pub struct SpaceSky;

/// Give each live room whose Mockingbird has taken its fight toward space a
/// space quad, and draw it as far up as the fight is.
pub fn present_space(
    mut commands: Commands,
    time: ambition_time::PresentationTime,
    mut elapsed: Local<f32>,
    rooms: LiveRoomSpecs,
    birds: Query<(&BossConfig, &BodyRecords, Option<&InRoomInstance>), Without<SpaceSky>>,
    mut skies: Query<(&InRoomInstance, &mut Visibility, &MeshMaterial2d<SpaceMaterial>), With<SpaceSky>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<SpaceMaterial>>,
    active_session: Option<Res<ActiveSessionScope>>,
) {
    *elapsed = (*elapsed + time.scaled_dt()).rem_euclid(3600.0);
    let Some(session_scope) = SessionSpawnScope::for_optional_active_session(active_session.as_deref()) else {
        return;
    };
    for (room, definition) in rooms.live_rooms() {
        let view = birds
            .iter()
            .filter(|(config, _, stamp)| {
                config.behavior.id == MOCKINGBIRD_ID && stamp.is_none_or(|stamp| stamp.0 == room)
            })
            .find_map(|(_, records, _)| view_of(records))
            .filter(|view| view.ascent > 0.0 || view.moon.is_some());
        let sky = skies.iter_mut().find(|(stamp, ..)| stamp.0 == room);
        let Some(view) = view else {
            if let Some((_, mut visibility, _)) = sky {
                *visibility = Visibility::Hidden;
            }
            continue;
        };
        let spec = rooms.rooms().spec(definition);
        let world = &spec.world;
        let size = Vec2::new(world.size.x, world.size.y);
        let moon = view.moon.map_or(Vec4::ZERO, |moon| Vec4::new(moon.x, moon.y, MOON_RADIUS, 1.0));
        let scroll = spec.metadata.visual_profile.sky_scroll_px_s.unwrap_or(0) as f32;
        let uniforms = SpaceMaterial {
            rect: Vec4::new(-SPACE_PAD, -SPACE_PAD, size.x + SPACE_PAD * 2.0, size.y + SPACE_PAD * 2.0),
            room: Vec4::new(size.x, size.y, view.ascent, *elapsed),
            moon,
            lane: Vec4::from_array(moon_lane(size)),
            sky: Vec4::new(scroll, 0.0, 0.0, 0.0),
        };
        match sky {
            Some((_, mut visibility, material)) => {
                *visibility = Visibility::Visible;
                if let Some(mut material) = materials.get_mut(&material.0) {
                    *material = uniforms;
                }
            }
            None => {
                commands.spawn_session_scoped(
                    session_scope.in_room(Some(room)),
                    (
                        Mesh2d(meshes.add(Rectangle::default())),
                        MeshMaterial2d(materials.add(uniforms)),
                        Transform::from_translation(world_to_bevy(world, world.size * 0.5, SPACE_Z))
                            .with_scale(Vec3::new(size.x + SPACE_PAD * 2.0, size.y + SPACE_PAD * 2.0, 1.0)),
                        Name::new("Mockingbird: space"),
                        RoomVisual,
                        SpaceSky,
                    ),
                );
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

    #[test]
    fn an_ease_moves_by_its_step_and_stops_at_its_goal() {
        assert_eq!(ease(0.0, 1.0, 0.25), 0.25);
        assert_eq!(ease(0.9, 1.0, 0.25), 1.0);
        assert_eq!(ease(1.0, 0.0, 0.25), 0.75);
    }
}

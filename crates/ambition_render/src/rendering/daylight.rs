//! The light of the time of day: a veil over the world of a sunlit room.
//!
//! One veil for each view covers the world above the foreground and effects
//! and below the UI. It MULTIPLIES what
//! is under it by the light of the hour: white by day, gold in the morning,
//! amber and violet at dusk, moonlit blue at night. A multiply keeps a shadow
//! dark and a lit wall the brightest thing in view; the translucent sprite it
//! replaced mixed every pixel toward one colour, which lifted the blacks and
//! read as fog. Where a 2D material cannot be built (a host with no renderer,
//! Android) the veil is that translucent sprite. It is compositing, not a
//! screen shader, so it is the same with screen shaders off.
//!
//! A room is sunlit when its parallax theme is one the game names in its
//! [`DaylightStyle`]; elsewhere (under the ground, indoors) the veil is clear.
//! The hour comes from [`ambition_time::DayClock`] under the game's
//! [`ambition_time::DayCycle`]. A game with no style has no veil.
//!
//! The town's own lights, at the tiers that draw a light map (High, Ultra:
//! the dark room's `DarknessQuality`): where a lamp the room places
//! (`LightSource`: a lit window, a lamp in a vault) or a thing that casts
//! light ([`CastsLight`]) reaches, the world is multiplied by the lamp's
//! colour instead of the hour's, as much as the lamp reaches there, and
//! never darker than the hour. So a night street is moonlit blue with warm
//! pools round its windows, and by day a lamp changes nothing. The lamps are
//! summed per pixel ([`super::lamps`]); at Ultra their light stops at the
//! room's solid ground. Lower tiers keep the hour's colour.

use std::collections::HashMap;

use bevy::{
    camera::visibility::RenderLayers,
    ecs::system::SystemParam,
    mesh::MeshVertexBufferLayoutRef,
    prelude::*,
    reflect::TypePath,
    render::render_resource::{
        AsBindGroup, BlendComponent, BlendFactor, BlendOperation, BlendState, RenderPipelineDescriptor,
        SpecializedMeshPipelineError,
    },
    shader::ShaderRef,
    sprite_render::{AlphaMode2d, Material2d, Material2dKey, Material2dPlugin},
};

use ambition_persistence::settings::{DarknessQuality, ResolvedVisualQuality};
use ambition_platformer2d_shared_tangle::camera_layers::PARALLAX_BACKGROUND_LAYER;
use ambition_platformer2d_shared_tangle::lifecycle::{LiveRoomInstance, LiveRooms};
use ambition_platformer2d_shared_tangle::gameplay_presentation::CastsLight;
use ambition_platformer2d_world::rooms::{LiveRoomSpecs, RoomSpec};
use ambition_sim_view::PresentedForView;
use ambition_sprite_sheet::game_assets::ParallaxTheme;

use super::camera::coverage::GameplayOutputs;
use super::darkness::{room_lamps, GridKey, Lamp, RoomShadows};
use super::lamps::{pack, reach_image, LampsPlugin, LampsUniform, ReachWrites};
use super::planar_mesh::{install_planar_mesh_2d, PreparedPlanarMesh};
use super::view_isolation::ProjectionRestingLayers;
use super::AnimatedWorldSpriteTint;

/// Over the world and the parallax foreground (`FOREGROUND_PARALLAX_Z`), under the UI.
pub const DAYLIGHT_VEIL_Z: f32 = 60.0;
/// The veil is this wide and high, centred on the camera: more than any view.
const VEIL_SIZE: f32 = 40_000.0;
/// How much more than the view the lamps are gathered over, so a lamp just
/// out of view still lights into it while the camera moves.
const LAMP_MARGIN: f32 = 1.25;
/// The hour's light is bright enough that no lamp shows (each channel of its
/// multiplier at least this): the lamps are not computed.
const DAY_BRIGHT: f32 = 0.97;
/// How bright a lamp's glow is in the darkest hour, and how big, as a part
/// of its radius.
const GLOW_STRENGTH: f32 = 0.55;
const GLOW_SIZE: f32 = 0.22;

/// How a game's day looks: the light at given hours (a colour and its
/// strength: the world is multiplied by white mixed toward the colour by the
/// alpha; between keys it goes smoothly round the clock), and which themes are
/// under the sky.
#[derive(Resource, Clone, Debug)]
pub struct DaylightStyle {
    /// `(hour, colour)` in any order; the veil at an hour between two is
    /// their mix, round midnight too.
    pub keys: Vec<(f32, Color)>,
    pub sunlit: Vec<ParallaxTheme>,
}

impl DaylightStyle {
    /// The light at `hour` (0 to 24).
    pub fn light_at(&self, hour: f32) -> Color {
        let mut keys = self.keys.clone();
        keys.sort_by(|a, b| a.0.total_cmp(&b.0));
        let Some(&first) = keys.first() else {
            return Color::NONE;
        };
        let last = *keys.last().expect("not empty");
        let hour = hour.rem_euclid(24.0);
        // The keys before and after `hour`, round the clock.
        let (before, after) = match keys.iter().position(|(at, _)| *at > hour) {
            Some(0) | None => (last, (first.0 + 24.0, first.1)),
            Some(i) => (keys[i - 1], keys[i]),
        };
        let mut from = before.0;
        let mut at = hour;
        if from > at {
            from -= 24.0;
        }
        let span = (after.0 - from).max(1e-4);
        if at < from {
            at += 24.0;
        }
        let t = ((at - from) / span).clamp(0.0, 1.0);
        let a = before.1.to_linear();
        let b = after.1.to_linear();
        Color::LinearRgba(a.mix(&b, t))
    }
}

/// The one veil.
#[derive(Component, Clone, Copy, Debug)]
pub struct DaylightVeil;

/// What the world is multiplied by under `light` (a key colour and its
/// strength): white mixed toward the colour by its alpha, in linear light.
pub fn multiplier(light: Color) -> LinearRgba {
    let c = light.to_linear();
    let a = c.alpha.clamp(0.0, 1.0);
    LinearRgba::rgb(1.0 - a + a * c.red, 1.0 - a + a * c.green, 1.0 - a + a * c.blue)
}

/// The veil's material: its fragment is the multiplier, and its blend
/// multiplies it into the target (`src * dst`).
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone, PartialEq)]
pub struct DaylightMaterial {
    #[uniform(0)]
    pub light: Vec4,
    /// The lamps in view (none by day, and below High).
    #[uniform(1)]
    pub lamps: LampsUniform,
    /// How far each lamp reaches round it (Ultra; [`super::lamps`]).
    #[texture(2, sample_type = "float", filterable = false)]
    pub reach: Option<Handle<Image>>,
}

const DAYLIGHT_BLEND: BlendState = BlendState {
    color: BlendComponent {
        src_factor: BlendFactor::Dst,
        dst_factor: BlendFactor::Zero,
        operation: BlendOperation::Add,
    },
    alpha: BlendComponent {
        src_factor: BlendFactor::Zero,
        dst_factor: BlendFactor::One,
        operation: BlendOperation::Add,
    },
};

#[cfg(feature = "card_view")]
impl super::card_view::CardMaterialSource for DaylightMaterial {
    fn card_blend() -> Option<BlendState> {
        Some(DAYLIGHT_BLEND)
    }
}

impl Material2d for DaylightMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://ambition_render/rendering/daylight.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }

    fn specialize(
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: Material2dKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        if let Some(fragment) = descriptor.fragment.as_mut() {
            for target in fragment.targets.iter_mut().flatten() {
                target.blend = Some(DAYLIGHT_BLEND);
            }
        }
        Ok(())
    }
}

/// The glow about each lamp at night ([`DaylightMaterial`]'s lamps): its
/// fragment is light, and its blend adds it to the target (`src + dst`).
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone, PartialEq)]
pub struct LampGlowMaterial {
    /// How dark the hour is (no glow by day), and the glow's size.
    #[uniform(0)]
    pub glow: Vec4,
    #[uniform(1)]
    pub lamps: LampsUniform,
}

const LAMP_GLOW_BLEND: BlendState = BlendState {
    color: BlendComponent { src_factor: BlendFactor::One, dst_factor: BlendFactor::One, operation: BlendOperation::Add },
    alpha: BlendComponent { src_factor: BlendFactor::Zero, dst_factor: BlendFactor::One, operation: BlendOperation::Add },
};

#[cfg(feature = "card_view")]
impl super::card_view::CardMaterialSource for LampGlowMaterial {
    fn card_blend() -> Option<BlendState> {
        Some(LAMP_GLOW_BLEND)
    }
}

impl Material2d for LampGlowMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://ambition_render/rendering/lamp_glow.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }

    fn specialize(
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: Material2dKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        if let Some(fragment) = descriptor.fragment.as_mut() {
            for target in fragment.targets.iter_mut().flatten() {
                target.blend = Some(LAMP_GLOW_BLEND);
            }
        }
        Ok(())
    }
}

/// The lamps' glow, over the veil.
#[derive(Component, Clone, Copy, Debug)]
pub struct LampGlow;

/// The shared daylight geometry.
#[derive(Resource)]
struct DaylightMesh(Handle<Mesh>);

/// Installs the multiplying veil's material. Android uses a translucent sprite.
pub struct DaylightPlugin;

impl Plugin for DaylightPlugin {
    fn build(&self, app: &mut App) {
        if app.get_sub_app(bevy::render::RenderApp).is_none() || cfg!(target_os = "android") {
            return;
        }
        bevy::asset::embedded_asset!(app, "daylight.wgsl");
        bevy::asset::embedded_asset!(app, "lamp_glow.wgsl");
        if !app.is_plugin_added::<LampsPlugin>() {
            app.add_plugins(LampsPlugin);
        }
        app.add_plugins((Material2dPlugin::<DaylightMaterial>::default(), Material2dPlugin::<LampGlowMaterial>::default()));
        install_planar_mesh_2d::<DaylightMaterial>(app);
        install_planar_mesh_2d::<LampGlowMaterial>(app);
        let world = app.world_mut();
        if let Some(mut meshes) = world.get_resource_mut::<Assets<Mesh>>() {
            let mesh = meshes.add(Rectangle::new(VEIL_SIZE, VEIL_SIZE));
            world.insert_resource(DaylightMesh(mesh));
        }
    }
}

#[derive(PartialEq)]
struct LampSample {
    bounds: Rect,
    lamps: Vec<Lamp>,
    room: LiveRoomInstance,
    shadows: Option<GridKey>,
}

struct DaylightShading {
    veil: Handle<DaylightMaterial>,
    glow: Handle<LampGlowMaterial>,
    reach: Handle<Image>,
    glow_entity: Entity,
    last: Option<LampSample>,
    lamps: LampsUniform,
    solid: Option<RoomShadows>,
}

struct DaylightForView {
    veil: Entity,
    shading: Option<DaylightShading>,
}

/// Retained daylight for the local views.
#[derive(Default)]
pub struct ViewDaylight(HashMap<Entity, DaylightForView>);

/// Assets that realize daylight with a shader.
#[derive(SystemParam)]
pub struct DaylightAssets<'w> {
    mesh: Option<Res<'w, DaylightMesh>>,
    images: Option<ResMut<'w, Assets<Image>>>,
    veils: Option<ResMut<'w, Assets<DaylightMaterial>>>,
    glows: Option<ResMut<'w, Assets<LampGlowMaterial>>>,
    writes: Option<ResMut<'w, ReachWrites>>,
}

impl DaylightAssets<'_> {
    fn spawn(
        &mut self, commands: &mut Commands, view: Entity, center: Vec2, tint: Color,
    ) -> DaylightForView {
        let mut veil = commands.spawn((
            DaylightVeil,
            PresentedForView(view),
            daylight_layers(),
            Transform::from_translation(center.extend(DAYLIGHT_VEIL_Z)),
            Name::new("Daylight veil"),
        ));
        let veil_entity = veil.id();
        let (Some(mesh), Some(images), Some(veils), Some(glows)) = (
            self.mesh.as_deref(), self.images.as_deref_mut(),
            self.veils.as_deref_mut(), self.glows.as_deref_mut(),
        ) else {
            veil.insert((Sprite::from_color(tint, Vec2::splat(VEIL_SIZE)), AnimatedWorldSpriteTint));
            return DaylightForView { veil: veil_entity, shading: None };
        };

        let reach = images.add(reach_image());
        let material = veils.add(DaylightMaterial {
            light: Vec4::ONE, lamps: LampsUniform::default(), reach: Some(reach.clone()),
        });
        let glow = glows.add(LampGlowMaterial { glow: Vec4::ZERO, lamps: LampsUniform::default() });
        veil.insert(PreparedPlanarMesh { mesh: mesh.0.clone(), material: material.clone() });
        let glow_entity = commands.spawn((
            LampGlow,
            ChildOf(veil_entity),
            daylight_layers(),
            PreparedPlanarMesh { mesh: mesh.0.clone(), material: glow.clone() },
            Transform::from_xyz(0.0, 0.0, 0.25),
            Visibility::Hidden,
            Name::new("Lamp glow"),
        )).id();
        DaylightForView {
            veil: veil_entity,
            shading: Some(DaylightShading {
                veil: material, glow, reach, glow_entity, last: None,
                lamps: LampsUniform::default(), solid: None,
            }),
        }
    }

    fn write(&mut self, shading: &DaylightShading, light: Vec4, glow: Vec4, lamps: LampsUniform) {
        if let Some(veils) = self.veils.as_deref_mut() {
            if veils.get(&shading.veil).is_some_and(|m| m.light != light || m.lamps != lamps) {
                if let Some(mut material) = veils.get_mut(&shading.veil) {
                    material.light = light;
                    material.lamps = lamps.clone();
                }
            }
        }
        if let Some(glows) = self.glows.as_deref_mut() {
            if glows.get(&shading.glow).is_some_and(|m| m.glow != glow || m.lamps != lamps) {
                if let Some(mut material) = glows.get_mut(&shading.glow) {
                    material.glow = glow;
                    material.lamps = lamps;
                }
            }
        }
    }
}

// A portal capture has its own lighting path. The veil lights its window
// as part of the main view.
fn daylight_layers() -> (RenderLayers, ProjectionRestingLayers) {
    let layers = RenderLayers::layer(PARALLAX_BACKGROUND_LAYER);
    (layers.clone(), ProjectionRestingLayers(layers))
}

impl DaylightShading {
    fn sample_lamps(
        &mut self,
        room: LiveRoomInstance,
        spec: &RoomSpec,
        bounds: Rect,
        shadowed: bool,
        moving: impl Iterator<Item = Lamp>,
        writes: Option<&mut ReachWrites>,
    ) {
        if shadowed {
            RoomShadows::keep_for(&mut self.solid, spec);
        }
        let shadows = self.solid.as_ref().filter(|_| shadowed);
        let mut lamps = room_lamps(spec);
        let room_count = lamps.len();
        lamps.extend(moving);
        let sample = LampSample { bounds, lamps, room, shadows: shadows.map(|grid| grid.key().clone()) };
        if self.last.as_ref() == Some(&sample) {
            return;
        }
        let reaches = shadows.map(|grid| grid.reaches(&sample.lamps[room_count..]));
        let (lamps, rows) = pack(&sample.lamps, reaches.as_deref(), bounds.center(), bounds.size() * LAMP_MARGIN);
        if let (Some(rows), Some(writes)) = (rows, writes) {
            writes.0.insert(self.reach.id(), rows);
        }
        self.last = Some(sample);
        self.lamps = lamps;
    }
}

/// Light each observer's room once for its gameplay outputs.
#[allow(clippy::too_many_arguments)]
pub fn light_the_world(
    mut commands: Commands,
    style: Option<Res<DaylightStyle>>,
    clock: Option<Res<ambition_time::DayClock>>,
    cycle: Option<Res<ambition_time::DayCycle>>,
    rooms: Option<LiveRoomSpecs>,
    outputs: GameplayOutputs,
    quality: Option<Res<ResolvedVisualQuality>>,
    mut assets: DaylightAssets,
    casters: Query<(Entity, &GlobalTransform, &CastsLight, &InheritedVisibility)>,
    live: LiveRooms,
    mut veils: Query<(&mut Transform, &mut Visibility, Option<&mut Sprite>), (With<DaylightVeil>, Without<LampGlow>, Without<Camera2d>)>,
    mut glows: Query<&mut Visibility, (With<LampGlow>, Without<DaylightVeil>)>,
    mut retained: Local<ViewDaylight>,
) {
    let active = style.as_deref().zip(clock.as_deref()).zip(cycle.as_deref())
        .filter(|((_, clock), _)| clock.day > 0);
    let views = if active.is_some() && rooms.is_some() { outputs.view_bounds() } else { Default::default() };
    retained.0.retain(|view, slot| {
        let keep = views.contains_key(view) && veils.contains(slot.veil)
            && slot.shading.as_ref().is_none_or(|s| glows.contains(s.glow_entity));
        if !keep {
            commands.entity(slot.veil).try_despawn();
        }
        keep
    });
    let (Some(((style, clock), cycle)), Some(rooms)) = (active, rooms) else {
        return;
    };
    let quality = quality.map_or(DarknessQuality::default(), |q| q.budget.glow.darkness);
    let lamps_drawn = matches!(quality, DarknessQuality::LightMap | DarknessQuality::ShadowedLightMap);
    let hour_tint = style.light_at(clock.hour(cycle));

    for (view, (frame, bounds)) in views {
        let sunlit = rooms.definition_in(frame.room).map(|definition| rooms.rooms().spec(definition))
            .filter(|spec| ParallaxTheme::named_by_room_metadata(&spec.metadata)
                .is_some_and(|theme| style.sunlit.contains(&theme)));
        let tint = sunlit.map_or(Color::NONE, |_| hour_tint);
        let light = multiplier(tint);
        let light = Vec4::new(light.red, light.green, light.blue, 1.0);
        let shown = if assets.mesh.is_some() { light != Vec4::ONE } else { tint.alpha() > 0.0 };
        if !shown {
            if let Some(slot) = retained.0.get(&view) {
                if let Ok((_, mut visibility, _)) = veils.get_mut(slot.veil) {
                    visibility.set_if_neq(Visibility::Hidden);
                }
            }
            continue;
        }
        let slot = retained.0.entry(view)
            .or_insert_with(|| assets.spawn(&mut commands, view, frame.plane_center, tint));
        if let Ok((mut transform, mut visibility, sprite)) = veils.get_mut(slot.veil) {
            let at = frame.plane_center.extend(DAYLIGHT_VEIL_Z);
            if transform.translation != at {
                transform.translation = at;
            }
            visibility.set_if_neq(Visibility::Inherited);
            if let Some(mut sprite) = sprite {
                if sprite.color != tint {
                    sprite.color = tint;
                }
            }
        }
        if let Some(shading) = &mut slot.shading {
            let mut lamps = LampsUniform::default();
            if let Some(spec) = sunlit.filter(|_| lamps_drawn && light.truncate().min_element() < DAY_BRIGHT) {
                if rooms.set_is_changed() {
                    shading.solid = None;
                    shading.last = None;
                }
                let moving = casters.iter().filter(|(entity, _, _, shown)| shown.get() && live.of(*entity) == Some(frame.room))
                    .map(|(_, at, cast, _)| {
                        let c = Color::srgb(cast.color[0], cast.color[1], cast.color[2]).to_linear();
                        Lamp { at: at.translation().truncate(), radius: cast.radius, intensity: cast.intensity, color: [c.red, c.green, c.blue] }
                    });
                shading.sample_lamps(frame.room, spec, bounds, quality == DarknessQuality::ShadowedLightMap,
                    moving, assets.writes.as_deref_mut());
                lamps = shading.lamps.clone();
            }
            let night = (1.0 - (light.x + light.y + light.z) / 3.0).clamp(0.0, 1.0);
            let strength = if lamps.count > 0 { GLOW_STRENGTH * night } else { 0.0 };
            if let Ok(mut visibility) = glows.get_mut(shading.glow_entity) {
                visibility.set_if_neq(if strength > 0.0 { Visibility::Inherited } else { Visibility::Hidden });
            } else if strength > 0.0 {
                commands.entity(shading.glow_entity).insert(Visibility::Inherited);
            }
            assets.write(shading, light, Vec4::new(strength, GLOW_SIZE, 0.0, 0.0), lamps);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn style() -> DaylightStyle {
        DaylightStyle {
            keys: vec![
                (12.0, Color::srgba(0.0, 0.0, 0.0, 0.0)),
                (0.0, Color::srgba(0.0, 0.0, 0.5, 0.5)),
            ],
            sunlit: vec![ParallaxTheme::new("math_town")],
        }
    }

    #[test]
    fn the_light_is_each_key_at_its_hour_and_a_mix_between_round_the_clock() {
        let s = style();
        assert!((s.light_at(12.0).alpha() - 0.0).abs() < 1e-4);
        assert!((s.light_at(0.0).alpha() - 0.5).abs() < 1e-4);
        assert!((s.light_at(24.0).alpha() - 0.5).abs() < 1e-4);
        assert!((s.light_at(6.0).alpha() - 0.25).abs() < 1e-3, "half way from midnight to noon");
        assert!((s.light_at(18.0).alpha() - 0.25).abs() < 1e-3, "half way from noon to midnight");
    }

    #[test]
    fn each_view_draws_only_the_daylight_and_lamps_of_its_room() {
        use ambition_persistence::settings::{VisualQualityBudget, VisualQualityProfile};
        use ambition_platformer2d_core as ae;
        use ambition_platformer2d_shared_tangle::camera_layers::MainCamera;
        use ambition_platformer2d_shared_tangle::lifecycle::{spawn_live_room, InRoomInstance};
        use ambition_platformer2d_world::rooms::{insert_room_set, LiveRoomDefinition, RoomSet};
        use ambition_sim_view::{LocalView, LocalViewId, PresentsView};
        use ambition_time::{DayClock, DayCycle};
        use bevy::camera::RenderTargetInfo;
        use crate::rendering::{PresentedViewFrame, PresentedViewFrames};

        let mut app = App::new();
        let cycle = DayCycle {
            day_length_s: 24.0, start_hour: 0.0, morning_at: 6.0, day_at: 9.0,
            evening_at: 18.0, night_at: 21.0,
        };
        let profile = VisualQualityProfile::High;
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(std::time::Duration::ZERO))
            .insert_resource(cycle)
            .insert_resource(DayClock::at_hour(&cycle, 0.0))
            .insert_resource(style())
            .insert_resource(ResolvedVisualQuality { profile, budget: VisualQualityBudget::for_profile(profile) })
            .insert_resource(DaylightMesh(Handle::default()))
            .init_resource::<Assets<Image>>()
            .init_resource::<Assets<DaylightMaterial>>()
            .init_resource::<Assets<LampGlowMaterial>>()
            .add_systems(Update, (light_the_world, super::super::view_isolation::isolate_per_view_projections).chain());

        let world = app.world_mut();
        let room = |id| RoomSpec::new(id, ae::World::new(id, ae::Vec2::splat(1000.0), ae::Vec2::ZERO, Vec::new()));
        let mut street = room("street");
        street.metadata.visual_profile.parallax_theme = Some("math_town".into());
        insert_room_set(world, RoomSet::from_parts_or_panic("inside", vec![room("inside"), street], Vec::new()));
        let inside = LiveRoomInstance::ACTIVATION;
        let street = inside.next();
        spawn_live_room(world, street, LiveRoomDefinition::from_index(1));

        let [first, second] = [LocalViewId::FIRST, LocalViewId(1)].map(|id| {
            let view = world.spawn((LocalView, id)).id();
            let mut camera = Camera::default();
            camera.computed.target_info = Some(RenderTargetInfo { physical_size: UVec2::new(640, 360), scale_factor: 1.0 });
            let output = world.spawn((Camera2d, MainCamera, camera, PresentsView(view), RenderLayers::layer(PARALLAX_BACKGROUND_LAYER))).id();
            (view, output)
        });
        let frame = |room, x| PresentedViewFrame { room, plane_center: Vec2::new(x, 0.0), roll_radians: 0.0, orthographic_scale: 1.0 };
        world.insert_resource(PresentedViewFrames::of_views(vec![(first.0, frame(inside, 0.0)), (second.0, frame(street, 48.0))]));
        for room in [inside, street] {
            world.spawn((InRoomInstance(room), GlobalTransform::IDENTITY, InheritedVisibility::VISIBLE,
                CastsLight { radius: 90.0, intensity: 1.0, color: [1.0, 0.5, 0.3] }));
        }

        app.update();
        let world = app.world_mut();
        let (entity, view, layers, at, source) = world.query_filtered::<(
            Entity, &PresentedForView, &RenderLayers, &Transform, &PreparedPlanarMesh<DaylightMaterial>,
        ), With<DaylightVeil>>().single(world).unwrap();
        assert_eq!(view.0, second.0);
        assert_eq!(at.translation.x, 48.0);
        assert!(layers.intersects(world.get::<RenderLayers>(second.1).unwrap()));
        assert!(!layers.intersects(world.get::<RenderLayers>(first.1).unwrap()));
        let material = world.resource::<Assets<DaylightMaterial>>().get(&source.material).unwrap();
        assert_eq!(material.lamps.count, 1, "only the lamp in the observed room is visible");

        app.insert_resource(DayClock::at_hour(&cycle, 12.0));
        app.update();
        assert_eq!(app.world().get::<Visibility>(entity), Some(&Visibility::Hidden));
        app.world_mut().despawn(second.0);
        app.update();
        assert!(app.world().get_entity(entity).is_err());
    }
}

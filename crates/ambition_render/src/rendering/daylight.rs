//! The light of the time of day: a veil over the world of a sunlit room.
//!
//! One rectangle follows the gameplay camera above the world (the play, the
//! parallax foreground and the effects) and under the UI. It MULTIPLIES what
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

use bevy::{
    mesh::MeshVertexBufferLayoutRef,
    prelude::*,
    reflect::TypePath,
    render::render_resource::{
        AsBindGroup, BlendComponent, BlendFactor, BlendOperation, BlendState, RenderPipelineDescriptor,
        SpecializedMeshPipelineError,
    },
    shader::ShaderRef,
    sprite_render::{AlphaMode2d, Material2d, Material2dKey, Material2dPlugin, MeshMaterial2d},
};

use ambition_persistence::settings::{DarknessQuality, ResolvedVisualQuality};
use ambition_platformer2d_shared_tangle::camera_layers::MainCamera;
use ambition_platformer2d_shared_tangle::body::PrimaryLiveRoom;
use ambition_platformer2d_shared_tangle::gameplay_presentation::CastsLight;
use ambition_platformer2d_world::rooms::LiveRoomSpecs;
use ambition_sprite_sheet::game_assets::ParallaxTheme;

use super::darkness::{room_lamps, GridKey, Lamp, RoomShadows};
use super::lamps::{pack, reach_image, LampsPlugin, LampsUniform, ReachWrites};

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
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
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
        let multiply = BlendState {
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
        if let Some(fragment) = descriptor.fragment.as_mut() {
            for target in fragment.targets.iter_mut().flatten() {
                target.blend = Some(multiply);
            }
        }
        Ok(())
    }
}

/// The glow about each lamp at night ([`DaylightMaterial`]'s lamps): its
/// fragment is light, and its blend adds it to the target (`src + dst`).
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct LampGlowMaterial {
    /// How dark the hour is (no glow by day), and the glow's size.
    #[uniform(0)]
    pub glow: Vec4,
    #[uniform(1)]
    pub lamps: LampsUniform,
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
        let add = BlendState {
            color: BlendComponent { src_factor: BlendFactor::One, dst_factor: BlendFactor::One, operation: BlendOperation::Add },
            alpha: BlendComponent { src_factor: BlendFactor::Zero, dst_factor: BlendFactor::One, operation: BlendOperation::Add },
        };
        if let Some(fragment) = descriptor.fragment.as_mut() {
            for target in fragment.targets.iter_mut().flatten() {
                target.blend = Some(add);
            }
        }
        Ok(())
    }
}

/// The lamps' glow, over the veil.
#[derive(Component, Clone, Copy, Debug)]
pub struct LampGlow;

/// The veil's mesh and material, where a 2D material can be built, the
/// lamps' reach texture and their glow's material.
#[derive(Resource, Clone)]
pub struct DaylightVeilAssets {
    mesh: Handle<Mesh>,
    material: Handle<DaylightMaterial>,
    reach: Handle<Image>,
    glow: Handle<LampGlowMaterial>,
}

/// Installs the multiplying veil's material. Without it (no renderer, Android)
/// the veil is a translucent sprite.
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
        let world = app.world_mut();
        let Some(reach) = world.get_resource_mut::<Assets<Image>>().map(|mut images| images.add(reach_image())) else {
            return;
        };
        let Some(material) = world.get_resource_mut::<Assets<DaylightMaterial>>().map(|mut materials| {
            materials.add(DaylightMaterial { light: Vec4::ONE, lamps: LampsUniform::default(), reach: Some(reach.clone()) })
        }) else {
            return;
        };
        let Some(mesh) = world
            .get_resource_mut::<Assets<Mesh>>()
            .map(|mut meshes| meshes.add(Rectangle::new(VEIL_SIZE, VEIL_SIZE)))
        else {
            return;
        };
        let Some(glow) = world
            .get_resource_mut::<Assets<LampGlowMaterial>>()
            .map(|mut materials| materials.add(LampGlowMaterial { glow: Vec4::ZERO, lamps: LampsUniform::default() }))
        else {
            return;
        };
        app.insert_resource(DaylightVeilAssets { mesh, material, reach, glow });
    }
}

/// Put the veil over the camera's view in the light of the hour.
#[allow(clippy::too_many_arguments)]
pub fn light_the_world(
    mut commands: Commands,
    style: Option<Res<DaylightStyle>>,
    clock: Option<Res<ambition_time::DayClock>>,
    cycle: Option<Res<ambition_time::DayCycle>>,
    rooms: Option<LiveRoomSpecs>,
    primary: Option<PrimaryLiveRoom>,
    cameras: Query<(&Transform, Option<&Projection>), (With<MainCamera>, Without<DaylightVeil>)>,
    mut veils: Query<(&mut Transform, Option<&mut Sprite>), (With<DaylightVeil>, Without<LampGlow>)>,
    veil_assets: Option<Res<DaylightVeilAssets>>,
    (mut materials, mut glow_materials, mut glows): (
        Option<ResMut<Assets<DaylightMaterial>>>,
        Option<ResMut<Assets<LampGlowMaterial>>>,
        Query<&mut Transform, (With<LampGlow>, Without<DaylightVeil>, Without<MainCamera>)>,
    ),
    (mut writes, quality): (Option<ResMut<ReachWrites>>, Option<Res<ResolvedVisualQuality>>),
    casters: Query<(&GlobalTransform, &CastsLight, &InheritedVisibility)>,
    // What the lamps were last gathered from, and what they gave: a still
    // frame gathers nothing.
    (mut drawn, mut last): (Local<Option<(Vec2, Vec2, Vec<Lamp>, Option<GridKey>)>>, Local<LampsUniform>),
    // The sunlit room's solid ground, at Ultra.
    mut solid: Local<Option<RoomShadows>>,
) {
    let Some(style) = style else {
        return;
    };
    let quality = quality.map_or(DarknessQuality::default(), |q| q.budget.glow.darkness);
    let lamps_drawn = matches!(quality, DarknessQuality::LightMap | DarknessQuality::ShadowedLightMap);
    let mut room_lights: Option<Vec<Lamp>> = None;
    let target = match (clock.as_deref(), cycle.as_deref(), rooms, primary) {
        (Some(clock), Some(cycle), Some(rooms), Some(primary)) if clock.day > 0 => {
            let spec = rooms.definition_named(primary.get()).map(|definition| rooms.rooms().spec(definition));
            // The place is sunlit, not its background: the room's own theme.
            let sunlit = spec.filter(|spec| {
                ParallaxTheme::named_by_room_metadata(&spec.metadata).is_some_and(|theme| style.sunlit.contains(&theme))
            });
            match sunlit {
                Some(spec) => {
                    if lamps_drawn {
                        if quality == DarknessQuality::ShadowedLightMap {
                            RoomShadows::keep_for(&mut solid, spec);
                        }
                        room_lights = Some(room_lamps(spec));
                    }
                    style.light_at(clock.hour(cycle))
                }
                None => Color::NONE,
            }
        }
        _ => Color::NONE,
    };
    let Some((camera, projection)) = cameras.iter().next() else {
        return;
    };
    let light = multiplier(target);
    // The lamps in view, when the hour is dark enough to show them.
    let mut lamps = LampsUniform::default();
    if let (Some(room_lights), Some(assets), true) =
        (room_lights, veil_assets.as_deref(), light.red.min(light.green).min(light.blue) < DAY_BRIGHT)
    {
        let view = match projection {
            Some(Projection::Orthographic(o)) => o.area.size(),
            _ => Vec2::new(1280.0, 720.0),
        };
        let (center, size) = (camera.translation.truncate(), view * LAMP_MARGIN);
        let room_count = room_lights.len();
        let mut all = room_lights;
        // A thing that casts light names its colour in sRGB; the room's lamps
        // and the veil are linear.
        all.extend(casters.iter().filter(|(_, _, shown)| shown.get()).map(|(at, cast, _)| {
            let c = Color::srgb(cast.color[0], cast.color[1], cast.color[2]).to_linear();
            Lamp { at: at.translation().truncate(), radius: cast.radius, intensity: cast.intensity, color: [c.red, c.green, c.blue] }
        }));
        let shadows = solid.as_ref().filter(|_| quality == DarknessQuality::ShadowedLightMap);
        let inputs = (center, size, all, shadows.map(|shadows| shadows.key().clone()));
        if drawn.as_ref() != Some(&inputs) {
            let reaches = shadows.map(|shadows| shadows.reaches(&inputs.2[room_count..]));
            let (packed, rows) = pack(&inputs.2, reaches.as_deref(), center, size);
            if let (Some(rows), Some(writes)) = (rows, writes.as_deref_mut()) {
                writes.0.insert(assets.reach.id(), rows);
            }
            *last = packed;
            *drawn = Some(inputs);
        }
        lamps = last.clone();
    }
    let at = camera.translation.truncate().extend(DAYLIGHT_VEIL_Z);
    // Not on the world layer: a portal capture draws that layer, and its
    // window, which the veil covers too, was lit two times. The main cameras
    // draw this layer and no capture does.
    let layer = bevy::camera::visibility::RenderLayers::layer(
        ambition_platformer2d_shared_tangle::camera_layers::PARALLAX_BACKGROUND_LAYER,
    );
    // The hour moves slowly, so the light needs no easing of its own; a change
    // of room is a cut behind the room transition's cover.
    if let (Some(assets), Some(materials)) = (veil_assets.as_deref(), materials.as_deref_mut()) {
        let light = Vec4::new(light.red, light.green, light.blue, 1.0);
        // The glow is as strong as the hour is dark.
        let night = (1.0 - (light.x + light.y + light.z) / 3.0).clamp(0.0, 1.0);
        let glow = Vec4::new(if lamps.count > 0 { GLOW_STRENGTH * night } else { 0.0 }, GLOW_SIZE, 0.0, 0.0);
        if let Some(glow_materials) = glow_materials.as_deref_mut() {
            let changed = |material: &LampGlowMaterial| material.glow != glow || material.lamps != lamps;
            if glow_materials.get(&assets.glow).is_some_and(changed) {
                if let Some(mut material) = glow_materials.get_mut(&assets.glow) {
                    material.glow = glow;
                    material.lamps = lamps.clone();
                }
            }
        }
        let changed = |material: &DaylightMaterial| material.light != light || material.lamps != lamps;
        if materials.get(&assets.material).is_some_and(changed) {
            if let Some(mut material) = materials.get_mut(&assets.material) {
                material.light = light;
                material.lamps = lamps;
            }
        }
    }
    // Over the veil, so the veil does not darken it; under the dark room's.
    let glow_at = camera.translation.truncate().extend(DAYLIGHT_VEIL_Z + 0.25);
    match (glows.single_mut(), veil_assets.as_deref()) {
        (Ok(mut transform), _) => transform.translation = glow_at,
        (Err(_), Some(assets)) => {
            commands.spawn((
                LampGlow,
                Mesh2d(assets.mesh.clone()),
                MeshMaterial2d(assets.glow.clone()),
                Transform::from_translation(glow_at),
                bevy::camera::visibility::RenderLayers::layer(
                    ambition_platformer2d_shared_tangle::camera_layers::PARALLAX_BACKGROUND_LAYER,
                ),
                Name::new("Lamp glow"),
            ));
        }
        (Err(_), None) => {}
    }
    let Ok((mut transform, sprite)) = veils.single_mut() else {
        let veil = commands.spawn((DaylightVeil, Transform::from_translation(at), layer, Name::new("Daylight veil"))).id();
        match veil_assets.as_deref() {
            Some(assets) => {
                commands.entity(veil).insert((Mesh2d(assets.mesh.clone()), MeshMaterial2d(assets.material.clone())));
            }
            None => {
                commands
                    .entity(veil)
                    .insert(Sprite { color: target, custom_size: Some(Vec2::splat(VEIL_SIZE)), ..Default::default() });
            }
        }
        return;
    };
    transform.translation = at;
    if let Some(mut sprite) = sprite {
        sprite.color = target;
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
}

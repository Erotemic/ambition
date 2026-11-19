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

use ambition_platformer2d_shared_tangle::camera_layers::MainCamera;
use ambition_platformer2d_shared_tangle::body::PrimaryLiveRoom;
use ambition_platformer2d_world::rooms::LiveRoomSpecs;
use ambition_sprite_sheet::game_assets::ParallaxTheme;

/// Over the world and the parallax foreground (`FOREGROUND_PARALLAX_Z`), under the UI.
pub const DAYLIGHT_VEIL_Z: f32 = 60.0;
/// The veil is this wide and high, centred on the camera: more than any view.
const VEIL_SIZE: f32 = 40_000.0;

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

/// The veil's mesh and material, where a 2D material can be built.
#[derive(Resource, Clone)]
pub struct DaylightVeilAssets {
    mesh: Handle<Mesh>,
    material: Handle<DaylightMaterial>,
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
        app.add_plugins(Material2dPlugin::<DaylightMaterial>::default());
        let world = app.world_mut();
        let Some(material) = world
            .get_resource_mut::<Assets<DaylightMaterial>>()
            .map(|mut materials| materials.add(DaylightMaterial { light: Vec4::ONE }))
        else {
            return;
        };
        let Some(mesh) = world
            .get_resource_mut::<Assets<Mesh>>()
            .map(|mut meshes| meshes.add(Rectangle::new(VEIL_SIZE, VEIL_SIZE)))
        else {
            return;
        };
        app.insert_resource(DaylightVeilAssets { mesh, material });
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
    cameras: Query<&Transform, (With<MainCamera>, Without<DaylightVeil>)>,
    mut veils: Query<(&mut Transform, Option<&mut Sprite>), With<DaylightVeil>>,
    veil_assets: Option<Res<DaylightVeilAssets>>,
    mut materials: Option<ResMut<Assets<DaylightMaterial>>>,
) {
    let Some(style) = style else {
        return;
    };
    let target = match (clock.as_deref(), cycle.as_deref(), rooms, primary) {
        (Some(clock), Some(cycle), Some(rooms), Some(primary)) if clock.day > 0 => {
            let sunlit = rooms.definition_named(primary.get()).is_some_and(|definition| {
                // The place is sunlit, not its background: the room's own theme.
                ParallaxTheme::named_by_room_metadata(&rooms.rooms().spec(definition).metadata)
                    .is_some_and(|theme| style.sunlit.contains(&theme))
            });
            if sunlit { style.light_at(clock.hour(cycle)) } else { Color::NONE }
        }
        _ => Color::NONE,
    };
    let Some(camera) = cameras.iter().next() else {
        return;
    };
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
        let light = multiplier(target);
        let light = Vec4::new(light.red, light.green, light.blue, 1.0);
        if materials.get(&assets.material).is_some_and(|material| material.light != light) {
            if let Some(mut material) = materials.get_mut(&assets.material) {
                material.light = light;
            }
        }
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

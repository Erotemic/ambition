//! The light about Eternal Cosmic Mary-O.
//!
//! Presentation only, and content-owned. She is a sprite, and she is whole
//! with no shader. This adds light behind her while she is in the form: one
//! quad, a child of her visual, so it goes where she goes and leaves when
//! she leaves the form.
//!
//! ## The art is a picture, and the shader only moves it
//!
//! The stars in the light are the shapes of the starlight shimmer picture
//! (`room_dressing/starlight_shimmer.png`, `terrain/shimmer.py` of the sprite
//! renderer): the picture the starlight rooms have in their air. The shader
//! (`shaders/cosmic_aura.wgsl`) makes each one come and go in a soft round
//! glow. No picture is added for it.
//!
//! ## She looks right with no light
//!
//! A device with no budget for shaders draws none ([`aura_is_in_budget`]),
//! and a headless composition installs none.

use bevy::{
    image::{ImageAddressMode, ImageFilterMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor},
    prelude::*,
    reflect::TypePath,
    render::render_resource::AsBindGroup,
    shader::ShaderRef,
    sprite::Anchor,
    sprite_render::{AlphaMode2d, Material2d, Material2dPlugin, MeshMaterial2d},
};

use ambition_platformer2d::characters::actor::WornCharacter;
use ambition_platformer2d::persistence::settings::ResolvedVisualQuality;
use ambition_platformer2d::render::rendering::PlayerVisual;

/// The side of the quad, as a part of the height of her frame.
const AURA_SIZE: f32 = 1.6;
/// The middle of her body is this far under the middle of her frame, as a
/// part of the height of the frame (the stage of `mary_o_v2_cosmic` in the
/// sprite renderer: her body is in the low part of it, under her wings).
const BODY_UNDER_FRAME_MIDDLE: f32 = 13.67 / 84.0;
/// Behind her, by less than the depth between two layers of a room.
const AURA_Z: f32 = -0.06;
/// The size of one star picture, in world px.
const STAR_TILE: f32 = 190.0;

/// The material of the light. Its fields are the uniforms of `cosmic_aura.wgsl`.
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct MaryOCosmicAuraMaterial {
    #[uniform(0)]
    pub inner: Vec4,
    #[uniform(1)]
    pub outer: Vec4,
    #[uniform(2)]
    pub star: Vec4,
    #[texture(3)]
    #[sampler(4)]
    pub stars: Handle<Image>,
}

impl Material2d for MaryOCosmicAuraMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://ambition_demo_mary_o/shaders/cosmic_aura.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

#[derive(Resource, Default)]
struct CosmicAuraInstalled;

/// The unit quad the light scales, and its one material.
#[derive(Resource)]
struct CosmicAuraAssets {
    quad: Handle<Mesh>,
    material: Option<Handle<MaryOCosmicAuraMaterial>>,
}

/// On her visual while it has its light: the quad.
#[derive(Component)]
struct HasCosmicAura(Entity);

/// Whether this device draws the light: it is a shader.
fn aura_is_in_budget(quality: Option<Res<ResolvedVisualQuality>>) -> bool {
    quality.is_none_or(|quality| quality.budget.shaders.draws_screen_shaders())
}

/// Install the material and the system that gives her the light.
/// Idempotent. An app that does not draw installs nothing.
pub fn install(app: &mut App) {
    if app.world().contains_resource::<CosmicAuraInstalled>() {
        return;
    }
    // `content_asset!` needs the asset registries.
    if app
        .world()
        .get_resource::<bevy::asset::io::embedded::EmbeddedAssetRegistry>()
        .is_none()
    {
        return;
    }
    app.insert_resource(CosmicAuraInstalled);
    ambition_platformer2d::content_asset!(app, "src/shaders/cosmic_aura.wgsl");
    // The material plugin installs render-world state, so it needs an app
    // that renders. An asset registry is not evidence of a renderer.
    if app.get_sub_app(bevy::render::RenderApp).is_some() {
        app.add_plugins(Material2dPlugin::<MaryOCosmicAuraMaterial>::default());
    }
    // The quad is made now and not with the first light: Bevy does not try
    // again to draw a quad whose mesh came late.
    if let Some(mut meshes) = app.world_mut().get_resource_mut::<Assets<Mesh>>() {
        let quad = meshes.add(Rectangle::new(1.0, 1.0));
        app.insert_resource(CosmicAuraAssets { quad, material: None });
    }
    app.add_systems(
        Update,
        light_her_while_she_is_cosmic
            .in_set(crate::MaryOPresentationSet)
            .run_if(resource_exists::<CosmicAuraAssets>)
            .run_if(resource_exists::<Assets<MaryOCosmicAuraMaterial>>),
    );
}

/// The starlight shimmer picture. It repeats, so its sampler repeats, and it
/// is data, so it is not read as sRGB.
fn star_picture(asset_server: &AssetServer) -> Handle<Image> {
    asset_server
        .load_builder()
        .with_settings(|settings: &mut ImageLoaderSettings| {
            settings.is_srgb = false;
            settings.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                address_mode_u: ImageAddressMode::Repeat,
                address_mode_v: ImageAddressMode::Repeat,
                mag_filter: ImageFilterMode::Linear,
                min_filter: ImageFilterMode::Linear,
                ..default()
            });
        })
        .load("room_dressing/starlight_shimmer.png")
}

/// Where the quad is and how large it is, from her sprite: about the middle
/// of her body, and a little larger than her frame is high.
fn aura_transform(sprite: &Sprite, anchor: Option<&Anchor>) -> Option<Transform> {
    let size = sprite.custom_size?;
    // The middle of her frame, from the place her anchor puts at her origin.
    let frame_middle = -anchor.map_or(Vec2::ZERO, |anchor| anchor.0) * size;
    let middle = frame_middle - Vec2::new(0.0, BODY_UNDER_FRAME_MIDDLE * size.y);
    let side = size.y * AURA_SIZE;
    Some(Transform::from_xyz(middle.x, middle.y, AURA_Z).with_scale(Vec3::new(side, side, 1.0)))
}

/// Give her the light while she is in the form and the device has a budget
/// for it, keep it about her body, and take it away after.
#[allow(clippy::type_complexity)]
fn light_her_while_she_is_cosmic(
    mut commands: Commands,
    mut assets: ResMut<CosmicAuraAssets>,
    mut materials: ResMut<Assets<MaryOCosmicAuraMaterial>>,
    asset_server: Res<AssetServer>,
    quality: Option<Res<ResolvedVisualQuality>>,
    bodies: Query<(Entity, &WornCharacter, &Sprite, Option<&Anchor>, Option<&HasCosmicAura>), With<PlayerVisual>>,
    mut quads: Query<&mut Transform>,
) {
    let in_budget = aura_is_in_budget(quality);
    for (body, worn, sprite, anchor, has) in &bodies {
        let wanted = in_budget && crate::cosmic::is_cosmic(worn.id());
        let place = aura_transform(sprite, anchor);
        match (wanted, has, place) {
            (true, None, Some(place)) => {
                let material = assets
                    .material
                    .get_or_insert_with(|| {
                        materials.add(MaryOCosmicAuraMaterial {
                            // Display colours: pink at the middle, cyan at the rim, pale gold stars.
                            inner: Vec4::new(1.0, 0.62, 0.86, 0.64),
                            outer: Vec4::new(0.52, 0.88, 1.0, 2.4),
                            star: Vec4::new(1.0, 0.96, 0.78, STAR_TILE),
                            stars: star_picture(&asset_server),
                        })
                    })
                    .clone();
                let quad = commands
                    .spawn((
                        Mesh2d(assets.quad.clone()),
                        MeshMaterial2d(material),
                        place,
                        Name::new("mary-o cosmic aura"),
                    ))
                    .id();
                commands.entity(body).try_insert(HasCosmicAura(quad)).add_child(quad);
            }
            (true, Some(has), Some(place)) => {
                // Her frame changes with her row, so the quad follows it.
                if let Ok(mut transform) = quads.get_mut(has.0) {
                    if *transform != place {
                        *transform = place;
                    }
                }
            }
            (false, Some(has), _) => {
                commands.entity(has.0).try_despawn();
                commands.entity(body).try_remove::<HasCosmicAura>();
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_headless_app_installs_no_light() {
        let mut app = App::new();
        install(&mut app);
        assert!(!app.world().contains_resource::<CosmicAuraInstalled>());
    }

    /// The light is about her body, which is under the middle of her frame:
    /// her wings and her star are over her.
    #[test]
    fn the_light_is_about_her_body_and_not_about_her_frame() {
        let sprite = Sprite {
            custom_size: Some(Vec2::new(120.0, 140.0)),
            ..default()
        };
        let place = aura_transform(&sprite, None).unwrap();
        assert!(place.translation.y < 0.0);
        assert!((place.scale.x - 140.0 * AURA_SIZE).abs() < 1e-3);
        assert!(aura_transform(&Sprite::default(), None).is_none());
    }
}

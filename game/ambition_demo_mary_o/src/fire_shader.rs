//! The glow of a fire in Mary-O: her spark blossom, and each orb of a bar of
//! fire.
//!
//! Presentation only, and content-owned. A fire is a sprite (the spark
//! blossom sheet, the fire orb entity sprite), and it is whole with no
//! shader. This adds light about it: one quad behind the sprite, a child of
//! its visual, so it goes where the sprite goes and leaves when it leaves.
//!
//! ## The art is a picture, and the shader only moves it
//!
//! The tongues of the glow are the flame picture of the sprite submodule
//! (`sprites/entities/fire_glow.png`). The shader (`shaders/fire_glow.wgsl`)
//! moves that picture up through a soft round light and gives it its colour.
//!
//! ## A fire looks right with no glow
//!
//! A device with no budget for shaders draws none ([`glow_is_in_budget`]),
//! and a headless composition installs none.

use bevy::{
    image::{ImageAddressMode, ImageFilterMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor},
    prelude::*,
    reflect::TypePath,
    render::render_resource::AsBindGroup,
    shader::ShaderRef,
    sprite_render::{AlphaMode2d, Material2d, Material2dPlugin, MeshMaterial2d},
};

use ambition_platformer2d::persistence::settings::ResolvedVisualQuality;
use ambition_platformer2d::render::rendering::projectile_visuals::{ProjectileVisual, VisualProjectile};
use ambition_platformer2d::render::rendering::{EntityArt, FeatureVisual};
use ambition_platformer2d::sim_view::ProjectileView;
use ambition_platformer2d::sprite_sheet::game_assets::EntitySprite;

/// How much larger than its fire a glow is.
const GLOW_SCALE: f32 = 2.6;
/// Behind its fire, by less than the depth between two layers of a room.
const GLOW_Z: f32 = -0.05;
/// The size of a glow whose fire states no size, in world px.
const DEFAULT_FIRE_SIZE: f32 = 24.0;

/// The material of a glow. Its fields are the uniforms of `fire_glow.wgsl`.
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct MaryOFireGlowMaterial {
    #[uniform(0)]
    pub outer: Vec4,
    #[uniform(1)]
    pub inner: Vec4,
    #[texture(2)]
    #[sampler(3)]
    pub flame: Handle<Image>,
}

impl Material2d for MaryOFireGlowMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://ambition_demo_mary_o/shaders/fire_glow.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

#[derive(Resource, Default)]
struct FireGlowInstalled;

/// The unit quad each glow scales, and the one material all glows share. The
/// shader reads the flame picture in world space, so one material gives each
/// fire its own flicker.
#[derive(Resource)]
struct FireGlowAssets {
    quad: Handle<Mesh>,
    material: Option<Handle<MaryOFireGlowMaterial>>,
}

/// On the visual of a fire that has its glow.
#[derive(Component)]
struct HasFireGlow;

/// Marks the quad of a glow.
#[derive(Component)]
struct FireGlow;

/// Whether this device draws a glow: it is a shader.
fn glow_is_in_budget(quality: Option<Res<ResolvedVisualQuality>>) -> bool {
    quality.is_none_or(|quality| quality.budget.shaders.draws_screen_shaders())
}

/// Install the material and the systems that give each fire its glow.
/// Idempotent. An app that does not draw installs nothing.
pub fn install(app: &mut App) {
    if app.world().contains_resource::<FireGlowInstalled>() {
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
    app.insert_resource(FireGlowInstalled);
    ambition_platformer2d::content_asset!(app, "src/shaders/fire_glow.wgsl");
    // The material plugin installs render-world state, so it needs an app
    // that renders. An asset registry is not evidence of a renderer.
    if app.get_sub_app(bevy::render::RenderApp).is_some() {
        app.add_plugins(Material2dPlugin::<MaryOFireGlowMaterial>::default());
    }
    // The quad is made now and not with the first fire: Bevy does not try
    // again to draw a quad whose mesh came late.
    if let Some(mut meshes) = app.world_mut().get_resource_mut::<Assets<Mesh>>() {
        let quad = meshes.add(Rectangle::new(1.0, 1.0));
        app.insert_resource(FireGlowAssets { quad, material: None });
    }
    app.add_systems(
        Update,
        (
            give_each_fire_its_glow
                .run_if(resource_exists::<FireGlowAssets>)
                .run_if(resource_exists::<Assets<MaryOFireGlowMaterial>>)
                .run_if(glow_is_in_budget),
            retire_glows_out_of_budget.run_if(not(glow_is_in_budget)),
        )
            .in_set(crate::MaryOPresentationSet),
    );
}

/// The flame picture. It repeats, so its sampler repeats, and it is data, so
/// it is not read as sRGB.
fn flame_picture(asset_server: &AssetServer) -> Handle<Image> {
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
        .load("sprites/entities/fire_glow.png")
}

/// Whether the look of a shot is a fire of Mary-O.
fn is_fire_look(visual_id: &str) -> bool {
    visual_id == crate::powerups::SPARK_VISUAL
}

/// Give its glow to each fire that has none: a hazard that is drawn as the
/// fire orb, and a shot whose look is the spark.
#[allow(clippy::type_complexity)]
fn give_each_fire_its_glow(
    mut commands: Commands,
    mut assets: ResMut<FireGlowAssets>,
    mut materials: ResMut<Assets<MaryOFireGlowMaterial>>,
    asset_server: Res<AssetServer>,
    orbs: Query<(Entity, &EntityArt, &Sprite), (With<FeatureVisual>, Without<HasFireGlow>)>,
    shots: Query<(Entity, &VisualProjectile, &Sprite), (With<ProjectileVisual>, Without<HasFireGlow>)>,
    views: Query<&ProjectileView>,
) {
    let fires = orbs
        .iter()
        .filter(|(_, art, _)| art.0 == EntitySprite::FireOrb)
        .map(|(entity, _, sprite)| (entity, sprite))
        .chain(
            shots
                .iter()
                .filter(|(_, shot, _)| views.get(shot.0).is_ok_and(|view| is_fire_look(&view.visual_id)))
                .map(|(entity, _, sprite)| (entity, sprite)),
        );
    for (fire, sprite) in fires {
        let material = assets
            .material
            .get_or_insert_with(|| {
                materials.add(MaryOFireGlowMaterial {
                    // Display colours: orange at the rim, pale gold at the middle.
                    outer: Vec4::new(1.0, 0.45, 0.10, 0.85),
                    inner: Vec4::new(1.0, 0.86, 0.45, 0.9),
                    flame: flame_picture(&asset_server),
                })
            })
            .clone();
        let size = sprite.custom_size.map_or(DEFAULT_FIRE_SIZE, |size| size.max_element());
        commands.entity(fire).try_insert(HasFireGlow).with_children(|children| {
            children.spawn((
                Mesh2d(assets.quad.clone()),
                MeshMaterial2d(material),
                Transform::from_xyz(0.0, 0.0, GLOW_Z).with_scale(Vec3::new(size * GLOW_SCALE, size * GLOW_SCALE, 1.0)),
                Name::new("mary-o fire glow"),
                FireGlow,
            ));
        });
    }
}

/// Take each glow away when the budget for it goes. A fire gets its glow
/// again when the budget comes back.
fn retire_glows_out_of_budget(
    mut commands: Commands,
    glows: Query<Entity, With<FireGlow>>,
    fires: Query<Entity, With<HasFireGlow>>,
) {
    for glow in &glows {
        commands.entity(glow).try_despawn();
    }
    for fire in &fires {
        commands.entity(fire).try_remove::<HasFireGlow>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_headless_app_installs_no_glow() {
        let mut app = App::new();
        install(&mut app);
        assert!(!app.world().contains_resource::<FireGlowInstalled>());
    }
}

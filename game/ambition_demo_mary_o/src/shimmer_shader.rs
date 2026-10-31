//! The shimmer in the air of a Mary-O room: sparkles and ribbons of light in
//! the starlight rooms, embers and a low mist in the starless room, sparks
//! and red heat in the castle, white specks in the void.
//!
//! Presentation only, and content-owned. A room has a shimmer when its biome
//! is one of Mary-O's (`ParallaxTheme::new("starlight")`, `::Stardune`, `::Starless`, `::VoidCastle`, `void`). The
//! shimmer is one quad the size of the room, behind the blocks and in front
//! of the parallax scene.
//!
//! ## The art is a picture, and the shader only moves it
//!
//! Each shape is drawn in the sprite submodule and published as
//! `room_dressing/<biome>_shimmer.png` (`terrain/shimmer.py`). The shader
//! (`shaders/shimmer.wgsl`) moves that picture, makes each shape come and
//! go, and gives it its colour.
//!
//! ## A room looks right with no shimmer
//!
//! The room is its published scene and its terrain skin. The shimmer adds
//! light on top. A device with no budget for a shader that fills the screen
//! draws none ([`shimmer_is_in_budget`]), and a headless composition
//! installs none.

use bevy::{
    image::{ImageAddressMode, ImageFilterMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor},
    prelude::*,
    reflect::TypePath,
    render::render_resource::AsBindGroup,
    shader::ShaderRef,
    sprite_render::{AlphaMode2d, Material2d, Material2dPlugin, MeshMaterial2d},
};

use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::engine_core::config::world_to_bevy;
use ambition_platformer2d::persistence::settings::ResolvedVisualQuality;
use ambition_platformer2d::platformer::lifecycle::{
    ActiveSessionScope, InRoomInstance, SessionScopeSet, SessionSpawnScope, SpawnSessionScopedExt,
};
use ambition_platformer2d::render::rendering::RoomVisual;
use ambition_platformer2d::sprite_sheet::game_assets::ParallaxTheme;
use ambition_platformer2d::world::rooms::LiveRoomSpecs;

/// Above the parallax panels (`-18.0..=-15.0`), below the blocks.
const SHIMMER_Z: f32 = -14.4;
/// How far the quad reaches past the room, for a camera at the room edge.
const SHIMMER_PAD: f32 = 1200.0;

/// One shimmer: its colours and its motion. Colours are display values.
#[derive(Clone, Copy, Debug, PartialEq)]
struct ShimmerStyle {
    /// The two colours of the shapes.
    shapes: [[f32; 3]; 2],
    /// The two colours of the ribbons.
    ribbons: [[f32; 3]; 2],
    /// How strong the shapes and the ribbons are, 0 to 1.
    shape_strength: f32,
    ribbon_strength: f32,
    /// How fast a shape comes and goes, in radians per second.
    twinkle: f32,
    /// The size of one picture, in world px.
    tile: f32,
    /// How fast the shapes go up, in world px per second.
    rise: f32,
    /// The band of the room the ribbons are in: the part of its height it
    /// starts at, and the part of its height it goes over.
    band: [f32; 2],
}

/// The shimmer of a theme, or `None` for a theme that is not Mary-O's.
fn style_of(theme: ParallaxTheme) -> Option<ShimmerStyle> {
    match theme.key() {
        // Pale gold and pink sparkles; ribbons of pink and mint across the sky.
        "starlight" => Some(ShimmerStyle {
            shapes: [[1.0, 0.96, 0.78], [1.0, 0.80, 0.92]],
            ribbons: [[1.0, 0.72, 0.88], [0.70, 0.96, 0.86]],
            shape_strength: 0.70,
            ribbon_strength: 0.22,
            twinkle: 1.3,
            tile: 620.0,
            rise: 5.0,
            band: [-0.05, 0.75],
        }),
        // Pale gold sparkles, few; ribbons of sand low over the ground.
        "stardune" => Some(ShimmerStyle {
            shapes: [[1.0, 0.97, 0.80], [1.0, 0.88, 0.70]],
            ribbons: [[1.0, 0.88, 0.66], [1.0, 0.78, 0.60]],
            shape_strength: 0.55,
            ribbon_strength: 0.18,
            twinkle: 1.1,
            tile: 660.0,
            rise: 3.0,
            band: [0.30, 0.60],
        }),
        // Magenta embers that go up; a low mist of violet.
        "starless" => Some(ShimmerStyle {
            shapes: [[1.0, 0.42, 0.72], [1.0, 0.70, 0.52]],
            ribbons: [[0.62, 0.22, 0.56], [0.36, 0.20, 0.62]],
            shape_strength: 0.80,
            ribbon_strength: 0.30,
            twinkle: 1.9,
            tile: 520.0,
            rise: 14.0,
            band: [0.40, 0.70],
        }),
        // Sparks of the fire that go up fast; red heat low in the hall.
        "void_castle" => Some(ShimmerStyle {
            shapes: [[1.0, 0.62, 0.26], [1.0, 0.86, 0.50]],
            ribbons: [[0.86, 0.22, 0.14], [0.66, 0.16, 0.30]],
            shape_strength: 0.85,
            ribbon_strength: 0.26,
            twinkle: 2.2,
            tile: 480.0,
            rise: 30.0,
            band: [0.55, 0.45],
        }),
        // The void: specks of white that come and go, slow; one thin ribbon
        // of grey mist. No colour.
        "void" => Some(ShimmerStyle {
            shapes: [[1.0, 1.0, 1.0], [0.82, 0.82, 0.84]],
            ribbons: [[0.30, 0.30, 0.32], [0.20, 0.20, 0.22]],
            shape_strength: 0.70,
            ribbon_strength: 0.16,
            twinkle: 0.9,
            tile: 560.0,
            rise: 4.0,
            band: [0.45, 0.40],
        }),
        _ => None,
    }
}

/// The material of a shimmer. Its fields are the uniforms of `shimmer.wgsl`.
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct MaryOShimmerMaterial {
    #[uniform(0)]
    pub room: Vec4,
    #[uniform(1)]
    pub shape_a: Vec4,
    #[uniform(2)]
    pub shape_b: Vec4,
    #[uniform(3)]
    pub ribbon_a: Vec4,
    #[uniform(4)]
    pub ribbon_b: Vec4,
    #[uniform(5)]
    pub band: Vec4,
    #[texture(6)]
    #[sampler(7)]
    pub shimmer: Handle<Image>,
}

impl Material2d for MaryOShimmerMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://ambition_demo_mary_o/shaders/shimmer.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

#[derive(Resource, Default)]
struct ShimmerInstalled;

/// The unit quad each shimmer scales.
#[derive(Resource)]
struct ShimmerQuad(Handle<Mesh>);

/// Marks the quad of a shimmer. It is stamped with its room, so it leaves
/// with the room.
#[derive(Component)]
struct PresentedShimmer;

/// Whether this device draws a shimmer: it is a shader that fills the screen.
fn shimmer_is_in_budget(quality: Option<Res<ResolvedVisualQuality>>) -> bool {
    quality.is_none_or(|quality| quality.budget.shaders.draws_screen_shaders())
}

/// Install the material and the system that gives each room its shimmer.
/// Idempotent. An app that does not draw installs nothing.
pub fn install(app: &mut App) {
    if app.world().contains_resource::<ShimmerInstalled>() {
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
    app.insert_resource(ShimmerInstalled);
    ambition_platformer2d::content_asset!(app, "src/shaders/shimmer.wgsl");
    // The material plugin installs render-world state, so it needs an app
    // that renders. An asset registry is not evidence of a renderer.
    if app.get_sub_app(bevy::render::RenderApp).is_some() {
        app.add_plugins(Material2dPlugin::<MaryOShimmerMaterial>::default());
    }
    // The quad is made now and not with the first room: Bevy does not try
    // again to draw a quad whose mesh came late.
    if let Some(mut meshes) = app.world_mut().get_resource_mut::<Assets<Mesh>>() {
        let quad = meshes.add(Rectangle::new(1.0, 1.0));
        app.insert_resource(ShimmerQuad(quad));
    }
    app.add_systems(
        Update,
        (
            present_shimmers
                // With the room's own visuals.
                .in_set(SessionScopeSet::Presentation)
                .run_if(resource_exists::<ShimmerQuad>)
                .run_if(resource_exists::<Assets<MaryOShimmerMaterial>>)
                .run_if(shimmer_is_in_budget),
            retire_shimmers_out_of_budget.run_if(not(shimmer_is_in_budget)),
        )
            .in_set(crate::MaryOPresentationSet),
    );
}

/// Take each shimmer away when the budget for it goes.
fn retire_shimmers_out_of_budget(mut commands: Commands, shimmers: Query<Entity, With<PresentedShimmer>>) {
    for shimmer in &shimmers {
        commands.entity(shimmer).try_despawn();
    }
}

fn rgb_and(colour: [f32; 3], w: f32) -> Vec4 {
    Vec4::new(colour[0], colour[1], colour[2], w)
}

/// The picture of a shimmer. It repeats, so its sampler repeats, and it is
/// data, so it is not read as sRGB.
fn shimmer_picture(asset_server: &AssetServer, theme: ParallaxTheme) -> Handle<Image> {
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
        .load(format!("room_dressing/{}_shimmer.png", theme.key()))
}

/// Give each live room of a Mary-O biome its shimmer, one time.
fn present_shimmers(
    mut commands: Commands,
    rooms: LiveRoomSpecs,
    presented: Query<&InRoomInstance, With<PresentedShimmer>>,
    quad: Res<ShimmerQuad>,
    mut materials: ResMut<Assets<MaryOShimmerMaterial>>,
    asset_server: Res<AssetServer>,
    active_session: Option<Res<ActiveSessionScope>>,
) {
    let Some(session_scope) = SessionSpawnScope::for_optional_active_session(active_session.as_deref()) else {
        return;
    };
    for (room, definition) in rooms.live_rooms() {
        if presented.iter().any(|stamp| stamp.0 == room) {
            continue;
        }
        let spec = rooms.rooms().spec(definition);
        let Some(theme) = ParallaxTheme::named_by_room_metadata(&spec.metadata) else {
            continue;
        };
        let Some(style) = style_of(theme) else {
            continue;
        };
        let world = &spec.world;
        let size = Vec2::new(world.size.x, world.size.y);
        let material = materials.add(MaryOShimmerMaterial {
            room: Vec4::new(size.x, size.y, style.shape_strength, style.ribbon_strength),
            shape_a: rgb_and(style.shapes[0], style.twinkle),
            shape_b: rgb_and(style.shapes[1], style.tile),
            ribbon_a: rgb_and(style.ribbons[0], style.rise),
            ribbon_b: rgb_and(style.ribbons[1], 0.0),
            band: Vec4::new(style.band[0], style.band[1], 0.0, 0.0),
            shimmer: shimmer_picture(&asset_server, theme),
        });
        commands.spawn_session_scoped(
            session_scope.in_room(Some(room)),
            (
                Mesh2d(quad.0.clone()),
                MeshMaterial2d(material),
                Transform::from_translation(world_to_bevy(world, ae::Vec2::new(size.x * 0.5, size.y * 0.5), SHIMMER_Z))
                    .with_scale(Vec3::new(size.x + SHIMMER_PAD * 2.0, size.y + SHIMMER_PAD * 2.0, 1.0)),
                Name::new(format!("mary-o shimmer: {}", theme.key())),
                RoomVisual,
                PresentedShimmer,
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_biomes_of_mary_o_have_a_shimmer() {
        let with: Vec<_> = ["starlight", "stardune", "starless", "void_castle", "void", "independent_game"].into_iter().map(ParallaxTheme::new).filter(|theme| style_of(*theme).is_some()).collect();
        assert_eq!(
            with,
            vec![ParallaxTheme::new("starlight"), ParallaxTheme::new("stardune"), ParallaxTheme::new("starless"), ParallaxTheme::new("void_castle"), ParallaxTheme::new("void")]
        );
    }

    #[test]
    fn a_headless_app_installs_no_shimmer() {
        let mut app = App::new();
        install(&mut app);
        assert!(!app.world().contains_resource::<ShimmerInstalled>());
    }
}

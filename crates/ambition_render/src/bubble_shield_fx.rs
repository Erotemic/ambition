//! The live layer of the bubble shield.
//!
//! The bubble is a sprite: an authored picture with a colour for the guard's
//! state (`rendering::bubble_shield`). That is the whole look on a device
//! with no budget for shaders. On a device with one, each bubble has a quad
//! over it that a shader draws (`shaders/bubble_shield_fx.wgsl`): a honeycomb
//! that shows near the rim and drifts, a ring that goes round the bubble
//! from the point where a blow landed, all of it lit in a parry, and cracks that grow from the rim as
//! the guard is spent.
//!
//! The shader draws no art of its own. The honeycomb and the cracks are
//! pictures of the sprite submodule (its `bubble_shield` target); the shader
//! says where on the bubble they show.
//!
//! Presentation only: it reads what the bubble's own sync has written
//! ([`BubbleShieldLook`]) and no simulation state.

use bevy::{
    asset::embedded_asset,
    prelude::*,
    reflect::TypePath,
    render::render_resource::AsBindGroup,
    shader::ShaderRef,
    sprite_render::{AlphaMode2d, Material2d, Material2dPlugin, MeshMaterial2d},
};

use crate::quality::ResolvedVisualQuality;
use crate::rendering::bubble_shield::{BubbleShieldLook, BubbleShieldSprite, BubbleShieldVisual};

/// Over the bubble's own sprite, by less than the space to the next layer.
const FX_Z: f32 = 0.002;

/// The material of the layer. Bindings are `vec4` uniforms and two textures,
/// as the other materials of the game are, for WebGL2.
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct BubbleShieldFxMaterial {
    /// `rgb`: the hue of the guard's state. `a`: the strength of the layer.
    #[uniform(0)]
    pub tint: Vec4,
    /// `x`: how whole the guard is. `y`: the pulse of a blow it took. `z`: 1
    /// in a parry. `w`: the width of the quad over its height.
    #[uniform(1)]
    pub state: Vec4,
    /// `xy`: where on the bubble a blocked blow landed, with the bubble as a
    /// circle of radius 1. `z`: how far through its beat, 0 to 1. `w`: 1
    /// while the beat runs.
    #[uniform(6)]
    pub blow: Vec4,
    #[texture(2)]
    #[sampler(3)]
    pub lattice: Handle<Image>,
    #[texture(4)]
    #[sampler(5)]
    pub cracks: Handle<Image>,
}

impl Material2d for BubbleShieldFxMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://ambition_render/shaders/bubble_shield_fx.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

/// The unit quad each layer scales.
#[derive(Resource)]
struct BubbleShieldFxQuad(Handle<Mesh>);

/// On a bubble: its layer.
#[derive(Component)]
struct HasBubbleShieldFx(Entity);

/// On the layer of a bubble.
#[derive(Component)]
pub struct BubbleShieldFxLayer;

/// Presentation plugin of the layer. An app that does not draw installs
/// nothing; a bubble is then its sprite.
pub struct BubbleShieldFxPlugin;

impl Plugin for BubbleShieldFxPlugin {
    fn build(&self, app: &mut App) {
        if app.get_sub_app(bevy::render::RenderApp).is_none() {
            return;
        }
        embedded_asset!(app, "shaders/bubble_shield_fx.wgsl");
        app.add_plugins(Material2dPlugin::<BubbleShieldFxMaterial>::default());
        let Some(mut meshes) = app.world_mut().get_resource_mut::<Assets<Mesh>>() else {
            return;
        };
        let quad = meshes.add(Rectangle::new(1.0, 1.0));
        app.insert_resource(BubbleShieldFxQuad(quad));
        app.add_systems(
            Update,
            (give_each_bubble_its_layer, sync_bubble_shield_fx)
                .chain()
                .after(crate::rendering::bubble_shield::sync_bubble_shield_visual)
                .run_if(resource_exists::<BubbleShieldSprite>),
        );
    }
}

/// Whether this device draws the layer: it is a shader.
fn layer_is_in_budget(quality: Option<&ResolvedVisualQuality>) -> bool {
    quality.is_none_or(|quality| quality.budget.shaders.draws_screen_shaders())
}

/// A child of each bubble that has none: the quad of its layer. A child, so
/// it is where the bubble is, turned as it is, and hidden when it is.
fn give_each_bubble_its_layer(
    mut commands: Commands,
    quad: Res<BubbleShieldFxQuad>,
    art: Res<BubbleShieldSprite>,
    mut materials: ResMut<Assets<BubbleShieldFxMaterial>>,
    bubbles: Query<Entity, (With<BubbleShieldVisual>, Without<HasBubbleShieldFx>)>,
) {
    for bubble in &bubbles {
        let material = materials.add(BubbleShieldFxMaterial {
            tint: Vec4::ZERO,
            state: Vec4::new(1.0, 0.0, 0.0, 1.0),
            blow: Vec4::ZERO,
            lattice: art.lattice.clone(),
            cracks: art.cracks.clone(),
        });
        let layer = commands
            .spawn((
                Mesh2d(quad.0.clone()),
                MeshMaterial2d(material),
                Transform::from_xyz(0.0, 0.0, FX_Z),
                Visibility::Hidden,
                BubbleShieldFxLayer,
                Name::new("Bubble Shield Fx"),
            ))
            .id();
        commands.entity(bubble).add_child(layer).insert(HasBubbleShieldFx(layer));
    }
}

/// Each layer takes the size, the room and the look of its bubble.
fn sync_bubble_shield_fx(
    mut commands: Commands,
    quality: Option<Res<ResolvedVisualQuality>>,
    mut materials: ResMut<Assets<BubbleShieldFxMaterial>>,
    bubbles: Query<
        (
            &Sprite,
            &BubbleShieldLook,
            &HasBubbleShieldFx,
            Option<&ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance>,
        ),
        With<BubbleShieldVisual>,
    >,
    mut layers: Query<
        (
            &mut Transform,
            &mut Visibility,
            &MeshMaterial2d<BubbleShieldFxMaterial>,
            Option<&ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance>,
        ),
        With<BubbleShieldFxLayer>,
    >,
) {
    let in_budget = layer_is_in_budget(quality.as_deref());
    for (sprite, look, has, room) in &bubbles {
        let Ok((mut transform, mut visibility, material, layer_room)) = layers.get_mut(has.0) else {
            continue;
        };
        let size = sprite.custom_size.unwrap_or(Vec2::ONE);
        // The bubble hides itself; the layer goes with it.
        let show = in_budget && size.x > 0.0 && size.y > 0.0;
        let wanted = if show { Visibility::Inherited } else { Visibility::Hidden };
        if *visibility != wanted {
            *visibility = wanted;
        }
        if !show {
            continue;
        }
        // A view draws the things of the room it frames: the layer is of its
        // bubble's room.
        if let Some(room) = room.filter(|room| layer_room.map(|stamp| stamp.0) != Some(room.0)) {
            commands
                .entity(has.0)
                .try_insert(ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance(room.0));
        }
        let scale = Vec3::new(size.x, size.y, 1.0);
        if transform.scale != scale {
            transform.scale = scale;
        }
        let colour = sprite.color.to_srgba();
        let tint = Vec4::new(colour.red, colour.green, colour.blue, look.strength);
        let state = Vec4::new(look.integrity, look.pulse, f32::from(u8::from(look.parrying)), size.x / size.y);
        let blow = look
            .blow_phase
            .map_or(Vec4::ZERO, |phase| Vec4::new(look.blow_at.x, look.blow_at.y, phase, 1.0));
        // Compared before it is taken mutably: `get_mut` marks the material
        // modified, and a modified material is prepared again for the GPU.
        let unchanged = materials
            .get(&material.0)
            .is_some_and(|current| current.tint == tint && current.state == state && current.blow == blow);
        if unchanged {
            continue;
        }
        if let Some(mut material) = materials.get_mut(&material.0) {
            material.tint = tint;
            material.state = state;
            material.blow = blow;
        }
    }
}

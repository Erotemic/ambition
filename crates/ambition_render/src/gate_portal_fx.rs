//! The wormhole of a gate, at Ultra.
//!
//! A gate's membrane is a sprite (`interdimensional_gate_portal`), and that is
//! the whole look below Ultra. At Ultra each membrane has a quad over it that a
//! shader draws (`shaders/gate_portal_fx.wgsl`): the inside of the wormhole
//! streaming down a twisting tunnel toward a light at its far end, darker
//! toward the lip, and a wave that runs out through it when a body goes in or
//! comes out.
//!
//! The shader draws no art of its own. The tunnel is a picture of the sprite
//! submodule (`interdimensional_gate_tunnel.png`, from the gate's target); the
//! membrane's own frame says where the layer shows, so it opens, closes and
//! ripples with the authored sheet.
//!
//! Presentation only: it reads the gate's phase and the passages through it,
//! and no simulation state.

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

use ambition_platformer2d_world::rooms::{GatePortalPhase, GatePortalPhases, GatePortalRegistry};

use crate::quality::ResolvedVisualQuality;
use crate::rendering::{PortalSprite, PropVisual};

const TUNNEL_PICTURE: &str = "sprites/interdimensional_gate_tunnel.png";

/// Over the membrane, by less than the space to the next layer.
const FX_Z: f32 = 0.01;

/// The material of the layer. Bindings are `vec4` uniforms and two textures,
/// as the other materials of the game are, for WebGL2.
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct GatePortalFxMaterial {
    /// The membrane's current frame on its sheet, `(min.x, min.y, max.x,
    /// max.y)` normalized.
    #[uniform(0)]
    pub uv_rect: Vec4,
    /// `x`: the strength of the layer (0 to 1, with the opening). `y`: 1 when
    /// the membrane is drawn flipped. `z`: how far through a passage's wave,
    /// 0 to 1. `w`: the wave's way: 1 coming out, -1 going in, 0 none.
    #[uniform(1)]
    pub control: Vec4,
    #[texture(2)]
    #[sampler(3)]
    pub membrane: Handle<Image>,
    #[texture(4)]
    #[sampler(5)]
    pub tunnel: Handle<Image>,
}

impl Material2d for GatePortalFxMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://ambition_render/shaders/gate_portal_fx.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

/// The tunnel picture and the unit quad each layer scales.
#[derive(Resource)]
struct GatePortalFxArt {
    tunnel: Handle<Image>,
    quad: Handle<Mesh>,
}

/// On a membrane: its layer.
#[derive(Component)]
struct HasGatePortalFx(Entity);

/// On the layer of a membrane.
#[derive(Component)]
pub struct GatePortalFxLayer;

/// Presentation plugin of the layer. An app that does not draw installs
/// nothing; a membrane is then its sprite.
pub struct GatePortalFxPlugin;

impl Plugin for GatePortalFxPlugin {
    fn build(&self, app: &mut App) {
        if app.get_sub_app(bevy::render::RenderApp).is_none() {
            return;
        }
        embedded_asset!(app, "shaders/gate_portal_fx.wgsl");
        app.add_plugins(Material2dPlugin::<GatePortalFxMaterial>::default());
        app.add_systems(Startup, load_gate_portal_fx_art);
        app.add_systems(
            Update,
            sync_gate_portal_fx
                .after(crate::rendering::gate_portal_visuals::sync_portal_sprite_animation)
                .run_if(resource_exists::<GatePortalFxArt>)
                .run_if(resource_exists::<GatePortalRegistry>)
                .run_if(resource_exists::<GatePortalPhases>),
        );
    }
}

fn load_gate_portal_fx_art(mut commands: Commands, asset_server: Res<AssetServer>, mut meshes: ResMut<Assets<Mesh>>) {
    use ambition_sprite_sheet::game_assets::load_sheet_image;
    commands.insert_resource(GatePortalFxArt {
        tunnel: load_sheet_image(&asset_server, "gate-portal", TUNNEL_PICTURE),
        quad: meshes.add(Rectangle::new(1.0, 1.0)),
    });
}

/// How strong the layer is in a phase: up with the opening, down with the
/// closing, none while the gate is shut.
fn phase_strength(phase: GatePortalPhase) -> f32 {
    let smooth = |t: f32| t * t * (3.0 - 2.0 * t);
    match phase {
        GatePortalPhase::Off => 0.0,
        GatePortalPhase::On => 1.0,
        GatePortalPhase::Opening { .. } => smooth(phase.sequence_progress().unwrap_or(0.0)),
        GatePortalPhase::Closing { .. } => 1.0 - smooth(phase.sequence_progress().unwrap_or(1.0)),
    }
}

/// Give each membrane its layer, and keep the layer on the membrane's frame,
/// its phase and the wave of a passage through it. Below Ultra the layer is
/// hidden.
#[allow(clippy::too_many_arguments)]
fn sync_gate_portal_fx(
    mut commands: Commands,
    art: Res<GatePortalFxArt>,
    portals: Res<GatePortalRegistry>,
    phases: Res<GatePortalPhases>,
    quality: Option<Res<ResolvedVisualQuality>>,
    passages: Option<Res<ambition_sim_view::PassageViews>>,
    layouts: Res<Assets<TextureAtlasLayout>>,
    mut materials: ResMut<Assets<GatePortalFxMaterial>>,
    membranes: Query<(Entity, &PropVisual, &Sprite, Option<&Anchor>, Option<&HasGatePortalFx>), With<PortalSprite>>,
    mut layers: Query<(&mut Transform, &MeshMaterial2d<GatePortalFxMaterial>, &mut Visibility), With<GatePortalFxLayer>>,
    rooms: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
) {
    let showcase = quality.as_deref().is_some_and(|quality| quality.budget.shaders.allow_showcase_materials);
    for (zone_id, config) in portals.iter() {
        let phase = phases.phase(zone_id);
        for (membrane, prop, sprite, anchor, layer) in &membranes {
            if prop.name != config.portal_sprite_name {
                continue;
            }
            let Some(layer) = layer else {
                if showcase {
                    let material = materials.add(GatePortalFxMaterial {
                        uv_rect: Vec4::new(0.0, 0.0, 1.0, 1.0),
                        control: Vec4::ZERO,
                        membrane: sprite.image.clone(),
                        tunnel: art.tunnel.clone(),
                    });
                    // A child, so it is where the membrane is, goes with its
                    // room, and is hidden when it is.
                    let layer = commands
                        .spawn((
                            Mesh2d(art.quad.clone()),
                            MeshMaterial2d(material),
                            Transform::from_xyz(0.0, 0.0, FX_Z),
                            Visibility::Hidden,
                            GatePortalFxLayer,
                            ChildOf(membrane),
                            Name::new("Gate Portal Fx"),
                        ))
                        .id();
                    commands.entity(membrane).try_insert(HasGatePortalFx(layer));
                }
                continue;
            };
            let Ok((mut transform, material, mut visibility)) = layers.get_mut(layer.0) else {
                commands.entity(membrane).try_remove::<HasGatePortalFx>();
                continue;
            };
            let strength = if showcase { phase_strength(phase) } else { 0.0 };
            let (Some(size), Some(uv_rect)) = (sprite.custom_size, frame_uv_rect(sprite, &layouts)) else {
                visibility.set_if_neq(Visibility::Hidden);
                continue;
            };
            visibility.set_if_neq(if strength > 0.0 { Visibility::Inherited } else { Visibility::Hidden });
            if strength <= 0.0 {
                continue;
            }
            // The quad covers the membrane's drawn frame: its size, moved by
            // its anchor as the sprite is.
            let offset = -anchor.map_or(Vec2::ZERO, |anchor| anchor.0) * size;
            transform.set_if_neq(Transform {
                translation: offset.extend(FX_Z),
                rotation: Quat::IDENTITY,
                scale: size.extend(1.0),
            });
            let wave = passages
                .as_deref()
                .and_then(|passages| passages.gate_ripple(rooms.of(membrane), zone_id))
                .map_or(Vec2::ZERO, |(row, k)| Vec2::new(k, if row == "emit" { 1.0 } else { -1.0 }));
            let control = Vec4::new(strength, if sprite.flip_x { 1.0 } else { 0.0 }, wave.x, wave.y);
            // Read before writing: `get_mut` uploads the material again.
            let unchanged = materials.get(&material.0).is_some_and(|m| {
                m.uv_rect == uv_rect && m.control == control && m.membrane == sprite.image
            });
            if !unchanged {
                if let Some(mut m) = materials.get_mut(&material.0) {
                    m.uv_rect = uv_rect;
                    m.control = control;
                    m.membrane = sprite.image.clone();
                }
            }
        }
    }
}

/// The sprite's current frame on its texture, normalized.
fn frame_uv_rect(sprite: &Sprite, layouts: &Assets<TextureAtlasLayout>) -> Option<Vec4> {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The layer comes up with the opening and goes with the closing, and
    /// shows nothing while the gate is shut.
    #[test]
    fn the_layer_follows_the_gates_phase() {
        use ambition_platformer2d_world::rooms::{PORTAL_CLOSING_DURATION_SECS, PORTAL_OPENING_DURATION_SECS};
        assert_eq!(phase_strength(GatePortalPhase::Off), 0.0);
        assert_eq!(phase_strength(GatePortalPhase::On), 1.0);
        assert_eq!(phase_strength(GatePortalPhase::Opening { elapsed: 0.0 }), 0.0);
        assert_eq!(phase_strength(GatePortalPhase::Opening { elapsed: PORTAL_OPENING_DURATION_SECS }), 1.0);
        assert_eq!(phase_strength(GatePortalPhase::Closing { elapsed: 0.0 }), 1.0);
        assert_eq!(phase_strength(GatePortalPhase::Closing { elapsed: PORTAL_CLOSING_DURATION_SECS }), 0.0);
    }
}

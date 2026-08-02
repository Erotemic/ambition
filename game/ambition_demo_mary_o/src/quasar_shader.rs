//! Mary-O's invincible rainbow-quasar presentation pass.
//!
//! The effect is presentation-only and content-owned. It activates for a
//! Mary-O player body while its ordinary [`BodyOffense::invincible`] fact is
//! true, so the eventual Cosmic Quasar pickup does not need to know anything
//! about Bevy materials. The source sprite remains visible underneath a synced
//! sibling `Material2d` mesh; a shader failure therefore degrades to ordinary
//! Mary-O rather than making the player disappear.
//!
//! [`BodyOffense::invincible`]: ambition_platformer2d::engine_core::BodyOffense::invincible

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

use ambition_platformer2d::characters::actor::WornCharacter;
use ambition_platformer2d::engine_core::BodyOffense;
use ambition_platformer2d::platformer::lifecycle::{
    SessionScopedEntity, SessionSpawnScope, SpawnSessionScopedExt,
};
use ambition_platformer2d::render::rendering::{ActorOverlaySet, PlayerVisual, RoomVisual};

const EFFECT_STRENGTH: f32 = 1.0;
const OVERLAY_ALPHA: f32 = 0.96;
const OVERLAY_Z_BIAS: f32 = 1.0;

#[derive(Resource, Default)]
struct QuasarShaderInstalled;

/// Runtime tuning for development and capture tooling.
#[derive(Resource, Reflect, Debug, Clone, Copy)]
#[reflect(Resource)]
pub struct MaryOQuasarShaderSettings {
    /// Disable the overlay without changing the authoritative invincible fact.
    pub disabled: bool,
    /// Multiplier applied to the shader's authored effect strength.
    pub strength: f32,
}

impl Default for MaryOQuasarShaderSettings {
    fn default() -> Self {
        Self {
            disabled: false,
            strength: 1.0,
        }
    }
}

/// Install the embedded WGSL material and its player-overlay lifecycle.
///
/// Headless apps intentionally skip the pass because they do not install the
/// embedded-asset/render registries. This function is idempotent so both Mary-O
/// composition paths may call it safely.
pub fn install(app: &mut App) {
    if app.world().contains_resource::<QuasarShaderInstalled>() {
        return;
    }
    if app
        .world()
        .get_resource::<bevy::asset::io::embedded::EmbeddedAssetRegistry>()
        .is_none()
    {
        return;
    }

    app.insert_resource(QuasarShaderInstalled);
    embedded_asset!(app, "shaders/invincible_rainbow_quasar.wgsl");
    app.add_plugins(Material2dPlugin::<MaryOQuasarMaterial>::default());
    app.init_resource::<MaryOQuasarShaderSettings>();
    app.register_type::<MaryOQuasarShaderSettings>();
    app.add_systems(
        Update,
        (
            attach_quasar_overlays,
            sync_quasar_overlays,
            cleanup_quasar_overlays,
        )
            .chain()
            .in_set(ActorOverlaySet),
    );
}

/// Atlas-aware material for Mary-O's invincible spectral overlay.
///
/// Bindings:
/// - `uv_rect`: normalized atlas-frame bounds.
/// - `control`: elapsed seconds, x-flip, strength, deterministic seed.
/// - `detail`: local-frame texel size, reserved pulse channel, overlay alpha.
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct MaryOQuasarMaterial {
    #[uniform(0)]
    pub uv_rect: Vec4,
    #[uniform(1)]
    pub control: Vec4,
    #[uniform(2)]
    pub detail: Vec4,
    #[texture(3)]
    #[sampler(4)]
    pub color_texture: Handle<Image>,
}

impl Material2d for MaryOQuasarMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://ambition_demo_mary_o/shaders/invincible_rainbow_quasar.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

#[derive(Component, Debug, Clone, Copy)]
struct MaryOQuasarSource {
    overlay: Entity,
    seed: f32,
}

#[derive(Component, Debug, Clone, Copy)]
struct MaryOQuasarOverlay {
    source: Entity,
}

fn attach_quasar_overlays(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<MaryOQuasarMaterial>>,
    texture_layouts: Res<Assets<TextureAtlasLayout>>,
    images: Res<Assets<Image>>,
    candidates: Query<
        (
            Entity,
            &WornCharacter,
            &Transform,
            &Sprite,
            Option<&Anchor>,
            Option<&SessionScopedEntity>,
        ),
        (With<PlayerVisual>, Without<MaryOQuasarSource>),
    >,
) {
    for (source_entity, worn, transform, sprite, anchor, session_owner) in &candidates {
        if !is_mary_o_form(worn.id()) {
            continue;
        }
        let Some(render_size) = sprite.custom_size else {
            continue;
        };
        let Some((uv_rect, frame_texel)) =
            current_sprite_frame(sprite, &texture_layouts, &images)
        else {
            continue;
        };

        let seed = seed_from_id(worn.id());
        let material = materials.add(MaryOQuasarMaterial {
            uv_rect,
            control: Vec4::new(0.0, flip_flag(sprite), EFFECT_STRENGTH, seed),
            detail: Vec4::new(frame_texel.x, frame_texel.y, 0.0, OVERLAY_ALPHA),
            color_texture: sprite.image.clone(),
        });
        let mesh = meshes.add(Rectangle::default());
        let overlay_transform = overlay_transform_from_source(transform, anchor, render_size);
        let session_scope = SessionSpawnScope::new(session_owner.map(|owner| owner.0));
        let overlay = commands
            .spawn_session_scoped(
                session_scope,
                (
                    Mesh2d(mesh),
                    MeshMaterial2d(material),
                    overlay_transform,
                    Visibility::Hidden,
                    MaryOQuasarOverlay {
                        source: source_entity,
                    },
                    RoomVisual,
                    Name::new("Mary-O Invincible Rainbow Quasar Overlay"),
                ),
            )
            .id();
        commands
            .entity(source_entity)
            .insert(MaryOQuasarSource { overlay, seed });
    }
}

fn sync_quasar_overlays(
    presentation_time: ambition_platformer2d::time::PresentationTime,
    mut elapsed: Local<f32>,
    settings: Res<MaryOQuasarShaderSettings>,
    texture_layouts: Res<Assets<TextureAtlasLayout>>,
    images: Res<Assets<Image>>,
    sources: Query<
        (
            Entity,
            &WornCharacter,
            &BodyOffense,
            &Transform,
            &Sprite,
            Option<&Anchor>,
            &MaryOQuasarSource,
            Option<&Visibility>,
        ),
        Without<MaryOQuasarOverlay>,
    >,
    mut overlays: Query<(
        &mut Transform,
        &mut Visibility,
        &MeshMaterial2d<MaryOQuasarMaterial>,
        &MaryOQuasarOverlay,
    )>,
    mut materials: ResMut<Assets<MaryOQuasarMaterial>>,
) {
    *elapsed += presentation_time.wall_dt();

    for (
        source_entity,
        worn,
        offense,
        source_transform,
        source_sprite,
        anchor,
        source,
        source_visibility,
    ) in &sources
    {
        let Ok((mut overlay_transform, mut overlay_visibility, material_handle, overlay)) =
            overlays.get_mut(source.overlay)
        else {
            continue;
        };
        if overlay.source != source_entity {
            continue;
        }

        let source_visible = !matches!(source_visibility, Some(v) if *v == Visibility::Hidden);
        let enabled = is_mary_o_form(worn.id())
            && offense.invincible
            && source_visible
            && !settings.disabled
            && settings.strength > 0.0;
        *overlay_visibility = if enabled {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if !enabled {
            continue;
        }

        let Some(render_size) = source_sprite.custom_size else {
            continue;
        };
        let Some((uv_rect, frame_texel)) =
            current_sprite_frame(source_sprite, &texture_layouts, &images)
        else {
            continue;
        };

        *overlay_transform =
            overlay_transform_from_source(source_transform, anchor, render_size);
        if let Some(material) = materials.get_mut(&material_handle.0) {
            material.uv_rect = uv_rect;
            material.control = Vec4::new(
                *elapsed,
                flip_flag(source_sprite),
                (EFFECT_STRENGTH * settings.strength).clamp(0.0, 2.0),
                source.seed,
            );
            material.detail = Vec4::new(
                frame_texel.x,
                frame_texel.y,
                0.0,
                OVERLAY_ALPHA,
            );
            material.color_texture = source_sprite.image.clone();
        }
    }
}

fn cleanup_quasar_overlays(
    mut commands: Commands,
    sources: Query<(), With<MaryOQuasarSource>>,
    overlays: Query<(Entity, &MaryOQuasarOverlay)>,
) {
    for (overlay_entity, overlay) in &overlays {
        if sources.get(overlay.source).is_err() {
            commands.entity(overlay_entity).despawn();
        }
    }
}

fn is_mary_o_form(id: &str) -> bool {
    matches!(id, "mary_o" | "mary_o_tall" | "mary_o_fire")
}

fn current_sprite_frame(
    sprite: &Sprite,
    texture_layouts: &Assets<TextureAtlasLayout>,
    images: &Assets<Image>,
) -> Option<(Vec4, Vec2)> {
    let atlas = sprite.texture_atlas.as_ref()?;
    let layout = texture_layouts.get(&atlas.layout)?;
    let rect = layout.textures.get(atlas.index)?;
    let image = images.get(&sprite.image)?;
    let texture_size = image.texture_descriptor.size;
    let texture_size = Vec2::new(
        texture_size.width.max(1) as f32,
        texture_size.height.max(1) as f32,
    );
    let frame_size = Vec2::new(
        (rect.max.x - rect.min.x).max(1) as f32,
        (rect.max.y - rect.min.y).max(1) as f32,
    );
    Some((
        Vec4::new(
            rect.min.x as f32 / texture_size.x,
            rect.min.y as f32 / texture_size.y,
            rect.max.x as f32 / texture_size.x,
            rect.max.y as f32 / texture_size.y,
        ),
        Vec2::ONE / frame_size,
    ))
}

fn overlay_transform_from_source(
    source: &Transform,
    anchor: Option<&Anchor>,
    render_size: Vec2,
) -> Transform {
    let anchor_offset = anchor_to_mesh_offset(anchor, render_size);
    let world_offset = source.rotation.mul_vec3(anchor_offset.extend(0.0));
    let mut transform = *source;
    transform.translation += world_offset;
    transform.translation.z += OVERLAY_Z_BIAS;
    transform.scale = render_size.extend(1.0);
    transform
}

fn anchor_to_mesh_offset(anchor: Option<&Anchor>, render_size: Vec2) -> Vec2 {
    let anchor = anchor.map(|a| a.0).unwrap_or(Vec2::ZERO);
    -anchor * render_size
}

fn flip_flag(sprite: &Sprite) -> f32 {
    if sprite.flip_x { 1.0 } else { 0.0 }
}

fn seed_from_id(id: &str) -> f32 {
    let mut hash: u32 = 0x811c9dc5;
    for byte in id.bytes() {
        hash ^= byte as u32;
        hash = hash.wrapping_mul(0x01000193);
    }
    (hash & 0x00ff_ffff) as f32 / 16_777_216.0
}

#[cfg(test)]
mod tests {
    use super::is_mary_o_form;

    #[test]
    fn quasar_shader_accepts_all_mary_o_power_forms_only() {
        assert!(is_mary_o_form("mary_o"));
        assert!(is_mary_o_form("mary_o_tall"));
        assert!(is_mary_o_form("mary_o_fire"));
        assert!(!is_mary_o_form("sanic"));
    }
}

//! Per-sprite deep-dream shader experiment for the Puppy Slug enemy.
//!
//! The regular character sprite remains visible and authoritative: the normal
//! `animate_characters` system still advances its atlas frame, facing bit, and
//! hit tint. This module adds a child `Material2d` quad that samples the same
//! atlas frame and draws a vivid, semi-transparent surreal overlay on top. The
//! source sprite is never hidden, so a material/shader failure cannot erase the
//! enemy.

use bevy::{
    image::TextureAtlasLayout,
    prelude::*,
    reflect::TypePath,
    render::render_resource::AsBindGroup,
    shader::ShaderRef,
    sprite::Anchor,
    sprite_render::{AlphaMode2d, Material2d, Material2dPlugin, MeshMaterial2d},
};

use super::primitives::{FeatureVisual, PlayerVisual, PropVisual};
use crate::features::{ActorRuntime, EnemyArchetype, FeatureId};

const SHADER_ASSET_PATH: &str = "shaders/puppy_slug_deep_dream.wgsl";

/// Keep the local material strong enough to read over the original sprite.
const EFFECT_STRENGTH: f32 = 1.0;

/// Install the material plugin that backs the puppy-slug deep-dream overlay.
pub fn add_puppy_slug_deep_dream_material_plugin(app: &mut App) {
    app.add_plugins(Material2dPlugin::<PuppySlugDeepDreamMaterial>::default());
}

/// Custom material used by the one-off puppy-slug shader.
///
/// Bindings intentionally stay small and WebGL2-friendly:
///
/// - `uv_rect.xy` / `uv_rect.zw`: current atlas frame, normalized into the
///   loaded spritesheet texture.
/// - `control.x`: elapsed seconds.
/// - `control.y`: x-flip flag (0 or 1).
/// - `control.z`: effect strength.
/// - `control.w`: deterministic per-entity seed.
/// - `tint`: reserved for future hit-flash / attack tint mixing.
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct PuppySlugDeepDreamMaterial {
    #[uniform(0)]
    pub uv_rect: Vec4,
    #[uniform(1)]
    pub control: Vec4,
    #[uniform(2)]
    pub tint: Vec4,
    #[texture(3)]
    #[sampler(4)]
    pub color_texture: Handle<Image>,
}

impl Material2d for PuppySlugDeepDreamMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

/// Marker on the normal character-sprite entity that owns the overlay child.
#[derive(Component, Debug, Clone, Copy)]
pub struct PuppySlugDeepDreamSource {
    seed: f32,
}

/// Marker on the child mesh that runs the material shader.
#[derive(Component, Debug, Clone, Copy)]
pub struct PuppySlugDeepDreamOverlay;

/// Attach the deep-dream material child to every upgraded Puppy Slug sprite.
///
/// The `Without<PuppySlugDeepDreamSource>` filter keeps this idempotent. If a
/// room reload despawns the feature visual, Bevy removes the child with it and
/// the next spawn gets a fresh material handle.
pub fn attach_puppy_slug_deep_dream_overlays(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<PuppySlugDeepDreamMaterial>>,
    texture_layouts: Res<Assets<TextureAtlasLayout>>,
    images: Res<Assets<Image>>,
    actors: Query<(&FeatureId, &ActorRuntime)>,
    candidates: Query<
        (Entity, &FeatureVisual, &Sprite, Option<&Anchor>),
        (
            Without<PlayerVisual>,
            Without<PropVisual>,
            Without<PuppySlugDeepDreamSource>,
        ),
    >,
) {
    for (entity, visual, sprite, anchor) in &candidates {
        if !is_puppy_slug_feature(&visual.id, &actors) {
            continue;
        }
        let Some(render_size) = sprite.custom_size else {
            continue;
        };
        let Some(uv_rect) = current_sprite_uv_rect(sprite, &texture_layouts, &images) else {
            continue;
        };
        let seed = seed_from_id(&visual.id);
        let material = materials.add(PuppySlugDeepDreamMaterial {
            uv_rect,
            control: Vec4::new(0.0, flip_flag(sprite), EFFECT_STRENGTH, seed),
            tint: Vec4::ONE,
            color_texture: sprite.image.clone(),
        });
        let anchor_offset = anchor_to_mesh_offset(anchor, render_size);
        let mesh = meshes.add(Rectangle::default());
        commands
            .entity(entity)
            .insert(PuppySlugDeepDreamSource { seed })
            .with_children(|parent| {
                parent.spawn((
                    Mesh2d(mesh),
                    MeshMaterial2d(material),
                    Transform::from_translation(anchor_offset.extend(0.25))
                        .with_scale(render_size.extend(1.0)),
                    GlobalTransform::default(),
                    Visibility::Inherited,
                    InheritedVisibility::default(),
                    ViewVisibility::default(),
                    PuppySlugDeepDreamOverlay,
                    Name::new("Puppy Slug Deep Dream Overlay"),
                ));
            });
    }
}

/// Mirror the visible source sprite's current atlas frame into the overlay
/// material. This runs after `animate_characters`, so it sees the same atlas
/// index and facing flip that the normal sprite draws.
pub fn sync_puppy_slug_deep_dream_overlays(
    world_time: Res<crate::WorldTime>,
    mut elapsed: Local<f32>,
    texture_layouts: Res<Assets<TextureAtlasLayout>>,
    images: Res<Assets<Image>>,
    parents: Query<(
        &Sprite,
        Option<&Anchor>,
        &Children,
        &PuppySlugDeepDreamSource,
    )>,
    mut overlays: Query<
        (&MeshMaterial2d<PuppySlugDeepDreamMaterial>, &mut Transform),
        With<PuppySlugDeepDreamOverlay>,
    >,
    mut materials: ResMut<Assets<PuppySlugDeepDreamMaterial>>,
) {
    *elapsed += world_time.wall_dt();
    for (source_sprite, anchor, children, source) in &parents {
        let Some(render_size) = source_sprite.custom_size else {
            continue;
        };
        let Some(uv_rect) = current_sprite_uv_rect(source_sprite, &texture_layouts, &images) else {
            continue;
        };
        let flip = flip_flag(source_sprite);

        for child in children.iter() {
            let Ok((material_handle, mut transform)) = overlays.get_mut(child) else {
                continue;
            };
            transform.translation = anchor_to_mesh_offset(anchor, render_size).extend(0.25);
            transform.scale = render_size.extend(1.0);
            if let Some(material) = materials.get_mut(&material_handle.0) {
                material.uv_rect = uv_rect;
                material.control = Vec4::new(*elapsed, flip, EFFECT_STRENGTH, source.seed);
                material.color_texture = source_sprite.image.clone();
            }
        }
    }
}

fn is_puppy_slug_feature(id: &str, actors: &Query<(&FeatureId, &ActorRuntime)>) -> bool {
    actors.iter().any(|(feature_id, actor)| {
        if feature_id.as_str() != id {
            return false;
        }
        matches!(
            actor,
            ActorRuntime::Hostile(enemy) if enemy.archetype == EnemyArchetype::PuppySlug
        )
    })
}

fn current_sprite_uv_rect(
    sprite: &Sprite,
    texture_layouts: &Assets<TextureAtlasLayout>,
    images: &Assets<Image>,
) -> Option<Vec4> {
    let atlas = sprite.texture_atlas.as_ref()?;
    let layout = texture_layouts.get(&atlas.layout)?;
    let rect = layout.textures.get(atlas.index)?;
    let image = images.get(&sprite.image)?;
    let texture_size = image.texture_descriptor.size;
    let size = Vec2::new(
        texture_size.width.max(1) as f32,
        texture_size.height.max(1) as f32,
    );
    Some(Vec4::new(
        rect.min.x as f32 / size.x,
        rect.min.y as f32 / size.y,
        rect.max.x as f32 / size.x,
        rect.max.y as f32 / size.y,
    ))
}

fn anchor_to_mesh_offset(anchor: Option<&Anchor>, render_size: Vec2) -> Vec2 {
    // Sprite anchors are normalized around the sprite centre. A centred mesh
    // child needs the opposite local translation to line up with the sprite's
    // anchored draw origin.
    let anchor = anchor.map(|a| a.0).unwrap_or(Vec2::ZERO);
    -anchor * render_size
}

fn flip_flag(sprite: &Sprite) -> f32 {
    if sprite.flip_x {
        1.0
    } else {
        0.0
    }
}

fn seed_from_id(id: &str) -> f32 {
    let mut hash: u32 = 0x811c9dc5;
    for byte in id.bytes() {
        hash ^= byte as u32;
        hash = hash.wrapping_mul(0x01000193);
    }
    (hash & 0x00ff_ffff) as f32 / 16_777_216.0
}

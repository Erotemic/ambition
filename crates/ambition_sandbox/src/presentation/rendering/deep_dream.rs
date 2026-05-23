//! Per-sprite deep-dream shader experiment for the Puppy Slug enemy.
//!
//! The regular character sprite remains the authoritative animation source:
//! `animate_characters` keeps advancing its atlas frame and facing bit. This
//! module hides only that source sprite's pixels and mirrors the current atlas
//! frame into a child `Material2d` quad. The shader then samples the same
//! spritesheet frame, uses the frame alpha as a hard mask, and performs all of
//! the surreal color/fractal/melt work inside the visible puppy-slug silhouette.
//!
//! Keeping this as an overlay child makes the experiment easy to delete if the
//! look is wrong, and it avoids touching the generic character animator path.

use bevy::{
    prelude::*,
    reflect::TypePath,
    render::render_resource::AsBindGroup,
    shader::ShaderRef,
    sprite::Anchor,
    sprite_render::{AlphaMode2d, Material2d, Material2dPlugin, MeshMaterial2d},
};

use crate::features::{ActorRuntime, EnemyArchetype, FeatureId};
use crate::presentation::character_sprites::CharacterAnimator;
use super::primitives::{FeatureVisual, PlayerVisual, PropVisual};

const SHADER_ASSET_PATH: &str = "shaders/puppy_slug_deep_dream.wgsl";

/// Alpha applied to the underlying character sprite while the deep-
/// dream overlay child is alive. Originally `0.0` (source fully
/// hidden, overlay assumed to take over) but that made every slug
/// invisible whenever the overlay material itself failed to draw —
/// e.g., a silent WGSL compile failure, a render-graph wiring issue,
/// or an asset that hadn't loaded yet. Set to `1.0` so the source
/// sprite stays visible and the overlay layers on top of it: the
/// shader becomes additive flavor rather than a load-bearing
/// renderer. If the shader works you see slug + dream blend; if it
/// silently fails you see the regular slug.
///
/// TODO: drop back toward `0.0` once the overlay path is confirmed
/// to render in-game and we have a way to detect material-load
/// failures rather than fail silent. The shader was authored to
/// be authoritative (its `discard` on transparent atlas padding
/// expects to be the only renderer); double-sampling the source
/// underneath will produce mild double-vision where both layers
/// overlap.
const SOURCE_ALPHA_WHILE_DREAMING: f32 = 1.0;

/// Install the material plugin that backs the puppy-slug deep-dream overlay.
/// Visible builds call this from `PresentationVisualAnimationPlugin`; headless
/// builds never touch the sprite-render material pipeline.
pub fn add_puppy_slug_deep_dream_material_plugin(app: &mut App) {
    app.add_plugins(Material2dPlugin::<PuppySlugDeepDreamMaterial>::default());
}

/// Custom material used by the one-off puppy-slug shader.
///
/// Bindings intentionally stay small and WebGL2-friendly:
///
/// - `uv_rect.xy` / `uv_rect.zw`: current atlas frame, normalized into the
///   spritesheet texture.
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

/// Marker on the normal character-sprite entity whose pixels are replaced by
/// the deep-dream child overlay.
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
    actors: Query<(&FeatureId, &ActorRuntime)>,
    candidates: Query<
        (Entity, &FeatureVisual, &Sprite, Option<&Anchor>, &CharacterAnimator),
        (
            Without<PlayerVisual>,
            Without<PropVisual>,
            Without<PuppySlugDeepDreamSource>,
        ),
    >,
) {
    for (entity, visual, sprite, anchor, animator) in &candidates {
        if !is_puppy_slug_feature(&visual.id, &actors) {
            continue;
        }
        let Some(render_size) = sprite.custom_size else {
            continue;
        };
        let uv_rect = current_sprite_uv_rect(sprite, animator);
        let material = materials.add(PuppySlugDeepDreamMaterial {
            uv_rect,
            control: Vec4::new(0.0, flip_flag(sprite), 1.0, seed_from_id(&visual.id)),
            tint: Vec4::ONE,
            color_texture: sprite.image.clone(),
        });
        let anchor_offset = anchor_to_mesh_offset(anchor, render_size);
        let mesh = meshes.add(Rectangle::default());
        commands
            .entity(entity)
            .insert(PuppySlugDeepDreamSource {
                seed: seed_from_id(&visual.id),
            })
            .with_children(|parent| {
                parent.spawn((
                    Mesh2d(mesh),
                    MeshMaterial2d(material),
                    Transform::from_translation(anchor_offset.extend(0.01))
                        .with_scale(render_size.extend(1.0)),
                    PuppySlugDeepDreamOverlay,
                    Name::new("Puppy Slug Deep Dream Overlay"),
                ));
            });
    }
}

/// Mirror the hidden source sprite's current atlas frame into the overlay
/// material. This runs after `animate_characters`, so it sees the same atlas
/// index and facing flip that the normal sprite would have drawn.
pub fn sync_puppy_slug_deep_dream_overlays(
    world_time: Res<crate::WorldTime>,
    mut elapsed: Local<f32>,
    mut parents: Query<(
        &mut Sprite,
        Option<&Anchor>,
        &Children,
        &CharacterAnimator,
        &PuppySlugDeepDreamSource,
    )>,
    mut overlays: Query<
        (&MeshMaterial2d<PuppySlugDeepDreamMaterial>, &mut Transform),
        With<PuppySlugDeepDreamOverlay>,
    >,
    mut materials: ResMut<Assets<PuppySlugDeepDreamMaterial>>,
) {
    *elapsed += world_time.wall_dt();
    for (mut source_sprite, anchor, children, animator, source) in &mut parents {
        let Some(render_size) = source_sprite.custom_size else {
            continue;
        };
        let uv_rect = current_sprite_uv_rect(&source_sprite, animator);
        let flip = flip_flag(&source_sprite);

        // Hide the ordinary sprite AFTER `animate_characters` has used it as
        // the animation state carrier. The material child draws the visible
        // pixels instead.
        source_sprite.color = Color::srgba(1.0, 1.0, 1.0, SOURCE_ALPHA_WHILE_DREAMING);

        for child in children.iter() {
            let Ok((material_handle, mut transform)) = overlays.get_mut(child) else {
                continue;
            };
            transform.translation = anchor_to_mesh_offset(anchor, render_size).extend(0.01);
            transform.scale = render_size.extend(1.0);
            if let Some(material) = materials.get_mut(&material_handle.0) {
                material.uv_rect = uv_rect;
                material.control = Vec4::new(*elapsed, flip, 1.0, source.seed);
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

fn current_sprite_uv_rect(sprite: &Sprite, animator: &CharacterAnimator) -> Vec4 {
    let Some(atlas) = sprite.texture_atlas.as_ref() else {
        return Vec4::new(0.0, 0.0, 1.0, 1.0);
    };
    let Some(rect) = animator.spec.texture_rect_for_flat_index(atlas.index) else {
        return Vec4::new(0.0, 0.0, 1.0, 1.0);
    };
    let size = animator.spec.atlas_texture_size().as_vec2().max(Vec2::ONE);
    Vec4::new(
        rect.min.x as f32 / size.x,
        rect.min.y as f32 / size.y,
        rect.max.x as f32 / size.x,
        rect.max.y as f32 / size.y,
    )
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

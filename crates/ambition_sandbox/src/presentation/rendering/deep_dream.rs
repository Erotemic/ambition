//! Per-sprite deep-dream shader experiment for the Puppy Slug enemy.
//!
//! The regular character sprite remains visible and authoritative: the normal
//! `animate_characters` system still advances its atlas frame, facing bit, and
//! hit tint. This module adds a separate world-space `Material2d` quad that
//! samples the same atlas frame and draws a vivid, semi-transparent surreal
//! overlay on top. The source sprite is never hidden, so a material/shader
//! failure cannot erase the enemy.
//!
//! v6 deliberately uses a sibling world-space mesh instead of a child. The
//! first attempt used a child quad, which is elegant but harder to debug when
//! hierarchy visibility, anchor math, or atlas UVs are off. A sibling that is
//! copied from the source sprite every frame is closer to a tiny local
//! post-process: if the material pipeline is alive, it should be visible.

use bevy::{
    image::TextureAtlasLayout,
    prelude::*,
    reflect::TypePath,
    render::render_resource::AsBindGroup,
    shader::ShaderRef,
    sprite::Anchor,
    sprite_render::{AlphaMode2d, Material2d, Material2dPlugin, MeshMaterial2d},
};

use super::primitives::{FeatureVisual, PlayerVisual, PropVisual, RoomVisual};
use crate::features::{ActorRuntime, EnemyArchetype, FeatureId};

const SHADER_ASSET_PATH: &str = "shaders/puppy_slug_deep_dream.wgsl";

/// Keep the local material strong enough to read over the original sprite.
const EFFECT_STRENGTH: f32 = 1.0;

/// Draw the local overlay clearly in front of the source sprite. This is still
/// below the foreground/player debug layers in normal rooms, but large enough
/// to avoid same-z ordering surprises in the transparent 2D phase.
const LOCAL_OVERLAY_Z_BIAS: f32 = 0.9;


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

/// Marker on the normal character-sprite entity that owns the overlay sibling.
#[derive(Component, Debug, Clone, Copy)]
pub struct PuppySlugDeepDreamSource {
    overlay: Entity,
    seed: f32,
}

/// Marker on the sibling mesh that runs the material shader.
#[derive(Component, Debug, Clone, Copy)]
pub struct PuppySlugDeepDreamOverlay {
    source: Entity,
}

/// Attach the deep-dream material sibling to every upgraded Puppy Slug sprite.
///
/// The `Without<PuppySlugDeepDreamSource>` filter keeps this idempotent. The
/// overlay is a sibling, not a child: `sync_puppy_slug_deep_dream_overlays`
/// copies the source transform/anchor into world space each frame.
pub fn attach_puppy_slug_deep_dream_overlays(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<PuppySlugDeepDreamMaterial>>,
    texture_layouts: Res<Assets<TextureAtlasLayout>>,
    images: Res<Assets<Image>>,
    actors: Query<(&FeatureId, &ActorRuntime)>,
    candidates: Query<
        (Entity, &FeatureVisual, &Transform, &Sprite, Option<&Anchor>),
        (
            Without<PlayerVisual>,
            Without<PropVisual>,
            Without<PuppySlugDeepDreamSource>,
        ),
    >,
) {
    for (source_entity, visual, transform, sprite, anchor) in &candidates {
        let Some(actor_seed) = puppy_slug_seed(&visual.id, &actors) else {
            continue;
        };
        let Some(render_size) = sprite.custom_size else {
            continue;
        };
        let Some(uv_rect) = current_sprite_uv_rect(sprite, &texture_layouts, &images) else {
            continue;
        };

        let seed = actor_seed + seed_from_id(&visual.id) * 0.37;
        let material = materials.add(PuppySlugDeepDreamMaterial {
            uv_rect,
            control: Vec4::new(0.0, flip_flag(sprite), EFFECT_STRENGTH, seed),
            tint: Vec4::ONE,
            color_texture: sprite.image.clone(),
        });
        let mesh = meshes.add(Rectangle::default());
        let overlay_transform = overlay_transform_from_source(transform, anchor, render_size);
        // Let Bevy's required-component machinery insert Transform's
        // GlobalTransform and Visibility's InheritedVisibility +
        // ViewVisibility with their proper defaults. Inserting them
        // explicitly here was a stealth bug: `InheritedVisibility::default()`
        // is `Self(false)` (HIDDEN), and once a tuple-insert assigns it,
        // the visibility propagator's PostUpdate pass on the *same* tick
        // can't override it before the render extraction reads
        // `view_visibility.get()` and skips the entity. With the explicit
        // default removed, the auto-inserted InheritedVisibility starts
        // unset and the propagator computes it from `Visibility::Visible`
        // immediately. Same reasoning for ViewVisibility.
        let overlay_entity = commands
            .spawn((
                Mesh2d(mesh),
                MeshMaterial2d(material),
                overlay_transform,
                Visibility::Visible,
                PuppySlugDeepDreamOverlay {
                    source: source_entity,
                },
                // Room cleanup should remove this sibling alongside the regular
                // feature visuals. The explicit cleanup system below also handles
                // encounter-mob despawns before a room unload.
                RoomVisual,
                Name::new(format!("Puppy Slug Deep Dream Overlay: {}", visual.id)),
            ))
            .id();
        commands
            .entity(source_entity)
            .insert(PuppySlugDeepDreamSource {
                overlay: overlay_entity,
                seed,
            });
    }
}

/// Mirror the visible source sprite's current atlas frame into the overlay
/// material. This runs after `animate_characters`, so it sees the same atlas
/// index and facing flip that the normal sprite draws.
pub fn sync_puppy_slug_deep_dream_overlays(
    world_time: Res<crate::WorldTime>,
    mut elapsed: Local<f32>,
    developer_tools: Res<crate::dev::dev_tools::DeveloperTools>,
    texture_layouts: Res<Assets<TextureAtlasLayout>>,
    images: Res<Assets<Image>>,
    mut sources: Query<
        (
            Entity,
            &Transform,
            &mut Sprite,
            Option<&Anchor>,
            &PuppySlugDeepDreamSource,
            Option<&Visibility>,
        ),
        Without<PuppySlugDeepDreamOverlay>,
    >,
    mut overlays: Query<(
        Entity,
        &mut Transform,
        &mut Visibility,
        &MeshMaterial2d<PuppySlugDeepDreamMaterial>,
        &PuppySlugDeepDreamOverlay,
    )>,
    mut materials: ResMut<Assets<PuppySlugDeepDreamMaterial>>,
) {
    let dt = world_time.wall_dt();
    *elapsed += dt;
    let disabled = developer_tools.disable_puppy_slug_dream;

    for (source_entity, source_transform, mut source_sprite, anchor, source, source_visibility) in
        &mut sources
    {
        let Some(render_size) = source_sprite.custom_size else {
            continue;
        };
        let Some(uv_rect) = current_sprite_uv_rect(&source_sprite, &texture_layouts, &images)
        else {
            continue;
        };
        let flip = flip_flag(&source_sprite);
        let source_visible = !matches!(source_visibility, Some(v) if *v == Visibility::Hidden);

        // Rainbow tint fail-safe (kept from `05e5feb`): even when the
        // sibling deep-dream material draws on top, multiplying the source
        // sprite's color through an HSV cycle keeps the slug visibly
        // "surreal" if the shader doesn't fully cover. Multiplies against
        // the existing color (rather than overwriting) so the upstream
        // hit-flash tint from `sync_visuals` still reads through —
        // a red flash multiplied by current HSV becomes a tinted red
        // rather than getting clobbered to plain rainbow.
        //
        // Skip the source-color recoloring while the disable toggle
        // is on so the slug renders in its native palette during A/B
        // tests against the mirror-duplication artifact.
        if !disabled {
            let hue = (*elapsed * 0.7 + source.seed * 0.91).rem_euclid(1.0);
            let tint_rgb = hsv_to_rgb(hue, 0.78, 1.0);
            let existing = source_sprite.color.to_srgba();
            source_sprite.color = Color::srgba(
                existing.red * tint_rgb.x,
                existing.green * tint_rgb.y,
                existing.blue * tint_rgb.z,
                existing.alpha,
            );
        } else {
            // Restore opaque white so the source sprite renders cleanly
            // without the carry-over rainbow multiplier from the
            // previous frame.
            source_sprite.color = Color::WHITE;
        }

        let Ok((
            overlay_entity,
            mut overlay_transform,
            mut overlay_visibility,
            material_handle,
            overlay,
        )) = overlays.get_mut(source.overlay)
        else {
            continue;
        };
        if overlay.source != source_entity {
            continue;
        }
        // Hide the sibling mesh entirely when the disable toggle is
        // on. The slug still renders via its base sprite; only the
        // deep-dream sibling stops drawing.
        *overlay_visibility = if disabled || !source_visible {
            Visibility::Hidden
        } else {
            Visibility::Visible
        };
        *overlay_transform = overlay_transform_from_source(source_transform, anchor, render_size);
        if let Some(material) = materials.get_mut(&material_handle.0) {
            material.uv_rect = uv_rect;
            material.control = Vec4::new(*elapsed, flip, EFFECT_STRENGTH, source.seed);
            material.color_texture = source_sprite.image.clone();
        }
        let _ = (overlay_entity, source_visible);
    }
}

/// Remove orphaned sibling overlays if their source entity despawns before the
/// room-scoped cleanup runs.
pub fn cleanup_puppy_slug_deep_dream_overlays(
    mut commands: Commands,
    sources: Query<(), With<PuppySlugDeepDreamSource>>,
    overlays: Query<(Entity, &PuppySlugDeepDreamOverlay)>,
) {
    for (overlay_entity, overlay) in &overlays {
        if sources.get(overlay.source).is_err() {
            commands.entity(overlay_entity).despawn();
        }
    }
}

fn puppy_slug_seed(id: &str, actors: &Query<(&FeatureId, &ActorRuntime)>) -> Option<f32> {
    actors.iter().find_map(|(feature_id, actor)| {
        if feature_id.as_str() != id {
            return None;
        }
        let (name, archetype) = match actor {
            ActorRuntime::Hostile(enemy) => (enemy.name.as_str(), Some(enemy.archetype)),
            ActorRuntime::Peaceful(npc) => (npc.name.as_str(), None),
        };
        let name_lc = name.to_ascii_lowercase();
        let is_slug = archetype == Some(EnemyArchetype::PuppySlug)
            || name_lc.contains("puppy")
            || name_lc.contains("slug");
        if is_slug {
            Some(seed_from_id(name) * 0.63 + archetype_seed(archetype))
        } else {
            None
        }
    })
}

fn archetype_seed(archetype: Option<EnemyArchetype>) -> f32 {
    match archetype {
        Some(EnemyArchetype::PuppySlug) => 0.271828,
        _ => 0.0,
    }
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

fn overlay_transform_from_source(
    source: &Transform,
    anchor: Option<&Anchor>,
    render_size: Vec2,
) -> Transform {
    let anchor_offset = anchor_to_mesh_offset(anchor, render_size);
    let world_offset = source.rotation.mul_vec3(anchor_offset.extend(0.0));
    let mut transform = *source;
    transform.translation += world_offset;
    transform.translation.z += LOCAL_OVERLAY_Z_BIAS;
    transform.scale = render_size.extend(1.0);
    transform
}

fn anchor_to_mesh_offset(anchor: Option<&Anchor>, render_size: Vec2) -> Vec2 {
    // Sprite anchors are normalized around the sprite centre. A centred mesh
    // sibling needs the opposite world-space translation to line up with the
    // sprite's anchored draw origin.
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

/// Cheap HSV → linear RGB conversion. Used by the rainbow-tint
/// fail-safe in `sync_puppy_slug_deep_dream_overlays`; doesn't need
/// to be color-space-accurate, just lively.
fn hsv_to_rgb(h: f32, s: f32, v: f32) -> Vec3 {
    let h = h.rem_euclid(1.0);
    let i = (h * 6.0).floor() as i32;
    let f = h * 6.0 - i as f32;
    let p = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let t = v * (1.0 - s * (1.0 - f));
    match i.rem_euclid(6) {
        0 => Vec3::new(v, t, p),
        1 => Vec3::new(q, v, p),
        2 => Vec3::new(p, v, t),
        3 => Vec3::new(p, q, v),
        4 => Vec3::new(t, p, v),
        _ => Vec3::new(v, p, q),
    }
}

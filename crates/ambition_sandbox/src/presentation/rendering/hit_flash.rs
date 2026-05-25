//! White-flash hit feedback for character sprites.
//!
//! Every character that can take damage (player, enemies, NPCs,
//! bosses) flashes pure white for the duration of its `hit_flash`
//! timer. The effect is implemented as a sibling `Material2d` mesh
//! drawn on top of the source sprite — same pattern as the
//! [`super::deep_dream`] puppy-slug overlay, but with a tiny shader
//! that just outputs `vec4(1, 1, 1, sample.alpha * intensity)`. When
//! the timer expires the overlay is hidden (no GPU work) and the
//! source sprite renders normally.
//!
//! Source-of-truth for the flash timer:
//!
//! - **Enemy / NPC**: [`crate::features::ActorRuntime`] inner
//!   `hit_flash: f32` (seconds remaining).
//! - **Boss**: [`crate::features::BossFeature::boss::hit_flash`].
//! - **Player**: [`crate::player::PlayerCombatState::flash_timer`].
//!
//! Replaces the pink multiplicative tint that
//! [`super::actors::animate_characters`] and
//! [`super::actors::animate_player`] used to set on `sprite.color`.

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
use crate::features::{ActorRuntime, BossFeature, FeatureId};

const SHADER_ASSET_PATH: &str = "shaders/hit_flash.wgsl";

/// Hold the flash at full intensity for the first 80% of the timer,
/// then fade smoothly to zero. Without this the flash ends in a
/// sudden cut that reads as a missing frame; the fade keeps the
/// transition readable at the cost of one extra frame of bright
/// pixels.
const FLASH_HOLD_FRACTION: f32 = 0.80;

/// Maximum hit_flash duration the codebase issues (enemy archetypes
/// use 0.18–0.24 seconds). Used to normalize the timer into the
/// [0, 1] range the shader expects. Values larger than this just
/// saturate at full white — they don't visually clip, just hold
/// longer.
const REFERENCE_FLASH_SECONDS: f32 = 0.24;

/// Z bias for the overlay mesh — sits in front of the source sprite
/// (and the gnu-ton hands child sprite) but behind the gradient-lane
/// telegraph quad which lives at +1.0 of the boss z.
const FLASH_OVERLAY_Z_BIAS: f32 = 0.45;

/// Install the material plugin behind the hit-flash overlay.
pub fn add_hit_flash_material_plugin(app: &mut App) {
    app.add_plugins(Material2dPlugin::<HitFlashMaterial>::default());
}

/// Material2d backing the white-silhouette overlay.
///
/// Bindings mirror the puppy-slug deep-dream material so the shader
/// driver can re-use the same WebGL2-friendly layout (vec4 uniforms,
/// no struct UBOs).
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct HitFlashMaterial {
    /// Current atlas frame as a UV rect on the loaded spritesheet.
    /// `(min.x, min.y, max.x, max.y)` normalized.
    #[uniform(0)]
    pub uv_rect: Vec4,
    /// `(intensity, flip_x, _, _)`. `intensity` is the shader's
    /// gate: 0.0 → discard everything; 1.0 → full white silhouette.
    #[uniform(1)]
    pub control: Vec4,
    #[texture(2)]
    #[sampler(3)]
    pub color_texture: Handle<Image>,
}

impl Material2d for HitFlashMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

/// Marker placed on the source sprite entity once an overlay sibling
/// has been spawned for it. Stores the overlay's entity id so we can
/// despawn / re-sync it without scanning.
#[derive(Component, Debug, Clone, Copy)]
pub struct HitFlashSource {
    overlay: Entity,
}

impl HitFlashSource {
    pub fn overlay(&self) -> Entity {
        self.overlay
    }
}

/// Marker on the sibling mesh that runs the hit-flash material.
#[derive(Component, Debug, Clone, Copy)]
pub struct HitFlashOverlay {
    source: Entity,
}

/// Attach a flash overlay to every textured character sprite that
/// doesn't already have one. Gates on `FeatureVisual` / `PlayerVisual`
/// presence so prop visuals and one-shot VFX don't pick up the
/// overlay accidentally.
pub fn attach_hit_flash_overlays(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<HitFlashMaterial>>,
    texture_layouts: Res<Assets<TextureAtlasLayout>>,
    images: Res<Assets<Image>>,
    candidates: Query<
        (
            Entity,
            &Transform,
            &Sprite,
            Option<&Anchor>,
            Option<&FeatureVisual>,
            Option<&PlayerVisual>,
        ),
        (Without<HitFlashSource>, Without<PropVisual>),
    >,
) {
    for (source_entity, transform, sprite, anchor, feature, player) in &candidates {
        // Eligibility: a textured sprite (atlas OR plain image) that
        // belongs to a character — FeatureVisual covers
        // enemies/NPCs/bosses, PlayerVisual covers the player. Props
        // are excluded by the query filter.
        if feature.is_none() && player.is_none() {
            continue;
        }
        let Some(render_size) = sprite.custom_size else {
            // Sprites without `custom_size` haven't been sized by
            // the render pipeline yet (initial spawn frame). Skip
            // and try again next frame — the upgrade systems set
            // custom_size on the next tick.
            continue;
        };
        let Some(uv_rect) = current_sprite_uv_rect(sprite, &texture_layouts, &images) else {
            // Texture / atlas not loaded yet; try again next frame.
            continue;
        };

        let material = materials.add(HitFlashMaterial {
            uv_rect,
            // Start hidden — `intensity = 0.0` causes the shader to
            // discard every fragment. The sync system bumps this
            // up whenever the source's hit_flash timer is positive.
            control: Vec4::new(0.0, flip_flag(sprite), 0.0, 0.0),
            color_texture: sprite.image.clone(),
        });
        let mesh = meshes.add(Rectangle::default());
        let overlay_transform = overlay_transform_from_source(transform, anchor, render_size);
        let overlay_entity = commands
            .spawn((
                Mesh2d(mesh),
                MeshMaterial2d(material),
                overlay_transform,
                // Hidden by default — the sync system flips this to
                // `Visible` for the frames where `intensity > 0`.
                Visibility::Hidden,
                HitFlashOverlay {
                    source: source_entity,
                },
                RoomVisual,
                Name::new("HitFlash Overlay"),
            ))
            .id();
        commands.entity(source_entity).insert(HitFlashSource {
            overlay: overlay_entity,
        });
    }
}

/// Mirror the source sprite's atlas frame / facing / world transform
/// into the overlay material and toggle visibility based on the
/// source's current `hit_flash` timer.
pub fn sync_hit_flash_overlays(
    texture_layouts: Res<Assets<TextureAtlasLayout>>,
    images: Res<Assets<Image>>,
    actors: Query<(&FeatureId, &ActorRuntime)>,
    bosses: Query<(&FeatureId, &BossFeature)>,
    player_state: Query<&crate::player::PlayerCombatState, crate::player::PrimaryPlayerOnly>,
    sources: Query<
        (
            Entity,
            &Transform,
            &Sprite,
            Option<&Anchor>,
            Option<&FeatureVisual>,
            Option<&PlayerVisual>,
            &HitFlashSource,
            Option<&Visibility>,
        ),
        Without<HitFlashOverlay>,
    >,
    mut overlays: Query<(
        &mut Transform,
        &mut Visibility,
        &MeshMaterial2d<HitFlashMaterial>,
        &HitFlashOverlay,
    )>,
    mut materials: ResMut<Assets<HitFlashMaterial>>,
) {
    for (
        source_entity,
        source_transform,
        source_sprite,
        anchor,
        feature,
        player,
        source,
        source_visibility,
    ) in &sources
    {
        let Some(render_size) = source_sprite.custom_size else {
            continue;
        };
        let Some(uv_rect) = current_sprite_uv_rect(source_sprite, &texture_layouts, &images) else {
            continue;
        };
        let flip = flip_flag(source_sprite);

        // Single dispatch covers every character type the universal
        // Brain/ActorControl architecture knows about — player, NPC,
        // enemy, boss. Each routes through a different per-entity
        // storage today (PlayerCombatState vs ActorRuntime vs
        // BossFeature) but they all converge on one shader uniform
        // through this lookup. A future refactor that unifies them
        // into a single `HitFlash` component can collapse this to
        // one query without changing the overlay sync.
        let hit_flash_secs =
            hit_flash_secs_for_source(feature, player, &actors, &bosses, &player_state);
        let intensity = hit_flash_secs
            .map(normalize_hit_flash)
            .unwrap_or(0.0);
        let source_visible = !matches!(source_visibility, Some(v) if *v == Visibility::Hidden);

        let Ok((mut overlay_transform, mut overlay_visibility, material_handle, overlay)) =
            overlays.get_mut(source.overlay)
        else {
            continue;
        };
        if overlay.source != source_entity {
            continue;
        }
        // Hide the overlay when intensity drops to zero. Bevy culls
        // hidden meshes from rendering, so we don't pay GPU cost
        // for resting characters.
        *overlay_visibility = if source_visible && intensity > 0.001 {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        *overlay_transform = overlay_transform_from_source(source_transform, anchor, render_size);
        if let Some(material) = materials.get_mut(&material_handle.0) {
            material.uv_rect = uv_rect;
            material.control = Vec4::new(intensity, flip, 0.0, 0.0);
            material.color_texture = source_sprite.image.clone();
        }
    }
}

/// Remove orphan overlays whose source entity despawned. Mirrors the
/// deep-dream cleanup pass — without it a despawn between
/// FeatureViewSync and PresentationVisualAnimationPlugin can leave
/// the white silhouette frozen mid-air for one frame on the next
/// scene load.
pub fn cleanup_hit_flash_overlays(
    mut commands: Commands,
    sources: Query<(), With<HitFlashSource>>,
    overlays: Query<(Entity, &HitFlashOverlay)>,
) {
    for (overlay_entity, overlay) in &overlays {
        if sources.get(overlay.source).is_err() {
            commands.entity(overlay_entity).despawn();
        }
    }
}

/// Unified hit_flash seconds dispatch.
///
/// One entry point for every character type the
/// universal-Brain unification covers — caller doesn't need
/// to know whether the source is a player, enemy, NPC, or
/// boss. All four arms read the per-tick countdown that
/// damage code already maintains, so adding hit feedback to
/// a new actor type is just "give it one of these timers and
/// the overlay attaches itself".
///
/// Source-of-truth per type:
///
/// | type | timer storage | set by damage |
/// |------|---------------|---------------|
/// | player | `PlayerCombatState::flash_timer` | `world_flow` damage paths |
/// | enemy  | `EnemyRuntime::hit_flash` (via `ActorRuntime::Hostile`) | `enemies::apply_damage_at` |
/// | NPC    | `NpcRuntime::hit_flash` (via `ActorRuntime::Peaceful`)   | NPC damage paths |
/// | boss   | `BossRuntime::hit_flash` (via `BossFeature.boss`)        | boss damage paths |
fn hit_flash_secs_for_source(
    feature: Option<&FeatureVisual>,
    player: Option<&PlayerVisual>,
    actors: &Query<(&FeatureId, &ActorRuntime)>,
    bosses: &Query<(&FeatureId, &BossFeature)>,
    player_state: &Query<&crate::player::PlayerCombatState, crate::player::PrimaryPlayerOnly>,
) -> Option<f32> {
    // Player path: the entity that carries `PlayerVisual` is the
    // same one that carries `PlayerCombatState`, so the
    // `PrimaryPlayerOnly` filter picks up the only matching state.
    if player.is_some() {
        return player_state.single().ok().map(|state| state.flash_timer);
    }
    let visual = feature?;
    let id = visual.id.as_str();
    // Feature path: cross-reference the visual entity's id against
    // the sim entity's `FeatureId`. Tries the actor list first
    // (enemies + NPCs share `ActorRuntime`), then bosses.
    if let Some(secs) = actors.iter().find_map(|(feature_id, actor)| {
        if feature_id.as_str() != id {
            return None;
        }
        match actor {
            ActorRuntime::Hostile(enemy) => Some(enemy.hit_flash),
            ActorRuntime::Peaceful(npc) => Some(npc.hit_flash),
        }
    }) {
        return Some(secs);
    }
    bosses.iter().find_map(|(feature_id, feature)| {
        if feature_id.as_str() != id {
            return None;
        }
        Some(feature.boss.hit_flash)
    })
}

/// Map raw seconds-remaining into a [0, 1] intensity. Holds at 1.0
/// for the first 80% of `REFERENCE_FLASH_SECONDS`, then ramps
/// linearly to 0 over the last 20%. Above `REFERENCE_FLASH_SECONDS`
/// stays clamped at 1.0; at or below zero stays at 0.0.
fn normalize_hit_flash(seconds: f32) -> f32 {
    if seconds <= 0.0 {
        return 0.0;
    }
    let fade_end = REFERENCE_FLASH_SECONDS * (1.0 - FLASH_HOLD_FRACTION);
    if seconds >= fade_end {
        1.0
    } else {
        (seconds / fade_end).clamp(0.0, 1.0)
    }
}

fn current_sprite_uv_rect(
    sprite: &Sprite,
    texture_layouts: &Assets<TextureAtlasLayout>,
    images: &Assets<Image>,
) -> Option<Vec4> {
    let image = images.get(&sprite.image)?;
    let texture_size = image.texture_descriptor.size;
    let size = Vec2::new(
        texture_size.width.max(1) as f32,
        texture_size.height.max(1) as f32,
    );
    if let Some(atlas) = sprite.texture_atlas.as_ref() {
        let layout = texture_layouts.get(&atlas.layout)?;
        let rect = layout.textures.get(atlas.index)?;
        Some(Vec4::new(
            rect.min.x as f32 / size.x,
            rect.min.y as f32 / size.y,
            rect.max.x as f32 / size.x,
            rect.max.y as f32 / size.y,
        ))
    } else {
        // Plain-image sprite: the whole texture is the "frame".
        Some(Vec4::new(0.0, 0.0, 1.0, 1.0))
    }
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
    transform.translation.z += FLASH_OVERLAY_Z_BIAS;
    transform.scale = render_size.extend(1.0);
    transform
}

fn anchor_to_mesh_offset(anchor: Option<&Anchor>, render_size: Vec2) -> Vec2 {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Above the reference duration: full intensity.
    #[test]
    fn normalize_above_reference_saturates() {
        assert_eq!(normalize_hit_flash(0.5), 1.0);
        assert_eq!(normalize_hit_flash(REFERENCE_FLASH_SECONDS), 1.0);
    }

    /// At and below zero: dark.
    #[test]
    fn normalize_at_or_below_zero_is_dark() {
        assert_eq!(normalize_hit_flash(0.0), 0.0);
        assert_eq!(normalize_hit_flash(-0.1), 0.0);
    }

    /// Inside the fade window the value scales linearly.
    #[test]
    fn normalize_fades_in_final_window() {
        let fade_end = REFERENCE_FLASH_SECONDS * (1.0 - FLASH_HOLD_FRACTION);
        let mid = fade_end * 0.5;
        let intensity = normalize_hit_flash(mid);
        assert!(
            (intensity - 0.5).abs() < 1e-3,
            "expected ~0.5 at fade midpoint; got {intensity}",
        );
    }

    /// Above the fade window but below the reference: full white.
    #[test]
    fn normalize_in_hold_window_full_intensity() {
        let fade_end = REFERENCE_FLASH_SECONDS * (1.0 - FLASH_HOLD_FRACTION);
        let between = (fade_end + REFERENCE_FLASH_SECONDS) * 0.5;
        assert_eq!(normalize_hit_flash(between), 1.0);
    }
}

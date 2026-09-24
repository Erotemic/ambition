//! Post-sync visual overlays / dev sprite overrides: hide-sprites & placeholder
//! art toggles plus the gradient-lane debug visual.

use bevy::math::Vec2 as BVec2;
use bevy::prelude::*;

use crate::rendering::primitives::{feature_color, FeatureVisual, PlayerVisual};
use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::config::world_to_bevy;
use ambition_platformer2d_shared_tangle::feature_kind::FeatureVisualKind;
use ambition_platformer2d_shared_tangle::lifecycle::{
    ActiveSessionScope, SessionSpawnScope, SpawnSessionScopedExt,
};
use ambition_sim_view::FeatureViewIndex;

/// When `DeveloperTools::hide_sprites` is on, force every `Sprite` entity to
/// `Hidden` so only gizmo outlines show. When it turns off, restore every
/// sprite to `Inherited` once, on the falling edge. Writing `Inherited` every
/// frame would undo legitimate `Hidden` writes from other systems (collected
/// pickups, the morph ball, a morphed player) and make them flicker. UI uses
/// `Node`/`ImageNode`, not `Sprite`, so HUD and menus are unaffected.
pub fn apply_hide_sprites_override(
    developer_tools: Res<ambition_dev_tools::dev_tools::DeveloperTools>,
    mut prev_active: Local<bool>,
    mut sprites: Query<&mut Visibility, With<Sprite>>,
) {
    let active = effective_hide_sprites(&developer_tools);
    if active {
        for mut vis in sprites.iter_mut() {
            if *vis != Visibility::Hidden {
                *vis = Visibility::Hidden;
            }
        }
    } else if *prev_active {
        for mut vis in sprites.iter_mut() {
            if *vis != Visibility::Inherited {
                *vis = Visibility::Inherited;
            }
        }
    }
    *prev_active = active;
}

fn effective_hide_sprites(developer_tools: &ambition_dev_tools::dev_tools::DeveloperTools) -> bool {
    // Placeholder art is a visible debug-art mode. If a persisted or
    // inspector state has both flags on, keep placeholders visible.
    developer_tools.hide_sprites && !developer_tools.placeholder_sprites
}

// =================================================================
// Gradient Sentinel: HazardColumn vertical-column visual
// =================================================================
//
// The HazardColumn boss attack is a tall hazard column at the boss x.
// `volumes_for_profile` gives the damage AABB; this system draws a visible
// rectangle so the player can see the column during telegraph (yellow,
// pulsing) and strike (red, solid).
//
// A `GradientLaneVisual` marker holds the owner boss. The manager spawns one
// when the boss enters HazardColumn telegraph or active, updates its
// transform and colour each frame, and despawns it when the boss leaves.

/// Marker for the HazardColumn column visual. Holds the owner boss's feature
/// id (the stable view identity, never a sim `Entity`) so the manager can
/// find or remove the matching visual.
#[derive(Component, Clone, Debug)]
pub struct GradientLaneVisual {
    pub owner_id: String,
}

const GRADIENT_LANE_TELEGRAPH_COLOR: Color = Color::srgba(1.0, 0.85, 0.20, 0.45);
const GRADIENT_LANE_STRIKE_COLOR: Color = Color::srgba(1.0, 0.32, 0.20, 0.75);
/// Z layer for the lane visual: behind feature sprites
/// (`feature_z(Boss) = 11.0`), in front of background tiles.
const GRADIENT_LANE_VISUAL_Z: f32 = 10.5;

/// Spawn/update/despawn a vertical column visual for every boss currently telegraphing or striking
/// `HazardColumn`.
pub fn manage_gradient_lane_visual(
    mut commands: Commands,
    world: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
        ambition_platformer2d_core::RoomGeometry,
    >,
    active_session: Option<Res<ActiveSessionScope>>,
    boss_frames: Res<ambition_sim_view::BossFrameIndex>,
    mut visuals: Query<(Entity, &GradientLaneVisual, &mut Transform, &mut Sprite)>,
) {
    let Some(session_scope) =
        SessionSpawnScope::for_optional_active_session(active_session.as_deref())
    else {
        return;
    };
    let mut active: std::collections::HashMap<&str, (bool, ae::Vec2, BVec2)> =
        std::collections::HashMap::new();
    for (id, frame) in boss_frames.iter() {
        if let Some(lane) = frame.hazard_lane {
            active.insert(
                id,
                (
                    lane.striking,
                    lane.center,
                    BVec2::new(lane.size.x, lane.size.y),
                ),
            );
        }
    }

    // Update existing visuals + remove stale ones.
    for (visual_entity, visual, mut transform, mut sprite) in &mut visuals {
        if let Some((in_strike, center, size)) = active.remove(visual.owner_id.as_str()) {
            transform.translation = world_to_bevy(&world.0, center, GRADIENT_LANE_VISUAL_Z);
            sprite.custom_size = Some(size);
            sprite.color = if in_strike {
                GRADIENT_LANE_STRIKE_COLOR
            } else {
                GRADIENT_LANE_TELEGRAPH_COLOR
            };
        } else {
            // The owner stopped telegraphing or striking HazardColumn: despawn.
            commands.entity(visual_entity).despawn();
        }
    }

    // Spawn visuals for bosses that newly entered HazardColumn.
    for (owner_id, (in_strike, center, size)) in active {
        let color = if in_strike {
            GRADIENT_LANE_STRIKE_COLOR
        } else {
            GRADIENT_LANE_TELEGRAPH_COLOR
        };
        commands.spawn_session_scoped(
            session_scope,
            (
                Sprite {
                    color,
                    custom_size: Some(size),
                    ..default()
                },
                Transform::from_translation(world_to_bevy(
                    &world.0,
                    center,
                    GRADIENT_LANE_VISUAL_Z,
                )),
                super::super::primitives::RoomVisual,
                GradientLaneVisual {
                    owner_id: owner_id.to_string(),
                },
                Name::new("Gradient Lane visual"),
            ),
        );
    }
}

/// Cached sprite state from before placeholder mode, so turning
/// `placeholder_sprites` off can restore the textured sprite. Stored the first
/// time a sprite becomes a coloured rectangle.
#[derive(Component, Clone)]
pub struct SpriteOriginalState {
    pub image: Handle<Image>,
    pub atlas: Option<bevy::image::TextureAtlas>,
    pub color: Color,
    pub custom_size: Option<BVec2>,
    pub image_mode: bevy::sprite::SpriteImageMode,
}

/// When `DeveloperTools::placeholder_sprites` is on, replace every textured
/// sprite with a coloured rectangle of the collision/debug size. When it
/// turns off, restore the original texture, atlas, tint, size, and image mode.
///
/// The colour comes from a per-entity marker (`FeatureVisual`, `PlayerVisual`,
/// boss, projectile) so similar entities group visually. Entities with no
/// known marker keep their sprite colour.
pub fn apply_placeholder_sprites_override(
    mut commands: Commands,
    developer_tools: Res<ambition_dev_tools::dev_tools::DeveloperTools>,
    feature_views: Res<FeatureViewIndex>,
    projectile_visuals: Res<ambition_projectiles::ProjectileVisualCatalog>,
    mut sprites: Query<(
        Entity,
        &mut Sprite,
        Option<&SpriteOriginalState>,
        Option<&FeatureVisual>,
        Option<&PlayerVisual>,
        Option<&ambition_sim_view::BodyPoseView>,
        Option<&ambition_projectiles::ProjectileVisualId>,
    )>,
) {
    if developer_tools.placeholder_sprites {
        for (entity, mut sprite, original, feature, player, player_pose, proj_id) in &mut sprites {
            // Record the original state once, to restore on toggle-off.
            if original.is_none() {
                // `try_insert`: `despawn_dead_dynamic_feature_visuals` can despawn
                // the target before this deferred write lands. Covered by
                // `deferred_write_safety::production_passes`.
                commands.entity(entity).try_insert(SpriteOriginalState {
                    image: sprite.image.clone(),
                    atlas: sprite.texture_atlas.clone(),
                    color: sprite.color,
                    custom_size: sprite.custom_size,
                    image_mode: sprite.image_mode.clone(),
                });
            }
            let feature_view = feature.and_then(|fv| feature_views.get(&fv.id));
            // Projectiles read their placeholder color from their visual id's
            // authored debug tint, resolved through the content-owned catalog.
            let proj_tint = proj_id.map(|id| projectile_visuals.debug_tint(id.as_str()));
            let placeholder_color = pick_placeholder_color(
                feature_view.map(|v| (v.kind, v.fighting)),
                player.is_some(),
                proj_tint,
            );
            // Drop the texture and atlas so the sprite is a flat rectangle. Size
            // feature placeholders to their gameplay AABB, not their render
            // bounds, so placeholder mode also shows collision size.
            if sprite.image != Handle::default() {
                sprite.image = Handle::default();
            }
            if sprite.texture_atlas.is_some() {
                sprite.texture_atlas = None;
            }
            sprite.image_mode = bevy::sprite::SpriteImageMode::Auto;
            if let Some(view) = feature_view {
                sprite.custom_size = Some(BVec2::new(view.size.x, view.size.y));
            } else if let Some(pose) = player_pose {
                sprite.custom_size = Some(BVec2::new(pose.size.x, pose.size.y));
            }
            sprite.color = placeholder_color;
        }
    } else {
        // Restore any cached originals.
        for (entity, mut sprite, original, _, _, _, _) in &mut sprites {
            if let Some(orig) = original {
                if sprite.image != orig.image {
                    sprite.image = orig.image.clone();
                }
                if sprite.texture_atlas != orig.atlas {
                    sprite.texture_atlas = orig.atlas.clone();
                }
                sprite.color = orig.color;
                sprite.custom_size = orig.custom_size;
                sprite.image_mode = orig.image_mode.clone();
                commands.entity(entity).try_remove::<SpriteOriginalState>();
            }
        }
    }
}

fn pick_placeholder_color(
    feature: Option<(FeatureVisualKind, bool)>,
    is_player: bool,
    proj_tint: Option<[f32; 4]>,
) -> Color {
    if is_player {
        return Color::srgba(0.55, 0.85, 1.00, 1.0);
    }
    // Projectiles use their visual id's authored debug tint (glider,
    // fireball, apple each distinct), not who fired them.
    if let Some([r, g, b, a]) = proj_tint {
        return Color::srgba(r, g, b, a);
    }
    match feature {
        Some((kind, fighting)) => feature_color(kind, fighting, false),
        None => Color::srgba(0.70, 0.70, 0.72, 1.0),
    }
}

#[cfg(test)]
mod tests {
    use super::effective_hide_sprites;
    use ambition_dev_tools::dev_tools::{DebugArtMode, DeveloperTools};

    #[test]
    fn placeholder_art_wins_over_stale_hide_flag() {
        let mut tools = DeveloperTools::default();
        tools.apply_debug_art_mode(DebugArtMode::Hidden);
        assert!(effective_hide_sprites(&tools));

        tools.hide_sprites = true;
        tools.placeholder_sprites = true;
        assert!(!effective_hide_sprites(&tools));
    }
}

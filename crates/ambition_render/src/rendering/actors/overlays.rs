//! Post-sync visual overlays / dev sprite overrides: hide-sprites & placeholder
//! art toggles plus the gradient-lane debug visual.
//!
//! Split out of the former 883-line `actors/mod.rs` (2026-06-15).

use super::*;

/// When `DeveloperTools::hide_sprites` is enabled, force every `Sprite`-bearing
/// entity to `Hidden` so only gizmo hitbox outlines remain visible. When the
/// flag flips off, restore every sprite to `Inherited` *exactly once* on the
/// falling edge — we deliberately do NOT keep stomping `Inherited` every
/// frame because that wipes out legitimate `Visibility::Hidden` writes from
/// upstream systems (collected pickups, idle morph-ball sphere, player while
/// in morph-ball mode, etc.) and makes them flicker back to visible.
/// UI uses `Node`/`ImageNode`, not `Sprite`, so HUD/menus are unaffected.
pub fn apply_hide_sprites_override(
    developer_tools: Res<ambition_gameplay_core::dev::dev_tools::DeveloperTools>,
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

fn effective_hide_sprites(
    developer_tools: &ambition_gameplay_core::dev::dev_tools::DeveloperTools,
) -> bool {
    // Placeholder art is a visible debug-art mode. If an old persisted or
    // inspector-mutated state leaves both booleans true, keep placeholders
    // visible instead of letting hide mode erase them.
    developer_tools.hide_sprites && !developer_tools.placeholder_sprites
}

// =================================================================
// Gradient Sentinel — HazardColumn vertical-column visual
// =================================================================
//
// The new HazardColumn boss attack profile is a tall vertical
// hazard column at the boss x. `volumes_for_profile` already
// returns the right AABB for damage; this system layers a visible
// rectangle so the player can read the column shape during
// telegraph (yellow pulsing) and strike (red solid). Without it
// the player only sees the boss's sprite tint and can't tell where
// the column is in world space.
//
// Pattern: a `GradientLaneVisual` marker component holds the owner
// boss entity. `manage_gradient_lane_visual` spawns one when the
// boss enters HazardColumn telegraph/active and despawns it when
// the boss leaves the profile. Per-frame, it also updates the
// visual's transform + color based on the live state.

/// Marker for the HazardColumn column visual entity. Carries the
/// owner boss entity so the manager system can find / remove the
/// matching visual.
#[derive(Component, Clone, Copy, Debug)]
pub struct GradientLaneVisual {
    pub owner: Entity,
}

const GRADIENT_LANE_TELEGRAPH_COLOR: Color = Color::srgba(1.0, 0.85, 0.20, 0.45);
const GRADIENT_LANE_STRIKE_COLOR: Color = Color::srgba(1.0, 0.32, 0.20, 0.75);
/// Z layer for the lane visual. Sits behind feature sprites
/// (`feature_z(Boss) = 11.0`) but in front of background tiles so
/// the column reads as a foreground hazard.
const GRADIENT_LANE_VISUAL_Z: f32 = 10.5;

/// Spawn/update/despawn a vertical column visual for every boss
/// currently telegraphing or striking `HazardColumn`. The column
/// re-uses the volume AABB computed by `volumes_for_profile` so
/// the visible rectangle always matches the damage geometry.
pub fn manage_gradient_lane_visual(
    mut commands: Commands,
    world: Res<ambition_gameplay_core::GameWorld>,
    bosses: Query<(
        Entity,
        BossClusterRef,
        &ambition_characters::brain::BossAttackState,
    )>,
    mut visuals: Query<(Entity, &GradientLaneVisual, &mut Transform, &mut Sprite)>,
) {
    use ambition_characters::brain::BossAttackProfile;
    let mut active: std::collections::HashMap<Entity, (bool, ae::Vec2, BVec2)> =
        std::collections::HashMap::new();
    for (entity, item, attack_state) in &bosses {
        let boss = item.as_boss_ref();
        if !boss.status.alive {
            continue;
        }
        let in_telegraph = matches!(
            attack_state.telegraph_profile,
            Some(BossAttackProfile::HazardColumn)
        );
        let in_strike = matches!(
            attack_state.active_profile,
            Some(BossAttackProfile::HazardColumn)
        );
        if !in_telegraph && !in_strike {
            continue;
        }
        // Use the same volume math as damage so the visual and the
        // hitbox are exactly coincident.
        let mut volumes = ambition_gameplay_core::features::volumes_for_profile(
            &BossAttackProfile::HazardColumn,
            boss.kin.pos,
            boss.combat_size(),
            &boss.config.behavior,
        );
        let Some(volume) = volumes.pop() else {
            continue;
        };
        let center = volume.center();
        let size = volume.half_size() * 2.0;
        active.insert(entity, (in_strike, center, BVec2::new(size.x, size.y)));
    }

    // Update existing visuals + remove stale ones.
    for (visual_entity, visual, mut transform, mut sprite) in &mut visuals {
        if let Some((in_strike, center, size)) = active.remove(&visual.owner) {
            transform.translation = world_to_bevy(&world.0, center, GRADIENT_LANE_VISUAL_Z);
            sprite.custom_size = Some(size);
            sprite.color = if in_strike {
                GRADIENT_LANE_STRIKE_COLOR
            } else {
                GRADIENT_LANE_TELEGRAPH_COLOR
            };
        } else {
            // Owner stopped telegraphing/striking HazardColumn — despawn.
            commands.entity(visual_entity).despawn();
        }
    }

    // Spawn visuals for bosses that newly entered HazardColumn.
    for (owner, (in_strike, center, size)) in active {
        let color = if in_strike {
            GRADIENT_LANE_STRIKE_COLOR
        } else {
            GRADIENT_LANE_TELEGRAPH_COLOR
        };
        commands.spawn((
            Sprite {
                color,
                custom_size: Some(size),
                ..default()
            },
            Transform::from_translation(world_to_bevy(&world.0, center, GRADIENT_LANE_VISUAL_Z)),
            super::super::primitives::RoomVisual,
            GradientLaneVisual { owner },
            Name::new("Gradient Lane visual"),
        ));
    }
}

/// Cached pre-placeholder sprite state so toggling `placeholder_sprites`
/// off can restore the textured rendering. Stored per-entity the first
/// time we collapse the sprite to a colored rectangle.
#[derive(Component, Clone)]
pub struct SpriteOriginalState {
    pub image: Handle<Image>,
    pub atlas: Option<bevy::image::TextureAtlas>,
    pub color: Color,
    pub custom_size: Option<BVec2>,
    pub image_mode: bevy::sprite::SpriteImageMode,
}

/// When `DeveloperTools::placeholder_sprites` is enabled, replace every
/// textured sprite with a colored rectangle of the collision/debug size —
/// the "placeholder art era" look. When the flag flips back off, restore
/// the original texture, atlas, tint, sizing, and image mode.
///
/// The placeholder color is derived from a per-entity discriminator
/// (`FeatureVisual` / `PlayerVisual` / boss / projectile markers) so
/// similar entities visually group. Anything without a known marker
/// falls back to the existing sprite color (kept as-is).
pub fn apply_placeholder_sprites_override(
    mut commands: Commands,
    developer_tools: Res<ambition_gameplay_core::dev::dev_tools::DeveloperTools>,
    feature_views: Res<FeatureViewIndex>,
    mut sprites: Query<(
        Entity,
        &mut Sprite,
        Option<&SpriteOriginalState>,
        Option<&FeatureVisual>,
        Option<&PlayerVisual>,
        Option<&ambition_gameplay_core::player::BodyKinematics>,
        Option<&crate::rendering::projectile_visuals::PlayerProjectileVisual>,
        Option<&crate::rendering::enemy_projectile_visuals::EnemyProjectileVisual>,
    )>,
) {
    if developer_tools.placeholder_sprites {
        for (entity, mut sprite, original, feature, player, player_body, p_proj, e_proj) in
            &mut sprites
        {
            // Record original state once so we can restore on toggle-off.
            if original.is_none() {
                commands.entity(entity).insert(SpriteOriginalState {
                    image: sprite.image.clone(),
                    atlas: sprite.texture_atlas.clone(),
                    color: sprite.color,
                    custom_size: sprite.custom_size,
                    image_mode: sprite.image_mode.clone(),
                });
            }
            let feature_view = feature.and_then(|fv| feature_views.get(&fv.id));
            let placeholder_color = pick_placeholder_color(
                feature_view.map(|v| v.kind),
                player.is_some(),
                p_proj.is_some(),
                e_proj.is_some(),
            );
            // Drop the texture and atlas so the sprite renders as a flat
            // rectangle. Size feature placeholders to their gameplay AABB
            // rather than their authored render bounds so placeholder mode
            // doubles as a collision-readability mode.
            if sprite.image != Handle::default() {
                sprite.image = Handle::default();
            }
            if sprite.texture_atlas.is_some() {
                sprite.texture_atlas = None;
            }
            sprite.image_mode = bevy::sprite::SpriteImageMode::Auto;
            if let Some(view) = feature_view {
                sprite.custom_size = Some(BVec2::new(view.size.x, view.size.y));
            } else if let Some(body) = player_body {
                sprite.custom_size = Some(BVec2::new(body.size.x, body.size.y));
            }
            sprite.color = placeholder_color;
        }
    } else {
        // Restore any cached originals.
        for (entity, mut sprite, original, _, _, _, _, _) in &mut sprites {
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
                commands.entity(entity).remove::<SpriteOriginalState>();
            }
        }
    }
}

fn pick_placeholder_color(
    feature_kind: Option<FeatureVisualKind>,
    is_player: bool,
    is_player_projectile: bool,
    is_enemy_projectile: bool,
) -> Color {
    if is_player {
        return Color::srgba(0.55, 0.85, 1.00, 1.0);
    }
    if is_player_projectile {
        return Color::srgba(1.00, 0.74, 0.30, 1.0);
    }
    if is_enemy_projectile {
        return Color::srgba(1.00, 0.32, 0.32, 1.0);
    }
    match feature_kind {
        Some(kind) => feature_color(kind, false),
        None => Color::srgba(0.70, 0.70, 0.72, 1.0),
    }
}

#[cfg(test)]
mod tests {
    use super::effective_hide_sprites;
    use ambition_gameplay_core::dev::dev_tools::{DebugArtMode, DeveloperTools};

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

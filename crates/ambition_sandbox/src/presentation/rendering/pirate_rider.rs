//! Composite rider sprite for fused `PirateOnShark` actors.
//!
//! The actor's primary sprite (shark) is owned by the regular enemy
//! sprite path. The rider (pirate) sits on top of the shark and has
//! an independent health pool — visually it's a *second sprite*
//! anchored above the shark's body and despawned the moment the
//! rider dies. Implemented as a per-frame despawn-and-respawn pass
//! (same pattern as `sync_enemy_projectile_visuals`) so the visual
//! set always matches the live actor list with no per-entity
//! lifecycle plumbing.
//!
//! Constraints:
//! - The rider must vanish the moment `rider_health` drops to 0
//!   (after which `apply_damage_at` morphs the archetype to
//!   `BurningFlyingShark` and the visual no longer applies).
//! - The rider must follow the shark's facing so a pirate looking
//!   right rides a shark heading right.
//! - The visual must NOT block the shark sprite — render at a
//!   slightly higher Z than the shark so it composites correctly.

use bevy::prelude::*;

use crate::assets::game_assets::GameAssets;
use crate::config::{world_to_bevy, WORLD_Z_PLAYER};
use crate::features::{ActorRuntime, EnemyArchetype, FeatureId};
use crate::presentation::character_sprites::{
    build_character_sprite_with_render_size, CharacterAnim, CharacterAnimator,
};

/// Marker on the per-frame rider sprite entities produced by
/// [`sync_pirate_rider_visuals`]. Despawned and rebuilt each tick so
/// the entity set always matches live `PirateOnShark` actors.
#[derive(Component)]
pub struct PirateRiderVisual;

/// Rider size derivation: both the vertical offset above the shark
/// center and the rider's render height scale off the live shark
/// body height (`enemy.size.y`) so an authored size tweak in
/// `enemy_archetypes.ron` propagates without re-tuning the constants
/// here. Pre-fix these were hardcoded for a 96-tall shark; the
/// shrink to 56-tall would have left the rider floating above and
/// visually dwarfing the shark.
///
/// Ratios chosen against the original 96-tall shark:
/// - rider center sits `0.35 * shark.y` above the shark's center
/// - rider renders at `0.75 * shark.y` tall (so a 56-tall shark
///   carries a 42-tall pirate, preserving the original visual ratio)
fn rider_vertical_offset(shark_height: f32) -> f32 {
    -0.35 * shark_height
}

fn rider_render_height(shark_height: f32) -> f32 {
    0.75 * shark_height
}

/// Rebuild a `PirateRiderVisual` sprite for every live-rider
/// `PirateOnShark` actor. Runs after `update_ecs_actors` so the
/// archetype + rider_health state is fresh.
pub fn sync_pirate_rider_visuals(
    mut commands: Commands,
    world: Res<crate::GameWorld>,
    assets: Option<Res<GameAssets>>,
    images: Res<Assets<Image>>,
    ecs_actors: Query<(&FeatureId, &ActorRuntime)>,
    existing: Query<Entity, With<PirateRiderVisual>>,
) {
    for entity in &existing {
        commands.entity(entity).despawn();
    }
    let Some(assets) = assets else {
        return;
    };
    for (_id, actor) in &ecs_actors {
        let ActorRuntime::Hostile(enemy) = actor else {
            continue;
        };
        if !matches!(
            enemy.archetype,
            EnemyArchetype::PirateOnShark | EnemyArchetype::PirateHeavyOnShark
        ) {
            continue;
        }
        if !enemy.alive || !enemy.has_live_rider() {
            continue;
        }
        let rider_sprite_name = rider_sprite_name_for(enemy);
        let Some(rider_asset) = assets.characters.npc_asset_for_name(rider_sprite_name) else {
            continue;
        };
        if images.get(&rider_asset.texture).is_none() {
            continue;
        }
        let shark_height = enemy.size.y;
        let rider_height = rider_render_height(shark_height);
        let rider_pos = crate::engine_core::Vec2::new(
            enemy.pos.x,
            enemy.pos.y + rider_vertical_offset(shark_height),
        );
        // Scale render size to match the desired rider height while
        // preserving the sheet's aspect ratio (frame is 128×128 for
        // the pirate raider; ~172×138 for heavy variants).
        let aspect = rider_asset.spec.frame_width as f32 / rider_asset.spec.frame_height as f32;
        let render = bevy::math::Vec2::new(rider_height * aspect, rider_height);
        let mut sprite = build_character_sprite_with_render_size(rider_asset, render);
        sprite.flip_x = enemy.facing < 0.0;
        let translation = world_to_bevy(
            &world.0,
            rider_pos,
            // Slightly above the shark sprite so the rider isn't
            // occluded by the shark torso.
            WORLD_Z_PLAYER + 0.5,
        );
        let mut animator = CharacterAnimator::new(&rider_asset.spec);
        // The rider rides — show the idle pose. Future polish: swing
        // when the shark fires, hurt when rider takes damage.
        animator.request(CharacterAnim::Idle);
        let index = animator.tick(0.0);
        if let Some(atlas) = sprite.texture_atlas.as_mut() {
            atlas.index = index;
        }
        commands.spawn((
            sprite,
            Transform::from_translation(translation),
            animator,
            PirateRiderVisual,
            Name::new("Pirate rider visual"),
        ));
    }
}

/// Pick which NPC-sprite display name represents the rider on top of
/// a fused pirate-on-shark actor. For the legacy `PirateOnShark`
/// archetype this is always the Pirate Raider sheet. For the heavy
/// variant we look at the EnemySpawn's authored display name and
/// strip the " on Shark" suffix so the ground-side heavy sheet
/// (Broadside Bess / Iron Mary / Salt Annet) is reused above the
/// shark too.
fn rider_sprite_name_for(enemy: &crate::features::EnemyRuntime) -> &'static str {
    if enemy.archetype != EnemyArchetype::PirateHeavyOnShark {
        return "Pirate Raider";
    }
    // The runtime's `name` field is whatever the EnemySpawn was
    // authored as (e.g. "Iron Mary on Shark"). Strip the suffix to
    // find the matching ground-form sheet. Fall back to Broadside
    // Bess so a misspelled spawn still renders a heavy.
    let base = enemy.name.strip_suffix(" on Shark").unwrap_or(&enemy.name);
    match base {
        "Broadside Bess" => "Broadside Bess",
        "Iron Mary" => "Iron Mary",
        "Salt Annet" => "Salt Annet",
        _ => "Broadside Bess",
    }
}

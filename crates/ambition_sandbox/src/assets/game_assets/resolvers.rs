//! Sprite RESOLVERS: map sim/world entities (hazards, pickups, chests,
//! breakables, enemies, blocks, loading zones) to an `EntitySprite`.

use crate::engine_core as ae;
use bevy::prelude::*;

use super::*;
use crate::features::FeatureVisualKind;
use crate::rooms::LoadingZoneActivation;

pub fn entity_sprite(
    assets: &GameAssets,
    key: EntitySprite,
    size: Vec2,
    fallback_color: Color,
) -> Sprite {
    match assets.entities.get(key) {
        Some(handle) => {
            let mut sprite = Sprite::from_image(handle.clone());
            sprite.custom_size = Some(size);
            sprite
        }
        None => Sprite::from_color(fallback_color, size),
    }
}

/// Same as [`entity_sprite`] but `kind` is optional — `None` always falls
/// through to the colored rectangle. Useful for call sites that map a
/// runtime kind (e.g. `BlockKind`) to an `Option<EntitySprite>` because
/// some variants don't have a dedicated sprite.
pub fn entity_sprite_or_color(
    assets: &GameAssets,
    key: Option<EntitySprite>,
    size: Vec2,
    fallback_color: Color,
) -> Sprite {
    match key.and_then(|k| assets.entities.get(k)) {
        Some(handle) => {
            let mut sprite = Sprite::from_image(handle.clone());
            sprite.custom_size = Some(size);
            sprite
        }
        None => Sprite::from_color(fallback_color, size),
    }
}

/// Per-family entity-sprite resolvers. Stateless choices — the
/// runtime sync system swaps the sprite later for state-driven kinds
/// (chest open, breakable cracked).
pub fn entity_sprite_for_hazard(_volume: &crate::combat::DamageVolume) -> Option<EntitySprite> {
    Some(EntitySprite::HazardSpikes)
}

pub fn entity_sprite_for_pickup(pickup: &crate::interaction::Pickup) -> Option<EntitySprite> {
    Some(pickup_sprite(&pickup.kind))
}

pub fn entity_sprite_for_chest(_chest: &crate::interaction::Chest) -> Option<EntitySprite> {
    Some(EntitySprite::ChestClosed)
}

pub fn entity_sprite_for_breakable(
    _breakable: &crate::interaction::Breakable,
) -> Option<EntitySprite> {
    Some(EntitySprite::BreakableIntact)
}

pub fn entity_sprite_for_interactable(
    interactable: &crate::interaction::Interactable,
) -> Option<EntitySprite> {
    if matches!(
        interactable.kind,
        crate::interaction::InteractionKind::Npc { .. }
    ) {
        Some(EntitySprite::NpcTerminal)
    } else {
        None
    }
}

pub fn entity_sprite_for_enemy(brain: &crate::actor::EnemyBrain) -> Option<EntitySprite> {
    // Training dummies use a dedicated static sprite (authored
    // archetype data); other enemies use animated spritesheets, not a
    // static entity sprite — `upgrade_enemy_sprites` handles them.
    match crate::features::enemy_visual_kind(brain) {
        crate::features::FeatureVisualKind::TrainingDummy => Some(EntitySprite::SandbagDummy),
        _ => None,
    }
}

pub fn entity_sprite_for_boss(_brain: &crate::actor::BossBrain) -> Option<EntitySprite> {
    Some(EntitySprite::BossCore)
}

fn pickup_sprite(kind: &crate::interaction::PickupKind) -> EntitySprite {
    match kind {
        crate::interaction::PickupKind::Health { .. } => EntitySprite::PickupHealth,
        crate::interaction::PickupKind::Currency { .. } => EntitySprite::PickupCurrency,
        crate::interaction::PickupKind::Ability { .. } => EntitySprite::PickupAbility,
        // StoryFlag and Custom fall back to the ability look until they
        // get dedicated art.
        _ => EntitySprite::PickupAbility,
    }
}

/// State-aware sprite for a breakable based on its current health state.
pub fn breakable_state_sprite(state: crate::interaction::BreakableState) -> EntitySprite {
    match state {
        crate::interaction::BreakableState::Intact => EntitySprite::BreakableIntact,
        crate::interaction::BreakableState::Cracking => EntitySprite::BreakableCracked,
        crate::interaction::BreakableState::Broken
        | crate::interaction::BreakableState::Respawning => EntitySprite::BreakableBroken,
    }
}

/// State-aware sprite for a chest by opened-flag.
pub fn chest_state_sprite(opened: bool) -> EntitySprite {
    if opened {
        EntitySprite::ChestOpen
    } else {
        EntitySprite::ChestClosed
    }
}

/// Block-kind sprites. `BlockKind::Hazard` reuses the hazard-spikes art.
pub fn block_sprite(kind: ae::BlockKind) -> Option<EntitySprite> {
    match kind {
        ae::BlockKind::Solid => Some(EntitySprite::SolidBlock),
        ae::BlockKind::OneWay => Some(EntitySprite::OneWayPlatform),
        ae::BlockKind::Hazard => Some(EntitySprite::HazardSpikes),
        ae::BlockKind::PogoOrb => Some(EntitySprite::PogoOrb),
        ae::BlockKind::Rebound { .. } => Some(EntitySprite::ReboundPad),
        ae::BlockKind::BlinkWall {
            tier: ae::BlinkWallTier::Soft,
        } => Some(EntitySprite::SoftBlinkWall),
        ae::BlockKind::BlinkWall {
            tier: ae::BlinkWallTier::Hard,
        } => Some(EntitySprite::HardBlinkWall),
    }
}

/// Tile-sprite variant of `block_sprite` for IntGrid-derived blocks.
/// Returns the seamless 32×32 tile texture that the renderer should
/// REPEAT (via `Sprite::image_mode = Tiled`) across the block's
/// arbitrary aspect ratio. Returns `None` for kinds that don't have
/// a tile generator yet (PogoOrb / Rebound — those are point-shaped
/// authored entities, not tiled surfaces).
pub fn block_tile_sprite(kind: ae::BlockKind) -> Option<EntitySprite> {
    match kind {
        ae::BlockKind::Solid => Some(EntitySprite::SolidTile),
        ae::BlockKind::OneWay => Some(EntitySprite::OneWayTile),
        ae::BlockKind::Hazard => Some(EntitySprite::HazardTile),
        ae::BlockKind::BlinkWall {
            tier: ae::BlinkWallTier::Soft,
        } => Some(EntitySprite::SoftBlinkTile),
        ae::BlockKind::BlinkWall {
            tier: ae::BlinkWallTier::Hard,
        } => Some(EntitySprite::HardBlinkTile),
        // PogoOrb / Rebound stay on the entity-art path because they
        // are point objects, not tiled surfaces. Authored as single
        // entities with fixed-aspect art.
        _ => None,
    }
}

/// Loading-zone sprites — cosmetic, the actual zone behavior comes from
/// the gameplay layer.
pub fn loading_zone_sprite(activation: LoadingZoneActivation) -> EntitySprite {
    match activation {
        LoadingZoneActivation::Door => EntitySprite::DoorZone,
        LoadingZoneActivation::EdgeExit => EntitySprite::EdgeExit,
        // `Walk` zones (mid-room walk-through portals) reuse the
        // EdgeExit sprite for now — both are overlap-triggered, no
        // interact prompt. A dedicated portal-glow sprite can land
        // when art does.
        LoadingZoneActivation::Walk => EntitySprite::EdgeExit,
    }
}

/// Map a `FeatureVisualKind` to a default entity sprite, ignoring per-
/// instance state. Used as a backstop when the engine kind isn't known
/// in detail (e.g. inside `sync_visuals`).
///
/// Today only the tests use this; production sprite resolution goes
/// through the per-state helpers (`pickup_sprite`, `chest_state_sprite`,
/// etc.). Kept pub so a future "kind is the only signal" call site can
/// adopt it without re-deriving the mapping.
#[cfg_attr(not(test), allow(dead_code))]
pub fn entity_sprite_for_kind(kind: FeatureVisualKind) -> Option<EntitySprite> {
    match kind {
        FeatureVisualKind::Hazard => Some(EntitySprite::HazardSpikes),
        FeatureVisualKind::TrainingDummy => Some(EntitySprite::SandbagDummy),
        FeatureVisualKind::Boss => Some(EntitySprite::BossCore),
        FeatureVisualKind::Breakable => Some(EntitySprite::BreakableIntact),
        FeatureVisualKind::Chest => Some(EntitySprite::ChestClosed),
        FeatureVisualKind::Pickup => Some(EntitySprite::PickupHealth),
        FeatureVisualKind::Npc => Some(EntitySprite::NpcTerminal),
        // Enemies are animated; rendering handles them through the
        // character spritesheet, not a static entity sprite.
        FeatureVisualKind::Enemy => None,
        // Switches render as a colored block (red / green) rather
        // than a static entity sprite — see `feature_color` and
        // `switch_on_color` in `rendering.rs`.
        FeatureVisualKind::Switch => None,
    }
}

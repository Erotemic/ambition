//! Sprite resolvers: map sim and world entities (hazards, pickups, chests,
//! breakables, enemies, blocks, loading zones) to an `EntitySprite`.

use ambition_platformer2d_core as ae;

use super::*;
use ambition_platformer2d_shared_tangle::feature_kind::FeatureVisualKind;
use ambition_platformer2d_world::rooms::LoadingZoneActivation;

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

/// Like [`entity_sprite`], but `kind` is optional; `None` gives the colored
/// rectangle. Use it when some runtime kinds (for example `BlockKind`) have no
/// dedicated sprite.
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

/// Per-family entity-sprite resolvers. These are stateless; the runtime sync
/// system swaps the sprite later for state-driven kinds (chest open,
/// breakable cracked).
pub fn entity_sprite_for_hazard(
    _volume: &ambition_platformer2d_world::rooms::HazardVolumeSpec,
) -> Option<EntitySprite> {
    Some(EntitySprite::HazardSpikes)
}

pub fn entity_sprite_for_pickup(
    pickup: &ambition_platformer2d_world::rooms::PickupSpec,
) -> Option<EntitySprite> {
    Some(pickup_sprite(&pickup.kind))
}

/// Pickup resolver for a pickup the simulation created (a bounty coin, a
/// boss heart, a scattered ring). A dropped pickup has no authored spec, and
/// sim-view must not invent one, so this is the runtime twin of
/// [`entity_sprite_for_pickup`] (as with [`entity_sprite_for_runtime_chest`]).
pub fn entity_sprite_for_runtime_pickup(
    kind: &ambition_interaction::PickupKind,
) -> Option<EntitySprite> {
    use ambition_interaction::PickupKind as K;
    Some(match kind {
        K::Health { .. } => EntitySprite::PickupHealth,
        K::Currency { .. } => EntitySprite::PickupCurrency,
        K::Ability { .. } => EntitySprite::PickupAbility,
        // StoryFlag and Custom use the ability look until they get their own
        // art, as the authored twin does.
        K::StoryFlag { .. } | K::Custom(_) => EntitySprite::PickupAbility,
    })
}

pub fn entity_sprite_for_chest(
    _chest: &ambition_platformer2d_world::rooms::ChestSpec,
) -> Option<EntitySprite> {
    Some(EntitySprite::ChestClosed)
}

/// Chest resolver for sim-view after room specs are lowered into interaction
/// components. It lives here so sim-view does not rebuild world specs to pick
/// the art.
pub fn entity_sprite_for_runtime_chest(
    _chest: &ambition_interaction::Chest,
) -> Option<EntitySprite> {
    Some(EntitySprite::ChestClosed)
}

pub fn entity_sprite_for_breakable(
    _breakable: &ambition_platformer2d_world::rooms::BreakableSpec,
) -> Option<EntitySprite> {
    Some(EntitySprite::BreakableIntact)
}

pub fn entity_sprite_for_interactable(
    interactable: &ambition_platformer2d_world::rooms::InteractableSpec,
) -> Option<EntitySprite> {
    if matches!(
        interactable.kind,
        ambition_platformer2d_world::rooms::InteractionKindSpec::Npc { .. }
    ) {
        Some(EntitySprite::NpcTerminal)
    } else {
        None
    }
}

/// Interactable resolver for sim-view after room specs are lowered into
/// interaction components. It mirrors [`entity_sprite_for_interactable`]
/// without rebuilding authored specs.
pub fn entity_sprite_for_runtime_interactable(
    interactable: &ambition_interaction::Interactable,
) -> Option<EntitySprite> {
    if matches!(
        interactable.kind,
        ambition_interaction::InteractionKind::Npc { .. }
    ) {
        Some(EntitySprite::NpcTerminal)
    } else {
        None
    }
}

pub fn entity_sprite_for_enemy(
    brain: &ambition_entity_catalog::placements::CharacterBrain,
) -> Option<EntitySprite> {
    // Training dummies use a static sprite. Other actors use animated sheets
    // through `upgrade_actor_sprites`. This layer knows only the authored
    // placement vocabulary, so it uses the catalog-key convention of the
    // shipped training-dummy rows.
    if character_brain_is_sandbag(brain) {
        Some(EntitySprite::SandbagDummy)
    } else {
        None
    }
}

fn character_brain_is_sandbag(brain: &ambition_entity_catalog::placements::CharacterBrain) -> bool {
    matches!(
        brain,
        ambition_entity_catalog::placements::CharacterBrain::Custom(key)
            if key == "sandbag" || key == "sandbag_infinite" || key == "sandbag_finite"
    )
}

pub fn entity_sprite_for_boss(
    _brain: &ambition_entity_catalog::placements::BossBrain,
) -> Option<EntitySprite> {
    Some(EntitySprite::BossCore)
}

fn pickup_sprite(kind: &ambition_platformer2d_world::rooms::PickupKind) -> EntitySprite {
    match kind {
        ambition_platformer2d_world::rooms::PickupKind::Health { .. } => {
            EntitySprite::PickupHealth
        }
        ambition_platformer2d_world::rooms::PickupKind::Currency { .. } => {
            EntitySprite::PickupCurrency
        }
        ambition_platformer2d_world::rooms::PickupKind::Ability { .. } => {
            EntitySprite::PickupAbility
        }
        // StoryFlag and Custom fall back to the ability look until they
        // get dedicated art.
        _ => EntitySprite::PickupAbility,
    }
}

/// State-aware sprite for a breakable based on its current health state.
pub fn breakable_state_sprite(state: ambition_interaction::BreakableState) -> EntitySprite {
    match state {
        ambition_interaction::BreakableState::Intact => EntitySprite::BreakableIntact,
        ambition_interaction::BreakableState::Cracking => EntitySprite::BreakableCracked,
        ambition_interaction::BreakableState::Broken
        | ambition_interaction::BreakableState::Respawning => EntitySprite::BreakableBroken,
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

/// Art for a block that is a point, not a surface: its box is the art's own
/// shape, so drawing the art across the box does not distort it.
///
/// Art stretched across a surface stretches its transparent border too, so the
/// block collides past its visible ends. Surface art must repeat, and
/// [`block_tile_sprite`] gives the repeating texture. The renderer asks that
/// first and comes here only for a kind with no tile texture.
///
/// So a new surface kind must add a tile texture, not a prop.
/// `every_surface_kind_has_a_tile_texture` guards this.
pub fn point_block_sprite(kind: ae::BlockKind) -> Option<EntitySprite> {
    match kind {
        ae::BlockKind::PogoOrb => Some(EntitySprite::PogoOrb),
        ae::BlockKind::Rebound { .. } => Some(EntitySprite::ReboundPad),
        // `None` on purpose: a bonk-only block stays hidden until struck. Its
        // found look belongs to the game's dresser; a default here would show
        // the secret.
        ae::BlockKind::BonkOnly => None,
        // Every other kind is a surface and repeats its tile texture. The kinds
        // are listed, not wildcarded, so a new kind must choose.
        ae::BlockKind::Solid
        | ae::BlockKind::OneWay
        | ae::BlockKind::Hazard
        | ae::BlockKind::BlinkWall { .. } => None,
    }
}

/// The seamless texture a surface repeats. The renderer asks for it first,
/// because repeating at native pixel scale is the only correct way to cover a
/// box of a different size. `None` for point kinds (PogoOrb, Rebound), which
/// use [`point_block_sprite`].
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
        // Listed, not wildcarded, so a new kind must answer.
        ae::BlockKind::PogoOrb | ae::BlockKind::Rebound { .. } | ae::BlockKind::BonkOnly => None,
    }
}

/// True if this kind is a point, not a surface: its box is its art's shape,
/// so nothing stretches.
///
/// Keep the list short. Every other kind is a shape an author sizes freely,
/// and art that does not repeat cannot cover it.
pub fn is_point_block_kind(kind: ae::BlockKind) -> bool {
    matches!(
        kind,
        ae::BlockKind::PogoOrb | ae::BlockKind::Rebound { .. } | ae::BlockKind::BonkOnly
    )
}

/// Loading-zone sprites — cosmetic, the actual zone behavior comes from
/// the gameplay layer.
pub fn loading_zone_sprite(activation: LoadingZoneActivation) -> EntitySprite {
    match activation {
        LoadingZoneActivation::Door => EntitySprite::DoorZone,
        LoadingZoneActivation::EdgeExit => EntitySprite::EdgeExit,
        // `Walk` zones (mid-room portals) reuse the EdgeExit sprite for now:
        // both trigger on overlap with no interact prompt.
        LoadingZoneActivation::Walk => EntitySprite::EdgeExit,
    }
}

/// Map a `FeatureVisualKind` to a default entity sprite, ignoring instance
/// state. A backstop when the engine kind is not known in detail.
///
/// Only tests use this now; production goes through the per-state helpers
/// (`pickup_sprite`, `chest_state_sprite`, and so on).
#[cfg_attr(not(test), allow(dead_code))]
pub fn entity_sprite_for_kind(kind: FeatureVisualKind) -> Option<EntitySprite> {
    match kind {
        FeatureVisualKind::Hazard => Some(EntitySprite::HazardSpikes),
        FeatureVisualKind::Breakable => Some(EntitySprite::BreakableIntact),
        FeatureVisualKind::Chest => Some(EntitySprite::ChestClosed),
        FeatureVisualKind::Pickup => Some(EntitySprite::PickupHealth),
        // Actors are animated; `upgrade_actor_sprites` renders them, not a
        // static entity sprite.
        FeatureVisualKind::Actor => None,
        // Switches render as a colored block (red / green) rather
        // than a static entity sprite — see `feature_color` and
        // `switch_on_color` in `rendering.rs`.
        FeatureVisualKind::Switch => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every kind an author can drag has art that repeats.
    ///
    /// Stretched art also stretches its transparent border, so the block
    /// collides where nothing is drawn. A new surface kind with a prop and no
    /// tile texture would bring that back silently.
    #[test]
    fn every_surface_kind_has_a_tile_texture() {
        let kinds = [
            ae::BlockKind::Solid,
            ae::BlockKind::OneWay,
            ae::BlockKind::Hazard,
            ae::BlockKind::BonkOnly,
            ae::BlockKind::PogoOrb,
            ae::BlockKind::Rebound {
                impulse: ae::Vec2::ZERO,
            },
            ae::BlockKind::BlinkWall {
                tier: ae::BlinkWallTier::Soft,
            },
            ae::BlockKind::BlinkWall {
                tier: ae::BlinkWallTier::Hard,
            },
        ];
        for kind in kinds {
            if is_point_block_kind(kind) {
                continue;
            }
            assert!(
                block_tile_sprite(kind).is_some(),
                "{kind:?} is a surface an author sizes freely, so its art has to \
                 repeat: give it a tile texture, or say it is a point in \
                 `is_point_block_kind`"
            );
        }
    }

    /// The prop path is reachable only for points, so a surface is never
    /// drawn by stretching.
    #[test]
    fn only_point_kinds_answer_with_prop_art() {
        assert!(point_block_sprite(ae::BlockKind::Solid).is_none());
        assert!(point_block_sprite(ae::BlockKind::OneWay).is_none());
        assert!(point_block_sprite(ae::BlockKind::Hazard).is_none());
        assert!(point_block_sprite(ae::BlockKind::PogoOrb).is_some());
        assert!(point_block_sprite(ae::BlockKind::Rebound {
            impulse: ae::Vec2::ZERO
        })
        .is_some());
    }
}

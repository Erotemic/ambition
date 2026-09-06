//! Sprite RESOLVERS: map sim/world entities (hazards, pickups, chests,
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

/// Runtime pickup resolver used by sim-view facts for a pickup the SIMULATION
/// minted — an enemy's bounty coin, a boss's heart, a scattered ring. Twin of
/// [`entity_sprite_for_pickup`], which reads the authored spec, and it exists
/// for the reason [`entity_sprite_for_runtime_chest`] does: a dropped pickup
/// has no authored spec behind it, and sim-view must not fabricate one just to
/// pick the same art.
pub fn entity_sprite_for_runtime_pickup(
    kind: &ambition_interaction::PickupKind,
) -> Option<EntitySprite> {
    use ambition_interaction::PickupKind as K;
    Some(match kind {
        K::Health { .. } => EntitySprite::PickupHealth,
        K::Currency { .. } => EntitySprite::PickupCurrency,
        K::Ability { .. } => EntitySprite::PickupAbility,
        // StoryFlag and Custom fall back to the ability look until they get
        // dedicated art — the same fallback the authored twin takes.
        K::StoryFlag { .. } | K::Custom(_) => EntitySprite::PickupAbility,
    })
}

pub fn entity_sprite_for_chest(
    _chest: &ambition_platformer2d_world::rooms::ChestSpec,
) -> Option<EntitySprite> {
    Some(EntitySprite::ChestClosed)
}

/// Runtime chest resolver used by sim-view facts after authored room specs
/// have been lowered into interaction components. Keep this in the sprite
/// resolver layer so sim-view does not rebuild fake world specs just to pick
/// the same entity art.
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

/// Runtime interactable resolver used by sim-view facts after authored room
/// specs have been lowered into interaction components. This mirrors
/// [`entity_sprite_for_interactable`] without forcing sim-view to depend on
/// authored-spec reconstruction.
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
    // Training dummies use a dedicated static sprite; other actors use animated
    // spritesheets, not a static entity sprite — `upgrade_actor_sprites` handles
    // them. At this lower layer we only know the authored placement vocabulary,
    // so this follows the stable catalog-key convention used by the shipped
    // training-dummy rows.
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

/// Art for a block that is a POINT, not a surface — one whose box is its
/// art's own shape, so drawing the art across the box distorts nothing.
///
/// Stretched across an authored surface — Smash's 420×32 stage — the border stretches too, so the
/// platform collides about 18px further than it can be seen at each end: an invisible floor you
/// stand on and an invisible wall you hit. A surface's art has to REPEAT, and [`block_tile_sprite`]
/// is the one that repeats, so the renderer asks that first and only lands here for a kind with no
/// tile texture at all.
///
/// so a new surface kind must bring a tile texture, not a prop. The
/// contract is pinned by `every_surface_kind_has_a_tile_texture`; adding a kind
/// here instead is how the invisible edge comes back.
pub fn point_block_sprite(kind: ae::BlockKind) -> Option<EntitySprite> {
    match kind {
        ae::BlockKind::PogoOrb => Some(EntitySprite::PogoOrb),
        ae::BlockKind::Rebound { .. } => Some(EntitySprite::ReboundPad),
        // `None`, and that is the kind's whole point. A bonk-only block is
        // hidden until it has been struck; whatever a game wants it to look like
        // once found is that game's own dresser's decision, and a default here
        // would draw the secret.
        ae::BlockKind::BonkOnly => None,
        // Every remaining kind is a SURFACE and is drawn by repeating its tile
        // texture. Listed rather than wildcarded so a new kind has to choose.
        ae::BlockKind::Solid
        | ae::BlockKind::OneWay
        | ae::BlockKind::Hazard
        | ae::BlockKind::BlinkWall { .. } => None,
    }
}

/// The seamless texture a SURFACE repeats — the one the renderer asks for
/// first, whatever the block's provenance, because repeating at native pixel
/// scale is the only way art of one size honestly covers a box of another.
/// Returns `None` for the point-shaped kinds, which have no surface to tile
/// (PogoOrb / Rebound) and fall through to [`point_block_sprite`].
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
        // Listed rather than wildcarded so a new kind has to answer this question.
        ae::BlockKind::PogoOrb | ae::BlockKind::Rebound { .. } | ae::BlockKind::BonkOnly => None,
    }
}

/// Is this kind a POINT rather than a surface — its box IS its art's shape,
/// so nothing about it can be stretched into a lie?
///
/// The list exists to be short. Everything else is a shape an author drags to
/// whatever size the level wants, and art that does not repeat cannot honestly
/// cover it.
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
        FeatureVisualKind::Breakable => Some(EntitySprite::BreakableIntact),
        FeatureVisualKind::Chest => Some(EntitySprite::ChestClosed),
        FeatureVisualKind::Pickup => Some(EntitySprite::PickupHealth),
        // Actors are animated (or resolve a state-keyed fallback sheet); rendering
        // handles them through `upgrade_actor_sprites`, not a static entity sprite.
        // The sandbag/boss/NPC static-sprite arms died with the actor variants.
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

    /// Every kind an author can DRAG has art that repeats.
    ///
    /// the invariant a stretched prop broke: art of one size covers a box of
    /// another only by repeating. When it stretches instead, the transparent
    /// border every prop is generated with stretches too, and the block collides
    /// where nothing is drawn — Smash's stage was solid about 18px past each
    /// visible end. A new surface kind that arrives with a prop and no tile
    /// texture brings that back, silently, so the contract is stated here rather
    /// than in a comment somebody has to find.
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

    /// The prop path is now reachable ONLY for points — which is what stops a
    /// surface from ever being drawn by stretching again.
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

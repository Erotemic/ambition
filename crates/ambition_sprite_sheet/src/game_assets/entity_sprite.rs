//! The `EntitySprite` vocabulary (single-frame entity placeholders) + its
//! asset-id / manifest / catalog wiring and the `EntitySpriteSet` handle store.

use std::collections::HashMap;

use ambition_asset_manager::AssetId;
use super::*;

/// Single-frame entity sprites keyed off the gen2d manifest.
///
/// Each variant maps to `entities/<lower_snake_case>.png` under the configured
/// sprite folder. A new entry needs only a path in `relative_path`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EntitySprite {
    // Features
    ChestClosed,
    ChestOpen,
    BreakableIntact,
    BreakableCracked,
    BreakableBroken,
    PickupHealth,
    PickupCurrency,
    PickupAbility,
    HazardSpikes,
    NpcTerminal,
    BossCore,
    SandbagDummy,
    // Switch on/off: `state_aware_entity_sprite` chooses by
    // `FeatureView::switch_on` (armed = on, disabled = off).
    SwitchArmed,
    SwitchDisabled,
    // Blocks / surfaces
    PogoOrb,
    ReboundPad,
    MovingPlatform,
    // Loading zones
    DoorZone,
    /// A door of another make. No kind resolves to these: a game chooses one
    /// with `EntityArt` on a door visual. Each has the outer shape of
    /// [`Self::DoorZone`], so the swap does not change the geometry.
    DoorStone,
    DoorVoxel,
    DoorBlueprint,
    EdgeExit,
    // Player projectiles (Fireball + Hadouken share the same sprite)
    ProjectileEnergy,
    /// A round ball of fire, for a hazard that is fire (a bar of fire that
    /// turns). No hazard draws it by default: a game names it through the
    /// `EntityArt` component.
    FireOrb,
    // 32×32 tile sprites for IntGrid block surfaces. Rendered with
    // `Sprite::image_mode = Tiled` so they repeat across long floors and tall
    // walls instead of stretching.
    /// A live bonus block: warm plate, rivets, interrobang glyph.
    ///
    /// Not reachable from `BlockKind`: bonus blocks are `BlockKind::Solid` like
    /// every wall, so a game names this through the `EntityArt` component.
    BonusBlockTile,
    /// A used bonus block: the same plate and rivets, drained and glyphless.
    ///
    /// A spent block does not use `SolidTile`. A used block that looks like a
    /// wall would hide its history from a player deciding whether to hit it.
    SpentBlockTile,
    /// One tile of a bridge: a deck of boards with a chain under it, open
    /// below, so what is under the bridge shows. A game names it through the
    /// `EntityArt` component.
    BridgePlankTile,
    SolidTile,
    OneWayTile,
    HazardTile,
    SoftBlinkTile,
    HardBlinkTile,
    /// The encounter "lock wall" block that `sync_lock_walls` inserts into
    /// `world.blocks` during an encounter. It has its own tile so it looks new,
    /// not like the adjacent walls.
    LockWallTile,
}

impl EntitySprite {
    /// Path relative to the configured sprite folder.
    pub const fn relative_path(self) -> &'static str {
        match self {
            Self::ChestClosed => "entities/chest_closed.png",
            Self::ChestOpen => "entities/chest_open.png",
            Self::BreakableIntact => "entities/breakable_intact.png",
            Self::BreakableCracked => "entities/breakable_cracked.png",
            Self::BreakableBroken => "entities/breakable_broken.png",
            Self::PickupHealth => "entities/pickup_health.png",
            Self::PickupCurrency => "entities/pickup_currency.png",
            Self::PickupAbility => "entities/pickup_ability.png",
            Self::HazardSpikes => "entities/hazard_spikes.png",
            Self::NpcTerminal => "entities/npc_terminal.png",
            Self::BossCore => "entities/boss_core.png",
            Self::SandbagDummy => "entities/sandbag_dummy.png",
            Self::SwitchArmed => "entities/switch_armed.png",
            Self::SwitchDisabled => "entities/switch_disabled.png",
            Self::PogoOrb => "entities/pogo_orb.png",
            Self::ReboundPad => "entities/rebound_pad.png",
            Self::MovingPlatform => "entities/moving_platform.png",
            Self::DoorZone => "entities/door_zone.png",
            Self::DoorStone => "entities/door_stone.png",
            Self::DoorVoxel => "entities/door_voxel.png",
            Self::DoorBlueprint => "entities/door_blueprint.png",
            Self::EdgeExit => "entities/edge_exit.png",
            Self::ProjectileEnergy => "entities/projectile_energy.png",
            Self::FireOrb => "entities/fire_orb.png",
            Self::BonusBlockTile => "entities/bonus_block_tile.png",
            Self::SpentBlockTile => "entities/spent_block_tile.png",
            Self::BridgePlankTile => "entities/bridge_plank_tile.png",
            Self::SolidTile => "entities/solid_tile.png",
            Self::OneWayTile => "entities/one_way_tile.png",
            Self::HazardTile => "entities/hazard_tile.png",
            Self::SoftBlinkTile => "entities/soft_blink_tile.png",
            Self::HardBlinkTile => "entities/hard_blink_tile.png",
            Self::LockWallTile => "entities/lock_wall_tile.png",
        }
    }

    pub const ALL: &'static [Self] = &[
        Self::ChestClosed,
        Self::ChestOpen,
        Self::BreakableIntact,
        Self::BreakableCracked,
        Self::BreakableBroken,
        Self::PickupHealth,
        Self::PickupCurrency,
        Self::PickupAbility,
        Self::HazardSpikes,
        Self::NpcTerminal,
        Self::BossCore,
        Self::SandbagDummy,
        Self::SwitchArmed,
        Self::SwitchDisabled,
        Self::PogoOrb,
        Self::ReboundPad,
        Self::MovingPlatform,
        Self::DoorZone,
        Self::DoorStone,
        Self::DoorVoxel,
        Self::DoorBlueprint,
        Self::EdgeExit,
        Self::ProjectileEnergy,
        Self::FireOrb,
        Self::BonusBlockTile,
        Self::SpentBlockTile,
        Self::BridgePlankTile,
        Self::SolidTile,
        Self::OneWayTile,
        Self::HazardTile,
        Self::SoftBlinkTile,
        Self::HardBlinkTile,
        Self::LockWallTile,
    ];
}

/// Stable [`AssetId`] for an [`EntitySprite`].
///
/// The namespace `sprite.entity.<lower_snake>` is part of the public
/// asset-catalog contract; authored manifests depend on it.
/// [`tests::every_entity_sprite_has_a_unique_asset_id_in_sprite_entity_namespace`]
/// guards the mapping.
pub fn entity_sprite_asset_id(key: EntitySprite) -> AssetId {
    let suffix = match key {
        EntitySprite::ChestClosed => "chest_closed",
        EntitySprite::ChestOpen => "chest_open",
        EntitySprite::BreakableIntact => "breakable_intact",
        EntitySprite::BreakableCracked => "breakable_cracked",
        EntitySprite::BreakableBroken => "breakable_broken",
        EntitySprite::PickupHealth => "pickup_health",
        EntitySprite::PickupCurrency => "pickup_currency",
        EntitySprite::PickupAbility => "pickup_ability",
        EntitySprite::HazardSpikes => "hazard_spikes",
        EntitySprite::NpcTerminal => "npc_terminal",
        EntitySprite::BossCore => "boss_core",
        EntitySprite::SandbagDummy => "sandbag_dummy",
        EntitySprite::SwitchArmed => "switch_armed",
        EntitySprite::SwitchDisabled => "switch_disabled",
        EntitySprite::PogoOrb => "pogo_orb",
        EntitySprite::ReboundPad => "rebound_pad",
        EntitySprite::MovingPlatform => "moving_platform",
        EntitySprite::DoorZone => "door_zone",
        EntitySprite::DoorStone => "door_stone",
        EntitySprite::DoorVoxel => "door_voxel",
        EntitySprite::DoorBlueprint => "door_blueprint",
        EntitySprite::EdgeExit => "edge_exit",
        EntitySprite::ProjectileEnergy => "projectile_energy",
        EntitySprite::FireOrb => "fire_orb",
        EntitySprite::BonusBlockTile => "bonus_block_tile",
        EntitySprite::SpentBlockTile => "spent_block_tile",
        EntitySprite::BridgePlankTile => "bridge_plank_tile",
        EntitySprite::SolidTile => "solid_tile",
        EntitySprite::OneWayTile => "one_way_tile",
        EntitySprite::HazardTile => "hazard_tile",
        EntitySprite::SoftBlinkTile => "soft_blink_tile",
        EntitySprite::HardBlinkTile => "hard_blink_tile",
        EntitySprite::LockWallTile => "lock_wall_tile",
    };
    AssetId::new(format!("sprite.entity.{suffix}"))
}

/// Stable [`AssetId`] for a parallax background layer:
/// `background.parallax.<theme>.<layer>`, the dotted form of
/// `backgrounds/parallax_layers/{theme}_{layer}.png`.
pub fn parallax_layer_asset_id(theme: ParallaxTheme, layer: ParallaxLayerAsset) -> AssetId {
    AssetId::new(format!(
        "background.parallax.{}.{}",
        theme.key(),
        layer.key(),
    ))
}

/// Map from `EntitySprite` to its loaded `Handle<Image>`. A missing file or
/// no-asset mode leaves the key absent, so callers use
/// `get(...) -> Option<&Handle<Image>>`.
#[derive(Default, Clone)]
pub struct EntitySpriteSet {
    pub(super) handles: HashMap<EntitySprite, Handle<Image>>,
}

impl EntitySpriteSet {
    pub fn get(&self, key: EntitySprite) -> Option<&Handle<Image>> {
        self.handles.get(&key)
    }

    pub fn len(&self) -> usize {
        self.handles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.handles.is_empty()
    }
}

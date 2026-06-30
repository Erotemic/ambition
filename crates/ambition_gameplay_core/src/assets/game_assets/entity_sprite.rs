//! The `EntitySprite` vocabulary (single-frame entity placeholders) + its
//! asset-id / manifest / catalog wiring and the `EntitySpriteSet` handle store.

use bevy::prelude::*;
use std::collections::HashMap;

use ambition_asset_manager::{
    AmbitionAssetCatalog, AssetEntry, AssetId, AssetKind, AssetManifest, MissingAssetPolicy,
    PreloadGroup,
};

use super::*;
use crate::persistence::settings::TextureResolutionScale;

/// Single-frame entity sprites keyed off the gen2d manifest.
///
/// Every variant maps to `entities/<lower_snake_case>.png` under the
/// configured sprite folder. Adding a new entry here only requires a path
/// in `relative_path` — loading is data-driven.
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
    // Switch on/off — chosen state-aware in `state_aware_entity_sprite` by the
    // `FeatureView::switch_on` flag (armed = on, disabled = off).
    SwitchArmed,
    SwitchDisabled,
    // Blocks / surfaces
    SolidBlock,
    OneWayPlatform,
    SoftBlinkWall,
    HardBlinkWall,
    PogoOrb,
    ReboundPad,
    MovingPlatform,
    // Loading zones
    DoorZone,
    EdgeExit,
    // Player projectiles (Fireball + Hadouken share the same sprite)
    ProjectileEnergy,
    // 32×32 tile sprites for IntGrid-derived block surfaces. Rendered
    // via `Sprite::image_mode = Tiled` so they REPEAT across the
    // arbitrary aspect ratios that long floors / tall walls produce,
    // instead of stretching one sprite across the whole footprint.
    SolidTile,
    OneWayTile,
    HazardTile,
    SoftBlinkTile,
    HardBlinkTile,
    /// Visual for the encounter-driven "lock wall" block that
    /// `sync_lock_walls` inserts into `world.blocks` while an
    /// encounter is in flight. Uses a dedicated tile so it reads as
    /// "this just appeared" rather than blending into adjacent walls.
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
            Self::SolidBlock => "entities/solid_block.png",
            Self::OneWayPlatform => "entities/one_way_platform.png",
            Self::SoftBlinkWall => "entities/soft_blink_wall.png",
            Self::HardBlinkWall => "entities/hard_blink_wall.png",
            Self::PogoOrb => "entities/pogo_orb.png",
            Self::ReboundPad => "entities/rebound_pad.png",
            Self::MovingPlatform => "entities/moving_platform.png",
            Self::DoorZone => "entities/door_zone.png",
            Self::EdgeExit => "entities/edge_exit.png",
            Self::ProjectileEnergy => "entities/projectile_energy.png",
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
        Self::SolidBlock,
        Self::OneWayPlatform,
        Self::SoftBlinkWall,
        Self::HardBlinkWall,
        Self::PogoOrb,
        Self::ReboundPad,
        Self::MovingPlatform,
        Self::DoorZone,
        Self::EdgeExit,
        Self::ProjectileEnergy,
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
/// The id namespace is `sprite.entity.<lower_snake>` — part of the
/// public asset-catalog contract. Changing the format silently would
/// invalidate authored manifests, so the per-variant suffix mapping is
/// pinned by the test
/// [`tests::every_entity_sprite_has_a_unique_asset_id_in_sprite_entity_namespace`].
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
        EntitySprite::SolidBlock => "solid_block",
        EntitySprite::OneWayPlatform => "one_way_platform",
        EntitySprite::SoftBlinkWall => "soft_blink_wall",
        EntitySprite::HardBlinkWall => "hard_blink_wall",
        EntitySprite::PogoOrb => "pogo_orb",
        EntitySprite::ReboundPad => "rebound_pad",
        EntitySprite::MovingPlatform => "moving_platform",
        EntitySprite::DoorZone => "door_zone",
        EntitySprite::EdgeExit => "edge_exit",
        EntitySprite::ProjectileEnergy => "projectile_energy",
        EntitySprite::SolidTile => "solid_tile",
        EntitySprite::OneWayTile => "one_way_tile",
        EntitySprite::HazardTile => "hazard_tile",
        EntitySprite::SoftBlinkTile => "soft_blink_tile",
        EntitySprite::HardBlinkTile => "hard_blink_tile",
        EntitySprite::LockWallTile => "lock_wall_tile",
    };
    AssetId::new(format!("sprite.entity.{suffix}"))
}

/// Stable [`AssetId`] for a parallax background layer.
///
/// Namespace: `background.parallax.<theme>.<layer>` — same shape as
/// the existing relative path `backgrounds/parallax_layers/{theme}_{layer}.png`
/// but flattened into a dotted logical id.
pub fn parallax_layer_asset_id(theme: ParallaxTheme, layer: ParallaxLayerAsset) -> AssetId {
    AssetId::new(format!(
        "background.parallax.{}.{}",
        theme.key(),
        layer.key(),
    ))
}

/// Build the sandbox's optional-image manifest: every [`EntitySprite`]
/// plus every `(ParallaxTheme, ParallaxLayerAsset)` pair, keyed by
/// stable logical ids and pointed at the configured `sprite_folder` /
/// the canonical `backgrounds/parallax_layers/` tree.
///
/// All entries are optional — [`MissingAssetPolicy::SilentPlaceholder`]
/// so the rendering layer's colored-rectangle fallback fires for any
/// asset Bevy fails to locate. Preload groups:
///
/// - Entity sprites → [`PreloadGroup::SandboxCore`] (used everywhere)
/// - Parallax layers → [`PreloadGroup::Zone`] (per-room art)
///
/// The function is `pub` so tests + future tools (e.g. a content
/// validator) can introspect the registered ids without going through
/// Bevy. Live image loading consumes it through
/// [`load_game_assets`] / [`load_entity_sprites`] / [`load_parallax_layers`].
pub fn sandbox_image_manifest(sprite_folder: &str) -> AssetManifest {
    let mut manifest = AssetManifest::new();
    for &sprite in EntitySprite::ALL {
        let id = entity_sprite_asset_id(sprite);
        let logical_path = format!("{sprite_folder}/{}", sprite.relative_path());
        let entry = AssetEntry::new(id, AssetKind::Image, logical_path)
            .with_missing_policy(MissingAssetPolicy::SilentPlaceholder)
            .with_preload_group(PreloadGroup::SandboxCore);
        // Only author the Embedded candidate when the
        // `static_core_assets` feature is on. Without the feature,
        // `AmbitionAssetSourcePlugin` doesn't insert the bytes — the
        // candidate would resolve to a 404 on WebStatic.
        #[cfg(feature = "static_core_assets")]
        let entry = if let Some(embedded_url) = entity_sprite_embedded_core_url(sprite) {
            entry.with_location(
                ambition_asset_manager::AssetSourceProfile::EmbeddedBinary,
                ambition_asset_manager::AssetLocation::embedded(embedded_url.to_string()),
            )
        } else {
            entry
        };
        manifest.insert(entry);
        for scale in [
            TextureResolutionScale::Half,
            TextureResolutionScale::Quarter,
        ] {
            insert_scaled_image_entry(
                &mut manifest,
                &entity_sprite_asset_id(sprite),
                &format!(
                    "{}/{}",
                    scale.asset_subdir(sprite_folder),
                    sprite.relative_path()
                ),
                scale,
                PreloadGroup::SandboxCore,
            );
        }
    }
    for &theme in ParallaxTheme::ALL {
        for &layer in ParallaxLayerAsset::ALL {
            let id = parallax_layer_asset_id(theme, layer);
            let logical_path = layer.relative_path(theme);
            manifest.insert(
                AssetEntry::new(id.clone(), AssetKind::Image, logical_path)
                    .with_missing_policy(MissingAssetPolicy::SilentPlaceholder)
                    .with_preload_group(PreloadGroup::Zone),
            );
            for scale in [
                TextureResolutionScale::Half,
                TextureResolutionScale::Quarter,
            ] {
                let variant_path = format!(
                    "{}/{}_{}.png",
                    scale.parallax_subdir(),
                    theme.key(),
                    layer.key()
                );
                insert_scaled_image_entry(
                    &mut manifest,
                    &id,
                    &variant_path,
                    scale,
                    PreloadGroup::Zone,
                );
            }
        }
    }
    manifest
}

pub(crate) fn insert_scaled_image_entry(
    manifest: &mut AssetManifest,
    base_id: &AssetId,
    logical_path: &str,
    scale: TextureResolutionScale,
    preload_group: PreloadGroup,
) {
    let Some(id) = crate::assets::sandbox_assets::scaled_asset_id(base_id, scale) else {
        return;
    };
    manifest.insert(
        AssetEntry::new(id, AssetKind::Image, logical_path)
            .with_missing_policy(MissingAssetPolicy::SilentPlaceholder)
            .with_preload_group(preload_group),
    );
}

/// Return the embedded-core URL for an [`EntitySprite`] when that
/// sprite is part of the bounded "core visual" set the
/// `static_core_assets` feature packages. The URL pairs with the
/// `EmbeddedAssetRegistry::insert_asset` call inside
/// `crate::assets::sandbox_assets::register_embedded_core_assets`.
///
/// Out-of-set sprites (parallax layers, breakables, boss variants,
/// LDtk debug tiles) return `None` — they keep the colored-rectangle
/// fallback on `WebStatic` / `BundledStatic` until a follow-up slice
/// packages them.
#[cfg_attr(not(feature = "static_core_assets"), allow(dead_code))]
pub fn entity_sprite_embedded_core_url(sprite: EntitySprite) -> Option<&'static str> {
    use crate::assets::sandbox_assets::embedded_core;
    match sprite {
        EntitySprite::ChestClosed => Some(embedded_core::SPRITE_CHEST_CLOSED_URL),
        EntitySprite::ChestOpen => Some(embedded_core::SPRITE_CHEST_OPEN_URL),
        EntitySprite::PickupHealth => Some(embedded_core::SPRITE_PICKUP_HEALTH_URL),
        EntitySprite::PickupCurrency => Some(embedded_core::SPRITE_PICKUP_CURRENCY_URL),
        EntitySprite::PickupAbility => Some(embedded_core::SPRITE_PICKUP_ABILITY_URL),
        EntitySprite::DoorZone => Some(embedded_core::SPRITE_DOOR_ZONE_URL),
        EntitySprite::EdgeExit => Some(embedded_core::SPRITE_EDGE_EXIT_URL),
        EntitySprite::ProjectileEnergy => Some(embedded_core::SPRITE_PROJECTILE_ENERGY_URL),
        EntitySprite::SolidTile => Some(embedded_core::SPRITE_SOLID_TILE_URL),
        EntitySprite::OneWayTile => Some(embedded_core::SPRITE_ONE_WAY_TILE_URL),
        EntitySprite::HazardTile => Some(embedded_core::SPRITE_HAZARD_TILE_URL),
        EntitySprite::BossCore => Some(embedded_core::SPRITE_BOSS_CORE_URL),
        _ => None,
    }
}

/// Convenience: wrap [`sandbox_image_manifest`] in a Bevy-side catalog
/// resource. Constructed per [`load_game_assets`] call so a future
/// `--sprite-folder` toggle re-resolves cleanly.
///
/// Only the tests in this file build the catalog this way today;
/// production startup uses `build_sandbox_catalog` instead. Kept
/// pub so future `--sprite-folder` toggles can adopt it without a
/// visibility change.
#[cfg_attr(not(test), allow(dead_code))]
pub fn build_sandbox_image_catalog(sprite_folder: &str) -> AmbitionAssetCatalog {
    AmbitionAssetCatalog::new(sandbox_image_manifest(sprite_folder))
}

/// Map from `EntitySprite` to its loaded `Handle<Image>`. Missing handles
/// (file absent on disk OR no-asset mode) simply aren't keyed, so callers
/// just consult `get(...) -> Option<&Handle<Image>>`.
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

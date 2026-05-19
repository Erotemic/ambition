//! Bevy visual synchronization for engine state.
//!
//! Render-only component tags and visual sync systems. This module mirrors
//! player and world state from ECS components into Bevy transforms / sprites.
//!
//! ## Submodule layout (post-2026-05-09 split)
//!
//! - [`primitives`] — marker components ([`SceneEntities`],
//!   [`PlayerVisual`], [`HudText`], [`QuestPanelText`], [`RoomVisual`],
//!   [`FeatureVisual`], [`HealthOverlayVisual`]) plus color / Z /
//!   feature-kind helpers and `spawn_world_label`.
//! - [`actors`] — per-frame sync of player + enemy + boss sprites
//!   and animation. Owns [`sync_visuals`], [`animate_player`],
//!   [`animate_characters`], [`animate_bosses`], [`upgrade_enemy_sprites`],
//!   [`upgrade_boss_sprites`].
//! - [`world`] — static room visuals. Owns [`spawn_room_visuals`],
//!   [`spawn_block`], [`spawn_loading_zone`], [`spawn_grid`],
//!   [`spawn_room_object`].
//! - [`features`] — runtime-spawned feature visuals via
//!   [`spawn_dynamic_feature_visuals`].
//! - [`health`] — debug health-bar overlay
//!   ([`sync_health_overlays`]).
//! - [`parallax`] — optional generated sky/background/atmosphere layers
//!   ([`spawn_parallax_layers`], [`sync_parallax_layers`]).
//! - [`camera`] — player-following camera with eased zoom around
//!   encounter transitions ([`camera_follow`]).

mod actors;
mod camera;
mod features;
mod health;
mod parallax;
mod pirate_rider;
mod primitives;
mod world;

pub use actors::{
    animate_bosses, animate_characters, animate_player, animate_props, apply_hide_sprites_override,
    sync_visuals, upgrade_boss_sprites, upgrade_enemy_sprites, upgrade_npc_sprites,
};
pub use pirate_rider::{sync_pirate_rider_visuals, PirateRiderVisual};
pub use camera::{camera_follow, CameraViewState};
pub use features::spawn_dynamic_feature_visuals;
pub use health::sync_health_overlays;
pub use parallax::{spawn_parallax_layers, sync_parallax_layers, ParallaxLayerVisual};
pub use primitives::{
    block_color, switch_on_color, FeatureVisual, HealthOverlayVisual, HudText,
    LoadingZoneVisual, LockWallVisual, PlayerSpriteBaseline, PlayerVisual, PropVisual,
    QuestPanelText, RoomVisual, SceneEntities,
};
pub use world::{
    spawn_block, spawn_grid, spawn_loading_zone, spawn_room_object, spawn_room_prop,
    spawn_room_visuals, sync_lock_wall_visuals,
};

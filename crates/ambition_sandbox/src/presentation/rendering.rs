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

pub mod actors;
mod camera;
mod features;
mod health;
mod parallax;
mod pirate_rider;
mod primitives;
mod world;

pub use actors::{
    animate_bosses, animate_characters, animate_player, animate_props, apply_hide_sprites_override,
    apply_placeholder_sprites_override, sync_visuals, upgrade_boss_sprites, upgrade_enemy_sprites,
    upgrade_npc_sprites,
};
pub use camera::{camera_follow, CameraViewState};
pub use features::spawn_dynamic_feature_visuals;
pub use health::sync_health_overlays;
pub use parallax::{spawn_parallax_layers, sync_parallax_layers};
pub use pirate_rider::sync_pirate_rider_visuals;
pub use primitives::{
    HudText, LoadingZoneVisual, PlayerSpriteBaseline, PlayerVisual, QuestPanelText,
    RoomScopedEntity, RoomVisual, SceneEntities,
};
pub use world::{spawn_room_visuals, sync_lock_wall_visuals};

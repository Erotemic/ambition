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
mod deep_dream;
mod features;
mod health;
mod parallax;
mod pirate_rider;
mod pirate_weapon;
mod primitives;
mod world;

pub use actors::{
    animate_bosses, animate_characters, animate_player, apply_hide_sprites_override,
    apply_placeholder_sprites_override, sync_visuals, upgrade_boss_sprites, upgrade_enemy_sprites,
    upgrade_npc_sprites, BoundFeatureKind,
};
pub use camera::{camera_follow, CameraViewState};
pub use health::sync_health_overlays;
// Re-exported so the simulation side (e.g. `EnemyRuntime::update`
// in `content/features/enemies.rs`) can place projectile-spawn
// origins at the same hand position the visual lays the gun-sword
// on. Keeps "where the muzzle is" defined in ONE module.
pub use parallax::{spawn_parallax_layers, sync_parallax_layers};
pub use pirate_weapon::rider_hand_world_pos;
pub use primitives::{
    HudText, LoadingZoneVisual, PlayerSpriteBaseline, PlayerVisual, QuestPanelText,
    RoomScopedEntity, RoomVisual, SceneEntities,
};
pub use world::{spawn_room_visuals, sync_lock_wall_visuals};

/// Module-local Bevy plugin: schedules player-bound visual systems
/// (morph-ball sprite + bubble-shield sprite). Each follows the same
/// pattern — build the texture once at startup, spawn lazily once the
/// player entity exists, sync visibility / tint every frame after
/// `sync_visuals` has mirrored the player transform.
///
/// Carved out of `app/plugins.rs::install_player_visual_systems` per
/// OVERNIGHT-TODO #6. Lives in `presentation/rendering.rs` because
/// both subsystems chain `.after(sync_visuals)` and are presentation-
/// only — the body_mode + bubble_shield modules own the systems but
/// the schedule ordering is a presentation concern.
pub struct PlayerVisualSchedulePlugin;

impl bevy::prelude::Plugin for PlayerVisualSchedulePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        use bevy::prelude::{IntoScheduleConfigs, Startup, Update};
        app.add_systems(Startup, crate::body_mode::build_morph_ball_sprite)
            .add_systems(
                Update,
                (
                    crate::body_mode::spawn_morph_ball_visual,
                    crate::body_mode::sync_morph_ball_visual,
                )
                    .chain()
                    .after(actors::sync_visuals),
            )
            // Bubble shield visual: similar pattern — toggle / tint every
            // frame from `PlayerBody.shielding` and `PlayerBody.parrying`.
            // Must run after `write_player_ecs_components` so `PlayerBody`
            // is current.
            .add_systems(
                Startup,
                crate::player::bubble_shield::build_bubble_shield_sprite,
            )
            .add_systems(
                Update,
                (
                    crate::player::bubble_shield::spawn_bubble_shield_visual,
                    crate::player::bubble_shield::sync_bubble_shield_visual,
                )
                    .chain()
                    .after(actors::sync_visuals),
            );
    }
}

/// Module-local Bevy plugin: schedules the per-frame visual animation
/// chain into [`crate::app::SandboxSet::PresentationVisualSync`].
///
/// Spawns dynamic feature visuals first (so `sync_visuals` finds them
/// the same frame), then mirrors transforms / sprite atlas indices,
/// overrides gnu_ton boss z, upgrades enemy / boss sprites, ticks all
/// the per-actor animators, and finishes with the pirate rider
/// composite. Carved out of
/// `app/plugins.rs::install_visual_animation_systems` per
/// OVERNIGHT-TODO #6 — every system in this chain lives under
/// `presentation/rendering/`.
///
/// Pinned `.after(map_menu::handle_map_menu_hotkeys)` because the
/// map-menu input is the last presentation-input system this set
/// runs after; ordering is per the presentation install chain.
pub struct PresentationVisualAnimationPlugin;

impl bevy::prelude::Plugin for PresentationVisualAnimationPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        use bevy::prelude::{IntoScheduleConfigs, Update};
        deep_dream::add_puppy_slug_deep_dream_material_plugin(app);
        app.add_systems(
            Update,
            (
                // Spawn visual entities for encounter-spawned enemies
                // BEFORE sync_visuals reads positions for them.
                features::spawn_dynamic_feature_visuals,
                actors::sync_visuals,
                // Override gnu_ton boss z AFTER sync_visuals (which resets
                // it to `feature_z(Boss) = 11.0`) so the body silhouette
                // sits behind one-way platforms.
                actors::apply_gnu_ton_body_z,
                actors::upgrade_enemy_sprites,
                actors::upgrade_boss_sprites,
                // Attach the experimental material overlay after enemy sprite
                // upgrade has produced a real atlas-backed Puppy Slug sprite.
                deep_dream::attach_puppy_slug_deep_dream_overlays,
                actors::animate_player,
                actors::animate_characters,
                // Mirror the current atlas frame into the overlay after the
                // character animator has advanced for this frame.
                deep_dream::sync_puppy_slug_deep_dream_overlays,
                deep_dream::cleanup_puppy_slug_deep_dream_overlays,
                actors::animate_props,
                actors::animate_bosses,
                // Mirror parent atlas index + tint onto the hands overlay
                // after `animate_bosses` has updated the parent's frame.
                actors::sync_gnu_ton_hands,
                // Pirate rider composite — reads ECS actor state and
                // spawns/despawns presentation entities each frame, so
                // it belongs in `PresentationVisualSync` (after
                // `FeatureViewSync`) alongside `sync_visuals` rather
                // than the projectile/VFX batch. Placing it here means
                // a room reset's actor despawn is observed the same
                // frame the rider visual disappears — no stale
                // rider-on-no-shark across resets/transitions.
                pirate_rider::sync_pirate_rider_visuals,
                // Gun-sword visual on the rider — runs after the
                // rider visual so the weapon mounts on top of the
                // pirate sprite in the same frame.
                pirate_weapon::sync_pirate_weapon_visuals,
            )
                .chain()
                .in_set(crate::app::SandboxSet::PresentationVisualSync)
                .after(crate::map_menu::handle_map_menu_hotkeys),
        );
    }
}

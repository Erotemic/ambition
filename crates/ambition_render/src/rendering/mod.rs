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
//!   [`animate_characters`], [`animate_bosses`], [`upgrade_actor_sprites`],
//!   [`upgrade_boss_sprites`].
//! - [`world`] — static room visuals. Owns [`spawn_room_visuals`],
//!   [`spawn_block`], [`spawn_loading_zone`], [`spawn_grid`],
//!   [`spawn_room_object`].
//! - [`features`] — runtime-spawned feature visuals via
//!   [`spawn_dynamic_feature_visuals`].
//! - [`health`] — debug health-bar overlay
//!   ([`sync_health_overlays`]).
//! - [`nameplates`] — player-facing actor/door labels
//!   ([`sync_actor_nameplates`]).
//! - [`parallax`] — optional generated sky/background/atmosphere layers
//!   ([`spawn_parallax_layers`], [`sync_parallax_layers`]).
//! - [`camera`] — player-following camera with eased zoom around
//!   encounter transitions ([`camera_follow`]).

pub mod actors;
pub mod bubble_shield;
mod camera;
mod deep_dream;
mod features;
pub mod gate_portal_visuals;
pub mod gravity_visuals;
mod health;
mod hit_flash;
mod item_visuals;
pub mod mark_beacon;
pub mod morph_ball;
mod nameplates;
mod parallax;
mod pirate_weapon;
mod primitives;
pub mod projectile_visuals;
pub(crate) mod sheet_atlas;
pub mod shrine_visuals;
pub(crate) mod slash_visuals;
mod world;

pub use actors::{
    animate_bosses, animate_characters, animate_player, apply_hide_sprites_override,
    apply_placeholder_sprites_override, refresh_player_sprites_on_game_assets_change,
    refresh_prop_sprites_on_game_assets_change, sync_visuals, upgrade_actor_sprites,
    upgrade_boss_sprites,
};
// `BoundFeatureKind` moved to `combat` (sim owns it); re-exported here
// so existing render call sites resolve unchanged.
pub use ambition_gameplay_core::combat::BoundFeatureKind;
// `manage_gradient_lane_visual` + `GradientLaneVisual` stay
// module-private; the schedule registration uses
// `actors::manage_gradient_lane_visual` directly so no outside
// callers need a re-export.
pub use ambition_sim_view::camera_snapshot::{CameraSnapshot2d, SceneCaptureRequest};
#[cfg(feature = "portal_render")]
pub use camera::publish_portal_camera_clamp;
pub use camera::{camera_follow, publish_camera_viewport, CameraViewState};
pub use health::{sync_boss_health_bar_overlay, sync_health_overlays};
pub use nameplates::{
    sync_actor_nameplates, ActorNameplatePresentationPlugin, ActorNameplateSet,
    ActorNameplateSettings, ActorNameplateVisual, DoorNameplateSource,
};
// Re-exported so simulation/effects code can place projectile-spawn
// origins at the same hand position the visual lays the gun-sword on.
// Keeps "where the muzzle is" defined in one module.
pub use ambition_gameplay_core::features::rider_hand_world_pos;
#[cfg(feature = "portal_render")]
pub use parallax::sync_portal_capture_parallax_layers;
pub use parallax::{
    refresh_parallax_layers_on_quality_change, spawn_parallax_layers, sync_parallax_layers,
};
pub use primitives::{
    HudText, LoadingZoneVisual, PlayerSpriteBaseline, PlayerVisual, PropVisual, QuestPanelText,
    RoomScopedEntity, RoomVisual, SceneEntities,
};
pub use world::{
    refresh_entity_sprite_handles_on_game_assets_change, spawn_room_visuals, sync_lock_wall_visuals,
};

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
        app.add_systems(Startup, morph_ball::build_morph_ball_sprite)
            .add_systems(
                Update,
                (
                    morph_ball::spawn_morph_ball_visual,
                    morph_ball::sync_morph_ball_visual,
                )
                    .chain()
                    .after(actors::sync_visuals),
            )
            // Bubble shield visual: similar pattern — toggle / tint every
            // frame from `BodyShieldState::active` and
            // `BodyShieldState::parrying()`.
            .add_systems(Startup, bubble_shield::build_bubble_shield_sprite)
            .add_systems(
                Update,
                (
                    bubble_shield::spawn_bubble_shield_visual,
                    bubble_shield::sync_bubble_shield_visual,
                )
                    .chain()
                    .after(actors::sync_visuals),
            )
            // Load held-item prop sprites at startup.
            .add_systems(Startup, item_visuals::load_item_art)
            .add_systems(
                Update,
                (
                    item_visuals::sync_ground_item_visuals.after(actors::sync_visuals),
                    item_visuals::sync_held_item_visual.after(actors::sync_visuals),
                    item_visuals::sync_held_projectile_visuals.after(actors::sync_visuals),
                    shrine_visuals::sync_shrine_visual.after(actors::sync_visuals),
                    shrine_visuals::animate_shrine_visuals.after(actors::animate_props),
                    slash_visuals::spawn_slash_effects,
                    slash_visuals::animate_slash,
                    mark_beacon::sync_mark_beacon_visual.after(actors::sync_visuals),
                ),
            );

        // Portal-gun visuals (placed-portal quads, partial-transit pieces, the
        // disorientation / mode indicators) now live in the reusable
        // `ambition_portal_presentation` crate; the sandbox adds its plugin,
        // places its set, and bridges the host seams (world frame, scene-body
        // tag, gun art — see `ambition_gameplay_core::portal::host_adapter`). Gravity visuals
        // and the F7 dev off-switch stay host-side. All of it only compiles
        // with the portal mechanic + its render feature.
        #[cfg(feature = "portal_render")]
        {
            use ambition_portal_presentation::{PortalPresentationPlugin, PortalPresentationSet};
            app.add_plugins(PortalPresentationPlugin::default());
            // The Ambition host-adapter glue (world-frame/viewer/focus/debug
            // seam publishers, scene-body tagging, dev toggles, gun art) is
            // `PortalObservationPlugin`, added by the HOST (E4 slice 20) —
            // render holds exactly ONE label dependency on its set below,
            // never a system registration.
            // Portal body-copy visuals must run after the player animator, not
            // only after `sync_visuals`: trimmed sprites can update
            // `Sprite::custom_size` and `Anchor` during animation, and the
            // portal exit copy must clone that final per-frame render basis.
            app.configure_sets(
                Update,
                PortalPresentationSet
                    .after(actors::animate_player)
                    .after(camera::camera_follow)
                    .after(ambition_gameplay_core::portal::PortalObservationSet),
            );
            app.add_systems(
                Update,
                (
                    gravity_visuals::sync_gravity_switch_visual.after(actors::sync_visuals),
                    gravity_visuals::sync_gravity_zone_visual.after(actors::sync_visuals),
                ),
            );
        }
    }
}

/// Module-local Bevy plugin: schedules the per-frame visual animation
/// chain into [`ambition_gameplay_core::schedule::SandboxSet::PresentationVisualSync`].
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
        hit_flash::add_hit_flash_material_plugin(app);
        // The per-actor pose read-model (`ActorAnimIndex`) is rebuilt SIM-side
        // (E4 slice 19: `FeatureViewSyncSchedulePlugin` owns the resource and
        // the overlay-advance + rebuild pair, in the FeatureViewSync tail this
        // chain is ordered after) — presentation is a pure consumer.
        app.add_systems(
            Update,
            (
                // Spawn visual entities for encounter-spawned enemies
                // BEFORE sync_visuals reads positions for them.
                features::spawn_dynamic_feature_visuals,
                actors::sync_visuals,
                // Override a split-layer boss's body z AFTER sync_visuals (which
                // resets it to `feature_z(Boss) = 11.0`) so the body silhouette
                // sits behind one-way platforms.
                actors::apply_boss_split_body_z,
                actors::upgrade_actor_sprites,
                // Grouped (parallel within their chain slot): player-sprite and
                // prop-sprite quality refreshes touch disjoint entity families, so
                // they need no order between them. Nesting also keeps this chained
                // tuple within Bevy's 20-system arity after the pose-rebuild add.
                (
                    actors::refresh_player_sprites_on_game_assets_change,
                    actors::refresh_prop_sprites_on_game_assets_change,
                ),
                actors::upgrade_boss_sprites,
                // Attach the experimental material overlay after enemy sprite
                // upgrade has produced a real atlas-backed Puppy Slug sprite.
                deep_dream::attach_puppy_slug_deep_dream_overlays,
                // Attach the hit-flash white-silhouette overlay to every
                // character sprite (player + enemies + NPCs + bosses) once
                // its texture / atlas is loaded. Sized as a sibling mesh
                // — same world-space sync pattern as deep_dream.
                hit_flash::attach_hit_flash_overlays,
                actors::animate_player,
                actors::animate_characters,
                // Mirror the current atlas frame into the overlay after the
                // character animator has advanced for this frame.
                deep_dream::sync_puppy_slug_deep_dream_overlays,
                deep_dream::cleanup_puppy_slug_deep_dream_overlays,
                // Mirror the source sprite's atlas + transform into the
                // hit-flash overlay and gate visibility on the current
                // hit_flash timer. Runs after the animator so the overlay
                // tracks the same frame the source draws this tick.
                hit_flash::sync_hit_flash_overlays,
                hit_flash::cleanup_hit_flash_overlays,
                actors::animate_props,
                actors::animate_bosses,
                // HazardColumn vertical-column visual — yellow during
                // telegraph, red during strike. Runs after
                // `animate_bosses` so it can read the boss's
                // `BossAttackState` after the brain has populated it
                // upstream.
                actors::manage_gradient_lane_visual,
                // Mirror parent atlas index + tint onto a split-layer boss's
                // overlay after `animate_bosses` has updated the parent's frame.
                actors::sync_boss_split_overlay,
                // Gun-sword visual on the rider — composite pirate-
                // on-shark spawns are two linked entities (mount +
                // rider) and the rider entity draws via the standard
                // upgrade_actor_sprites path. The gun-sword sprite is
                // the only piece NOT covered by the standard sheet
                // (it's an over-hand prop tied to aim direction), so
                // this system queries the rider entity directly via
                // its [`RidingOn`] component and layers the weapon on
                // top.
                pirate_weapon::sync_pirate_weapon_visuals,
            )
                .chain()
                .in_set(ambition_gameplay_core::schedule::SandboxSet::PresentationVisualSync)
                .after(ambition_gameplay_core::menu::map::handle_map_menu_hotkeys),
        );

        // Rebuild the active room's static visuals + parallax when the sim asks
        // for it (sandbox reset). The sim emits `RespawnRoomVisualsRequested`; we
        // own the actual spawn here so the sim never imports the render layer.
        app.add_systems(Update, world::respawn_room_visuals_on_request);
    }
}

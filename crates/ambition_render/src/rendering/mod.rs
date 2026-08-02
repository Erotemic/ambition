//! Bevy visual synchronization for engine state.
//!
//! Render-only component tags and visual sync systems. This module mirrors
//! player and world state from ECS components into Bevy transforms / sprites.
//!
//! ## Submodule layout (post-2026-05-09 split)
//!
//! - [`primitives`] — marker components ([`PlayerVisual`],
//!   [`HudText`], [`QuestPanelText`], [`RoomVisual`],
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
//!   ([`sync_actor_nameplates`]). It publishes each plate's WANTED anchor and
//!   opacity; it does not place it.
//! - [`label_layout`] — the ONE ranked placement pass over every world-label
//!   family ([`layout_world_labels`]). Single writer of a label's transform,
//!   visibility and colour, so authored signage, fixture plates and actor
//!   plates are ranked against each other instead of each family only against
//!   itself.
//! - [`parallax`] — optional generated sky/background/atmosphere layers
//!   ([`spawn_parallax_layers`], [`sync_parallax_layers`]).
//! - [`camera`] — player-following camera with eased zoom around
//!   encounter transitions ([`camera_follow`]).
//! - [`debug_viz`] — the engine-generic F1 debug gizmo layers (world
//!   blocks, surface chains, read-model body/feature boxes) + the opt-in
//!   [`debug_viz::DebugVizPlugin`] a game host adds to get them.

/// **Every pass that decides an actor sprite's handle, tint or visibility.**
///
/// The dev-tool sprite OVERRIDES (placeholder-sprites, hide-sprites) have to run
/// after all of them or the toggle is nondeterministic — the comment at that
/// registration records sprites "sporadically remaining visible" when Bevy chose
/// the other order. Saying so used to take four `.after` edges naming four
/// functions in two crates, three of them reached through the facade from the
/// app.
///
/// ⚠ FOUR members, and unusually for this campaign that is the whole point
/// rather than a compromise: the boundary IS "all of them", so the set is
/// exactly as wide as the claim. A new sprite pass joins here and the override
/// keeps working; under the old style it silently did not.
///
/// ⚠ `sync_projectile_visuals` is registered by `ambition_platformer2d_host`,
/// not by this crate's plugin, so the set spans a composition boundary. That is
/// legitimate — a set is a NAME, and the crate that owns the name need not own
/// every registration — but it does mean a composition that installs the render
/// plugin and not the host gets a three-member set. The override is `.after` it
/// either way, which stays correct.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SpriteVisualSync;

pub mod actors;
pub mod bubble_shield;
mod camera;
pub mod debug_viz;
pub mod deferred_write_safety;
mod features;
pub mod gate_portal_visuals;
pub mod gravity_visuals;
mod health;
mod hit_flash;
mod item_visuals;
pub mod label_layout;
pub mod mark_beacon;
pub mod morph_ball;
mod nameplates;
mod parallax;
mod primitives;
pub mod projectile_visuals;
pub(crate) mod sheet_atlas;
pub mod shrine_visuals;
pub(crate) mod slash_visuals;
mod unauthored_volumes;
mod wielded_item_visuals;
mod world;

pub use actors::{
    actor_sprite_path_owns, animate_bosses, animate_characters, animate_feature_sprites,
    animate_player, apply_hide_sprites_override, apply_placeholder_sprites_override,
    refresh_player_sprites_on_game_assets_change, refresh_prop_sprites_on_game_assets_change,
    sync_visuals, upgrade_actor_sprites, upgrade_boss_sprites, BossAnimation,
    PlayerSpriteCharacter,
};
// `BoundFeatureKind` lives with the foundation feature taxonomy; re-exported
// here so existing render call sites resolve unchanged.
pub use ambition_platformer2d_shared_tangle::feature_kind::BoundFeatureKind;
// `manage_gradient_lane_visual` + `GradientLaneVisual` stay
// module-private; the schedule registration uses
// `actors::manage_gradient_lane_visual` directly so no outside
// callers need a re-export.
pub use ambition_sim_view::camera_snapshot::{CameraSnapshot2d, SceneCaptureRequest};
#[cfg(feature = "portal_render")]
pub use camera::publish_portal_camera_clamp;
pub use camera::{camera_follow, CameraViewState};
/// The presentation FLOOR's marker: a feature the sim published that no render
/// family has drawn. Exported because it is the readable form of "this room is
/// not presentable yet" — the room-transition cover waits on it.
pub use features::UnclaimedBodyPlaceholder;
pub use health::{sync_boss_health_bar_overlay, sync_health_overlays};
pub use label_layout::{
    layout_world_labels, WorldLabel, WorldLabelFamily, WorldLabelLayoutPlugin, WorldLabelLayoutSet,
    WorldLabelLayoutSettings,
};
pub use nameplates::{
    sync_actor_nameplates, ActorNameplatePresentationPlugin, ActorNameplateSet,
    ActorNameplateSettings, ActorNameplateVisual, DoorNameplateSource,
};
#[cfg(feature = "portal_render")]
pub use parallax::sync_portal_capture_parallax_layers;
pub use parallax::{
    ensure_active_room_parallax_theme,
    refresh_parallax_layers_on_quality_change,
    spawn_parallax_layers,
    sync_parallax_layers,
    // ⚠ the MARKER, not just the systems. A consumer could install the whole
    // parallax family and had no way to ask whether a backdrop existed — the
    // component was behind a private module, so "is my sky drawn" was a question
    // only this crate could answer. `fixtures/external_consumer` asks it now,
    // which is the consumer that makes this worth exporting.
    ParallaxLayerVisual,
};
pub use primitives::{
    FeatureVisual, HudText, LoadingZoneVisual, PlayerSpriteBaseline, PlayerVisual, PropVisual,
    QuestPanelText, RoomScopedEntity, RoomVisual,
};
// Game-supplied art map for walk-into world items; the reusable renderer owns the
// seam, each game fills it with its own pickups' images.
pub use item_visuals::WorldItemArt;
pub use wielded_item_visuals::{
    WieldedItemVisualAppExt, WieldedItemVisualCatalog, WieldedItemVisualSpec,
};
pub use world::{
    refresh_entity_sprite_handles_on_game_assets_change, spawn_room_visuals,
    spawn_surface_chain_visuals, sync_lock_wall_visuals, sync_removed_block_visuals,
};

/// The public seam for CONTENT-OWNED per-actor overlay presentation: sibling
/// meshes/materials that decorate animated actor sprites (e.g. Ambition's
/// puppy-slug deep-dream pass). [`PresentationVisualAnimationPlugin`] positions
/// this set inside the presentation visual-sync chain — after the character
/// animators have advanced the frame the overlays mirror, before the renderer's
/// own hit-flash mirror — and gates it on session readiness. A game adds its
/// named overlay systems `.in_set(ActorOverlaySet)` from its content crate; the
/// reusable renderer names no game's look.
#[derive(bevy::prelude::SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ActorOverlaySet;

/// Presentation systems below consume session-created resources and entities.
/// During startup, loading, and the launcher there is deliberately no gameplay
/// session, so the complete per-frame presentation graph must stay dormant.
fn session_presentation_is_ready(
    gate: Option<
        bevy::prelude::Res<ambition_platformer2d_shared_tangle::lifecycle::SessionGatedSimulation>,
    >,
    active: Option<
        bevy::prelude::Res<ambition_platformer2d_shared_tangle::lifecycle::ActiveSessionScope>,
    >,
    roots: bevy::prelude::Query<&ambition_platformer2d_shared_tangle::lifecycle::SessionRoot>,
    // The primary player body IS the readiness signal now: presentation runs only
    // once the session has lowered its home avatar. Derived from the canonical
    // marker instead of a process-global handle bag that outlives its session.
    primary_player: bevy::prelude::Query<
        (),
        (
            bevy::prelude::With<ambition_platformer2d_shared_tangle::markers::PlayerEntity>,
            bevy::prelude::With<ambition_platformer2d_shared_tangle::markers::PrimaryPlayer>,
        ),
    >,
) -> bool {
    let exact_world = roots.single().is_ok_and(|root| {
        gate.is_none()
            || active.as_deref().and_then(
                ambition_platformer2d_shared_tangle::lifecycle::ActiveSessionScope::current,
            ) == Some(root.0)
    });
    exact_world && !primary_player.is_empty()
}

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
        app.init_resource::<item_visuals::FailedItemArt>()
            .add_systems(Startup, morph_ball::build_morph_ball_sprite)
            .add_systems(
                Update,
                (
                    morph_ball::spawn_morph_ball_visual,
                    morph_ball::sync_morph_ball_visual.in_set(SpriteVisualSync),
                )
                    .chain()
                    .after(actors::sync_visuals)
                    .run_if(session_presentation_is_ready),
            )
            // Bubble shield visual: similar pattern — toggle / tint every
            // frame from `BodyShieldState::active` and
            // `BodyShieldState::parrying()`.
            .add_systems(Startup, bubble_shield::build_bubble_shield_sprite)
            .add_systems(
                Update,
                (
                    bubble_shield::spawn_bubble_shield_visual,
                    bubble_shield::sync_bubble_shield_visual.in_set(SpriteVisualSync),
                )
                    .chain()
                    .after(actors::sync_visuals)
                    .run_if(session_presentation_is_ready),
            )
            // Resolve every provider's contributed held-item art (the
            // `HeldItemArtManifest` data) into loaded `HeldItemArt` handles.
            .add_systems(Startup, item_visuals::build_held_item_art)
            // Resolve every provider's contributed walk-into pickup art (the
            // `WorldItemArtManifest` data) into loaded `WorldItemArt` handles.
            .add_systems(Startup, item_visuals::build_world_item_art)
            // Deliberately NOT session-gated: an art file that failed to load is
            // a fact about the build, and waiting for a session to be presentable
            // to say so is how the spark blossom stayed quiet.
            .add_systems(Update, item_visuals::report_unloadable_item_art)
            .add_systems(
                Update,
                (
                    item_visuals::sync_ground_item_visuals.after(actors::sync_visuals),
                    item_visuals::sync_world_item_visuals.after(actors::sync_visuals),
                    // Despawn any authored block the collision overlay is subtracting
                    // this frame (a broken brick, a gate-dropped wall) — the render
                    // half of `removed_block_names`. Generic; every game gets it.
                    sync_removed_block_visuals,
                    item_visuals::sync_held_item_visual.after(actors::sync_visuals),
                    item_visuals::sync_held_projectile_visuals.after(actors::sync_visuals),
                    shrine_visuals::sync_shrine_visual.after(actors::sync_visuals),
                    shrine_visuals::animate_shrine_visuals.after(actors::animate_props),
                    unauthored_volumes::draw_unauthored_attack_volumes,
                    slash_visuals::spawn_slash_effects,
                    // After the spawn, so a swing born this frame is already on
                    // its body when the frame is drawn.
                    slash_visuals::follow_slash_owner.after(slash_visuals::spawn_slash_effects),
                    slash_visuals::animate_slash,
                    mark_beacon::sync_mark_beacon_visual.after(actors::sync_visuals),
                )
                    .after(item_visuals::report_unloadable_item_art)
                    .run_if(session_presentation_is_ready),
            );

        // Portal-gun visuals (placed-portal quads, partial-transit pieces, the
        // disorientation / mode indicators) now live in the reusable
        // `ambition_portal2d_presentation` crate; the sandbox adds its plugin,
        // places its set, and bridges the host seams (world frame, scene-body
        // tag, gun art — see `ambition_portal2d::host_adapter`). Gravity visuals
        // and the F7 dev off-switch stay host-side. All of it only compiles
        // with the portal mechanic + its render feature.
        #[cfg(feature = "portal_render")]
        {
            use ambition_portal2d_presentation::{PortalPresentationPlugin, PortalPresentationSet};
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
                    .after(ambition_portal2d_presentation::PortalObservationSet),
            );
            app.add_systems(
                Update,
                (
                    gravity_visuals::sync_gravity_switch_visual.after(actors::sync_visuals),
                    gravity_visuals::sync_gravity_zone_visual.after(actors::sync_visuals),
                )
                    .run_if(session_presentation_is_ready),
            );
        }
    }
}

/// Module-local Bevy plugin: schedules the per-frame visual animation
/// chain into [`ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith::PresentationVisualSync`].
///
/// Spawns dynamic feature visuals first (so `sync_visuals` finds them
/// the same frame), then mirrors transforms / sprite atlas indices,
/// upgrades enemy / boss sprites, ticks all the per-actor animators,
/// and finishes with provider-authored wielded-item overlays. Carved out of
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
        // Every visual below draws from the frame-clock presented poses, so the
        // resample must already have run this frame. Schedule-local edge: both
        // sides live in `Update` for all three sim hosts.
        app.configure_sets(
            Update,
            ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith::PresentationVisualSync
                .after(ambition_sim_view::PresentedPoseSet),
        );
        app.init_resource::<wielded_item_visuals::WieldedItemVisualCatalog>();
        app.init_resource::<slash_visuals::SlashSources>();
        // Open, content-owned projectile art registry (empty until a game's
        // content crate registers looks). The renderer resolves each in-flight
        // projectile's `ProjectileVisualId` through it.
        app.init_resource::<ambition_projectiles::ProjectileVisualCatalog>();
        hit_flash::add_hit_flash_material_plugin(app);
        // Position the content-owned actor-overlay seam: after the character
        // animator (overlays mirror the frame it just advanced), before the
        // hit-flash mirror (the flash silhouette reads the sprite state overlay
        // syncs may tint). The set carries the session gate so member systems
        // stay dormant outside a running session, exactly like the chain below.
        app.configure_sets(
            Update,
            ActorOverlaySet
                .after(actors::animate_characters)
                .before(hit_flash::sync_hit_flash_overlays)
                .in_set(
                    ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith::PresentationVisualSync,
                )
                .run_if(session_presentation_is_ready),
        );
        // The per-actor pose read-model (`ActorAnimIndex`) is rebuilt SIM-side
        // (E4 slice 19: `FeatureViewSyncSchedulePlugin` owns the resource and
        // the overlay-advance + rebuild pair, in the FeatureViewSync tail this
        // chain is ordered after) — presentation is a pure consumer.
        app.add_systems(
            Update,
            (
                // Spawn visual entities for encounter-spawned enemies
                // BEFORE sync_visuals reads positions for them, and retire the
                // ones whose sim feature is gone (an expired loot drop) so a
                // room doesn't accumulate invisible sprites.
                features::spawn_dynamic_feature_visuals,
                features::despawn_dead_dynamic_feature_visuals,
                // The reusable selected-character binder: install (and rebind) the
                // worn character's sheet/animator/anchor from the canonical
                // `WornCharacter` identity. Runs BEFORE the fallback so a
                // worn-identity player never gets the neutral rectangle. The app
                // and every standalone demo consume this ONE path.
                actors::bind_worn_character_presentation,
                // Safety net for a bare PlayerVisual with no worn identity (a
                // minimal shell): give it a drawable fallback before sync_visuals
                // queries `&mut Sprite`.
                actors::ensure_player_visual_sprite,
                actors::sync_visuals.in_set(SpriteVisualSync),
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
                // Attach the hit-flash white-silhouette overlay to every
                // character sprite (player + enemies + NPCs + bosses) once
                // its texture / atlas is loaded. Sized as a sibling mesh
                // synced in world space every frame.
                hit_flash::attach_hit_flash_overlays,
                actors::animate_player,
                actors::animate_characters,
                // Content-owned overlays (the `ActorOverlaySet` seam) run here:
                // after `animate_characters`, before the hit-flash mirror.
                //
                // Mirror the source sprite's atlas + transform into the
                // hit-flash overlay and gate visibility on the current
                // hit_flash timer. Runs after the animator so the overlay
                // tracks the same frame the source draws this tick.
                hit_flash::sync_hit_flash_overlays,
                hit_flash::cleanup_hit_flash_overlays,
                actors::animate_props,
                actors::animate_feature_sprites,
                actors::animate_bosses.in_set(actors::BossAnimation),
                // HazardColumn vertical-column visual — yellow during
                // telegraph, red during strike. Runs after
                // `animate_bosses` so it can read the move-derived
                // `BossAttackState` read model upstream.
                actors::manage_gradient_lane_visual,
                // Provider-authored over-hand item sprites consume the generic
                // wielded-item read model and App-local visual catalog.
                wielded_item_visuals::sync_wielded_item_visuals,
                // The FLOOR, and it is LAST because that is the only position in
                // which its comment is true: a body the sim published a view for
                // that no family claimed gets a marked rectangle rather than
                // nothing at all.
                //
                // It sat second in this chain — before the worn-character
                // binder, the player fallback, the sprite upgrades and the boss
                // pass — while claiming to run "after every family", and it was
                // ALSO registered a second time outside the chain, ungated by
                // `session_presentation_is_ready` (GPT 5.6, 2026-07-27). Two
                // copies of a spawner is one copy too many, and the ungated one
                // could draw a stand-in before the intended family was even
                // allowed to run.
                features::draw_unclaimed_feature_views,
            )
                .chain()
                .in_set(
                    ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith::PresentationVisualSync,
                )
                .run_if(session_presentation_is_ready),
        );

        // Rebuild the active room's static visuals + parallax when the sim asks
        // for it (sandbox reset). The sim emits `RespawnRoomVisualsRequested`; we
        // own the actual spawn here so the sim never imports the render layer.
        app.add_systems(
            Update,
            world::respawn_room_visuals_on_request.run_if(session_presentation_is_ready),
        );
    }
}

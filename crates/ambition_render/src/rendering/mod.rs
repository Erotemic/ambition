//! Bevy presentation systems that project simulation/view state into visuals.
//!
//! This module owns sprite/world synchronization, per-view projections, camera
//! presentation, labels, parallax, and debug visualization. Simulation authority
//! remains outside the render crate.

/// All systems that can decide an actor sprite's handle, tint, or visibility.
///
/// Dev-tool sprite overrides run after this set. Any new sprite-authority pass
/// must join the set, including passes registered by another composing crate.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SpriteVisualSync;

/// Every writer of a body-owned drawable's geometry this frame: the clock bar
/// above a marked fighter, the hit-flash silhouette, the morph ball, the wire,
/// the tether, the bubble. When this set is done, each of those is final for
/// the frame.
///
/// `publish_portal_compositing_candidates` runs after this set. It reads each
/// drawable's pose and frame to decide what a pane may hide. One set edge,
/// not an edge per overlay, so a new overlay cannot be forgotten. The edge
/// also makes Bevy flush commands in between, so a drawable spawned inside the
/// set is a candidate on its first frame.
///
/// A new body-owned drawable writer must join this set. Outside it, the
/// writer is composited a frame late at best.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BodyOwnedDrawableSync;

pub mod actors;
pub mod body_clock;
pub mod body_cues;
pub mod bubble_shield;
mod camera;
pub mod debug_viz;
pub mod deferred_write_safety;
pub mod dizzy_stars;
#[cfg(feature = "portal_render")]
pub mod portal_compositing;
mod features;
pub mod flyline;
pub mod tether;
pub mod gate_portal_visuals;
pub mod gravity_visuals;
mod health;
/// Public so the shipped schedule can be asked, by type, whether its writer
/// sits in `BodyOwnedDrawableSync`.
pub mod hit_flash;
mod item_visuals;
pub mod label_layout;
pub mod knockout;
pub mod launch_trail;
pub mod mark_beacon;
pub mod morph_ball;
pub mod submerged;
pub mod moving_platforms;
mod nameplates;
mod parallax;
mod primitives;
pub mod projectile_visuals;
pub(crate) mod sheet_atlas;
pub mod shrine_visuals;
pub(crate) mod slash_visuals;
mod unauthored_volumes;
pub mod view_isolation;
mod wielded_item_visuals;
mod world;

pub use actors::{
    actor_sprite_path_owns, animate_bosses, animate_characters, animate_feature_sprites,
    animate_player, apply_hide_sprites_override, apply_placeholder_sprites_override,
    player_presentation_for_collision, refresh_player_sprites_for_resident_quality,
    refresh_prop_sprites_on_game_assets_change,
    sync_visuals, upgrade_actor_sprites, upgrade_boss_sprites, BossAnimation,
    PlayerSpriteCharacter,
};
// `BoundFeatureKind` lives with the foundation feature taxonomy;
// re-exported so render call sites resolve unchanged.
pub use ambition_platformer2d_shared_tangle::feature_kind::BoundFeatureKind;
// `manage_gradient_lane_visual` and `GradientLaneVisual` stay private; the
// schedule uses `actors::manage_gradient_lane_visual` directly.
pub use ambition_sim_view::camera_snapshot::{CameraSnapshot2d, SceneCaptureRequest};
#[cfg(feature = "portal_render")]
pub use camera::publish_portal_camera_clamp;
pub use camera::{camera_follow, CameraViewState};
/// The fallback stand-in's marker: a feature the sim published that no render
/// family has drawn for long enough to be a bug.
///
/// Diagnostic only. "Is this room presentable yet" is
/// [`UnclaimedFeatureViews`], which answers at once; this answers late.
// Test-only: lets a regression test run the pass that draws the stand-in.
// `#[cfg(test)]` keeps the shipped surface unchanged.
#[cfg(test)]
pub(crate) use features::draw_unclaimed_feature_views;
pub use features::UnclaimedBodyPlaceholder;
pub use features::UnclaimedFeatureViews;
pub use health::{sync_boss_health_bar_overlay, sync_health_overlays};
pub use label_layout::{
    layout_world_labels,
    mirror_static_world_labels_per_view,
    MirroredWorldLabel,
    // The marker is part of the seam: a game that spawns its own static
    // world text uses it to ask for one copy per view. Without it, a
    // second view would share the single entity.
    StaticWorldLabel,
    WorldLabel,
    WorldLabelFamily,
    WorldLabelLayoutPlugin,
    WorldLabelLayoutSet,
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
    // The loader's outcome, which presentation reads to tell "not yet" from
    // "never". See `ParallaxThemeAttempts`.
    ParallaxThemeAttempts,
    mirror_parallax_layers_per_view,
    refresh_parallax_layers_on_quality_change,
    spawn_parallax_layers,
    sync_parallax_layers,
    // The per-view copy's key back to the room's panel. Exported so a
    // consumer in a two-view session can tell a root from a copy.
    MirroredParallaxLayer,
    // The marker, so a consumer can ask whether a backdrop exists.
    // `fixtures/external_consumer` uses it.
    ParallaxLayerVisual,
};
pub use primitives::{
    BlockArt, BlockVisual, FeatureVisual, HudText, LoadingZoneVisual, PlayerSpriteBaseline,
    PlayerVisual, PropVisual, QuestPanelText, RoomScopedEntity, RoomVisual,
};
// Game-supplied art map for walk-into world items: the renderer owns the
// seam, and each game fills it with its own pickup images.
pub use item_visuals::WorldItemArt;
pub use wielded_item_visuals::{
    WieldedItemVisualAppExt, WieldedItemVisualCatalog, WieldedItemVisualSpec,
};
pub use world::{
    apply_block_art, flinch_struck_blocks, refresh_entity_sprite_handles_on_game_assets_change,
    spawn_room_visuals, spawn_surface_chain_visuals, sync_lock_wall_visuals,
    sync_removed_block_visuals,
};

/// The public seam for content-owned per-actor overlays: sibling meshes and
/// materials that decorate animated actor sprites (for example Ambition's
/// puppy-slug deep-dream pass). [`PresentationVisualAnimationPlugin`] places
/// this set after the character animators (so overlays mirror the new frame)
/// and before the hit-flash mirror, and gates it on session readiness. A game
/// adds its overlay systems `.in_set(ActorOverlaySet)`; the renderer names no
/// game's look.
#[derive(bevy::prelude::SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ActorOverlaySet;

/// Presentation systems below use session-created resources and entities.
/// During startup, loading, and the launcher there is no gameplay session, so
/// the per-frame presentation graph stays dormant.
///
/// Presentation must not demand more of a session than simulation does.
/// `simulation_authorized` asks for exactly one `SessionRoot` naming the
/// active scope. A stricter gate here would refuse to draw a session that the
/// engine simulates.
fn session_presentation_is_ready(
    gate: Option<
        bevy::prelude::Res<ambition_platformer2d_shared_tangle::lifecycle::SessionGatedSimulation>,
    >,
    active: Option<
        bevy::prelude::Res<ambition_platformer2d_shared_tangle::lifecycle::ActiveSessionScope>,
    >,
    roots: bevy::prelude::Query<&ambition_platformer2d_shared_tangle::lifecycle::SessionRoot>,
) -> bool {
    roots.single().is_ok_and(|root| {
        gate.is_none()
            || active.as_deref().and_then(
                ambition_platformer2d_shared_tangle::lifecycle::ActiveSessionScope::current,
            ) == Some(root.0)
    })
}

/// Schedules player-bound visual systems (morph-ball sprite, bubble-shield
/// sprite, and related). Each builds its texture once at startup, spawns
/// lazily once the player exists, and syncs visibility and tint every frame
/// after `sync_visuals` mirrors the player transform. The modules own the
/// systems; the ordering is a presentation concern, so it lives here.
pub struct PlayerVisualSchedulePlugin;

impl bevy::prelude::Plugin for PlayerVisualSchedulePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        use bevy::prelude::{IntoScheduleConfigs, Startup, Update};
        app.init_resource::<item_visuals::FailedItemArt>()
            .add_systems(Startup, morph_ball::build_morph_ball_sprite)
            .add_systems(Startup, submerged::build_trapdoor_sprite)
            .add_systems(Startup, flyline::build_flyline_sprite)
            .add_systems(
                Update,
                (
                    morph_ball::spawn_morph_ball_visual,
                    morph_ball::sync_morph_ball_visual.in_set(SpriteVisualSync),
                    // After the morph-ball sync (`chain()` below). Both restore a
                    // hidden body to `Inherited`; the last one wins, and only this
                    // one knows the body is under the stage.
                    submerged::sync_submerged_visibility.in_set(SpriteVisualSync),
                    // What the stage shows instead of the hidden body. After the
                    // hide, or a door and the body would both show.
                    submerged::sync_trapdoor_visuals.in_set(SpriteVisualSync),
                    // The wire. Not ordered against the door: a move uses one
                    // technique or the other, and a body on a rope is not under
                    // the stage. It is in this group only for the shared
                    // `after(sync_visuals)` and readiness gate.
                    flyline::sync_flyline_visuals.in_set(SpriteVisualSync),
                    // The tether line, beside the wire whose rope it borrows: same
                    // set, same lifecycle, same both-roads rule.
                    tether::sync_tether_visuals.in_set(SpriteVisualSync),
                )
                    .chain()
                    // All body-owned drawables: the portal publisher waits for this
                    // set. See `BodyOwnedDrawableSync`.
                    .in_set(BodyOwnedDrawableSync)
                    .after(actors::sync_visuals)
                    .run_if(session_presentation_is_ready),
            )
            // Bubble shield visual: toggle and tint every frame from
            // `BodyShieldState::active` and `BodyShieldState::parrying()`.
            .add_systems(Startup, bubble_shield::build_bubble_shield_sprite)
            .add_systems(
                Update,
                (
                    bubble_shield::spawn_bubble_shield_visual,
                    bubble_shield::sync_bubble_shield_visual.in_set(SpriteVisualSync),
                )
                    .chain()
                    .in_set(BodyOwnedDrawableSync)
                    .after(actors::sync_visuals)
                    .run_if(session_presentation_is_ready),
            )
            // Resolve each provider's held-item art (`HeldItemArtManifest`) into
            // loaded `HeldItemArt` handles.
            .add_systems(Startup, item_visuals::build_held_item_art)
            // Resolve each provider's walk-into pickup art
            // (`WorldItemArtManifest`) into loaded `WorldItemArt` handles.
            .add_systems(Startup, item_visuals::build_world_item_art)
            // Not session-gated: an art file that failed to load is a fact about
            // the build, and should be reported without waiting for a session.
            .add_systems(Update, item_visuals::report_unloadable_item_art)
            .add_systems(
                Update,
                (
                    item_visuals::sync_ground_item_visuals.after(actors::sync_visuals),
                    item_visuals::sync_world_item_visuals.after(actors::sync_visuals),
                    // Despawn any authored block the collision overlay removes this
                    // frame (a broken brick, a gate-dropped wall): the render half of
                    // `removed_block_names`.
                    sync_removed_block_visuals,
                    // A struck block flinches (presentation only; see `block_nudge`).
                    flinch_struck_blocks,
                    item_visuals::sync_held_item_visual.after(actors::sync_visuals),
                    shrine_visuals::sync_shrine_visual.after(actors::sync_visuals),
                    shrine_visuals::animate_shrine_visuals.after(actors::animate_props),
                    unauthored_volumes::draw_unauthored_attack_volumes,
                    slash_visuals::spawn_slash_effects,
                    // After the spawn, so a swing born this frame is on its body
                    // when drawn.
                    slash_visuals::follow_slash_owner.after(slash_visuals::spawn_slash_effects),
                    slash_visuals::animate_slash,
                    mark_beacon::sync_mark_beacon_visual.after(actors::sync_visuals),
                    // A clock above a body that carries one (the delayed mark's
                    // telegraph). After `sync_visuals`, so it uses this frame's pose.
                    body_clock::sync_body_clock_visuals
                        .in_set(BodyOwnedDrawableSync)
                        .after(actors::sync_visuals),
                    // Derived from `MovingPlatformSet`; never writes it. See
                    // `moving_platforms`.
                    moving_platforms::sync_moving_platform_visuals,
                )
                    .after(item_visuals::report_unloadable_item_art)
                    .run_if(session_presentation_is_ready),
            );

        // The sprite-effect capability, installed unconditionally and outside
        // the portal cfg: `SpriteEffect` is an engine concept any sprite may
        // carry, not a portal feature.
        app.add_plugins(ambition_sprite_fx::SpriteFxPlugin);

        // Portal-gun visuals (placed-portal quads, partial-transit pieces, the
        // disorientation and mode indicators) live in
        // `ambition_portal2d_presentation`. The sandbox adds its plugin, places
        // its set, and bridges host seams (see `ambition_portal2d::host_adapter`).
        // Gravity visuals and the F7 dev off-switch stay host-side.
        #[cfg(feature = "portal_render")]
        {
            use ambition_portal2d_presentation::{PortalPresentationPlugin, PortalPresentationSet};
            // Measurement only (D-HEADLESS-DESPAWN): a view-cone capture rig
            // renders to an `Image`. With `backends: None` there is no RenderApp,
            // so the image never fills, and despawning its `Camera` runs a hook
            // that reads a resource only `SyncWorldPlugin` creates. Same reasoning
            // as `tile_spine`.
            let has_render_app = app.get_sub_app(bevy::render::RenderApp).is_some();
            app.add_plugins(PortalPresentationPlugin {
                view_cones: has_render_app,
                ..PortalPresentationPlugin::default()
            });
            // Portal body-copy visuals run after the player animator, not only
            // `sync_visuals`: trimmed sprites change `Sprite::custom_size` and
            // `Anchor` during animation, and the exit copy clones the final basis.
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
                    gravity_visuals::sync_gravity_zone_visual.after(actors::sync_visuals),
                )
                    .run_if(session_presentation_is_ready),
            );
        }
    }
}

/// Schedules the per-frame visual animation chain into
/// [`ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith::PresentationVisualSync`].
///
/// Spawns dynamic feature visuals first (so `sync_visuals` finds them the same
/// frame), then mirrors transforms and atlas indices, upgrades enemy and boss
/// sprites, ticks the per-actor animators, and ends with provider-authored
/// wielded-item overlays.
///
/// Pinned `.after(map_menu::handle_map_menu_hotkeys)`: the map-menu input is
/// the last presentation-input system this set runs after.
pub struct PresentationVisualAnimationPlugin;

impl bevy::prelude::Plugin for PresentationVisualAnimationPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        use bevy::prelude::{IntoScheduleConfigs, Update};
        // Every visual below draws from frame-clock presented poses, so the
        // resample must run first. Both sides are in `Update` for all three sim
        // hosts.
        app.configure_sets(
            Update,
            ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith::PresentationVisualSync
                .after(ambition_sim_view::PresentedPoseSet),
        );
        app.init_resource::<wielded_item_visuals::WieldedItemVisualCatalog>();
        app.init_resource::<slash_visuals::SlashSources>();
        // The fallback census ("which published views is nothing drawing") is
        // what the room-transition cover waits on. The chain below publishes it.
        // The dormant system clears it, because a `Resource` is not cleaned up
        // with its session, and a stale non-zero census blacks out the screen
        // until the cover deadline.
        app.init_resource::<features::UnclaimedFeatureViews>();
        app.add_systems(
            Update,
            features::forget_unclaimed_feature_views_while_dormant
                .in_set(
                    ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith::PresentationVisualSync,
                )
                .run_if(bevy::ecs::schedule::common_conditions::not(
                    session_presentation_is_ready,
                )),
        );
        // Content-owned projectile art registry (empty until a game registers
        // looks). The renderer resolves each projectile's `ProjectileVisualId`
        // through it.
        app.init_resource::<ambition_projectiles::ProjectileVisualCatalog>();
        hit_flash::add_hit_flash_material_plugin(app);
        // Place the content-owned overlay seam: after the character animator
        // (overlays mirror its new frame), before the hit-flash mirror (the
        // silhouette reads sprite state that overlays may tint). The set carries
        // the session gate, like the chain below.
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
        // `ActorAnimIndex` is rebuilt sim-side (`FeatureViewSyncSchedulePlugin`,
        // in the FeatureViewSync tail this chain runs after). Presentation only
        // consumes it.
        app.add_systems(
            Update,
            (
                // Head of the chain: the room must be drawn before `sync_visuals`
                // reads its positions. The edge also gets an auto-inserted
                // `ApplyDeferred` (`Update` keeps default build settings), which makes
                // the spawns visible, not only earlier.
                world::respawn_room_visuals_on_request,
                // Spawn visuals for encounter-spawned enemies before `sync_visuals`
                // reads them, and remove ones whose sim feature is gone (an expired
                // drop), so a room does not collect invisible sprites.
                features::spawn_dynamic_feature_visuals,
                features::despawn_dead_dynamic_feature_visuals,
                // The selected-character binder: install or rebind the worn
                // character's sheet, animator, and anchor from `WornCharacter`.
                // Before the fallback, so a worn player never gets the neutral
                // rectangle. The app and every demo use this one path.
                actors::bind_worn_character_presentation,
                // Fallback for a bare `PlayerVisual` with no worn identity (a
                // minimal shell): give it a sprite before `sync_visuals` queries
                // `&mut Sprite`.
                actors::ensure_player_visual_sprite,
                actors::sync_visuals.in_set(SpriteVisualSync),
                actors::upgrade_actor_sprites,
                // Grouped: player and prop quality refreshes touch disjoint entities,
                // so they need no order. Nesting also keeps this tuple within Bevy's
                // 20-system limit.
                (
                    actors::refresh_player_sprites_for_resident_quality,
                    actors::refresh_prop_sprites_on_game_assets_change,
                ),
                actors::upgrade_boss_sprites,
                // Attach the hit-flash overlay to every character sprite once its
                // texture or atlas is loaded.
                // Guarded on all three assets it uses (`Assets<Mesh>`,
                // `Assets<HitFlashMaterial>`, `Assets<TextureAtlasLayout>`): Bevy
                // 0.19 panics when a parameter is missing, and a headless
                // composition has no render stack. See
                // `engine/headless-verification.md`.
                hit_flash::attach_hit_flash_overlays
                    .run_if(bevy::ecs::schedule::common_conditions::resource_exists::<
                        bevy::asset::Assets<bevy::mesh::Mesh>,
                    >)
                    .run_if(bevy::ecs::schedule::common_conditions::resource_exists::<
                        bevy::asset::Assets<hit_flash::HitFlashMaterial>,
                    >)
                    .run_if(bevy::ecs::schedule::common_conditions::resource_exists::<
                        bevy::asset::Assets<bevy::image::TextureAtlasLayout>,
                    >),
                actors::animate_player,
                actors::animate_characters,
                // Content-owned overlays (`ActorOverlaySet`) run here: after
                // `animate_characters`, before the hit-flash mirror.
                //
                // Mirror the source sprite's atlas and transform into the hit-flash
                // overlay, after the animator, so it tracks this tick's frame.
                hit_flash::sync_hit_flash_overlays.in_set(BodyOwnedDrawableSync),
                hit_flash::cleanup_hit_flash_overlays,
                actors::animate_props,
                actors::animate_feature_sprites,
                actors::animate_bosses.in_set(actors::BossAnimation),
                // HazardColumn column visual: yellow during telegraph, red during
                // strike. After `animate_bosses`, so it reads the `BossAttackState`
                // read model.
                actors::manage_gradient_lane_visual,
                // Provider-authored over-hand item sprites, from the wielded-item
                // read model and the app-local visual catalog.
                wielded_item_visuals::sync_wielded_item_visuals,
                // The fallback runs last, after every family: a body the sim
                // published that no family claimed gets a marked rectangle. It must
                // be registered only once, here, under the session gate.
                features::draw_unclaimed_feature_views,
            )
                .chain()
                .in_set(
                    ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith::PresentationVisualSync,
                )
                .run_if(session_presentation_is_ready),
        );

        // The hard-launch trail and smash-charge cues read only a read model
        // and write only messages, so they need no edge against the sprite
        // chain. They share this set for the session gate, which
        // `schedule_tests` checks.
        app.add_systems(
            Update,
            (
                launch_trail::emit_launch_trails,
                knockout::emit_knockout_beat,
                body_cues::emit_smash_charge_cues,
                body_cues::emit_parry_cues,
                dizzy_stars::emit_dizzy_stars,
            )
                .in_set(
                    ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith::PresentationVisualSync,
                )
                .run_if(session_presentation_is_ready),
        );

        // The room's static-visual respawn is the head of the chain above. The
        // sim emits `RespawnRoomVisualsRequested`; the spawn is here so the sim
        // never imports the render layer.
    }
}

#[cfg(test)]
mod schedule_tests {
    use super::*;

    /// The room's visuals must be spawned inside the ordered visual chain, not
    /// unordered in `Update`.
    ///
    /// `Platformer2dSimulationPhaseMonolith::PresentationVisualSync` sounds like a
    /// sim phase, but its members and the respawn are all registered in `Update`,
    /// so an ordinary edge works. Without it, a room transition leaves every
    /// authored feature with a stand-in.
    #[test]
    fn the_room_visual_respawn_is_inside_the_presentation_chain() {
        use bevy::ecs::schedule::{Schedules, SystemSet};
        use bevy::prelude::{App, Update};

        // The plugin installs a `Material2dPlugin`, so a bare `App` panics in
        // `bevy_asset`. Minimal plus asset plugins are enough to build the
        // schedule.
        let mut app = App::new();
        app.add_plugins((bevy::MinimalPlugins, bevy::asset::AssetPlugin::default()));
        app.add_plugins(PresentationVisualAnimationPlugin);

        // Systems cannot be matched by name: without Bevy's `debug` feature,
        // every name is a placeholder. So the assertion is a count: every
        // system this plugin puts in `Update` must be inside the ordered set.
        let total = {
            let schedules = app.world().resource::<Schedules>();
            schedules
                .get(Update)
                .expect("Update exists")
                .graph()
                .systems
                .len()
        };
        // `systems_in_set` works only on a built graph; an unbuilt one reports
        // `Uninitialized`, not an empty set. Build it without running anything.
        app.world_mut()
            .resource_scope(|world, mut schedules: bevy::prelude::Mut<Schedules>| {
                schedules
                    .get_mut(Update)
                    .expect("the plugin registers systems in Update")
                    .initialize(world)
                    .expect("the Update schedule builds");
            });
        let schedules = app.world().resource::<Schedules>();
        let ordered = schedules
            .get(Update)
            .expect("Update exists")
            .graph()
            .systems_in_set(
                ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith::PresentationVisualSync
                    .intern(),
            )
            .expect("the chain registers that set in Update")
            .len();
        assert!(total > 0, "the plugin registers systems in Update at all");
        assert_eq!(
            ordered, total,
            "{} of this plugin's {total} `Update` systems are inside \
             PresentationVisualSync. The stragglers have no ordering and no \
             flush point against `draw_unclaimed_feature_views`, so whatever \
             they spawn may not be visible when the floor asks what is undrawn \
             — which draws a stand-in for every authored feature and holds the \
             room-transition cover black for its full deadline. \
             `respawn_room_visuals_on_request` was the straggler.",
            ordered
        );
    }
}

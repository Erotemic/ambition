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

pub mod actors;
pub mod body_cues;
pub mod bubble_shield;
mod camera;
pub mod debug_viz;
pub mod deferred_write_safety;
pub mod dizzy_stars;
mod features;
pub mod flyline;
pub mod gate_portal_visuals;
pub mod gravity_visuals;
mod health;
mod hit_flash;
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
    refresh_player_sprites_for_resident_quality, refresh_prop_sprites_on_game_assets_change,
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
/// family has drawn, and that has stayed that way long enough to be a bug.
///
///  it is a DIAGNOSTIC only. "Is this room presentable yet" is
/// [`UnclaimedFeatureViews`], which answers immediately where this one answers
/// late — see that type for why one entity could not do both.
pub use features::UnclaimedBodyPlaceholder;
pub use features::UnclaimedFeatureViews;
pub use health::{sync_boss_health_bar_overlay, sync_health_overlays};
pub use label_layout::{
    layout_world_labels,
    mirror_static_world_labels_per_view,
    MirroredWorldLabel,
    //  the MARKER is part of the seam, not an internal detail: a game that
    // spawns its own static world text has to be able to say "one of these per
    // view, please" — without it the mirror leaves the label as a single shared
    // entity that a second view would fight over.
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
    // The loader's OUTCOME, which presentation reads to tell "not yet" from
    // "never". See `ParallaxThemeAttempts`.
    ParallaxThemeAttempts,
    mirror_parallax_layers_per_view,
    refresh_parallax_layers_on_quality_change,
    spawn_parallax_layers,
    sync_parallax_layers,
    // The per-view copy's key back to the panel the room spawned. Exported
    // beside the marker for the same reason: a consumer asking "is my sky
    // drawn" in a two-view session has to be able to tell a ROOT from a COPY.
    MirroredParallaxLayer,
    //  the MARKER, not just the systems. A consumer could install the whole
    // parallax family and had no way to ask whether a backdrop existed — the
    // component was behind a private module, so "is my sky drawn" was a question
    // only this crate could answer. `fixtures/external_consumer` asks it now,
    // which is the consumer that makes this worth exporting.
    ParallaxLayerVisual,
};
pub use primitives::{
    BlockArt, BlockVisual, FeatureVisual, HudText, LoadingZoneVisual, PlayerSpriteBaseline,
    PlayerVisual, PropVisual, QuestPanelText, RoomScopedEntity, RoomVisual,
};
// Game-supplied art map for walk-into world items; the reusable renderer owns the
// seam, each game fills it with its own pickups' images.
pub use item_visuals::WorldItemArt;
pub use wielded_item_visuals::{
    WieldedItemVisualAppExt, WieldedItemVisualCatalog, WieldedItemVisualSpec,
};
pub use world::{
    apply_block_art, flinch_struck_blocks, refresh_entity_sprite_handles_on_game_assets_change,
    spawn_room_visuals, spawn_surface_chain_visuals, sync_lock_wall_visuals,
    sync_removed_block_visuals,
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
///
/// Each of those finds its own subject and no-ops without it.
///
///  it was never a deliberate precondition. The session identity that translation was
/// protecting is entirely carried by the `SessionRoot` + `ActiveSessionScope` check below.
///
///  the invariant to hold this to: presentation must not demand more of a
/// session than SIMULATION does. `simulation_authorized` — the gate on the
/// gameplay sim itself — asks for exactly one `SessionRoot` naming the active
/// scope and nothing more. Anything stricter here means a session the engine
/// agreed to simulate is one it refuses to draw.
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

/// Module-local Bevy plugin: schedules player-bound visual systems
/// (morph-ball sprite + bubble-shield sprite). Each follows the same
/// pattern — build the texture once at startup, spawn lazily once the
/// player entity exists, sync visibility / tint every frame after
/// `sync_visuals` has mirrored the player transform.
///
/// Carved out of `app/plugins.rs::install_player_visual_systems` per
/// OVERNIGHT-TODO #6. Lives here in `ambition_render/src/rendering/` because
/// both subsystems chain `.after(sync_visuals)` and are presentation-
/// only — the body_mode + bubble_shield modules own the systems but
/// the schedule ordering is a presentation concern.
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
                    // ⛔ AFTER THE MORPH-BALL SYNC, and `chain()` below is what
                    // makes that true. Both restore a hidden body to
                    // `Inherited`; whichever runs last wins, and only one of
                    // them knows the body is under the stage.
                    submerged::sync_submerged_visibility.in_set(SpriteVisualSync),
                    // The other half of hiding her: what the stage shows
                    // INSTEAD. Behind the hide because a door drawn for a body
                    // still on screen would be two of her.
                    submerged::sync_trapdoor_visuals.in_set(SpriteVisualSync),
                    // THE WIRE. ⛔ NOT ordered against the door above: a move
                    // authors one technique or the other, and a body on a rope
                    // is not under the stage — a `chain()` here would state a
                    // relationship that does not exist. It is inside this group
                    // only because it shares the group's `after(sync_visuals)`
                    // and readiness gate.
                    flyline::sync_flyline_visuals.in_set(SpriteVisualSync),
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
                    // A struck block flinches — presentation only, see `block_nudge`.
                    flinch_struck_blocks,
                    item_visuals::sync_held_item_visual.after(actors::sync_visuals),
                    shrine_visuals::sync_shrine_visual.after(actors::sync_visuals),
                    shrine_visuals::animate_shrine_visuals.after(actors::animate_props),
                    unauthored_volumes::draw_unauthored_attack_volumes,
                    slash_visuals::spawn_slash_effects,
                    // After the spawn, so a swing born this frame is already on
                    // its body when the frame is drawn.
                    slash_visuals::follow_slash_owner.after(slash_visuals::spawn_slash_effects),
                    slash_visuals::animate_slash,
                    mark_beacon::sync_mark_beacon_visual.after(actors::sync_visuals),
                    // Reconciled from `MovingPlatformSet` here, it derives and never writes — see
                    // `moving_platforms`.
                    moving_platforms::sync_moving_platform_visuals,
                )
                    .after(item_visuals::report_unloadable_item_art)
                    .run_if(session_presentation_is_ready),
            );

        // ⭐ The sprite-effect capability, installed UNCONDITIONALLY and outside
        // the portal cfg below: `SpriteEffect` is an engine concept any sprite
        // may carry, and gating it on the portal mechanic would make a general
        // facility silently absent in a build that simply has no portals.
        // The plugin installs its own state and systems, so this is one line.
        app.add_plugins(ambition_sprite_fx::SpriteFxPlugin);

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
            // Portal body-copy visuals must run after the player animator, not only after
            // `sync_visuals`: trimmed sprites can update `Sprite::custom_size` and `Anchor` during
            // animation, and the portal exit copy must clone that final per-frame render basis.
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
        // The presentation floor's CENSUS — "which published views is nothing
        // drawing" — is what the room-transition cover waits on. Its publisher
        // is the tail of the chain below; its dormant answer is the system
        // beside it, because a `Resource` does not clean itself up when the
        // session that filled it goes away and a stale non-zero census is an
        // eight-second black screen.
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
                //  I claimed in `632ecf1b4` that an edge here would be a vacuous
                // cross-schedule `.after`.
                //
                // Head of the chain, beside the other spawner, because the room
                // has to be drawn before `sync_visuals` reads positions for it —
                // not merely before the floor. The dependency also earns an
                // auto-inserted `ApplyDeferred` (`Update` keeps Bevy's default
                // build settings), which is the part that makes the spawns
                // VISIBLE rather than merely earlier.
                world::respawn_room_visuals_on_request,
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
                    actors::refresh_player_sprites_for_resident_quality,
                    actors::refresh_prop_sprites_on_game_assets_change,
                ),
                actors::upgrade_boss_sprites,
                // Attach the hit-flash white-silhouette overlay to every
                // character sprite (player + enemies + NPCs + bosses) once
                // its texture / atlas is loaded. Sized as a sibling mesh
                // synced in world space every frame.
                // ⛔⛔ GUARDED: it takes `ResMut<Assets<Mesh>>`,
                // `ResMut<Assets<HitFlashMaterial>>` and
                // `Res<Assets<TextureAtlasLayout>>`, and Bevy 0.19 panics the
                // schedule when a parameter is absent where 0.18 skipped. In a
                // headless composition with no render stack that took the whole
                // app down — 26 of the feature union's failures.
                // ⭐ THE DOC PREDICTED THIS SYSTEM BY NAME.
                // `engine/headless-verification.md` records three of these
                // hiding in succession — "a missing `Assets<TextureAtlasLayout>`,
                // then `GizmoConfigStore`, then `Assets<Mesh>`" — and this one
                // takes all three. Guarding on all three rather than on the one
                // that happened to fail first is the whole point of that
                // sentence.
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
                // `session_presentation_is_ready`. Two
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

        // The hard-launch smoke trail and the smash-charge cues. Both read only
        // a read-model and write only messages, so neither needs an edge
        // against the sprite chain — but both belong in the same set, which is
        // what carries the session gate and what `schedule_tests` pins.
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

        // (The room's static-visual respawn is the head of the chain above. The
        // sim emits `RespawnRoomVisualsRequested`; we own the actual spawn here
        // so the sim never imports the render layer.)
    }
}

#[cfg(test)]
mod schedule_tests {
    use super::*;

    /// The room's visuals must be SPAWNED inside the ordered visual chain,
    /// not floating unordered in `Update`.
    ///
    /// `632ecf1b4` recorded that an ordering edge here would be a vacuous cross-schedule
    /// `.after` — reasoning from the SET'S NAME
    /// (`Platformer2dSimulationPhaseMonolith::PresentationVisualSync` reads like a simulation
    /// phase) instead of from where its members are registered. They are registered in
    /// `Update`, the set is configured in `Update`, and the respawn was in `Update`: one
    /// schedule, ordinary edge.
    ///
    /// That is how a room transition left every authored feature wearing a stand-in.
    #[test]
    fn the_room_visual_respawn_is_inside_the_presentation_chain() {
        use bevy::ecs::schedule::{Schedules, SystemSet};
        use bevy::prelude::{App, Update};

        //  the plugin installs a `Material2dPlugin`, so a bare `App` panics
        // inside `bevy_asset`. Minimal + asset infrastructure is enough to build
        // the schedule, which is all this test reads.
        let mut app = App::new();
        app.add_plugins((bevy::MinimalPlugins, bevy::asset::AssetPlugin::default()));
        app.add_plugins(PresentationVisualAnimationPlugin);

        // systems cannot be identified by NAME here. Bevy compiles system names out unless its
        // `debug` feature is on, so every one of them reports `<Enable the debug feature to see the
        // name>`; a name-matching pin silently matches nothing. The mirror image of a check that
        // cannot fail, and just as worthless.)
        //
        // So the assertion is a COUNT, and it is the better one anyway: every system this plugin
        // puts in `Update` must be inside the ordered set.
        let total = {
            let schedules = app.world().resource::<Schedules>();
            schedules
                .get(Update)
                .expect("Update exists")
                .graph()
                .systems
                .len()
        };
        // The graph answers `systems_in_set` only once it has been BUILT — an
        // unbuilt one reports `Uninitialized` rather than an empty set, which is
        // the good failure direction. Build it without running anything.
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

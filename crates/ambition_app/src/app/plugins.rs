use bevy::prelude::*;
use bevy_asset_loader::asset_collection::AssetCollectionApp;
use bevy_ecs_ldtk::prelude::LdtkPlugin;
#[cfg(feature = "ui")]
use bevy_material_ui::MaterialUiPlugin;
#[cfg(feature = "input")]
use leafwing_input_manager::prelude::InputManagerPlugin;

use ambition_gameplay_core::assets::loading;
use ambition_gameplay_core::dev::dev_tools::{
    self, DeveloperTools, EditableAbilitySet, EditableMovementTuning, EditablePlayerStats,
    MovementProfile, PlayerBodyProfile,
};
use ambition_gameplay_core::dialog;
use ambition_gameplay_core::game_mode::{gameplay_allowed, gameplay_suspended};
use ambition_gameplay_core::inventory_ui;
use ambition_gameplay_core::ldtk_world;
use ambition_gameplay_core::rooms;
#[cfg(feature = "input")]
use ambition_gameplay_core::schedule::{
    apply_menu_frame_to_cutscene_request, populate_control_frame_from_actions,
    populate_menu_control_frame_from_actions,
};
use ambition_gameplay_core::schedule::{
    attach_player_input_components, configure_sandbox_sets,
    toggle_player_trail_emission_from_actions, SandboxSet,
};
use ambition_gameplay_core::time::feel::SandboxFeelTuning;
use ambition_gameplay_core::world::physics;
#[cfg(feature = "physics_debris")]
use ambition_gameplay_core::world::physics::physics_spawn_debris_messages;
use ambition_input::MenuControlFrame;
#[cfg(feature = "input")]
use ambition_input::{MenuInputState, PlayerDashTriggerState, SandboxAction};
use ambition_render::fx::{self, vfx_spawn_messages};
use ambition_render::rendering::{animate_bosses, camera_follow, sync_visuals};
use ambition_render::ui_fonts;

use crate::dev::debug_overlay;
use crate::host::windowing;

use super::dev_runtime::{handle_debug_hotkeys, handle_ldtk_hot_reload, sync_preset_input_map};
use super::hud::{update_hud, update_quest_panel};
use super::player_tick::{apply_home_reset_policy, sync_player_presentation};
use super::resources::init_sandbox_resources;
use super::setup_systems::{
    reload_visual_quality_assets_on_scale_change, setup_presentation_system,
    setup_simulation_system,
};
use super::sim_systems::{
    apply_cut_rope_room_replay_request_system, apply_player_reset_input_system,
    apply_suspended_time_scale_system, cleanup_timers_system, input_timer_system,
    interaction_input_system, sync_live_player_dev_edits_system,
};
use super::world_flow::{apply_room_transition_system, ensure_requested_room_parallax_system};
use ambition_gameplay_core::player::PlayerBodyFrameOutput;

/// Register core simulation plugins, message types, and the gameplay
/// schedule. Headless and visible both call this.
///
/// The body is split into per-set helpers below so each section is short
/// enough to read in one screen and stays under Bevy's 20-system tuple
/// arity limit. New simulation systems should go into the matching
/// `register_*_systems` helper rather than back into this orchestrator.
pub fn add_simulation_plugins(app: &mut App) {
    // AmbitionPhysicsPlugin (Avian2D) is intentionally NOT here. Per
    // ADR 0007 Avian is secondary physics for debris/ragdoll visuals;
    // the player controller is custom via parry2d in ambition_engine_core.
    // Avian's collider backend needs `SceneSpawner` (from ScenePlugin in
    // DefaultPlugins), which headless doesn't have. Until Avian's debris
    // role is migrated to presentation events end-to-end (or Avian gains
    // a headless-friendly init path), it lives in
    // `add_presentation_plugins`.

    // Declare the canonical simulation-phase ordering. Individual system
    // registrations below only need `.in_set(SandboxSet::X)`; they no longer
    // need to pin a cross-set system via `.after(other_system)`. Intra-set
    // `.chain()` ordering is still expressed per-system.
    configure_sandbox_sets(app);
    app.init_resource::<ambition_gameplay_core::shrine::ShrineActivationPulse>();
    // Slot-keyed gesture/buffer authority (double-tap, interact buffer). Local
    // input publishes it; body mode / interaction / transitions consume it for the
    // controlled body's slot — no privileged per-body interaction component.
    app.init_resource::<ambition_gameplay_core::player::SlotInteractionState>();
    // Which character the local player spawns as. Default = the `player`
    // protagonist (no change); the character-select surface rewrites this, and
    // both startup halves (sim moveset/name + presentation sprite) read it.
    app.init_resource::<ambition_gameplay_core::player::StartingCharacter>();

    app.add_plugins(super::sim_resources::SandboxSimulationResourcesPlugin);

    // Named Ambition game content: quests, bosses, dialogue/cutscenes, intro
    // hooks, and portal adapters. Installed after simulation resources so content
    // registries land at the expected assembly point.
    app.add_plugins(ambition_content::AmbitionContentPlugin);

    // Yarn dialogue stack: compile `.yarn`, bridge runner events into sandbox
    // state, and register the commands / functions / markup used by content.
    #[cfg(feature = "ui")]
    {
        app.add_plugins(ambition_gameplay_core::dialog::yarn_spinner_plugin());
        app.add_plugins(ambition_gameplay_core::dialog::YarnBridgePlugin);
        app.add_plugins(ambition_gameplay_core::dialog::YarnBindingsPlugin);
    }

    app.add_plugins(ambition_gameplay_core::features::WorldPrepSchedulePlugin);
    // Universal-brain messages/resources; per-tick systems are registered below.
    app.add_plugins(ambition_characters::brain::BrainPlugin);
    register_player_input_systems(app);
    register_player_simulation_systems(app);
    // Ambition's player ability/weapon kit plus its small shared app state.
    app.add_plugins(ambition_gameplay_core::abilities::AmbitionAbilitiesPlugin);
    // "Tie a knot": the emitted player trail used as the substrate for future
    // cycle/spell mechanics. Emission starts disabled until input toggles it.
    app.add_plugins(ambition_gameplay_core::player::trail::PlayerTrailPlugin);
    // Gravity zones / switches and their per-frame ambient-gravity snapshot.
    app.add_plugins(ambition_gameplay_core::gravity::GravityPlugin);
    #[cfg(feature = "portal")]
    {
        app.add_plugins(ambition_gameplay_core::portal::PortalPlugin);
        // Host-side placement for portal's internal sets.
        wire_portal_schedule(app);
    }
    app.add_plugins(ambition_gameplay_core::items::pickup::ItemPickupSimulationPlugin);
    register_room_transition_systems(app);
    app.add_plugins(super::combat_schedule::CombatSchedulePlugin);
    register_presentation_sync_systems(app);
    app.add_plugins(ambition_gameplay_core::features::FeatureCollectionSchedulePlugin);
    app.add_plugins(ambition_gameplay_core::features::FeatureInteractionSchedulePlugin);
    app.add_plugins(ldtk_world::LdtkRuntimeSpinePlugin);
    app.add_plugins(ambition_gameplay_core::encounter::EncounterSimulationSchedulePlugin);
    app.add_plugins(ambition_gameplay_core::cutscene::CutsceneSchedulePlugin);
    app.add_plugins(ambition_gameplay_core::features::GameplayEffectsSchedulePlugin);
    app.add_plugins(super::progression_schedule::ProgressionSchedulePlugin);
    app.add_plugins(ambition_gameplay_core::features::FeatureViewSyncSchedulePlugin);
    app.add_plugins(ambition_gameplay_core::session::reset::SandboxResetSchedulePlugin);
    app.add_plugins(ambition_gameplay_core::trace::TraceSchedulePlugin);
    // Per-frame "what would each verb do right now?" table consumed
    // by the touch / control-prompt HUD and (future) gameplay code.
    // Registered alongside simulation so headless / RL builds can
    // also inspect affordances; the resources are cheap and the
    // compute systems no-op when no primary player exists.
    app.add_plugins(ambition_gameplay_core::player::affordances::AffordancesPlugin);
}

/// Host-side placement for portal systems: map each portal-internal set to
/// the sandbox phase, cross-set ordering edge, and gameplay run condition.
#[cfg(feature = "portal")]
fn wire_portal_schedule(app: &mut App) {
    use ambition_gameplay_core::portal::PortalSet;

    // Carves publish after gravity-zone collection and before core simulation.
    app.configure_sets(
        Update,
        PortalSet::Carves
            .after(ambition_gameplay_core::physics::collect_gravity_zones)
            .before(SandboxSet::CoreSimulation),
    );

    // InputWarp: input rewrite in the player-input phase, after interaction
    // input and before the player input frame is synced (the Move-axis-fix
    // window), gated to gameplay.
    app.configure_sets(
        Update,
        PortalSet::InputWarp
            .in_set(SandboxSet::PlayerInput)
            .after(crate::app::interaction_input_system)
            .before(ambition_gameplay_core::player::sync_local_player_input_frame)
            .run_if(ambition_gameplay_core::gameplay_allowed),
    );

    // Weapon maintenance stays ungated for orphan cleanup / roll readiness.
    app.configure_sets(
        Update,
        PortalSet::WeaponAndProjectiles
            .in_set(SandboxSet::PlayerSimulation)
            .run_if(ambition_gameplay_core::gameplay_allowed),
    );
    app.configure_sets(
        Update,
        PortalSet::WeaponMaintenance.in_set(SandboxSet::PlayerSimulation),
    );

    // RoomReset: reset-time portal cleanup in the room-transition phase, after
    // the content layer's room-reset work (the cut-rope boss arena reset).
    app.configure_sets(
        Update,
        PortalSet::RoomReset
            .in_set(SandboxSet::RoomTransition)
            .after(ambition_gameplay_core::session::reset::ContentRoomResetSet),
    );

    // TransitGuards: suppress ledge-grab while transiting, BEFORE the unified body
    // integration reads it. Movement moved into `WorldPrep` (`integrate_sim_bodies`),
    // so the guard runs there too, ahead of it. Gated to gameplay.
    app.configure_sets(
        Update,
        PortalSet::TransitGuards
            .in_set(SandboxSet::WorldPrep)
            .before(ambition_gameplay_core::features::integrate_sim_bodies)
            .run_if(ambition_gameplay_core::gameplay_allowed),
    );

    // Transit: teleports run after body + ground-item integration so this frame's
    // integrated body positions are what cross the portal. Body integration now
    // completes in `WorldPrep`; `PlayerSimulation` runs after it, so membership +
    // the CoreHeldItems edge are enough. Gated to gameplay.
    app.configure_sets(
        Update,
        PortalSet::Transit
            .in_set(SandboxSet::PlayerSimulation)
            .after(ambition_gameplay_core::items::pickup::ItemPickupSet::CoreHeldItems)
            .run_if(ambition_gameplay_core::gameplay_allowed),
    );
}

// Core simulation, split into 6 finer-grained sub-sets that are
// chained inside `SandboxSet::CoreSimulation`. See
// `schedule.rs::configure_sandbox_sets` for the sub-set ordering.
// External presentation/audio/HUD systems still pin against
// `SandboxSet::CoreSimulation`; that constraint covers all six
// sub-sets transitively.

/// Dev-edit sync + input-driven reset + gameplay timer decay + interact
/// buffer + suspended-time fallback. Each subsequent system depends on
/// the previous one's ControlFrame / component mutation, so they stay
/// chained.
///
/// Ordering subtleties (ADR 0010 §"Suspended time"):
/// * `apply_suspended_time_scale_system` runs FIRST so when gameplay
///   is suspended (pause / dialogue / cutscene / room transition) the
///   sim_clock target and `SandboxSimState::time_scale` are zeroed
///   BEFORE `refresh_world_time` snapshots them. Previously this
///   system ran last in the chain, so `WorldTime::scaled_dt` could
///   be non-zero on the very first suspended frame and presentation
///   systems scaling animations by `time_scale * dt` would tick once
///   after pause landed.
/// * The emit → apply → smooth trio is gated to `gameplay_allowed`
///   so it doesn't immediately re-populate `RequestedClockScale` /
///   `time_scale` back from the zero the suspended fallback just
///   wrote. On the first re-resumed frame they run again and the
///   smoother ramps back up from 0 to 1.0 at the authored rate.
/// * `refresh_world_time` then snapshots whichever path won this
///   frame, so downstream systems always see a coherent `scaled_dt`.
fn register_player_input_systems(app: &mut App) {
    app.add_systems(
        Update,
        (
            apply_suspended_time_scale_system.run_if(gameplay_suspended),
            // ADR 0010 — time-control pipeline. Gated to
            // `gameplay_allowed` so suspended frames don't re-emit a
            // default 1.0 request that would compete with the
            // suspended fallback above.
            ambition_gameplay_core::time::time_control::emit_player_time_intent_system
                .run_if(gameplay_allowed),
            ambition_gameplay_core::time::time_control::apply_clock_scale_requests
                .run_if(gameplay_allowed),
            ambition_gameplay_core::time::time_control::smooth_sim_clock_toward_target_system
                .run_if(gameplay_allowed),
            // Unconditional: snapshot whichever path (suspended-zero
            // or gameplay-smoothed) wrote `SandboxSimState::time_scale`
            // this frame into `WorldTime` for downstream readers.
            ambition_time::refresh_world_time,
            // Mirror the freshly-snapshotted `WorldTime::sim_dt()` into the
            // runtime crate's neutral `SimDt` so every downstream runtime
            // system (gravity / zones / orient-roll) reads scaled dt without a
            // sandbox dependency. Runs immediately after `refresh_world_time`.
            ambition_gameplay_core::mirror_sim_dt_into_runtime,
            sync_live_player_dev_edits_system,
            apply_player_reset_input_system.run_if(gameplay_allowed),
            ambition_content::bosses::emit_cut_rope_room_replay_after_dialogue_closes,
            apply_cut_rope_room_replay_request_system,
            input_timer_system
                .run_if(gameplay_allowed)
                .in_set(ambition_input::InputSet::Populate),
            interaction_input_system.run_if(gameplay_allowed),
            // Portal-warped held movement input is registered by
            // `ambition_gameplay_core::portal::PortalPlugin` so the portal subsystem owns
            // its input seam.
            // Controller-input setup, nested into one chained group (keeps the
            // outer tuple within Bevy's 20-system limit):
            // 1. Resolve the CONTROLLED SUBJECT — the body carrying
            //    `Brain::Player(PRIMARY)` this frame (home avatar, or a possessed
            //    actor). Camera / portal viewer / nameplates / the player melee
            //    lifecycle read it; it replaces the old
            //    `PrimaryPlayer + PossessionState` overrides.
            // 2. Publish the local device frame into the slot-based controller
            //    model (`SlotControls[PRIMARY]`) — the canonical source every
            //    controlled body reads by its brain's slot.
            // 3. Mirror each controlled body's slot frame onto its
            //    PlayerInputFrame (gated on brain ownership: a vacated avatar
            //    sees neutral input, so it has no local attack authority).
            (
                ambition_gameplay_core::abilities::traversal::possession::resolve_controlled_subject,
                ambition_gameplay_core::player::populate_slot_controls,
                ambition_gameplay_core::player::sync_local_player_input_frame,
            )
                .chain(),
            // Universal-brain seam: translate this frame's slot input into each
            // controlled body's ActorControl frame. Runs after the input sync so the
            // brain sees this frame's inputs. The ActorControl output is the
            // polarity-flip authority for `player_control_system` /
            // `player_simulation_system` (see `engine_input_from_actor_control`).
            ambition_gameplay_core::player::tick_player_brains,
            // Body-mode policy (crouch / morph / climb) consumes the CONTROLLED
            // body's freshly-produced ActorControl + its slot gestures, so it must run
            // AFTER `tick_player_brains` and still before WorldPrep movement so the
            // resize/mode change lands on the same frame as the edge.
            ambition_gameplay_core::body_mode::update_body_mode,
            ambition_gameplay_core::player::sync_player_actor_poses,
        )
            .chain()
            .in_set(SandboxSet::PlayerInput),
    );
    // Universal-brain effects resolver — moved OUT of `PlayerInput` to run AFTER
    // `WorldPrep`. `PlayerInput` now precedes `WorldPrep`, so this must run after
    // the actor/boss brain ticks (`update_ecs_actors` / `tick_boss_brains_system`)
    // to observe THIS frame's actor `ActorControl` (not last frame's) — otherwise
    // every enemy's melee/ranged would lag a frame. It still runs before `Combat`
    // (where the consumers spawn hitboxes/projectiles), same frame.
    //   - `emit_brain_action_messages`: ActionSet × ActorControl → per-request
    //     `ActorActionMessage` (enemy ranged, enemy melee start, player melee/pogo
    //     gating, boss specials).
    //   - `emit_player_projectile_tick_messages`: per-tick charge axis/edges for
    //     charge-capable bodies → `PlayerProjectileTick`.
    //   - `observe_brain_action_counter`: HUD/debug "any brain wants something".
    app.add_systems(
        Update,
        (
            ambition_characters::brain::emit_brain_action_messages,
            ambition_characters::brain::emit_player_projectile_tick_messages,
            ambition_characters::brain::observe_brain_action_counter,
        )
            .chain()
            .after(SandboxSet::WorldPrep)
            .before(SandboxSet::PlayerSimulation),
    );
}

/// Main player tick: clear the reset flag, run control, run simulation,
/// then drain damage. Simulation short-circuits when control already reset
/// the player so same-frame respawns are not clobbered.
fn register_player_simulation_systems(app: &mut App) {
    // Every player body carries the movement→presentation hand-off the movement
    // phase writes and the presentation phase reads (required so both phase queries
    // always match the player + any clone).
    app.register_required_components::<ambition_gameplay_core::actor::PlayerEntity, PlayerBodyFrameOutput>();
    // Every player body publishes the same gravity-oriented combat footprint an
    // actor does (fable review 2026-07-02 §A6); integrate_home_body writes it.
    app.register_required_components_with::<ambition_gameplay_core::actor::PlayerEntity, ambition_engine_core::CenteredAabb>(
        || ambition_engine_core::CenteredAabb::new(ambition_engine_core::Vec2::ZERO, ambition_engine_core::Vec2::ZERO),
    );
    // Brain-driven player clone (press K): a `PlayerEntity` body driven by a
    // PlayerDemo brain through the SAME shared player systems as the human player.
    // Spawn lands in `WorldPrep` (the earliest set) so the new body exists before
    // the PlayerInput/PlayerSimulation phases pick it up the same frame; the brain
    // tick produces its `ActorControl` in `PlayerInput` (before the control phase
    // consumes it); the transform sync runs in `PresentationSync` after the shared
    // simulation has moved it. Movement itself is no longer here — it flows through
    // `player_control_system` / `player_simulation_system` like every player body.
    app.init_resource::<crate::app::player_clone::PlayerCloneClock>()
        .init_resource::<crate::app::player_clone::SpawnPlayerCloneRequest>()
        .add_systems(
            Update,
            (
                crate::app::player_clone::request_player_clone_on_key,
                crate::app::player_clone::spawn_requested_player_clone,
            )
                .chain()
                .in_set(SandboxSet::WorldPrep),
        )
        .add_systems(
            Update,
            crate::app::player_clone::tick_player_clone_brains
                .run_if(gameplay_allowed)
                .in_set(SandboxSet::PlayerInput),
        )
        .add_systems(
            Update,
            crate::app::player_clone::sync_player_clone_transform
                .in_set(SandboxSet::PresentationSync),
        )
        .add_systems(
            Update,
            crate::app::player_clone::despawn_player_clones_on_reset
                .in_set(SandboxSet::ResetProcessing)
                .before(ambition_gameplay_core::session::reset::process_sandbox_reset_request),
        );
    // Possession systems stay chained with the player tick. Possession is now
    // pure BRAIN TRANSFER, so there is no `not_possessing` control gate: the
    // vacated home avatar is inert because it no longer carries a player brain
    // (its `ActorControl` is neutral), and the possessed actor is driven through
    // the actor tick by the transferred `Brain::Player`.
    app.add_systems(
        Update,
        (
            // Possession: Down+Interact hold transfers the player brain onto the
            // nearest non-boss actor; a press releases. Brain transfer + the
            // target-lost safety run here; the actual body movement already happened
            // in `WorldPrep` (`integrate_sim_bodies`), one frame's-worth of latency
            // across the handover, exactly as before.
            ambition_gameplay_core::abilities::traversal::possession::possession_trigger_system
                .run_if(gameplay_allowed),
            ambition_gameplay_core::abilities::traversal::possession::release_possession_if_target_lost,
            // HOME RESET POLICY. Movement already integrated the home body in
            // `WorldPrep` and flagged any reset in `PlayerBodyFrameOutput`; this owns
            // the home-only sandbox + room reset on that flag (an actor never
            // teleports to the player spawn). Moves no body.
            apply_home_reset_policy.run_if(gameplay_allowed),
            // HOME PRESENTATION — screen shake + landing SFX + the per-op
            // anim/SFX/VFX — reads the same hand-off. Moves no body.
            sync_player_presentation.run_if(gameplay_allowed),
            ambition_gameplay_core::combat::damage::apply_player_hit_events
                .run_if(gameplay_allowed),
        )
            .chain()
            .in_set(SandboxSet::PlayerSimulation),
    );
}

/// Detection emits `RoomTransitionRequested`; apply consumes it and runs
/// `load_room`; the feature-side `reset_ecs_room_features` system tears
/// down per-room ECS state.
fn register_room_transition_systems(app: &mut App) {
    app.add_systems(
        Update,
        (
            ambition_gameplay_core::rooms::detect_room_transition_system.run_if(gameplay_allowed),
            ensure_requested_room_parallax_system,
            apply_room_transition_system,
            // One reset over the unified actor cluster (NPCs + enemies).
            ambition_gameplay_core::features::reset_ecs_room_features,
            // Content-side reset work carries the ContentRoomResetSet
            // label so generic plugins (gravity, portal) can order
            // after it without naming content systems.
            ambition_content::bosses::reset_cut_rope_boss_arena_on_room_reset
                .in_set(ambition_gameplay_core::session::reset::ContentRoomResetSet),
            // Portal room-reset cleanup is registered by
            // `ambition_gameplay_core::portal::PortalPlugin`.
        )
            .chain()
            .in_set(SandboxSet::RoomTransition),
    );
}

/// Slash/pogo attack lifecycle, projectile tick, and the feature-side
/// damage event apply.
/// Player ECS body write-back + presentation timer decays. Runs
/// unconditionally so paused / dialogue modes still wind down flash and
/// landing-pose timers.
fn register_presentation_sync_systems(app: &mut App) {
    app.add_systems(
        Update,
        (
            ambition_gameplay_core::player::write_player_ecs_components,
            cleanup_timers_system,
        )
            .chain()
            .in_set(SandboxSet::PresentationSync),
    );
}

/// Register Bevy's `LdtkPlugin` plus the supporting Ambition glue
/// (entity registrations, asset collection, LdtkWorldBundle spawn,
/// level-set sync, asset handle preload). Visible binary only —
/// `LdtkPlugin` panics in headless because its tile pipeline expects a
/// `RenderApp` sub-app, and `asset_server.load::<LdtkProject>` requires
/// the LDtk asset type to be registered.
///
/// Once the LDtk runtime-spine roadmap finishes promoting LDtk entity
/// categories to direct Ambition components (see
/// `project_ldtk_roadmap` memory), this dependency goes away and
/// headless can spawn the same entity set without bevy_ecs_ldtk's
/// rendering machinery.
pub fn add_ldtk_runtime_plugin(app: &mut App) {
    // `SandboxAssetCollection` includes a typed LDtk handle, so the LDtk
    // asset type and loader must be initialized before bevy_asset_loader
    // allocates collection handles. Keep this before `init_collection`.
    app.add_plugins(LdtkPlugin)
        .init_collection::<loading::SandboxAssetCollection>()
        .add_plugins(ldtk_world::AmbitionLdtkRegistrationPlugin)
        .add_systems(
            Startup,
            (
                ldtk_world::load_ldtk_asset_handle,
                spawn_ldtk_world_root.after(setup_simulation_system),
            ),
        )
        .add_systems(
            Update,
            (
                ldtk_world::sync_ldtk_level_set,
                // ADR 0015 §Coordinate-frame reconciliation — keep the
                // LdtkWorldBundle's root transform aligned with the
                // current active area's centered frame. Runs every
                // frame; cheap and idempotent.
                ldtk_world::sync_ldtk_world_transform,
            ),
        );
}

/// Spawn the `LdtkWorldBundle` entity. Runs in `add_ldtk_runtime_plugin`
/// (visible binary only) after `setup_simulation_system` so the
/// `LdtkRuntimeIndex` resource is available.
pub(super) fn spawn_ldtk_world_root(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    ldtk_index: Res<ldtk_world::LdtkRuntimeIndex>,
    room_set: Res<rooms::RoomSet>,
    ldtk_asset: Option<Res<ldtk_world::SandboxLdtkAsset>>,
    intro_asset: Option<Res<ldtk_world::IntroLdtkAsset>>,
    cut_rope_asset: Option<Res<ldtk_world::CutRopeLdtkAsset>>,
    sandbox_asset_collection: Option<Res<loading::SandboxAssetCollection>>,
) {
    let ldtk_handle = ldtk_asset
        .as_ref()
        .map(|asset| asset.0.clone())
        .or_else(|| {
            sandbox_asset_collection
                .as_ref()
                .map(|collection| collection.ldtk_project.clone())
        })
        .unwrap_or_else(|| asset_server.load(ldtk_world::sandbox_ldtk_asset_path()));
    let initial_level_set = ldtk_index.level_set_for(&room_set.active_spec().id);
    commands.spawn((
        bevy_ecs_ldtk::prelude::LdtkWorldBundle {
            ldtk_handle: ldtk_handle.into(),
            level_set: initial_level_set.clone(),
            // AMBITION_REVIEW(spatial): migrate each registered marker from
            // adapter-driven semantics to direct Ambition components.
            ..default()
        },
        ldtk_world::SandboxLdtkWorldRoot,
        Name::new("LDtk Runtime Spine Root (sandbox.ldtk)"),
    ));
    // Secondary intro LDtk bundle. bevy_ecs_ldtk's asset loader is
    // per-file; Ambition's merged JSON loader doesn't propagate into
    // the Bevy asset system. Each .ldtk file therefore needs its own
    // bundle to get its painted tile layers rendered. The shared
    // sync system writes the same LevelSet to both bundles; only the
    // bundle whose loaded asset contains the active level iids spawns
    // any levels.
    let intro_handle = intro_asset
        .as_ref()
        .map(|asset| asset.0.clone())
        .unwrap_or_else(|| asset_server.load("ambition/worlds/intro.ldtk"));
    commands.spawn((
        bevy_ecs_ldtk::prelude::LdtkWorldBundle {
            ldtk_handle: intro_handle.into(),
            level_set: initial_level_set.clone(),
            ..default()
        },
        ldtk_world::IntroLdtkWorldRoot,
        Name::new("LDtk Runtime Spine Root (intro.ldtk)"),
    ));

    let cut_rope_handle = cut_rope_asset
        .as_ref()
        .map(|asset| asset.0.clone())
        .unwrap_or_else(|| asset_server.load("ambition/worlds/you_have_to_cut_the_rope.ldtk"));
    commands.spawn((
        bevy_ecs_ldtk::prelude::LdtkWorldBundle {
            ldtk_handle: cut_rope_handle.into(),
            level_set: initial_level_set,
            ..default()
        },
        ldtk_world::CutRopeLdtkWorldRoot,
        Name::new("LDtk Runtime Spine Root (you_have_to_cut_the_rope.ldtk)"),
    ));
}

/// Register presentation-side plugins (input, dialogue, inspector, audio
/// and VFX subscribers, HUD, debug overlays). Visible binary only.
pub fn add_presentation_plugins(app: &mut App) {
    install_presentation_resources_and_subplugins(app);
    app.add_plugins(ambition_gameplay_core::persistence::PersistenceSchedulePlugin);
    install_menu_setup_and_hotkeys(app);
    app.add_plugins(ambition_render::rendering::PresentationVisualAnimationPlugin);
    install_camera_and_debug_overlay_systems(app);
    app.add_plugins(ambition_render::rendering::ActorNameplatePresentationPlugin);
    install_fx_and_hud_systems(app);
    install_misc_visual_sync_systems(app);
    app.add_plugins(ambition_render::rendering::PlayerVisualSchedulePlugin);
    install_projectile_and_vfx_systems(app);
}

/// Visible-side resources, registered types, and presentation child
/// plugins (input, audio, dev_tools, physics_debris, ui, mobile touch,
/// FPS overlay, font loader).
fn install_presentation_resources_and_subplugins(app: &mut App) {
    app.insert_resource(ClearColor(Color::srgb(0.020, 0.024, 0.035)))
        .init_resource::<ambition_render::quality::ResolvedVisualQuality>()
        .insert_resource(windowing::DisplayModeState::default())
        .register_type::<DeveloperTools>()
        .register_type::<PlayerBodyProfile>()
        .register_type::<MovementProfile>()
        .register_type::<EditableAbilitySet>()
        .register_type::<EditableMovementTuning>()
        .register_type::<EditablePlayerStats>()
        .register_type::<SandboxFeelTuning>()
        .register_type::<ambition_gameplay_core::portal::PortalConvention>()
        .register_type::<ambition_gameplay_core::portal::PortalTuning>();

    #[cfg(feature = "portal_render")]
    app.register_type::<ambition_gameplay_core::portal::PortalVisualEffect>()
        .register_type::<ambition_gameplay_core::portal::PortalEffectSelection>()
        .register_type::<ambition_gameplay_core::portal::PortalCameraTransitMode>()
        .register_type::<ambition_gameplay_core::portal::PortalCameraContinuitySelection>()
        .register_type::<ambition_gameplay_core::portal::PortalCameraContinuityConfig>()
        .register_type::<ambition_gameplay_core::portal::PortalCameraContinuityState>()
        .register_type::<ambition_gameplay_core::portal::PortalViewConeConfig>();

    app.add_plugins(crate::host::platform::PlatformPlugin);
    app.add_plugins(ambition_render::screen_effects::ScreenEffectsPlugin);
    // Loads baked `*_spritesheet.ron` manifests for runtime sheet metadata.
    app.add_plugins(ambition_gameplay_core::character_sprites::SheetRegistryPlugin);
    app.add_plugins(crate::dev::DevToolsPlugin);
    add_physics_debris_plugins(app);
    add_ui_plugins(app);
    add_input_plugins(app);
    add_audio_plugins(app);
    add_mobile_touch_plugin(app);
    #[cfg(feature = "falling_sand")]
    app.add_plugins(ambition_gameplay_core::falling_sand::FallingSandRoomPlugin);
    // Frame pacing / battery saver. Enabled by the normal visible personas so
    // desktop and Android exercise the same pacing behavior by default.
    #[cfg(feature = "frame_pacing")]
    app.add_plugins(crate::host::framepace::FramePacePlugin);

    app.add_systems(Startup, ui_fonts::load_ui_fonts);
    app.add_systems(
        Update,
        (
            ambition_render::quality::sync_resolved_visual_quality,
            reload_visual_quality_assets_on_scale_change,
            ambition_render::rendering::refresh_entity_sprite_handles_on_game_assets_change,
            ambition_render::rendering::refresh_parallax_layers_on_quality_change,
        )
            .chain(),
    );
    #[cfg(feature = "portal_render")]
    app.add_systems(Update, ambition_render::quality::sync_portal_quality_budget);
}

/// Pause menu, inventory, map menu, presentation startup, dev/dialog
/// hotkeys.
fn install_menu_setup_and_hotkeys(app: &mut App) {
    // Starter item-ownership roster (the 24-item catalog default set).
    app.add_plugins(ambition_content::items::AmbitionItemRosterPlugin);
    app.insert_resource(inventory_ui::InventoryUiState::default())
        .init_resource::<ambition_gameplay_core::items::persist::InventoryRestored>()
        // Persist the inventory + wallet across save/load: restore the saved set
        // once the player exists, then mirror live changes back into the save
        // (the existing autosave writes the dirtied save to disk).
        .add_systems(
            Update,
            (
                ambition_gameplay_core::items::persist::restore_inventory_from_save,
                ambition_gameplay_core::items::persist::persist_inventory_to_save,
            )
                .chain(),
        )
        .add_systems(
            Update,
            (ambition_gameplay_core::menu::map::sync_map_menu,).after(SandboxSet::CoreSimulation),
        )
        .add_systems(
            Startup,
            (
                ambition_gameplay_core::dev::profiling::phase_mark("before_setup_presentation"),
                // `PresentationSetupSet` is the machinery-facing label for
                // this slot: audio init (and any future machinery startup
                // work) orders `.after(the set)` instead of naming this
                // app system.
                setup_presentation_system
                    .in_set(ambition_gameplay_core::schedule::PresentationSetupSet),
                ambition_gameplay_core::dev::profiling::phase_mark("after_setup_presentation"),
                ambition_gameplay_core::menu::map::populate_map_rooms,
                ambition_gameplay_core::menu::map::spawn_map_menu,
                ambition_gameplay_core::dev::profiling::phase_mark("after_map_menu_spawn"),
            )
                .chain()
                .after(setup_simulation_system)
                .after(ui_fonts::load_ui_fonts),
        )
        .add_systems(
            Update,
            (
                dialog::dialog_input,
                handle_ldtk_hot_reload,
                handle_debug_hotkeys,
                dev_tools::sync_developer_body_profile,
                ambition_gameplay_core::trace::handle_trace_hotkey,
                ambition_gameplay_core::menu::map::handle_map_menu_hotkeys,
            )
                .chain()
                .after(SandboxSet::CoreSimulation),
        );

    // Unified menu (the one menu): install backend-agnostic menu state first,
    // then install each compiled backend independently. The backend features are
    // platform-neutral so desktop and Android stay in sync unless a build profile
    // intentionally opts out of a backend.
    crate::menu::kaleidoscope_app::install_unified_menu_shared(app);
    if ambition_gameplay_core::menu::backend::KALEIDOSCOPE_MENU_BACKEND_ENABLED {
        crate::menu::kaleidoscope_app::install_kaleidoscope_menu_backend(app);
    }
    #[cfg(feature = "bevy_ui_menu")]
    if ambition_gameplay_core::menu::backend::BEVY_UI_MENU_BACKEND_ENABLED {
        crate::menu::grid_backend::install_grid_unified_menu(app);
    }
}

fn install_camera_and_debug_overlay_systems(app: &mut App) {
    app.init_resource::<debug_overlay::DebugOverlayLabels>();
    app.add_systems(
        Update,
        (
            ambition_gameplay_core::time::camera_ease::tick_camera_shake,
            camera_follow,
            debug_overlay::draw_debug_overlay,
            // Materialize the labels the overlay just queued (Text2d). Runs
            // right after so the labels track this frame's boxes.
            debug_overlay::render_debug_overlay_labels,
        )
            .chain()
            .after(animate_bosses),
    );
    #[cfg(feature = "portal_render")]
    app.add_systems(
        Update,
        ambition_gameplay_core::portal::apply_portal_camera_continuity
            .after(SandboxSet::CoreSimulation)
            .after(ambition_gameplay_core::portal::sync_portal_camera_continuity_focus)
            .before(camera_follow),
    );
    #[cfg(feature = "portal_render")]
    app.add_systems(
        Update,
        ambition_gameplay_core::portal::tag_portal_camera_continuity_camera
            .after(camera_follow)
            .before(debug_overlay::draw_debug_overlay),
    );
}

fn install_fx_and_hud_systems(app: &mut App) {
    app.add_systems(
        Update,
        (
            fx::update_particles,
            fx::update_explosions,
            fx::update_impacts,
            fx::update_speech_bubbles,
            fx::update_speech_bubble_outlines,
            windowing::window_mode_hotkeys,
        )
            .chain()
            .after(debug_overlay::draw_debug_overlay),
    )
    .add_systems(
        Update,
        (
            update_hud,
            ambition_render::rendering::sync_boss_health_bar_overlay,
            dialog::dialog_reveal_tick,
            ambition_render::dialog_ui::sync_dialog_ui,
            ambition_render::cutscene::sync_cutscene_ui,
        )
            .chain()
            .after(windowing::window_mode_hotkeys),
    )
    // Always-on player HUD overlay (health / mana / money bars). Spawns once
    // a player exists, then mirrors the meters each frame. Mana regen is a
    // gameplay system (sim dt), kept in this group for cohesion.
    .add_systems(
        Update,
        (
            ambition_render::hud::regen_player_mana,
            ambition_render::hud::spawn_player_hud,
            ambition_render::hud::update_player_hud,
        )
            .chain(),
    );
}

/// Health overlays, portal sprite sync, parallax, dialog redirect,
/// lock-wall visuals, NPC sprite upgrade, map-menu pointer dismiss,
/// quest panel. Each system is its own `add_systems` call because the
/// big presentation tuple is already at Bevy's 20-system arity ceiling.
fn install_misc_visual_sync_systems(app: &mut App) {
    #[cfg(feature = "portal_render")]
    app.add_systems(
        Update,
        ambition_render::rendering::sync_portal_capture_parallax_layers
            .after(ambition_gameplay_core::portal::PortalPresentationSet),
    );

    app.add_systems(
        Update,
        ambition_render::rendering::sync_health_overlays.after(sync_visuals),
    )
    // Idle barks fire on a 5-10s cadence while the boss is in an
    // attacking phase, so the scholar feels alive between strikes.
    .add_systems(Update, ambition_content::bosses::tick_boss_idle_barks)
    // Portal presentation: read GatePortalRegistry.phase + apply
    // visibility / animation row / ring-spin to the matching
    // FeatureName-tagged sprites + hide the redundant debug
    // door-zone visual for portal-mode LoadingZones. Visible-
    // only; headless has no FeatureName ↔ Bevy-entity binding
    // anyway. Runs after sync_visuals so the sprite entities
    // exist this frame.
    .add_systems(
        Update,
        (
            ambition_gameplay_core::rooms::sync_portal_sprite_visibility,
            ambition_gameplay_core::rooms::sync_portal_sprite_animation,
            ambition_gameplay_core::rooms::sync_portal_ring_rotation_system,
            ambition_gameplay_core::rooms::hide_portal_loading_zone_visuals,
        )
            .after(sync_visuals),
    )
    .add_systems(
        Update,
        ambition_render::rendering::sync_parallax_layers.after(camera_follow),
    )
    // Encounter / intro LockWall visuals. Reconciles `LockWallVisual`
    // Bevy entities against the collision overlay's `gate_solids` (the
    // lock walls the gate contributors derive each frame in WorldPrep,
    // no longer mutated into the authored base) so the wall is visible
    // when an encounter slams it shut. Pinned after
    // `update_encounters_from_world` so it runs late in the frame, well
    // after the WorldPrep contributor has populated `gate_solids`.
    .add_systems(
        Update,
        ambition_render::rendering::sync_lock_wall_visuals
            .after(ambition_gameplay_core::encounter::update_encounters_from_world),
    )
    // Dev "hide sprites" / "placeholder sprites" overrides — must run
    // after every other visibility- or sprite-setting system so they
    // win the last-write battle. `sync_morph_ball_visual`,
    // `sync_bubble_shield_visual`, and the projectile rebuild systems
    // all also run `.after(sync_visuals)` and unconditionally set
    // `Visibility` (or despawn-respawn fresh `Inherited` sprites). If
    // the override ran in parallel, Bevy could schedule either order
    // and the player / shield / projectile sprites would sporadically
    // remain visible. Explicit ordering keeps the toggle deterministic.
    .add_systems(
        Update,
        (
            ambition_render::rendering::apply_placeholder_sprites_override,
            ambition_render::rendering::apply_hide_sprites_override,
        )
            .chain()
            .after(sync_visuals)
            .after(ambition_render::rendering::morph_ball::sync_morph_ball_visual)
            .after(ambition_render::rendering::bubble_shield::sync_bubble_shield_visual)
            .after(ambition_render::rendering::projectile_visuals::sync_projectile_visuals),
    )
    // Mouse / touch dismissal for the map menu.
    .add_systems(
        Update,
        ambition_gameplay_core::menu::map::map_menu_pointer_dismiss,
    )
    // Quest panel runs alongside the verbose HUD.
    .add_systems(
        Update,
        update_quest_panel.after(ambition_render::dialog_ui::sync_dialog_ui),
    );
}

/// Projectile sprite ring + VFX/debris message subscribers + (input-
/// feature-gated) blink preview ring.
fn install_projectile_and_vfx_systems(app: &mut App) {
    // Player projectile visuals: rebuild the sprite ring each tick
    // from `PlayerProjectileState::bodies`. Must run after
    // `update_projectiles` so the body list reflects this frame's
    // spawn / tick / collision before the visuals are rebuilt —
    // otherwise newly-fired projectiles would only become visible
    // one frame late.
    app.add_systems(
        Update,
        (
            // One unified, kind-driven visual pass for ALL projectiles (player +
            // enemy); the charge indicator is its own player-only pass.
            ambition_render::rendering::projectile_visuals::sync_projectile_visuals
                .after(ambition_gameplay_core::projectile::step_projectiles),
            ambition_render::rendering::projectile_visuals::sync_projectile_charge_visuals
                .after(ambition_gameplay_core::projectile::step_projectiles),
        ),
    )
    // VFX + debris subscribe on the visible binary only. Audio's
    // subscriber lives in `add_audio_plugins` so the entire kira
    // chain stays behind the `audio` feature. Headless builds omit
    // these so the message queues drain without entity spawns or
    // audio playback.
    .add_systems(
        Update,
        (
            fx::process_fireworks_requests,
            fx::tick_firework_sequences,
            fx::process_explosion_requests,
        )
            .chain()
            .after(SandboxSet::CoreSimulation)
            .before(vfx_spawn_messages),
    )
    .add_systems(
        Update,
        vfx_spawn_messages.after(fx::process_explosion_requests),
    );
    // Live blink-destination preview ring. Reads leafwing action state to
    // know when the blink button is held, so it lives behind the `input`
    // feature alongside the other gameplay-input-driven presentation.
    #[cfg(feature = "input")]
    app.add_systems(
        Update,
        fx::update_blink_preview.after(SandboxSet::CoreSimulation),
    );
}

/// Install the Avian2D secondary-physics plugin and its presentation-side
/// debris subscriber. Gated by `physics_debris` so headless / minimal
/// builds drop `avian2d` from the dep graph entirely. Per ADR 0007, this
/// is secondary physics for debris/ragdoll visuals only — the player
/// controller stays kinematic.
#[cfg(feature = "physics_debris")]
pub(super) fn add_physics_debris_plugins(app: &mut App) {
    app.add_plugins(physics::AmbitionPhysicsPlugin).add_systems(
        Update,
        physics_spawn_debris_messages.after(SandboxSet::CoreSimulation),
    );
}

#[cfg(not(feature = "physics_debris"))]
pub(super) fn add_physics_debris_plugins(_app: &mut App) {}

/// Install UI-shell plugins: bevy_material_ui's styling layer. The
/// dialogue overlay (`ambition_render::dialog_ui::sync_dialog_ui`) draws with Bevy's
/// core UI primitives and stays installed unconditionally; only
/// the optional plugins live behind `ui`.
///
/// `YarnSpinnerPlugin` is mounted alongside the bridge in
/// `build_sandbox_simulation_plugins` so the yarn runtime,
/// the bridge observers, and the binding registrations all spawn
/// in one place. Don't re-mount it here.
#[cfg(feature = "ui")]
pub(super) fn add_ui_plugins(app: &mut App) {
    app.add_plugins(MaterialUiPlugin);
}

#[cfg(not(feature = "ui"))]
pub(super) fn add_ui_plugins(_app: &mut App) {}

/// Install the leafwing-input-manager plugin, the player-input attach
/// startup system, and the bridge that keeps `Res<ControlFrame>` in sync
/// with leafwing's `ActionState`. Gated behind `input` so headless /
/// minimal builds can drop `leafwing-input-manager` from the dep graph;
/// the sim itself reads `Res<ControlFrame>` (always-available) and is
/// agnostic to where the frame came from.
#[cfg(feature = "input")]
pub(super) fn add_input_plugins(app: &mut App) {
    app.init_resource::<MenuInputState>()
        .init_resource::<MenuControlFrame>()
        .init_resource::<PlayerDashTriggerState>()
        .init_resource::<ambition_input::ActiveInputKind>()
        .add_plugins(InputManagerPlugin::<SandboxAction>::default())
        // Track which input source is CURRENTLY active (last to produce
        // GENUINE input). This gates the menu mouse-hover handlers so a
        // rebuild-induced `Pointer<Over>` under a stationary mouse can't
        // snap the cursor back while the player navigates with the
        // keyboard / gamepad / touch. Runs in the input populate set so
        // the value is fresh before this frame's menu consumers + before
        // the hover observers fire on rebuilt controls. The detector
        // covers keyboard / mouse / gamepad; the touch fold in the
        // mobile_input plugin flips it to `Touch` itself.
        .add_systems(
            Update,
            ambition_input::update_active_input_kind.in_set(ambition_input::InputSet::Populate),
        )
        .add_systems(
            Startup,
            attach_player_input_components.after(setup_simulation_system),
        )
        // Collect semantic menu intent before gameplay input is suppressed.
        // `populate_control_frame_from_actions` may zero the sim-side
        // `ControlFrame` in UI modes, but it must not mutate leafwing's
        // `ActionState`; held keyboard/menu buttons should not become
        // `just_pressed` again on every dialog frame.
        //
        // Therefore the order is:
        // 1. read keyboard/gamepad menu actions into `MenuControlFrame`,
        // 2. read/suppress gameplay into `ControlFrame`,
        // 3. let touch folds merge into both seams before the consumers below.
        //
        // Touch fold (mobile_input plugin) runs
        // `.after(populate_control_frame_from_actions)` for gameplay and
        // `.after(populate_menu_control_frame_from_actions)` for menus, then
        // `.before(MenuNavConsume)` (the unified menu's nav set), so pause /
        // inventory / navigation see keyboard, gamepad, and touch contributions
        // in one frame.
        .add_systems(
            Update,
            (
                populate_menu_control_frame_from_actions,
                populate_control_frame_from_actions.in_set(ambition_input::InputSet::Populate),
                toggle_player_trail_emission_from_actions,
                apply_menu_frame_to_cutscene_request,
                dialog::dialog_pointer_input,
            )
                .chain()
                .before(SandboxSet::CoreSimulation),
        )
        .add_systems(
            Update,
            sync_preset_input_map.before(SandboxSet::CoreSimulation),
        );
}

#[cfg(not(feature = "input"))]
pub(super) fn add_input_plugins(_app: &mut App) {}

/// Register the [`TouchControlsPlugin`](ambition_touch_input::TouchControlsPlugin)
/// (`virtual_joystick` sticks + on-screen action buttons that fold into
/// ControlFrame). The touch adapter lives in the sibling `ambition_touch_input`
/// crate now (app-thinness); the app's `mobile_touch` feature forwards to
/// `ambition_touch_input/mobile_touch`, which pulls the optional
/// `virtual_joystick` dep. Added UNCONDITIONALLY whenever `mobile_touch` is
/// compiled — no runtime boolean gates it. To rip the touch controls out, remove
/// the single `add_plugins(TouchControlsPlugin)` line below. On builds compiled
/// without `mobile_touch` this is a no-op.
///
/// The touch plugin runs ALONGSIDE the desktop input pipeline --
/// both write into the same `ControlFrame` resource, with the
/// mobile-side write happening after the desktop one in this
/// session's chain. On a phone, the desktop pipeline produces
/// neutral output (no keyboard / gamepad); on desktop, the mobile
/// stick UI is invisible without touch input, so neither path
/// stomps the other in practice. A future polish pass can detect
/// the active input source (touch vs keyboard) and skip the
/// inactive folder.
#[cfg(feature = "mobile_touch")]
pub(super) fn add_mobile_touch_plugin(app: &mut App) {
    app.add_plugins(ambition_touch_input::TouchControlsPlugin);
}

#[cfg(not(feature = "mobile_touch"))]
pub(super) fn add_mobile_touch_plugin(_app: &mut App) {}

/// Install the sandbox audio subsystem. Gated by `audio` so headless
/// / RL builds drop `bevy_kira_audio` from the dep graph entirely;
/// the sim still emits `SfxMessage`s and the queue drains harmlessly
/// per the ADR 0012 seam.
#[cfg(feature = "audio")]
pub(super) fn add_audio_plugins(app: &mut App) {
    app.add_plugins(ambition_gameplay_core::audio::SandboxAudioPlugin);
}

#[cfg(not(feature = "audio"))]
pub(super) fn add_audio_plugins(_app: &mut App) {}

// ── Domain plugin structs ──────────────────────────────────────────────────
//
// These are the public Bevy `Plugin` API for callers that just want to
// `app.add_plugins(…)` without knowing about the internal helper functions.
// The helper functions (`init_sandbox_resources`, `add_simulation_plugins`,
// etc.) stay public so callers that need to inject resources between steps
// (e.g. inserting `StartRoomOverride` before resources are consumed) can
// still call them in sequence.

/// Installs all sandbox simulation resources and systems — the subset
/// that is safe for both visible and headless builds. Calls
/// `init_sandbox_resources` then `add_simulation_plugins`.
pub struct SandboxSimulationPlugin;

impl Plugin for SandboxSimulationPlugin {
    fn build(&self, app: &mut App) {
        // `init_sandbox_resources` installs the named boss roster first (it
        // resolves `BossBehaviorProfile::from_data` while populating the boss
        // encounter registry).
        init_sandbox_resources(app);
        add_simulation_plugins(app);
    }
}

/// Installs LDtk runtime spine registrations and `LdtkPlugin`. Visible
/// binary only — `LdtkPlugin` panics in headless (no `RenderApp`).
pub struct SandboxLdtkPlugin;

impl Plugin for SandboxLdtkPlugin {
    fn build(&self, app: &mut App) {
        add_ldtk_runtime_plugin(app);
    }
}

/// Installs all presentation-side plugins: input, audio, VFX, HUD, debug
/// overlays, and platform plugins. Visible binary only.
pub struct SandboxPresentationPlugin;

impl Plugin for SandboxPresentationPlugin {
    fn build(&self, app: &mut App) {
        add_presentation_plugins(app);
    }
}

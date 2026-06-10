#[allow(unused_imports)]
use super::cli::*;
#[allow(unused_imports)]
use super::dev_runtime::*;
#[allow(unused_imports)]
use super::feedback::*;
#[allow(unused_imports)]
use super::hud::*;
#[allow(unused_imports)]
use super::input_systems::*;
#[allow(unused_imports)]
use super::phases::*;
#[allow(unused_imports)]
use super::player_tick::*;
#[allow(unused_imports)]
use super::resources::*;
#[allow(unused_imports)]
use super::schedule::*;
#[allow(unused_imports)]
use super::setup_systems::*;
#[allow(unused_imports)]
use super::sim_systems::*;
#[allow(unused_imports)]
use super::world_flow::*;
#[allow(unused_imports)]
use super::*;

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
    // the player controller is custom via parry2d in crate::engine_core.
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
    app.init_resource::<crate::shrine::ShrineActivationPulse>();

    app.add_plugins(super::sim_resources::SandboxSimulationResourcesPlugin);

    // Named Ambition game content (quests, bosses, cutscenes/banter, the
    // intro/cut-rope story hooks, and — under the `portal` feature — the
    // portal adapters). Registration ownership lives behind one composer
    // so the named-content rosters are constructed in a single content-
    // owned place instead of inline in `sim_resources.rs`. Installed right
    // after the simulation resources so the registries land at the same
    // point in app assembly as before (preserving replay / scripted
    // ordering).
    app.add_plugins(crate::ambition_content::AmbitionContentPlugin);

    // Yarn dialogue stack (gated by `ui` feature):
    //   1. `yarn_spinner_plugin()` — bevy_yarnspinner: compiles
    //      `.yarn` files into a `YarnProject` resource at startup.
    //   2. `YarnBridgePlugin` — spawns the persistent `DialogueRunner`
    //      entity once `YarnProject` resolves + registers observers
    //      that translate Yarn lifecycle events into sandbox state.
    //   3. `YarnBindingsPlugin` — registers custom commands /
    //      functions / markup the .yarn content can invoke (phase
    //      1: empty scaffold; phases 2-4 fill it in).
    #[cfg(feature = "ui")]
    {
        app.add_plugins(crate::dialog::yarn_spinner_plugin());
        app.add_plugins(crate::dialog::YarnBridgePlugin);
        app.add_plugins(crate::dialog::YarnBindingsPlugin);
    }

    app.add_plugins(crate::features::WorldPrepSchedulePlugin);
    // Universal-brain plugin: registers ActorActionMessage +
    // BrainActionCounter resource. Scheduling of the per-tick
    // systems lives in register_player_input_systems below.
    app.add_plugins(crate::brain::BrainPlugin);
    register_player_input_systems(app);
    register_player_simulation_systems(app);
    // Ambition's player ability / weapon kit (blink, dive, grapple,
    // possession, beam, meteor, …). The 14 abilities are mostly pure-logic
    // modules driven from combat / item-pickup / projectile code; this
    // umbrella plugin owns the only Bevy `App` state any of them register
    // today (possession's `PossessionState` resource). A sibling of the
    // mechanic plugins below.
    app.add_plugins(crate::abilities::AmbitionAbilitiesPlugin);
    // Gravity-zone mechanic (Stage 6 follow-up): the gravity zones / switches
    // that flip ambient gravity + their per-frame snapshot. Owns its own
    // resources + scheduling; installed alongside the other mechanic plugins and
    // independent of the `portal` feature (it used to live inside portal).
    app.add_plugins(crate::mechanics::gravity::GravityPlugin);
    #[cfg(feature = "portal")]
    {
        app.add_plugins(crate::portal::PortalPlugin);
        // Wire portal's internal `PortalSet`s into the sandbox app schedule.
        // `crate::portal` owns only its own intra-set ordering; the sandbox
        // declares the cross-set phase placement, `.before`/`.after` edges, and
        // `gameplay_allowed` gating here so portal can become a standalone crate
        // without naming `SandboxSet`/app-systems/`gameplay_allowed`/etc.
        wire_portal_schedule(app);
    }
    // The Ambition portal adapters (AmbitionPortalAdaptersPlugin) are now
    // installed by `crate::ambition_content::AmbitionContentPlugin` above so
    // all Ambition content lives in one composer.
    app.add_plugins(crate::items::pickup::ItemPickupSimulationPlugin);
    register_room_transition_systems(app);
    app.add_plugins(super::combat_schedule::CombatSchedulePlugin);
    register_presentation_sync_systems(app);
    app.add_plugins(crate::features::FeatureCollectionSchedulePlugin);
    app.add_plugins(crate::features::FeatureInteractionSchedulePlugin);
    app.add_plugins(ldtk_world::LdtkRuntimeSpinePlugin);
    app.add_plugins(crate::encounter::EncounterSimulationSchedulePlugin);
    app.add_plugins(crate::presentation::cutscene::CutsceneSchedulePlugin);
    app.add_plugins(crate::features::GameplayEffectsSchedulePlugin);
    app.add_plugins(super::progression_schedule::ProgressionSchedulePlugin);
    app.add_plugins(crate::features::FeatureViewSyncSchedulePlugin);
    app.add_plugins(crate::runtime::reset::SandboxResetSchedulePlugin);
    app.add_plugins(crate::trace::TraceSchedulePlugin);
    // Per-frame "what would each verb do right now?" table consumed
    // by the touch / control-prompt HUD and (future) gameplay code.
    // Registered alongside simulation so headless / RL builds can
    // also inspect affordances; the resources are cheap and the
    // compute systems no-op when no primary player exists.
    app.add_plugins(crate::player::affordances::AffordancesPlugin);
}

/// Place portal's internal [`crate::portal::PortalSet`] variants into the
/// sandbox app schedule.
///
/// `crate::portal::PortalPlugin` registers each portal system `.in_set(PortalSet::X)`
/// with only PORTAL-INTERNAL ordering (edges between portal's own sets). This
/// function — the sandbox side of the seam — declares everything that ties
/// portal to the host app schedule:
///   * which `SandboxSet` phase each `PortalSet` runs in,
///   * the cross-set `.before`/`.after` edges against concrete sandbox systems
///     (`interaction_input_system`, `sync_local_player_input_frame`,
///     `player_simulation_system`, `collect_gravity_zones`) and sets
///     (`ItemPickupSet::CoreHeldItems`, `reset_cut_rope_boss_arena_on_room_reset`,
///     `SandboxSet::CoreSimulation`),
///   * the `gameplay_allowed` run condition.
///
/// These are EXACTLY the edges `portal/plugin.rs` used to bake; moving where they
/// are declared does not change the resulting execution order.
#[cfg(feature = "portal")]
fn wire_portal_schedule(app: &mut App) {
    use crate::portal::PortalSet;

    // Carves: published with the same early-world snapshot cadence as the
    // gravity-zone snapshot — after `collect_gravity_zones`, before
    // `CoreSimulation` (byte-identical to the pre-extraction
    // `oscillate → collect → carves` chain).
    app.configure_sets(
        Update,
        PortalSet::Carves
            .after(crate::physics::collect_gravity_zones)
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
            .before(crate::player::sync_local_player_input_frame)
            .run_if(crate::gameplay_allowed),
    );

    // WeaponAndProjectiles: gameplay-gated weapon/projectile systems in the
    // player-simulation phase. WeaponMaintenance (orphan cleanup + roll
    // readiness) runs in the same phase but stays UNGATED, matching the
    // pre-extraction per-system gating; portal already chains it after
    // WeaponAndProjectiles.
    app.configure_sets(
        Update,
        PortalSet::WeaponAndProjectiles
            .in_set(SandboxSet::PlayerSimulation)
            .run_if(crate::gameplay_allowed),
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
            .after(crate::runtime::reset::ContentRoomResetSet),
    );

    // TransitGuards: suppress ledge-grab while transiting, before player
    // simulation integrates movement, gated to gameplay.
    app.configure_sets(
        Update,
        PortalSet::TransitGuards
            .in_set(SandboxSet::PlayerSimulation)
            .before(crate::app::player_simulation_system)
            .run_if(crate::gameplay_allowed),
    );

    // Transit: teleports run after player + ground-item integration so this
    // frame's integrated body positions are what cross the portal, gated to
    // gameplay.
    app.configure_sets(
        Update,
        PortalSet::Transit
            .in_set(SandboxSet::PlayerSimulation)
            .after(crate::app::player_simulation_system)
            .after(crate::items::pickup::ItemPickupSet::CoreHeldItems)
            .run_if(crate::gameplay_allowed),
    );
}

// Core simulation, split into 6 finer-grained sub-sets that are
// chained inside `SandboxSet::CoreSimulation`. See
// `schedule.rs::configure_sandbox_sets` for the sub-set ordering.
// External presentation/audio/HUD systems still pin against
// `SandboxSet::CoreSimulation`; that constraint covers all six
// sub-sets transitively.

// WorldPrep schedule moved to `crate::features::WorldPrepSchedulePlugin`
// (OVERNIGHT-TODO #6 — module-local plugins).

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
            crate::time::time_control::emit_player_time_intent_system.run_if(gameplay_allowed),
            crate::time::time_control::apply_clock_scale_requests.run_if(gameplay_allowed),
            crate::time::time_control::smooth_sim_clock_toward_target_system
                .run_if(gameplay_allowed),
            // Unconditional: snapshot whichever path (suspended-zero
            // or gameplay-smoothed) wrote `SandboxSimState::time_scale`
            // this frame into `WorldTime` for downstream readers.
            crate::refresh_world_time,
            // Mirror the freshly-snapshotted `WorldTime::sim_dt()` into the
            // runtime crate's neutral `SimDt` so every downstream runtime
            // system (gravity / zones / orient-roll) reads scaled dt without a
            // sandbox dependency. Runs immediately after `refresh_world_time`.
            crate::mirror_sim_dt_into_runtime,
            sync_live_player_dev_edits_system,
            apply_player_reset_input_system.run_if(gameplay_allowed),
            crate::ambition_content::bosses::emit_cut_rope_room_replay_after_dialogue_closes,
            apply_cut_rope_room_replay_request_system,
            input_timer_system
                .run_if(gameplay_allowed)
                .in_set(crate::input::InputSet::Populate),
            interaction_input_system.run_if(gameplay_allowed),
            // Portal-warped held movement input is registered by
            // `crate::portal::PortalPlugin` so the portal subsystem owns
            // its input seam.
            // Per-player input migration (OVERNIGHT-TODO #17.5). Mirror
            // the now-final `Res<ControlFrame>` onto the local primary
            // player's `PlayerInputFrame` so simulation systems can
            // move toward reading input from a Query<&PlayerInputFrame>
            // rather than the single global resource. Runs last in the
            // PlayerInput phase so every input writer (leafwing, mobile
            // bridge, RL) has finalized the resource for this frame.
            crate::player::sync_local_player_input_frame,
            // Ladder body-mode policy needs the freshly mirrored input
            // frame, but it must still run before the player tick so
            // climb/jump/dash exits land on the same frame as the edge.
            crate::body_mode::update_body_mode,
            // Universal-brain seam: translate PlayerInputFrame into
            // the player's ActorControl frame. Runs after the input
            // sync so the brain sees this frame's inputs. The
            // ActorControl output is the polarity-flip authority for
            // `player_control_system` / `player_simulation_system`
            // (see `engine_input_from_actor_control`).
            crate::player::tick_player_brains,
            crate::player::sync_player_actor_poses,
            // Universal-brain effects resolver: walk every actor's
            // ActionSet against the actor's ActorControl frame and
            // emit ActorActionMessage entries for each concrete
            // request. Live consumers read the emitted stream in
            // Combat: enemy ranged, enemy melee start, player
            // melee + pogo start gating, GNU-ton apple rain, and
            // Gradient Sentinel specials.
            crate::brain::emit_brain_action_messages,
            // Sibling emitter: for every player-brain actor, surface
            // the per-tick projectile state (axis sample + press /
            // held / released edges) into the same ActorActionMessage
            // channel under `ActionRequest::PlayerProjectileTick`.
            // `crate::projectile::update_projectiles` consumes this
            // stream instead of reading `PlayerInputFrame` directly,
            // so player projectile charging now flows through the
            // universal action seam.
            crate::brain::emit_player_projectile_tick_messages,
            // Observe the resolver output into a per-frame counter
            // so the HUD + debug tooling have a quick "any brain
            // wants something this tick" signal.
            crate::brain::observe_brain_action_counter,
        )
            .chain()
            .in_set(SandboxSet::PlayerInput),
    );
}

/// Main player tick (two-clock control + simulation) plus post-sim
/// damage / safe-respawn.
///
/// Per the actor/brain migration, the player tick is no longer
/// a monolithic `sandbox_update` orchestrator. Instead:
///
/// 1. `clear_sandbox_reset_this_frame` zeros the per-frame reset
///    flag at the start of the player tick.
/// 2. `player_control_system` runs the control-clock half. If
///    `update_player_control_with_clusters` reports a reset, the
///    `SandboxResetThisFrame` flag is set.
/// 3. `player_simulation_system` runs the sim-clock half. It
///    short-circuits when the flag is set so the same-frame reset
///    isn't clobbered. May set the flag itself if its own engine
///    call reports a reset.
/// 4. `apply_player_hit_events` drains pending damage events.
///
/// Both systems read `ActorControl` as the brain-output authority +
/// `PlayerInputFrame` for player-specific verbs (the polarity flip).
fn register_player_simulation_systems(app: &mut App) {
    app.init_resource::<crate::app::SandboxResetThisFrame>();
    // `PossessionState` is now initialized by
    // `crate::abilities::AmbitionAbilitiesPlugin` (installed in
    // `add_simulation_plugins`). The possession *systems* below stay chained
    // here because they are interleaved with the player control / simulation
    // tick (`not_possessing` run conditions inside this `.chain()`); lifting
    // them would change execution order.
    app.add_systems(
        Update,
        (
            clear_sandbox_reset_this_frame,
            // Possession: Down+Interact takes over a nearby actor. The trigger +
            // input sync run before the player tick so the possessed actor reads
            // fresh input; the player's own control is gated OFF while possessing
            // so the same input doesn't drive both bodies.
            crate::abilities::traversal::possession::possession_trigger_system
                .run_if(gameplay_allowed),
            crate::abilities::traversal::possession::release_possession_if_target_lost,
            crate::abilities::traversal::possession::sync_possession_input,
            player_control_system
                .run_if(gameplay_allowed)
                .run_if(crate::abilities::traversal::possession::not_possessing),
            player_simulation_system.run_if(gameplay_allowed),
            apply_player_hit_events.run_if(gameplay_allowed),
        )
            .chain()
            .in_set(SandboxSet::PlayerSimulation),
    );
}

// Portal and held-item simulation schedules moved to
// `crate::portal::PortalPlugin` and
// `crate::items::pickup::ItemPickupSimulationPlugin`.

/// Detection emits `RoomTransitionRequested`; apply consumes it and runs
/// `load_room`; the feature-side `reset_ecs_room_features` system tears
/// down per-room ECS state.
fn register_room_transition_systems(app: &mut App) {
    app.add_systems(
        Update,
        (
            detect_room_transition_system.run_if(gameplay_allowed),
            ensure_requested_room_parallax_system,
            apply_room_transition_system,
            crate::features::reset_ecs_room_features,
            crate::features::reset_ecs_npc_actors,
            // Content-side reset work carries the ContentRoomResetSet
            // label so generic plugins (gravity, portal) can order
            // after it without naming content systems.
            crate::ambition_content::bosses::reset_cut_rope_boss_arena_on_room_reset
                .in_set(crate::runtime::reset::ContentRoomResetSet),
            // Portal room-reset cleanup is registered by
            // `crate::portal::PortalPlugin`.
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
            crate::player::write_player_ecs_components,
            cleanup_timers_system,
        )
            .chain()
            .in_set(SandboxSet::PresentationSync),
    );
}

// FeatureViewSync, FeatureCollection, and FeatureInteraction schedules
// moved to `crate::features::{FeatureViewSyncSchedulePlugin,
// FeatureCollectionSchedulePlugin, FeatureInteractionSchedulePlugin}`
// (OVERNIGHT-TODO #6 — module-local plugins). The per-frame
// `FeatureViewIndex` rebuild gets its own set rather than living at
// the end of `PresentationSync` because pickup / chest / switch /
// encounter-mob / save-driven boss sync mutations land in sets that
// fire *after* `CoreSimulation`; rebuilding at the very tail of the
// sim chain guarantees the cache reflects this frame's full feature
// state before the presentation half reads it.

// LDtk runtime spine schedule moved to
// `ldtk_world::LdtkRuntimeSpinePlugin` (OVERNIGHT-TODO #6).

// EncounterSimulation schedule moved to
// `crate::encounter::EncounterSimulationSchedulePlugin` (OVERNIGHT-TODO #6).

// Progression chain: cutscenes, gameplay-effect routing, boss encounters,
// quest events, and the F3 stats editor sync. Split into several chained
// groups so each tuple stays under Bevy's macro arity limit while
// preserving the old drain-before-progression order.

// Cutscene schedule moved to
// `crate::presentation::cutscene::CutsceneSchedulePlugin` (OVERNIGHT-TODO #6).

// Gameplay-effects bus schedule moved to
// `crate::features::GameplayEffectsSchedulePlugin` (OVERNIGHT-TODO #6).

// Progression schedule moved to
// `super::progression_schedule::ProgressionSchedulePlugin` (ecs-cleanup-plan #8).
// Sandbox reset schedule moved to
// `crate::runtime::reset::SandboxResetSchedulePlugin` (OVERNIGHT-TODO #6).
// Trace recorder schedule moved to `crate::trace::TraceSchedulePlugin`
// (OVERNIGHT-TODO #6 — module-local plugins). `add_simulation_plugins`
// installs them via `app.add_plugins`.

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
    app.add_plugins(crate::persistence::PersistenceSchedulePlugin);
    install_menu_setup_and_hotkeys(app);
    app.add_plugins(crate::presentation::rendering::PresentationVisualAnimationPlugin);
    install_camera_and_debug_overlay_systems(app);
    install_fx_and_hud_systems(app);
    install_misc_visual_sync_systems(app);
    app.add_plugins(crate::presentation::rendering::PlayerVisualSchedulePlugin);
    install_projectile_and_vfx_systems(app);
}

/// Visible-side resources, registered types, and presentation child
/// plugins (input, audio, dev_tools, physics_debris, ui, mobile touch,
/// FPS overlay, font loader).
fn install_presentation_resources_and_subplugins(app: &mut App) {
    app.insert_resource(ClearColor(Color::srgb(0.020, 0.024, 0.035)))
        .insert_resource(windowing::DisplayModeState::default())
        .register_type::<DeveloperTools>()
        .register_type::<PlayerBodyProfile>()
        .register_type::<MovementProfile>()
        .register_type::<EditableAbilitySet>()
        .register_type::<EditableMovementTuning>()
        .register_type::<EditablePlayerStats>()
        .register_type::<SandboxFeelTuning>();

    app.add_plugins(crate::host::platform::PlatformPlugin);
    app.add_plugins(crate::presentation::screen_effects::ScreenEffectsPlugin);
    // Loads `*_spritesheet.ron` manifests so the registry (and, in a
    // future pass, the consumer code) can read sheet sizing / feet
    // anchors / per-frame anchors from data instead of hardcoded
    // consts in `presentation::character_sprites::sheets`.
    app.add_plugins(crate::presentation::character_sprites::SheetRegistryPlugin);
    add_dev_tools_plugins(app);
    add_physics_debris_plugins(app);
    add_ui_plugins(app);
    add_input_plugins(app);
    add_audio_plugins(app);
    add_mobile_touch_plugin(app);
    #[cfg(feature = "falling_sand")]
    app.add_plugins(crate::falling_sand::FallingSandRoomPlugin);
    // Lightweight FPS / frame-time overlay. ON by default on wasm,
    // OFF on desktop; F3 toggles. See `crate::fps_overlay`.
    app.add_plugins(crate::dev::fps_overlay::FpsOverlayPlugin);
    // Frame pacing / battery saver. Enabled by the normal visible personas so
    // desktop and Android exercise the same pacing behavior by default.
    #[cfg(feature = "frame_pacing")]
    app.add_plugins(crate::host::framepace::FramePacePlugin);

    app.add_systems(Startup, ui_fonts::load_ui_fonts);
}

// Settings + sandbox-save persistence schedule moved to
// `crate::persistence::PersistenceSchedulePlugin` (OVERNIGHT-TODO #6).

/// Pause menu, inventory, map menu, presentation startup, dev/dialog
/// hotkeys.
fn install_menu_setup_and_hotkeys(app: &mut App) {
    // Starter item-ownership roster (the 24-item catalog default set).
    // Owned by the content boundary but installed here to preserve its
    // original presentation-scoped insertion point (Stage 11 / Task J).
    app.add_plugins(crate::ambition_content::items::AmbitionItemRosterPlugin);
    app.insert_resource(inventory::InventoryUiState::default())
        .insert_resource(inventory::PlayerInventory::starter())
        .init_resource::<crate::items::persist::InventoryRestored>()
        // Persist the inventory + wallet across save/load: restore the saved set
        // once the player exists, then mirror live changes back into the save
        // (the existing autosave writes the dirtied save to disk).
        .add_systems(
            Update,
            (
                crate::items::persist::restore_inventory_from_save,
                crate::items::persist::persist_inventory_to_save,
            )
                .chain(),
        )
        .add_systems(
            Update,
            (crate::menu::map::sync_map_menu,).after(SandboxSet::CoreSimulation),
        )
        .add_systems(
            Startup,
            (
                crate::dev::profiling::phase_mark("before_setup_presentation"),
                setup_presentation_system,
                crate::dev::profiling::phase_mark("after_setup_presentation"),
                crate::menu::map::populate_map_rooms,
                crate::menu::map::spawn_map_menu,
                crate::dev::profiling::phase_mark("after_map_menu_spawn"),
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
                crate::trace::handle_trace_hotkey,
                crate::menu::map::handle_map_menu_hotkeys,
            )
                .chain()
                .after(SandboxSet::CoreSimulation),
        );

    // Unified menu (the one menu): install backend-agnostic menu state first,
    // then install each compiled backend independently. The backend features are
    // platform-neutral so desktop and Android stay in sync unless a build profile
    // intentionally opts out of a backend.
    crate::menu::kaleidoscope_app::install_unified_menu_shared(app);
    if crate::menu::kaleidoscope_app::KALEIDOSCOPE_MENU_BACKEND_ENABLED {
        crate::menu::kaleidoscope_app::install_kaleidoscope_menu_backend(app);
    }
    #[cfg(feature = "bevy_ui_menu")]
    if crate::menu::kaleidoscope_app::BEVY_UI_MENU_BACKEND_ENABLED {
        crate::menu::grid_backend::install_grid_unified_menu(app);
    }
}

// Visual animation chain moved to
// `crate::presentation::rendering::PresentationVisualAnimationPlugin`
// (OVERNIGHT-TODO #6). The chain spawns visual entities for dynamic
// features plus the sprite/animation pipeline, chained after
// `handle_map_menu_hotkeys` in `SandboxSet::PresentationVisualSync`.
// `sync_visuals` and `upgrade_enemy_sprites` / `upgrade_npc_sprites`
// all read `FeatureViewIndex`; the
// `.after(SandboxSet::FeatureViewSync)` constraint lives on the set
// itself in `configure_sandbox_sets` so the ordering contract is
// testable via a probe in the same set rather than re-typed on every
// call site.

fn install_camera_and_debug_overlay_systems(app: &mut App) {
    app.add_systems(
        Update,
        (
            crate::time::camera_ease::tick_camera_shake,
            camera_follow,
            debug_overlay::draw_debug_overlay,
        )
            .chain()
            .after(animate_bosses),
    );
}

fn install_fx_and_hud_systems(app: &mut App) {
    app.add_systems(
        Update,
        (
            fx::update_particles,
            fx::update_explosions,
            fx::update_impacts,
            fx::update_slash_previews,
            fx::update_speech_bubbles,
            windowing::window_mode_hotkeys,
        )
            .chain()
            .after(debug_overlay::draw_debug_overlay),
    )
    .add_systems(
        Update,
        (
            update_hud,
            crate::presentation::rendering::sync_boss_health_bar_overlay,
            dialog::dialog_reveal_tick,
            dialog::sync_dialog_ui,
            crate::presentation::cutscene::sync_cutscene_ui,
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
            crate::presentation::hud::regen_player_mana,
            crate::presentation::hud::spawn_player_hud,
            crate::presentation::hud::update_player_hud,
        )
            .chain(),
    );
}

/// Health overlays, portal sprite sync, parallax, dialog redirect,
/// lock-wall visuals, NPC sprite upgrade, map-menu pointer dismiss,
/// quest panel. Each system is its own `add_systems` call because the
/// big presentation tuple is already at Bevy's 20-system arity ceiling.
fn install_misc_visual_sync_systems(app: &mut App) {
    app.add_systems(
        Update,
        crate::presentation::rendering::sync_health_overlays.after(sync_visuals),
    )
    // Idle barks fire on a 5-10s cadence while the boss is in an
    // attacking phase, so the scholar feels alive between strikes.
    .add_systems(
        Update,
        crate::ambition_content::bosses::tick_boss_idle_barks,
    )
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
            crate::rooms::sync_portal_sprite_visibility,
            crate::rooms::sync_portal_sprite_animation,
            crate::rooms::sync_portal_ring_rotation_system,
            crate::rooms::hide_portal_loading_zone_visuals,
        )
            .after(sync_visuals),
    )
    .add_systems(
        Update,
        crate::presentation::rendering::sync_parallax_layers.after(camera_follow),
    )
    // Yarn-driven dialog migration retired `redirect_post_quest_dialog`:
    // boss-cleared / flag-set redirects are now inline `<<if>>`
    // branches inside the `.yarn` files (the Yarn runner evaluates
    // them on each conversation start).
    // Encounter-driven LockWall visuals. Reconciles `LockWallVisual`
    // Bevy entities against `world.blocks` so the wall is visible
    // for the player when an encounter slams it shut. Must run
    // after `update_encounters_from_world` (which inserts /
    // removes the backing `lockwall:*` blocks) so we observe the
    // current frame's world state, not last frame's.
    .add_systems(
        Update,
        crate::presentation::rendering::sync_lock_wall_visuals
            .after(crate::encounter::update_encounters_from_world),
    )
    // NPC spritesheet upgrade. `.after(sync_visuals)` preserves the
    // ordering guarantee the chain otherwise provided (FeatureVisuals
    // must exist before we look them up).
    .add_systems(
        Update,
        crate::presentation::rendering::upgrade_npc_sprites.after(sync_visuals),
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
            crate::presentation::rendering::apply_placeholder_sprites_override,
            crate::presentation::rendering::apply_hide_sprites_override,
        )
            .chain()
            .after(sync_visuals)
            .after(crate::body_mode::sync_morph_ball_visual)
            .after(crate::player::bubble_shield::sync_bubble_shield_visual)
            .after(crate::projectile::sync_projectile_visuals)
            .after(crate::enemy_projectile::sync_enemy_projectile_visuals),
    )
    // Mouse / touch dismissal for the map menu.
    .add_systems(Update, crate::menu::map::map_menu_pointer_dismiss)
    // Quest panel runs alongside the verbose HUD.
    .add_systems(Update, update_quest_panel.after(dialog::sync_dialog_ui));
}

// Player visual schedule moved to
// `crate::presentation::rendering::PlayerVisualSchedulePlugin`
// (OVERNIGHT-TODO #6).

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
            crate::projectile::sync_projectile_visuals.after(crate::projectile::update_projectiles),
            crate::enemy_projectile::sync_enemy_projectile_visuals
                .after(crate::enemy_projectile::update_enemy_projectiles),
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

/// Install the egui inspector plugins. Gated by the `dev_tools` feature so
/// shipping/headless builds don't pay for `bevy-inspector-egui` /
/// `bevy_egui` in the dep graph. The inspector quick plugins require
/// EguiPlugin first; that's why both live behind the same gate.
#[cfg(feature = "dev_tools")]
pub(super) fn add_dev_tools_plugins(app: &mut App) {
    app.add_plugins(EguiPlugin::default())
        .add_plugins(
            ResourceInspectorPlugin::<DeveloperTools>::default()
                .run_if(dev_tools::inspector_visible),
        )
        .add_plugins(
            ResourceInspectorPlugin::<EditableAbilitySet>::default()
                .run_if(dev_tools::inspector_visible),
        )
        .add_plugins(
            ResourceInspectorPlugin::<EditableMovementTuning>::default()
                .run_if(dev_tools::inspector_visible),
        )
        .add_plugins(
            ResourceInspectorPlugin::<EditablePlayerStats>::default()
                .run_if(dev_tools::inspector_visible),
        )
        .add_plugins(
            ResourceInspectorPlugin::<SandboxFeelTuning>::default()
                .run_if(dev_tools::inspector_visible),
        )
        .add_plugins(WorldInspectorPlugin::new().run_if(dev_tools::world_inspector_visible));
}

#[cfg(not(feature = "dev_tools"))]
pub(super) fn add_dev_tools_plugins(_app: &mut App) {}

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
/// dialogue overlay (`dialog::sync_dialog_ui`) draws with Bevy's
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
        .init_resource::<crate::input::ActiveInputKind>()
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
            crate::input::update_active_input_kind.in_set(crate::input::InputSet::Populate),
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
                populate_control_frame_from_actions.in_set(crate::input::InputSet::Populate),
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

/// Register the [`TouchControlsPlugin`](crate::host::mobile_input::bevy_plugin::TouchControlsPlugin)
/// (`virtual_joystick` sticks + on-screen action buttons that fold into
/// ControlFrame). Added UNCONDITIONALLY whenever the `mobile_touch`
/// feature is compiled (which pulls the optional `virtual_joystick`
/// dep) — there is no runtime boolean gating its existence. To rip the
/// touch controls out, remove the single `add_plugins(TouchControlsPlugin)`
/// line below. On builds compiled without `mobile_touch` this is a no-op.
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
    app.add_plugins(crate::host::mobile_input::bevy_plugin::TouchControlsPlugin);
}

#[cfg(not(feature = "mobile_touch"))]
pub(super) fn add_mobile_touch_plugin(_app: &mut App) {}

/// Install the sandbox audio subsystem. Gated by `audio` so headless
/// / RL builds drop `bevy_kira_audio` from the dep graph entirely;
/// the sim still emits `SfxMessage`s and the queue drains harmlessly
/// per the ADR 0012 seam.
///
/// Moved to `crate::audio::SandboxAudioPlugin` per OVERNIGHT-TODO #6;
/// this helper is now a one-liner that installs that plugin.
#[cfg(feature = "audio")]
pub(super) fn add_audio_plugins(app: &mut App) {
    app.add_plugins(crate::audio::SandboxAudioPlugin);
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

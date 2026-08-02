//! The windowed-HOST face — [the windowed host] (decomposition E5 step 5):
//! [`PlatformerHostPlugins`], a Bevy [`PluginGroup`] that assembles the wiring
//! only a VISIBLE platformer host needs on top of
//! [`ambition_platformer2d_runtime::PlatformerEnginePlugins`]:
//!
//! - [`HostInputBindingsPlugin`] (feature `input`) — the leafwing input map +
//!   the device → `ControlFrame`/`MenuControlFrame` bridge;
//! - [`HostCameraPlugin`] — the camera follow/shake cluster consuming the
//!   sim's resolved camera observation, plus (feature `portal_render`) the
//!   portal camera-continuity wiring;
//! - [`gameplay_presentation::HostGameplayPresentationPlugin`] — where the
//!   gameplay camera renders on the physical display and where subjects should
//!   stay inside it, resolved once per frame from the active provider's
//!   declared profile.
//!
//! ## Why this crate
//!
//! A VISIBLE game (Ambition, or a demo) builds its host App by adding the
//! engine group + this host group + its own content crate:
//!
//! ```ignore
//! let mut app = App::new();
//! ambition_platformer2d_runtime::add_headless_foundation(&mut app); // or DefaultPlugins
//! app.add_plugins(ambition_platformer2d_runtime::PlatformerEnginePlugins::default())
//!    .add_plugins(ambition_platformer2d_host::PlatformerHostPlugins)   // <- this group
//!    .add_plugins(my_content::MyGameContentPlugin);
//! ```
//!
//! A headless / RL entry point adds only the engine group — the shared
//! per-frame SIM wiring (player input chain, brains, room transitions, portal
//! schedule) lives in `ambition_platformer2d_runtime`, NOT here, precisely because
//! headless runs it too (the E5 step-5 ruling; see decomposition.md).
//!
//! The host MAY dep `ambition_render` / `ambition_input` / `leafwing-input-
//! manager` / `ambition_platformer2d_runtime`; it must NEVER dep `ambition_platformer2d_actor_monolith` or
//! `ambition_content`
//! (enforced by `tests/host_names_no_content.rs`).

use bevy::app::{App, Plugin, PluginGroup, PluginGroupBuilder};
use bevy::prelude::*;

pub mod gameplay_presentation;
#[cfg(feature = "portal_render")]
pub mod portal;

// Only the input bridge + portal continuity order against the sandbox phases.
#[cfg(any(feature = "input", feature = "portal_render"))]
use ambition_platformer2d_shared_tangle::schedule::{
    Platformer2dSimulationPhaseMonolith, SimScheduleExt as _,
};

/// The windowed-host plugin group (see the crate docs).
pub struct PlatformerHostPlugins;

impl PluginGroup for PlatformerHostPlugins {
    fn build(self) -> PluginGroupBuilder {
        let builder = PluginGroupBuilder::start::<Self>()
            .add(ambition_platformer2d_shared_tangle::developer_hotkeys::DeveloperHotkeyPlugin)
            .add(HostCameraPlugin)
            .add(HostProjectileVisualsPlugin);
        #[cfg(feature = "input")]
        let builder = builder.add(HostInputBindingsPlugin);
        builder
    }
}

/// The leafwing-input-manager plugin, the player-input attach startup system,
/// and the bridge that keeps `Res<ControlFrame>` in sync with leafwing's
/// `ActionState`. Behind the `input` feature so sim-only hosts can drop
/// `leafwing-input-manager` from the dep graph; the sim itself reads
/// `Res<ControlFrame>` (always-available) and is agnostic to where the frame
/// came from.
#[cfg(feature = "input")]
pub struct HostInputBindingsPlugin;

#[cfg(feature = "input")]
impl Plugin for HostInputBindingsPlugin {
    fn build(&self, app: &mut App) {
        use ambition_input::{
            MenuControlFrame, MenuInputState, Platformer2dInputActionMonolith,
            PlayerDashTriggerState,
        };
        use ambition_platformer2d_runtime::host_input::{
            apply_menu_frame_to_cutscene_request, declare_gameplay_input_context,
            declare_in_session_input_contexts, dialog_pointer_input,
            freeze_local_seating_for_the_decided_match, populate_control_frame_from_actions,
            populate_menu_control_frame_from_actions, populate_seat_menu_frames,
            populate_secondary_slot_controls, publish_latched_slot_controls,
            seat_input_participants_for_roster, spawn_primary_input_participant,
            toggle_player_trail_emission_from_actions, MenuFrameCutsceneSkip, MenuFramePopulate,
        };
        use leafwing_input_manager::prelude::InputManagerPlugin;

        // ── The participant input pipeline (ordered, same-frame) ──────────
        //
        // Device adapters complete before routing; routed semantics complete
        // before shell/menu consumers. An edge produced this frame is
        // consumed this frame — the pipeline never tolerates "the edge may
        // arrive one frame later".
        app.configure_sets(
            Update,
            (
                ambition_input::InputSet::Collect,
                ambition_input::InputSet::ResolveActions,
                ambition_input::InputSet::ResolveContext,
                ambition_input::InputSet::Route,
                ambition_input::InputSet::PublishCues,
                ambition_input::InputSet::Consume,
            )
                .chain(),
        );
        // Device ownership, BEFORE leafwing resolves this frame's actions.
        //
        // In `PreUpdate` and pinned ahead of `InputManagerSystem::Update`
        // because the association is an input to that resolution: made after
        // it, a seat that joins reads its controller a frame late, and the
        // join press itself lands on nobody.
        app.init_resource::<ambition_input::LocalDeviceOrder>();
        // Which pad each seat is HOLDING, remembered across disconnects. Without
        // it `assign_local_seat_devices` panics on a missing resource; with it, a
        // seat keeps its controller when somebody else unplugs theirs.
        app.init_resource::<ambition_input::LocalSeatDeviceOwnership>();
        app.add_systems(
            PreUpdate,
            (
                ambition_input::track_local_device_order,
                ambition_input::assign_local_seat_devices,
            )
                .chain()
                .before(leafwing_input_manager::plugin::InputManagerSystem::Update),
        );
        app.init_resource::<ambition_input::SeatInputContexts>();
        app.init_resource::<ambition_input::SeatBindings>();
        app.init_resource::<ambition_input::ActiveUiCues>();

        // ── The frame→tick input latch (netcode N0.1) ─────────────────────
        //
        // A device samples on the FEEL clock, once per rendered frame. When the
        // sim runs on the TICK clock the two diverge, and every device sample
        // between two ticks has to reach the sim as ONE control frame: axes
        // take the latest, press/release edges OR together so a sub-tick tap is
        // never swallowed and a single tap never fires twice.
        //
        // This lives in the DEVICE plugin, not the engine group: headless, RL,
        // and replay drivers have no device and author the per-tick
        // `ControlFrame` themselves. Frame-stepped hosts skip it too — one
        // frame IS one tick, so there is nothing to bridge.
        // FIXED-TICK **OR** GGRS. Both advance the simulation on a cadence of
        // their own, so several rendered frames can pass between ticks and a
        // short press sampled in between must not be lost — which is the whole
        // job of these latches.
        //
        // `sim_is_fixed_tick()` asks whether the sim runs on `FixedUpdate`, and
        // under rollback it runs on `GgrsSchedule`, so the answer was NO and
        // NEITHER latch was installed. Both consumers take them as `Option` and
        // quietly fall back to live device state, so the visible rollback host
        // sampled every seat from whatever the pad happened to be doing at
        // replay time rather than from confirmed input (GPT 5.6, 2026-07-28).
        //
        // Asked through `SimulationHost` rather than by naming `GgrsSchedule`,
        // because this crate must not depend on `bevy_ggrs` — the schedule
        // owner is optional and the host's vocabulary stays independent of it.
        let rollback_host = app
            .world()
            .get_resource::<ambition_platformer2d_runtime::SimulationHost>()
            .is_some_and(|host| host.is_ggrs());
        // THE SEAT LATCHES, for a fixed-tick host AND a rollback one.
        //
        // `SlotControlLatches` needs no system of its own:
        // `populate_secondary_slot_controls` folds into it whenever the resource
        // exists and writes `SlotControls` straight through when it does not, so
        // installing the resource IS the switch. Under rollback,
        // `capture_latched_local_input` drains it on the `ReadInputs` edge —
        // which is where a rollback host asks for input — and publishes each
        // seat into the session.
        //
        // The PRIMARY latch is deliberately absent from this arm: under GGRS the
        // rollback observatory already owns `ControlFrameLatch` and its
        // accumulator, and adding a second registration here is a doubled
        // system rather than a fix. That is what the seats were missing and the
        // primary was not (GPT 5.6, 2026-07-28 — their reading was right and
        // mine was not).
        if app.sim_is_fixed_tick() || rollback_host {
            app.init_resource::<ambition_platformer2d_runtime::host_input::SlotControlLatches>();
        }
        if app.sim_is_fixed_tick() {
            app.init_resource::<ambition_platformer2d_core::ControlFrameLatch>();
            app.add_systems(
                Update,
                ambition_platformer2d_core::accumulate_control_frame_latch
                    .after(ambition_input::InputSet::Route),
            );
        }
        // THE PUBLISHING HALF IS FIXED-TICK ONLY, and that asymmetry is the
        // point: under rollback the SESSION publishes input, from the frame
        // GGRS confirmed, so a host system writing `ControlFrame` and
        // `SlotControls` from the latch would bypass the rollback stream
        // entirely and hand a resimulated tick whatever the pad said just now.
        // `capture_latched_local_input` drains the same latches on the
        // `ReadInputs` edge instead, which is where a rollback host asks.
        if app.sim_is_fixed_tick() {
            let sim = app.sim_schedule();
            app.add_systems(
                sim,
                ambition_platformer2d_core::publish_latched_control_frame
                    .in_set(Platformer2dSimulationPhaseMonolith::PlayerInput)
                    .before(ambition_input::InputSet::Route),
            );
            // The SECONDARY seats' half of the same bridge (queue Y2). A couch
            // match is two people on two pads, and giving only one of them
            // sub-tick forgiveness is a fairness asymmetry rather than a
            // rounding error.
            app.add_systems(
                sim,
                publish_latched_slot_controls
                    .in_set(Platformer2dSimulationPhaseMonolith::PlayerInput)
                    .before(ambition_input::InputSet::Route),
            );
        }

        // leafwing's `InputManagerPlugin` runs systems (e.g. `filter_captured_input`)
        // over Bevy's `ButtonInput<..>` resources, which `bevy::input::InputPlugin`
        // provides. A windowed host gets it from `DefaultPlugins`; a headless boot
        // (exit_3, RL, tests) uses `add_headless_foundation`, which has no
        // `InputPlugin` — and Bevy 0.18's strict system-param validation PANICS on the
        // missing `ButtonInput<MouseButton>` rather than skipping the system. Add it
        // here so the host input group is SELF-SUFFICIENT headless — the "boots from
        // the host groups alone" claim `exit_3` makes. Guarded, so it is a no-op when
        // `DefaultPlugins` already added it.
        if !app.is_plugin_added::<bevy::input::InputPlugin>() {
            app.add_plugins(bevy::input::InputPlugin);
        }
        // `update_active_input_kind` (added below) reads `MessageReader<CursorMoved>`,
        // a WINDOW message that `InputPlugin` does NOT register — a windowed host
        // gets it from `WindowPlugin`/`DefaultPlugins`, but a headless boot has no
        // window and Bevy 0.18 PANICS on the unregistered channel. Register it here
        // (idempotent) so the standard host-input path runs headlessly — the shape
        // RL and `tests/standard_input_path.rs` need.
        app.add_message::<bevy::window::CursorMoved>();

        app.init_resource::<MenuInputState>()
            .init_resource::<MenuControlFrame>()
            .init_resource::<ambition_input::SeatMenuFrames>()
            .init_resource::<PlayerDashTriggerState>()
            .init_resource::<ambition_input::ActiveInputKind>()
            .add_plugins(InputManagerPlugin::<Platformer2dInputActionMonolith>::default())
            .add_systems(
                bevy::app::PreUpdate,
                tune_clash_strategy_to_bindings
                    .before(leafwing_input_manager::plugin::InputManagerSystem::Update),
            )
            // Leafwing orders both its Tick set (which CLEARS the central
            // input store) and its Unify set (which recomputes it from
            // devices) before Update, but leaves Tick vs Unify UNORDERED — a
            // topology seed decides whether a device kind's freshly computed
            // values survive to the action update or are wiped first. Pin
            // the only correct order explicitly: clear first, then every
            // device kind publishes, then actions resolve.
            .configure_sets(
                bevy::app::PreUpdate,
                leafwing_input_manager::plugin::InputManagerSystem::Tick
                    .before(leafwing_input_manager::plugin::InputManagerSystem::Unify),
            )
            // Track which input source is CURRENTLY active (last to produce
            // GENUINE input). This gates the menu mouse-hover handlers so a
            // rebuild-induced `Pointer<Over>` under a stationary mouse can't
            // snap the cursor back while the player navigates with the
            // keyboard / gamepad / touch. Runs in the input populate set so
            // the value is fresh before this frame's menu consumers + before
            // the hover observers fire on rebuilt controls. The detector
            // covers keyboard / mouse / gamepad; the touch virtual-device /
            // gesture adapter marks `Touch` itself.
            .add_systems(
                Update,
                ambition_input::update_active_input_kind.in_set(ambition_input::InputSet::Route),
            )
            // The persistent participant spawns ONCE at boot — before any
            // route, session, or gameplay actor exists — and is never
            // session-scoped. Startup cards and the launcher read the same
            // participant a later gameplay session does; possession, session
            // relaunch, and actor death never touch its device state.
            .add_systems(Startup, spawn_primary_input_participant)
            // The menu crate cannot know an asset path and the render crate must
            // not own the menu IR, so the host is where the font handle crosses.
            .add_systems(Update, publish_menu_font)
            // Extra local seats come and go with the match roster, so unlike
            // the primary they are a per-frame reconciliation rather than a
            // boot-time spawn. `Collect` because a seat is a device source: it
            // has to exist before bindings resolve anything for it.
            .add_systems(
                Update,
                // The seating is frozen from the ROSTER before the seats it
                // describes materialize, so nothing downstream sees a frame where
                // participants exist and the topology does not.
                (
                    freeze_local_seating_for_the_decided_match,
                    seat_input_participants_for_roster,
                )
                    .chain()
                    .in_set(ambition_input::InputSet::Collect),
            )
            // Context ownership: surfaces declare claims during
            // `ResolveContext` (the session lifecycle here; the shell's
            // startup/launcher surfaces in `ambition_game_shell`), then the
            // one resolver reduces them before anything routes on the answer.
            .add_systems(
                Update,
                (
                    // ONE authority for "which physical control is this
                    // action on", projected from the live `InputMap` every
                    // frame so a rebind cannot leave a prompt behind.
                    ambition_input::publish_seat_bindings
                        .in_set(ambition_input::InputSet::ResolveActions),
                    declare_gameplay_input_context.in_set(ambition_input::InputSet::ResolveContext),
                    // The in-session surfaces (dialogue, cutscene) declare in the
                    // same set as the session itself, so one resolver sees every
                    // claim before any router reads the answer.
                    declare_in_session_input_contexts
                        .in_set(ambition_input::InputSet::ResolveContext),
                    ambition_input::resolve_active_input_context
                        .after(ambition_input::InputSet::ResolveContext)
                        .before(ambition_input::InputSet::Route),
                ),
            )
            // Collect semantic menu intent before gameplay input is
            // suppressed. `populate_control_frame_from_actions` may zero the
            // sim-side `ControlFrame` in UI modes, but it must not mutate
            // leafwing's `ActionState`; held keyboard/menu buttons should not
            // become `just_pressed` again on every dialog frame.
            //
            // Therefore the order is:
            // 1. read the participant's unified keyboard/gamepad/touch actions
            //    into `MenuControlFrame`,
            // 2. read/suppress gameplay into `ControlFrame`,
            // 3. let pointer gestures add scroll before consumers.
            .add_systems(
                Update,
                (
                    populate_menu_control_frame_from_actions
                        .in_set(ambition_input::InputSet::Route)
                        .in_set(MenuFramePopulate),
                    // The per-seat companion. Same phase, same inputs, different
                    // question: the global frame answers "did anybody press
                    // this", this one answers "which seat did".
                    populate_seat_menu_frames
                        .in_set(ambition_input::InputSet::Route)
                        .in_set(MenuFramePopulate),
                    populate_control_frame_from_actions.in_set(ambition_input::InputSet::Route),
                    // Every seat past the first writes its own slot directly.
                    // Same phase as the primary bridge: both are device→control
                    // translation, and neither reads the other's output.
                    populate_secondary_slot_controls.in_set(ambition_input::InputSet::Route),
                    toggle_player_trail_emission_from_actions,
                    apply_menu_frame_to_cutscene_request.in_set(MenuFrameCutsceneSkip),
                    dialog_pointer_input,
                )
                    .chain()
                    .before(Platformer2dSimulationPhaseMonolith::CoreSimulation),
            );
    }
}

/// Derive leafwing's [`ClashStrategy`] from the live bindings: chord-free maps
/// run `PressAll` (skipping the per-frame `possible_clash` pair scans — 1-2.6%
/// of CPU in desktop-lifecycle-4, where no chord existed to resolve), and the
/// moment any composed game authors a chorded binding the strategy flips back
/// to `PrioritizeLongest` the same frame. Per-game by construction: each
/// composition's actual `InputMap`s decide, no configuration to forget.
#[cfg(feature = "input")]
fn tune_clash_strategy_to_bindings(
    maps: Query<
        bevy::ecs::change_detection::Ref<
            leafwing_input_manager::prelude::InputMap<
                ambition_input::Platformer2dInputActionMonolith,
            >,
        >,
    >,
    mut strategy: ResMut<leafwing_input_manager::prelude::ClashStrategy>,
) {
    use leafwing_input_manager::clashing_inputs::BasicInputs;
    use leafwing_input_manager::prelude::ClashStrategy;
    if !maps.iter().any(|map| map.is_changed()) {
        return;
    }
    let any_chord = maps.iter().any(|map| {
        map.iter_buttonlike().any(|(_, inputs)| {
            inputs
                .iter()
                .any(|input| matches!(input.decompose(), BasicInputs::Chord(_)))
        })
    });
    let desired = if any_chord {
        ClashStrategy::PrioritizeLongest
    } else {
        ClashStrategy::PressAll
    };
    if *strategy != desired {
        *strategy = desired;
    }
}

/// **Projectiles that were fired are projectiles that are drawn.**
///
/// ⚠ **This lived in `game/ambition_app` until 2026-07-31**, which meant every
/// other composition simulated projectiles it never rendered: the ring of
/// bodies ticked, collided and expired with nothing on screen. Found by
/// `scripts/check_engine_systems_are_engine_installed.py`, built after the same
/// shape cost three defects in four days (the world-label pass, the parallax
/// theme load, the parallax layer sync).
///
/// It lives in the HOST rather than in `PlatformerPresentationPlugin` for a
/// layering reason, not a taste one: the ordering edges name
/// `ambition_platformer2d_runtime::projectile_schedule::step_projectiles` and
/// `ambition_sim_view::PresentedPoseSet`, and `ambition_render` depends on
/// neither runtime nor the schedule. This crate's own description says it MAY
/// name render, runtime and sim_view — that is what the host layer is for, and
/// `camera_follow` is already here for exactly the same reason.
pub struct HostProjectileVisualsPlugin;

impl Plugin for HostProjectileVisualsPlugin {
    fn build(&self, app: &mut App) {
        // The systems below draw from sheet metadata, and the registry that
        // holds it was ALSO app-local — moving the systems out without this
        // made them fail `Res<SheetRegistry>` validation in eight tests, which
        // is the class one layer down: not a system nobody installed, a
        // RESOURCE nobody installed. `SheetRegistryPlugin` is idempotent for
        // exactly this reason.
        app.add_plugins(ambition_sprite_sheet::SheetRegistryPlugin);
        app.add_systems(
            Update,
            (
                // One unified, kind-driven visual pass for ALL projectiles
                // (player + enemy); the charge indicator is its own player-only
                // pass. Both after the step so a projectile fired this frame is
                // visible this frame rather than one frame late.
                ambition_render::rendering::projectile_visuals::sync_projectile_visuals
                    .in_set(ambition_render::rendering::SpriteVisualSync)
                    .after(ambition_platformer2d_runtime::projectile_schedule::ProjectileStepSet),
                ambition_render::rendering::projectile_visuals::sync_projectile_charge_visuals
                    .after(ambition_platformer2d_runtime::projectile_schedule::ProjectileStepSet),
            )
                // Both passes hang art off the PRESENTED body pose (the charge
                // orb tracks the hand; a projectile's origin tracks the firer),
                // so the frame-clock resample must already have run. Without
                // this edge they read last frame's presented pose while the
                // camera is on this one, and the art shears away from the body
                // it belongs to by a frame of motion.
                .after(ambition_sim_view::PresentedPoseSet)
                .run_if(ambition_platformer2d_shared_tangle::lifecycle::session_world_exists),
        );
    }
}

/// The camera follow/shake cluster: publish the observer viewport into the
/// sim's camera-observation resolve, then apply the resolved snapshot
/// (`camera_follow`) after shake ticks. With `portal_render`, also the portal
/// camera-continuity wiring and the portal observation glue.
///
/// A host that needs to draw AFTER the camera lands (debug overlays, HUD
/// anchors) orders `.after(ambition_render::rendering::camera_follow)`.
pub struct HostCameraPlugin;

impl Plugin for HostCameraPlugin {
    fn build(&self, app: &mut App) {
        use ambition_render::rendering::camera_follow;

        // Render-owned camera view state, initialized with the presentation
        // half that reads it (nameplates, HUD, overlays) — the sim never
        // touches it.
        app.init_resource::<ambition_render::rendering::CameraViewState>();
        // The observer facts (gameplay viewport + subject-safe region) are
        // published by HostGameplayPresentationPlugin, which orders its whole
        // cluster before the sim's observation resolve. `camera_follow` only
        // APPLIES the resulting snapshot (E4-17).
        app.add_plugins(crate::gameplay_presentation::HostGameplayPresentationPlugin);
        // A viewport-clipped camera never clears outside itself, so a
        // fixed-aspect profile OWES the display a surround. Render owns the
        // painting; the host owns the fact that it is owed.
        app.add_plugins(ambition_render::gameplay_surround::GameplaySurroundPlugin);
        // The declared-HUD surface. A game gets a HUD by declaring slots on
        // its provider and publishing readouts; a route that declared none
        // spawns nothing, so this is inert for every game that has no HUD.
        app.add_plugins(ambition_render::hud::declared::DeclaredHudPlugin);
        app.add_systems(
            Update,
            (
                ambition_platformer2d_shared_tangle::camera_ease::tick_camera_shake,
                camera_follow,
            )
                .chain()
                .after(ambition_render::rendering::BossAnimation)
                // Read THIS frame's resolved snapshot, not last frame's. Keyed
                // on the observation SET: the resolve lives in `Update` for
                // every host, so this edge is real under fixed-tick and GGRS
                // too (naming the system worked only while the sim happened to
                // share `Update`).
                .after(ambition_sim_view::camera_snapshot::CameraObservationSet)
                .run_if(ambition_platformer2d_shared_tangle::lifecycle::session_world_exists),
        );

        // The Ambition portal host-adapter observation glue (world-frame /
        // viewer / focus / debug seam publishers, scene-body tagging, dev
        // toggles, gun art) — sim-owned plugin, host-added (E4 slice 20).
        #[cfg(feature = "portal_render")]
        {
            app.add_plugins(crate::portal::PortalObservationPlugin);
            app.add_systems(
                Update,
                (
                    crate::portal::apply_portal_camera_continuity
                        .after(Platformer2dSimulationPhaseMonolith::CoreSimulation)
                        .after(crate::portal::sync_portal_camera_continuity_focus)
                        .before(camera_follow),
                    // Same-frame pad into the sim resolve (E4-17): after the
                    // continuity update, before the observation resolves.
                    ambition_render::rendering::publish_portal_camera_clamp
                        .after(crate::portal::apply_portal_camera_continuity)
                        .before(ambition_sim_view::camera_snapshot::CameraObservationSet),
                )
                    .run_if(ambition_platformer2d_shared_tangle::lifecycle::session_world_exists),
            );
            // Hosts drawing camera-anchored debug visuals order themselves
            // `.after(this system)` (the Ambition debug overlay does).
            app.add_systems(
                Update,
                crate::portal::tag_portal_camera_continuity_camera
                    .in_set(crate::portal::PortalContinuityCameraTagged)
                    .after(camera_follow),
            );
        }
    }
}

#[cfg(all(test, feature = "input"))]
mod clash_strategy_tests {
    use super::tune_clash_strategy_to_bindings;
    use ambition_input::Platformer2dInputActionMonolith;
    use bevy::prelude::*;
    use leafwing_input_manager::prelude::{ButtonlikeChord, ClashStrategy, InputMap};

    fn app_with_map(map: InputMap<Platformer2dInputActionMonolith>) -> App {
        let mut app = App::new();
        app.init_resource::<ClashStrategy>()
            .add_systems(Update, tune_clash_strategy_to_bindings);
        app.world_mut().spawn(map);
        app
    }

    #[test]
    fn chord_free_bindings_relax_to_press_all() {
        let map = InputMap::new([(Platformer2dInputActionMonolith::Jump, KeyCode::Space)]);
        let mut app = app_with_map(map);
        app.update();
        assert_eq!(
            *app.world().resource::<ClashStrategy>(),
            ClashStrategy::PressAll,
        );
    }

    #[test]
    fn authoring_a_chord_reenables_clash_resolution_same_frame() {
        let map = InputMap::new([(Platformer2dInputActionMonolith::Jump, KeyCode::Space)]);
        let mut app = app_with_map(map);
        app.update();
        assert_eq!(
            *app.world().resource::<ClashStrategy>(),
            ClashStrategy::PressAll,
        );
        let entity = app
            .world_mut()
            .query_filtered::<Entity, With<InputMap<Platformer2dInputActionMonolith>>>()
            .single(app.world())
            .unwrap();
        app.world_mut()
            .get_mut::<InputMap<Platformer2dInputActionMonolith>>(entity)
            .unwrap()
            .insert(
                Platformer2dInputActionMonolith::Interact,
                ButtonlikeChord::new([KeyCode::ControlLeft, KeyCode::KeyE]),
            );
        app.update();
        assert_eq!(
            *app.world().resource::<ClashStrategy>(),
            ClashStrategy::PrioritizeLongest,
        );
    }
}

/// **Hand the menu crate the font the render side resolved.**
///
/// ⛔ `ambition_menu` set a font SIZE and no handle, so Bevy resolved
/// `Handle::<Font>::default()` — its built-in `FiraMono-subset.ttf`. Every menu
/// in every game drew a hollow box for `·` (U+00B7) or `—` (U+2014), invisibly,
/// until a launcher footer was photographed. Ten hypotheses died on that bug:
/// they checked the fonts the REPOSITORY ships — `JetBrainsMono-Regular.ttf` and
/// `InterDisplay-Regular.otf`, both of which carry the glyphs — and none of them
/// asked what `Handle::default()` points at. Forcing the default handle back
/// reproduces the box; see `MenuFont`, which also records what about this is
/// still UNEXPLAINED (the same handle renders those glyphs elsewhere).
///
/// ⚠ **this is the composition root because it is the only place the two crates
/// can meet.** `ambition_render` must not depend on `ambition_menu`
/// (presentation does not own the menu IR) and `ambition_menu` must not know an
/// asset path (it is renderer-agnostic). A host is exactly the thing that knows
/// both.
///
/// Idempotent and cheap: it writes only when the resolved handle changes, which
/// is once.
///
/// Gated with its only caller (`HostInputBindingsPlugin`, which is
/// `feature = "input"`); without this a feature-stripped build warns
/// `never used`, and CI compiles with `-D warnings` across configs.
#[cfg(feature = "input")]
fn publish_menu_font(
    mut commands: bevy::prelude::Commands,
    fonts: Option<bevy::prelude::Res<ambition_render::ui_fonts::UiFonts>>,
    current: Option<bevy::prelude::Res<ambition_menu::render::bevy_ui::MenuFont>>,
) {
    let Some(fonts) = fonts else { return };
    let wanted = fonts.regular.clone();
    if current.map(|current| current.0.clone()) == Some(wanted.clone()) {
        return;
    }
    commands.insert_resource(ambition_menu::render::bevy_ui::MenuFont(wanted));
}

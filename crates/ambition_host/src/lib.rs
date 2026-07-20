//! The windowed-HOST face — [the windowed host] (decomposition E5 step 5):
//! [`PlatformerHostPlugins`], a Bevy [`PluginGroup`] that assembles the wiring
//! only a VISIBLE platformer host needs on top of
//! [`ambition_runtime::PlatformerEnginePlugins`]:
//!
//! - [`HostInputBindingsPlugin`] (feature `input`) — the leafwing input map +
//!   the device → `ControlFrame`/`MenuControlFrame` bridge;
//! - [`HostCameraPlugin`] — the camera follow/shake cluster consuming the
//!   sim's resolved camera observation, plus (feature `portal_render`) the
//!   portal camera-continuity wiring.
//!
//! ## Why this crate
//!
//! A VISIBLE game (Ambition, or a demo) builds its host App by adding the
//! engine group + this host group + its own content crate:
//!
//! ```ignore
//! let mut app = App::new();
//! ambition_runtime::add_headless_foundation(&mut app); // or DefaultPlugins
//! app.add_plugins(ambition_runtime::PlatformerEnginePlugins::default())
//!    .add_plugins(ambition_host::PlatformerHostPlugins)   // <- this group
//!    .add_plugins(my_content::MyGameContentPlugin);
//! ```
//!
//! A headless / RL entry point adds only the engine group — the shared
//! per-frame SIM wiring (player input chain, brains, room transitions, portal
//! schedule) lives in `ambition_runtime`, NOT here, precisely because
//! headless runs it too (the E5 step-5 ruling; see decomposition.md).
//!
//! The host MAY dep `ambition_render` / `ambition_input` / `leafwing-input-
//! manager` / `ambition_runtime`; it must NEVER dep `ambition_actors` or
//! `ambition_content`
//! (enforced by `tests/host_names_no_content.rs`).

use bevy::app::{App, Plugin, PluginGroup, PluginGroupBuilder};
use bevy::prelude::*;

#[cfg(feature = "portal_render")]
pub mod portal;

// Only the input bridge + portal continuity order against the sandbox phases.
#[cfg(any(feature = "input", feature = "portal_render"))]
use ambition_platformer_primitives::schedule::{SandboxSet, SimScheduleExt as _};

/// The windowed-host plugin group (see the crate docs).
pub struct PlatformerHostPlugins;

impl PluginGroup for PlatformerHostPlugins {
    fn build(self) -> PluginGroupBuilder {
        let builder = PluginGroupBuilder::start::<Self>()
            .add(ambition_platformer_primitives::developer_hotkeys::DeveloperHotkeyPlugin)
            .add(HostCameraPlugin);
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
            MenuControlFrame, MenuInputState, PlayerDashTriggerState, SandboxAction,
        };
        use ambition_runtime::host_input::{
            apply_menu_frame_to_cutscene_request, declare_gameplay_input_context,
            dialog_pointer_input, populate_control_frame_from_actions,
            populate_menu_control_frame_from_actions, spawn_primary_input_participant,
            toggle_player_trail_emission_from_actions,
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
        app.init_resource::<ambition_input::ActiveInputContext>();
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
        if app.sim_is_fixed_tick() {
            let sim = app.sim_schedule();
            app.init_resource::<ambition_engine_core::ControlFrameLatch>();
            app.add_systems(
                Update,
                ambition_engine_core::accumulate_control_frame_latch
                    .after(ambition_input::InputSet::Route),
            );
            app.add_systems(
                sim,
                ambition_engine_core::publish_latched_control_frame
                    .in_set(SandboxSet::PlayerInput)
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
            .init_resource::<PlayerDashTriggerState>()
            .init_resource::<ambition_input::ActiveInputKind>()
            .add_plugins(InputManagerPlugin::<SandboxAction>::default())
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
            // Context ownership: surfaces declare claims during
            // `ResolveContext` (the session lifecycle here; the shell's
            // startup/launcher surfaces in `ambition_game_shell`), then the
            // one resolver reduces them before anything routes on the answer.
            .add_systems(
                Update,
                (
                    declare_gameplay_input_context.in_set(ambition_input::InputSet::ResolveContext),
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
                        .in_set(ambition_input::InputSet::Route),
                    populate_control_frame_from_actions.in_set(ambition_input::InputSet::Route),
                    toggle_player_trail_emission_from_actions,
                    apply_menu_frame_to_cutscene_request,
                    dialog_pointer_input,
                )
                    .chain()
                    .before(SandboxSet::CoreSimulation),
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
        // The observer fact: publish THIS frame's physical viewport before
        // the sim's observation resolve consumes it (E4-17 — the resolve
        // lives in CameraObservationPlugin; camera_follow only APPLIES the
        // snapshot).
        app.add_systems(
            Update,
            ambition_render::rendering::publish_camera_viewport
                .before(ambition_sim_view::camera_snapshot::resolve_camera_observation),
        );
        app.add_systems(
            Update,
            (
                ambition_platformer_primitives::camera_ease::tick_camera_shake,
                camera_follow,
            )
                .chain()
                .after(ambition_render::rendering::animate_bosses)
                // Read THIS tick's resolved snapshot, not last frame's.
                .after(ambition_sim_view::camera_snapshot::resolve_camera_observation)
                .run_if(ambition_platformer_primitives::lifecycle::session_world_exists),
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
                        .after(SandboxSet::CoreSimulation)
                        .after(crate::portal::sync_portal_camera_continuity_focus)
                        .before(camera_follow),
                    // Same-frame pad into the sim resolve (E4-17): after the
                    // continuity update, before the observation resolves.
                    ambition_render::rendering::publish_portal_camera_clamp
                        .after(crate::portal::apply_portal_camera_continuity)
                        .before(ambition_sim_view::camera_snapshot::resolve_camera_observation),
                )
                    .run_if(ambition_platformer_primitives::lifecycle::session_world_exists),
            );
            // Hosts drawing camera-anchored debug visuals order themselves
            // `.after(this system)` (the Ambition debug overlay does).
            app.add_systems(
                Update,
                crate::portal::tag_portal_camera_continuity_camera.after(camera_follow),
            );
        }
    }
}

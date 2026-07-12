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
        let builder = PluginGroupBuilder::start::<Self>().add(HostCameraPlugin);
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
            apply_menu_frame_to_cutscene_request, attach_player_input_components,
            dialog_pointer_input, populate_control_frame_from_actions,
            populate_menu_control_frame_from_actions, toggle_player_trail_emission_from_actions,
            SimulationSetupSet,
        };
        use leafwing_input_manager::prelude::InputManagerPlugin;

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
                    .after(ambition_input::InputSet::Populate),
            );
            app.add_systems(
                sim,
                ambition_engine_core::publish_latched_control_frame
                    .in_set(SandboxSet::PlayerInput)
                    .before(ambition_input::InputSet::Populate),
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
            // Preserve the zero-lag path for historical Startup-built worlds.
            // The attachment system no longer requires `SceneEntities`, so it is
            // also safe when a shell provider has not activated a route yet.
            .add_systems(
                Startup,
                attach_player_input_components.after(SimulationSetupSet),
            )
            // Shell providers spawn a fresh player during Update, including on
            // relaunch after Startup is permanently over. Re-run the idempotent
            // `Without<ActionState<_>>` attachment before the frame's consumers;
            // deferred insertion becomes visible on the following input frame.
            .add_systems(
                Update,
                attach_player_input_components
                    .before(populate_menu_control_frame_from_actions),
            )
            // Collect semantic menu intent before gameplay input is
            // suppressed. `populate_control_frame_from_actions` may zero the
            // sim-side `ControlFrame` in UI modes, but it must not mutate
            // leafwing's `ActionState`; held keyboard/menu buttons should not
            // become `just_pressed` again on every dialog frame.
            //
            // Therefore the order is:
            // 1. read keyboard/gamepad menu actions into `MenuControlFrame`,
            // 2. read/suppress gameplay into `ControlFrame`,
            // 3. let touch folds merge into both seams before the consumers.
            .add_systems(
                Update,
                (
                    populate_menu_control_frame_from_actions,
                    populate_control_frame_from_actions.in_set(ambition_input::InputSet::Populate),
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
                .after(ambition_sim_view::camera_snapshot::resolve_camera_observation),
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
                ),
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

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
//! app.add_plugins(ambition_runtime::PlatformerEnginePlugins)
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
//! manager` / `ambition_gameplay_core`; it must NEVER dep `ambition_content`
//! (enforced by `tests/host_names_no_content.rs`).

use bevy::app::{App, Plugin, PluginGroup, PluginGroupBuilder};
use bevy::prelude::*;

// Only the input bridge + portal continuity order against the sandbox phases.
#[cfg(any(feature = "input", feature = "portal_render"))]
use ambition_gameplay_core::schedule::SandboxSet;

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
        use ambition_gameplay_core::dialog;
        use ambition_gameplay_core::schedule::{
            apply_menu_frame_to_cutscene_request, attach_player_input_components,
            populate_control_frame_from_actions, populate_menu_control_frame_from_actions,
            toggle_player_trail_emission_from_actions, SimulationSetupSet,
        };
        use ambition_input::{
            MenuControlFrame, MenuInputState, PlayerDashTriggerState, SandboxAction,
        };
        use leafwing_input_manager::prelude::InputManagerPlugin;

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
            // Attach the input components once the host's simulation setup
            // has spawned the player (the `SimulationSetupSet` label is the
            // machinery-facing name for that startup slot).
            .add_systems(
                Startup,
                attach_player_input_components.after(SimulationSetupSet),
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
                    dialog::dialog_pointer_input,
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
                ambition_gameplay_core::time::camera_ease::tick_camera_shake,
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
            app.add_plugins(ambition_gameplay_core::portal::PortalObservationPlugin);
            app.add_systems(
                Update,
                (
                    ambition_gameplay_core::portal::apply_portal_camera_continuity
                        .after(SandboxSet::CoreSimulation)
                        .after(ambition_gameplay_core::portal::sync_portal_camera_continuity_focus)
                        .before(camera_follow),
                    // Same-frame pad into the sim resolve (E4-17): after the
                    // continuity update, before the observation resolves.
                    ambition_render::rendering::publish_portal_camera_clamp
                        .after(ambition_gameplay_core::portal::apply_portal_camera_continuity)
                        .before(ambition_sim_view::camera_snapshot::resolve_camera_observation),
                ),
            );
            // Hosts drawing camera-anchored debug visuals order themselves
            // `.after(this system)` (the Ambition debug overlay does).
            app.add_systems(
                Update,
                ambition_gameplay_core::portal::tag_portal_camera_continuity_camera
                    .after(camera_follow),
            );
        }
    }
}

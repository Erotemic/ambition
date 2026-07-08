//! App-level dev presentation: the F1 debug overlay and the F3 FPS
//! counter. These are pure presentation/host systems with no lib
//! consumer, moved up from `ambition_actors::dev` (Stage 20 devtools
//! split). The lib keeps the dev STATE (`DeveloperTools` + editable
//! profiles, read by persistence/presentation), the gameplay `trace`
//! recorder (written by sim code), and `profiling` (read by audio).
pub mod debug_overlay;
pub mod fps_overlay;
pub mod portal_inspector;

use bevy::prelude::*;

/// The game's developer tooling, as one plugin (components-as-plugins):
/// the F1 debug overlay + F3 FPS counter, plus (behind the `dev_tools`
/// feature) the egui resource/world inspectors. The dev STATE it drives
/// (`DeveloperTools`, the editable profiles) lives in the machinery lib
/// (`ambition_dev_tools::dev_tools`); this plugin only wires the
/// app-side presentation/inspection of it.
pub struct DevToolsPlugin;

impl Plugin for DevToolsPlugin {
    fn build(&self, app: &mut App) {
        // FPS overlay (ON by default on wasm, OFF on desktop; F3 toggles).
        app.add_plugins(fps_overlay::FpsOverlayPlugin);
        install_egui_inspectors(app);
    }
}

/// Install the egui inspector plugins. Gated by `dev_tools` so
/// shipping/headless builds don't pull `bevy-inspector-egui` /
/// `bevy_egui` into the dep graph; the quick plugins require
/// `EguiPlugin` first, hence the shared gate.
#[cfg(feature = "dev_tools")]
fn install_egui_inspectors(app: &mut App) {
    use ambition_dev_tools::dev_tools::{
        inspector_visible, world_inspector_visible, DeveloperTools, EditableAbilitySet,
        EditableMovementTuning, EditablePlayerStats,
    };
    use ambition_actors::time::feel::SandboxFeelTuning;
    use bevy_inspector_egui::bevy_egui::EguiPlugin;
    use bevy_inspector_egui::quick::{ResourceInspectorPlugin, WorldInspectorPlugin};

    app.add_plugins(EguiPlugin::default())
        .add_plugins(ResourceInspectorPlugin::<DeveloperTools>::default().run_if(inspector_visible))
        .add_plugins(
            ResourceInspectorPlugin::<EditableAbilitySet>::default().run_if(inspector_visible),
        )
        .add_plugins(
            ResourceInspectorPlugin::<EditableMovementTuning>::default().run_if(inspector_visible),
        )
        .add_plugins(
            ResourceInspectorPlugin::<EditablePlayerStats>::default().run_if(inspector_visible),
        )
        .add_plugins(
            ResourceInspectorPlugin::<SandboxFeelTuning>::default().run_if(inspector_visible),
        )
        .add_plugins(portal_inspector::PortalInspectorPlugin);

    app.add_plugins(WorldInspectorPlugin::new().run_if(world_inspector_visible));
}

#[cfg(not(feature = "dev_tools"))]
fn install_egui_inspectors(_app: &mut App) {}

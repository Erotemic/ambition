//! App-level developer presentation: the F1 debug overlay, F3 FPS counter,
//! and the F9 one-shot GGRS rollback proof. These are host/presentation systems with
//! no simulation-state ownership. The observatory's control resource is
//! platform-neutral so desktop keys and future Android developer UI can share
//! one proof-request seam.
pub mod debug_overlay;
pub mod fps_overlay;
pub mod portal_inspector;
#[cfg(feature = "dev_tools")]
pub mod rollback_observatory;

use bevy::prelude::*;

/// The game's developer tooling, as one plugin (components-as-plugins):
/// the F1 debug overlay + F3 FPS counter, plus (behind the `dev_tools`
/// feature) the egui resource/world inspectors. The dev STATE it drives
/// (`DeveloperTools`, the editable profiles) lives in the machinery lib
/// (`ambition::dev_tools::dev_tools`); this plugin only wires the
/// app-side presentation/inspection of it.
pub struct DevToolsPlugin;

impl Plugin for DevToolsPlugin {
    fn build(&self, app: &mut App) {
        // FPS overlay (ON by default on wasm, OFF on desktop; F3 toggles).
        app.add_plugins(fps_overlay::FpsOverlayPlugin);
        #[cfg(feature = "dev_tools")]
        app.add_plugins(rollback_observatory::RollbackObservatoryPlugin);
        install_egui_inspectors(app);
    }
}

/// Install the egui inspector plugins. Gated by `dev_tools` so
/// shipping/headless builds don't pull `bevy-inspector-egui` /
/// `bevy_egui` into the dep graph; the quick plugins require
/// `EguiPlugin` first, hence the shared gate.
#[cfg(feature = "dev_tools")]
fn install_egui_inspectors(app: &mut App) {
    use ambition::actors::time::feel::SandboxFeelTuning;
    use ambition::dev_tools::dev_tools::{
        inspector_visible, world_inspector_visible, DeveloperTools, EditableAbilitySet,
        EditableMovementTuning, EditablePlayerStats,
    };
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

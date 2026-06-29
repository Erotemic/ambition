//! Custom F3 portal inspector: one app-side window that groups the portal
//! resources without changing their ownership or defaults.

#[cfg(all(feature = "dev_tools", feature = "portal"))]
mod enabled {
    use ambition_gameplay_core::dev::dev_tools::inspector_visible;
    use bevy::prelude::*;
    use bevy_inspector_egui::bevy_egui::{
        egui, EguiContext, EguiPrimaryContextPass, PrimaryEguiContext,
    };
    use bevy_inspector_egui::{bevy_inspector, DefaultInspectorConfigPlugin};

    const PORTAL_INSPECTOR_WIDTH: f32 = 680.0;
    const PORTAL_INSPECTOR_HEIGHT: f32 = 620.0;

    pub struct PortalInspectorPlugin;

    impl Plugin for PortalInspectorPlugin {
        fn build(&self, app: &mut App) {
            if !app.is_plugin_added::<DefaultInspectorConfigPlugin>() {
                app.add_plugins(DefaultInspectorConfigPlugin);
            }
            app.add_systems(
                EguiPrimaryContextPass,
                portal_inspector_ui.run_if(inspector_visible),
            );
        }
    }

    fn portal_inspector_ui(world: &mut World) {
        let Ok(egui_context) = world
            .query_filtered::<&mut EguiContext, With<PrimaryEguiContext>>()
            .single(world)
        else {
            return;
        };
        let mut egui_context = egui_context.clone();

        egui::Window::new("Portal")
            .default_width(PORTAL_INSPECTOR_WIDTH)
            .default_height(PORTAL_INSPECTOR_HEIGHT)
            .min_width(PORTAL_INSPECTOR_WIDTH)
            .show(egui_context.get_mut(), |ui| {
                egui::ScrollArea::both().show(ui, |ui| {
                    ui.set_min_width(PORTAL_INSPECTOR_WIDTH - 40.0);
                    mechanics_section(world, ui);
                    effects_section(world, ui);
                    camera_continuity_section(world, ui);
                    view_cones_section(world, ui);

                    ui.allocate_space(ui.available_size());
                });
            });
    }

    fn mechanics_section(world: &mut World, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("Mechanics")
            .default_open(true)
            .show(ui, |ui| {
                bevy_inspector::ui_for_resource::<ambition_gameplay_core::portal::PortalTuning>(
                    world, ui,
                );
            });
    }

    #[cfg(feature = "portal_render")]
    fn effects_section(world: &mut World, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("Effects")
            .default_open(true)
            .show(ui, |ui| {
                bevy_inspector::ui_for_resource::<
                    ambition_gameplay_core::portal::PortalEffectSelection,
                >(world, ui);
            });
    }

    #[cfg(not(feature = "portal_render"))]
    fn effects_section(_world: &mut World, _ui: &mut egui::Ui) {}

    #[cfg(feature = "portal_render")]
    fn camera_continuity_section(world: &mut World, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("Camera Continuity")
            .default_open(true)
            .show(ui, |ui| {
                bevy_inspector::ui_for_resource::<
                    ambition_gameplay_core::portal::PortalCameraContinuitySelection,
                >(world, ui);
                ui.separator();
                bevy_inspector::ui_for_resource::<
                    ambition_gameplay_core::portal::PortalCameraContinuityConfig,
                >(world, ui);
            });
    }

    #[cfg(not(feature = "portal_render"))]
    fn camera_continuity_section(_world: &mut World, _ui: &mut egui::Ui) {}

    #[cfg(feature = "portal_render")]
    fn view_cones_section(world: &mut World, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("View Cones")
            .default_open(true)
            .show(ui, |ui| {
                bevy_inspector::ui_for_resource::<
                    ambition_gameplay_core::portal::PortalViewConeConfig,
                >(world, ui);
            });
    }

    #[cfg(not(feature = "portal_render"))]
    fn view_cones_section(_world: &mut World, _ui: &mut egui::Ui) {}
}

#[cfg(not(all(feature = "dev_tools", feature = "portal")))]
mod disabled {
    use bevy::prelude::*;

    pub struct PortalInspectorPlugin;

    impl Plugin for PortalInspectorPlugin {
        fn build(&self, _app: &mut App) {}
    }
}

#[cfg(not(all(feature = "dev_tools", feature = "portal")))]
pub use disabled::PortalInspectorPlugin;
#[cfg(all(feature = "dev_tools", feature = "portal"))]
pub use enabled::PortalInspectorPlugin;

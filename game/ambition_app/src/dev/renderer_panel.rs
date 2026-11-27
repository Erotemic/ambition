use ambition_platformer2d::dev_tools::dev_tools::inspector_visible;
#[cfg(feature = "card_view")]
use ambition_platformer2d::render::rendering::card_view::{CardViewMode, CardViewSettings};
use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui::{
    EguiContext, EguiPrimaryContextPass, PrimaryEguiContext, egui,
};

pub(super) fn install(app: &mut App) {
    app.add_systems(
        EguiPrimaryContextPass,
        renderer_panel.run_if(inspector_visible),
    );
}

fn renderer_panel(
    mut contexts: Query<&mut EguiContext, With<PrimaryEguiContext>>,
    #[cfg(feature = "card_view")] settings: Option<ResMut<CardViewSettings>>,
) {
    let Ok(mut context) = contexts.single_mut() else {
        return;
    };

    egui::Window::new("Renderer")
        .anchor(egui::Align2::RIGHT_TOP, [-12.0, 12.0])
        .resizable(false)
        .show(context.get_mut(), |ui| {
            #[cfg(feature = "card_view")]
            if let Some(mut settings) = settings {
                let mode = select_mode(ui, settings.mode);
                if mode != settings.mode {
                    settings.mode = mode;
                    info!("Gameplay renderer: {:?}", mode);
                }
                return;
            }

            ui.label("2D");
            #[cfg(not(feature = "card_view"))]
            ui.horizontal(|ui| {
                ui.label("For 2.5D, run");
                ui.code("./run_game.sh --card-view");
            });
        });
}

#[cfg(feature = "card_view")]
fn select_mode(ui: &mut egui::Ui, mut mode: CardViewMode) -> CardViewMode {
    use CardViewMode::{Gameplay, Inset, Plane, SideOn};

    egui::ComboBox::from_id_salt("renderer_mode")
        .selected_text(mode_label(mode))
        .show_ui(ui, |ui| {
            for choice in [Plane, SideOn, Gameplay, Inset] {
                ui.selectable_value(&mut mode, choice, mode_label(choice));
            }
        });
    mode
}

#[cfg(feature = "card_view")]
fn mode_label(mode: CardViewMode) -> &'static str {
    match mode {
        CardViewMode::Plane => "2D",
        CardViewMode::SideOn => "Side-on 2.5D",
        CardViewMode::Gameplay => "Oblique 2.5D",
        CardViewMode::Inset => "Comparison inset",
    }
}

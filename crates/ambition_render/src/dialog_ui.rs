//! The dialog-box overlay UI: spawns/refreshes the on-screen dialog panel.
//!
//! Render-only. [`sync_dialog_ui`] mirrors `ambition_actors::dialog::DialogState`
//! into a Bevy UI tree under [`DialogOverlayRoot`]; the per-choice
//! `DialogChoiceSlot` marker (owned sim-side) bridges to the dialog pointer-input
//! system. Fonts come from [`crate::ui_fonts`].

use bevy::log::info;
use bevy::prelude::*;

use crate::ui_fonts::{UiFontWeight, UiFonts};
// The choice-row marker bridges this UI (which spawns it) and the sim-side
// dialog pointer-input system (which reads it), so it lives in the sandbox
// dialog module, not here.
use ambition_actors::dialog::DialogChoiceSlot;

const DIALOG_CONTINUE_HINT: &str =
    "Tap an option, press Confirm / Jump / Interact, or drag / use Up-Down. Back closes.";

#[derive(Component)]
pub struct DialogOverlayRoot;

pub fn sync_dialog_ui(
    mut commands: Commands,
    dialogue: Res<ambition_actors::dialog::DialogState>,
    overlays: Query<Entity, With<DialogOverlayRoot>>,
    ui_fonts: Option<Res<UiFonts>>,
    mut logged_font_state: Local<bool>,
) {
    for entity in overlays.iter() {
        commands.entity(entity).despawn();
    }
    if !dialogue.active() {
        return;
    }

    let title = dialogue.title();
    let body = dialogue.body();
    let options = dialogue.options();
    let selected = dialogue.selected_option();

    let selected_marker = ui_fonts
        .as_deref()
        .map(UiFonts::selected_marker)
        .unwrap_or(">");

    if !*logged_font_state {
        let marker_codepoints = selected_marker
            .chars()
            .map(|ch| format!("U+{:04X}", ch as u32))
            .collect::<Vec<_>>()
            .join(" ");

        let font_state = ui_fonts
            .as_deref()
            .map(|fonts| {
                format!(
                    "regular={}, semibold={}",
                    fonts.regular.is_some(),
                    fonts.semibold.is_some()
                )
            })
            .unwrap_or_else(|| "UiFonts resource missing".to_string());

        info!(
            "dialog UI font state: {font_state}; selected_marker='{selected_marker}' ({marker_codepoints})"
        );

        *logged_font_state = true;
    }

    let dialog_font = |font_size: f32, weight: UiFontWeight| {
        ui_fonts
            .as_deref()
            .map(|fonts| fonts.text_font(font_size, weight))
            .unwrap_or(TextFont {
                font_size,
                ..default()
            })
    };

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                top: Val::Percent(15.0),
                bottom: Val::Px(0.0),
                padding: UiRect::axes(Val::Px(18.0), Val::Px(18.0)),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::Center,
                ..default()
            },
            ZIndex(45),
            Name::new("Dialogue Overlay Root"),
            DialogOverlayRoot,
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    width: Val::Percent(72.0),
                    min_width: Val::Px(120.0),
                    max_width: Val::Px(960.0),
                    max_height: Val::Percent(94.0),
                    padding: UiRect::all(Val::Px(14.0)),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(5.0),
                    border: UiRect::all(Val::Px(2.0)),
                    border_radius: BorderRadius::all(Val::Px(20.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.025, 0.030, 0.045, 0.95)),
                BorderColor::all(Color::srgba(0.42, 0.78, 1.00, 0.86)),
                Name::new("Dialogue Overlay Panel"),
            ))
            .with_children(|parent| {
                parent.spawn((
                    Text::new(title),
                    dialog_font(24.0, UiFontWeight::Semibold),
                    TextColor(Color::srgba(0.82, 0.94, 1.00, 1.0)),
                ));
                parent.spawn((
                    Text::new(body),
                    dialog_font(20.0, UiFontWeight::Regular),
                    TextColor(Color::srgba(0.93, 0.96, 1.00, 1.0)),
                ));
                if !options.is_empty() {
                    for (idx, option) in options.iter().enumerate() {
                        spawn_dialog_choice_row(
                            parent,
                            idx,
                            &option.label,
                            idx == selected,
                            selected_marker,
                            &dialog_font,
                        );
                    }
                }
                // No options → the body is either accumulating
                // (auto-advance pending) or the runner finished and
                // is waiting for the player to acknowledge the
                // final line. Either way, no explicit "Continue"
                // button — the hint text below tells the player
                // they can press Confirm to advance / close.
                parent.spawn((
                    Text::new(DIALOG_CONTINUE_HINT),
                    dialog_font(14.0, UiFontWeight::Regular),
                    TextColor(Color::srgba(0.66, 0.76, 0.88, 0.96)),
                ));
            });
        });
}

fn spawn_dialog_choice_row(
    parent: &mut ChildSpawnerCommands,
    index: usize,
    label: &str,
    selected: bool,
    selected_marker: &str,
    dialog_font: &impl Fn(f32, UiFontWeight) -> TextFont,
) {
    let bg = if selected {
        Color::srgba(0.95, 0.78, 0.32, 0.96)
    } else {
        Color::srgba(0.10, 0.13, 0.20, 0.74)
    };
    let fg = if selected {
        Color::srgba(0.18, 0.06, 0.04, 1.0)
    } else {
        Color::srgba(0.82, 0.90, 1.0, 0.98)
    };
    let marker = if selected { selected_marker } else { " " };
    parent
        .spawn((
            Button,
            Node {
                width: Val::Percent(100.0),
                min_height: Val::Px(46.0),
                padding: UiRect::axes(Val::Px(16.0), Val::Px(8.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::FlexStart,
                border_radius: BorderRadius::all(Val::Px(12.0)),
                ..default()
            },
            BackgroundColor(bg),
            DialogChoiceSlot { index },
            Name::new(format!("Dialogue choice {index}: {label}")),
        ))
        .with_children(|row| {
            row.spawn((
                Text::new(format!("{marker} {label}")),
                dialog_font(20.0, UiFontWeight::Regular),
                TextColor(fg),
            ));
        });
}

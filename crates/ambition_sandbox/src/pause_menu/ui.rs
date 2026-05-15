use super::model::{MAX_ROWS, RADIO_VISIBLE_ROWS, SETTINGS_VISIBLE_ROWS};
use super::*;

use crate::ui_nav::ListCursor;

pub fn spawn_pause_menu(mut commands: Commands) {
    let root = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                padding: UiRect::all(Val::Px(14.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.03, 0.06, 0.78)),
            ZIndex(50),
            Visibility::Hidden,
            PauseMenuRoot,
            Name::new("Pause menu"),
        ))
        .id();

    let top_panel = commands
        .spawn((
            Node {
                width: Val::Px(400.0),
                max_width: Val::Percent(92.0),
                max_height: Val::Percent(94.0),
                padding: UiRect::all(Val::Px(18.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(5.0),
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.08, 0.10, 0.16, 0.94)),
            BorderColor::all(Color::srgba(0.42, 0.78, 1.00, 0.85)),
            PauseMenuTopPanel,
            Name::new("Pause panel — top"),
        ))
        .id();
    commands.entity(root).add_child(top_panel);

    let title = commands
        .spawn((
            Text::new("Paused"),
            TextFont {
                font_size: 25.0,
                ..default()
            },
            TextColor(Color::srgba(0.92, 0.96, 1.0, 0.98)),
            Name::new("Pause title"),
        ))
        .id();
    commands.entity(top_panel).add_child(title);

    for item in PauseMenuItem::ALL {
        let label = item.static_label();
        let entity = commands
            .spawn((
                Button,
                Node {
                    width: Val::Percent(100.0),
                    min_height: Val::Px(34.0),
                    padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::NONE),
                Text::new(label),
                TextFont {
                    font_size: 19.0,
                    ..default()
                },
                TextColor(Color::srgba(0.78, 0.86, 0.96, 0.96)),
                item,
                Name::new(format!("Pause item: {label}")),
            ))
            .id();
        commands.entity(top_panel).add_child(entity);
    }

    let settings_panel = commands
        .spawn((
            Node {
                width: Val::Px(500.0),
                max_width: Val::Percent(94.0),
                max_height: Val::Percent(94.0),
                padding: UiRect::all(Val::Px(16.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                align_items: AlignItems::Center,
                display: Display::None,
                ..default()
            },
            BackgroundColor(Color::srgba(0.08, 0.10, 0.16, 0.94)),
            BorderColor::all(Color::srgba(0.42, 0.78, 1.00, 0.85)),
            PauseMenuSettingsPanel,
            Name::new("Pause panel — settings"),
        ))
        .id();
    commands.entity(root).add_child(settings_panel);

    let settings_title = commands
        .spawn((
            Text::new("Settings"),
            TextFont {
                font_size: 22.0,
                ..default()
            },
            TextColor(Color::srgba(0.92, 0.96, 1.0, 0.98)),
            SettingsTitle,
            Name::new("Settings title"),
        ))
        .id();
    commands.entity(settings_panel).add_child(settings_title);

    // Pre-spawn enough slot rows to hold the largest page. Each frame
    // the renderer fills `slot.index < rows.len()` slots with text and
    // hides the rest. This avoids respawning UI nodes per page swap,
    // which can cost a frame of layout instability.
    for index in 0..MAX_ROWS {
        let entity = commands
            .spawn((
                Button,
                Node {
                    width: Val::Percent(100.0),
                    min_height: Val::Px(30.0),
                    padding: UiRect::axes(Val::Px(10.0), Val::Px(5.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    // Start out of layout; sync_pause_menu flips to Flex
                    // for slots that map to a real row this frame.
                    display: Display::None,
                    ..default()
                },
                BackgroundColor(Color::NONE),
                Text::new(""),
                TextFont {
                    font_size: 17.0,
                    ..default()
                },
                TextColor(Color::srgba(0.78, 0.86, 0.96, 0.96)),
                Visibility::Hidden,
                SettingsRowSlot { index },
                Name::new(format!("Settings row slot {index}")),
            ))
            .id();
        commands.entity(settings_panel).add_child(entity);
    }
}

#[derive(Component)]
pub struct SettingsTitle;

/// Show/hide the pause overlay based on `GameMode` and update item highlights.
#[cfg(feature = "audio")]
#[allow(clippy::too_many_arguments)]
pub fn sync_pause_menu(
    mode: Res<State<GameMode>>,
    state: Res<PauseMenuState>,
    inventory: Res<InventoryUiState>,
    library: Res<AudioLibrary>,
    music_state: Res<MusicPlaybackState>,
    radio: Res<RadioStationState>,
    user_settings: Res<UserSettings>,
    dev_view: DevToggleView,
    mut roots: Query<&mut Visibility, With<PauseMenuRoot>>,
    mut top_panels: Query<&mut Node, (With<PauseMenuTopPanel>, Without<PauseMenuSettingsPanel>)>,
    mut settings_panels: Query<
        &mut Node,
        (With<PauseMenuSettingsPanel>, Without<PauseMenuTopPanel>),
    >,
    mut titles: Query<(&mut Text, &SettingsTitle), Without<SettingsRowSlot>>,
    mut top_items: Query<
        (
            &PauseMenuItem,
            &mut Text,
            &mut TextColor,
            &mut BackgroundColor,
        ),
        (Without<SettingsRowSlot>, Without<SettingsTitle>),
    >,
    mut row_slots: Query<
        (
            &SettingsRowSlot,
            &mut Node,
            &mut Visibility,
            &mut Text,
            &mut TextColor,
            &mut BackgroundColor,
        ),
        (
            Without<PauseMenuRoot>,
            Without<PauseMenuItem>,
            Without<SettingsTitle>,
            Without<PauseMenuTopPanel>,
            Without<PauseMenuSettingsPanel>,
        ),
    >,
) {
    let visible = matches!(mode.get(), GameMode::Paused) && !inventory.visible;
    for mut visibility in &mut roots {
        *visibility = if visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !visible {
        return;
    }

    let on_top = matches!(state.page, PauseMenuPage::Top);
    apply_page_visibility(on_top, &mut top_panels, &mut settings_panels);
    if matches!(state.page, PauseMenuPage::Top) {
        let selected_item = PauseMenuItem::ALL.get(state.selected).copied();
        for (item, mut text, mut color, mut bg) in &mut top_items {
            **text = item.label(Some(&music_state), Some(&library));
            apply_item_highlight(&mut color, &mut bg, Some(*item) == selected_item);
        }
        for (_, mut node, mut vis, _, _, _) in &mut row_slots {
            hide_row_slot(&mut node, &mut vis);
        }
    } else if let PauseMenuPage::Settings(page) = state.page {
        let rows = SettingsItem::rows_for(page);
        let dev = DevToggleSnapshot::capture(
            &dev_view.dev_state,
            &dev_view.developer,
            &dev_view.ldtk_reload,
        );
        let cursor = ListCursor::new(state.selected, rows.len());
        for (mut text, _) in &mut titles {
            **text = cursor.windowed_title(page.title(), SETTINGS_VISIBLE_ROWS);
        }
        for (slot, mut node, mut vis, mut text, mut color, mut bg) in &mut row_slots {
            if let Some(row_index) = cursor.visible_row_for_slot(slot.index, SETTINGS_VISIBLE_ROWS)
            {
                if let Some(item) = rows.get(row_index) {
                    **text = cursor.decorate_visible_label(
                        item.label_with_dev(&user_settings, dev),
                        row_index,
                        SETTINGS_VISIBLE_ROWS,
                    );
                    let selected = state.selected == row_index;
                    apply_item_highlight(&mut color, &mut bg, selected);
                    show_row_slot(&mut node, &mut vis);
                    continue;
                }
            }
            hide_row_slot(&mut node, &mut vis);
        }
    } else if matches!(state.page, PauseMenuPage::Radio) {
        let count = library.track_count();
        let cursor = ListCursor::new(state.selected, count);
        for (mut text, _) in &mut titles {
            **text = cursor.windowed_title("Radio", RADIO_VISIBLE_ROWS);
        }
        let active = radio
            .selected_track()
            .unwrap_or(music_state.active_track.as_str());
        for (slot, mut node, mut vis, mut text, mut color, mut bg) in &mut row_slots {
            if let Some(track_index) = cursor.visible_row_for_slot(slot.index, RADIO_VISIBLE_ROWS) {
                if let Some(label) = library.radio_label(track_index, active) {
                    **text = cursor.decorate_visible_label(label, track_index, RADIO_VISIBLE_ROWS);
                    let selected = state.selected == track_index;
                    apply_item_highlight(&mut color, &mut bg, selected);
                    show_row_slot(&mut node, &mut vis);
                    continue;
                }
            }
            hide_row_slot(&mut node, &mut vis);
        }
    }
}

/// Show a row slot. Toggles `display: Flex` so the row participates in the
/// settings panel's column layout (matching the original spawn-time default)
/// and clears any leftover `Visibility::Hidden` from the previous frame.
fn show_row_slot(node: &mut Node, vis: &mut Visibility) {
    if node.display != Display::Flex {
        node.display = Display::Flex;
    }
    // `Inherited` (not `Visible`) so the row follows the pause-menu root.
    // `Visible` is force-visible and would survive the root flipping to
    // `Hidden` on unpause, leaving stale rows on screen until the menu
    // was toggled twice.
    if *vis != Visibility::Inherited {
        *vis = Visibility::Inherited;
    }
}

/// Hide a row slot and remove it from layout entirely so empty slots do not
/// pad the settings/radio panel with blank vertical space.
fn hide_row_slot(node: &mut Node, vis: &mut Visibility) {
    if node.display != Display::None {
        node.display = Display::None;
    }
    if *vis != Visibility::Hidden {
        *vis = Visibility::Hidden;
    }
}

#[cfg(not(feature = "audio"))]
#[allow(clippy::too_many_arguments)]
pub fn sync_pause_menu(
    mode: Res<State<GameMode>>,
    state: Res<PauseMenuState>,
    inventory: Res<InventoryUiState>,
    user_settings: Res<UserSettings>,
    dev_view: DevToggleView,
    mut roots: Query<&mut Visibility, With<PauseMenuRoot>>,
    mut top_panels: Query<&mut Node, (With<PauseMenuTopPanel>, Without<PauseMenuSettingsPanel>)>,
    mut settings_panels: Query<
        &mut Node,
        (With<PauseMenuSettingsPanel>, Without<PauseMenuTopPanel>),
    >,
    mut titles: Query<(&mut Text, &SettingsTitle), Without<SettingsRowSlot>>,
    mut top_items: Query<
        (
            &PauseMenuItem,
            &mut Text,
            &mut TextColor,
            &mut BackgroundColor,
        ),
        (Without<SettingsRowSlot>, Without<SettingsTitle>),
    >,
    mut row_slots: Query<
        (
            &SettingsRowSlot,
            &mut Node,
            &mut Visibility,
            &mut Text,
            &mut TextColor,
            &mut BackgroundColor,
        ),
        (
            Without<PauseMenuRoot>,
            Without<PauseMenuItem>,
            Without<SettingsTitle>,
            Without<PauseMenuTopPanel>,
            Without<PauseMenuSettingsPanel>,
        ),
    >,
) {
    let visible = matches!(mode.get(), GameMode::Paused) && !inventory.visible;
    for mut visibility in &mut roots {
        *visibility = if visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !visible {
        return;
    }
    let on_top = matches!(state.page, PauseMenuPage::Top);
    apply_page_visibility(on_top, &mut top_panels, &mut settings_panels);
    if matches!(state.page, PauseMenuPage::Top) {
        let selected_item = PauseMenuItem::ALL.get(state.selected).copied();
        for (item, mut text, mut color, mut bg) in &mut top_items {
            **text = item.label();
            apply_item_highlight(&mut color, &mut bg, Some(*item) == selected_item);
        }
        for (_, mut node, mut vis, _, _, _) in &mut row_slots {
            hide_row_slot(&mut node, &mut vis);
        }
    } else if let PauseMenuPage::Settings(page) = state.page {
        let rows = SettingsItem::rows_for(page);
        let dev = DevToggleSnapshot::capture(
            &dev_view.dev_state,
            &dev_view.developer,
            &dev_view.ldtk_reload,
        );
        let cursor = ListCursor::new(state.selected, rows.len());
        for (mut text, _) in &mut titles {
            **text = cursor.windowed_title(page.title(), SETTINGS_VISIBLE_ROWS);
        }
        for (slot, mut node, mut vis, mut text, mut color, mut bg) in &mut row_slots {
            if let Some(row_index) = cursor.visible_row_for_slot(slot.index, SETTINGS_VISIBLE_ROWS)
            {
                if let Some(item) = rows.get(row_index) {
                    **text = cursor.decorate_visible_label(
                        item.label_with_dev(&user_settings, dev),
                        row_index,
                        SETTINGS_VISIBLE_ROWS,
                    );
                    let selected = state.selected == row_index;
                    apply_item_highlight(&mut color, &mut bg, selected);
                    show_row_slot(&mut node, &mut vis);
                    continue;
                }
            }
            hide_row_slot(&mut node, &mut vis);
        }
    } else if matches!(state.page, PauseMenuPage::Radio) {
        for (mut text, _) in &mut titles {
            **text = "Radio".to_string();
        }
        for (slot, mut node, mut vis, mut text, mut color, mut bg) in &mut row_slots {
            if slot.index == 0 {
                **text = "Audio feature disabled".to_string();
                apply_item_highlight(&mut color, &mut bg, state.selected == 0);
                show_row_slot(&mut node, &mut vis);
            } else {
                hide_row_slot(&mut node, &mut vis);
            }
        }
    }
}

fn apply_page_visibility(
    on_top: bool,
    top_panels: &mut Query<&mut Node, (With<PauseMenuTopPanel>, Without<PauseMenuSettingsPanel>)>,
    settings_panels: &mut Query<
        &mut Node,
        (With<PauseMenuSettingsPanel>, Without<PauseMenuTopPanel>),
    >,
) {
    for mut node in &mut *top_panels {
        node.display = if on_top { Display::Flex } else { Display::None };
    }
    for mut node in &mut *settings_panels {
        node.display = if on_top { Display::None } else { Display::Flex };
    }
}

fn apply_item_highlight(color: &mut TextColor, bg: &mut BackgroundColor, is_selected: bool) {
    *color = if is_selected {
        TextColor(Color::srgba(0.18, 0.06, 0.04, 1.0))
    } else {
        TextColor(Color::srgba(0.78, 0.86, 0.96, 0.96))
    };
    *bg = if is_selected {
        BackgroundColor(Color::srgba(0.95, 0.78, 0.32, 0.96))
    } else {
        BackgroundColor(Color::NONE)
    };
}

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

    // Settings rows + scrollbar share a horizontal row container so the
    // scrollbar pins to the right edge of the panel without overlapping
    // any row text or slider widget.
    let rows_and_bar = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Stretch,
                column_gap: Val::Px(6.0),
                ..default()
            },
            Name::new("Settings rows + scrollbar"),
        ))
        .id();
    commands.entity(settings_panel).add_child(rows_and_bar);

    let rows_column = commands
        .spawn((
            Node {
                flex_grow: 1.0,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                ..default()
            },
            Name::new("Settings rows"),
        ))
        .id();
    commands.entity(rows_and_bar).add_child(rows_column);

    // Pre-spawn enough slot rows to hold the largest page. Each frame
    // the renderer fills `slot.index < rows.len()` slots with text and
    // hides the rest. This avoids respawning UI nodes per page swap,
    // which can cost a frame of layout instability.
    for index in 0..MAX_ROWS {
        spawn_settings_row_slot(&mut commands, rows_column, index);
    }

    // Vertical scrollbar pinned to the right edge of the settings panel.
    // Width is intentionally chunky (~12 px) so a thumb can be grabbed
    // with a finger on Android; on desktop it doubles as a visible
    // affordance for the windowed-list scroll position.
    let scrollbar_track = commands
        .spawn((
            Node {
                width: Val::Px(14.0),
                height: Val::Percent(100.0),
                padding: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.10, 0.13, 0.20, 0.85)),
            BorderColor::all(Color::srgba(0.30, 0.42, 0.62, 0.60)),
            SettingsScrollbarTrack,
            // RelativeCursorPosition is filled by Bevy UI from cursor +
            // first-touch positions; the drag system reads `normalized`.
            bevy::ui::RelativeCursorPosition::default(),
            Interaction::default(),
            Name::new("Settings scrollbar track"),
        ))
        .id();
    let scrollbar_thumb = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(20.0),
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.62, 0.78, 0.96, 0.92)),
            SettingsScrollbarThumb,
            Name::new("Settings scrollbar thumb"),
        ))
        .id();
    commands.entity(scrollbar_track).add_child(scrollbar_thumb);
    commands.entity(rows_and_bar).add_child(scrollbar_track);
}

fn spawn_settings_row_slot(commands: &mut Commands, parent: Entity, index: usize) {
    // Each row slot is a Button whose body is a column flex: the text
    // label on top, the slider track underneath. Percent-style rows
    // show + drive the slider; toggle / enum / nav rows hide it via
    // `Display::None` in `sync_pause_menu`.
    let row = commands
        .spawn((
            Button,
            Node {
                width: Val::Percent(100.0),
                min_height: Val::Px(30.0),
                padding: UiRect::axes(Val::Px(10.0), Val::Px(5.0)),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(3.0),
                // Start out of layout; sync_pause_menu flips to Flex
                // for slots that map to a real row this frame.
                display: Display::None,
                ..default()
            },
            BackgroundColor(Color::NONE),
            Visibility::Hidden,
            SettingsRowSlot { index },
            Name::new(format!("Settings row slot {index}")),
        ))
        .id();
    let label = commands
        .spawn((
            Text::new(""),
            TextFont {
                font_size: 17.0,
                ..default()
            },
            TextColor(Color::srgba(0.78, 0.86, 0.96, 0.96)),
            SettingsRowLabel { index },
            Name::new(format!("Settings row label {index}")),
        ))
        .id();
    commands.entity(row).add_child(label);

    // Slider track: dark background with a brighter fill child whose
    // width is the normalized value. Both the track and the fill are
    // pure visuals; touch hit-testing keys on the track's
    // `RelativeCursorPosition`.
    let track = commands
        .spawn((
            Node {
                width: Val::Percent(86.0),
                height: Val::Px(8.0),
                display: Display::None,
                padding: UiRect::all(Val::Px(0.0)),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.06, 0.09, 0.14, 0.90)),
            BorderColor::all(Color::srgba(0.30, 0.42, 0.62, 0.60)),
            bevy::ui::RelativeCursorPosition::default(),
            Interaction::default(),
            SettingsRowSliderTrack { index },
            Name::new(format!("Settings row slider track {index}")),
        ))
        .id();
    let fill = commands
        .spawn((
            Node {
                width: Val::Percent(0.0),
                height: Val::Percent(100.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.62, 0.86, 1.00, 0.92)),
            SettingsRowSliderFill { index },
            Name::new(format!("Settings row slider fill {index}")),
        ))
        .id();
    commands.entity(track).add_child(fill);
    commands.entity(row).add_child(track);

    commands.entity(parent).add_child(row);
}

#[derive(Component)]
pub struct SettingsTitle;

/// Show/hide the pause overlay based on `GameMode` and update item highlights.
///
/// The settings/radio row content (text, slider fill, scrollbar) lives
/// in [`sync_settings_panel_rows`] because the slot Button no longer
/// owns its own `Text` — moving the label to a child entity was what
/// made the per-row slider widget possible. This system stays focused
/// on visibility + the still-button-owned Top page items.
#[cfg(feature = "audio")]
#[allow(clippy::too_many_arguments)]
pub fn sync_pause_menu(
    mode: Res<State<GameMode>>,
    state: Res<PauseMenuState>,
    inventory: Res<InventoryUiState>,
    library: Res<AudioLibrary>,
    music_state: Res<MusicPlaybackState>,
    mut roots: Query<&mut Visibility, With<PauseMenuRoot>>,
    mut top_panels: Query<&mut Node, (With<PauseMenuTopPanel>, Without<PauseMenuSettingsPanel>)>,
    mut settings_panels: Query<
        &mut Node,
        (With<PauseMenuSettingsPanel>, Without<PauseMenuTopPanel>),
    >,
    mut top_items: Query<
        (
            &PauseMenuItem,
            &mut Text,
            &mut TextColor,
            &mut BackgroundColor,
        ),
        (
            Without<SettingsRowSlot>,
            Without<SettingsTitle>,
            Without<SettingsRowLabel>,
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
    mut roots: Query<&mut Visibility, With<PauseMenuRoot>>,
    mut top_panels: Query<&mut Node, (With<PauseMenuTopPanel>, Without<PauseMenuSettingsPanel>)>,
    mut settings_panels: Query<
        &mut Node,
        (With<PauseMenuSettingsPanel>, Without<PauseMenuTopPanel>),
    >,
    mut top_items: Query<
        (
            &PauseMenuItem,
            &mut Text,
            &mut TextColor,
            &mut BackgroundColor,
        ),
        (
            Without<SettingsRowSlot>,
            Without<SettingsTitle>,
            Without<SettingsRowLabel>,
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
    }
}

/// Companion to [`sync_pause_menu`]: owns the Settings + Radio page
/// content. Updates the title, every slot's visibility + highlight, the
/// inner label text, the per-row slider visibility + fill, and the
/// scrollbar thumb position.
///
/// Split out from `sync_pause_menu` after the row slot stopped owning
/// its own `Text` (the label moved to a child so a slider can live in
/// the same row). Keeping these two systems separate also lets the
/// audio variant of the radio rendering stay cfg-gated without
/// duplicating the entire pause overlay.
#[allow(clippy::too_many_arguments)]
pub fn sync_settings_panel_rows(
    mode: Res<State<GameMode>>,
    state: Res<PauseMenuState>,
    inventory: Res<InventoryUiState>,
    user_settings: Res<UserSettings>,
    dev_view: DevToggleView,
    #[cfg(feature = "audio")] library: Res<AudioLibrary>,
    #[cfg(feature = "audio")] music_state: Res<MusicPlaybackState>,
    #[cfg(feature = "audio")] radio: Res<RadioStationState>,
    mut titles: Query<(&mut Text, &SettingsTitle), Without<SettingsRowLabel>>,
    mut row_slots: Query<
        (
            &SettingsRowSlot,
            &mut Node,
            &mut Visibility,
            &mut BackgroundColor,
        ),
        (
            Without<PauseMenuRoot>,
            Without<PauseMenuItem>,
            Without<SettingsTitle>,
            Without<PauseMenuTopPanel>,
            Without<PauseMenuSettingsPanel>,
            Without<SettingsRowLabel>,
            Without<SettingsRowSliderTrack>,
            Without<SettingsRowSliderFill>,
            Without<SettingsScrollbarThumb>,
        ),
    >,
    mut row_labels: Query<
        (&SettingsRowLabel, &mut Text, &mut TextColor),
        (Without<SettingsTitle>, Without<PauseMenuItem>),
    >,
    mut slider_tracks: Query<
        (&SettingsRowSliderTrack, &mut Node),
        (
            Without<SettingsRowSlot>,
            Without<SettingsRowSliderFill>,
            Without<SettingsScrollbarThumb>,
        ),
    >,
    mut slider_fills: Query<
        (&SettingsRowSliderFill, &mut Node),
        (
            Without<SettingsRowSlot>,
            Without<SettingsRowSliderTrack>,
            Without<SettingsScrollbarThumb>,
        ),
    >,
    mut scrollbar_thumb: Query<
        &mut Node,
        (
            With<SettingsScrollbarThumb>,
            Without<SettingsRowSlot>,
            Without<SettingsRowSliderTrack>,
            Without<SettingsRowSliderFill>,
        ),
    >,
) {
    let visible = matches!(mode.get(), GameMode::Paused) && !inventory.visible;
    if !visible {
        hide_all_rows(&mut row_slots, &mut slider_tracks);
        return;
    }
    match state.page {
        PauseMenuPage::Top => {
            hide_all_rows(&mut row_slots, &mut slider_tracks);
            for (mut text, _) in &mut titles {
                **text = "Settings".to_string();
            }
            hide_scrollbar(&mut scrollbar_thumb);
        }
        PauseMenuPage::Settings(page) => {
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
            // Reusable closure: for slot index N, return the row item
            // currently mapped there (or None if N is above the page's
            // row count or out of the scroll window).
            let item_for_slot = |slot_index: usize| -> Option<(usize, SettingsItem)> {
                let row_index = cursor.visible_row_for_slot(slot_index, SETTINGS_VISIBLE_ROWS)?;
                Some((row_index, *rows.get(row_index)?))
            };
            apply_visible_rows(
                state.selected,
                rows.len(),
                SETTINGS_VISIBLE_ROWS,
                &user_settings,
                |slot_index, settings| item_for_slot(slot_index).map(|(_, item)| {
                    RowRender {
                        label: cursor.decorate_visible_label(
                            item.label_with_dev(settings, dev),
                            item_for_slot(slot_index).unwrap().0,
                            SETTINGS_VISIBLE_ROWS,
                        ),
                        slider: item.normalized_value(settings),
                    }
                }),
                &cursor,
                &mut row_slots,
                &mut row_labels,
                &mut slider_tracks,
                &mut slider_fills,
            );
            update_scrollbar(
                state.selected,
                rows.len(),
                SETTINGS_VISIBLE_ROWS,
                &mut scrollbar_thumb,
            );
        }
        PauseMenuPage::Radio => {
            #[cfg(feature = "audio")]
            {
                let count = library.track_count();
                let cursor = ListCursor::new(state.selected, count);
                for (mut text, _) in &mut titles {
                    **text = cursor.windowed_title("Radio", RADIO_VISIBLE_ROWS);
                }
                let active = radio
                    .selected_track()
                    .unwrap_or(music_state.active_track.as_str());
                apply_visible_rows(
                    state.selected,
                    count,
                    RADIO_VISIBLE_ROWS,
                    &user_settings,
                    |slot_index, _settings| {
                        let track_index =
                            cursor.visible_row_for_slot(slot_index, RADIO_VISIBLE_ROWS)?;
                        let label = library.radio_label(track_index, active)?;
                        Some(RowRender {
                            label: cursor.decorate_visible_label(
                                label,
                                track_index,
                                RADIO_VISIBLE_ROWS,
                            ),
                            // Radio rows are confirm-to-select; no
                            // slider here.
                            slider: None,
                        })
                    },
                    &cursor,
                    &mut row_slots,
                    &mut row_labels,
                    &mut slider_tracks,
                    &mut slider_fills,
                );
                update_scrollbar(
                    state.selected,
                    count,
                    RADIO_VISIBLE_ROWS,
                    &mut scrollbar_thumb,
                );
            }
            #[cfg(not(feature = "audio"))]
            {
                for (mut text, _) in &mut titles {
                    **text = "Radio".to_string();
                }
                let cursor = ListCursor::new(0, 1);
                apply_visible_rows(
                    0,
                    1,
                    1,
                    &user_settings,
                    |slot_index, _settings| {
                        if slot_index == 0 {
                            Some(RowRender {
                                label: "Audio feature disabled".to_string(),
                                slider: None,
                            })
                        } else {
                            None
                        }
                    },
                    &cursor,
                    &mut row_slots,
                    &mut row_labels,
                    &mut slider_tracks,
                    &mut slider_fills,
                );
                hide_scrollbar(&mut scrollbar_thumb);
            }
        }
    }
}

/// Pre-computed render data for one settings/radio row.
struct RowRender {
    label: String,
    /// `Some(value)` to show a slider with the given 0..1 fill; `None`
    /// to hide the slider entirely.
    slider: Option<f32>,
}

#[allow(clippy::too_many_arguments)]
fn apply_visible_rows<F>(
    selected: usize,
    _row_count: usize,
    _visible_rows: usize,
    settings: &UserSettings,
    mut row_render: F,
    _cursor: &ListCursor,
    row_slots: &mut Query<
        (
            &SettingsRowSlot,
            &mut Node,
            &mut Visibility,
            &mut BackgroundColor,
        ),
        (
            Without<PauseMenuRoot>,
            Without<PauseMenuItem>,
            Without<SettingsTitle>,
            Without<PauseMenuTopPanel>,
            Without<PauseMenuSettingsPanel>,
            Without<SettingsRowLabel>,
            Without<SettingsRowSliderTrack>,
            Without<SettingsRowSliderFill>,
            Without<SettingsScrollbarThumb>,
        ),
    >,
    row_labels: &mut Query<
        (&SettingsRowLabel, &mut Text, &mut TextColor),
        (Without<SettingsTitle>, Without<PauseMenuItem>),
    >,
    slider_tracks: &mut Query<
        (&SettingsRowSliderTrack, &mut Node),
        (
            Without<SettingsRowSlot>,
            Without<SettingsRowSliderFill>,
            Without<SettingsScrollbarThumb>,
        ),
    >,
    slider_fills: &mut Query<
        (&SettingsRowSliderFill, &mut Node),
        (
            Without<SettingsRowSlot>,
            Without<SettingsRowSliderTrack>,
            Without<SettingsScrollbarThumb>,
        ),
    >,
) where
    F: FnMut(usize, &UserSettings) -> Option<RowRender>,
{
    use bevy::platform::collections::HashMap;

    // Render every slot first into a small table keyed by slot.index;
    // the queries below then index into it without re-running the
    // closure (which may borrow caller-owned state per call).
    let mut renders: HashMap<usize, Option<RowRender>> = HashMap::default();
    let mut selected_slot: Option<usize> = None;
    for (slot, _, _, _) in row_slots.iter() {
        let render = row_render(slot.index, settings);
        if render.is_some() && slot.index == selected {
            selected_slot = Some(slot.index);
        }
        renders.insert(slot.index, render);
    }
    // Re-check whether the slot mapped to `selected` is actually a
    // visible row — the closure above already gated on it but the
    // index→row mapping uses the cursor's windowing.
    let _ = selected_slot;

    for (slot, mut node, mut vis, mut bg) in row_slots.iter_mut() {
        let render = renders.get(&slot.index).and_then(|r| r.as_ref());
        if render.is_some() {
            show_row_slot(&mut node, &mut vis);
            let is_selected =
                row_index_for_slot_in_cursor(_cursor, slot.index, _visible_rows)
                    .map(|i| i == selected)
                    .unwrap_or(false);
            apply_slot_bg(&mut bg, is_selected);
        } else {
            hide_row_slot(&mut node, &mut vis);
        }
    }

    for (label_marker, mut text, mut color) in row_labels.iter_mut() {
        let render = renders.get(&label_marker.index).and_then(|r| r.as_ref());
        if let Some(render) = render {
            **text = render.label.clone();
            let is_selected = row_index_for_slot_in_cursor(_cursor, label_marker.index, _visible_rows)
                .map(|i| i == selected)
                .unwrap_or(false);
            apply_label_color(&mut color, is_selected);
        } else {
            text.clear();
        }
    }

    for (track_marker, mut track_node) in slider_tracks.iter_mut() {
        let slider = renders
            .get(&track_marker.index)
            .and_then(|r| r.as_ref())
            .and_then(|r| r.slider);
        track_node.display = if slider.is_some() {
            Display::Flex
        } else {
            Display::None
        };
    }

    for (fill_marker, mut fill_node) in slider_fills.iter_mut() {
        let slider = renders
            .get(&fill_marker.index)
            .and_then(|r| r.as_ref())
            .and_then(|r| r.slider);
        let pct = slider.unwrap_or(0.0).clamp(0.0, 1.0) * 100.0;
        fill_node.width = Val::Percent(pct);
    }
}

fn row_index_for_slot_in_cursor(
    cursor: &ListCursor,
    slot_index: usize,
    visible_rows: usize,
) -> Option<usize> {
    cursor.visible_row_for_slot(slot_index, visible_rows)
}

fn hide_all_rows(
    row_slots: &mut Query<
        (
            &SettingsRowSlot,
            &mut Node,
            &mut Visibility,
            &mut BackgroundColor,
        ),
        (
            Without<PauseMenuRoot>,
            Without<PauseMenuItem>,
            Without<SettingsTitle>,
            Without<PauseMenuTopPanel>,
            Without<PauseMenuSettingsPanel>,
            Without<SettingsRowLabel>,
            Without<SettingsRowSliderTrack>,
            Without<SettingsRowSliderFill>,
            Without<SettingsScrollbarThumb>,
        ),
    >,
    slider_tracks: &mut Query<
        (&SettingsRowSliderTrack, &mut Node),
        (
            Without<SettingsRowSlot>,
            Without<SettingsRowSliderFill>,
            Without<SettingsScrollbarThumb>,
        ),
    >,
) {
    for (_, mut node, mut vis, _) in row_slots.iter_mut() {
        hide_row_slot(&mut node, &mut vis);
    }
    for (_, mut node) in slider_tracks.iter_mut() {
        if node.display != Display::None {
            node.display = Display::None;
        }
    }
}

fn hide_scrollbar(
    scrollbar_thumb: &mut Query<
        &mut Node,
        (
            With<SettingsScrollbarThumb>,
            Without<SettingsRowSlot>,
            Without<SettingsRowSliderTrack>,
            Without<SettingsRowSliderFill>,
        ),
    >,
) {
    for mut node in scrollbar_thumb.iter_mut() {
        node.height = Val::Percent(0.0);
        node.top = Val::Px(0.0);
    }
}

/// Bind the scrollbar thumb's height + top to the current windowed
/// list position. Height is the fraction of rows currently visible;
/// top is the fractional scroll offset of the selected row.
fn update_scrollbar(
    selected: usize,
    row_count: usize,
    visible_rows: usize,
    scrollbar_thumb: &mut Query<
        &mut Node,
        (
            With<SettingsScrollbarThumb>,
            Without<SettingsRowSlot>,
            Without<SettingsRowSliderTrack>,
            Without<SettingsRowSliderFill>,
        ),
    >,
) {
    if row_count == 0 || visible_rows >= row_count {
        hide_scrollbar(scrollbar_thumb);
        return;
    }
    let height_pct = (visible_rows as f32 / row_count as f32 * 100.0).clamp(8.0, 100.0);
    // `selected` ranges [0, row_count-1]; map it onto the available
    // vertical travel so the thumb sits flush at the ends.
    let denom = (row_count.saturating_sub(1)).max(1) as f32;
    let frac = (selected as f32 / denom).clamp(0.0, 1.0);
    let top_pct = frac * (100.0 - height_pct);
    for mut node in scrollbar_thumb.iter_mut() {
        node.height = Val::Percent(height_pct);
        node.top = Val::Percent(top_pct);
    }
}

fn apply_slot_bg(bg: &mut BackgroundColor, is_selected: bool) {
    *bg = if is_selected {
        BackgroundColor(Color::srgba(0.95, 0.78, 0.32, 0.96))
    } else {
        BackgroundColor(Color::NONE)
    };
}

fn apply_label_color(color: &mut TextColor, is_selected: bool) {
    *color = if is_selected {
        TextColor(Color::srgba(0.18, 0.06, 0.04, 1.0))
    } else {
        TextColor(Color::srgba(0.78, 0.86, 0.96, 0.96))
    };
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

/// Touch/mouse drag on a slider track sets the underlying value.
///
/// Reads Bevy UI's [`Interaction`] + [`RelativeCursorPosition`] on the
/// slider track entity. While the slot is pressed (mouse held or finger
/// down) and the cursor is over the track, the slider's normalized X
/// position is written to `UserSettings` through
/// `SettingsItem::try_set_normalized`. Also bumps `PauseMenuState.selected`
/// so the row highlight follows the slider you grabbed.
#[cfg(feature = "input")]
pub fn settings_slider_drag_input(
    mode: Res<State<GameMode>>,
    inventory: Res<InventoryUiState>,
    mut state: ResMut<PauseMenuState>,
    mut user_settings: ResMut<UserSettings>,
    sliders: Query<
        (&SettingsRowSliderTrack, &Interaction, &bevy::ui::RelativeCursorPosition),
        Changed<bevy::ui::RelativeCursorPosition>,
    >,
) {
    if !matches!(mode.get(), GameMode::Paused) || inventory.visible {
        return;
    }
    let PauseMenuPage::Settings(page) = state.page else {
        return;
    };
    let rows = SettingsItem::rows_for(page);
    let cursor = ListCursor::new(state.selected, rows.len());
    for (track, interaction, rel) in &sliders {
        if !matches!(interaction, Interaction::Pressed) {
            continue;
        }
        let Some(row_index) = cursor.visible_row_for_slot(track.index, SETTINGS_VISIBLE_ROWS)
        else {
            continue;
        };
        let Some(item) = rows.get(row_index).copied() else {
            continue;
        };
        let Some(norm) = rel.normalized else {
            continue;
        };
        // `normalized` is `[-0.5, 0.5]` relative to the node center.
        // Convert to `[0, 1]` and write back.
        let fraction = (norm.x + 0.5).clamp(0.0, 1.0);
        if item.try_set_normalized(&mut user_settings, fraction) {
            state.selected = row_index;
        }
    }
}

/// Touch/mouse drag on the scrollbar track moves the row selection.
/// Maps the relative Y position to a row index, which scrolls the
/// windowed list because the cursor is bound to `state.selected`.
#[cfg(feature = "input")]
pub fn settings_scrollbar_drag_input(
    mode: Res<State<GameMode>>,
    inventory: Res<InventoryUiState>,
    mut state: ResMut<PauseMenuState>,
    #[cfg(feature = "audio")] library: Res<AudioLibrary>,
    track: Query<
        (&Interaction, &bevy::ui::RelativeCursorPosition),
        (
            With<SettingsScrollbarTrack>,
            Changed<bevy::ui::RelativeCursorPosition>,
        ),
    >,
) {
    if !matches!(mode.get(), GameMode::Paused) || inventory.visible {
        return;
    }
    let row_count = match state.page {
        PauseMenuPage::Settings(page) => SettingsItem::rows_for(page).len(),
        PauseMenuPage::Radio => {
            #[cfg(feature = "audio")]
            {
                library.track_count()
            }
            #[cfg(not(feature = "audio"))]
            {
                1
            }
        }
        PauseMenuPage::Top => 0,
    };
    if row_count == 0 {
        return;
    }
    for (interaction, rel) in &track {
        if !matches!(interaction, Interaction::Pressed) {
            continue;
        }
        let Some(norm) = rel.normalized else {
            continue;
        };
        let fraction = (norm.y + 0.5).clamp(0.0, 1.0);
        // Land on the closest row: round to nearest so a thumb in the
        // middle picks the middle row instead of always biasing low.
        let max_index = row_count.saturating_sub(1) as f32;
        state.selected = (fraction * max_index).round().clamp(0.0, max_index) as usize;
        state.pointer_armed = None;
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

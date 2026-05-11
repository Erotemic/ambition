use super::*;

use crate::ui_nav::ScrollWindow;

const MAP_TAB_VISIBLE_LINES: usize = 10;
const QUEST_TAB_VISIBLE_LINES: usize = 9;

pub fn spawn_inventory_panel(mut commands: Commands) {
    let root = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                padding: UiRect::all(Val::Px(18.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.03, 0.06, 0.84)),
            ZIndex(60),
            Visibility::Hidden,
            InventoryRoot,
            Name::new("Adventure menu root"),
        ))
        .id();

    let panel = commands
        .spawn((
            Node {
                width: Val::Px(620.0),
                max_width: Val::Percent(96.0),
                max_height: Val::Percent(92.0),
                padding: UiRect::all(Val::Px(22.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(10.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.08, 0.10, 0.16, 0.96)),
            Name::new("Adventure menu panel"),
        ))
        .id();
    commands.entity(root).add_child(panel);

    let header = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(12.0),
                ..default()
            },
            Name::new("Adventure menu header"),
        ))
        .id();
    commands.entity(panel).add_child(header);

    let top_back = commands
        .spawn((
            Button,
            Node {
                min_width: Val::Px(118.0),
                min_height: Val::Px(48.0),
                padding: UiRect::axes(Val::Px(14.0), Val::Px(10.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.15, 0.20, 0.30, 0.96)),
            Text::new("← Back"),
            TextFont {
                font_size: 18.0,
                ..default()
            },
            TextColor(Color::srgba(0.90, 0.95, 1.0, 0.98)),
            InventoryBackButton,
            Name::new("Adventure menu top back button"),
        ))
        .id();
    commands.entity(header).add_child(top_back);

    let title = commands
        .spawn((
            Node {
                flex_grow: 1.0,
                justify_content: JustifyContent::Center,
                ..default()
            },
            Text::new("Adventure Menu"),
            TextFont {
                font_size: 28.0,
                ..default()
            },
            TextColor(Color::srgba(0.92, 0.96, 1.0, 0.98)),
            InventoryTitleText,
            Name::new("Adventure menu title"),
        ))
        .id();
    commands.entity(header).add_child(title);

    let tab_bar = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(8.0),
                ..default()
            },
            Name::new("Adventure menu tab bar"),
        ))
        .id();
    commands.entity(panel).add_child(tab_bar);

    for tab in InventoryTab::ALL {
        let entity = commands
            .spawn((
                Button,
                Node {
                    flex_grow: 1.0,
                    min_height: Val::Px(44.0),
                    padding: UiRect::axes(Val::Px(12.0), Val::Px(10.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::NONE),
                Text::new(tab.label()),
                TextFont {
                    font_size: 20.0,
                    ..default()
                },
                TextColor(Color::srgba(0.78, 0.86, 0.96, 0.96)),
                InventoryTabButton { tab },
                Name::new(format!("Adventure tab: {}", tab.label())),
            ))
            .id();
        commands.entity(tab_bar).add_child(entity);
    }

    for kind in ItemKind::ALL {
        let row = commands
            .spawn((
                Button,
                Node {
                    width: Val::Percent(100.0),
                    min_height: Val::Px(46.0),
                    padding: UiRect::axes(Val::Px(14.0), Val::Px(10.0)),
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::NONE),
                Text::new(""),
                TextFont {
                    font_size: 22.0,
                    ..default()
                },
                TextColor(Color::srgba(0.82, 0.92, 1.0, 0.96)),
                InventoryItemRow { kind },
                Name::new(format!("Inventory row: {}", kind.label())),
            ))
            .id();
        commands.entity(panel).add_child(row);
    }

    let tab_content = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                min_height: Val::Px(180.0),
                padding: UiRect::all(Val::Px(12.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.03, 0.06, 0.36)),
            Text::new(""),
            TextFont {
                font_size: 16.0,
                ..default()
            },
            TextColor(Color::srgba(0.78, 0.88, 0.98, 0.94)),
            InventoryTabContentText,
            Name::new("Adventure menu tab content"),
        ))
        .id();
    commands.entity(panel).add_child(tab_content);

    let description = commands
        .spawn((
            Node {
                margin: UiRect::top(Val::Px(8.0)),
                ..default()
            },
            Text::new(""),
            TextFont {
                font_size: 14.0,
                ..default()
            },
            TextColor(Color::srgba(0.74, 0.84, 0.96, 0.92)),
            InventoryDescriptionText,
            Name::new("Inventory description"),
        ))
        .id();
    commands.entity(panel).add_child(description);

    let footer = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(10.0),
                align_items: AlignItems::Center,
                ..default()
            },
            Name::new("Adventure menu footer"),
        ))
        .id();
    commands.entity(panel).add_child(footer);

    let back = commands
        .spawn((
            Button,
            Node {
                min_width: Val::Px(118.0),
                min_height: Val::Px(44.0),
                padding: UiRect::axes(Val::Px(14.0), Val::Px(10.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.15, 0.20, 0.30, 0.96)),
            Text::new("← Back"),
            TextFont {
                font_size: 18.0,
                ..default()
            },
            TextColor(Color::srgba(0.90, 0.95, 1.0, 0.98)),
            InventoryBackButton,
            Name::new("Adventure menu back button"),
        ))
        .id();
    commands.entity(footer).add_child(back);

    let status = commands
        .spawn((
            Node {
                flex_grow: 1.0,
                ..default()
            },
            Text::new(""),
            TextFont {
                font_size: 12.0,
                ..default()
            },
            TextColor(Color::srgba(0.58, 0.70, 0.86, 0.86)),
            InventoryStatusText,
            Name::new("Adventure menu status"),
        ))
        .id();
    commands.entity(footer).add_child(status);
}

#[allow(clippy::too_many_arguments)]
pub fn sync_inventory_panel(
    state: Res<InventoryUiState>,
    inventory: Res<PlayerInventory>,
    map: Res<crate::map_menu::MapMenuState>,
    quests: Res<crate::quest::QuestRegistry>,
    room_set: Res<crate::rooms::RoomSet>,
    mut roots: Query<&mut Visibility, With<InventoryRoot>>,
    mut widgets: Query<
        (
            Option<&InventoryTitleText>,
            Option<&InventoryTabButton>,
            Option<&InventoryItemRow>,
            Option<&InventoryTabContentText>,
            Option<&InventoryDescriptionText>,
            Option<&InventoryStatusText>,
            &mut Text,
            Option<&mut TextColor>,
            Option<&mut BackgroundColor>,
            Option<&mut Node>,
        ),
        Or<(
            With<InventoryTitleText>,
            With<InventoryTabButton>,
            With<InventoryItemRow>,
            With<InventoryTabContentText>,
            With<InventoryDescriptionText>,
            With<InventoryStatusText>,
        )>,
    >,
) {
    for mut visibility in &mut roots {
        *visibility = if state.visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !state.visible {
        return;
    }

    let selected_kind = ItemKind::ALL.get(state.selected).copied();
    let show_items = state.tab == InventoryTab::Items;
    let description_text = match state.tab {
        InventoryTab::Items => selected_kind
            .map(|kind| {
                if inventory.count(kind) == 0 {
                    format!("{} — empty", kind.label())
                } else {
                    kind.description().to_string()
                }
            })
            .unwrap_or_default(),
        InventoryTab::Map => {
            "Known rooms and current position. Drag/scroll to move through long lists.".into()
        }
        InventoryTab::Quests => {
            "Active quest steps. Drag/scroll to move through long lists.".into()
        }
    };
    let content_text = match state.tab {
        InventoryTab::Items => String::new(),
        InventoryTab::Map => map_tab_text(&state, &map, &room_set),
        InventoryTab::Quests => quest_tab_text(&state, &quests),
    };
    let status_text = match state.tab {
        InventoryTab::Items => {
            "Tap tabs or ←/→ pages   Confirm uses item   Back closes".to_string()
        }
        InventoryTab::Map => {
            "Tap tabs or ←/→ pages   ↑/↓ or drag scrolls rooms   Back closes".to_string()
        }
        InventoryTab::Quests => {
            "Tap tabs or ←/→ pages   ↑/↓ or drag scrolls quests   Back closes".to_string()
        }
    };

    // Use a single query over all adventure-menu text widgets instead of several
    // mutable `Text` queries. Bevy validates access at schedule initialization,
    // and a single query is the simplest way to prove there is only one mutable
    // access path to `Text` in this system.
    for (
        title_marker,
        tab_marker,
        item_marker,
        content_marker,
        description_marker,
        status_marker,
        mut text,
        color,
        bg,
        node,
    ) in &mut widgets
    {
        if title_marker.is_some() {
            **text = format!("Adventure Menu — {}", state.tab.label());
            continue;
        }

        if let Some(tab) = tab_marker {
            **text = tab.tab.label().to_string();
            let selected = state.tab == tab.tab;
            if let (Some(mut color), Some(mut bg)) = (color, bg) {
                apply_adventure_highlight(&mut color, &mut bg, selected);
            }
            continue;
        }

        if let Some(row) = item_marker {
            if let Some(mut node) = node {
                node.display = if show_items {
                    Display::Flex
                } else {
                    Display::None
                };
            }
            if !show_items {
                continue;
            }

            let count = inventory.count(row.kind);
            **text = format!("{:<20} x {}", row.kind.label(), count);
            let is_selected = Some(row.kind) == selected_kind;
            if let Some(mut color) = color {
                *color = if is_selected {
                    TextColor(Color::srgba(0.18, 0.06, 0.04, 1.0))
                } else if count == 0 {
                    TextColor(Color::srgba(0.50, 0.56, 0.66, 0.86))
                } else {
                    TextColor(Color::srgba(0.82, 0.92, 1.0, 0.96))
                };
            }
            if let Some(mut bg) = bg {
                *bg = if is_selected {
                    BackgroundColor(Color::srgba(0.95, 0.78, 0.32, 0.96))
                } else {
                    BackgroundColor(Color::NONE)
                };
            }
            continue;
        }

        if content_marker.is_some() {
            if let Some(mut node) = node {
                node.display = if show_items {
                    Display::None
                } else {
                    Display::Flex
                };
            }
            **text = content_text.clone();
            continue;
        }

        if description_marker.is_some() {
            **text = description_text.clone();
            continue;
        }

        if status_marker.is_some() {
            **text = status_text.clone();
        }
    }
}

fn map_tab_text(
    state: &InventoryUiState,
    map: &crate::map_menu::MapMenuState,
    room_set: &crate::rooms::RoomSet,
) -> String {
    let current = room_set.active_spec().id.as_str();
    let mut lines = Vec::new();
    lines.push(format!("Current room: {current}"));
    lines.push(format!(
        "Visited: {} / {} rooms",
        map.visited.len(),
        map.rooms.len()
    ));
    lines.push(String::new());
    if map.visited.is_empty() {
        lines.push("No rooms visited yet.".into());
    } else {
        let window = ScrollWindow::new(
            state.content_scroll,
            map.visited.len(),
            MAP_TAB_VISIBLE_LINES,
        );
        for id in map
            .visited
            .iter()
            .skip(window.start)
            .take(MAP_TAB_VISIBLE_LINES)
        {
            let marker = if id == current { "→" } else { " " };
            lines.push(format!("{marker} {id}"));
        }
        if let Some(hint) = window.hint_line() {
            lines.push(String::new());
            lines.push(hint);
        }
    }
    lines.join("\n")
}

fn quest_tab_text(state: &InventoryUiState, quests: &crate::quest::QuestRegistry) -> String {
    let lines = quests.quest_log_lines();
    if lines.is_empty() {
        return "No active quests.".into();
    }
    let window = ScrollWindow::new(state.content_scroll, lines.len(), QUEST_TAB_VISIBLE_LINES);
    let mut visible = Vec::new();
    if window.has_before() {
        visible.push(format!(
            "↑ more   rows {}-{} of {}",
            window.start + 1,
            window.end(),
            window.total
        ));
    }
    for line in lines
        .iter()
        .skip(window.start)
        .take(QUEST_TAB_VISIBLE_LINES)
    {
        visible.push(format!("• {line}"));
    }
    if window.has_after() {
        visible.push(format!(
            "rows {}-{} of {}   ↓ more",
            window.start + 1,
            window.end(),
            window.total
        ));
    }
    visible.join("\n")
}

fn apply_adventure_highlight(color: &mut TextColor, bg: &mut BackgroundColor, is_selected: bool) {
    *color = if is_selected {
        TextColor(Color::srgba(0.18, 0.06, 0.04, 1.0))
    } else {
        TextColor(Color::srgba(0.78, 0.86, 0.96, 0.96))
    };
    *bg = if is_selected {
        BackgroundColor(Color::srgba(0.95, 0.78, 0.32, 0.96))
    } else {
        BackgroundColor(Color::srgba(0.12, 0.16, 0.24, 0.84))
    };
}

//! Bevy-UI rendering for the OoT 6×4 item grid overlay.
//!
//! This is the placeholder/native renderer: plain Bevy UI nodes laid out as a
//! CSS grid, with text labels standing in for item icons until art exists. It
//! reads the same [`OwnedItems`] + [`GridMenuState`] a future 3D-cube renderer
//! (submodule `ambition_inventory_ui`) would consume, so swapping renderers does
//! not touch the catalog or the input/effects logic.

use bevy::prelude::*;

use super::state::GridMenuState;
use crate::items::{Item, ItemCategory, OwnedItems, ITEM_GRID_COLS, ITEM_GRID_ROWS};

/// Root overlay node (fullscreen dim + centered panel). Toggled by visibility.
#[derive(Component)]
pub struct GridMenuRoot;

/// One of the 24 grid slots. Carries which catalog item it represents.
#[derive(Component, Clone, Copy)]
pub struct GridSlot {
    pub item: Item,
}

/// The single multi-line detail/footer text (name + description + status), kept
/// as one entity so it stays disjoint from the slots' `&mut Text` query.
#[derive(Component)]
pub struct GridDetailText;

/// Back / close button.
#[derive(Component)]
pub struct GridBackButton;

// Touch-friendly slot sizing — large enough to tap on a phone.
const SLOT_MIN_W: f32 = 88.0;
const SLOT_MIN_H: f32 = 66.0;

fn slot_bg(selected: bool, owned: bool, equipped: bool) -> Color {
    if selected {
        Color::srgba(0.95, 0.78, 0.32, 0.96) // gold cursor
    } else if equipped {
        Color::srgba(0.18, 0.42, 0.24, 0.96) // green = equipped
    } else if owned {
        Color::srgba(0.14, 0.20, 0.30, 0.95)
    } else {
        Color::srgba(0.09, 0.11, 0.16, 0.85) // dim = not acquired
    }
}

fn slot_fg(selected: bool, owned: bool) -> Color {
    if selected {
        Color::srgba(0.05, 0.06, 0.10, 1.0)
    } else if owned {
        Color::srgba(0.88, 0.94, 1.0, 1.0)
    } else {
        Color::srgba(0.45, 0.50, 0.60, 0.7)
    }
}

/// Compact slot label: optional ▶ (equipped), the item name, and `×N` for
/// stacked consumables.
fn slot_label(item: Item, count: u32, equipped: bool) -> String {
    let mut s = String::new();
    if equipped {
        s.push_str("▶ ");
    }
    s.push_str(item.display_name());
    if !item.category().is_unique() && count > 1 {
        s.push_str(&format!("\n×{count}"));
    }
    s
}

pub fn spawn_grid_menu(mut commands: Commands) {
    let root = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                padding: UiRect::all(Val::Px(16.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.03, 0.06, 0.86)),
            ZIndex(62),
            Visibility::Hidden,
            GridMenuRoot,
            Name::new("OoT item menu root"),
        ))
        .id();

    let panel = commands
        .spawn((
            Node {
                width: Val::Px(660.0),
                max_width: Val::Percent(96.0),
                max_height: Val::Percent(94.0),
                padding: UiRect::all(Val::Px(20.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(12.0),
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.07, 0.09, 0.15, 0.97)),
            Name::new("OoT item menu panel"),
        ))
        .id();
    commands.entity(root).add_child(panel);

    // Header row: title + back.
    let header = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            },
            Name::new("OoT item menu header"),
        ))
        .id();
    commands.entity(panel).add_child(header);

    let title = commands
        .spawn((
            Text::new("Items"),
            TextFont {
                font_size: 24.0,
                ..default()
            },
            TextColor(Color::srgba(0.95, 0.85, 0.45, 1.0)),
            Name::new("OoT item menu title"),
        ))
        .id();
    commands.entity(header).add_child(title);

    let back = commands
        .spawn((
            Button,
            Node {
                min_width: Val::Px(110.0),
                min_height: Val::Px(46.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                padding: UiRect::axes(Val::Px(14.0), Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.15, 0.20, 0.30, 0.96)),
            Text::new("Close"),
            TextFont {
                font_size: 18.0,
                ..default()
            },
            TextColor(Color::srgba(0.90, 0.95, 1.0, 0.98)),
            GridBackButton,
            Name::new("OoT item menu back button"),
        ))
        .id();
    commands.entity(header).add_child(back);

    // The 6×4 grid.
    let grid = commands
        .spawn((
            Node {
                display: Display::Grid,
                grid_template_columns: vec![RepeatedGridTrack::flex(ITEM_GRID_COLS as u16, 1.0)],
                grid_template_rows: vec![RepeatedGridTrack::flex(ITEM_GRID_ROWS as u16, 1.0)],
                column_gap: Val::Px(8.0),
                row_gap: Val::Px(8.0),
                width: Val::Percent(100.0),
                ..default()
            },
            Name::new("OoT item grid"),
        ))
        .id();
    commands.entity(panel).add_child(grid);

    for item in Item::ALL {
        let slot = commands
            .spawn((
                Button,
                Node {
                    min_width: Val::Px(SLOT_MIN_W),
                    min_height: Val::Px(SLOT_MIN_H),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    padding: UiRect::all(Val::Px(4.0)),
                    ..default()
                },
                BackgroundColor(slot_bg(false, false, false)),
                Text::new(item.display_name()),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(slot_fg(false, false)),
                GridSlot { item },
                Name::new(format!("OoT slot: {}", item.display_name())),
            ))
            .id();
        commands.entity(grid).add_child(slot);
    }

    // Detail / footer text (name + description + status).
    let detail = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                min_height: Val::Px(74.0),
                padding: UiRect::all(Val::Px(10.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.07, 0.12, 0.96)),
            Text::new(""),
            TextFont {
                font_size: 15.0,
                ..default()
            },
            TextColor(Color::srgba(0.85, 0.90, 0.98, 0.98)),
            GridDetailText,
            Name::new("OoT item menu detail text"),
        ))
        .id();
    commands.entity(panel).add_child(detail);
}

/// Mirror state + ownership into the grid visuals every frame.
pub fn sync_grid_menu(
    state: Res<GridMenuState>,
    overlay: Res<crate::inventory::InventoryUiState>,
    // The grid is one of two inventory frontends; when the Cube backend is active it
    // renders the inventory, so the bevy_ui grid must stay hidden (otherwise it
    // flashes behind the order-8 cube on open). `Option<Res>` so the grid still works
    // if the cube hookup is ever absent.
    backend: Option<Res<crate::lunex_kaleidoscope_app::InventoryUiBackend>>,
    owned: Res<OwnedItems>,
    mut roots: Query<&mut Visibility, With<GridMenuRoot>>,
    mut slots: Query<
        (&GridSlot, &mut BackgroundColor, &mut Text, &mut TextColor),
        Without<GridDetailText>,
    >,
    mut detail: Query<&mut Text, (With<GridDetailText>, Without<GridSlot>)>,
) {
    let grid_backend = backend
        .map(|b| *b == crate::lunex_kaleidoscope_app::InventoryUiBackend::Grid)
        .unwrap_or(true);
    let visible = overlay.visible && grid_backend;
    for mut vis in &mut roots {
        *vis = if visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !visible {
        return;
    }

    let selected = state.selected_item();
    for (slot, mut bg, mut text, mut fg) in &mut slots {
        let item = slot.item;
        let is_selected = item == selected;
        let count = owned.count(item);
        let is_owned = count > 0;
        let is_equipped = owned.is_equipped(item);
        bg.0 = slot_bg(is_selected, is_owned, is_equipped);
        fg.0 = slot_fg(is_selected, is_owned);
        text.0 = slot_label(item, count, is_equipped);
    }

    if let Ok(mut detail_text) = detail.single_mut() {
        let item = selected;
        let count = owned.count(item);
        let owned_line = match item.category() {
            ItemCategory::Reserved => "Empty slot".to_string(),
            _ if count == 0 => "Not acquired".to_string(),
            ItemCategory::Consumable => format!("Owned: ×{count}"),
            ItemCategory::Weapon if owned.is_equipped(item) => "Equipped".to_string(),
            ItemCategory::Weapon => "Owned — confirm to equip".to_string(),
            _ => "Owned".to_string(),
        };
        let status = if state.status.is_empty() {
            String::new()
        } else {
            format!("\n{}", state.status)
        };
        detail_text.0 = format!(
            "{}\n{}\n{}{}",
            item.display_name(),
            item.description(),
            owned_line,
            status
        );
    }
}

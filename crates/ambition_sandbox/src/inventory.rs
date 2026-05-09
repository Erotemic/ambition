//! Inventory/adventure menu model + overlay panel.
//!
//! The runtime owns a flat `PlayerInventory` resource (item kind → count).
//! The presentation side renders a phone-friendly adventure menu with
//! left/right tabs for Items, Map, and Quests. The menu consumes the semantic
//! `MenuControlFrame` instead of raw keyboard/gamepad/touch input so desktop,
//! gamepad, Android touch, and future controller schemes can all drive the same
//! UI contract.
//!
//! Items are currently minimal: pressing confirm uses the selected item. The
//! only effect today is `HealthPotion`, which heals the player by a fixed amount
//! and decrements the stack.

use bevy::prelude::*;

#[cfg(feature = "input")]
use crate::game_mode::GameMode;
#[cfg(feature = "input")]
use crate::input::MenuControlFrame;
use crate::SandboxRuntime;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ItemKind {
    HealthPotion,
    SpareBattery,
    DataChip,
}

impl ItemKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::HealthPotion => "Health Cell",
            Self::SpareBattery => "Spare Battery",
            Self::DataChip => "Data Chip",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::HealthPotion => "Restores 2 HP.",
            Self::SpareBattery => "Reserved for future ability charge.",
            Self::DataChip => "Lore fragment — does nothing yet.",
        }
    }

    pub const ALL: [Self; 3] = [Self::HealthPotion, Self::SpareBattery, Self::DataChip];
}

/// Top-level adventure-menu tab.
///
/// Keep this intentionally small: this is not an editor/debug surface, it is
/// the phone-friendly player-facing overlay that mirrors the Zelda-style
/// left/right page mental model.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum InventoryTab {
    #[default]
    Items,
    Map,
    Quests,
}

impl InventoryTab {
    pub const ALL: [Self; 3] = [Self::Items, Self::Map, Self::Quests];

    pub fn label(self) -> &'static str {
        match self {
            Self::Items => "Items",
            Self::Map => "Map",
            Self::Quests => "Quests",
        }
    }

    fn index(self) -> usize {
        Self::ALL
            .iter()
            .position(|tab| *tab == self)
            .expect("InventoryTab::ALL contains every tab")
    }

    fn from_index(index: usize) -> Self {
        Self::ALL[index % Self::ALL.len()]
    }

    fn next(self) -> Self {
        Self::from_index(self.index() + 1)
    }

    fn previous(self) -> Self {
        Self::from_index((self.index() + Self::ALL.len() - 1) % Self::ALL.len())
    }
}

/// Counted-item bag.
#[derive(Resource, Default, Clone)]
pub struct PlayerInventory {
    counts: [u32; 3],
}

impl PlayerInventory {
    fn slot(kind: ItemKind) -> usize {
        match kind {
            ItemKind::HealthPotion => 0,
            ItemKind::SpareBattery => 1,
            ItemKind::DataChip => 2,
        }
    }

    pub fn count(&self, kind: ItemKind) -> u32 {
        self.counts[Self::slot(kind)]
    }

    pub fn add(&mut self, kind: ItemKind, n: u32) {
        self.counts[Self::slot(kind)] = self.counts[Self::slot(kind)].saturating_add(n);
    }

    pub fn remove(&mut self, kind: ItemKind, n: u32) -> u32 {
        let slot = &mut self.counts[Self::slot(kind)];
        let removed = (*slot).min(n);
        *slot -= removed;
        removed
    }

    pub fn entries(&self) -> impl Iterator<Item = (ItemKind, u32)> + '_ {
        ItemKind::ALL
            .into_iter()
            .map(move |kind| (kind, self.count(kind)))
    }

    pub fn total_items(&self) -> u32 {
        self.counts.iter().sum()
    }

    /// Seed with one of each item so the menu has something to show on a
    /// fresh run. The data model can later swap this for save-game restore.
    pub fn starter() -> Self {
        let mut bag = Self::default();
        bag.add(ItemKind::HealthPotion, 2);
        bag.add(ItemKind::SpareBattery, 1);
        bag.add(ItemKind::DataChip, 1);
        bag
    }
}

#[derive(Resource, Default)]
pub struct InventoryUiState {
    pub visible: bool,
    pub selected: usize,
    pub tab: InventoryTab,
    /// Scroll offset for non-item text tabs. Items are short enough to remain
    /// fully visible for now; map/quest pages can grow as the world grows.
    pub content_scroll: usize,
    /// True when the inventory was opened from the pause menu (vs. directly
    /// from gameplay). Determines what mode to return to when it closes.
    pub opened_from_pause: bool,
    /// Set by the pointer system when a tap should activate the currently
    /// selected row. Consumed by `inventory_input` on the same frame and
    /// treated like a confirm press.
    pub pointer_confirm: bool,
    /// Tracks the row "armed" by a prior tap under tap-then-confirm modes.
    /// Cleared once the user taps it again or moves away.
    pub pointer_armed: Option<usize>,
}

impl InventoryUiState {
    fn reset_for_open(&mut self, opened_from_pause: bool) {
        self.visible = true;
        self.selected = 0;
        self.tab = InventoryTab::Items;
        self.content_scroll = 0;
        self.opened_from_pause = opened_from_pause;
        self.pointer_confirm = false;
        self.pointer_armed = None;
    }

    fn close(&mut self) {
        self.visible = false;
        self.pointer_confirm = false;
        self.pointer_armed = None;
    }

    fn set_tab(&mut self, tab: InventoryTab) {
        if self.tab != tab {
            self.tab = tab;
            self.selected = 0;
            self.content_scroll = 0;
            self.pointer_confirm = false;
            self.pointer_armed = None;
        }
    }

    fn next_tab(&mut self) {
        self.set_tab(self.tab.next());
    }

    fn previous_tab(&mut self) {
        self.set_tab(self.tab.previous());
    }
}

#[derive(Component)]
pub struct InventoryRoot;

#[derive(Component)]
pub struct InventoryTitleText;

#[derive(Component)]
pub struct InventoryTabButton {
    pub tab: InventoryTab,
}

#[derive(Component)]
pub struct InventoryBackButton;

#[derive(Component)]
pub struct InventoryItemRow {
    pub kind: ItemKind,
}

#[derive(Component)]
pub struct InventoryDescriptionText;

#[derive(Component)]
pub struct InventoryStatusText;

#[derive(Component)]
pub struct InventoryTabContentText;

#[cfg(feature = "input")]
pub fn inventory_input(
    menu: Res<MenuControlFrame>,
    mut state: ResMut<InventoryUiState>,
    mode: Res<State<GameMode>>,
    mut next_mode: ResMut<NextState<GameMode>>,
    mut inventory: ResMut<PlayerInventory>,
    mut runtime: ResMut<SandboxRuntime>,
) {
    // Toggle the adventure menu directly from gameplay using the semantic menu
    // frame. Keyboard/gamepad still feed this through the Inventory action;
    // touch can also reach the same panel through the pause menu.
    if menu.inventory {
        if state.visible {
            close_inventory(&mut state, mode.get(), &mut next_mode);
        } else if matches!(mode.get(), GameMode::Playing | GameMode::Paused) {
            state.reset_for_open(matches!(mode.get(), GameMode::Paused));
            if matches!(mode.get(), GameMode::Playing) {
                next_mode.set(GameMode::Paused);
            }
        }
    }

    if !state.visible {
        // Drop stale pointer signals so reopening does not auto-fire.
        state.pointer_confirm = false;
        state.pointer_armed = None;
        return;
    }

    if menu.back || menu.start {
        close_inventory(&mut state, mode.get(), &mut next_mode);
        return;
    }

    if menu.left {
        state.previous_tab();
    }
    if menu.right {
        state.next_tab();
    }

    match state.tab {
        InventoryTab::Items => {
            handle_item_tab_input(&menu, &mut state, &mut inventory, &mut runtime)
        }
        InventoryTab::Map | InventoryTab::Quests => handle_text_tab_input(&menu, &mut state),
    }
}

#[cfg(feature = "input")]
fn close_inventory(
    state: &mut InventoryUiState,
    mode: &GameMode,
    next_mode: &mut NextState<GameMode>,
) {
    let opened_from_pause = state.opened_from_pause;
    state.close();
    if !opened_from_pause && matches!(mode, GameMode::Paused) {
        next_mode.set(GameMode::Playing);
    }
}

#[cfg(feature = "input")]
fn handle_item_tab_input(
    menu: &MenuControlFrame,
    state: &mut InventoryUiState,
    inventory: &mut PlayerInventory,
    runtime: &mut SandboxRuntime,
) {
    let total = ItemKind::ALL.len();
    let mut nav_up = menu.up;
    let mut nav_down = menu.down;
    let steps = menu.vertical_scroll_steps();
    if steps > 0 {
        nav_up = true;
    } else if steps < 0 {
        nav_down = true;
    }
    if nav_up {
        state.selected = (state.selected + total - 1) % total;
    }
    if nav_down {
        state.selected = (state.selected + 1) % total;
    }
    // Keyboard / gamepad / gesture navigation clears any tap-armed row so the
    // next pointer press starts fresh.
    if nav_up || nav_down || menu.scroll_y.abs() >= 0.5 {
        state.pointer_armed = None;
    }

    let confirm = menu.select || state.pointer_confirm;
    state.pointer_confirm = false;
    if confirm {
        let kind = ItemKind::ALL[state.selected];
        if inventory.count(kind) > 0 {
            apply_item_effect(kind, inventory, runtime);
        }
    }
}

#[cfg(feature = "input")]
fn handle_text_tab_input(menu: &MenuControlFrame, state: &mut InventoryUiState) {
    let mut delta: isize = 0;
    if menu.up {
        delta -= 1;
    }
    if menu.down {
        delta += 1;
    }
    // Positive scroll_y means user moved content up / requested previous rows
    // in the MenuControlFrame convention used by pause menu navigation.
    delta -= menu.vertical_scroll_steps() as isize;
    if delta < 0 {
        state.content_scroll = state.content_scroll.saturating_sub((-delta) as usize);
    } else if delta > 0 {
        state.content_scroll = state.content_scroll.saturating_add(delta as usize).min(256);
    }
}

/// Mouse / touch input for the adventure-menu panel.
///
/// Touch-native tabs and Back are handled here, while item-row taps still route
/// through `MenuTapMode::resolve_press`. The keyboard/gamepad path remains in
/// `inventory_input`, so the UI can be operated without special raw-device
/// knowledge.
#[cfg(feature = "input")]
pub fn inventory_pointer_input(
    mode: Res<State<GameMode>>,
    mut next_mode: ResMut<NextState<GameMode>>,
    mut state: ResMut<InventoryUiState>,
    user_settings: Res<crate::settings::UserSettings>,
    rows: Query<(&Interaction, &InventoryItemRow), Changed<Interaction>>,
    tabs: Query<(&Interaction, &InventoryTabButton), Changed<Interaction>>,
    back_buttons: Query<&Interaction, (With<InventoryBackButton>, Changed<Interaction>)>,
) {
    if !state.visible {
        return;
    }

    for interaction in &back_buttons {
        if matches!(interaction, Interaction::Pressed) {
            close_inventory(&mut state, mode.get(), &mut next_mode);
            return;
        }
    }

    for (interaction, tab_button) in &tabs {
        if matches!(interaction, Interaction::Pressed) {
            state.set_tab(tab_button.tab);
            return;
        }
    }

    if state.tab != InventoryTab::Items {
        return;
    }

    let tap_mode = user_settings.controls.menu_tap_mode;
    let items = ItemKind::ALL;
    for (interaction, row) in &rows {
        let Some(index) = items.iter().position(|k| k == &row.kind) else {
            continue;
        };
        match interaction {
            Interaction::Hovered => {
                if state.selected != index {
                    state.selected = index;
                }
            }
            Interaction::Pressed => {
                let press =
                    tap_mode.resolve_press(index, state.selected, false, &mut state.pointer_armed);
                state.selected = index;
                if matches!(press, crate::settings::MenuPointerPress::Confirm) {
                    state.pointer_confirm = true;
                }
            }
            Interaction::None => {}
        }
    }
}

fn apply_item_effect(
    kind: ItemKind,
    inventory: &mut PlayerInventory,
    runtime: &mut SandboxRuntime,
) {
    match kind {
        ItemKind::HealthPotion => {
            if inventory.remove(ItemKind::HealthPotion, 1) > 0 {
                runtime.player_health.heal(2);
            }
        }
        ItemKind::SpareBattery | ItemKind::DataChip => {
            // Reserved for future effects; intentionally no-op for now.
        }
    }
}

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
        InventoryTab::Map => "Tap tabs or ←/→ pages   Drag to scroll   Back closes".to_string(),
        InventoryTab::Quests => "Tap tabs or ←/→ pages   Drag to scroll   Back closes".to_string(),
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
        for id in map.visited.iter().skip(state.content_scroll).take(10) {
            let marker = if id == current { "→" } else { " " };
            lines.push(format!("{marker} {id}"));
        }
    }
    if state.content_scroll > 0 {
        lines.push("↑ more".into());
    }
    if state.content_scroll + 10 < map.visited.len() {
        lines.push("↓ more".into());
    }
    lines.join("\n")
}

fn quest_tab_text(state: &InventoryUiState, quests: &crate::quest::QuestRegistry) -> String {
    let lines = quests.quest_log_lines();
    if lines.is_empty() {
        return "No active quests.".into();
    }
    let mut visible = Vec::new();
    for line in lines.iter().skip(state.content_scroll).take(9) {
        visible.push(format!("• {line}"));
    }
    if state.content_scroll > 0 {
        visible.insert(0, "↑ more".into());
    }
    if state.content_scroll + 9 < lines.len() {
        visible.push("↓ more".into());
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_accumulates_per_kind() {
        let mut bag = PlayerInventory::default();
        bag.add(ItemKind::HealthPotion, 2);
        bag.add(ItemKind::HealthPotion, 3);
        assert_eq!(bag.count(ItemKind::HealthPotion), 5);
        assert_eq!(bag.count(ItemKind::SpareBattery), 0);
    }

    #[test]
    fn remove_returns_actual_amount_removed() {
        let mut bag = PlayerInventory::default();
        bag.add(ItemKind::DataChip, 4);
        // Removing more than present clamps and returns the actual.
        assert_eq!(bag.remove(ItemKind::DataChip, 10), 4);
        assert_eq!(bag.count(ItemKind::DataChip), 0);
        // Removing from empty returns 0.
        assert_eq!(bag.remove(ItemKind::DataChip, 5), 0);
    }

    #[test]
    fn add_saturates_at_u32_max() {
        let mut bag = PlayerInventory::default();
        bag.add(ItemKind::HealthPotion, u32::MAX);
        bag.add(ItemKind::HealthPotion, 100); // should saturate, not overflow
        assert_eq!(bag.count(ItemKind::HealthPotion), u32::MAX);
    }

    #[test]
    fn entries_yields_all_kinds() {
        let mut bag = PlayerInventory::default();
        bag.add(ItemKind::HealthPotion, 2);
        bag.add(ItemKind::DataChip, 1);
        let entries: Vec<_> = bag.entries().collect();
        assert_eq!(entries.len(), 3); // every kind, even zero-count ones
        let map: std::collections::HashMap<_, _> = entries.into_iter().collect();
        assert_eq!(map[&ItemKind::HealthPotion], 2);
        assert_eq!(map[&ItemKind::SpareBattery], 0);
        assert_eq!(map[&ItemKind::DataChip], 1);
    }

    #[test]
    fn total_items_sums_across_kinds() {
        let mut bag = PlayerInventory::default();
        bag.add(ItemKind::HealthPotion, 2);
        bag.add(ItemKind::SpareBattery, 3);
        bag.add(ItemKind::DataChip, 4);
        assert_eq!(bag.total_items(), 9);
    }

    #[test]
    fn starter_seeds_each_kind() {
        let bag = PlayerInventory::starter();
        assert!(bag.count(ItemKind::HealthPotion) > 0);
        assert!(bag.count(ItemKind::SpareBattery) > 0);
        assert!(bag.count(ItemKind::DataChip) > 0);
        assert!(bag.total_items() >= 3);
    }

    /// Pin the implicit invariant that `ItemKind::ALL` covers every
    /// variant in order matching `PlayerInventory::slot`. Adding a
    /// new variant without updating both arrays silently breaks
    /// `entries()` / `total_items()` / the inventory UI.
    #[test]
    fn item_kind_all_matches_slot_count() {
        let mut bag = PlayerInventory::default();
        for (i, kind) in ItemKind::ALL.iter().copied().enumerate() {
            // Add a unique amount per kind so we can verify each
            // slot is independently addressable.
            bag.add(kind, (i + 1) as u32);
        }
        for (i, kind) in ItemKind::ALL.iter().copied().enumerate() {
            assert_eq!(
                bag.count(kind),
                (i + 1) as u32,
                "kind {kind:?} (index {i}) didn't round-trip independently",
            );
        }
    }

    #[test]
    fn item_kind_label_and_description_are_non_empty() {
        for kind in ItemKind::ALL {
            assert!(!kind.label().is_empty());
            assert!(!kind.description().is_empty());
        }
    }

    #[test]
    fn inventory_tabs_cycle_left_and_right() {
        assert_eq!(InventoryTab::Items.next(), InventoryTab::Map);
        assert_eq!(InventoryTab::Map.next(), InventoryTab::Quests);
        assert_eq!(InventoryTab::Quests.next(), InventoryTab::Items);
        assert_eq!(InventoryTab::Items.previous(), InventoryTab::Quests);
    }

    #[test]
    fn inventory_state_tab_change_resets_local_selection() {
        let mut state = InventoryUiState {
            visible: true,
            selected: 2,
            tab: InventoryTab::Items,
            content_scroll: 4,
            opened_from_pause: true,
            pointer_confirm: true,
            pointer_armed: Some(1),
        };
        state.set_tab(InventoryTab::Quests);
        assert_eq!(state.tab, InventoryTab::Quests);
        assert_eq!(state.selected, 0);
        assert_eq!(state.content_scroll, 0);
        assert!(!state.pointer_confirm);
        assert_eq!(state.pointer_armed, None);
    }
}

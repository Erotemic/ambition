//! Simple inventory model + overlay panel.
//!
//! The runtime owns a flat `PlayerInventory` resource (item kind → count).
//! The presentation side renders a small panel listing the stocked items
//! with a selection cursor; pressing `MenuConfirm` "uses" the selected
//! item — currently a no-op except for `HealthPotion`, which heals the
//! player by a fixed amount and decrements the stack.
//!
//! The inventory is intentionally minimal: the design door is open for
//! richer item effects (key/quest items, ability unlocks) without forcing
//! a schema decision right now.

use bevy::prelude::*;
#[cfg(feature = "input")]
use leafwing_input_manager::prelude::ActionState;

#[cfg(feature = "input")]
use crate::game_mode::GameMode;
#[cfg(feature = "input")]
use crate::input::SandboxAction;
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
    /// True when the inventory was opened from the pause menu (vs. directly
    /// from gameplay). Determines what mode to return to when it closes.
    pub opened_from_pause: bool,
}

#[derive(Component)]
pub struct InventoryRoot;

#[derive(Component)]
pub struct InventoryItemRow {
    pub kind: ItemKind,
}

#[derive(Component)]
pub struct InventoryDescriptionText;

#[derive(Component)]
pub struct InventoryStatusText;

#[cfg(feature = "input")]
pub fn inventory_input(
    action_state: Query<&ActionState<SandboxAction>>,
    mut state: ResMut<InventoryUiState>,
    mode: Res<State<GameMode>>,
    mut next_mode: ResMut<NextState<GameMode>>,
    mut inventory: ResMut<PlayerInventory>,
    mut runtime: ResMut<SandboxRuntime>,
) {
    let Ok(actions) = action_state.single() else {
        return;
    };

    // Toggle inventory directly from gameplay using `MenuToggle` is owned by
    // the pause menu; here we listen for the dedicated `Inventory` action so
    // `I` (or gamepad Y) opens the panel without going through pause. When
    // already open, the same key dismisses it.
    if actions.just_pressed(&SandboxAction::Inventory) {
        if state.visible {
            state.visible = false;
            // Direct opens (from gameplay) return to gameplay; opens from
            // the pause menu fall back to the pause menu.
            if !state.opened_from_pause && matches!(mode.get(), GameMode::Paused) {
                next_mode.set(GameMode::Playing);
            }
        } else if matches!(mode.get(), GameMode::Playing | GameMode::Paused) {
            state.visible = true;
            state.selected = 0;
            state.opened_from_pause = matches!(mode.get(), GameMode::Paused);
            if matches!(mode.get(), GameMode::Playing) {
                next_mode.set(GameMode::Paused);
            }
        }
    }

    if !state.visible {
        return;
    }

    let total = ItemKind::ALL.len();
    if actions.just_pressed(&SandboxAction::MoveUp) {
        state.selected = (state.selected + total - 1) % total;
    }
    if actions.just_pressed(&SandboxAction::MoveDown) {
        state.selected = (state.selected + 1) % total;
    }

    if actions.just_pressed(&SandboxAction::Jump) {
        let kind = ItemKind::ALL[state.selected];
        if inventory.count(kind) > 0 {
            apply_item_effect(kind, &mut inventory, &mut runtime);
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
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.03, 0.06, 0.84)),
            ZIndex(60),
            Visibility::Hidden,
            InventoryRoot,
            Name::new("Inventory root"),
        ))
        .id();

    let panel = commands
        .spawn((
            Node {
                width: Val::Px(440.0),
                padding: UiRect::all(Val::Px(24.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(10.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.08, 0.10, 0.16, 0.96)),
            Name::new("Inventory panel"),
        ))
        .id();
    commands.entity(root).add_child(panel);

    let title = commands
        .spawn((
            Text::new("Inventory"),
            TextFont {
                font_size: 22.0,
                ..default()
            },
            TextColor(Color::srgba(0.92, 0.96, 1.0, 0.98)),
            Name::new("Inventory title"),
        ))
        .id();
    commands.entity(panel).add_child(title);

    for kind in ItemKind::ALL {
        let row = commands
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                    justify_content: JustifyContent::SpaceBetween,
                    ..default()
                },
                BackgroundColor(Color::NONE),
                Text::new(""),
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
                TextColor(Color::srgba(0.82, 0.92, 1.0, 0.96)),
                InventoryItemRow { kind },
                Name::new(format!("Inventory row: {}", kind.label())),
            ))
            .id();
        commands.entity(panel).add_child(row);
    }

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

    let status = commands
        .spawn((
            Node {
                margin: UiRect::top(Val::Px(4.0)),
                ..default()
            },
            Text::new("[Up/Down] select  [Enter] use  [Esc] close"),
            TextFont {
                font_size: 12.0,
                ..default()
            },
            TextColor(Color::srgba(0.58, 0.70, 0.86, 0.86)),
            InventoryStatusText,
            Name::new("Inventory status"),
        ))
        .id();
    commands.entity(panel).add_child(status);
}

pub fn sync_inventory_panel(
    state: Res<InventoryUiState>,
    inventory: Res<PlayerInventory>,
    mut roots: Query<&mut Visibility, With<InventoryRoot>>,
    mut rows: Query<(
        &InventoryItemRow,
        &mut Text,
        &mut TextColor,
        &mut BackgroundColor,
    )>,
    mut descriptions: Query<
        &mut Text,
        (
            With<InventoryDescriptionText>,
            Without<InventoryItemRow>,
            Without<InventoryStatusText>,
        ),
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
    for (row, mut text, mut color, mut bg) in &mut rows {
        let count = inventory.count(row.kind);
        **text = format!("{:<20} x {}", row.kind.label(), count);
        let is_selected = Some(row.kind) == selected_kind;
        *color = if is_selected {
            TextColor(Color::srgba(0.18, 0.06, 0.04, 1.0))
        } else if count == 0 {
            TextColor(Color::srgba(0.50, 0.56, 0.66, 0.86))
        } else {
            TextColor(Color::srgba(0.82, 0.92, 1.0, 0.96))
        };
        *bg = if is_selected {
            BackgroundColor(Color::srgba(0.95, 0.78, 0.32, 0.96))
        } else {
            BackgroundColor(Color::NONE)
        };
    }
    if let Some(kind) = selected_kind {
        if let Ok(mut text) = descriptions.single_mut() {
            **text = if inventory.count(kind) == 0 {
                format!("{} — empty", kind.label())
            } else {
                kind.description().to_string()
            };
        }
    }
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
}

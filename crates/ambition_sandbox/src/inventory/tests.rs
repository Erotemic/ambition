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

//! Unit tests for the 24-item catalog: grid/index alignment, the `ItemMeta`
//! table layout, dialog/held-item id round-trips, and `OwnedItems` grant/take.

use super::*;

#[test]
fn catalog_has_exactly_one_grid_of_items() {
    assert_eq!(ITEM_COUNT, 24);
    assert_eq!(Item::ALL.len(), ITEM_COUNT);
    assert_eq!(ITEM_GRID_COLS * ITEM_GRID_ROWS, ITEM_COUNT);
}

#[test]
fn index_and_grid_round_trip_for_every_slot() {
    for (i, item) in Item::ALL.into_iter().enumerate() {
        assert_eq!(item.index(), i, "discriminant order == grid order");
        assert_eq!(Item::from_index(i), Some(item));
        let (row, col) = item.grid_pos();
        assert!(row < ITEM_GRID_ROWS && col < ITEM_GRID_COLS);
        assert_eq!(Item::from_grid(row, col), Some(item));
    }
    assert_eq!(Item::from_index(ITEM_COUNT), None);
    assert_eq!(Item::from_grid(ITEM_GRID_ROWS, 0), None);
    assert_eq!(Item::from_grid(0, ITEM_GRID_COLS), None);
}

#[test]
fn item_meta_table_is_index_aligned() {
    // Refactor 1: ITEM_META rows must line up with the Item discriminants.
    // A whole-row shift self-consistently passes the dialog round-trip (the
    // moved row carries its own dialog_id), so pin the variant→name mapping
    // by NAME here — referencing each variant directly, not by index.
    let expected: [(Item, &str); ITEM_COUNT] = [
        (Item::PortalGun, "Portal Gun"),
        (Item::Axe, "Axe"),
        (Item::Javelin, "Javelin"),
        (Item::GunSword, "Gun-Sword"),
        (Item::PuppySlugGun, "Puppy-Slug Gun"),
        (Item::Fireball, "Fireball"),
        (Item::Blink, "Blink"),
        (Item::Fly, "Flight"),
        (Item::Grapple, "Grapple Hook"),
        (Item::MorphBall, "Morph Ball"),
        (Item::MarkRecall, "Mark / Recall"),
        (Item::BubbleShield, "Bubble Shield"),
        (Item::HealthCell, "Health Cell"),
        (Item::ManaCell, "Mana Cell"),
        (Item::SpareBattery, "Spare Battery"),
        (Item::DataChip, "Data Chip"),
        (Item::Bomb, "Bomb"),
        (Item::GoldPouch, "Gold Pouch"),
        (Item::MapFragment, "Map Fragment"),
        (Item::SealedNote, "Sealed Note"),
        (Item::FieldSurvey, "Field Survey"),
        (Item::GateKey, "Gate Key"),
        (Item::DebugLens, "Debug Lens"),
        (Item::ReservedSlot, "—"),
    ];
    for (item, name) in expected {
        assert_eq!(item.display_name(), name, "row for {item:?} is misaligned");
    }
    // Category + held-item anchors that would also catch a shift.
    assert_eq!(Item::Fireball.category(), ItemCategory::Ability);
    assert_eq!(Item::Bomb.category(), ItemCategory::Weapon);
    assert_eq!(Item::ReservedSlot.category(), ItemCategory::Reserved);
}

#[test]
fn held_item_ids_round_trip_for_every_wired_item() {
    for item in Item::ALL {
        if let Some(id) = item.held_item_id() {
            assert_eq!(
                Item::from_held_item_id(id),
                Some(item),
                "held_item_id {id} should resolve back to {item:?}"
            );
        }
    }
    // The wired set is exactly these nine (everything else is None).
    let wired: Vec<Item> = Item::ALL
        .into_iter()
        .filter(|i| i.held_item_id().is_some())
        .collect();
    assert_eq!(wired.len(), 9, "wired held-items: {wired:?}");
}

#[test]
fn dialog_ids_are_unique_and_round_trip() {
    let mut seen = std::collections::HashSet::new();
    for item in Item::ALL {
        assert!(
            seen.insert(item.dialog_id()),
            "duplicate dialog id {item:?}"
        );
        assert_eq!(Item::from_dialog_id(item.dialog_id()), Some(item));
    }
    // Loose spellings normalize.
    assert_eq!(Item::from_dialog_id("Portal Gun"), Some(Item::PortalGun));
    assert_eq!(Item::from_dialog_id("gun_sword"), Some(Item::GunSword));
    assert_eq!(Item::from_dialog_id("nonsense"), None);
}

#[test]
fn legacy_health_alias_resolves_to_health_cell() {
    // The old 3-kind bag spelled the health consumable "healthpotion"; the
    // catalog id is "healthcell". `from_dialog_id` keeps the alias resolving,
    // and `legacy_dialog_alias` reports it (only HealthCell diverges).
    assert_eq!(Item::from_dialog_id("healthpotion"), Some(Item::HealthCell));
    assert_eq!(
        Item::from_dialog_id("health_potion"),
        Some(Item::HealthCell)
    );
    assert_eq!(Item::from_dialog_id("healthcell"), Some(Item::HealthCell));
    assert_eq!(Item::HealthCell.legacy_dialog_alias(), Some("healthpotion"));
    // The other two overlapping items already share their ids — no alias.
    assert_eq!(Item::SpareBattery.legacy_dialog_alias(), None);
    assert_eq!(Item::DataChip.legacy_dialog_alias(), None);
}

#[test]
fn grant_clamps_unique_but_stacks_consumables() {
    let mut owned = OwnedItems::default();
    // Unique weapon clamps at 1.
    owned.grant(Item::PortalGun, 5);
    assert_eq!(owned.count(Item::PortalGun), 1);
    owned.grant(Item::PortalGun, 3);
    assert_eq!(owned.count(Item::PortalGun), 1);
    // Consumable stacks.
    owned.grant(Item::HealthCell, 2);
    owned.grant(Item::HealthCell, 3);
    assert_eq!(owned.count(Item::HealthCell), 5);
    assert_eq!(owned.take(Item::HealthCell, 4), 4);
    assert_eq!(owned.count(Item::HealthCell), 1);
    assert_eq!(
        owned.take(Item::HealthCell, 9),
        1,
        "take floors and reports actual"
    );
}

#[test]
fn equip_toggle_tracks_the_active_weapon() {
    let mut owned = OwnedItems::default();
    assert_eq!(owned.equipped(), None);
    owned.set_equipped(Some(Item::Axe));
    assert!(owned.is_equipped(Item::Axe));
    assert!(!owned.is_equipped(Item::GunSword));
    owned.set_equipped(None);
    assert_eq!(owned.equipped(), None);
}

#[test]
fn held_item_ids_point_at_real_specs() {
    // The three wired weapons resolve to a known held-item spec.
    assert_eq!(Item::Axe.held_item_id(), Some("axe"));
    assert_eq!(Item::GunSword.held_item_id(), Some("gun_sword"));
    assert!(ambition_characters::brain::held_item_by_id("gun_sword").is_some());
}

#[test]
fn held_item_id_round_trips_through_reverse_lookup() {
    for item in Item::ALL {
        if let Some(id) = item.held_item_id() {
            assert_eq!(
                Item::from_held_item_id(id),
                Some(item),
                "ground-item id {id} maps back to its slot"
            );
        }
    }
    assert_eq!(Item::from_held_item_id("axe"), Some(Item::Axe));
    assert_eq!(Item::from_held_item_id("gun_sword"), Some(Item::GunSword));
    assert_eq!(Item::from_held_item_id("nonexistent"), None);
}

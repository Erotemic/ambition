//! Unit tests for the 24-item catalog: grid/index alignment, the `ItemMeta`
//! table layout, dialog/held-item id round-trips, and `OwnedItems` grant/take.

use super::*;

/// Ambition's shipped catalog (`items.ron`), the fixture.
fn shipped() -> &'static crate::ItemCatalog {
    crate::test_item_catalog()
}

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
    // The catalog rows must line up with the Item discriminants.
    // A whole-row shift self-consistently passes the dialog round-trip (the
    // moved row carries its own dialog_id), so pin the variant→name mapping
    // by NAME here — referencing each variant directly, not by index.
    let expected: [(Item, &str); ITEM_COUNT] = [
        (Item::Fireball, "Fireball"),
        (Item::EyeLaser, "Eye Laser"),
        (Item::Lightning, "Lightning"),
        (Item::SwordStance, "Sword Stance"),
        (Item::AxeStance, "Axe Stance"),
        (Item::WhipStance, "Whip Stance"),
        (Item::PrecisionBlink, "Precision Blink"),
        (Item::ContextualBlink, "Contextual Blink"),
        (Item::Dash, "Dash"),
        (Item::DashTwo, "Dash II"),
        (Item::Grapple, "Grapple Hook"),
        (Item::MarkRecall, "Mark / Recall"),
        (Item::PortalGun, "Portal Gun"),
        (Item::Javelin, "Javelin"),
        (Item::GunSword, "Gun-Sword"),
        (Item::PuppySlugGun, "Puppy-Slug Gun"),
        (Item::Bomb, "Bomb"),
        (Item::Empty1, "Empty"),
        (Item::HealthCell, "Health Cell"),
        (Item::ManaCell, "Mana Cell"),
        (Item::SpareBattery, "Spare Battery"),
        (Item::SealedNote, "Sealed Note"),
        (Item::FieldSurvey, "Field Survey"),
        (Item::Empty2, "Empty"),
    ];
    for (item, name) in expected {
        assert_eq!(shipped().display_name(item), Some(name), "row for {item:?} is misaligned");
    }
    // Category + held-item anchors that would also catch a shift.
    assert_eq!(shipped().category(Item::Fireball), Some(ItemCategory::Ability));
    assert_eq!(shipped().category(Item::Bomb), Some(ItemCategory::Held));
    assert_eq!(shipped().category(Item::FieldSurvey), Some(ItemCategory::KeyItem));
}

#[test]
fn held_item_ids_round_trip_for_every_wired_item() {
    for item in Item::ALL {
        if let Some(id) = shipped().held_item_id(item) {
            assert_eq!(
                shipped().item_by_held_item_id(id),
                Some(item),
                "held_item_id {id} should resolve back to {item:?}"
            );
        }
    }
    // The wired set is exactly these six (everything else is None).
    let wired: Vec<Item> = Item::ALL
        .into_iter()
        .filter(|i| shipped().held_item_id(*i).is_some())
        .collect();
    assert_eq!(wired.len(), 6, "wired held-items: {wired:?}");
}

#[test]
fn dialog_ids_are_unique_and_round_trip() {
    let mut seen = std::collections::HashSet::new();
    for item in Item::ALL {
        assert!(
            seen.insert(shipped().dialog_id(item)),
            "duplicate dialog id {item:?}"
        );
        assert_eq!(shipped().item_by_dialog_id(shipped().dialog_id(item).unwrap()), Some(item));
    }
    // Loose spellings normalize.
    assert_eq!(shipped().item_by_dialog_id("Portal Gun"), Some(Item::PortalGun));
    assert_eq!(shipped().item_by_dialog_id("gun_sword"), Some(Item::GunSword));
    assert_eq!(shipped().item_by_dialog_id("nonsense"), None);
}

#[test]
fn legacy_health_alias_resolves_to_health_cell() {
    // The old 3-kind bag spelled the health consumable "healthpotion"; the catalog id is
    // "healthcell".
    assert_eq!(shipped().item_by_dialog_id("healthpotion"), Some(Item::HealthCell));
    assert_eq!(
        shipped().item_by_dialog_id("health_potion"),
        Some(Item::HealthCell)
    );
    assert_eq!(shipped().item_by_dialog_id("healthcell"), Some(Item::HealthCell));
    // The other overlapping item already shares its id — no alias needed.
    assert_eq!(
        shipped().item_by_dialog_id("sparebattery"),
        Some(Item::SpareBattery)
    );
}

#[test]
fn grant_clamps_unique_but_stacks_consumables() {
    let mut owned = OwnedItems::default();
    // Unique weapon clamps at 1.
    owned.grant(shipped(), Item::PortalGun, 5);
    assert_eq!(owned.count(Item::PortalGun), 1);
    owned.grant(shipped(), Item::PortalGun, 3);
    assert_eq!(owned.count(Item::PortalGun), 1);
    // Consumable stacks.
    owned.grant(shipped(), Item::HealthCell, 2);
    owned.grant(shipped(), Item::HealthCell, 3);
    assert_eq!(owned.count(Item::HealthCell), 5);
    assert_eq!(owned.take(Item::HealthCell, 4), 4);
    assert_eq!(owned.count(Item::HealthCell), 1);
    assert_eq!(
        owned.take(Item::HealthCell, 9),
        1,
        "take floors and reports actual"
    );
}

/// A held item stacks when its row says so, up to the row's stack: the bomb
/// is a held item that the bag holds a stock of, and the gun-sword is not.
#[test]
fn a_held_bomb_stacks_to_its_row_and_a_held_axe_stays_unique() {
    let bombs = shipped().stack(Item::Bomb).expect("the bomb has a row");
    assert!(bombs > 1, "the bomb is a stack: {bombs}");
    assert!(shipped().stacks(Item::Bomb));
    assert!(!shipped().stacks(Item::GunSword), "a held gun-sword is unique");

    let mut owned = OwnedItems::default();
    owned.grant(shipped(), Item::Bomb, 3);
    owned.grant(shipped(), Item::Bomb, 2);
    assert_eq!(owned.count(Item::Bomb), 5, "grants add up");
    assert!(owned.has_room_for(shipped(), Item::Bomb));
    owned.grant(shipped(), Item::Bomb, 100);
    assert_eq!(owned.count(Item::Bomb), bombs, "a grant stops at the row's stack");
    assert!(!owned.has_room_for(shipped(), Item::Bomb), "a full stack takes no more");
    assert_eq!(owned.take(Item::Bomb, 1), 1);
    assert_eq!(owned.count(Item::Bomb), bombs - 1, "a take removes one");

    owned.grant(shipped(), Item::GunSword, 2);
    assert_eq!(owned.count(Item::GunSword), 1, "the gun-sword stays at one");
    assert!(!owned.has_room_for(shipped(), Item::GunSword));
}

/// A save written with more than a stack holds comes back at the stack.
#[test]
fn a_persisted_count_over_the_stack_loads_at_the_stack() {
    let bombs = shipped().stack(Item::Bomb).unwrap();
    let mut owned = OwnedItems::default();
    owned.apply_persisted(
        shipped(),
        &[ambition_persistence::save_data::PersistedItem::new("bomb", bombs + 5)],
    );
    assert_eq!(owned.count(Item::Bomb), bombs);
}

/// The demo's starting bag holds a stock of bombs.
#[test]
fn the_starter_bag_holds_a_stock_of_bombs() {
    assert_eq!(OwnedItems::starter(shipped()).count(Item::Bomb), crate::STARTER_BOMBS);
    assert!(crate::STARTER_BOMBS <= shipped().stack(Item::Bomb).unwrap());
}

/// The hand is a PROJECTION over the bag, never a row in it: a wielded weapon
/// with no stored copy still reads as owned, and putting it down leaves the
/// bag exactly as it was.
#[test]
fn the_hand_projects_over_the_bag_without_writing_it() {
    let owned = OwnedItems::default();
    let wielding = Inventory::new(&owned, Some(Item::GunSword));
    assert!(wielding.has(Item::GunSword));
    assert!(wielding.is_equipped(Item::GunSword));
    assert!(!wielding.is_equipped(Item::Javelin));
    assert_eq!(wielding.count(Item::GunSword), 1);
    assert_eq!(
        owned.count(Item::GunSword),
        0,
        "the bag never learned about the hand"
    );
    let empty_handed = Inventory::new(&owned, None);
    assert!(!empty_handed.has(Item::GunSword));
    assert_eq!(empty_handed.equipped(), None);
}

#[test]
fn held_item_ids_point_at_real_specs() {
    // The three wired weapons resolve to a known held-item spec.
    assert_eq!(shipped().held_item_id(Item::GunSword), Some("gun_sword"));
    assert_eq!(shipped().held_item_id(Item::GunSword), Some("gun_sword"));
    assert!(ambition_characters::brain::held_item_by_id("gun_sword").is_some());
}

#[test]
fn held_item_id_round_trips_through_reverse_lookup() {
    for item in Item::ALL {
        if let Some(id) = shipped().held_item_id(item) {
            assert_eq!(
                shipped().item_by_held_item_id(id),
                Some(item),
                "ground-item id {id} maps back to its slot"
            );
        }
    }
    assert_eq!(shipped().item_by_held_item_id("gun_sword"), Some(Item::GunSword));
    assert_eq!(shipped().item_by_held_item_id("gun_sword"), Some(Item::GunSword));
    assert_eq!(shipped().item_by_held_item_id("nonexistent"), None);
}

/// A catalog is its authored rows (Q104: there is no built-in table). A row is
/// the item of its slot, and a slot the rows do not reach is no item: nothing
/// is granted there.
#[test]
fn a_catalog_is_its_rows_and_a_slot_it_does_not_name_is_no_item() {
    let ron = r#"[
        (
            display_name: "Warp Cannon",
            description: "A second game's item, authored purely as data.",
            category: Held,
            held_item_id: Some("warp_cannon"),
            dialog_id: "warpcannon",
        ),
    ]"#;
    let catalog = ItemCatalog::from_ron(ron);
    let row = catalog.meta(Item::Fireball).expect("the first slot is authored");
    assert_eq!(row.display_name, "Warp Cannon");
    assert_eq!(row.held_item_id.as_deref(), Some("warp_cannon"));
    assert_eq!(catalog.meta(Item::GunSword), None, "a slot the rows do not reach is no item");
    let mut owned = OwnedItems::default();
    owned.grant(&catalog, Item::GunSword, 1);
    owned.grant(&catalog, Item::Fireball, 1);
    assert!(!owned.has(Item::GunSword), "an item the catalog does not name is not granted");
    assert!(owned.has(Item::Fireball), "control: the authored item is granted");
}

/// The variants of one exclusive slot are mutually exclusive, and the variant
/// whose row changes nothing is the body's own action: it is in effect until
/// another variant takes its slot.
#[test]
fn one_variant_of_a_slot_is_in_effect() {
    use ambition_characters::equipment::WornEquipment;
    let catalog = shipped();
    let fresh = EquippedItems::of(catalog, None, None);
    assert!(fresh.contains(Item::Fireball), "the body's own projectile is in effect");
    assert!(!fresh.contains(Item::EyeLaser));
    assert!(fresh.contains(Item::SwordStance) && fresh.contains(Item::Dash) && fresh.contains(Item::PrecisionBlink));
    assert!(!fresh.contains(Item::Lightning), "a placeholder is never in effect");

    let mut worn = WornEquipment::default();
    worn.equip(catalog.wears(Item::EyeLaser).unwrap().clone());
    let lasing = EquippedItems::of(catalog, None, Some(&worn));
    assert!(lasing.contains(Item::EyeLaser));
    assert!(!lasing.contains(Item::Fireball), "the laser took the projectile slot");
    assert!(lasing.contains(Item::SwordStance), "another slot is not touched");

    worn.equip(catalog.wears(Item::Fireball).unwrap().clone());
    assert_eq!(worn.rows.len(), 1, "equipping the fireball replaced the laser in its slot");
    let back = EquippedItems::of(catalog, Some(Item::GunSword), Some(&worn));
    assert!(back.contains(Item::Fireball) && !back.contains(Item::EyeLaser));
    assert!(back.contains(Item::GunSword), "the hand is equipped too");
}

/// The demo unlocks every real item (held items, consumables, and every
/// ability that has a row), and no placeholder, empty slot or key item: a key
/// item is a fact of a quest, not an unlock.
#[test]
fn the_starter_bag_unlocks_every_real_item() {
    let catalog = shipped();
    let owned = OwnedItems::starter(catalog);
    for item in Item::ALL {
        let quest_item = catalog.category(item) == Some(ItemCategory::KeyItem);
        assert_eq!(owned.has(item), catalog.is_real(item) && !quest_item, "{item:?}");
        if catalog.category(item) == Some(ItemCategory::Ability) {
            assert_eq!(catalog.is_real(item), catalog.wears(item).is_some(), "{item:?}");
        }
    }
    for item in [Item::EyeLaser, Item::PortalGun, Item::GunSword, Item::Javelin, Item::Grapple] {
        assert!(owned.has(item), "{item:?} is unlocked");
    }
    for placeholder in [Item::Lightning, Item::AxeStance, Item::WhipStance, Item::DashTwo, Item::ContextualBlink] {
        assert!(!owned.has(placeholder), "{placeholder:?} is a placeholder");
    }
    for quest_item in [Item::SealedNote, Item::FieldSurvey] {
        assert!(!owned.has(quest_item), "{quest_item:?} is given by the intro quest");
    }
    assert!(!owned.has(Item::Empty1) && !owned.has(Item::Empty2));
}

/// A save that lacks an unlock (made before the item was added, or while it
/// was in the hand) does not lock it for good: the load gives each unique
/// ability and held item of the starting bag back. A key item and a stack
/// stay as the save says, and an item whose object the save holds is not
/// given a second time.
#[test]
fn a_load_keeps_the_unlocks_of_the_starting_bag() {
    let catalog = shipped();
    let starting = OwnedItems::starter(catalog);
    let mut loaded = OwnedItems::default();
    loaded.grant(catalog, Item::HealthCell, 1);
    let javelin = catalog.held_item_id(Item::Javelin).expect("the javelin is a held item");
    loaded.keep_unlocks(catalog, &starting, |held| held == javelin);

    for item in [Item::EyeLaser, Item::Fireball, Item::GunSword, Item::PortalGun, Item::Grapple] {
        assert!(loaded.has(item), "{item:?} is unlocked again");
    }
    assert!(!loaded.has(Item::Javelin), "the save holds the javelin's object: no second one");
    assert!(!loaded.has(Item::SealedNote), "a key item stays as the save says");
    assert_eq!(loaded.count(Item::Bomb), 0, "a stack stays as the save says");
    assert_eq!(loaded.count(Item::HealthCell), 1, "a stack stays as the save says");
    assert!(!loaded.has(Item::AxeStance), "a placeholder is not in the starting bag");
}

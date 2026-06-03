//! Canonical finite item catalog — the game's complete set of pickup items.
//!
//! Jon's design call (2026-06-03): the inventory menu is modeled on the
//! Ocarina-of-Time "Select Item" subscreen, which is a **6 × 4 = 24-slot grid**
//! (`submodules/ambition_inventory_ui/DESIGN-OOT-DEMO.md`: "The Items page uses a
//! 6 × 4 item grid based on OoT's inventory slot order"). That slot count is not
//! just a UI detail — **24 is the finite number of distinct pickup items in this
//! game.** Every collectible/equippable/usable item the player can ever hold maps
//! to exactly one of these 24 slots, in a fixed grid order.
//!
//! This module is the source of truth for that set. It is deliberately
//! presentation-independent: the OoT grid menu ([`crate::oot_menu`], behind the
//! `oot_inventory` feature) renders it, but pickups, dialogue (`<<give_item>>` /
//! `inventory_has`), and the equip path all read/write [`OwnedItems`] here. The
//! menu can be cut without touching this catalog.
//!
//! Some slots map to systems that already exist (portal gun, axe, javelin,
//! gun-sword, fireball, bubble shield, health/mana cells, the legacy
//! [`crate::inventory::ItemKind`] bag). Others are reserved placeholders for
//! planned items (puppy-slug gun, grapple, morph ball, bombs, the Alice/Bob
//! cartography key items) — they still occupy a real, stable slot so the grid
//! shows "every item you could ever have," OoT-style, with un-acquired entries
//! dimmed.

use bevy::prelude::Resource;

/// Number of item slots — the OoT item subscreen's 6 × 4 grid, and therefore the
/// total number of distinct pickup items in the game.
pub const ITEM_GRID_COLS: usize = 6;
pub const ITEM_GRID_ROWS: usize = 4;
pub const ITEM_COUNT: usize = ITEM_GRID_COLS * ITEM_GRID_ROWS; // 24

/// Broad behavior class for an item. Drives how the menu's confirm action treats
/// the slot and how the slot reads ("Equip" vs "Use" vs key item).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ItemCategory {
    /// An equippable weapon/tool that grants an `ActionSet` via a `HeldItem`
    /// (portal gun, axe, gun-sword, …). Confirm = equip/unequip.
    Weapon,
    /// A movement/utility ability (blink, fly, morph ball, …). Owned = available;
    /// confirm currently just inspects (real toggle wiring is a follow-up).
    Ability,
    /// A stackable consumable (health/mana cell, bomb, …). Confirm = use one.
    Consumable,
    /// A unique quest/key item (map fragment, sealed note, gate key, …). Owned
    /// flag only; not directly usable from the grid.
    KeyItem,
    /// A reserved slot kept to hold the grid's shape — a future item lands here.
    Reserved,
}

impl ItemCategory {
    /// True for items the player holds at most one of (the menu shows them as a
    /// bright/dim icon rather than a count badge).
    pub fn is_unique(self) -> bool {
        !matches!(self, Self::Consumable)
    }
}

/// The complete, finite set of pickup items, in OoT grid order (row-major:
/// slot index = `row * 6 + col`). The discriminant order **is** the grid order;
/// do not reorder without updating saves/authoring that key off the index.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(usize)]
pub enum Item {
    // Row 1 — offensive held tools.
    PortalGun = 0,
    Axe = 1,
    Javelin = 2,
    GunSword = 3,
    PuppySlugGun = 4,
    Fireball = 5,
    // Row 2 — movement / utility abilities (the "math theorem" verbs).
    Blink = 6,
    Fly = 7,
    Grapple = 8,
    MorphBall = 9,
    MarkRecall = 10,
    BubbleShield = 11,
    // Row 3 — consumables & resources.
    HealthCell = 12,
    ManaCell = 13,
    SpareBattery = 14,
    DataChip = 15,
    Bomb = 16,
    GoldPouch = 17,
    // Row 4 — key / quest items.
    MapFragment = 18,
    SealedNote = 19,
    FieldSurvey = 20,
    GateKey = 21,
    DebugLens = 22,
    ReservedSlot = 23,
}

impl Item {
    /// All 24 items in grid order. The compile-time length check below pins the
    /// catalog to exactly [`ITEM_COUNT`].
    pub const ALL: [Item; ITEM_COUNT] = [
        Item::PortalGun,
        Item::Axe,
        Item::Javelin,
        Item::GunSword,
        Item::PuppySlugGun,
        Item::Fireball,
        Item::Blink,
        Item::Fly,
        Item::Grapple,
        Item::MorphBall,
        Item::MarkRecall,
        Item::BubbleShield,
        Item::HealthCell,
        Item::ManaCell,
        Item::SpareBattery,
        Item::DataChip,
        Item::Bomb,
        Item::GoldPouch,
        Item::MapFragment,
        Item::SealedNote,
        Item::FieldSurvey,
        Item::GateKey,
        Item::DebugLens,
        Item::ReservedSlot,
    ];

    /// Grid slot index 0..24 (row-major). Equal to the enum discriminant.
    pub fn index(self) -> usize {
        self as usize
    }

    /// `(row, col)` position in the 6×4 grid.
    pub fn grid_pos(self) -> (usize, usize) {
        let i = self.index();
        (i / ITEM_GRID_COLS, i % ITEM_GRID_COLS)
    }

    /// Resolve a slot index back to an item.
    pub fn from_index(index: usize) -> Option<Item> {
        Item::ALL.get(index).copied()
    }

    /// Resolve the item at `(row, col)`, if in bounds.
    pub fn from_grid(row: usize, col: usize) -> Option<Item> {
        if row >= ITEM_GRID_ROWS || col >= ITEM_GRID_COLS {
            return None;
        }
        Item::from_index(row * ITEM_GRID_COLS + col)
    }

    pub fn category(self) -> ItemCategory {
        use Item::*;
        use ItemCategory::*;
        match self {
            PortalGun | Axe | Javelin | GunSword | PuppySlugGun | Bomb => Weapon,
            Fireball | Blink | Fly | Grapple | MorphBall | MarkRecall | BubbleShield => Ability,
            HealthCell | ManaCell | GoldPouch => Consumable,
            SpareBattery | DataChip => Consumable,
            MapFragment | SealedNote | FieldSurvey | GateKey | DebugLens => KeyItem,
            ReservedSlot => Reserved,
        }
    }

    pub fn display_name(self) -> &'static str {
        use Item::*;
        match self {
            PortalGun => "Portal Gun",
            Axe => "Axe",
            Javelin => "Javelin",
            GunSword => "Gun-Sword",
            PuppySlugGun => "Puppy-Slug Gun",
            Fireball => "Fireball",
            Blink => "Blink",
            Fly => "Flight",
            Grapple => "Grapple Hook",
            MorphBall => "Morph Ball",
            MarkRecall => "Mark / Recall",
            BubbleShield => "Bubble Shield",
            HealthCell => "Health Cell",
            ManaCell => "Mana Cell",
            SpareBattery => "Spare Battery",
            DataChip => "Data Chip",
            Bomb => "Bomb",
            GoldPouch => "Gold Pouch",
            MapFragment => "Map Fragment",
            SealedNote => "Sealed Note",
            FieldSurvey => "Field Survey",
            GateKey => "Gate Key",
            DebugLens => "Debug Lens",
            ReservedSlot => "—",
        }
    }

    pub fn description(self) -> &'static str {
        use Item::*;
        match self {
            PortalGun => "Fire a linked blue/orange portal pair. Carries momentum.",
            Axe => "A heavy pirate axe. Replaces your attack with a cleaving swing.",
            Javelin => "A throwing spear. Using it throws it.",
            GunSword => "A laser sword with a gun on it that shoots swords. Fires aimed bolts.",
            PuppySlugGun => "Fires friendly puppy slugs that harry your enemies. (planned)",
            Fireball => "A thrown bolt of fire. Spends mana.",
            Blink => "Short-range teleport. Your favorite, and high-skill.",
            Fly => "Sustained flight while you have charge.",
            Grapple => "Pull yourself toward anchor points. (planned)",
            MorphBall => "Compress into a ball to fit through gaps only an AI can take.",
            MarkRecall => "Drop marks and recall to them — travel and combat tool. (planned)",
            BubbleShield => "Raise a shield bubble; time it to parry.",
            HealthCell => "Restores health.",
            ManaCell => "Restores mana.",
            SpareBattery => "Reserved ability charge. Does nothing yet.",
            DataChip => "A lore fragment. Does nothing yet.",
            Bomb => "A thrown explosive — it detonates on a short fuse, blasting nearby enemies.",
            GoldPouch => "Loose currency. Spends as money.",
            MapFragment => "A piece of the world map, from Alice and Bob.",
            SealedNote => "Alice's sealed note — carry it to Bob.",
            FieldSurvey => "Bob's field survey of a nearby zone.",
            GateKey => "Opens a sealed dimension-gate door.",
            DebugLens => "See the seams of the world. For an AI, debug is a sense organ.",
            ReservedSlot => "An empty slot, waiting for an item that does not exist yet.",
        }
    }

    /// For [`ItemCategory::Weapon`] items, the `HeldItem` id whose `ActionSet` the
    /// player gains on equip (resolved via [`crate::brain::held_item_by_id`] or a
    /// dedicated `*_spec` in [`crate::item_pickup`]). `None` for non-equippables
    /// and for weapons whose held-item wiring is not built yet.
    pub fn held_item_id(self) -> Option<&'static str> {
        use Item::*;
        match self {
            Axe => Some("axe"),
            Javelin => Some("javelin"),
            GunSword => Some("gun_sword"),
            PuppySlugGun => Some("puppy_slug_gun"),
            Bomb => Some("bomb"),
            MarkRecall => Some("mark_recall"),
            Fireball => Some("fireball"),
            Blink => Some("blink"),
            Grapple => Some("grapple"),
            // PortalGun equips via its own `PortalGun` component (handled
            // specially by the menu), not a HeldItemSpec.
            _ => None,
        }
    }

    /// Reverse of [`Self::held_item_id`]: which catalog slot a world held-item
    /// (`GroundItem`/`HeldItemSpec` id) corresponds to, so picking one up grants
    /// the right slot.
    pub fn from_held_item_id(id: &str) -> Option<Item> {
        Item::ALL.into_iter().find(|i| i.held_item_id() == Some(id))
    }

    /// Bridge to the legacy 3-kind [`crate::inventory::ItemKind`] bag so existing
    /// dialogue ids and the old menu keep resolving. Only the three overlapping
    /// items map; everything else is new and lives only in [`OwnedItems`].
    pub fn legacy_kind(self) -> Option<crate::inventory::ItemKind> {
        use crate::inventory::ItemKind;
        match self {
            Item::HealthCell => Some(ItemKind::HealthPotion),
            Item::SpareBattery => Some(ItemKind::SpareBattery),
            Item::DataChip => Some(ItemKind::DataChip),
            _ => None,
        }
    }

    pub fn from_legacy_kind(kind: crate::inventory::ItemKind) -> Item {
        use crate::inventory::ItemKind;
        match kind {
            ItemKind::HealthPotion => Item::HealthCell,
            ItemKind::SpareBattery => Item::SpareBattery,
            ItemKind::DataChip => Item::DataChip,
        }
    }

    /// Stable lowercase id for dialogue/authoring, e.g. `inventory_has("portal_gun")`.
    /// Normalized the same way the Yarn bindings normalize (lowercase, drop
    /// non-alphanumerics), so `"PortalGun"`, `"portal_gun"`, `"portal gun"` all
    /// resolve here.
    pub fn dialog_id(self) -> &'static str {
        use Item::*;
        match self {
            PortalGun => "portalgun",
            Axe => "axe",
            Javelin => "javelin",
            GunSword => "gunsword",
            PuppySlugGun => "puppysluggun",
            Fireball => "fireball",
            Blink => "blink",
            Fly => "fly",
            Grapple => "grapple",
            MorphBall => "morphball",
            MarkRecall => "markrecall",
            BubbleShield => "bubbleshield",
            HealthCell => "healthcell",
            ManaCell => "manacell",
            SpareBattery => "sparebattery",
            DataChip => "datachip",
            Bomb => "bomb",
            GoldPouch => "goldpouch",
            MapFragment => "mapfragment",
            SealedNote => "sealednote",
            FieldSurvey => "fieldsurvey",
            GateKey => "gatekey",
            DebugLens => "debuglens",
            ReservedSlot => "reservedslot",
        }
    }

    /// Normalize a raw authoring string the same way the Yarn bindings do, then
    /// resolve it. Also accepts the legacy [`crate::inventory::ItemKind`] dialog
    /// ids (`"healthpotion"` → [`Item::HealthCell`]) so old scripts keep working.
    pub fn from_dialog_id(raw: &str) -> Option<Item> {
        let key: String = raw
            .chars()
            .filter(|c| c.is_alphanumeric())
            .flat_map(|c| c.to_lowercase())
            .collect();
        if let Some(found) = Item::ALL.into_iter().find(|i| i.dialog_id() == key) {
            return Some(found);
        }
        // Fall back to the legacy 3-kind ids.
        crate::inventory::ItemKind::from_dialog_id(&key).map(Item::from_legacy_kind)
    }
}

/// Authoritative ownership of the 24 catalog items.
///
/// `counts[i]` is how many of [`Item::from_index(i)`] the player holds; for unique
/// items it is 0 or 1, for [`ItemCategory::Consumable`] it is a stack size.
/// `equipped` is the currently-equipped [`ItemCategory::Weapon`] slot, if any.
///
/// This is the single source of truth the OoT menu, pickups, dialogue, and the
/// equip path share. The legacy [`crate::inventory::PlayerInventory`] is kept in
/// sync one-way (here → there) for the three overlapping items so the old menu
/// still displays correct counts when the `oot_inventory` feature is off.
#[derive(Resource, Clone, Debug)]
pub struct OwnedItems {
    counts: [u32; ITEM_COUNT],
    equipped: Option<Item>,
}

impl Default for OwnedItems {
    fn default() -> Self {
        Self {
            counts: [0; ITEM_COUNT],
            equipped: None,
        }
    }
}

impl OwnedItems {
    pub fn count(&self, item: Item) -> u32 {
        self.counts[item.index()]
    }

    pub fn has(&self, item: Item) -> bool {
        self.counts[item.index()] > 0
    }

    /// Add `n` of an item. Unique items clamp at 1 so a second pickup doesn't
    /// inflate a non-stackable slot.
    pub fn grant(&mut self, item: Item, n: u32) {
        let slot = &mut self.counts[item.index()];
        let next = slot.saturating_add(n);
        *slot = if item.category().is_unique() {
            next.min(1)
        } else {
            next
        };
    }

    /// Remove up to `n`; returns how many were actually removed.
    pub fn take(&mut self, item: Item, n: u32) -> u32 {
        let slot = &mut self.counts[item.index()];
        let removed = (*slot).min(n);
        *slot -= removed;
        removed
    }

    pub fn equipped(&self) -> Option<Item> {
        self.equipped
    }

    pub fn is_equipped(&self, item: Item) -> bool {
        self.equipped == Some(item)
    }

    /// Mark a weapon slot equipped (does not itself attach the `HeldItem` — the
    /// menu effect system does that). Toggling the already-equipped item clears it.
    pub fn set_equipped(&mut self, item: Option<Item>) {
        self.equipped = item;
    }

    /// Seed a small starter set so a fresh sandbox run has something to show in
    /// the grid. Mirrors the legacy `PlayerInventory::starter` plus the items the
    /// sandbox debug-spawns (axe, gun-sword, portal gun).
    pub fn starter() -> Self {
        let mut owned = Self::default();
        owned.grant(Item::HealthCell, 3);
        owned.grant(Item::ManaCell, 2);
        owned.grant(Item::SpareBattery, 1);
        owned.grant(Item::DataChip, 1);
        owned.grant(Item::Fireball, 1);
        owned.grant(Item::BubbleShield, 1);
        owned.grant(Item::Blink, 1);
        owned
    }

    #[cfg(test)]
    pub fn total(&self) -> u32 {
        self.counts.iter().sum()
    }
}

#[cfg(test)]
mod tests {
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
    fn dialog_ids_are_unique_and_round_trip() {
        let mut seen = std::collections::HashSet::new();
        for item in Item::ALL {
            assert!(seen.insert(item.dialog_id()), "duplicate dialog id {item:?}");
            assert_eq!(Item::from_dialog_id(item.dialog_id()), Some(item));
        }
        // Loose spellings normalize.
        assert_eq!(Item::from_dialog_id("Portal Gun"), Some(Item::PortalGun));
        assert_eq!(Item::from_dialog_id("gun_sword"), Some(Item::GunSword));
        assert_eq!(Item::from_dialog_id("nonsense"), None);
    }

    #[test]
    fn legacy_kind_bridge_is_consistent_both_ways() {
        use crate::inventory::ItemKind;
        for kind in ItemKind::ALL {
            let item = Item::from_legacy_kind(kind);
            assert_eq!(item.legacy_kind(), Some(kind));
        }
        // Legacy dialog ids still resolve through the new catalog.
        assert_eq!(Item::from_dialog_id("healthpotion"), Some(Item::HealthCell));
        assert_eq!(Item::from_dialog_id("health_potion"), Some(Item::HealthCell));
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
        assert_eq!(owned.take(Item::HealthCell, 9), 1, "take floors and reports actual");
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
        assert!(crate::brain::held_item_by_id("gun_sword").is_some());
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
}

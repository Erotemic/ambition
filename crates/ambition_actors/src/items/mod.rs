//! Canonical finite item catalog — the game's complete set of pickup items.
//!
//! Jon's design call (2026-06-03): the inventory menu is modeled on the
//! Ocarina-of-Time "Select Item" subscreen, which is a **6 × 4 = 24-slot grid**
//! (`submodules/ambition_menu/DESIGN-OOT-DEMO.md`: "The Items page uses a
//! 6 × 4 item grid based on OoT's inventory slot order"). That slot count is not
//! just a UI detail — **24 is the finite number of distinct pickup items in this
//! game.** Every collectible/equippable/usable item the player can ever hold maps
//! to exactly one of these 24 slots, in a fixed grid order.
//!
//! This module is the source of truth for that set. It is deliberately
//! presentation-independent: the unified tabbed menu ([`crate::menu`]) renders
//! it, but pickups, dialogue (`<<give_item>>` /
//! `inventory_has`), and the equip path all read/write [`OwnedItems`] here. The
//! menu can be cut without touching this catalog.
//!
//! Some slots map to systems that already exist (portal gun, axe, javelin,
//! gun-sword, fireball, bubble shield, health/mana cells). Others are
//! reserved placeholders for
//! planned items (puppy-slug gun, grapple, morph ball, bombs, the Alice/Bob
//! cartography key items) — they still occupy a real, stable slot so the grid
//! shows "every item you could ever have," OoT-style, with un-acquired entries
//! dimmed.

pub mod persist;
pub mod pickup;
pub mod shop;

use bevy::prelude::Resource;

/// Number of item slots — the OoT item subscreen's 6 × 4 grid, and therefore the
/// total number of distinct pickup items in the game.
pub const ITEM_GRID_COLS: usize = 6;
pub const ITEM_GRID_ROWS: usize = 4;
pub const ITEM_COUNT: usize = ITEM_GRID_COLS * ITEM_GRID_ROWS; // 24

/// Broad behavior class for an item. Drives how the menu's confirm action treats
/// the slot and how the slot reads ("Equip" vs "Use" vs key item).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
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

/// Per-item metadata, one row per catalog slot (Refactor 1). Replaces the five
/// parallel 24-arm `match self` functions (`category` / `display_name` /
/// `description` / `held_item_id` / `dialog_id`), so
/// adding or renaming an item is **one row** here, not edits scattered across
/// several functions with nothing stopping a forgotten arm. Row order must
/// match the [`Item`] discriminants (pinned by `item_meta_table_is_index_aligned`).
///
/// **Owned + serde-authorable (C1):** the fields are owned + `serde`-round-trippable,
/// so a content game authors its item flavor/wiring as DATA in `items.ron`
/// (installed via [`install_item_catalog`], the [`ItemCatalog`] override) — the same
/// "content out of core" pattern as `boss_profiles.ron` / boss sheets (C6). The
/// engine's built-in 24 stay in [`ITEM_META`] as the byte-identical default.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ItemMeta {
    pub display_name: String,
    pub description: String,
    pub category: ItemCategory,
    /// `HeldItem` id granted on equip (`None` for non-equippables / unwired weapons).
    pub held_item_id: Option<String>,
    /// Stable lowercase authoring id (`inventory_has("portalgun")`).
    pub dialog_id: String,
}

/// Content-installed item CATALOG override (C1), mirroring
/// [`crate::boss_encounter::sprites::BossSheetRegistry`] / `BossProfileRegistry`.
/// A content game authors its item table in `items.ron` (a `Vec<ItemMeta>` in grid
/// order) and installs it via [`install_item_catalog`]; an installed row REPLACES
/// the built-in default for that grid slot. Absent rows (and no install) fall back
/// to the built-in [`ITEM_META`] — the E58/C6 "empty default = built-in" pattern,
/// so no core edit is needed to re-author item flavor.
#[derive(Clone, Debug, Default)]
pub struct ItemCatalog {
    rows: Vec<ItemMeta>,
}

impl ItemCatalog {
    /// Parse an item-catalog RON document (`[ItemMeta]` in grid order) — the
    /// content layer's install entry point.
    pub fn from_ron(ron: &str) -> Self {
        let rows = ron::from_str(ron).unwrap_or_else(|err| {
            panic!("items.ron failed to deserialize as Vec<ItemMeta>: {err}")
        });
        Self { rows }
    }

    fn row(&self, index: usize) -> Option<&ItemMeta> {
        self.rows.get(index)
    }
}

/// Content-installed item-catalog override. Set once at plugin-build time;
/// ADDITIVE per grid slot (the engine ships its own 24-item default table).
static ITEM_CATALOG_OVERRIDE: std::sync::OnceLock<ItemCatalog> = std::sync::OnceLock::new();

/// Install the authored item catalog — `ambition_content` calls this at
/// plugin-build time alongside the other roster installs.
pub fn install_item_catalog(catalog: ItemCatalog) {
    let _ = ITEM_CATALOG_OVERRIDE.set(catalog);
}

/// Resolve an item's metadata: the content-authored override row for `index` if one
/// was installed, else the built-in default. Both live behind a process-global, so
/// the borrow is effectively `'static`.
fn item_meta(index: usize) -> &'static ItemMeta {
    ITEM_CATALOG_OVERRIDE
        .get()
        .and_then(|c| c.row(index))
        .unwrap_or_else(|| &ITEM_META[index])
}

/// One row per [`Item`], in discriminant order — the engine's built-in default
/// item table. See [`ItemMeta`].
static ITEM_META: std::sync::LazyLock<[ItemMeta; ITEM_COUNT]> = std::sync::LazyLock::new(|| {
    use ItemCategory::*;
    [
        ItemMeta {
            display_name: "Portal Gun".into(),
            description: "Fire a linked blue/orange portal pair. Carries momentum.".into(),
            category: Weapon,
            held_item_id: None,
            dialog_id: "portalgun".into(),
        },
        ItemMeta {
            display_name: "Axe".into(),
            description: "A heavy pirate axe. Replaces your attack with a cleaving swing.".into(),
            category: Weapon,
            held_item_id: Some("axe".into()),
            dialog_id: "axe".into(),
        },
        ItemMeta {
            display_name: "Javelin".into(),
            description: "A throwing spear. Using it throws it.".into(),
            category: Weapon,
            held_item_id: Some("javelin".into()),
            dialog_id: "javelin".into(),
        },
        ItemMeta {
            display_name: "Gun-Sword".into(),
            description: "A laser sword with a gun on it that shoots swords. Fires aimed bolts."
                .into(),
            category: Weapon,
            held_item_id: Some("gun_sword".into()),
            dialog_id: "gunsword".into(),
        },
        ItemMeta {
            display_name: "Puppy-Slug Gun".into(),
            description: "Fires friendly puppy slugs that harry your enemies. (planned)".into(),
            category: Weapon,
            held_item_id: Some("puppy_slug_gun".into()),
            dialog_id: "puppysluggun".into(),
        },
        ItemMeta {
            display_name: "Fireball".into(),
            description: "A thrown bolt of fire. Spends mana.".into(),
            category: Ability,
            held_item_id: Some("fireball".into()),
            dialog_id: "fireball".into(),
        },
        ItemMeta {
            display_name: "Blink".into(),
            description: "Short-range teleport. Your favorite, and high-skill.".into(),
            category: Ability,
            held_item_id: Some("blink".into()),
            dialog_id: "blink".into(),
        },
        ItemMeta {
            display_name: "Flight".into(),
            description: "Sustained flight while you have charge.".into(),
            category: Ability,
            held_item_id: None,
            dialog_id: "fly".into(),
        },
        ItemMeta {
            display_name: "Grapple Hook".into(),
            description: "Pull yourself toward anchor points. (planned)".into(),
            category: Ability,
            held_item_id: Some("grapple".into()),
            dialog_id: "grapple".into(),
        },
        ItemMeta {
            display_name: "Morph Ball".into(),
            description: "Compress into a ball to fit through gaps only an AI can take.".into(),
            category: Ability,
            held_item_id: None,
            dialog_id: "morphball".into(),
        },
        ItemMeta {
            display_name: "Mark / Recall".into(),
            description: "Drop marks and recall to them — travel and combat tool. (planned)".into(),
            category: Ability,
            held_item_id: Some("mark_recall".into()),
            dialog_id: "markrecall".into(),
        },
        ItemMeta {
            display_name: "Bubble Shield".into(),
            description: "Raise a shield bubble; time it to parry.".into(),
            category: Ability,
            held_item_id: None,
            dialog_id: "bubbleshield".into(),
        },
        ItemMeta {
            display_name: "Health Cell".into(),
            description: "Restores health.".into(),
            category: Consumable,
            held_item_id: None,
            dialog_id: "healthcell".into(),
        },
        ItemMeta {
            display_name: "Mana Cell".into(),
            description: "Restores mana.".into(),
            category: Consumable,
            held_item_id: None,
            dialog_id: "manacell".into(),
        },
        ItemMeta {
            display_name: "Spare Battery".into(),
            description: "Reserved ability charge. Does nothing yet.".into(),
            category: Consumable,
            held_item_id: None,
            dialog_id: "sparebattery".into(),
        },
        ItemMeta {
            display_name: "Data Chip".into(),
            description: "A lore fragment. Does nothing yet.".into(),
            category: Consumable,
            held_item_id: None,
            dialog_id: "datachip".into(),
        },
        ItemMeta {
            display_name: "Bomb".into(),
            description:
                "A thrown explosive — it detonates on a short fuse, blasting nearby enemies.".into(),
            category: Weapon,
            held_item_id: Some("bomb".into()),
            dialog_id: "bomb".into(),
        },
        ItemMeta {
            display_name: "Gold Pouch".into(),
            description: "Loose currency. Spends as money.".into(),
            category: Consumable,
            held_item_id: None,
            dialog_id: "goldpouch".into(),
        },
        ItemMeta {
            display_name: "Map Fragment".into(),
            description: "A piece of the world map, from Alice and Bob.".into(),
            category: KeyItem,
            held_item_id: None,
            dialog_id: "mapfragment".into(),
        },
        ItemMeta {
            display_name: "Sealed Note".into(),
            description: "Alice's sealed note — carry it to Bob.".into(),
            category: KeyItem,
            held_item_id: None,
            dialog_id: "sealednote".into(),
        },
        ItemMeta {
            display_name: "Field Survey".into(),
            description: "Bob's field survey of a nearby zone.".into(),
            category: KeyItem,
            held_item_id: None,
            dialog_id: "fieldsurvey".into(),
        },
        ItemMeta {
            display_name: "Gate Key".into(),
            description: "Opens a sealed dimension-gate door.".into(),
            category: KeyItem,
            held_item_id: None,
            dialog_id: "gatekey".into(),
        },
        ItemMeta {
            display_name: "Debug Lens".into(),
            description: "See the seams of the world. For an AI, debug is a sense organ.".into(),
            category: KeyItem,
            held_item_id: None,
            dialog_id: "debuglens".into(),
        },
        ItemMeta {
            display_name: "—".into(),
            description: "An empty slot, waiting for an item that does not exist yet.".into(),
            category: Reserved,
            held_item_id: None,
            dialog_id: "reservedslot".into(),
        },
    ]
});

impl Item {
    /// This item's row — the content-installed override if present, else the
    /// built-in default (both `'static` behind a process-global).
    fn meta(self) -> &'static ItemMeta {
        item_meta(self.index())
    }

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
        self.meta().category
    }

    pub fn display_name(self) -> &'static str {
        self.meta().display_name.as_str()
    }

    pub fn description(self) -> &'static str {
        self.meta().description.as_str()
    }

    /// For [`ItemCategory::Weapon`] items, the `HeldItem` id whose `ActionSet` the
    /// player gains on equip (resolved via [`ambition_characters::brain::held_item_by_id`] or a
    /// dedicated `*_spec` in [`crate::items::pickup`]). `None` for non-equippables
    /// and for weapons whose held-item wiring is not built yet.
    pub fn held_item_id(self) -> Option<&'static str> {
        // PortalGun equips via its own `PortalGun` component (handled specially
        // by the menu), not a HeldItemSpec — so its row's `held_item_id` is None.
        self.meta().held_item_id.as_deref()
    }

    /// Asset path (relative to Bevy's asset root) of this item's icon sprite, if
    /// one already exists in `sprites/props/`. Items render this picture in the OoT
    /// cube's Items grid instead of their name; items with no authored sprite return
    /// `None` and fall back to the text label.
    ///
    /// The set of available sprites is the same `sprites/props/` art used for
    /// ground/held items ([`crate::items::pickup::ItemArt`] / `GAUNTLET_PROP_IDS`):
    /// the three physical weapons (axe/javelin/gunsword), the portal gun, and the
    /// abstract gauntlet abilities that have a generated icon. This is a deliberate,
    /// explicit map (not a derived lookup) so a missing sprite is a visible `None`
    /// here, not a silent runtime miss. Items with no art (Flight, Morph Ball,
    /// Bubble Shield, the cells/resources, and all the key/quest items) stay text.
    pub fn icon_path(self) -> Option<&'static str> {
        use Item::*;
        let path = match self {
            PortalGun => "sprites/props/portal_gun_blue.png",
            Axe => "sprites/props/axe.png",
            Javelin => "sprites/props/javelin.png",
            GunSword => "sprites/props/gunsword.png",
            PuppySlugGun => "sprites/props/gauntlet_puppy_slug_gun.png",
            Fireball => "sprites/props/gauntlet_fireball.png",
            Blink => "sprites/props/gauntlet_blink.png",
            Grapple => "sprites/props/gauntlet_grapple.png",
            MarkRecall => "sprites/props/gauntlet_mark_recall.png",
            Bomb => "sprites/props/gauntlet_bomb.png",
            // No authored sprite — these fall back to the text label in the grid.
            Fly | MorphBall | BubbleShield | HealthCell | ManaCell | SpareBattery | DataChip
            | GoldPouch | MapFragment | SealedNote | FieldSurvey | GateKey | DebugLens
            | ReservedSlot => return None,
        };
        Some(path)
    }

    /// Reverse of [`Self::held_item_id`]: which catalog slot a world held-item
    /// (`GroundItem`/`HeldItemSpec` id) corresponds to, so picking one up grants
    /// the right slot.
    pub fn from_held_item_id(id: &str) -> Option<Item> {
        Item::ALL.into_iter().find(|i| i.held_item_id() == Some(id))
    }

    /// Stable lowercase id for dialogue/authoring, e.g. `inventory_has("portal_gun")`.
    /// Normalized the same way the Yarn bindings normalize (lowercase, drop
    /// non-alphanumerics), so `"PortalGun"`, `"portal_gun"`, `"portal gun"` all
    /// resolve here.
    pub fn dialog_id(self) -> &'static str {
        self.meta().dialog_id.as_str()
    }

    /// Normalize a raw authoring string the same way the Yarn bindings do, then
    /// resolve it. Also accepts the legacy `"healthpotion"` alias →
    /// [`Item::HealthCell`] so old scripts keep working.
    pub fn from_dialog_id(raw: &str) -> Option<Item> {
        let key: String = raw
            .chars()
            .filter(|c| c.is_alphanumeric())
            .flat_map(|c| c.to_lowercase())
            .collect();
        if let Some(found) = Item::ALL.into_iter().find(|i| i.dialog_id() == key) {
            return Some(found);
        }
        // Legacy alias: the old 3-kind bag spelled the health consumable
        // "healthpotion"; the catalog id is "healthcell". (SpareBattery/DataChip
        // already share their ids, so this is the only divergent alias.)
        if key == "healthpotion" {
            return Some(Item::HealthCell);
        }
        None
    }

    /// The legacy bag's dialogue alias for this item, if it differs from
    /// [`Self::dialog_id`]. Only `HealthCell` (old "healthpotion") diverges;
    /// the yarn snapshot mirrors counts under this alias too so older scripts
    /// using `inventory_has("healthpotion")` keep resolving.
    pub fn legacy_dialog_alias(self) -> Option<&'static str> {
        match self {
            Item::HealthCell => Some("healthpotion"),
            _ => None,
        }
    }
}

/// Authoritative ownership of the 24 catalog items.
///
/// `counts[i]` is how many of [`Item::from_index(i)`] the player holds; for unique
/// items it is 0 or 1, for [`ItemCategory::Consumable`] it is a stack size.
/// `equipped` is the currently-equipped [`ItemCategory::Weapon`] slot, if any.
///
/// This is the single source of truth the unified menu, pickups, dialogue, and
/// the equip path share. (It replaced the legacy 3-kind `PlayerInventory` bag,
/// which was deleted once this catalog became the only item store.)
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
    /// the grid: the consumables (health/mana cells, battery, chip), a couple of
    /// starter abilities, plus the items the sandbox debug-spawns.
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

    /// Serialize the owned counts to the persisted save form (every item with a
    /// non-zero count, keyed by stable `dialog_id`). Equipped state is not
    /// persisted yet — re-equip from the grid on load (handoff).
    pub fn to_persisted(&self) -> Vec<ambition_persistence::save_data::PersistedItem> {
        Item::ALL
            .into_iter()
            .filter_map(|item| {
                let c = self.count(item);
                (c > 0).then(|| {
                    ambition_persistence::save_data::PersistedItem::new(item.dialog_id(), c)
                })
            })
            .collect()
    }

    /// Replace the owned counts from a persisted save (clears first, then grants
    /// each — so `grant`'s unique-item clamp still applies to a hand-edited save).
    /// Unknown ids (a catalog item removed since the save) are skipped.
    pub fn apply_persisted(&mut self, items: &[ambition_persistence::save_data::PersistedItem]) {
        *self = Self::default();
        for p in items {
            if let Some(item) = Item::from_dialog_id(&p.id) {
                self.grant(item, p.count);
            }
        }
    }
}

#[cfg(test)]
mod tests;

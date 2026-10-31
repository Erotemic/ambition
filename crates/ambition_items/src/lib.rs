//! Canonical 24-slot item catalog and owned-item state.

pub mod shop;

/// The `item_catalog` authored-content schema this capability owns. Behind the
/// `content_pack` feature: a game that never validates its content must not
/// link a compiler.
#[cfg(feature = "content_pack")]
pub mod content_schema;

use bevy::prelude::Resource;

/// Number of item slots — the OoT item subscreen's 6 × 4 grid, and therefore the
/// total number of distinct pickup items in the game.
pub const ITEM_GRID_COLS: usize = 6;
pub const ITEM_GRID_ROWS: usize = 4;
pub const ITEM_COUNT: usize = ITEM_GRID_COLS * ITEM_GRID_ROWS; // 24

/// Broad behavior class for an item. Drives how the menu's confirm action treats
/// the slot and how the slot reads ("Equip" vs "Use" vs key item).
///
/// A body equips an item in one of two ways, and the two are different facts:
///
/// * a [`Self::Held`] item is an object in the body's hand ([`Inventory::in_hand`]);
/// * an [`Self::Ability`] is a variant the body wears into an action slot
///   ([`ItemMeta::wears`]). Abilities whose rows name one exclusive slot are
///   mutually exclusive: to equip one replaces the other.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ItemCategory {
    /// An object the body holds: a weapon or tool that grants an `ActionSet` via
    /// a `HeldItem` (portal gun, axe, grapple gauntlet, …). Confirm = equip to
    /// the hand, or stow it.
    Held,
    /// An ability variant. Confirm = wear its [`ItemMeta::wears`] row. A row
    /// with an exclusive slot replaces the variant that held the slot. An
    /// ability with no row is a placeholder: nothing grants it and nothing
    /// equips it.
    Ability,
    /// A stackable consumable (health/mana cell, …). Confirm = use one.
    Consumable,
    /// A unique quest/key item (map fragment, sealed note, …). Owned flag only;
    /// not directly usable from the grid.
    KeyItem,
    /// A reserved slot kept to hold the grid's shape — a future item lands here.
    Reserved,
}

/// The complete, finite set of grid items, in grid order (row-major:
/// slot index = `row * 6 + col`). The discriminant order is the grid order;
/// do not reorder without updating `items.ron`. Saves and scripts name an item
/// by its `dialog_id`, not by its index.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(usize)]
pub enum Item {
    // Row 1 — combat ability variants: the projectile and the stance.
    Fireball = 0,
    EyeLaser = 1,
    Lightning = 2,
    SwordStance = 3,
    AxeStance = 4,
    WhipStance = 5,
    // Row 2 — movement ability variants, then the movement gauntlets.
    PrecisionBlink = 6,
    ContextualBlink = 7,
    Dash = 8,
    DashTwo = 9,
    Grapple = 10,
    MarkRecall = 11,
    // Row 3 — held weapons and tools.
    PortalGun = 12,
    Javelin = 13,
    GunSword = 14,
    PuppySlugGun = 15,
    Bomb = 16,
    /// An empty slot of the grid: no item. The Axe was here (removed
    /// 2026-10-30: nothing in the game gave it or used it).
    Empty1 = 17,
    // Row 4 — consumables, then key items.
    HealthCell = 18,
    ManaCell = 19,
    SpareBattery = 20,
    SealedNote = 21,
    FieldSurvey = 22,
    /// An empty slot of the grid: no item. The Map Fragment was here
    /// (removed 2026-10-30: nothing in the game gave it or used it).
    Empty2 = 23,
}

/// Per-item metadata, one row per catalog slot (Refactor 1). Replaces the five
/// parallel 24-arm `match self` functions (`category` / `display_name` /
/// `description` / `held_item_id` / `dialog_id`), so
/// adding or renaming an item is one row here, not edits scattered across
/// several functions with nothing stopping a forgotten arm. Row order must
/// match the [`Item`] discriminants (pinned by `item_meta_table_is_index_aligned`).
///
/// Owned + serde-authorable (C1): the fields are owned + `serde`-round-trippable, so a
/// content game authors its item flavor/wiring as DATA in `items.ron` (inserted as
/// the [`ItemCatalog`] resource) — the same "content out of core"
/// pattern as `boss_profiles.ron` / boss sheets (C6). The `item_catalog` schema maps the
/// resulting serde error to `DiagnosticCode::UnknownField`.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemMeta {
    pub display_name: String,
    pub description: String,
    pub category: ItemCategory,
    /// `HeldItem` id granted on equip (`None` for non-equippables / unwired weapons).
    pub held_item_id: Option<String>,
    /// Stable lowercase authoring id (`condition("inventory.holds", "portalgun")`).
    pub dialog_id: String,
    /// For an [`ItemCategory::Ability`], the equipment row a body wears when
    /// it equips the ability. `None` for every other category, and for an
    /// ability that is only a placeholder.
    #[serde(default)]
    pub wears: Option<ambition_characters::equipment::EquipmentRow>,
    /// The cell's picture, relative to the asset root. `None` shows the name.
    #[serde(default)]
    pub icon: Option<String>,
    /// The most of this item that the bag holds. `1` (the default) is a unique
    /// item: a second grant adds nothing. A number above 1 is a stack, and the
    /// menu shows its count. The category does not decide this: a held bomb
    /// stacks, and a held axe does not.
    #[serde(default = "unique_stack")]
    pub stack: u32,
}

/// The [`ItemMeta::stack`] of a row that does not author one.
fn unique_stack() -> u32 {
    1
}

/// The item CATALOG one `App` plays (C1): what each grid slot is called, what it
/// does, and how authoring names it.
///
/// A content game authors its item table in `items.ron` (a `Vec<ItemMeta>` in
/// grid order, refused unless it names every slot) and inserts the catalog as
/// a resource. The pack is the one authority (Q104): there is no built-in
/// table. A slot the catalog does not name is no item: it is not granted,
/// drawn or saved. `ItemCatalog::default()` names no slot.
///
/// ⭐ APP-LOCAL (2026-10-01). It was a process-global `OnceLock` installed once,
/// so a running game could not take an `items.ron` edit and two Apps in one
/// process could not play two catalogs. Every reader asks the resource now.
#[derive(bevy::prelude::Resource, Clone, Debug, Default, PartialEq)]
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

    /// The catalog the `item_catalog` schema LOWERS a validated `items.ron` to.
    ///
    /// The compiler has already parsed and judged these rows, so re-entering
    /// through [`Self::from_ron`] would parse the same bytes a second time —
    /// the two-readers shape the content pack exists to remove.
    pub fn from_rows(rows: Vec<ItemMeta>) -> Self {
        Self { rows }
    }

    /// The authored rows, in grid order.
    pub fn rows(&self) -> &[ItemMeta] {
        &self.rows
    }

    /// The row this catalog plays for `item`, or `None` when the catalog does
    /// not name that slot.
    pub fn meta(&self, item: Item) -> Option<&ItemMeta> {
        self.rows.get(item.index())
    }

    pub fn category(&self, item: Item) -> Option<ItemCategory> {
        self.meta(item).map(|meta| meta.category)
    }

    pub fn display_name(&self, item: Item) -> Option<&str> {
        self.meta(item).map(|meta| meta.display_name.as_str())
    }

    pub fn description(&self, item: Item) -> Option<&str> {
        self.meta(item).map(|meta| meta.description.as_str())
    }

    /// For [`ItemCategory::Held`] items, the `HeldItem` id whose `ActionSet` the
    /// player gains on equip. `None` for non-equippables and for weapons whose
    /// held-item wiring is not built yet. PortalGun equips through its own
    /// `PortalGun` component (handled by the menu), so its row says `None`.
    pub fn held_item_id(&self, item: Item) -> Option<&str> {
        self.meta(item)?.held_item_id.as_deref()
    }

    /// Which catalog slot a world held-item (`GroundItem`/`HeldItemSpec` id)
    /// corresponds to, so picking one up grants the right slot.
    pub fn item_by_held_item_id(&self, id: &str) -> Option<Item> {
        Item::ALL.into_iter().find(|item| self.held_item_id(*item) == Some(id))
    }

    /// For an ability, the equipment row a body wears when it equips it.
    pub fn wears(&self, item: Item) -> Option<&ambition_characters::equipment::EquipmentRow> {
        self.meta(item)?.wears.as_ref()
    }

    /// The cell's picture, if the catalog names one.
    pub fn icon(&self, item: Item) -> Option<&str> {
        self.meta(item)?.icon.as_deref()
    }

    /// The most of `item` the bag holds ([`ItemMeta::stack`]), or `None` when
    /// the catalog does not name the slot.
    pub fn stack(&self, item: Item) -> Option<u32> {
        self.meta(item).map(|meta| meta.stack)
    }

    /// Does the bag hold more than one of `item`? The menu shows the count of
    /// such an item.
    pub fn stacks(&self, item: Item) -> bool {
        self.stack(item).is_some_and(|stack| stack > 1)
    }

    /// Is this slot a real item, one a player can have? False for a slot the
    /// catalog does not name, a `Reserved` slot, and a placeholder ability (an
    /// ability with no row to wear: nothing is built behind it yet).
    pub fn is_real(&self, item: Item) -> bool {
        match self.category(item) {
            None | Some(ItemCategory::Reserved) => false,
            Some(ItemCategory::Ability) => self.wears(item).is_some(),
            Some(_) => true,
        }
    }

    /// Does confirming this item EQUIP it, to the hand or into an action slot?
    /// An ability with no row is a placeholder and equips nothing.
    pub fn equips(&self, item: Item) -> bool {
        match self.category(item) {
            Some(ItemCategory::Held) => true,
            Some(ItemCategory::Ability) => self.wears(item).is_some(),
            _ => false,
        }
    }

    /// Stable lowercase id for dialogue/authoring, e.g.
    /// `condition("inventory.holds", "portal_gun")`.
    pub fn dialog_id(&self, item: Item) -> Option<&str> {
        self.meta(item).map(|meta| meta.dialog_id.as_str())
    }

    /// Normalize a raw authoring string the way the Yarn bindings do
    /// (lowercase, drop non-alphanumerics), so `"PortalGun"`, `"portal_gun"`
    /// and `"portal gun"` all resolve, then look it up. Also accepts the legacy
    /// `"healthpotion"` alias for [`Item::HealthCell`].
    pub fn item_by_dialog_id(&self, raw: &str) -> Option<Item> {
        let key: String = raw
            .chars()
            .filter(|c| c.is_alphanumeric())
            .flat_map(|c| c.to_lowercase())
            .collect();
        if let Some(found) = Item::ALL.into_iter().find(|item| self.dialog_id(*item) == Some(key.as_str())) {
            return Some(found);
        }
        // The old 3-kind bag spelled the health consumable "healthpotion"; the
        // catalog id is "healthcell". It is the only divergent alias.
        if key == "healthpotion" {
            return Some(Item::HealthCell);
        }
        None
    }
}

/// The catalog of a composition that installs none: it names no slot.
static NO_ITEM_CATALOG: ItemCatalog = ItemCatalog { rows: Vec::new() };

/// Ambition's shipped `items.ron`, for tests of the crates that read a
/// catalog: a fixture, not a default. Production code reads the App's
/// catalog ([`ItemCatalogRead`], [`item_catalog`]).
#[cfg(any(test, feature = "test-support"))]
pub fn test_item_catalog() -> &'static ItemCatalog {
    static CATALOG: std::sync::OnceLock<ItemCatalog> = std::sync::OnceLock::new();
    CATALOG.get_or_init(|| ItemCatalog::from_ron(include_str!("../../../game/ambition_content/assets/data/items.ron")))
}

/// The item catalog a system reads: the App's, or a catalog that names no slot
/// when the composition installs none.
#[derive(bevy::ecs::system::SystemParam)]
pub struct ItemCatalogRead<'w> {
    catalog: Option<bevy::prelude::Res<'w, ItemCatalog>>,
}

impl ItemCatalogRead<'_> {
    pub fn get(&self) -> &ItemCatalog {
        self.catalog.as_deref().unwrap_or(&NO_ITEM_CATALOG)
    }

    /// Did the App's catalog change since this system last ran? Only a content
    /// reload writes it, so a change is a new catalog, not a spurious tick.
    pub fn is_changed(&self) -> bool {
        self.catalog.as_ref().is_some_and(|catalog| {
            bevy::prelude::DetectChanges::is_changed(catalog)
        })
    }
}

/// [`ItemCatalogRead::get`] for code that holds a `&World`.
pub fn item_catalog(world: &bevy::prelude::World) -> &ItemCatalog {
    world
        .get_resource::<ItemCatalog>()
        .unwrap_or(&NO_ITEM_CATALOG)
}

impl Item {
    /// All 24 items in grid order. The compile-time length check below pins the
    /// catalog to exactly [`ITEM_COUNT`].
    pub const ALL: [Item; ITEM_COUNT] = [
        Item::Fireball,
        Item::EyeLaser,
        Item::Lightning,
        Item::SwordStance,
        Item::AxeStance,
        Item::WhipStance,
        Item::PrecisionBlink,
        Item::ContextualBlink,
        Item::Dash,
        Item::DashTwo,
        Item::Grapple,
        Item::MarkRecall,
        Item::PortalGun,
        Item::Javelin,
        Item::GunSword,
        Item::PuppySlugGun,
        Item::Bomb,
        Item::Empty1,
        Item::HealthCell,
        Item::ManaCell,
        Item::SpareBattery,
        Item::SealedNote,
        Item::FieldSurvey,
        Item::Empty2,
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
}

/// Quantity-owned catalog items: the BAG.
///
/// `counts` is authoritative only for entitlements with no world object. Held
/// item instances remain authoritative on the object/custody side — a body's
/// `HeldItem` / `PortalGun` IS the record that it wields something — and
/// [`Inventory`] projects the two populations together for a reader that wants
/// one answer. Persistence writes stored quantities through
/// [`Self::to_persisted`] and never serializes that projection as a second
/// ownership authority.
///
/// There is no `equipped` field (I1). A body's hand is the record of what it
/// holds; a global mirror cannot serve four seats.
#[derive(Resource, Clone, Debug, PartialEq, Eq)]
pub struct OwnedItems {
    counts: [u32; ITEM_COUNT],
}

impl Default for OwnedItems {
    fn default() -> Self {
        Self {
            counts: [0; ITEM_COUNT],
        }
    }
}

/// What ONE body can be said to have: its bag plus the item in its hand.
///
/// The bag is process-wide (an entitlement ledger); the hand is the body's own
/// `HeldItem` / `PortalGun`, projected to a catalog [`Item`] by whoever built
/// this view. `max` rather than `+` because these are two readings of one
/// possession, never two possessions: a weapon picked up off the floor has no
/// stored quantity at all — the object is the record that you have it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Inventory<'a> {
    pub bag: &'a OwnedItems,
    pub in_hand: Option<Item>,
}

impl<'a> Inventory<'a> {
    pub fn new(bag: &'a OwnedItems, in_hand: Option<Item>) -> Self {
        Self { bag, in_hand }
    }

    /// How many of `item` this body has: the stored quantity, OR the one in
    /// its hand.
    pub fn count(&self, item: Item) -> u32 {
        self.bag
            .count(item)
            .max(u32::from(self.in_hand == Some(item)))
    }

    pub fn has(&self, item: Item) -> bool {
        self.count(item) > 0
    }

    pub fn is_equipped(&self, item: Item) -> bool {
        self.in_hand == Some(item)
    }

    pub fn equipped(&self) -> Option<Item> {
        self.in_hand
    }
}

/// Which catalog items one body has equipped: the item in its hand, and each
/// ability whose variant is in effect on its slot.
///
/// A value, read from the body each time: the hand and the worn set are the
/// records, and this is only their projection to grid items.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct EquippedItems(u32);

const _: () = assert!(ITEM_COUNT <= u32::BITS as usize, "EquippedItems has one bit per grid item");

impl EquippedItems {
    /// What a body with `in_hand` in its hand, wearing `worn`, has equipped.
    pub fn of(
        catalog: &ItemCatalog,
        in_hand: Option<Item>,
        worn: Option<&ambition_characters::equipment::WornEquipment>,
    ) -> Self {
        let mut bits = 0u32;
        for item in Item::ALL {
            if in_hand == Some(item) || ability_in_effect(catalog, item, worn) {
                bits |= 1 << item.index();
            }
        }
        Self(bits)
    }

    pub fn contains(self, item: Item) -> bool {
        self.0 & (1 << item.index()) != 0
    }
}

/// Is the ability `item` in effect on the body that wears `worn`?
///
/// It is in effect when the body wears its row. A row that changes nothing is
/// the body's own action on its exclusive slot
/// ([`EquipmentRow::changes_nothing`](ambition_characters::equipment::EquipmentRow::changes_nothing)),
/// so it is also in effect while no row holds that slot.
pub fn ability_in_effect(
    catalog: &ItemCatalog,
    item: Item,
    worn: Option<&ambition_characters::equipment::WornEquipment>,
) -> bool {
    let Some(row) = catalog.wears(item) else {
        return false;
    };
    if worn.is_some_and(|worn| worn.wears(&row.id)) {
        return true;
    }
    row.changes_nothing()
        && row
            .exclusive_slot
            .as_deref()
            .is_some_and(|slot| worn.is_none_or(|worn| worn.in_slot(slot).is_none()))
}

/// Simulation-side request to grant an item.
///
/// `count` is already normalized to a whole number by the command/parser layer;
/// the simulation only applies the request.
#[derive(bevy::prelude::Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ItemGrantRequested {
    pub item: Item,
    pub count: u32,
}

/// Simulation-side request to equip an owned ability: the primary player
/// wears its row.
///
/// A menu asks for this; the simulation changes the worn set on one stamped
/// tick, so a rewind across the equip replays it.
#[derive(bevy::prelude::Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ItemEquipRequested {
    pub item: Item,
}

/// Simulation-side request to use one of an owned consumable.
///
/// A menu asks for this; the simulation spends the item and applies its
/// effect on the same tick, so a rewind across the use cannot refund the item
/// or revoke the effect.
#[derive(bevy::prelude::Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ItemUseRequested {
    pub item: Item,
}

/// How many bombs the demo's starting bag holds.
pub const STARTER_BOMBS: u32 = 5;

impl OwnedItems {
    /// Entity-free value projection: each item's count, in catalog order.
    ///
    /// ⭐ THE BAG IS COMPARED (Q129). Shared durable state is peer state, and
    /// peers compare its canonical form, so the bag is in the peer checksum
    /// by its own value rather than only through the save that mirrors it.
    pub fn checksum(&self) -> u64 {
        use ambition_platformer2d_core::snapshot::{checksum_bytes, put_u32, put_u64};
        let mut bytes = Vec::new();
        put_u64(&mut bytes, self.counts.len() as u64);
        for count in &self.counts {
            put_u32(&mut bytes, *count);
        }
        checksum_bytes(&bytes)
    }

    /// How many of `item` the bag stores. The hand is NOT counted here — see
    /// [`Inventory`] for the reader that wants both.
    pub fn count(&self, item: Item) -> u32 {
        self.stored(item)
    }

    /// The stored quantity — an entitlement with no object behind it. The
    /// durable save's view ([`Self::to_persisted`]) and every mutation
    /// ([`Self::grant`] / [`Self::take`]) go through this.
    fn stored(&self, item: Item) -> u32 {
        self.counts[item.index()]
    }

    pub fn has(&self, item: Item) -> bool {
        self.count(item) > 0
    }

    /// Add `n` of an item, up to its row's [`ItemMeta::stack`]. A unique item
    /// (a stack of 1) stays at 1, so a second pickup does not inflate it. A
    /// grant never lowers a count. A slot the catalog does not name is no
    /// item, and nothing is granted.
    pub fn grant(&mut self, catalog: &ItemCatalog, item: Item, n: u32) {
        let Some(stack) = catalog.stack(item) else {
            return;
        };
        let slot = &mut self.counts[item.index()];
        *slot = (*slot).max(slot.saturating_add(n).min(stack));
    }

    /// Can the bag take one more of `item`? False for a full stack and for a
    /// slot the catalog does not name.
    pub fn has_room_for(&self, catalog: &ItemCatalog, item: Item) -> bool {
        catalog.stack(item).is_some_and(|stack| self.stored(item) < stack)
    }

    /// Remove up to `n`; returns how many were actually removed.
    pub fn take(&mut self, item: Item, n: u32) -> u32 {
        let slot = &mut self.counts[item.index()];
        let removed = (*slot).min(n);
        *slot -= removed;
        removed
    }

    /// The demo's starting bag: every real item is unlocked (Jon, 2026-10-30:
    /// "I need to have everything unlocked by default"). A stack starts with
    /// a few (health and mana cells, a stock of bombs); each other item with
    /// one. A placeholder (`ItemCatalog::is_real` is false) stays locked.
    ///
    /// ⛔ A KEY ITEM IS NOT AN UNLOCK. It is a fact of a quest: the intro
    /// quest gives the Sealed Note and takes it for the Field Survey, so a bag
    /// that starts with them lets the player hand Bob a note Alice never gave.
    pub fn starter(catalog: &ItemCatalog) -> Self {
        let mut owned = Self::default();
        for item in Item::ALL {
            if !catalog.is_real(item) || catalog.category(item) == Some(ItemCategory::KeyItem) {
                continue;
            }
            let n = match item {
                Item::HealthCell => 3,
                Item::ManaCell => 2,
                Item::Bomb => STARTER_BOMBS,
                _ => 1,
            };
            owned.grant(catalog, item, n);
        }
        owned
    }

    #[cfg(test)]
    pub fn total(&self) -> u32 {
        self.counts.iter().sum()
    }

    /// Serialize the owned quantities to the persisted save form (every item with
    /// a non-zero STORED count, keyed by stable `dialog_id`). Equipped state is
    /// not persisted yet — re-equip from the grid on load (handoff).
    ///
    ///  [`Self::stored`], never [`Self::count`]. The projected hand is an
    /// OBJECT, and an object written into the save as a quantity comes back on
    /// the next load as a row while the room that authors it re-authors the
    /// object — one weapon saved, two loaded. What the save does not describe it
    /// does not keep, and durable custody is a frontier of its own.
    pub fn to_persisted(
        &self,
        catalog: &ItemCatalog,
    ) -> Vec<ambition_persistence::save_data::PersistedItem> {
        Item::ALL
            .into_iter()
            .filter_map(|item| {
                let c = self.stored(item);
                let id = catalog.dialog_id(item)?;
                (c > 0).then(|| ambition_persistence::save_data::PersistedItem::new(id, c))
            })
            .collect()
    }

    /// Give back each unlock of `starting` that this bag (just loaded from a
    /// save) does not hold: a unique ability or held item, unless `carried`
    /// says that its object is held in the save (a held item's object, keyed
    /// by its held-item id).
    ///
    /// The demo unlocks every real item (Jon, 2026-10-30), but a save made
    /// before an item was added, or while an item was in the hand (the hand is
    /// not saved), replaces the bag without it, and the item stays locked for
    /// good. An unlock is equipment: a key item and a stack are left as the
    /// save says, because a quest or a throw can spend them.
    pub fn keep_unlocks(&mut self, catalog: &ItemCatalog, starting: &OwnedItems, carried: impl Fn(&str) -> bool) {
        for item in Item::ALL {
            let equipment = matches!(catalog.category(item), Some(ItemCategory::Ability | ItemCategory::Held));
            if !equipment || catalog.stacks(item) || !starting.has(item) || self.has(item) {
                continue;
            }
            if catalog.held_item_id(item).is_some_and(&carried) {
                continue;
            }
            self.grant(catalog, item, 1);
        }
    }

    /// Replace the owned counts from a persisted save (clears first, then grants
    /// each — so `grant`'s unique-item clamp still applies to a hand-edited save).
    /// Unknown ids (a catalog item removed since the save) are skipped.
    pub fn apply_persisted(
        &mut self,
        catalog: &ItemCatalog,
        items: &[ambition_persistence::save_data::PersistedItem],
    ) {
        *self = Self::default();
        for p in items {
            if let Some(item) = catalog.item_by_dialog_id(&p.id) {
                self.grant(catalog, item, p.count);
            }
        }
    }
}

#[cfg(test)]
mod tests;

// Domain-owned rollback declaration; the host supplies the backend registrar.
mod rollback_registration;
pub use rollback_registration::register_rollback_state;

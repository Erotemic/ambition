// The legacy adventure-menu UI that consumed several of these component markers
// and `InventoryUiState` helpers was deleted in Phase D2; the unified menu reads
// the data model (`ItemKind` / `PlayerInventory` / `InventoryUiState`) directly.
// Keep the markers so the data model stays self-describing.
#![allow(dead_code)]

use bevy::prelude::*;

use crate::ui_nav::MenuFocusState;

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

    /// Stable lowercase id used by authored dialogue, e.g.
    /// `inventory_has("HealthPotion")`. Keyed off the variant name
    /// (not the display `label`, which can change for flavor). The
    /// Yarn binding normalizes both sides by lowercasing and dropping
    /// non-alphanumerics, so `"HealthPotion"`, `"health_potion"`, and
    /// `"health potion"` all resolve here.
    pub fn dialog_id(self) -> &'static str {
        match self {
            Self::HealthPotion => "healthpotion",
            Self::SpareBattery => "sparebattery",
            Self::DataChip => "datachip",
        }
    }

    /// Resolve an authored dialogue item id back to a kind, matching
    /// [`Self::dialog_id`] after the same normalization the Yarn
    /// bindings apply (lowercase, non-alphanumerics dropped). Returns
    /// `None` for an unknown item so callers can warn instead of
    /// silently granting the wrong thing.
    pub fn from_dialog_id(raw: &str) -> Option<Self> {
        let key: String = raw
            .chars()
            .filter(|c| c.is_alphanumeric())
            .flat_map(|c| c.to_lowercase())
            .collect();
        Self::ALL.into_iter().find(|k| k.dialog_id() == key)
    }
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

    pub(super) fn next(self) -> Self {
        Self::from_index(self.index() + 1)
    }

    pub(super) fn previous(self) -> Self {
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

    #[cfg(test)]
    pub fn entries(&self) -> impl Iterator<Item = (ItemKind, u32)> + '_ {
        ItemKind::ALL
            .into_iter()
            .map(move |kind| (kind, self.count(kind)))
    }

    #[cfg(test)]
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
    /// Which input source currently owns selection focus, plus the
    /// last row the pointer actually hovered.
    pub focus: MenuFocusState,
}

impl InventoryUiState {
    pub(super) fn reset_for_open(&mut self, opened_from_pause: bool) {
        self.visible = true;
        self.selected = 0;
        self.tab = InventoryTab::Items;
        self.content_scroll = 0;
        self.opened_from_pause = opened_from_pause;
        self.pointer_confirm = false;
        self.pointer_armed = None;
        self.focus = MenuFocusState::default();
    }

    pub(super) fn close(&mut self) {
        self.visible = false;
        self.pointer_confirm = false;
        self.pointer_armed = None;
        self.focus = MenuFocusState::default();
    }

    pub(super) fn set_tab(&mut self, tab: InventoryTab) {
        if self.tab != tab {
            self.tab = tab;
            self.selected = 0;
            self.content_scroll = 0;
            self.pointer_confirm = false;
            self.pointer_armed = None;
            self.focus = MenuFocusState::default();
        }
    }

    pub(super) fn next_tab(&mut self) {
        self.set_tab(self.tab.next());
    }

    pub(super) fn previous_tab(&mut self) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_dialog_id_resolves_loose_spelling_and_rejects_unknown() {
        assert_eq!(
            ItemKind::from_dialog_id("HealthPotion"),
            Some(ItemKind::HealthPotion)
        );
        assert_eq!(
            ItemKind::from_dialog_id("health_potion"),
            Some(ItemKind::HealthPotion)
        );
        assert_eq!(
            ItemKind::from_dialog_id("Spare Battery"),
            Some(ItemKind::SpareBattery)
        );
        assert_eq!(
            ItemKind::from_dialog_id("DATACHIP"),
            Some(ItemKind::DataChip)
        );
        assert_eq!(ItemKind::from_dialog_id("grapple"), None);
        // dialog_id round-trips through from_dialog_id for every kind.
        for kind in ItemKind::ALL {
            assert_eq!(ItemKind::from_dialog_id(kind.dialog_id()), Some(kind));
        }
    }

    #[test]
    fn inventory_tab_cycles_forward_and_backward_with_wraparound() {
        use InventoryTab::{Items, Map, Quests};
        assert_eq!(Items.next(), Map);
        assert_eq!(Map.next(), Quests);
        assert_eq!(Quests.next(), Items, "next wraps");
        assert_eq!(Items.previous(), Quests, "previous wraps");
        assert_eq!(Quests.previous(), Map);
    }

    #[test]
    fn inventory_bag_adds_saturates_and_removes_floors() {
        let mut bag = PlayerInventory::default();
        assert_eq!(bag.count(ItemKind::HealthPotion), 0);

        bag.add(ItemKind::HealthPotion, 3);
        assert_eq!(bag.count(ItemKind::HealthPotion), 3);

        // remove returns how many were actually removed.
        assert_eq!(bag.remove(ItemKind::HealthPotion, 2), 2);
        assert_eq!(bag.count(ItemKind::HealthPotion), 1);

        // removing more than present floors at 0 and reports the real count.
        assert_eq!(bag.remove(ItemKind::HealthPotion, 5), 1);
        assert_eq!(bag.count(ItemKind::HealthPotion), 0);

        // add saturates rather than overflowing.
        bag.add(ItemKind::DataChip, u32::MAX);
        bag.add(ItemKind::DataChip, 10);
        assert_eq!(bag.count(ItemKind::DataChip), u32::MAX);
        // items are tracked per-kind independently.
        assert_eq!(bag.count(ItemKind::HealthPotion), 0);
    }
}

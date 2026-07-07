//! Inventory-screen UI state model: [`InventoryUiState`] (visible/selected/tab/
//! scroll/focus) + the [`InventoryTab`] enum (Items / Map / Quests).
//!
//! This is menu-NAVIGATION state only — it holds no items. The actual item
//! store is the `OwnedItems` catalog in [`crate`]; this struct just
//! tracks where the cursor is in the unified menu's inventory view.

// The legacy adventure-menu UI that consumed the per-entity inventory component
// markers was deleted in Phase D2; the unified menu reads the data model
// (`InventoryUiState`) + the `OwnedItems` catalog (`crate`) directly. The
// dead markers and the legacy 3-kind `ItemKind`/`PlayerInventory` bag were
// removed once `OwnedItems` became the single item store. What remains is the
// live menu-navigation state.
#![allow(dead_code)]

use bevy::prelude::*;

use ambition_ui_nav::MenuFocusState;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inventory_tab_cycles_forward_and_backward_with_wraparound() {
        use InventoryTab::{Items, Map, Quests};
        assert_eq!(Items.next(), Map);
        assert_eq!(Map.next(), Quests);
        assert_eq!(Quests.next(), Items, "next wraps");
        assert_eq!(Items.previous(), Quests, "previous wraps");
        assert_eq!(Quests.previous(), Map);
    }
}

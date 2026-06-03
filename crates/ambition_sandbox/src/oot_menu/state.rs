//! State + 6×4 grid cursor navigation for the OoT-style item menu.

use bevy::prelude::*;

use crate::items::{Item, ITEM_GRID_COLS, ITEM_GRID_ROWS};
use crate::ui_nav::MenuFocusState;

/// Visibility + cursor state for the OoT item grid overlay.
///
/// The cursor is a flat slot index `0..24`; [`Self::grid`] decodes it to
/// `(row, col)`. Navigation wraps within the row (left/right) and column
/// (up/down), matching the OoT item subscreen's wrap-around feel.
#[derive(Resource, Default)]
pub struct OotMenuState {
    pub visible: bool,
    /// Selected slot index, `0..ITEM_COUNT`.
    pub cursor: usize,
    /// True when opened from the pause menu (vs. directly from gameplay), so we
    /// know whether to return to `Playing` or `Paused` on close.
    pub opened_from_pause: bool,
    /// Set by the pointer system when a tap should confirm the current slot;
    /// consumed by `oot_menu_input` the same frame.
    pub pointer_confirm: bool,
    /// Row "armed" by a prior tap under tap-then-confirm policy.
    pub pointer_armed: Option<usize>,
    /// Which input source owns focus + the last hovered slot.
    pub focus: MenuFocusState,
    /// Short transient status line (e.g. "Equipped Axe", "Not acquired").
    pub status: String,
}

impl OotMenuState {
    pub fn open(&mut self, opened_from_pause: bool) {
        self.visible = true;
        self.cursor = 0;
        self.opened_from_pause = opened_from_pause;
        self.pointer_confirm = false;
        self.pointer_armed = None;
        self.focus = MenuFocusState::default();
        self.status.clear();
    }

    pub fn close(&mut self) {
        self.visible = false;
        self.pointer_confirm = false;
        self.pointer_armed = None;
        self.focus = MenuFocusState::default();
    }

    /// The item currently under the cursor.
    pub fn selected_item(&self) -> Item {
        Item::from_index(self.cursor).unwrap_or(Item::ALL[0])
    }

    /// `(row, col)` of the cursor.
    pub fn grid(&self) -> (usize, usize) {
        (self.cursor / ITEM_GRID_COLS, self.cursor % ITEM_GRID_COLS)
    }

    /// Move the cursor by a grid delta with per-axis wrap. Returns true if the
    /// cursor moved (always true here since wrap means motion is never blocked,
    /// except a zero delta).
    pub fn move_cursor(&mut self, dcol: isize, drow: isize) -> bool {
        if dcol == 0 && drow == 0 {
            return false;
        }
        let (row, col) = self.grid();
        let cols = ITEM_GRID_COLS as isize;
        let rows = ITEM_GRID_ROWS as isize;
        let new_col = (col as isize + dcol).rem_euclid(cols) as usize;
        let new_row = (row as isize + drow).rem_euclid(rows) as usize;
        let next = new_row * ITEM_GRID_COLS + new_col;
        let moved = next != self.cursor;
        self.cursor = next;
        moved
    }

    pub fn set_cursor(&mut self, index: usize) {
        if index < crate::items::ITEM_COUNT {
            self.cursor = index;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_decodes_to_grid_and_back() {
        let mut s = OotMenuState::default();
        s.set_cursor(0);
        assert_eq!(s.grid(), (0, 0));
        s.set_cursor(7);
        assert_eq!(s.grid(), (1, 1));
        s.set_cursor(23);
        assert_eq!(s.grid(), (3, 5));
        assert_eq!(s.selected_item(), Item::ReservedSlot);
    }

    #[test]
    fn horizontal_nav_wraps_within_the_row() {
        let mut s = OotMenuState::default();
        s.set_cursor(0); // row 0, col 0
        s.move_cursor(-1, 0); // wrap to col 5 of row 0
        assert_eq!(s.grid(), (0, 5));
        s.move_cursor(1, 0); // back to col 0
        assert_eq!(s.grid(), (0, 0));
    }

    #[test]
    fn vertical_nav_wraps_within_the_column() {
        let mut s = OotMenuState::default();
        s.set_cursor(2); // row 0, col 2
        s.move_cursor(0, -1); // wrap to bottom row, same column
        assert_eq!(s.grid(), (3, 2));
        s.move_cursor(0, 1); // back to top row
        assert_eq!(s.grid(), (0, 2));
    }

    #[test]
    fn diagonal_and_zero_moves_behave() {
        let mut s = OotMenuState::default();
        s.set_cursor(7); // (1,1)
        assert!(s.move_cursor(1, 1)); // (2,2)
        assert_eq!(s.grid(), (2, 2));
        assert!(!s.move_cursor(0, 0), "zero delta does not move");
    }

    #[test]
    fn open_resets_and_close_hides() {
        let mut s = OotMenuState::default();
        s.set_cursor(10);
        s.open(true);
        assert!(s.visible);
        assert_eq!(s.cursor, 0);
        assert!(s.opened_from_pause);
        s.close();
        assert!(!s.visible);
    }
}

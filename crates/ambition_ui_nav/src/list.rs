//! Windowed-list math: visible-window start computation and converting discrete
//! scroll steps into up/down menu edges on a `MenuInputFrame` (from
//! `ambition_input`, behind the `input` feature). Pure helpers, no ECS.

#![allow(dead_code)] // Several helpers are reserved API for future menu callers.

#[cfg(feature = "input")]
use ambition_input::MenuInputFrame;

/// Convert discrete vertical scroll steps into menu up/down edges.
///
/// Positive scroll values mean "previous row" / "scroll up". Negative values
/// mean "next row" / "scroll down". This mirrors `MenuControlFrame::scroll_y`
/// and keeps menus from duplicating that sign convention.
#[cfg(feature = "input")]
pub fn apply_vertical_scroll(frame: &mut MenuInputFrame, steps: i32) {
    if steps > 0 {
        frame.up = true;
    } else if steps < 0 {
        frame.down = true;
    }
}

/// Pure windowing helper for a selected item inside a long menu/list.
///
/// `selected` and `total` are absolute list coordinates. `capacity` is the
/// maximum number of visible row slots. The returned start keeps `selected`
/// visible while centering it when possible and clamping at the list edges.
pub fn visible_window_start(selected: usize, total: usize, capacity: usize) -> usize {
    if total <= capacity || capacity == 0 {
        return 0;
    }
    let half = capacity / 2;
    let start = selected.saturating_sub(half);
    start.min(total - capacity)
}

/// Scroll the window only as far as it must to show `selected` — the
/// alternative to recentering, and the one a finger can use.
///
/// Re-centering on every selection change moves a touched row between presses.
/// Keep the existing window whenever the selected row is already visible.
///
/// This keeps `previous_start` whenever the selection is already visible, and
/// otherwise moves by the minimum needed to bring it to the near edge. A row
/// therefore holds its screen position for as long as the selection stays in
/// the window, which is what makes a direct touch mean what it looked like it
/// meant.
pub fn scroll_into_view(
    previous_start: usize,
    selected: usize,
    total: usize,
    capacity: usize,
) -> usize {
    if total <= capacity || capacity == 0 {
        return 0;
    }
    let max_start = total - capacity;
    let start = previous_start.min(max_start);
    if selected < start {
        selected
    } else if selected >= start + capacity {
        // The near edge, not the centre: one step of travel per step of
        // navigation, so a held direction scrolls smoothly instead of paging.
        (selected + 1 - capacity).min(max_start)
    } else {
        start
    }
}

/// Map a visible slot index back to its absolute list row.
pub fn visible_row_index(
    slot_index: usize,
    selected: usize,
    total: usize,
    capacity: usize,
) -> Option<usize> {
    if total == 0 || slot_index >= capacity {
        return None;
    }
    let start = visible_window_start(selected, total, capacity);
    let absolute = start + slot_index;
    (absolute < total).then_some(absolute)
}

/// Stateful selected-row cursor for menu/list navigation.
///
/// The cursor owns the boring but easy-to-drift rules shared by pause-menu,
/// radio, settings, inventory, and future mobile-sized lists: wrapping
/// up/down movement, clamping after the backing list changes, and rendering a
/// window that keeps the selected row visible.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ListCursor {
    selected: usize,
    total: usize,
}

impl ListCursor {
    pub fn new(selected: usize, total: usize) -> Self {
        let mut cursor = Self { selected, total };
        cursor.clamp();
        cursor
    }

    pub fn empty() -> Self {
        Self::new(0, 0)
    }

    pub fn selected(self) -> usize {
        self.selected
    }

    pub fn total(self) -> usize {
        self.total
    }

    pub fn is_empty(self) -> bool {
        self.total == 0
    }

    pub fn set_selected(&mut self, selected: usize) {
        self.selected = selected;
        self.clamp();
    }

    pub fn set_total(&mut self, total: usize) {
        self.total = total;
        self.clamp();
    }

    pub fn clamp(&mut self) {
        if self.total == 0 {
            self.selected = 0;
        } else if self.selected >= self.total {
            self.selected = self.total - 1;
        }
    }

    pub fn move_previous_wrapping(&mut self) -> bool {
        if self.total == 0 {
            self.selected = 0;
            return false;
        }
        let before = self.selected;
        self.selected = (self.selected + self.total - 1) % self.total;
        self.selected != before
    }

    pub fn move_next_wrapping(&mut self) -> bool {
        if self.total == 0 {
            self.selected = 0;
            return false;
        }
        let before = self.selected;
        self.selected = (self.selected + 1) % self.total;
        self.selected != before
    }

    pub fn apply_directional(&mut self, previous: bool, next: bool) -> bool {
        let before = self.selected;
        if previous {
            self.move_previous_wrapping();
        }
        if next {
            self.move_next_wrapping();
        }
        self.selected != before
    }

    pub fn apply_scroll_steps(&mut self, steps: i32) -> bool {
        let before = self.selected;
        if steps > 0 {
            for _ in 0..steps {
                self.move_previous_wrapping();
            }
        } else if steps < 0 {
            for _ in 0..steps.unsigned_abs() {
                self.move_next_wrapping();
            }
        }
        self.selected != before
    }

    pub fn window(self, capacity: usize) -> WindowedList {
        WindowedList::new(self.selected, self.total, capacity)
    }

    pub fn visible_row_for_slot(self, slot_index: usize, capacity: usize) -> Option<usize> {
        self.window(capacity).slot_to_index(slot_index)
    }

    pub fn windowed_title(self, base: &str, capacity: usize) -> String {
        windowed_title(base, self.selected, self.total, capacity)
    }

    pub fn indexed_title(self, base: &str) -> String {
        indexed_title(base, self.selected, self.total)
    }

    pub fn decorate_visible_label(self, label: String, index: usize, capacity: usize) -> String {
        decorate_windowed_label(label, index, self.selected, self.total, capacity)
    }
}

/// A clipped text/content window that is controlled by scroll offset rather
/// than selected row. This is useful for read-only pages like quest/map text
/// inside the adventure menu.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScrollWindow {
    pub start: usize,
    pub total: usize,
    pub capacity: usize,
}

impl ScrollWindow {
    pub fn new(start: usize, total: usize, capacity: usize) -> Self {
        let mut window = Self {
            start,
            total,
            capacity,
        };
        window.clamp();
        window
    }

    pub fn clamp(&mut self) {
        if self.total <= self.capacity || self.capacity == 0 {
            self.start = 0;
        } else {
            self.start = self.start.min(self.total - self.capacity);
        }
    }

    pub fn apply_scroll_steps(&mut self, steps: i32) -> bool {
        let before = self.start;
        if steps > 0 {
            self.start = self.start.saturating_sub(steps as usize);
        } else if steps < 0 {
            self.start = self.start.saturating_add(steps.unsigned_abs() as usize);
        }
        self.clamp();
        self.start != before
    }

    pub fn end(self) -> usize {
        (self.start + self.capacity).min(self.total)
    }

    pub fn has_before(self) -> bool {
        self.start > 0
    }

    pub fn has_after(self) -> bool {
        self.end() < self.total
    }

    pub fn range(self) -> std::ops::Range<usize> {
        self.start..self.end()
    }

    pub fn hint_line(self) -> Option<String> {
        match (self.has_before(), self.has_after()) {
            (false, false) => None,
            (true, true) => Some(format!(
                "↑ more   rows {}-{} of {}   ↓ more",
                self.start + 1,
                self.end(),
                self.total
            )),
            (true, false) => Some(format!(
                "↑ more   rows {}-{} of {}",
                self.start + 1,
                self.end(),
                self.total
            )),
            (false, true) => Some(format!(
                "rows {}-{} of {}   ↓ more",
                self.start + 1,
                self.end(),
                self.total
            )),
        }
    }
}

/// A small value object for rendering/clamping long UI lists.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowedList {
    pub selected: usize,
    pub total: usize,
    pub capacity: usize,
}

impl WindowedList {
    pub const fn new(selected: usize, total: usize, capacity: usize) -> Self {
        Self {
            selected,
            total,
            capacity,
        }
    }

    pub fn start(self) -> usize {
        visible_window_start(self.selected, self.total, self.capacity)
    }

    pub fn slot_to_index(self, slot_index: usize) -> Option<usize> {
        visible_row_index(slot_index, self.selected, self.total, self.capacity)
    }

    pub fn selected_display_index(self) -> usize {
        self.selected.min(self.total.saturating_sub(1)) + 1
    }

    pub fn is_windowed(self) -> bool {
        self.total > self.capacity && self.capacity > 0
    }

    pub fn end(self) -> usize {
        (self.start() + self.capacity).min(self.total)
    }
}

/// Title for pages that only show an index when the list is clipped.
pub fn windowed_title(base: &str, selected: usize, total: usize, capacity: usize) -> String {
    let list = WindowedList::new(selected, total, capacity);
    if list.is_windowed() {
        format!("{base} — {}/{}", list.selected_display_index(), total)
    } else {
        base.to_string()
    }
}

/// Title for pages where the current index is useful even when every row fits.
pub fn indexed_title(base: &str, selected: usize, total: usize) -> String {
    if total > 1 {
        let index = selected.min(total.saturating_sub(1)) + 1;
        format!("{base} — {index}/{total}")
    } else {
        base.to_string()
    }
}

/// Add lightweight up/down overflow hints to visible rows.
pub fn decorate_windowed_label(
    label: String,
    index: usize,
    selected: usize,
    total: usize,
    capacity: usize,
) -> String {
    let list = WindowedList::new(selected, total, capacity);
    if !list.is_windowed() {
        return label;
    }
    let start = list.start();
    let end = list.end();
    let prefix = if index == start && start > 0 {
        "↑ "
    } else {
        "  "
    };
    let suffix = if index + 1 == end && end < total {
        " ↓"
    } else {
        ""
    };
    format!("{prefix}{label}{suffix}")
}

#[cfg(test)]
mod tests {

    /// A row holds its screen position while the selection stays visible —
    /// the property that makes a direct touch mean what it looked like it meant.
    #[test]
    fn the_window_does_not_move_while_the_selection_is_already_visible() {
        // Capacity 3 over 8 rows, window showing 2..5.
        for selected in 2..5 {
            assert_eq!(
                scroll_into_view(2, selected, 8, 3),
                2,
                "selecting a VISIBLE row must not scroll: recentering is what \
                 moved the row out from under the finger between the two taps \
                 Android needs"
            );
        }
    }

    /// Leaving the window scrolls by the MINIMUM, to the near edge.
    #[test]
    fn leaving_the_window_scrolls_one_step_not_a_page() {
        assert_eq!(scroll_into_view(2, 5, 8, 3), 3, "stepping down past the bottom edge scrolls by one");
        assert_eq!(scroll_into_view(2, 1, 8, 3), 1, "stepping up past the top edge scrolls by one");
        assert_eq!(scroll_into_view(0, 7, 8, 3), 5, "a jump to the end clamps at the last full window");
    }

    /// A list that fits needs no window at all, and a stale start cannot push
    /// the window past the end.
    #[test]
    fn a_short_list_and_a_stale_start_are_both_handled() {
        assert_eq!(scroll_into_view(4, 1, 3, 5), 0, "everything fits: start is 0");
        assert_eq!(scroll_into_view(99, 7, 8, 3), 5, "a start beyond the end clamps before it is used");
    }
    use super::*;

    #[test]
    fn window_tracks_selected_row_without_overflow() {
        assert_eq!(visible_window_start(0, 12, 5), 0);
        assert_eq!(visible_window_start(4, 12, 5), 2);
        assert_eq!(visible_window_start(11, 12, 5), 7);
        assert_eq!(visible_row_index(0, 11, 12, 5), Some(7));
        assert_eq!(visible_row_index(4, 11, 12, 5), Some(11));
        assert_eq!(visible_row_index(5, 11, 12, 5), None);
    }

    #[test]
    fn titles_only_show_window_context_when_requested() {
        assert_eq!(windowed_title("Settings", 0, 4, 6), "Settings");
        assert_eq!(windowed_title("Settings", 3, 12, 6), "Settings — 4/12");
        assert_eq!(indexed_title("Radio", 0, 4), "Radio — 1/4");
    }

    #[test]
    fn overflow_hints_decorate_window_edges() {
        // Windowed rows without an explicit overflow marker keep a two-space
        // gutter so their labels line up with rows that show `↑ ` / ` ↓`.
        assert_eq!(decorate_windowed_label("A".into(), 0, 0, 10, 4), "  A");
        assert_eq!(decorate_windowed_label("C".into(), 2, 4, 10, 4), "↑ C");
        assert_eq!(decorate_windowed_label("F".into(), 5, 4, 10, 4), "  F ↓");
    }

    #[test]
    fn list_cursor_wraps_and_clamps_selection() {
        let mut cursor = ListCursor::new(99, 4);
        assert_eq!(cursor.selected(), 3);
        assert!(cursor.move_next_wrapping());
        assert_eq!(cursor.selected(), 0);
        assert!(cursor.move_previous_wrapping());
        assert_eq!(cursor.selected(), 3);
        cursor.set_total(0);
        assert_eq!(cursor.selected(), 0);
        assert!(cursor.is_empty());
    }

    #[test]
    fn list_cursor_maps_visible_slots() {
        let cursor = ListCursor::new(11, 12);
        assert_eq!(cursor.windowed_title("Radio", 5), "Radio — 12/12");
        assert_eq!(cursor.visible_row_for_slot(0, 5), Some(7));
        assert_eq!(cursor.visible_row_for_slot(4, 5), Some(11));
        assert_eq!(cursor.visible_row_for_slot(5, 5), None);
    }

    #[test]
    fn scroll_window_clamps_and_reports_hints() {
        let mut window = ScrollWindow::new(250, 12, 5);
        assert_eq!(window.start, 7);
        assert_eq!(window.end(), 12);
        assert!(window.has_before());
        assert!(!window.has_after());
        assert_eq!(window.range(), 7..12);
        assert!(window.apply_scroll_steps(2));
        assert_eq!(window.start, 5);
        assert!(window.hint_line().unwrap().contains("rows 6-10 of 12"));
    }
}

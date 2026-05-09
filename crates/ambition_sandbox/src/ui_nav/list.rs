use crate::input::MenuInputFrame;

/// Convert discrete vertical scroll steps into menu up/down edges.
///
/// Positive scroll values mean "previous row" / "scroll up". Negative values
/// mean "next row" / "scroll down". This mirrors `MenuControlFrame::scroll_y`
/// and keeps menus from duplicating that sign convention.
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
}

# Menu navigation model

Ambition's pause, radio, settings, inventory, and future mobile-friendly menus
should share the same small navigation model instead of each screen hand-rolling
selection, wraparound, scroll, and visible-window math.

## Building blocks

- `ui_nav::ListCursor` owns selected-row state for interactive lists. It clamps
  when the backing list length changes, wraps up/down movement, applies discrete
  scroll steps, and maps visible slots back to absolute rows.
- `ui_nav::WindowedList` is the read-only rendering view for a selected row and
  visible capacity. It keeps the selected row visible and exposes row slots.
- `ui_nav::ScrollWindow` owns scroll-offset windows for read-only text/list
  panels such as map and quest summaries.
- `ui_nav::decorate_windowed_label` and `ListCursor::decorate_visible_label`
  keep overflow hints (`↑` / `↓`) consistent across menus.

## UI polish expectations

Long menus should not assume desktop-sized panels. Prefer a small visible row
count and expose position/overflow hints in the title or row gutter. This is
especially important for radio tracks, settings pages, quest logs, and mobile
screens where the same row count must work with touch input.

For pointer and touch input, preserve the existing two-step confirmation rules
for destructive actions, but let hover/drag/scroll update the same selected row
that keyboard and gamepad use. This avoids separate desktop and mobile menu
state machines.

## Pattern

```rust
let mut cursor = ListCursor::new(state.selected, rows.len());
cursor.apply_directional(frame.up, frame.down);
cursor.apply_scroll_steps(frame.vertical_scroll_steps());
state.selected = cursor.selected();

let cursor = ListCursor::new(state.selected, rows.len());
let title = cursor.windowed_title("Radio", visible_rows);
let Some(row_index) = cursor.visible_row_for_slot(slot.index, visible_rows) else {
    hide_row();
    return;
};
```

Read-only text panels should use `ScrollWindow` instead of open-coded
`skip(...).take(...)` plus separate `↑ more` / `↓ more` checks.

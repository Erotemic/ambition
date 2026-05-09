# UI navigation refactor benchmark candidates

These candidates are kept separate from `rust-questions.md` because another
agent may be editing the main Rust benchmark file. They focus on the shared menu
/ touch / pointer navigation layer introduced during the UI refactor.

## 2026-05-09: Avoid overlapping mutable borrows when extracting a Bevy UI helper

Tags: `rust-borrow-checker`, `bevy-resource`, `ui-nav`, `rust-module-refactor`, `touch-ui`

### Setup

You are refactoring a Bevy game menu. Before the refactor, pointer interaction
logic lived directly in `pause_menu.rs` and mutated fields on one resource:

```rust
#[derive(Resource)]
struct PauseMenuState {
    selected: usize,
    pointer_armed: Option<usize>,
    pointer_confirm: bool,
}
```

The menu has rows that can be hovered or pressed. Press handling needs both the
currently selected row and the armed row used by the user's tap-confirm mode:

```rust
let press = tap_mode.resolve_press(index, state.selected, destructive, &mut state.pointer_armed);
state.selected = index;
if matches!(press, MenuPointerPress::Confirm) {
    state.pointer_confirm = true;
}
```

You introduce a shared `ui_nav::pointer` helper so pause menus, dialog choices,
and future inventory/map menus can share one pointer-row contract. A tempting
helper signature is:

```rust
pub fn handle_selectable_row_interaction(
    interaction: &Interaction,
    index: usize,
    selected: &mut usize,
    tap_mode: MenuTapMode,
    destructive: bool,
    pointer_armed: &mut Option<usize>,
) -> RowPointerOutcome;
```

Then `pause_menu.rs` calls it like this from a system that has
`mut state: ResMut<PauseMenuState>`:

```rust
let outcome = handle_selectable_row_interaction(
    interaction,
    index,
    &mut state.selected,
    tap_mode,
    destructive,
    &mut state.pointer_armed,
);
```

### Question

When extracting this shared helper, how should you design the helper boundary so
callers can update both `selected` and `pointer_armed` without taking two
simultaneous mutable borrows from the same Bevy resource, and why is that shape
better for future menu-like UI callers?

### Expected answer

Do not make the primary shared helper require callers to pass two `&mut` field
borrows from the same parent state object. Instead, use a value-oriented helper
that takes the current values and returns the updated values plus the semantic
outcome:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RowPointerUpdate {
    pub selected: usize,
    pub pointer_armed: Option<usize>,
    pub outcome: RowPointerOutcome,
}

pub fn resolve_selectable_row_interaction(
    interaction: &Interaction,
    index: usize,
    selected: usize,
    tap_mode: MenuTapMode,
    destructive: bool,
    pointer_armed: Option<usize>,
) -> RowPointerUpdate {
    let mut selected = selected;
    let mut pointer_armed = pointer_armed;
    let outcome = handle_selectable_row_interaction(
        interaction,
        index,
        &mut selected,
        tap_mode,
        destructive,
        &mut pointer_armed,
    );
    RowPointerUpdate { selected, pointer_armed, outcome }
}
```

The Bevy system then updates the resource sequentially:

```rust
let update = resolve_selectable_row_interaction(
    interaction,
    index,
    state.selected,
    tap_mode,
    destructive,
    state.pointer_armed,
);
state.selected = update.selected;
state.pointer_armed = update.pointer_armed;
if matches!(update.outcome, RowPointerOutcome::Confirmed) {
    state.pointer_confirm = true;
}
```

This preserves one shared pointer-row contract while avoiding aliasing problems
for callers whose menu state is stored in a single resource or component. It is
also easier to reuse from dialog, inventory, and map UIs because they can adapt
from their own state shape into a small value update instead of exposing their
internal fields to the helper.

### Validation

Run:

```bash
cargo fmt --all
cargo test -p ambition_sandbox --lib
```

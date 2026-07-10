# `ambition_ui_nav` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_ui_nav** — Shared UI/menu navigation helpers.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`drag`](src/drag.rs) | Drag accumulation: `DragScrollState` turns continuous pointer/touch motion into discrete row-scroll steps so swipe-scroll reuses the same navigation path as arrow-key/d-pad row movement. |
| [`list`](src/list.rs) | Windowed-list math: visible-window start computation and converting discrete scroll steps into up/down menu edges on a `MenuInputFrame` (from `ambition_input`, behind the `input` feature). |
| [`pointer`](src/pointer.rs) | Pointer/touch row activation: `MenuFocusOwner` / `MenuFocusState` track which source owns focus, and `resolve_selectable_row_interaction` applies the host's `MenuTapMode` (from `ambition_input::settings`) to a Bevy `Interaction` to decide hover-vs-select-vs-activate. |

_3 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._

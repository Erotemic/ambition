//! Shared UI/menu navigation helpers.
//!
//! Pause menus, dialog choices, inventory/map pages, and mobile touch gestures
//! should all consume the same small set of semantic primitives: windowed list
//! math, discrete scroll-to-row navigation, pointer row activation, and drag
//! accumulation. Keeping those pieces here prevents the keyboard/gamepad/touch
//! paths from drifting apart as the desktop-first UI gets mobile affordances.

mod drag;
mod list;
mod pointer;

pub use drag::DragScrollState;
pub use list::{apply_vertical_scroll, visible_window_start};
pub use pointer::{resolve_selectable_row_interaction, MenuFocusOwner, MenuFocusState};

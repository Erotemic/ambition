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
#[cfg(feature = "input")]
pub use list::apply_vertical_scroll;
pub use list::scroll_into_view;
// ⭐ THE PAUSE MENU IS THE FIRST NAME IN `ListCursor`'s OWN DOC and was the last
// caller still hand-rolling the rules it owns. Exported so it can stop.
pub use list::visible_window_start;
pub use list::ListCursor;
pub use pointer::{
    resolve_selectable_row_interaction, DialogChoiceSlot, MenuFocusOwner, MenuFocusState, PressArm,
    RowPointerOutcome, RowPress, ROW_TAP_SLOP_PX,
};

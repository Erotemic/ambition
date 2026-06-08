//! Map / minimap state and UI.
//!
//! The module is split by concern:
//! - `model` owns the map resource, room nodes, and zoom state.
//! - `systems` hydrates visits and room geometry from save / LDtk runtime state.
//! - `input` and `pointer` own user interactions.
//! - `ui` owns full-map and minimap spawning / sync.

mod input;
mod model;
mod pointer;
mod systems;
mod ui;

#[cfg(test)]
mod tests;

pub use input::handle_map_menu_hotkeys;
pub use model::MapMenuState;
pub use pointer::map_menu_pointer_dismiss;
pub use systems::{populate_map_rooms, sync_map_from_save, track_room_visits};
pub use ui::{spawn_map_menu, sync_map_menu};

#[cfg(test)]
pub(super) use model::{MAP_ZOOM_MAX, MAP_ZOOM_MIN};

#[cfg(test)]
use ui::short_room_label;

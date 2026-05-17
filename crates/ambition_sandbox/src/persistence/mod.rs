//! Save game I/O and the user-settings store.
//!
//! - `save`     — autosave + load of the per-session `Save` resource.
//! - `settings` — typed model + persistence for audio / video /
//!                controls / gameplay preferences.

pub mod save;
pub mod settings;

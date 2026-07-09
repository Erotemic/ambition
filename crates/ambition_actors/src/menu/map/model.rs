//! Compatibility re-export for the Map/minimap state model.
//!
//! Canonical renderer-agnostic map state lives in `ambition_menu::map`; the
//! actor-side map module keeps the systems/UI adapters that hydrate and draw it
//! from Ambition room/save state.

pub use ambition_menu::map::*;

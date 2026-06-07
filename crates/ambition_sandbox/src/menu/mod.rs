//! Unified menu content for the sandbox.
//!
//! This module is the single home for the game's menu *content* — the
//! backend-agnostic page model, the concrete settings IR, and the Map tab —
//! consolidated out of the formerly-scattered top-level modules
//! (`menu_model`, `map_menu`, `persistence/settings/{menu,system_menu}`).
//! Presentation (the cube + the bevy_ui grid) still lives elsewhere for now;
//! see `docs/planning/unified_tabbed_menu.md` §10 for the full plan.
//!
//! Submodules:
//! - [`model`] — `MenuPage` / `MenuFocus` / `MenuPageAction` + the page builders
//!   (was `crate::menu_model`).
//! - [`ir`] — Ambition's concrete settings IR: the shared `SettingsOption`
//!   model ([`ir::settings`]) and the System-menu layer ([`ir::system`]) that
//!   the cube System face renders. Reads `crate::persistence::settings`.
//! - [`map`] — the Map tab content (was `crate::map_menu`).

pub mod ir;
pub mod map;
#[cfg(feature = "oot_inventory")]
pub mod model;

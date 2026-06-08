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

/// Backend-agnostic action dispatcher (`dispatch_menu_action`); shared by the
/// cube backend and any future menu frontend. See [`dispatch`].
pub mod dispatch;
/// Backend-agnostic item-confirmation effects (`MenuAction` + `decide` +
/// `apply_menu_action`/`dispatch_item_confirm` + the shared player/mana query
/// shapes). The ONE place an item confirmation becomes ECS side effects; shared
/// by [`dispatch`], [`grid_backend`], and the cube host. Relocated from the
/// deleted `crate::bevy_ui_grid_menu` (Phase D1).
pub mod effects;
/// The unified flat tabbed menu — the `InventoryUiBackend::Grid` presentation
/// (Phase C2b). Ambition's wiring of the engine `bevy_ui` renderer; the flat analog
/// of the cube backend, sharing the page model + dispatcher + cursor.
pub mod grid_backend;
pub mod ir;
pub mod map;
pub mod model;

/// Cross-backend parity / no-drift tests (design doc §8): the safety net that
/// locks the "one content model + IR + dispatcher, two presentations" invariant.
#[cfg(test)]
mod parity_tests;

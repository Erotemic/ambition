//! Unified menu content for the sandbox.
//!
//! This module is the single home for the game's menu *content* — the
//! backend-agnostic page model, the concrete settings IR, and the Map tab —
//! consolidated out of the formerly-scattered top-level modules
//! (`menu_model`, `map_menu`, `persistence/settings/{menu,system_menu}`).
//! Presentation backends (the cube + the bevy_ui grid) are installed through
//! independent feature-gated plugins; see `docs/planning/unified_tabbed_menu.md`
//! §10 for the full plan.
//!
//! Submodules:
//! - [`backend`] — the `InventoryUiBackend` vocabulary that picks which compiled
//!   frontend (Grid / cube) renders, collapsing to an available one per build.
//! - [`ir`] — Ambition's concrete settings IR: the shared `SettingsOption`
//!   model ([`ir::settings`]) and the System-menu layer ([`ir::system`]) that
//!   the cube System face renders. Reads `crate::persistence::settings`.
//! - [`map`] — the Map tab content (map / minimap state, hydration, and UI).

pub mod backend;
pub mod ir;
pub mod map;

//! Unified menu content for the sandbox.
//!
//! This module is the single home for the game's menu *content* — the
//! backend-agnostic page model, the concrete settings IR, and the Map tab —
//! consolidated out of the formerly-scattered top-level modules
//! (`menu_model`, `map_menu`, `persistence/settings/{menu,system_menu}`).
//! Presentation backends (the cube + the bevy_ui grid) are installed through
//! independent feature-gated plugins
//! §10 for the full plan.
//!
//! Submodules:
//! - backend selection (`InventoryUiBackend`) now lives in `ambition_menu::backend`.
//! - settings / System-menu IR lives in `ambition_settings_menu`; actor
//!   persistence keeps only pause-menu compatibility helpers.
//! - [`map`] — the Map tab content (save/room hydration and Bevy UI adapters).

pub mod map;

//! Compatibility adapter for persistence paths that still sit inside the
//! gameplay-core UI surface.
//!
//! The stored save/settings/quest shapes moved to `ambition_persistence` in
//! E1a; `DeveloperTools` disk persistence and its schedule plugin live in
//! `ambition_dev_tools`. The local residue is the settings/menu IR, still tied
//! to gameplay-core state until E1e.

pub use ambition_persistence::{save, save_data, PersistenceSchedulePlugin};

pub mod settings;

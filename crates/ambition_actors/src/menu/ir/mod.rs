//! Ambition's concrete settings IR — carved into `ambition_settings_menu` (E1e).
//!
//! The settings model (`SettingsMenuModel` / `SettingsOption` /
//! `apply_settings_option`) and the System-menu layer on top of it are now a
//! standalone foundational crate that depends only on
//! `ambition_persistence::settings`. They are re-exported here on the historical
//! `crate::menu::ir::{settings, system}` paths so the `persistence::settings`
//! facade + the app-menu hosts need no import edits.

pub use ambition_settings_menu::{settings, system};

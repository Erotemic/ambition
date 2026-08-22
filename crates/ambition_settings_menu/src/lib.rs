//! Renderer-agnostic settings and system-menu IR.
//!
//! - [`settings`] — the shared settings model (`SettingsMenuModel` /
//!   `SettingsOption` / `settings_menu_model` / `apply_settings_option`), built
//!   from `ambition_persistence::settings::UserSettings`. The bevy-UI grid and
//!   the lunex cube's System face both render this one model.
//! - [`system`] — the System-menu layer on top of [`settings`] (Radio / Video /
//!   Audio / Controls / Gameplay / Language / Reset* / Quit / Developer).
//!
//! Pure logic: no Bevy, renderer, or game-state dependency; it names only the
//! `ambition_persistence` settings vocabulary.

pub mod settings;
pub mod system;

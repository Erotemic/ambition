//! The renderer-agnostic settings + system menu IR (E1e carve out of
//! `ambition_gameplay_core::menu::ir`).
//!
//! - [`settings`] — the shared settings model (`SettingsMenuModel` /
//!   `SettingsOption` / `settings_menu_model` / `apply_settings_option`), built
//!   from `ambition_persistence::settings::UserSettings`. The bevy-UI grid and
//!   the lunex cube's System face both render this one model.
//! - [`system`] — the System-menu layer on top of [`settings`] (Radio / Video /
//!   Audio / Controls / Gameplay / Language / Reset* / Quit / Developer).
//!
//! Pure logic: no bevy, no renderer, no game state — it names only the
//! `ambition_persistence` settings vocabulary. That is the whole point of the
//! carve: the settings IR was the god-dep that forced the menu presentation to
//! reach back into gameplay-core; here it stands alone below both.

pub mod settings;
pub mod system;

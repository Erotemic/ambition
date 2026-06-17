//! Ambition's concrete settings IR.
//!
//! - [`settings`] — the shared, renderer-agnostic settings model
//!   (`SettingsMenuModel` / `SettingsOption` / `apply_settings_option`); built
//!   from `crate::persistence::settings::UserSettings`. (Was
//!   `crate::persistence::settings::menu`.)
//! - [`system`] — the System-menu layer that sits on top of [`settings`]
//!   (Radio / Video / Audio / Controls / Gameplay / Language / Reset* / Quit /
//!   Developer). (Was `crate::persistence::settings::system_menu`.)

pub mod settings;
pub mod system;

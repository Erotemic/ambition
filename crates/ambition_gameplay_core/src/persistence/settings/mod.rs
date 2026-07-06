//! Settings adapter for the remaining gameplay-core menu/dev IR.
//!
//! Typed persisted settings live in `ambition_persistence::settings`. This
//! module re-exports those shapes on the historical path and keeps the
//! gameplay-core-local menu/model helpers that still read dev state, LDtk
//! hot-reload state, and body tuning.

pub use ambition_persistence::settings::{
    audio, controls, gameplay, platform_paths, update_trigger_edge, video, AssistMode,
    AudioSettings, BackgroundTextureBudget, CameraAspectPolicy, ControlSettings, DashInputMode,
    GameplaySettings, MenuPointerPress, MenuTapMode, ParallaxBudget, ParticleBudget,
    PortalCaptureBudget, ScreenShaderSettings, ShaderBudget, SpriteTextureBudget,
    TextureResolutionScale, TriggerEdgeState, UserSettings, VideoSettings, VisualQualityBudget,
    VisualQualityProfile, VisualQualitySettings,
};

pub mod model;
pub mod persistence;

// Public IR surface used by renderers (the cube today; the pause menu migrates
// onto it next). The IR still lives in `crate::menu::ir` until E1e.
pub use crate::menu::ir::settings::{
    apply_settings_option, settings_menu_model, SettingsOption, SettingsOptionId,
    SettingsOptionKind,
};
pub use crate::menu::ir::system::{
    DevSnapshot, DevToggleId, RadioSnapshot, SystemMenuAction, SystemMenuEntryId, SystemMenuModel,
    SystemMenuTarget, SystemOptionId,
};
pub use model::{
    apply_action, apply_display_mode, DevToggleSnapshot, SettingsAction, SettingsItem,
    SettingsOutcome, SettingsPage,
};

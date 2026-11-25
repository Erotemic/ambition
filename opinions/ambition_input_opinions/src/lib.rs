//! Opt-in shared input policy. An engine consumer can supply a different catalog.
mod defaults;
pub use defaults::{defaults, presets, keyboard_preset, arrows_zxc, wasd_jkl, arrows_qwer, wasd_uipo};

pub fn install(app: &mut bevy::prelude::App) {
    if !app.world().contains_resource::<ambition_input::BindingDefaults>() {
        app.insert_resource(defaults());
    }
}

pub fn recipe(preset: ambition_input::PresetId) -> ambition_input::BindingRecipe { defaults().recipe(preset) }
pub fn gamepad_only() -> ambition_input::BindingRecipe {
    recipe(ambition_input::PresetId::ArrowsZxc).with_sources(ambition_input::BindingSources::GamepadOnly)
}

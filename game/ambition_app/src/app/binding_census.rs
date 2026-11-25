//! Print the effective contextual rows used by the runtime sampler.

use ambition_platformer2d::input::{BindingLayout, InputProfile, MenuButtonIntent,
    MenuShortcutPages, PresetId, contextual_button_rows};
use ambition_platformer2d::persistence::settings::persistence::{load_settings, settings_path};

pub(super) fn print() {
    let settings = load_settings(&settings_path());
    println!("game\tpreset\tcontext\tcontrol\taction\torigin\toutput\tsample");
    let ambition_pages = ambition_platformer2d::menu::backend::InventoryUiBackend::default()
        .effective().is_available();
    let defaults = ambition_input_opinions::defaults();
    for (game, layout, shortcuts) in [
        ("Ambition", BindingLayout::default(), ambition_pages),
        ("Mary-O", BindingLayout::default(), false),
        ("Sanic", BindingLayout::default(), false),
        ("Smash", ambition_demo_smash::input_bindings::layout(), false),
        ("TwinTrack", BindingLayout::default(), false),
    ] {
        let pages = MenuShortcutPages { inventory: shortcuts, map: shortcuts };
        for preset in PresetId::ALL {
            let recipe = defaults.recipe(preset);
            let (_, base) = recipe.resolve();
            let recipe = recipe.with_layout(layout.clone());
            let (_, game_rows) = recipe.resolve();
            let (map, table) = recipe.with_overrides(settings.controls.binding_overrides.clone()).resolve();
            #[cfg(feature = "mobile_touch")]
            let (map, table) = {
                let (mut map, mut table) = (map, table);
                ambition_platformer2d::touch_input::virtual_device::insert_touch_virtual_bindings(&mut map);
                ambition_platformer2d::touch_input::virtual_device::insert_touch_contextual_bindings(&mut table);
                (map, table)
            };
            for profile in [InputProfile::Gameplay, InputProfile::SessionUi, InputProfile::Paused,
                InputProfile::Frontend, InputProfile::Blocked] {
                let prefix = format!("{game}\t{preset:?}\t{profile:?}");
                for (control, action) in contextual_button_rows(&table, profile) {
                    let origin = if action != game_rows.action_for(profile, &control) { "Override/Touch" }
                        else if action != base.action_for(profile, &control) { "Game" } else { "Default" };
                    let output = match action {
                        Some(action) if MenuButtonIntent::from_action(action).is_some_and(|intent| !pages.allows(intent)) => "Unavailable".to_string(),
                        Some(action) => format!("{action:?}"),
                        None => "None".to_string(),
                    };
                    println!("{prefix}\t{control:?}\t{action:?}\t{origin}\t{output}\tButton");
                }
                for (action, inputs) in map.iter_axislike() {
                    if profile.accepts(*action) {
                        for input in inputs { println!("{prefix}\t{input:?}\t{action:?}\tAxis\t{action:?}\tScalar"); }
                    }
                }
                for (action, inputs) in map.iter_dual_axislike() {
                    if profile.accepts(*action) {
                        for input in inputs { println!("{prefix}\t{input:?}\t{action:?}\tAxis\t{action:?}\tVector"); }
                    }
                }
            }
        }
    }
}

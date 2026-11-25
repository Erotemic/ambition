//! Print the bindings from the same recipes as the input host.

use ambition_platformer2d::input::{
    ActionBindings, BindingLayout, BindingRecipe, InputProfile, KeyboardPreset, MenuButtonIntent,
    MenuShortcutPages, Platformer2dInputActionMonolith as Action, contextual_button_rows,
};
use ambition_platformer2d::persistence::settings::persistence::{load_settings, settings_path};

pub(super) fn print() {
    use InputProfile::*;

    let settings = load_settings(&settings_path());
    println!("game\tpreset\tcontext\tcontrol\tbindings\torigin\toutput\tsample");
    let ambition_pages = ambition_platformer2d::menu::backend::InventoryUiBackend::default()
        .effective()
        .is_available();
    for (game, layout, shortcuts) in [
        ("Ambition", BindingLayout::Standard, ambition_pages),
        ("Mary-O", BindingLayout::Standard, false),
        ("Sanic", BindingLayout::Standard, false),
        ("Smash", BindingLayout::Smash, false),
    ] {
        let pages = MenuShortcutPages {
            inventory: shortcuts,
            map: shortcuts,
        };
        for preset in KeyboardPreset::presets() {
            let recipe = BindingRecipe::preset(preset.id);
            let preset_bindings = ActionBindings::from_map(&recipe.build());
            let recipe = recipe.with_layout(layout);
            let layout_bindings = ActionBindings::from_map(&recipe.build());
            let map = recipe
                .with_overrides(settings.controls.binding_overrides.clone())
                .build();
            let user_bindings = ActionBindings::from_map(&map);
            #[cfg(feature = "mobile_touch")]
            let map = {
                let mut map = map;
                ambition_platformer2d::touch_input::virtual_device::insert_touch_virtual_bindings(
                    &mut map,
                );
                map
            };
            let bindings = ActionBindings::from_map(&map);
            for (surface, profile) in [
                ("Gameplay", Gameplay),
                ("GateTerminal", SessionUi),
                ("Pause", Paused),
                ("Frontend", Frontend),
                ("Blocked", Blocked),
            ] {
                let prefix = format!("{game}\t{:?}\t{surface}", preset.id);
                for (control, declared, intent) in contextual_button_rows(&bindings, profile) {
                    let names = declared
                        .iter()
                        .map(|action| format!("{action:?}"))
                        .collect::<Vec<_>>()
                        .join(", ");
                    let origins = declared
                        .iter()
                        .map(|action| {
                            if !user_bindings.controls(action).contains(&control) {
                                "Touch"
                            } else if !layout_bindings.controls(action).contains(&control) {
                                "Override"
                            } else if !preset_bindings.controls(action).contains(&control) {
                                "Layout"
                            } else {
                                "Preset"
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(", ");
                    let (output, sample) = if let Some(intent) =
                        intent.filter(|intent| pages.allows(*intent))
                    {
                        let sample = match intent {
                            MenuButtonIntent::Confirm | MenuButtonIntent::Back => "Edge and held",
                            _ => "Edge",
                        };
                        (format!("{intent:?}"), sample)
                    } else {
                        (remaining_actions(profile, &declared), "Button")
                    };
                    println!("{prefix}\t{control:?}\t{names}\t{origins}\t{output}\t{sample}");
                }
                let mut axes = map
                    .iter_axislike()
                    .flat_map(|(action, inputs)| {
                        inputs
                            .iter()
                            .map(move |input| (format!("{input:?}"), *action, "Scalar"))
                    })
                    .chain(map.iter_dual_axislike().flat_map(|(action, inputs)| {
                        inputs
                            .iter()
                            .map(move |input| (format!("{input:?}"), *action, "Vector"))
                    }))
                    .collect::<Vec<_>>();
                axes.sort_by_key(|(control, action, _)| (control.clone(), format!("{action:?}")));
                for (control, action, sample) in axes {
                    let output = remaining_actions(profile, &[action]);
                    let origin = if control.starts_with("Touch") {
                        "Touch"
                    } else {
                        "Preset"
                    };
                    println!("{prefix}\t{control}\t{action:?}\t{origin}\t{output}\t{sample}");
                }
            }
        }
    }
}

fn remaining_actions(profile: InputProfile, declared: &[Action]) -> String {
    use Action::*;
    use InputProfile::*;
    let enabled = |action: &Action| match profile {
        Gameplay => !action.is_menu_only() && !matches!(action, Start | Map | Inventory),
        SessionUi | Paused | Frontend => matches!(
            action,
            MenuNavigateUp
                | MenuNavigateDown
                | MenuNavigateLeft
                | MenuNavigateRight
                | MenuPageLeft
                | MenuPageRight
                | MenuStick
        ),
        Blocked => false,
    };
    let output = declared
        .iter()
        .filter(|action| enabled(action))
        .map(|action| match action {
            Reset => "Development Reset".to_string(),
            _ => format!("{action:?}"),
        })
        .collect::<Vec<_>>()
        .join(", ");
    if output.is_empty() {
        "None".to_string()
    } else {
        output
    }
}

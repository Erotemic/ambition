#[cfg(feature = "input")]
use super::input::close_inventory;
use super::*;

/// Mouse / touch input for the adventure-menu panel.
///
/// Touch-native tabs and Back are handled here, while item-row taps still route
/// through `MenuTapMode::resolve_press`. The keyboard/gamepad path remains in
/// `inventory_input`, so the UI can be operated without special raw-device
/// knowledge.
#[cfg(feature = "input")]
pub fn inventory_pointer_input(
    mode: Res<State<GameMode>>,
    mut next_mode: ResMut<NextState<GameMode>>,
    mut state: ResMut<InventoryUiState>,
    user_settings: Res<crate::persistence::settings::UserSettings>,
    rows: Query<(&Interaction, &InventoryItemRow), Changed<Interaction>>,
    tabs: Query<(&Interaction, &InventoryTabButton), Changed<Interaction>>,
    back_buttons: Query<&Interaction, (With<InventoryBackButton>, Changed<Interaction>)>,
) {
    if !state.visible {
        return;
    }

    for interaction in &back_buttons {
        if matches!(interaction, Interaction::Pressed) {
            close_inventory(&mut state, mode.get(), &mut next_mode);
            return;
        }
    }

    for (interaction, tab_button) in &tabs {
        if matches!(interaction, Interaction::Pressed) {
            state.set_tab(tab_button.tab);
            return;
        }
    }

    if state.tab != InventoryTab::Items {
        return;
    }

    let tap_mode = user_settings.controls.menu_tap_mode;
    let items = ItemKind::ALL;
    for (interaction, row) in &rows {
        let Some(index) = items.iter().position(|k| k == &row.kind) else {
            continue;
        };
        match interaction {
            Interaction::Hovered => {
                if state.selected != index {
                    state.selected = index;
                }
            }
            Interaction::Pressed => {
                let press =
                    tap_mode.resolve_press(index, state.selected, false, &mut state.pointer_armed);
                state.selected = index;
                if matches!(press, crate::persistence::settings::MenuPointerPress::Confirm) {
                    state.pointer_confirm = true;
                }
            }
            Interaction::None => {}
        }
    }
}

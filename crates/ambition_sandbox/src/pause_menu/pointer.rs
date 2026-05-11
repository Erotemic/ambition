use super::model::{RADIO_VISIBLE_ROWS, SETTINGS_VISIBLE_ROWS};
use super::*;

/// Mouse / touch input for the pause menu and its settings sub-pages.
///
/// Hover (mouse-over) moves the highlight; press routes through
/// `MenuTapMode::resolve_press` to decide whether to also confirm.
/// Confirms are deferred to `pause_menu_navigate` via
/// `state.pointer_confirm` so the rest of the menu pipeline keeps a
/// single confirm path.
#[cfg(feature = "input")]
pub fn pause_menu_pointer_input(
    mode: Res<State<GameMode>>,
    inventory: Res<InventoryUiState>,
    user_settings: Res<UserSettings>,
    mut state: ResMut<PauseMenuState>,
    top_items: Query<(&Interaction, &PauseMenuItem), Changed<Interaction>>,
    settings_rows: Query<(&Interaction, &SettingsRowSlot), Changed<Interaction>>,
    #[cfg(feature = "audio")] library: Res<AudioLibrary>,
) {
    if !matches!(mode.get(), GameMode::Paused) {
        return;
    }
    if inventory.visible {
        return;
    }
    let tap_mode = user_settings.controls.menu_tap_mode;

    match state.page {
        PauseMenuPage::Top => {
            let items = PauseMenuItem::ALL;
            for (interaction, item) in &top_items {
                let Some(index) = items.iter().position(|i| i == item) else {
                    continue;
                };
                let update = resolve_selectable_row_interaction(
                    interaction,
                    index,
                    state.selected,
                    tap_mode,
                    item.is_destructive(),
                    state.pointer_armed,
                );
                state.selected = update.selected;
                state.pointer_armed = update.pointer_armed;
                if matches!(update.outcome, RowPointerOutcome::Confirmed) {
                    state.pointer_confirm = true;
                }
            }
        }
        PauseMenuPage::Settings(page) => {
            let rows = SettingsItem::rows_for(page);
            for (interaction, slot) in &settings_rows {
                let Some(index) = visible_row_index(
                    slot.index,
                    state.selected,
                    rows.len(),
                    SETTINGS_VISIBLE_ROWS,
                ) else {
                    continue;
                };
                handle_row_pointer_interaction(interaction, index, tap_mode, &mut state);
            }
        }
        PauseMenuPage::Radio => {
            #[cfg(feature = "audio")]
            let row_count = library.track_count();
            #[cfg(not(feature = "audio"))]
            let row_count = 1;
            for (interaction, slot) in &settings_rows {
                let Some(index) =
                    visible_row_index(slot.index, state.selected, row_count, RADIO_VISIBLE_ROWS)
                else {
                    continue;
                };
                handle_row_pointer_interaction(interaction, index, tap_mode, &mut state);
            }
        }
    }
}

fn handle_row_pointer_interaction(
    interaction: &Interaction,
    index: usize,
    tap_mode: crate::settings::MenuTapMode,
    state: &mut PauseMenuState,
) {
    let update = resolve_selectable_row_interaction(
        interaction,
        index,
        state.selected,
        tap_mode,
        false,
        state.pointer_armed,
    );
    state.selected = update.selected;
    state.pointer_armed = update.pointer_armed;
    if matches!(update.outcome, RowPointerOutcome::Confirmed) {
        state.pointer_confirm = true;
    }
}

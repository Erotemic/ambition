use super::effects::apply_item_effect;
use super::*;

#[cfg(feature = "input")]
pub fn inventory_input(
    menu: Res<MenuControlFrame>,
    mut state: ResMut<InventoryUiState>,
    mode: Res<State<GameMode>>,
    mut next_mode: ResMut<NextState<GameMode>>,
    mut inventory: ResMut<PlayerInventory>,
    mut runtime: ResMut<SandboxRuntime>,
) {
    // Toggle the adventure menu directly from gameplay using the semantic menu
    // frame. Keyboard/gamepad still feed this through the Inventory action;
    // touch can also reach the same panel through the pause menu.
    if menu.inventory {
        if state.visible {
            close_inventory(&mut state, mode.get(), &mut next_mode);
        } else if matches!(mode.get(), GameMode::Playing | GameMode::Paused) {
            state.reset_for_open(matches!(mode.get(), GameMode::Paused));
            if matches!(mode.get(), GameMode::Playing) {
                next_mode.set(GameMode::Paused);
            }
        }
    }

    if !state.visible {
        // Drop stale pointer signals so reopening does not auto-fire.
        state.pointer_confirm = false;
        state.pointer_armed = None;
        return;
    }

    if menu.back || menu.start {
        close_inventory(&mut state, mode.get(), &mut next_mode);
        return;
    }

    if menu.left {
        state.previous_tab();
    }
    if menu.right {
        state.next_tab();
    }

    match state.tab {
        InventoryTab::Items => {
            handle_item_tab_input(&menu, &mut state, &mut inventory, &mut runtime)
        }
        InventoryTab::Map | InventoryTab::Quests => handle_text_tab_input(&menu, &mut state),
    }
}

#[cfg(feature = "input")]
pub(super) fn close_inventory(
    state: &mut InventoryUiState,
    mode: &GameMode,
    next_mode: &mut NextState<GameMode>,
) {
    let opened_from_pause = state.opened_from_pause;
    state.close();
    if !opened_from_pause && matches!(mode, GameMode::Paused) {
        next_mode.set(GameMode::Playing);
    }
}

#[cfg(feature = "input")]
fn handle_item_tab_input(
    menu: &MenuControlFrame,
    state: &mut InventoryUiState,
    inventory: &mut PlayerInventory,
    runtime: &mut SandboxRuntime,
) {
    let total = ItemKind::ALL.len();
    let mut nav_up = menu.up;
    let mut nav_down = menu.down;
    let steps = menu.vertical_scroll_steps();
    if steps > 0 {
        nav_up = true;
    } else if steps < 0 {
        nav_down = true;
    }
    if nav_up {
        state.selected = (state.selected + total - 1) % total;
    }
    if nav_down {
        state.selected = (state.selected + 1) % total;
    }
    // Keyboard / gamepad / gesture navigation clears any tap-armed row so the
    // next pointer press starts fresh.
    if nav_up || nav_down || menu.scroll_y.abs() >= 0.5 {
        state.pointer_armed = None;
    }

    let confirm = menu.select || state.pointer_confirm;
    state.pointer_confirm = false;
    if confirm {
        let kind = ItemKind::ALL[state.selected];
        if inventory.count(kind) > 0 {
            apply_item_effect(kind, inventory, runtime);
        }
    }
}

#[cfg(feature = "input")]
fn handle_text_tab_input(menu: &MenuControlFrame, state: &mut InventoryUiState) {
    let mut delta: isize = 0;
    if menu.up {
        delta -= 1;
    }
    if menu.down {
        delta += 1;
    }
    // Positive scroll_y means user moved content up / requested previous rows
    // in the MenuControlFrame convention used by pause menu navigation.
    delta -= menu.vertical_scroll_steps() as isize;
    if delta < 0 {
        state.content_scroll = state.content_scroll.saturating_sub((-delta) as usize);
    } else if delta > 0 {
        state.content_scroll = state.content_scroll.saturating_add(delta as usize).min(256);
    }
}

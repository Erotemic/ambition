//! Keyboard / gamepad + pointer/touch input for the OoT item grid.

use bevy::prelude::*;

use super::effects::{self, MenuAction};
use super::state::GridMenuState;
use super::ui::{GridBackButton, GridSlot};
use crate::brain::ActionSet;
use crate::game_mode::GameMode;
use crate::input::MenuControlFrame;
use crate::item_pickup::{equip_held_spec, held_spec_for_item, unequip_held, StashedActionSet};
use crate::items::{Item, OwnedItems};
use crate::player::{PlayerEntity, PlayerHealRequested, PlayerMana, PrimaryPlayer};
use crate::ui_nav::{resolve_selectable_row_interaction, RowPointerOutcome};

/// One health cell restores this much HP; one mana cell this much mana. Sandbox
/// values — a real balance pass is just a number change.
const HEALTH_CELL_HEAL: i32 = 4;
const MANA_CELL_RESTORE: f32 = 40.0;

/// Toggle, navigate, and confirm the OoT item grid via the semantic menu frame.
///
/// Visibility lives in the shared `InventoryUiState.visible` flag (see
/// [`GridMenuState`]) so the pause menu's existing suppression keeps working.
#[allow(clippy::too_many_arguments)]
pub fn grid_menu_input(
    menu: Res<MenuControlFrame>,
    // When the Cube backend renders the inventory, it owns navigation / confirm via
    // `kaleidoscope_focus_nav`; the grid still owns the shared open/close toggle (the
    // Inventory button) so the menu can be opened regardless of backend.
    backend: Option<Res<crate::lunex_kaleidoscope_app::InventoryUiBackend>>,
    mut state: ResMut<GridMenuState>,
    mut overlay: ResMut<crate::inventory::InventoryUiState>,
    mode: Res<State<GameMode>>,
    mut next_mode: ResMut<NextState<GameMode>>,
    mut owned: ResMut<OwnedItems>,
    mut commands: Commands,
    mut players: MenuEffectPlayers,
    mut mana_q: MenuEffectManaQuery,
    mut heals: MessageWriter<PlayerHealRequested>,
) {
    if menu.inventory {
        if overlay.visible {
            close_grid_menu(&mut state, &mut overlay, mode.get(), &mut next_mode);
        } else if matches!(mode.get(), GameMode::Playing | GameMode::Paused) {
            overlay.visible = true;
            state.open(matches!(mode.get(), GameMode::Paused));
            if matches!(mode.get(), GameMode::Playing) {
                next_mode.set(GameMode::Paused);
            }
        }
    }

    if !overlay.visible {
        state.pointer_confirm = false;
        state.pointer_armed = None;
        return;
    }

    // The Cube frontend owns navigation / confirm / back while it's active; the grid
    // only kept the shared open toggle above. Bail before grid nav so the two
    // frontends don't both act on the same frame's input.
    let kaleidoscope_active = backend
        .map(|b| *b == crate::lunex_kaleidoscope_app::InventoryUiBackend::LunexKaleidoscope)
        .unwrap_or(false);
    if kaleidoscope_active {
        state.pointer_confirm = false;
        state.pointer_armed = None;
        return;
    }

    if menu.back || menu.start {
        close_grid_menu(&mut state, &mut overlay, mode.get(), &mut next_mode);
        return;
    }

    // Grid navigation (wraps within row/column).
    let mut dcol: isize = 0;
    let mut drow: isize = 0;
    if menu.left {
        dcol -= 1;
    }
    if menu.right {
        dcol += 1;
    }
    if menu.up {
        drow -= 1;
    }
    if menu.down {
        drow += 1;
    }
    if state.move_cursor(dcol, drow) {
        state.focus.mark_keyboard();
        state.pointer_armed = None;
    }

    let confirm = menu.select || state.pointer_confirm;
    state.pointer_confirm = false;
    if confirm {
        let item = state.selected_item();
        let action = effects::decide(item, &owned);
        apply_menu_action(
            action,
            &mut owned,
            &mut commands,
            &mut players,
            &mut mana_q,
            &mut heals,
        );
        state.status = effects::status_for(action);
    }
}

/// The player query shape every menu-effect dispatch shares (grid + cube). The
/// lifetimes stay free so callers (systems with their own `'w`/`'s`) can pass
/// `&mut their_query` without the borrow escaping to `'static`.
pub(crate) type MenuEffectPlayers<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static mut ActionSet,
        Option<&'static StashedActionSet>,
    ),
    (With<PlayerEntity>, With<PrimaryPlayer>),
>;

/// The player-mana query shape shared by every menu-effect dispatch.
pub(crate) type MenuEffectManaQuery<'w, 's> =
    Query<'w, 's, &'static mut PlayerMana, (With<PlayerEntity>, With<PrimaryPlayer>)>;

/// Decide and apply the effect of confirming `item` (equip / unequip / use /
/// inspect). The ONE place both the Bevy-UI grid and the 3D cube turn an item
/// confirmation into ECS side effects — neither duplicates the portal/equip/heal
/// logic. Returns the decided [`MenuAction`] so callers can surface its status.
pub(crate) fn dispatch_item_confirm(
    item: Item,
    owned: &mut OwnedItems,
    commands: &mut Commands,
    players: &mut MenuEffectPlayers<'_, '_>,
    mana_q: &mut MenuEffectManaQuery<'_, '_>,
    heals: &mut MessageWriter<PlayerHealRequested>,
) -> MenuAction {
    let action = effects::decide(item, owned);
    apply_menu_action(action, owned, commands, players, mana_q, heals);
    action
}

/// Turn a decided [`MenuAction`] into its ECS side effects.
pub(crate) fn apply_menu_action(
    action: MenuAction,
    owned: &mut OwnedItems,
    commands: &mut Commands,
    players: &mut MenuEffectPlayers<'_, '_>,
    mana_q: &mut MenuEffectManaQuery<'_, '_>,
    heals: &mut MessageWriter<PlayerHealRequested>,
) {
    match action {
        MenuAction::Equip(item) => {
            // The portal gun equips via its own component; other weapons via a
            // HeldItemSpec. Bail early if the item is neither. With the portal
            // mechanic compiled out, the Portal Gun roster slot still exists but
            // has no equip path, so it behaves like an unwired weapon.
            #[cfg(feature = "portal")]
            let is_portal_gun = item == Item::PortalGun;
            #[cfg(not(feature = "portal"))]
            let is_portal_gun = false;
            let held_spec = held_spec_for_item(item);
            if !is_portal_gun && held_spec.is_none() {
                return;
            }
            if let Ok((player, mut action_set, stashed)) = players.single_mut() {
                // Clear whatever weapon is currently held (a held item OR the
                // portal gun) so we re-stash the true base, then equip the new one.
                if stashed.is_some() {
                    unequip_held(commands, player, &mut action_set, stashed);
                    #[cfg(feature = "portal")]
                    commands.entity(player).remove::<crate::portal::PortalGun>();
                }
                #[cfg(feature = "portal")]
                if is_portal_gun {
                    crate::ambition_content::portal::equip_portal_gun(
                        commands,
                        player,
                        &mut action_set,
                    );
                } else if let Some(spec) = held_spec {
                    equip_held_spec(commands, player, &mut action_set, spec);
                }
                #[cfg(not(feature = "portal"))]
                if let Some(spec) = held_spec {
                    equip_held_spec(commands, player, &mut action_set, spec);
                }
                owned.set_equipped(Some(item));
            }
        }
        MenuAction::Unequip(_item) => {
            if let Ok((player, mut action_set, stashed)) = players.single_mut() {
                // Detach both possible weapon front-ends (held item + portal gun).
                unequip_held(commands, player, &mut action_set, stashed);
                #[cfg(feature = "portal")]
                commands.entity(player).remove::<crate::portal::PortalGun>();
            }
            owned.set_equipped(None);
        }
        MenuAction::UseConsumable(Item::HealthCell) => {
            if owned.take(Item::HealthCell, 1) > 0 {
                heals.write(PlayerHealRequested::new(HEALTH_CELL_HEAL));
            }
        }
        MenuAction::UseConsumable(Item::ManaCell) => {
            if owned.take(Item::ManaCell, 1) > 0 {
                if let Ok(mut mana) = mana_q.single_mut() {
                    mana.meter.refill(MANA_CELL_RESTORE);
                }
            }
        }
        MenuAction::UseConsumable(_) | MenuAction::Inspect(_) | MenuAction::NotOwned(_) => {}
    }
}

/// Close + restore the prior game mode (mirrors the legacy adventure menu).
pub(super) fn close_grid_menu(
    state: &mut GridMenuState,
    overlay: &mut crate::inventory::InventoryUiState,
    mode: &GameMode,
    next_mode: &mut NextState<GameMode>,
) {
    let opened_from_pause = state.opened_from_pause;
    state.close();
    overlay.visible = false;
    if !opened_from_pause && matches!(mode, GameMode::Paused) {
        next_mode.set(GameMode::Playing);
    }
}

/// Mouse / touch input for the grid. Slot taps route through the same
/// `MenuTapMode::resolve_press` policy the adventure menu uses, so touch users
/// get select-then-tap by default while mouse/desktop can confirm immediately.
pub fn grid_menu_pointer_input(
    mode: Res<State<GameMode>>,
    mut next_mode: ResMut<NextState<GameMode>>,
    mut state: ResMut<GridMenuState>,
    mut overlay: ResMut<crate::inventory::InventoryUiState>,
    user_settings: Res<crate::persistence::settings::UserSettings>,
    slots: Query<(&Interaction, &GridSlot), Changed<Interaction>>,
    back_buttons: Query<&Interaction, (With<GridBackButton>, Changed<Interaction>)>,
) {
    if !overlay.visible {
        return;
    }

    for interaction in &back_buttons {
        if matches!(interaction, Interaction::Pressed) {
            close_grid_menu(&mut state, &mut overlay, mode.get(), &mut next_mode);
            return;
        }
    }

    let tap_mode = user_settings.controls.menu_tap_mode;
    for (interaction, slot) in &slots {
        let index = slot.item.index();
        match interaction {
            Interaction::Hovered => {
                let update = resolve_selectable_row_interaction(
                    interaction,
                    index,
                    state.cursor,
                    tap_mode,
                    false,
                    state.pointer_armed,
                    state.focus,
                );
                state.cursor = update.selected;
                state.pointer_armed = update.pointer_armed;
                state.focus = update.focus;
                if matches!(update.outcome, RowPointerOutcome::Confirmed) {
                    state.pointer_confirm = true;
                }
            }
            Interaction::Pressed => {
                let press =
                    tap_mode.resolve_press(index, state.cursor, false, &mut state.pointer_armed);
                state.cursor = index;
                state.focus.mark_pointer(index);
                if matches!(
                    press,
                    crate::persistence::settings::MenuPointerPress::Confirm
                ) {
                    state.pointer_confirm = true;
                }
            }
            Interaction::None => {}
        }
    }
}

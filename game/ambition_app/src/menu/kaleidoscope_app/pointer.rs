//! Cube-menu pointer interaction: press/move/release for picking a face cell,
//! including the drag-vs-click discrimination.

use super::*;

/// Feature E: record the start of a pointer press on a cube control so a
/// press-then-drag-away can be CANCELLED (no activation). Stores the pressed entity
/// + the press origin; `kaleidoscope_pointer_move` marks it cancelled once the
/// pointer drags past the tap threshold, and `kaleidoscope_pointer_release` honours
/// that. Mouse OR touch (same `Pointer<Press>` path).
pub(crate) fn kaleidoscope_pointer_press(
    press: On<Pointer<Press>>,
    backend: Res<InventoryUiBackend>,
    controls: Query<(
        &AmbitionMenuControl<MenuPageAction>,
        &ambition_platformer2d::menu::MenuSurface,
    )>,
    mut menus: Query<(
        &ambition_platformer2d::inventory_ui::InventoryUiState,
        &mut ambition_platformer2d::menu::MenuInteraction<MenuPageAction>,
    )>,
) {
    if backend.effective() != InventoryUiBackend::LunexKaleidoscope {
        return;
    }
    let Ok((control, surface)) = controls.get(press.entity) else {
        return;
    };
    let Ok((state, mut interaction)) = menus.get_mut(surface.0) else {
        return;
    };
    if !state.visible {
        return;
    }
    if let Some(action) = control.action {
        interaction.press_pointer(
            press.pointer_id,
            control.focus,
            action,
            press.entity,
            press.pointer_location.position,
        );
    }
}

/// Pointer motion (mouse/touch) over a cube control: move the focus cursor to it.
/// We listen to `Pointer<Move>` instead of `Pointer<Over>` so a menu that opens
/// under a parked mouse does not immediately select whatever is already under the
/// cursor. A real move is required before hover can take ownership.
///
/// Two guards (both essential), mirroring the grid's `MenuFocusState`:
///
/// 1. Semantic dedup. A moving pointer can emit several events while it stays
///    over the same control. We compare the hovered focus against `last_pointer_focus`
///    and bail when unchanged, so the cursor only reacts once per logical focus.
/// 2. Pointer-vs-keyboard ownership. The pointer only re-claims the cursor when
///    it moves onto a genuinely different control. This fixes "can't move away from
///    the hovered option."
pub(crate) fn kaleidoscope_pointer_move(
    move_: On<Pointer<Move>>,
    controls: Query<(
        &AmbitionMenuControl<MenuPageAction>,
        &ambition_platformer2d::menu::MenuSurface,
    )>,
    mut menus: Query<(
        &ambition_platformer2d::inventory_ui::InventoryUiState,
        &ActiveMenuPages<MenuPage, MenuPageAction>,
        &KaleidoscopeSystemNav,
        &VisualQualityConfirmState,
        &mut KaleidoscopeCursor,
        &CachedSystemMenu,
    )>,
    settings: Res<UserSettings>,
    devices: Res<ambition_platformer2d::input::SeatActiveDevices>,
    snapshot: SystemMenuSnapshotParams,
    mut interactions: Query<&mut ambition_platformer2d::menu::MenuInteraction<MenuPageAction>>,
    mut sfx: SfxWriter,
) {
    // A drag must cancel its press even if the backend or focus source changed.
    for mut interaction in &mut interactions {
        interaction.move_pointer(move_.pointer_id, move_.pointer_location.position);
    }
    if snapshot.backend.effective() != InventoryUiBackend::LunexKaleidoscope
        || devices.machine() != ambition_platformer2d::input::ActiveDevice::Mouse
    {
        return;
    }
    let Ok((control, surface)) = controls.get(move_.entity) else {
        return;
    };
    let Ok((state, pages, system_nav, quality_confirm, mut cursor, cache)) =
        menus.get_mut(surface.0)
    else {
        return;
    };
    if !state.visible {
        return;
    }
    let Some(active_page) = pages.active else {
        return;
    };
    let Some(action) = control.action else {
        return;
    };
    let next = match focus_without_system_model(action, active_page) {
        Some(focus) => focus,
        None if cache.rows_are_current_for(system_nav.open_entry) => {
            focus_for_action(action, active_page, &cache.rows)
        }
        None => {
            // An observer can run before the cache sees a changed drill-down.
            let model = SystemMenuModel::build(
                &settings,
                &snapshot.radio_snapshot(),
                &snapshot.dev_snapshot(),
            );
            let rows = system_rows_with_quality_prompt(
                &model,
                system_nav.open_entry,
                quality_confirm.pending(),
            );
            focus_for_action(action, active_page, &rows)
        }
    };
    if cursor.last_pointer_focus == Some(next) {
        return;
    }
    cursor.last_pointer_focus = Some(next);
    if cursor.focus != next {
        cursor.focus = next;
        cursor.owner = FocusSource::Pointer;
        play_ui(&mut sfx, ambition_platformer2d::sfx::ids::UI_MENU_MOVE);
    }
}

/// Pointer release (mouse/touch) dispatches the action armed at press time.
///
/// Cube controls can be despawned/rebuilt between press and release, so dispatch is
/// entity-independent: store the action on `Pointer<Press>` and consume it on
/// release. Drag cancellation still wins when movement exceeds the tap threshold.
#[allow(clippy::too_many_arguments)]
pub(crate) fn kaleidoscope_pointer_release(
    release: On<Pointer<Release>>,
    mut menus: Query<(
        Entity,
        &ambition_platformer2d::inventory_ui::InventoryUiState,
        &ActiveMenuPages<MenuPage, MenuPageAction>,
        &mut ambition_platformer2d::menu::MenuInteraction<MenuPageAction>,
        &mut KaleidoscopeCursor,
        &KaleidoscopeSystemNav,
        &VisualQualityConfirmState,
    )>,
    settings: Res<UserSettings>,
    mut activated: MessageWriter<MenuActionActivated<MenuPageAction>>,
    // Still needed for the focus resolve below (`system.model`), not for dispatch.
    system: SystemMenuParams,
) {
    if system.backend() != InventoryUiBackend::LunexKaleidoscope {
        return;
    }
    for (menu, state, pages, mut interaction, mut cursor, system_nav, quality_confirm) in &mut menus
    {
        let Some(action) = interaction.release_pointer(release.pointer_id) else {
            continue;
        };
        if !state.visible {
            continue;
        }
        if let Some(active_page) = pages.active {
            // Same rule as the hover path: build the settings IR only for the actions
            // whose focus is a System ROW INDEX.
            let next = match focus_without_system_model(action, active_page) {
                Some(focus) => focus,
                None => {
                    let model = system.model(&settings);
                    let rows = system_rows_with_quality_prompt(
                        &model,
                        system_nav.open_entry,
                        quality_confirm.pending(),
                    );
                    focus_for_action(action, active_page, &rows)
                }
            };
            cursor.focus = next;
            cursor.owner = FocusSource::Pointer;
            cursor.last_pointer_focus = Some(next);
        }
        activated.write(MenuActionActivated { menu, action });
    }
}

//! Cube-menu scroll handling: the System-face row count + the wheel and
//! drag scroll appliers that clamp the scroll override.

use super::*;

/// The live System row count for the current drill-down state (0 outside the System
/// face). Shared by the wheel + drag scroll appliers to clamp the scroll position.
fn system_row_count(
    pages: &ActiveMenuPages<MenuPage, MenuPageAction>,
    system_nav: &KaleidoscopeSystemNav,
    settings: &UserSettings,
    snapshot: &SystemMenuSnapshotParams,
    pending_quality: Option<VisualQualityProfile>,
) -> usize {
    if pages.active != Some(MenuPage::System) {
        return 0;
    }
    let model = SystemMenuModel::build(
        settings,
        &snapshot.radio_snapshot(),
        &snapshot.dev_snapshot(),
    );
    system_rows_with_quality_prompt(&model, system_nav.open_entry, pending_quality).len()
}

/// Feature D: the MOUSE WHEEL scrolls the System window (the visible rows), NOT the
/// keyboard selection. Each wheel notch moves the scroll override by one row,
/// clamped to `[0, system_max_window_start]`. The cursor/selection is untouched — a
/// later keyboard move clears the override and the window snaps back to the cursor.
/// Only acts on a scrollable System list (more rows than fit); a short list ignores
/// the wheel. Mouse OR touchpad scroll both arrive as `MouseWheel`.
///
/// A finger dragged over the list scrolls the same window, so the rows follow
/// the finger. The drag arrives on the menu frame, already folded into row
/// steps; the tap it began as is cancelled by the pointer observers.
pub(crate) fn kaleidoscope_scroll_wheel(
    backend: Res<InventoryUiBackend>,
    mut menus: Query<(
        Entity,
        &ambition_platformer2d::inventory_ui::InventoryUiState,
        &ambition_platformer2d::inventory_ui::InventoryMenu,
        &ActiveMenuPages<MenuPage, MenuPageAction>,
        &KaleidoscopeSystemNav,
        &VisualQualityConfirmState,
        &KaleidoscopeCursor,
        &mut KaleidoscopeScroll,
    )>,
    settings: Res<UserSettings>,
    snapshot: SystemMenuSnapshotParams,
    mut wheel: crate::menu::regions::MenuWheelInput,
    menu_frames: Res<ambition_platformer2d::input::SeatMenuFrames>,
) {
    if backend.effective() != InventoryUiBackend::LunexKaleidoscope {
        wheel.clear();
        return;
    }
    wheel.read();
    for (menu, overlay, menu_owner, pages, system_nav, quality_confirm, cursor, mut scroll) in
        &mut menus
    {
        if !overlay.visible {
            continue;
        }
        let mut steps = wheel.steps_for(menu);
        // A drag up the screen is negative on the frame and moves the window down.
        steps -= menu_frames
            .for_seat(menu_owner.owner.slot())
            .drag_scroll_y
            .round() as i32;
        if steps == 0 {
            continue;
        }
        let total = system_row_count(
            &pages,
            &system_nav,
            &settings,
            &snapshot,
            quality_confirm.pending(),
        );
        if total <= SYSTEM_VISIBLE_ROWS {
            continue; // nothing to scroll
        }
        let max = system_max_window_start(total) as i32;
        // Seed from the effective start so the first wheel notch moves relative to what
        // is currently shown (cursor-derived window) rather than jumping to 0.
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
        let current =
            system_effective_window_start(&rows, cursor.focus, scroll.system_window_start) as i32;
        let next = (current + steps).clamp(0, max) as usize;
        scroll.system_window_start = Some(next);
    }
}

/// Apply the lib's backend-agnostic scrollbar-drag fraction (`0..=1`) to the
/// host System-menu window. Selection is unchanged; only the visible rows move.
pub(crate) fn kaleidoscope_apply_scroll_drag(
    backend: Res<InventoryUiBackend>,
    mut menus: Query<(
        &ambition_platformer2d::inventory_ui::InventoryUiState,
        &ActiveMenuPages<MenuPage, MenuPageAction>,
        &KaleidoscopeSystemNav,
        &VisualQualityConfirmState,
        &mut KaleidoscopeScroll,
    )>,
    settings: Res<UserSettings>,
    snapshot: SystemMenuSnapshotParams,
    mut dragged: MessageReader<ambition_platformer2d::menu::MenuScrollDragged>,
) {
    if backend.effective() != InventoryUiBackend::LunexKaleidoscope {
        dragged.clear();
        return;
    }
    for drag in dragged.read() {
        let Ok((state, pages, system_nav, quality_confirm, mut scroll)) = menus.get_mut(drag.menu)
        else {
            continue;
        };
        if !state.visible {
            continue;
        }
        let total = system_row_count(
            pages,
            system_nav,
            &settings,
            &snapshot,
            quality_confirm.pending(),
        );
        if let Some(start) = scroll_fraction_to_window_start(total, drag.fraction.clamp(0.0, 1.0)) {
            scroll.system_window_start = Some(start);
        }
    }
}

//! Cube-menu scroll handling: the System-face row count + the wheel and
//! drag scroll appliers that clamp the scroll override.
//!
//! Split out of the kaleidoscope menu host (2026-06-15).

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
pub(crate) fn kaleidoscope_scroll_wheel(
    backend: Res<InventoryUiBackend>,
    ui_state: Option<Res<ambition::inventory_ui::InventoryUiState>>,
    pages: Res<ActiveMenuPages<MenuPage, MenuPageAction>>,
    system_nav: Res<KaleidoscopeSystemNav>,
    settings: Res<UserSettings>,
    snapshot: SystemMenuSnapshotParams,
    quality_confirm: Res<VisualQualityConfirmState>,
    cursor: Res<KaleidoscopeCursor>,
    mut scroll: ResMut<KaleidoscopeScroll>,
    mut wheel: MessageReader<bevy::input::mouse::MouseWheel>,
) {
    let open = ui_state.map(|s| s.visible).unwrap_or(false);
    if backend.effective() != InventoryUiBackend::LunexKaleidoscope || !open {
        wheel.clear();
        return;
    }
    // Sum this frame's wheel deltas into integer row steps (wheel up = scroll up).
    let mut steps = 0i32;
    for ev in wheel.read() {
        steps += if ev.y > 0.0 {
            -1
        } else if ev.y < 0.0 {
            1
        } else {
            0
        };
    }
    if steps == 0 {
        return;
    }
    let total = system_row_count(
        &pages,
        &system_nav,
        &settings,
        &snapshot,
        quality_confirm.pending(),
    );
    if total <= SYSTEM_VISIBLE_ROWS {
        return; // nothing to scroll
    }
    let max = system_max_window_start(total) as i32;
    // Seed from the effective start so the first wheel notch moves relative to what
    // is currently shown (cursor-derived window) rather than jumping to 0.
    let model = SystemMenuModel::build(
        &settings,
        &snapshot.radio_snapshot(),
        &snapshot.dev_snapshot(),
    );
    let rows =
        system_rows_with_quality_prompt(&model, system_nav.open_entry, quality_confirm.pending());
    let current =
        system_effective_window_start(&rows, cursor.focus, scroll.system_window_start) as i32;
    let next = (current + steps).clamp(0, max) as usize;
    scroll.system_window_start = Some(next);
}

/// Apply the lib's backend-agnostic scrollbar-drag fraction (`0..=1`) to the
/// host System-menu window. Selection is unchanged; only the visible rows move.
pub(crate) fn kaleidoscope_apply_scroll_drag(
    backend: Res<InventoryUiBackend>,
    ui_state: Option<Res<ambition::inventory_ui::InventoryUiState>>,
    pages: Res<ActiveMenuPages<MenuPage, MenuPageAction>>,
    system_nav: Res<KaleidoscopeSystemNav>,
    settings: Res<UserSettings>,
    snapshot: SystemMenuSnapshotParams,
    quality_confirm: Res<VisualQualityConfirmState>,
    mut scroll: ResMut<KaleidoscopeScroll>,
    mut dragged: MessageReader<ambition::menu::MenuScrollDragged>,
) {
    let open = ui_state.map(|s| s.visible).unwrap_or(false);
    if backend.effective() != InventoryUiBackend::LunexKaleidoscope || !open {
        dragged.clear();
        return;
    }
    // Use the LAST drag fraction this frame (the freshest pointer position).
    let Some(fraction) = dragged.read().last().map(|d| d.fraction.clamp(0.0, 1.0)) else {
        return;
    };
    let total = system_row_count(
        &pages,
        &system_nav,
        &settings,
        &snapshot,
        quality_confirm.pending(),
    );
    let result = scroll_fraction_to_window_start(total, fraction);
    if let Some(start) = result {
        scroll.system_window_start = Some(start);
    }
}

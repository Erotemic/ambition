//! Cube-menu pointer interaction: press/move/release for picking a face cell,
//! including the drag-vs-click discrimination.
//!
//! Split out of the kaleidoscope menu host (2026-06-15).

use super::*;

/// Feature E: record the start of a pointer press on a cube control so a
/// press-then-drag-away can be CANCELLED (no activation). Stores the pressed entity
/// + the press origin; `kaleidoscope_pointer_move` marks it cancelled once the
/// pointer drags past the tap threshold, and `kaleidoscope_pointer_release` honours
/// that. Mouse OR touch (same `Pointer<Press>` path).
pub(crate) fn kaleidoscope_pointer_press(
    press: On<Pointer<Press>>,
    backend: Res<InventoryUiBackend>,
    ui_state: Option<Res<ambition_platformer2d::inventory_ui::InventoryUiState>>,
    controls: Query<&AmbitionMenuControl<MenuPageAction>>,
    mut state: ResMut<KaleidoscopePointerPress>,
) {
    let open = ui_state.map(|s| s.visible).unwrap_or(false);
    if backend.effective() != InventoryUiBackend::LunexKaleidoscope || !open {
        return;
    }
    // Only arm the tap-guard for a control that DOES something: the arm's
    // identity IS the action, so a press on decoration cannot arm and needs no
    // separate "armed but actionless" state to fall through on release.
    if let Ok(control) = controls.get(press.entity) {
        if let Some(action) = control.action {
            // The ACTION, not the entity, so RELEASE dispatches
            // entity-independently and survives a face rebuild in between.
            state.0.press(action, Some(press.pointer_location.position));
        }
    }
}

/// Pointer motion (mouse/touch) over a cube control: move the focus cursor to it.
/// We listen to `Pointer<Move>` instead of `Pointer<Over>` so a menu that opens
/// under a parked mouse does not immediately select whatever is already under the
/// cursor. A real move is required before hover can take ownership.
///
/// Two guards (both essential), mirroring the grid's `MenuFocusState`:
///
/// 1. **Semantic dedup.** A moving pointer can emit several events while it stays
///    over the same control. We compare the hovered focus against `last_pointer_focus`
///    and bail when unchanged, so the cursor only reacts once per logical focus.
/// 2. **Pointer-vs-keyboard ownership.** The pointer only re-claims the cursor when
///    it moves onto a genuinely different control. This fixes "can't move away from
///    the hovered option."
pub(crate) fn kaleidoscope_pointer_move(
    move_: On<Pointer<Move>>,
    controls: Query<&AmbitionMenuControl<MenuPageAction>>,
    pages: Res<ActiveMenuPages<MenuPage, MenuPageAction>>,
    system_nav: Res<KaleidoscopeSystemNav>,
    settings: Res<UserSettings>,
    quality_confirm: Res<VisualQualityConfirmState>,
    devices: Res<ambition_platformer2d::input::SeatActiveDevices>,
    snapshot: SystemMenuSnapshotParams,
    mut cursor: ResMut<KaleidoscopeCursor>,
    // Feature E: a press in flight is cancelled (no click) once the pointer drags
    // past the tap threshold from its press origin.
    mut press: ResMut<KaleidoscopePointerPress>,
    mut sfx: SfxWriter,
) {
    // Feature E: if a press is active and the pointer has now travelled past the tap
    // threshold, this is a DRAG — the arm marks itself cancelled so the eventual
    // release does not activate the control. (This drag-cancel runs regardless of the
    // active-input gate below: a touch/pen drag must still cancel a tap.)
    press.0.moved(Some(move_.pointer_location.position));
    // Hover-select is gated on a GENUINE mouse being the active source. A cube
    // republish respawns controls under a stationary mouse and fires `Pointer<Move>`
    // for the new control; without this gate the cursor snaps back to the mouse on
    // every keyboard/gamepad/touch directional move. A real mouse move sets
    // active=Mouse (see `update_active_input_kind`) so hovering still works; clicks
    // are unaffected (separate press/release observers).
    if devices.machine() != ambition_platformer2d::input::ActiveDevice::Mouse {
        return;
    }
    let Some(active_page) = pages.active else {
        return;
    };
    if let Ok(control) = controls.get(move_.entity) {
        if let Some(action) = control.action {
            let model = SystemMenuModel::build(
                &settings,
                &snapshot.radio_snapshot(),
                &snapshot.dev_snapshot(),
            );
            let next = focus_for_action(
                action,
                active_page,
                &model,
                system_nav.open_entry,
                quality_confirm.pending(),
            );
            // The pointer hasn't moved to a new control (same logical focus as the
            // previous move event): do nothing.
            if cursor.last_pointer_focus == Some(next) {
                return;
            }
            cursor.last_pointer_focus = Some(next);
            if cursor.focus != next {
                cursor.focus = next;
                cursor.owner = FocusSource::Pointer;
                // The move landed on a genuinely different control: play the move
                // sound, matching the keyboard nav path.
                play_ui(&mut sfx, ambition_platformer2d::sfx::ids::UI_MENU_MOVE);
            }
        }
    }
}

/// Pointer release (mouse/touch) dispatches the action armed at press time.
///
/// Cube controls can be despawned/rebuilt between press and release, so dispatch is
/// entity-independent: store the action on `Pointer<Press>` and consume it on
/// release. Drag cancellation still wins when movement exceeds the tap threshold.
#[allow(clippy::too_many_arguments)]
pub(crate) fn kaleidoscope_pointer_release(
    _release: On<Pointer<Release>>,
    ui_state: Option<Res<ambition_platformer2d::inventory_ui::InventoryUiState>>,
    pages: Res<ActiveMenuPages<MenuPage, MenuPageAction>>,
    mut cursor: ResMut<KaleidoscopeCursor>,
    system_nav: Res<KaleidoscopeSystemNav>,
    settings: Res<UserSettings>,
    quality_confirm: Res<VisualQualityConfirmState>,
    // ⭐ **the release ANNOUNCES the action now.** It used to dispatch inline,
    // which is why an observer needed the whole effect stack — `owned`,
    // `commands`, `players`, `mana_q`, `heals` — plus `GameModeIo` bundled to
    // stay under Bevy's 16-parameter ceiling, to unpause on a close-via-action.
    // All of that lives in `kaleidoscope_menu_action_activated`, once, shared
    // with the keyboard route.
    mut activated: MessageWriter<MenuActionActivated<MenuPageAction>>,
    // Still needed for the focus resolve below (`system.model`), not for dispatch.
    system: SystemMenuParams,
    // In-flight press; activation uses the action stored at press time.
    mut press: ResMut<KaleidoscopePointerPress>,
) {
    let open = ui_state.as_deref().map(|s| s.visible).unwrap_or(false);
    // Read the backend from `system` (it owns the resource); a separate `Res` here
    // would be a B0002 conflict with that `ResMut`.
    if system.backend() != InventoryUiBackend::LunexKaleidoscope || !open {
        return;
    }
    // Consume the press guard (whatever happens, the next press starts fresh). A
    // release with no armed press, or a drag-away cancel, falls through to "no
    // activation".
    //
    // ⚠ `release_anywhere`, not `release` — the cube respawns its cells under the
    // finger, so which control is under the pointer NOW is not evidence about which
    // one the press began on. The press already captured that. A flat list has that
    // evidence and uses the stricter form.
    let Some(action) = press.0.release_anywhere() else {
        return;
    };
    if let Some(active_page) = pages.active {
        let model = system.model(&settings);
        let next = focus_for_action(
            action,
            active_page,
            &model,
            system_nav.open_entry,
            quality_confirm.pending(),
        );
        cursor.focus = next;
        cursor.owner = FocusSource::Pointer;
        cursor.last_pointer_focus = Some(next);
    }
    // A release and a controller submit are the SAME event, so they say it the
    // same way. The dispatch — and the close-via-action unpause that used to be
    // copied here — belong to `kaleidoscope_menu_action_activated`.
    activated.write(MenuActionActivated { action });
}

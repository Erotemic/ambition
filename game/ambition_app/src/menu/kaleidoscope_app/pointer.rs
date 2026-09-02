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
/// 1. Semantic dedup. A moving pointer can emit several events while it stays
///    over the same control. We compare the hovered focus against `last_pointer_focus`
///    and bail when unchanged, so the cursor only reacts once per logical focus.
/// 2. Pointer-vs-keyboard ownership. The pointer only re-claims the cursor when
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
    // The rows `cache_system_menu` already built this frame. See the hover path
    // below for why a hover may use them and a press may not.
    cache: Res<CachedSystemMenu>,
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
            // A hover fires on EVERY mouse move across a control, so the settings IR
            // is built only for the actions that actually read it (a System row's
            // index). Hovering the Items/Map/Quest faces now costs nothing:
            // `SystemMenuModel::build` walks the whole settings IR and allocates a
            // `String` per label, description and value, and both snapshots clone
            // their lists.
            let next = match focus_without_system_model(action, active_page) {
                Some(focus) => focus,
                // ⭐⭐ THE ROWS THIS FRAME ALREADY BUILT. A hover fires on every
                // mouse move across a control and this used to rebuild the WHOLE
                // settings IR — a `String` per label, description and value, plus
                // both snapshots — to resolve one row index. D-CUBE-CHURN closed
                // around it and recorded it as larger than either allocation the
                // row had named.
                //
                // ⛔ AND IT IS NOT A BARE CACHE SWAP, which is why the row left it.
                // `cache.rows` is populated ONLY while the System face is active,
                // and a System action stays reachable off it
                // (`focus_without_system_model` answers only for
                // Equip/Use/ChangePage) — so substituting an empty `cache.rows`
                // would resolve every such hover to `MenuFocus::System(0)`.
                // `rows_are_current_for` is that guard plus the one staleness an
                // OBSERVER can hit: this fires whenever a `Pointer<Move>` arrives,
                // which may precede this frame's `cache_system_menu`, and the
                // drill-down is the only input a press can move underneath it.
                None if cache.rows_are_current_for(system_nav.open_entry) => {
                    focus_for_action(action, active_page, &cache.rows)
                }
                None => {
                    // Off the System face, or the drill state moved since the
                    // cache ran: build the model, as this path always did.
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
    // `release_anywhere`, not `release` — the cube respawns its cells under the
    // finger, so which control is under the pointer NOW is not evidence about which
    // one the press began on. The press already captured that. A flat list has that
    // evidence and uses the stricter form.
    let Some(action) = press.0.release_anywhere() else {
        return;
    };
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
    activated.write(MenuActionActivated { action });
}

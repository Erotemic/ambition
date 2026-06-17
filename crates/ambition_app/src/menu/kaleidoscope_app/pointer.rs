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
    ui_state: Option<Res<ambition_gameplay_core::inventory::InventoryUiState>>,
    controls: Query<&AmbitionMenuControl<MenuPageAction>>,
    mut state: ResMut<KaleidoscopePointerPress>,
) {
    let open = ui_state.map(|s| s.visible).unwrap_or(false);
    if backend.effective() != InventoryUiBackend::LunexKaleidoscope || !open {
        return;
    }
    // Only arm the tap-guard for real controls (so a press on decoration is a no-op).
    if let Ok(control) = controls.get(press.entity) {
        state.entity = Some(press.entity);
        // Capture the action NOW so RELEASE can dispatch it entity-independently
        // (survives a face rebuild between press and release).
        state.action = control.action;
        state.origin = press.pointer_location.position;
        state.cancelled = false;
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
    active_input: Res<ambition_gameplay_core::input::ActiveInputKind>,
    snapshot: SystemMenuSnapshotParams,
    mut cursor: ResMut<KaleidoscopeCursor>,
    // Feature E: a press in flight is cancelled (no click) once the pointer drags
    // past the tap threshold from its press origin.
    mut press: ResMut<KaleidoscopePointerPress>,
    mut sfx: MessageWriter<SfxMessage>,
) {
    // Feature E: if a press is active and the pointer has now travelled past the tap
    // threshold, this is a DRAG — mark the press cancelled so the eventual click does
    // not activate the control. (This drag-cancel runs regardless of the active-input
    // gate below: a touch/pen drag must still cancel a tap.)
    if press.entity.is_some()
        && !press.cancelled
        && move_.pointer_location.position.distance(press.origin) > KALEIDOSCOPE_TAP_DRAG_THRESHOLD
    {
        press.cancelled = true;
    }
    // Hover-select is gated on a GENUINE mouse being the active source. A cube
    // republish respawns controls under a stationary mouse and fires `Pointer<Move>`
    // for the new control; without this gate the cursor snaps back to the mouse on
    // every keyboard/gamepad/touch directional move. A real mouse move sets
    // active=Mouse (see `update_active_input_kind`) so hovering still works; clicks
    // are unaffected (separate press/release observers).
    if *active_input != ambition_gameplay_core::input::ActiveInputKind::Mouse {
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
            let next = focus_for_action(action, active_page, &model, system_nav.open_entry);
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
                play_ui(&mut sfx, ambition_sfx::ids::UI_MENU_MOVE);
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
    mut ui_state: Option<ResMut<ambition_gameplay_core::inventory::InventoryUiState>>,
    // A close-via-action (e.g. Reset Sandbox) must restore `GameMode::Playing` exactly
    // like the canonical Esc-close — so route the close through `close_kaleidoscope_menu`.
    // Bundled into one `SystemParam` to stay under Bevy's 16-param ceiling.
    mut mode_io: GameModeIo,
    mut pages: ResMut<ActiveMenuPages<MenuPage, MenuPageAction>>,
    mut cursor: ResMut<KaleidoscopeCursor>,
    mut system_nav: ResMut<KaleidoscopeSystemNav>,
    mut owned: ResMut<OwnedItems>,
    mut settings: ResMut<UserSettings>,
    mut commands: Commands,
    mut players: MenuEffectPlayers,
    mut mana_q: MenuEffectManaQuery,
    mut heals: MessageWriter<PlayerHealRequested>,
    mut sfx: MessageWriter<SfxMessage>,
    mut system: SystemMenuParams,
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
    // release with no armed press, a drag-away cancel, or a press on a control with
    // no action all fall through to "no activation".
    let armed = press.entity.is_some();
    let cancelled = press.cancelled;
    let action = press.action;
    press.entity = None;
    press.action = None;
    press.cancelled = false;
    if !armed || cancelled {
        return;
    }
    let Some(action) = action else {
        return;
    };
    if let Some(active_page) = pages.active {
        let model = system.model(&settings);
        let next = focus_for_action(action, active_page, &model, system_nav.open_entry);
        cursor.focus = next;
        cursor.owner = FocusSource::Pointer;
        cursor.last_pointer_focus = Some(next);
    }
    let mut close_menu = false;
    // Releases route through the SAME `crate::menu::dispatch::dispatch_menu_action` as the keyboard
    // select path, so the action sounds (equip/use/rotate/toggle/...) live in
    // one place and are identical for pointer + keyboard.
    crate::menu::dispatch::dispatch_menu_action(
        action,
        &mut pages,
        &mut system_nav,
        &mut cursor,
        &mut owned,
        &mut settings,
        &mut close_menu,
        &mut commands,
        &mut players,
        &mut mana_q,
        &mut heals,
        &mut sfx,
        &mut system,
    );
    if close_menu {
        if let Some(ui_state) = ui_state.as_deref_mut() {
            // A close-via-action must unpause exactly like the canonical Esc-close.
            close_kaleidoscope_menu(ui_state, mode_io.state.get(), &mut mode_io.next);
        }
    }
}

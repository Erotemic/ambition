//! Flat tabbed menu backend (`InventoryUiBackend::Grid`).
//!
//! Wires `ambition_menu::render::bevy_ui::spawn_bevy_ui_menu` into the same menu
//! model/action/cursor resources used by the 3D kaleidoscope backend. Only the
//! presentation differs: a flat tab bar and body instead of cube faces.
//!
//! Owned systems, gated to `backend == Grid`:
//! - open/close routing for Esc/Start, inventory, and Back;
//! - tab switching and remembered active tab;
//! - keyboard/gamepad focus navigation and action dispatch;
//! - dirty republish of the active page model through the Bevy-UI renderer;
//! - pointer press/release/hover handling resilient to entity rebuilds.

use bevy::prelude::*;

use ambition_menu::render::bevy_ui::{
    BevyUiMenuRoot, BevyUiMenuTab, BevyUiMenuTabSpec, BevyUiMenuView,
};
use ambition_menu::{ActiveMenuPages, AmbitionMenuControl, MenuFocusKey, MenuNode, MenuRect};

use crate::menu::effects::{MenuEffectManaQuery, MenuEffectPlayers};
use crate::menu::kaleidoscope_app::{
    focus_for_action, owned_item_action, play_ui, system_focus_nav, KaleidoscopeCursor,
    KaleidoscopeSystemNav, SystemMenuParams,
};
use crate::menu::model::{
    build_inventory_pages_with_quality_prompt, scroll_fraction_to_window_start,
    system_max_window_start, system_rows_with_quality_prompt, MenuFocus, MenuPage, MenuPageAction,
    SYSTEM_VISIBLE_ROWS,
};
use crate::menu::quality_confirm::VisualQualityConfirmState;
use ambition_actors::items::{OwnedItems, ITEM_GRID_COLS, ITEM_GRID_ROWS};
use ambition_actors::menu::backend::{InventoryUiBackend, BEVY_UI_MENU_BACKEND_ENABLED};
use ambition_actors::persistence::settings::{SystemMenuModel, UserSettings, VisualQualityProfile};
use ambition_actors::player::PlayerHealRequested;
use ambition_input::MenuControlFrame;
use ambition_sfx::SfxMessage;

/// The effect/dispatch resources shared by [`grid_menu_nav`] and
/// [`grid_menu_pointer_release`], bundled into one [`SystemParam`] so each stays
/// under Bevy's 16-param ceiling (the same reason the cube bundles `SystemMenuParams`).
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct MenuDispatchParams<'w, 's> {
    owned: ResMut<'w, OwnedItems>,
    settings: ResMut<'w, UserSettings>,
    quality_confirm: ResMut<'w, VisualQualityConfirmState>,
    commands: Commands<'w, 's>,
    players: MenuEffectPlayers<'w, 's>,
    mana_q: MenuEffectManaQuery<'w, 's>,
    heals: MessageWriter<'w, PlayerHealRequested>,
    sfx: MessageWriter<'w, SfxMessage>,
    system: SystemMenuParams<'w>,
}

/// Run condition: the Grid backend is the active inventory frontend.
pub(crate) fn grid_backend_active(backend: Res<InventoryUiBackend>) -> bool {
    BEVY_UI_MENU_BACKEND_ENABLED && backend.effective() == InventoryUiBackend::Grid
}

/// Grid-only state: remembered tab plus republish bookkeeping. Shared cursor and
/// drill state stay on [`KaleidoscopeCursor`] / [`KaleidoscopeSystemNav`].
#[derive(Resource)]
pub(crate) struct GridMenuTabState {
    /// Index into [`MenuPage::ALL`] of the active tab. Remembered across opens.
    pub active_tab: usize,
    /// True last frame (so we detect the rising edge of an open to seed the tab).
    was_open: bool,
    /// The last-rendered view key; a change re-spawns the bevy_ui tree.
    last_key: Option<ViewKey>,
    /// Fix 4: which keyboard-focus zone the cursor is in. `Body` = focus is on the
    /// page controls (the normal case, driven by the shared `KaleidoscopeCursor`);
    /// `Tabs` = focus is on the TAB BAR (UP from the top body row), where LEFT/RIGHT
    /// cycle tabs and SELECT/DOWN drop back into the body. This is a flat-menu
    /// affordance the cube doesn't need, so it lives grid-local here.
    focus_zone: GridFocusZone,
    /// SELECTION-INDEPENDENT scroll position for the System tab's windowed list,
    /// mirroring the cube's `KaleidoscopeScroll::system_window_start`. `None` = the
    /// window follows the keyboard/pointer cursor (the historical behaviour, which
    /// made HOVERING scroll the list); `Some(start)` = an explicit scroll override
    /// set by the MOUSE WHEEL ([`grid_menu_scroll_wheel`]) or a scrollbar DRAG
    /// ([`grid_menu_apply_scroll_drag`], via the engine's neutral `MenuScrollDragged`
    /// signal). Keyboard navigation CLEARS the override so the window resumes
    /// following the cursor. With an override active, hover (cursor-follow) no longer
    /// forces the window — the override wins — so hovering rows stops scrolling.
    system_window_start: Option<usize>,
}

/// Fix 4: the grid-local keyboard-focus zone. The cube's `MenuFocus` has Item/System/
/// EdgeLeft/EdgeRight but no notion of "focus is on the tab bar", because the cube
/// switches faces with the bumpers / a rotation, not by navigating onto a tab strip.
/// The flat tabbed menu has a real tab bar above the body, so UP from the top row
/// should reach it — modeled by this zone.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum GridFocusZone {
    /// Focus is on the active page's body controls (shared cursor drives it).
    #[default]
    Body,
    /// Focus is on the tab bar; LEFT/RIGHT cycle, SELECT/DOWN activate + drop to body.
    Tabs,
}

impl Default for GridMenuTabState {
    fn default() -> Self {
        Self {
            // Default tab on open: Inventory (= `MenuPage::Items`, index 0).
            active_tab: 0,
            was_open: false,
            last_key: None,
            focus_zone: GridFocusZone::Body,
            system_window_start: None,
        }
    }
}

/// The republish-dirty key: re-render the flat tree only when one of these changes.
/// Keyed off the active tab, the drill state, the focus cursor (so the highlight
/// follows), and the shared pages `version` (so a model rebuild — inventory/settings
/// change — re-renders). Mirrors the cube's `last` republish key, flat.
#[derive(Clone, Copy, PartialEq, Eq)]
struct ViewKey {
    tab: usize,
    open_entry: Option<ambition_actors::persistence::settings::SystemMenuEntryId>,
    focus: MenuFocus,
    version: u64,
    /// Fix 4: the focus zone is part of the key so moving onto / off the tab bar
    /// re-renders (the tab focus ring appears/disappears).
    zone: GridFocusZone,
    /// The EFFECTIVE System scroll-window start (override or cursor-derived). Keying
    /// the rebuild off this — rather than the raw `focus` alone — means a wheel/drag
    /// scroll rebuilds the windowed rows while a cursor-only move inside the window
    /// still does not (preserving the click-drop fix), mirroring the cube's republish.
    window_start: usize,
    pending_quality: Option<VisualQualityProfile>,
}

/// Flat Grid backend SFX ids. The cube keeps kaleidoscope-specific `ui.menu.*`
/// sounds; Grid maps each menu event onto the generic `ui.*` ids:
///
/// | event             | cube id (was)       | old-menu id (now)   |
/// |-------------------|---------------------|---------------------|
/// | open              | `UI_MENU_OPEN`      | `UI_PAUSE_OPEN`     |
/// | close             | `UI_MENU_CLOSE`     | `UI_PAUSE_CLOSE`    |
/// | select / confirm  | `UI_MENU_ACCEPT`    | `UI_ACCEPT`         |
/// | back / cancel     | `UI_MENU_BACK`      | `UI_BACK`           |
/// | error             | `UI_MENU_ERROR`     | `UI_ERROR`          |
/// | tab / page change | `UI_TAB_CHANGE`     | `UI_TAB_CHANGE`     |
///
/// `UI_TAB_CHANGE` is a generic id the OLD menus and the cube share, so it is NOT
/// swapped (only the genuinely cube-specific `ui.menu.*` ids change).
mod grid_sfx {
    use ambition_sfx::ids;
    use ambition_sfx::SfxId;

    pub const OPEN: SfxId = ids::UI_PAUSE_OPEN;
    pub const CLOSE: SfxId = ids::UI_PAUSE_CLOSE;
    pub const ACCEPT: SfxId = ids::UI_ACCEPT;
    pub const BACK: SfxId = ids::UI_BACK;
    pub const ERROR: SfxId = ids::UI_ERROR;
    pub const TAB_CHANGE: SfxId = ids::UI_TAB_CHANGE;
}

/// The active tab's [`MenuPage`].
fn tab_page(active_tab: usize) -> MenuPage {
    MenuPage::ALL[active_tab.min(MenuPage::ALL.len() - 1)]
}

/// Which menu-open input fired, so BOTH backends route the SAME entry key to the
/// SAME landing tab/page. See [`pause_entry_target`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PauseEntrySource {
    /// Esc / Start (the pause button) → the System face.
    Pause,
    /// The dedicated inventory key → the Items (Inventory) tab.
    Inventory,
    /// The dedicated map key → the Map tab.
    Map,
}

/// THE single mapping from a menu-open input to the page/tab it should land on.
/// Used by both the Grid backend ([`grid_menu_open_routing`]) and the cube
/// (`kaleidoscope_menu_open_routing`) so the entry key sets the SAME target tab in
/// either presentation: Esc/Start → System, inventory key → Items, map key → Map.
///
/// Note: this governs only the ENTRY key's target. In-menu bumper switches still
/// "remember last tab" independently (the entry key overrides that on open).
pub(crate) fn pause_entry_target(source: PauseEntrySource) -> MenuPage {
    match source {
        PauseEntrySource::Pause => MenuPage::System,
        PauseEntrySource::Inventory => MenuPage::Items,
        PauseEntrySource::Map => MenuPage::Map,
    }
}

/// The [`MenuPage::ALL`] index of a page (for seeding `active_tab`).
fn tab_index_of(page: MenuPage) -> usize {
    MenuPage::ALL.iter().position(|p| *p == page).unwrap_or(0)
}

/// Carry the active PAGE across an inventory-backend switch (the `\` hotkey or the
/// in-menu "Menu Backend" row) so you land on the SAME screen in the new frontend
/// instead of being dumped back on Inventory. The cube keeps the page in
/// `ActiveMenuPages.active`; the Grid keeps it in `GridMenuTabState.active_tab` (it
/// renders its OWN tab, not the shared `pages.active`). The shared cursor + drill
/// state already carry the within-page position, so only the page/tab needs syncing.
/// Ordered before BOTH republish systems so the arriving backend draws the carried
/// page on the switch frame (no Inventory flash).
pub(crate) fn sync_menu_page_across_backend_switch(
    backend: Res<InventoryUiBackend>,
    overlay: Res<ambition_actors::inventory_ui::InventoryUiState>,
    mut pages: ResMut<ActiveMenuPages<MenuPage, MenuPageAction>>,
    mut tab_state: ResMut<GridMenuTabState>,
    mut last: Local<Option<InventoryUiBackend>>,
    // The page the user is on, snapshotted each stable frame so a switch can carry it
    // even after `grid_menu_nav` clobbers the live `pages.active`.
    mut carried: Local<Option<MenuPage>>,
) {
    let now = backend.effective();
    // A genuine switch WHILE THE MENU IS OPEN carries the page across; a switch while
    // closed is irrelevant (the next open's entry key sets the landing page). The very
    // first run has no prior backend to carry from.
    if *last != Some(now) && last.is_some() && overlay.visible {
        // Carry the page captured LAST frame from the OLD backend. We do NOT read the
        // live `pages.active` here: `grid_menu_nav` rewrites it to its own (stale) tab
        // the instant the backend flips to Grid, clobbering the cube's page before we
        // run. The snapshot below is taken on stable frames, so it is reliable.
        if let Some(page) = *carried {
            match now {
                InventoryUiBackend::Grid => tab_state.active_tab = tab_index_of(page),
                InventoryUiBackend::LunexKaleidoscope => pages.active = Some(page),
            }
        }
    }
    *last = Some(now);
    // Snapshot the page the user is currently on, from the ACTIVE backend's own source
    // of truth, for the NEXT switch.
    if overlay.visible {
        *carried = match now {
            InventoryUiBackend::LunexKaleidoscope => pages.active,
            InventoryUiBackend::Grid => Some(tab_page(tab_state.active_tab)),
        };
    }
}

/// The tab specs (page id + label) drawn left→right, matching [`MenuPage::ALL`].
fn tab_specs() -> Vec<BevyUiMenuTabSpec<MenuPage>> {
    MenuPage::ALL
        .into_iter()
        .map(|p| BevyUiMenuTabSpec::new(p, page_tab_label(p)))
        .collect()
}

/// The human label drawn on a tab button.
fn page_tab_label(page: MenuPage) -> &'static str {
    match page {
        MenuPage::Items => "Inventory",
        MenuPage::System => "System",
        MenuPage::Map => "Map",
        MenuPage::Quest => "Quest",
    }
}

/// Derive a control's stable [`MenuFocusKey`] from its rect, the SAME formula the
/// engine renderer (`render::bevy_ui::focus_key_for`) and the cube use. Keeping this
/// identical means the cursor key we pass as `view.focused` addresses exactly the
/// tagged control the renderer drew — the cross-backend nav contract.
fn focus_key_for(rect: MenuRect) -> MenuFocusKey {
    MenuFocusKey {
        row: (rect.y * 10.0).round() as i32,
        col: (rect.x * 10.0).round() as i32,
        order: (rect.y * 100.0 + rect.x).round() as i32,
    }
}

/// The [`MenuFocusKey`] of the control the shared cursor sits on, for the active
/// page. We match the cube cursor ([`MenuFocus`]) to a rendered control by walking
/// the page's actionable nodes and asking [`focus_for_action`] which `MenuFocus`
/// each control's action maps to — the control whose action maps to the live cursor
/// is the focused one; its rect gives the key. This reuses the cube's own
/// action→focus mapping so render + nav agree by construction.
fn cursor_focus_key(
    page: &ambition_menu::MenuPageModel<MenuPage, MenuPageAction>,
    active_page: MenuPage,
    cursor: MenuFocus,
    model: &SystemMenuModel,
    open_entry: Option<ambition_actors::persistence::settings::SystemMenuEntryId>,
    pending_quality: Option<VisualQualityProfile>,
) -> Option<MenuFocusKey> {
    for node in &page.nodes {
        let MenuNode::Control {
            rect,
            action: Some(action),
            ..
        } = node
        else {
            continue;
        };
        if focus_for_action(*action, active_page, model, open_entry, pending_quality) == cursor {
            return Some(focus_key_for(*rect));
        }
    }
    None
}

/// Open/close routing for the Grid backend — the flat analog of
/// `kaleidoscope_menu_open_routing`. Owns the Esc/Start toggle + the inventory/map
/// keys; consumes the co-firing `menu.back` on an Esc so [`grid_menu_nav`] can't
/// double-act on the same press. Opening pauses + raises `InventoryUiState.visible`;
/// Back inside a System drill pops one level, else closes (restoring `GameMode`).
#[cfg(feature = "input")]
#[allow(clippy::too_many_arguments)]
pub(crate) fn grid_menu_open_routing(
    mut menu: ResMut<MenuControlFrame>,
    mut overlay: ResMut<ambition_actors::inventory_ui::InventoryUiState>,
    mode: Res<State<ambition_actors::session::game_mode::GameMode>>,
    mut next_mode: ResMut<NextState<ambition_actors::session::game_mode::GameMode>>,
    mut tab_state: ResMut<GridMenuTabState>,
    mut cursor: ResMut<KaleidoscopeCursor>,
    mut system_nav: ResMut<KaleidoscopeSystemNav>,
    mut sfx: MessageWriter<SfxMessage>,
    mut last_start: Local<bool>,
) {
    use ambition_actors::session::game_mode::GameMode;

    // Esc / Start: rising-edge toggle (debounced like the cube to avoid the
    // close-then-reopen on a multi-frame `just_pressed`).
    let start_edge = menu.start && !*last_start;
    *last_start = menu.start;
    if start_edge {
        // Esc co-fires `menu.back`; this system OWNS the Esc toggle, so consume the
        // duplicate `back` to keep `grid_menu_nav` from acting on the same Esc.
        menu.back = false;
        if overlay.visible {
            if system_nav.open_entry.is_some() {
                play_ui(&mut sfx, grid_sfx::BACK);
                system_nav.open_entry = None;
                // Drilling out changes the row set; drop the scroll override so the
                // window snaps to the (re-seeded) cursor rather than a stale offset.
                tab_state.system_window_start = None;
                cursor.mark_keyboard(MenuFocus::System(0));
            } else {
                play_ui(&mut sfx, grid_sfx::CLOSE);
                close_grid_unified_menu(&mut overlay, mode.get(), &mut next_mode);
            }
        } else if matches!(mode.get(), GameMode::Playing | GameMode::Paused) {
            // Esc/Start opens on the System face (the shared entry→tab mapping),
            // NOT the remembered tab — the pause button targets System.
            play_ui(&mut sfx, grid_sfx::OPEN);
            tab_state.active_tab = tab_index_of(pause_entry_target(PauseEntrySource::Pause));
            tab_state.focus_zone = GridFocusZone::Body;
            open_grid_unified_menu(
                tab_state.active_tab,
                &mut overlay,
                mode.get(),
                &mut next_mode,
                &mut cursor,
                &mut system_nav,
            );
        }
        return;
    }

    // Inventory key: open ON the Inventory tab (the shared entry→tab mapping), or close.
    if menu.inventory {
        if overlay.visible {
            play_ui(&mut sfx, grid_sfx::CLOSE);
            close_grid_unified_menu(&mut overlay, mode.get(), &mut next_mode);
        } else if matches!(mode.get(), GameMode::Playing | GameMode::Paused) {
            play_ui(&mut sfx, grid_sfx::OPEN);
            tab_state.active_tab = tab_index_of(pause_entry_target(PauseEntrySource::Inventory));
            tab_state.focus_zone = GridFocusZone::Body;
            open_grid_unified_menu(
                tab_state.active_tab,
                &mut overlay,
                mode.get(),
                &mut next_mode,
                &mut cursor,
                &mut system_nav,
            );
        }
        return;
    }

    // Map key: open on the Map tab (the shared entry→tab mapping).
    if menu.map && matches!(mode.get(), GameMode::Playing | GameMode::Paused) && !overlay.visible {
        play_ui(&mut sfx, grid_sfx::OPEN);
        tab_state.active_tab = tab_index_of(pause_entry_target(PauseEntrySource::Map));
        tab_state.focus_zone = GridFocusZone::Body;
        open_grid_unified_menu(
            tab_state.active_tab,
            &mut overlay,
            mode.get(),
            &mut next_mode,
            &mut cursor,
            &mut system_nav,
        );
    }
}

/// Open the unified menu on `active_tab`, pausing the sim + seeding the cursor.
/// Mirrors `open_kaleidoscope_menu`: raise `visible`, record `opened_from_pause`,
/// switch to `Paused` when coming from gameplay.
#[cfg(feature = "input")]
fn open_grid_unified_menu(
    active_tab: usize,
    overlay: &mut ambition_actors::inventory_ui::InventoryUiState,
    mode: &ambition_actors::session::game_mode::GameMode,
    next_mode: &mut NextState<ambition_actors::session::game_mode::GameMode>,
    cursor: &mut KaleidoscopeCursor,
    system_nav: &mut KaleidoscopeSystemNav,
) {
    use ambition_actors::session::game_mode::GameMode;
    overlay.visible = true;
    overlay.opened_from_pause = matches!(mode, GameMode::Paused);
    system_nav.open_entry = None;
    seed_cursor_for_tab(active_tab, cursor);
    if matches!(mode, GameMode::Playing) {
        next_mode.set(GameMode::Paused);
    }
}

/// Seed a sensible cursor for the tab being shown.
fn seed_cursor_for_tab(active_tab: usize, cursor: &mut KaleidoscopeCursor) {
    cursor.mark_keyboard(match tab_page(active_tab) {
        MenuPage::Items => MenuFocus::Item(0),
        MenuPage::System => MenuFocus::System(0),
        MenuPage::Map | MenuPage::Quest => MenuFocus::EdgeLeft,
    });
}

/// Close the unified menu, restoring `GameMode::Playing` when it was opened directly
/// from gameplay (respecting `opened_from_pause`). Same contract as
/// `close_kaleidoscope_menu`.
pub(crate) fn close_grid_unified_menu(
    overlay: &mut ambition_actors::inventory_ui::InventoryUiState,
    mode: &ambition_actors::session::game_mode::GameMode,
    next_mode: &mut NextState<ambition_actors::session::game_mode::GameMode>,
) {
    use ambition_actors::session::game_mode::GameMode;
    let opened_from_pause = overlay.opened_from_pause;
    overlay.visible = false;
    if !opened_from_pause && matches!(mode, GameMode::Paused) {
        next_mode.set(GameMode::Playing);
    }
}

/// Keyboard / gamepad navigation for the Grid backend. Bumpers switch tabs
/// (wraparound); up/down/left/right move the focus cursor over the active page;
/// `select` dispatches the focused control's action; `back` pops a System drill
/// else closes. The Esc toggle is owned by [`grid_menu_open_routing`] (it bails on
/// `menu.start`), so this never fights it.
#[cfg(feature = "input")]
#[allow(clippy::too_many_arguments)]
pub(crate) fn grid_menu_nav(
    mut menu_frame: ResMut<MenuControlFrame>,
    mut tab_state: ResMut<GridMenuTabState>,
    mut cursor: ResMut<KaleidoscopeCursor>,
    mut system_nav: ResMut<KaleidoscopeSystemNav>,
    mut pages: ResMut<ActiveMenuPages<MenuPage, MenuPageAction>>,
    mut overlay: ResMut<ambition_actors::inventory_ui::InventoryUiState>,
    mode: Res<State<ambition_actors::session::game_mode::GameMode>>,
    mut next_mode: ResMut<NextState<ambition_actors::session::game_mode::GameMode>>,
    mut fx: MenuDispatchParams,
) {
    // Read the backend from `fx.system` (it owns the resource); a separate `Res`
    // here would be a B0002 conflict with that `ResMut`.
    if fx.system.backend() != InventoryUiBackend::Grid || !overlay.visible {
        // Not the active backend — leave the frame for whichever nav owns it.
        return;
    }
    // This frame's menu navigation belongs to the Grid now: snapshot it, then CONSUME
    // the one-shot nav edges so the Cube backend's nav (sharing this `Res` in the same
    // frame) can't re-fire the same press if the "Menu Backend" row flips
    // `InventoryUiBackend` mid-frame.
    let menu = *menu_frame;
    menu_frame.consume_nav_edges();
    // The Esc/Start toggle is owned by `grid_menu_open_routing`; bail so a single Esc
    // can't both close here and reopen there (the cube's Esc-Esc reopen guard).
    if menu.start {
        return;
    }

    // Bumpers cycle tabs with wraparound (the shared MenuControlFrame contract).
    // Bumpers act regardless of focus zone and leave focus in the body (they're the
    // "fast" tab switch); arrow-key tab nav is the discoverable alternative (Fix 4).
    let bump = (menu.page_right as i32) - (menu.page_left as i32);
    if bump != 0 {
        let n = MenuPage::ALL.len() as i32;
        tab_state.active_tab = ((tab_state.active_tab as i32 + bump).rem_euclid(n)) as usize;
        system_nav.open_entry = None;
        tab_state.system_window_start = None;
        seed_cursor_for_tab(tab_state.active_tab, &mut cursor);
        tab_state.focus_zone = GridFocusZone::Body;
        pages.active = Some(tab_page(tab_state.active_tab));
        play_ui(&mut fx.sfx, grid_sfx::TAB_CHANGE);
        return;
    }

    let active_page = tab_page(tab_state.active_tab);
    // Keep the shared pages pointer aligned with the active tab so the republished
    // model (built by `republish_kaleidoscope_pages`) is the tab we render.
    pages.active = Some(active_page);

    let dx = (menu.right as i32) - (menu.left as i32);
    let dy = (menu.down as i32) - (menu.up as i32);

    // Fix 4: when focus is on the TAB BAR, the arrow keys drive the tabs: LEFT/RIGHT
    // cycle (live, so the body under them updates), DOWN or SELECT activate the focused
    // tab and drop focus back into the body, and Back drops to the body without
    // switching. The bumpers above still work independently.
    if tab_state.focus_zone == GridFocusZone::Tabs {
        if dx != 0 {
            let n = MenuPage::ALL.len() as i32;
            tab_state.active_tab = ((tab_state.active_tab as i32 + dx).rem_euclid(n)) as usize;
            fx.quality_confirm.cancel();
            system_nav.open_entry = None;
            tab_state.system_window_start = None;
            seed_cursor_for_tab(tab_state.active_tab, &mut cursor);
            pages.active = Some(tab_page(tab_state.active_tab));
            play_ui(&mut fx.sfx, grid_sfx::TAB_CHANGE);
            return;
        }
        if menu.select || dy > 0 {
            // Activate: focus is already the live tab; just drop into the body (the
            // cursor was seeded for this tab when we switched onto it).
            tab_state.focus_zone = GridFocusZone::Body;
            seed_cursor_for_tab(tab_state.active_tab, &mut cursor);
            play_ui(&mut fx.sfx, grid_sfx::ACCEPT);
            return;
        }
        if menu.back {
            tab_state.focus_zone = GridFocusZone::Body;
            play_ui(&mut fx.sfx, grid_sfx::BACK);
            return;
        }
        // No other input does anything while the tab bar holds focus.
        return;
    }

    // Fix 4: UP from the TOP body row moves focus onto the tab bar.
    if dy < 0 && cursor_on_top_row(active_page, cursor.focus(), system_nav.open_entry) {
        tab_state.focus_zone = GridFocusZone::Tabs;
        play_ui(&mut fx.sfx, grid_sfx::TAB_CHANGE);
        return;
    }

    match active_page {
        MenuPage::Items => {
            if dx != 0 || dy != 0 {
                let next = move_items_cursor(cursor.focus(), dx, dy);
                cursor.mark_keyboard(next);
            }
            if menu.back {
                play_ui(&mut fx.sfx, grid_sfx::CLOSE);
                fx.quality_confirm.cancel();
                close_grid_unified_menu(&mut overlay, mode.get(), &mut next_mode);
                return;
            }
            if menu.select {
                let idx = cursor.focus().item_index();
                if let Some(action) = owned_item_action(&fx.owned, idx) {
                    let mut close_menu = false;
                    crate::menu::dispatch::dispatch_menu_action(
                        action,
                        &mut pages,
                        &mut system_nav,
                        &mut cursor,
                        &mut fx.owned,
                        &mut fx.settings,
                        &mut fx.quality_confirm,
                        &mut close_menu,
                        &mut fx.commands,
                        &mut fx.players,
                        &mut fx.mana_q,
                        &mut fx.heals,
                        &mut fx.sfx,
                        &mut fx.system,
                    );
                    // `dispatch_menu_action`'s `ChangePage` could move us off Items;
                    // re-pin to the active tab so the cube's page-turn semantics don't
                    // leak into the flat tabs.
                    pages.active = Some(active_page);
                    // Fix 3: force the next republish so the new state (e.g. the equip
                    // checkmark) shows immediately, without waiting for a cursor move.
                    tab_state.last_key = None;
                    if close_menu {
                        fx.quality_confirm.cancel();
                        close_grid_unified_menu(&mut overlay, mode.get(), &mut next_mode);
                    }
                } else {
                    play_ui(&mut fx.sfx, grid_sfx::ERROR);
                }
            }
        }
        MenuPage::System => {
            // Features C/D: a keyboard move/select takes the selection cursor back
            // over from the wheel/scrollbar — drop any explicit scroll override so the
            // window snaps to follow the cursor again (the cube's clear-on-keyboard rule).
            if dx != 0 || dy != 0 || menu.select {
                tab_state.system_window_start = None;
            }
            // Reuse the cube's System row nav (drill in/out, value-step, select →
            // dispatch, back → drill-out/close): identical behavior + one dispatcher.
            system_focus_nav(
                &menu,
                dx,
                dy,
                &mut cursor,
                &mut system_nav,
                &mut pages,
                &mut overlay,
                mode.get(),
                &mut next_mode,
                &mut fx.settings,
                &mut fx.quality_confirm,
                active_page,
                &mut fx.owned,
                &mut fx.commands,
                &mut fx.players,
                &mut fx.mana_q,
                &mut fx.heals,
                &mut fx.sfx,
                &mut fx.system,
                // Grid switches pages via the TAB BAR — never the cube's edge
                // page-turn. Passing `false` keeps System-row LEFT/RIGHT from walking
                // onto an edge and firing `turn_page` (the cube rotate-SFX + a
                // one-frame face flip that leaked into Grid mode).
                false,
                // No vertical wrap: the Grid's rows sit below the tab bar, a real
                // target UP off the top row must reach — so clamp, don't wrap.
                false,
            );
            // Belt-and-braces: keep the shared page pointer on this tab (republish also
            // aligns it). With `allow_page_turn=false` the nav can no longer move it.
            if overlay.visible {
                pages.active = Some(active_page);
            }
            // Fix 3: a System select can toggle/cycle a setting (mutating state but not
            // the cursor); force the next republish so the new value/cursor shows now.
            if menu.select {
                tab_state.last_key = None;
            }
        }
        MenuPage::Map | MenuPage::Quest => {
            // Placeholder tabs: only Back does anything.
            if menu.back {
                play_ui(&mut fx.sfx, grid_sfx::CLOSE);
                fx.quality_confirm.cancel();
                close_grid_unified_menu(&mut overlay, mode.get(), &mut next_mode);
            }
        }
    }
}

/// Fix 4: is the cursor on the TOP row of the active page's body? UP from here moves
/// focus onto the tab bar. For the Items grid that's the first row of cells; for the
/// System list it's the first row (index 0, and only at the top level — inside a drill
/// UP should navigate the drilled rows, not escape to the tabs); for the placeholder
/// Map/Quest tabs (which seed `EdgeLeft`) any UP reaches the tabs.
fn cursor_on_top_row(
    page: MenuPage,
    focus: MenuFocus,
    open_entry: Option<ambition_actors::persistence::settings::SystemMenuEntryId>,
) -> bool {
    match page {
        MenuPage::Items => match focus {
            MenuFocus::Item(i) => i < ITEM_GRID_COLS,
            _ => true,
        },
        MenuPage::System => open_entry.is_none() && matches!(focus, MenuFocus::System(0)),
        MenuPage::Map | MenuPage::Quest => true,
    }
}

/// Move the 6×4 Items grid cursor by one step, clamped (no wraparound, no edges —
/// the flat grid has no page-turn arrows). Non-Item focuses re-enter at slot 0.
fn move_items_cursor(focus: MenuFocus, dx: i32, dy: i32) -> MenuFocus {
    let cols = ITEM_GRID_COLS as i32;
    let rows = ITEM_GRID_ROWS as i32;
    let idx = match focus {
        MenuFocus::Item(i) => i as i32,
        _ => 0,
    };
    let row = (idx / cols + dy).clamp(0, rows - 1);
    let col = (idx % cols + dx).clamp(0, cols - 1);
    MenuFocus::Item((row * cols + col) as usize)
}

/// Re-render the flat bevy_ui tree when the view changes. The active tab's
/// already-built [`MenuPageModel`] comes from the shared `ActiveMenuPages` (filled by
/// the cube's `republish_kaleidoscope_pages`, which now runs for BOTH backends), so
/// the Grid and cube draw the SAME model. Despawn + respawn only on a dirty key.
#[allow(clippy::too_many_arguments)]
pub(crate) fn grid_menu_republish_view(
    overlay: Res<ambition_actors::inventory_ui::InventoryUiState>,
    pages: Res<ActiveMenuPages<MenuPage, MenuPageAction>>,
    owned: Res<OwnedItems>,
    cursor: Res<KaleidoscopeCursor>,
    system_nav: Res<KaleidoscopeSystemNav>,
    settings: Res<UserSettings>,
    quality_confirm: Res<VisualQualityConfirmState>,
    system: SystemMenuParams,
    mut tab_state: ResMut<GridMenuTabState>,
    roots: Query<Entity, With<BevyUiMenuRoot>>,
    assets: Option<Res<AssetServer>>,
    mut commands: Commands,
) {
    // Read the backend from `system` (it owns the resource); a separate `Res` here
    // would be a B0002 conflict with that `ResMut`.
    let active = system.backend() == InventoryUiBackend::Grid && overlay.visible;
    if !active {
        // Tear the tree down when not the active+open backend, and forget the key so
        // a reopen always rebuilds.
        if tab_state.was_open || !roots.is_empty() {
            for e in &roots {
                commands.entity(e).despawn();
            }
            tab_state.was_open = false;
            tab_state.last_key = None;
        }
        return;
    }
    tab_state.was_open = true;

    // RENDER THE GRID'S TAB, not the shared `pages.active` (which the cube drives).
    // The grid builds the active tab's `MenuPageModel` from the SAME backend-agnostic
    // builder the cube uses (`build_inventory_pages`), then renders THE PAGE WHOSE
    // `id == grid tab` — so switching tabs actually switches the body. Building here
    // (instead of fishing the cube's `pages.pages` out by id) makes the grid
    // self-sufficient: it does not depend on the cube's republish ordering/gating,
    // which was why the body could lag a tab behind / always read Items.
    let active_page = tab_page(tab_state.active_tab);
    let model = system.model(&settings);
    // The EFFECTIVE System window start: an explicit wheel/drag override wins
    // (Features C/D), otherwise it follows the cursor — exactly the cube's rule via
    // the shared `system_effective_window_start`. Hovering moves the cursor but, with
    // an override set, does NOT shift the window, so hovering no longer scrolls.
    let window_start = if active_page == MenuPage::System {
        let rows = system_rows_with_quality_prompt(
            &model,
            system_nav.open_entry,
            quality_confirm.pending(),
        );
        crate::menu::model::system_effective_window_start(
            &rows,
            cursor.focus(),
            tab_state.system_window_start,
        )
    } else {
        0
    };
    let key = ViewKey {
        tab: tab_state.active_tab,
        open_entry: system_nav.open_entry,
        focus: cursor.focus(),
        version: pages.version,
        zone: tab_state.focus_zone,
        window_start,
        pending_quality: quality_confirm.pending(),
    };
    // Fix 3: detect inventory/settings STATE changes too, mirroring the cube's
    // `republish_kaleidoscope_pages` (`owned.is_changed() || settings.is_changed()`).
    // A select that mutates state (equip an item, toggle a setting, play a radio song)
    // changes `OwnedItems`/`UserSettings` but NOT the focus cursor, so the old key
    // `(tab, open_entry, focus, version)` stayed equal and the view did not refresh
    // until the cursor moved. The dispatch paths ALSO clear `last_key` directly (the
    // belt-and-braces force-republish), so even a state change this key can't see
    // (e.g. a dev snapshot) still re-renders.
    let state_changed = owned.is_changed() || settings.is_changed() || quality_confirm.is_changed();
    if tab_state.last_key == Some(key) && !roots.is_empty() && !state_changed {
        return;
    }
    tab_state.last_key = Some(key);

    let built = build_inventory_pages_with_quality_prompt(
        &owned,
        owned.equipped(),
        cursor.focus(),
        &settings,
        &system.radio_snapshot(),
        &system.dev_snapshot(),
        window_start,
        system_nav.open_entry,
        quality_confirm.pending(),
    );
    let Some(page) = built
        .iter()
        .find(|p| p.id == active_page)
        .or_else(|| built.first())
    else {
        return;
    };

    // The cursor's focus key, so the renderer highlights the right control.
    let focused = cursor_focus_key(
        page,
        active_page,
        cursor.focus(),
        &model,
        system_nav.open_entry,
        quality_confirm.pending(),
    );

    // BUG 2: strip the cube's page-turn EDGE controls (`MenuPageAction::ChangePage`).
    // The page builders bake `< Prev` / `> Next` edge buttons for the cube; the flat
    // tabbed renderer replaces them with the tab bar, so they must NOT be drawn (they
    // leaked in as "< Items" / "> Quest" flashes, and keyboard/gamepad nav could land
    // on them). The flat backend filters them out before handing the model to the
    // engine renderer.
    let mut page = page.clone();
    page.nodes.retain(|n| {
        !matches!(
            n,
            MenuNode::Control {
                action: Some(MenuPageAction::ChangePage(_)),
                ..
            }
        )
    });

    // Despawn the previous tree, then respawn ON THE SAME command buffer so both
    // land in the same flush. (The old code despawned here but deferred the spawn
    // into a `commands.queue(world.commands())` closure that flushed LATER, leaving
    // a one-frame gap with no body content — the menu body looked empty and only
    // flashed its content for the frame a respawn happened to land. Spawning
    // directly with the system's own `Commands` closes that gap.)
    for e in &roots {
        commands.entity(e).despawn();
    }
    let tabs = tab_specs();
    // Fix 4: when focus is on the tab bar, tell the renderer which tab to ring; when
    // in the body, no tab is focused (only the active tab is highlighted).
    let focused_tab = match tab_state.focus_zone {
        GridFocusZone::Tabs => Some(tab_state.active_tab),
        GridFocusZone::Body => None,
    };
    let view = BevyUiMenuView {
        tabs: &tabs,
        active_tab: tab_state.active_tab,
        page: &page,
        focused,
        focused_tab,
    };
    // Fix 3: hand the `AssetServer` to the renderer so the Items tab shows its
    // sprite ICONS (the model's per-cell icon path), like the cube does.
    ambition_menu::render::bevy_ui::spawn_bevy_ui_menu_with_assets(
        &mut commands,
        &view,
        assets.as_deref(),
    );
}

/// The live System row count for the Grid's current drill-down state (0 outside the
/// System tab). Shared by the wheel + drag scroll appliers to clamp the override,
/// mirroring the cube's `system_row_count`.
fn grid_system_row_count(
    active_page: MenuPage,
    system_nav: &KaleidoscopeSystemNav,
    model: &SystemMenuModel,
    pending_quality: Option<VisualQualityProfile>,
) -> usize {
    if active_page != MenuPage::System {
        return 0;
    }
    system_rows_with_quality_prompt(model, system_nav.open_entry, pending_quality).len()
}

/// Feature D (Grid): the MOUSE WHEEL scrolls the System window (the visible rows),
/// NOT the keyboard selection — the direct mirror of the cube's
/// `kaleidoscope_scroll_wheel`. Each wheel notch moves the scroll override by one
/// row, clamped to `[0, system_max_window_start]`. The cursor/selection is
/// untouched; a later keyboard move clears the override and the window snaps back to
/// the cursor. Only a scrollable System list reacts; a short list ignores the wheel.
#[cfg(feature = "input")]
pub(crate) fn grid_menu_scroll_wheel(
    overlay: Res<ambition_actors::inventory_ui::InventoryUiState>,
    mut tab_state: ResMut<GridMenuTabState>,
    system_nav: Res<KaleidoscopeSystemNav>,
    settings: Res<UserSettings>,
    quality_confirm: Res<VisualQualityConfirmState>,
    cursor: Res<KaleidoscopeCursor>,
    system: SystemMenuParams,
    mut wheel: MessageReader<bevy::input::mouse::MouseWheel>,
) {
    // Backend read from `system` (it owns the resource); a separate `Res` would
    // B0002-conflict with that `ResMut`.
    if system.backend() != InventoryUiBackend::Grid || !overlay.visible {
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
    let active_page = tab_page(tab_state.active_tab);
    let model = system.model(&settings);
    let total = grid_system_row_count(active_page, &system_nav, &model, quality_confirm.pending());
    if total <= SYSTEM_VISIBLE_ROWS {
        return; // nothing to scroll
    }
    let max = system_max_window_start(total) as i32;
    // Seed from the effective start so the first notch moves relative to what is
    // currently shown (cursor-derived window) rather than jumping to 0.
    let rows =
        system_rows_with_quality_prompt(&model, system_nav.open_entry, quality_confirm.pending());
    let current = crate::menu::model::system_effective_window_start(
        &rows,
        cursor.focus(),
        tab_state.system_window_start,
    ) as i32;
    let next = (current + steps).clamp(0, max) as usize;
    tab_state.system_window_start = Some(next);
}

/// Feature C (Grid): apply the engine's backend-agnostic scrollbar-drag signal
/// (`ambition_menu::MenuScrollDragged`, emitted by the
/// `bevy_ui` scrollbar observers) to the Grid's scroll override — the mirror of the
/// cube's `kaleidoscope_apply_scroll_drag`. The neutral `0..=1` fraction maps across
/// the scrollable range to a window-start row. Selection-independent, like the wheel.
#[cfg(feature = "input")]
pub(crate) fn grid_menu_apply_scroll_drag(
    overlay: Res<ambition_actors::inventory_ui::InventoryUiState>,
    mut tab_state: ResMut<GridMenuTabState>,
    system_nav: Res<KaleidoscopeSystemNav>,
    settings: Res<UserSettings>,
    quality_confirm: Res<VisualQualityConfirmState>,
    system: SystemMenuParams,
    mut dragged: MessageReader<ambition_menu::MenuScrollDragged>,
) {
    // Backend read from `system` (it owns the resource); a separate `Res` would
    // B0002-conflict with that `ResMut`.
    if system.backend() != InventoryUiBackend::Grid || !overlay.visible {
        dragged.clear();
        return;
    }
    // Use the LAST drag fraction this frame (the freshest pointer position).
    let Some(fraction) = dragged.read().last().map(|d| d.fraction.clamp(0.0, 1.0)) else {
        return;
    };
    let active_page = tab_page(tab_state.active_tab);
    let model = system.model(&settings);
    let total = grid_system_row_count(active_page, &system_nav, &model, quality_confirm.pending());
    let result = scroll_fraction_to_window_start(total, fraction);
    if let Some(start) = result {
        tab_state.system_window_start = Some(start);
    }
}

/// Pointer/touch: a press on a tagged control or tab captures the intent;
/// [`grid_menu_pointer_release`] dispatches on release using the CAPTURED action
/// (entity-independent), so a republish that despawns + respawns the control between
/// press and release cannot drop the click (the cube's Bug-2 fix, flat). Hover moves
/// the cursor.
#[derive(Resource, Default)]
pub(crate) struct GridPointerPress {
    /// A captured control action (Items/System select) to fire on release.
    action: Option<MenuPageAction>,
    /// A captured tab index to switch to on release.
    tab: Option<usize>,
}

/// Capture a press on a control (its action) or a tab (its index).
pub(crate) fn grid_menu_pointer_press(
    press: On<Pointer<Press>>,
    backend: Res<InventoryUiBackend>,
    overlay: Res<ambition_actors::inventory_ui::InventoryUiState>,
    controls: Query<&AmbitionMenuControl<MenuPageAction>>,
    tabs: Query<&BevyUiMenuTab>,
    mut state: ResMut<GridPointerPress>,
) {
    let e = press.entity;
    if backend.effective() != InventoryUiBackend::Grid || !overlay.visible {
        return;
    }
    // A single click emits a `Pointer<Press>` for EVERY entity under the cursor —
    // the interactive tab/control PLUS its panel/scrim/window ancestors. Only an
    // interactive hit may SET the capture; a non-interactive hit must NOT clobber
    // it (the bug: a later scrim/window press reset the captured tab to None, so the
    // release saw nothing). The capture is cleared by the release that consumes it,
    // so each click starts fresh.
    if let Ok(tab) = tabs.get(e) {
        state.tab = Some(tab.index);
        state.action = None;
    } else if let Ok(ctrl) = controls.get(e) {
        if let Some(action) = ctrl.action {
            state.action = Some(action);
            state.tab = None;
        }
    }
}

/// Dispatch the captured press on release: switch tabs, or route the control's
/// action through the SHARED [`crate::menu::dispatch::dispatch_menu_action`].
#[cfg(feature = "input")]
#[allow(clippy::too_many_arguments)]
pub(crate) fn grid_menu_pointer_release(
    _release: On<Pointer<Release>>,
    mut state: ResMut<GridPointerPress>,
    mut tab_state: ResMut<GridMenuTabState>,
    mut cursor: ResMut<KaleidoscopeCursor>,
    mut system_nav: ResMut<KaleidoscopeSystemNav>,
    mut pages: ResMut<ActiveMenuPages<MenuPage, MenuPageAction>>,
    mut overlay: ResMut<ambition_actors::inventory_ui::InventoryUiState>,
    mode: Res<State<ambition_actors::session::game_mode::GameMode>>,
    mut next_mode: ResMut<NextState<ambition_actors::session::game_mode::GameMode>>,
    mut fx: MenuDispatchParams,
) {
    // Backend read from `fx.system` (it owns the resource); a separate `Res` would
    // B0002-conflict with that `ResMut`.
    if fx.system.backend() != InventoryUiBackend::Grid || !overlay.visible {
        state.action = None;
        state.tab = None;
        return;
    }
    if let Some(tab) = state.tab.take() {
        tab_state.active_tab = tab.min(MenuPage::ALL.len() - 1);
        fx.quality_confirm.cancel();
        system_nav.open_entry = None;
        tab_state.system_window_start = None;
        seed_cursor_for_tab(tab_state.active_tab, &mut cursor);
        // Clicking a tab lands focus in that tab's body (Fix 4: pointer doesn't park
        // on the tab bar — only arrow-key nav holds the Tabs zone).
        tab_state.focus_zone = GridFocusZone::Body;
        pages.active = Some(tab_page(tab_state.active_tab));
        play_ui(&mut fx.sfx, grid_sfx::TAB_CHANGE);
        return;
    }
    let Some(action) = state.action.take() else {
        return;
    };
    let mut close_menu = false;
    crate::menu::dispatch::dispatch_menu_action(
        action,
        &mut pages,
        &mut system_nav,
        &mut cursor,
        &mut fx.owned,
        &mut fx.settings,
        &mut fx.quality_confirm,
        &mut close_menu,
        &mut fx.commands,
        &mut fx.players,
        &mut fx.mana_q,
        &mut fx.heals,
        &mut fx.sfx,
        &mut fx.system,
    );
    pages.active = Some(tab_page(tab_state.active_tab));
    // Fix 3: force the next republish so a click-dispatched state change (equip,
    // setting toggle, radio song) refreshes the view immediately.
    tab_state.last_key = None;
    if close_menu {
        fx.quality_confirm.cancel();
        close_grid_unified_menu(&mut overlay, mode.get(), &mut next_mode);
    }
}

/// Hover: move the cursor onto the hovered control (so keyboard + pointer agree).
///
/// Gated on `ActiveInputKind == Mouse`: the menu republishes (despawn +
/// respawn its controls) on every cursor move, and a fresh control spawning
/// under a STATIONARY mouse makes `bevy_ui` picking fire a `Pointer<Over>`. If
/// this handler reacted to that while the player was on the keyboard / gamepad /
/// touch, it would snap the cursor straight back to the mouse on every
/// directional move (the recurring "can't move away from the hovered option"
/// bug). A GENUINE mouse move sets `ActiveInputKind = Mouse` first (see
/// `update_active_input_kind`), so real hovering still works; only the
/// rebuild-induced `Over` is ignored. Mouse CLICKS are NOT gated (they go
/// through the press/release observers), so click-to-select keeps working.
pub(crate) fn grid_menu_pointer_hover(
    over: On<Pointer<Over>>,
    overlay: Res<ambition_actors::inventory_ui::InventoryUiState>,
    active_input: Res<ambition_input::ActiveInputKind>,
    controls: Query<&AmbitionMenuControl<MenuPageAction>>,
    settings: Res<UserSettings>,
    quality_confirm: Res<VisualQualityConfirmState>,
    system: SystemMenuParams,
    tab_state: Res<GridMenuTabState>,
    system_nav: Res<KaleidoscopeSystemNav>,
    mut cursor: ResMut<KaleidoscopeCursor>,
) {
    // Backend read from `system` (it owns the resource); a separate `Res` would
    // B0002-conflict with that `ResMut`.
    if system.backend() != InventoryUiBackend::Grid || !overlay.visible {
        return;
    }
    // Only a genuine mouse move (which set active=Mouse) may move the cursor;
    // a rebuild-induced `Over` while on keyboard/gamepad/touch is ignored.
    if *active_input != ambition_input::ActiveInputKind::Mouse {
        return;
    }
    let Ok(ctrl) = controls.get(over.entity) else {
        return;
    };
    let Some(action) = ctrl.action else {
        return;
    };
    let active_page = tab_page(tab_state.active_tab);
    let model = system.model(&settings);
    let focus = focus_for_action(
        action,
        active_page,
        &model,
        system_nav.open_entry,
        quality_confirm.pending(),
    );
    cursor.mark_keyboard(focus);
}

/// Install the flat Bevy-UI/Grid backend systems. Registered independently from
/// the cube backend so builds can omit this presentation without installing its
/// Bevy-UI tree, picking observers, or scroll systems.
pub fn install_grid_unified_menu(app: &mut App) {
    app.init_resource::<GridMenuTabState>()
        .init_resource::<GridPointerPress>()
        // The pointer-hover observer reads `ActiveInputKind`; the input plugin
        // also inits it, but init here too so the Grid backend is self-sufficient
        // (`init_resource` is idempotent).
        .init_resource::<ambition_input::ActiveInputKind>();
    #[cfg(feature = "input")]
    app.add_systems(
        Update,
        (
            grid_menu_open_routing.run_if(grid_backend_active),
            grid_menu_nav
                .run_if(grid_backend_active)
                // Join the shared menu-nav consume set so the touch-joystick
                // fold (mobile_input) can pin `.before(MenuNavConsume)` and
                // land its directional intent before this reads the frame.
                .in_set(ambition_actors::schedule::MenuNavConsume),
        )
            .chain()
            .before(ambition_actors::schedule::SandboxSet::CoreSimulation),
    );
    app.add_systems(
        Update,
        grid_menu_republish_view.after(ambition_actors::schedule::SandboxSet::CoreSimulation),
    );
    // Carry the active page across a backend switch BEFORE the Grid republishes its
    // body, so you land on the same screen you were on (not Inventory). Ordered AFTER
    // `MenuNavConsume` (both backends' nav live there) so an in-menu "Menu Backend"
    // flip is seen on the SAME frame. The cube direction (grid→cube) settles via the
    // cube's own republish next frame, hidden by the cube's fold-in animation.
    app.add_systems(
        Update,
        sync_menu_page_across_backend_switch
            .after(ambition_actors::schedule::MenuNavConsume)
            .before(grid_menu_republish_view),
    );
    // Features C/D: the wheel + scrollbar-drag scroll appliers run BEFORE republish so
    // a scroll set this frame rebuilds the windowed rows the same frame. The drag
    // signal comes from the engine's `bevy_ui` scrollbar observers
    // (`install_bevy_ui_menu_scroll`), which also registers the `MenuScrollDragged`
    // message the applier reads.
    #[cfg(feature = "input")]
    {
        ambition_menu::render::bevy_ui::install_bevy_ui_menu_scroll(app);
        app.add_systems(
            Update,
            (grid_menu_scroll_wheel, grid_menu_apply_scroll_drag)
                .run_if(grid_backend_active)
                .before(grid_menu_republish_view),
        );
    }
    #[cfg(feature = "input")]
    app.add_observer(grid_menu_pointer_press)
        .add_observer(grid_menu_pointer_release)
        .add_observer(grid_menu_pointer_hover);
}

#[cfg(all(test, feature = "input"))]
mod tests;

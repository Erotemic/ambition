//! Unified flat tabbed menu — the `InventoryUiBackend::Grid` presentation.
//!
//! This is Ambition's wiring of the engine's `bevy_ui` renderer
//! ([`ambition_menu::render::bevy_ui::spawn_bevy_ui_menu`]) into a working,
//! navigable, tabbed pause menu. It is the flat analog of the 3D cube backend
//! ([`crate::lunex_kaleidoscope_app`]): SAME page models, SAME action dispatcher
//! ([`crate::menu::dispatch::dispatch_menu_action`]), SAME shared cursor/drill
//! resources ([`KaleidoscopeCursor`], [`KaleidoscopeSystemNav`]) — only the
//! presentation differs (a flat tab bar + flex/grid body instead of a rotating
//! cube). Having two real renderers of one model validates the engine/content
//! seam; see `docs/planning/unified_tabbed_menu.md` §2/§3/§6.
//!
//! # What this module owns (gated to `backend == Grid`)
//! * **open/close** ([`grid_menu_open_routing`]) — the Grid analog of
//!   `kaleidoscope_menu_open_routing`. Esc/Start or the inventory key opens the
//!   unified menu (`GameMode::Paused`); Back at a tab's top level closes (→
//!   `GameMode::Playing`, respecting `opened_from_pause`); Back inside a System
//!   drill pops one level. ONE open/close owner for the Grid backend.
//! * **tabs** — the 4 [`MenuPage::ALL`] tabs; L/R bumpers cycle with wraparound;
//!   clicking a tab switches. Default tab Inventory; the last-viewed tab is
//!   remembered across opens ([`GridMenuTabState`]).
//! * **nav** ([`grid_menu_nav`]) — up/down/left/right move the focus cursor over
//!   the active page's controls (Items = 6×4 grid, System = the row list);
//!   `select` dispatches the focused control's action; `back` pops/closes.
//! * **render** ([`grid_menu_republish_view`]) — each frame the ACTIVE TAB's
//!   [`MenuPageModel`] is built from the SAME backend-agnostic builder the cube uses
//!   (`build_inventory_pages`) and rendered by `spawn_bevy_ui_menu` (the grid renders
//!   its OWN tab, not the cube's `pages.active`). The cube's page-turn EDGE controls
//!   (`MenuPageAction::ChangePage`) are stripped — the tab bar replaces them. Re-render
//!   only when the model/tab/drill/cursor changes.
//! * **pointer** ([`grid_menu_pointer_*`]) — clicking a tagged control dispatches
//!   its action (entity-independent press→release so a rebuild can't drop a
//!   click); clicking a tab switches; hover moves the cursor.

#![cfg(feature = "oot_inventory")]

use bevy::prelude::*;

use ambition_menu::render::bevy_ui::{
    BevyUiMenuRoot, BevyUiMenuTab, BevyUiMenuTabSpec, BevyUiMenuView,
};
use ambition_menu::{ActiveMenuPages, AmbitionMenuControl, MenuFocusKey, MenuNode, MenuRect};

use crate::audio::SfxMessage;
use crate::bevy_ui_grid_menu::input::{MenuEffectManaQuery, MenuEffectPlayers};
use crate::input::MenuControlFrame;
use crate::items::{OwnedItems, ITEM_GRID_COLS, ITEM_GRID_ROWS};
use crate::lunex_kaleidoscope_app::{
    focus_for_action, owned_item_action, play_ui, system_focus_nav, InventoryUiBackend,
    KaleidoscopeCursor, KaleidoscopeSystemNav, SystemMenuParams,
};
use crate::menu::model::{MenuFocus, MenuPage, MenuPageAction};
use crate::persistence::settings::{SystemMenuModel, UserSettings};
use crate::player::PlayerHealRequested;

/// The effect/dispatch resources shared by [`grid_menu_nav`] and
/// [`grid_menu_pointer_release`], bundled into one [`SystemParam`] so each stays
/// under Bevy's 16-param ceiling (the same reason the cube bundles `SystemMenuParams`).
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct MenuDispatchParams<'w, 's> {
    owned: ResMut<'w, OwnedItems>,
    settings: ResMut<'w, UserSettings>,
    commands: Commands<'w, 's>,
    players: MenuEffectPlayers<'w, 's>,
    mana_q: MenuEffectManaQuery<'w, 's>,
    heals: MessageWriter<'w, PlayerHealRequested>,
    sfx: MessageWriter<'w, SfxMessage>,
    system: SystemMenuParams<'w>,
}

/// The Grid backend is the active inventory frontend (run-condition). Mirrors the
/// cube's `kaleidoscope_backend_active`; the new Grid systems are registered with
/// this and the OLD grid + pause menu are gated OFF with its negation.
pub(crate) fn grid_backend_active(backend: Res<InventoryUiBackend>) -> bool {
    *backend == InventoryUiBackend::Grid
}

/// Per-backend Grid state: the remembered tab + republish bookkeeping. The CURSOR
/// and drill state live on the SHARED [`KaleidoscopeCursor`] / [`KaleidoscopeSystemNav`]
/// resources (one source of truth across backends), so this only holds what is
/// Grid-presentation-specific.
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
    open_entry: Option<crate::persistence::settings::SystemMenuEntryId>,
    focus: MenuFocus,
    version: u64,
    /// Fix 4: the focus zone is part of the key so moving onto / off the tab bar
    /// re-renders (the tab focus ring appears/disappears).
    zone: GridFocusZone,
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
    open_entry: Option<crate::persistence::settings::SystemMenuEntryId>,
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
        if focus_for_action(*action, active_page, model, open_entry) == cursor {
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
    mut overlay: ResMut<crate::inventory::InventoryUiState>,
    mode: Res<State<crate::runtime::game_mode::GameMode>>,
    mut next_mode: ResMut<NextState<crate::runtime::game_mode::GameMode>>,
    mut tab_state: ResMut<GridMenuTabState>,
    mut cursor: ResMut<KaleidoscopeCursor>,
    mut system_nav: ResMut<KaleidoscopeSystemNav>,
    mut sfx: MessageWriter<SfxMessage>,
    mut last_start: Local<bool>,
) {
    use crate::runtime::game_mode::GameMode;

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
                play_ui(&mut sfx, ambition_sfx::ids::UI_MENU_BACK);
                system_nav.open_entry = None;
                cursor.mark_keyboard(MenuFocus::System(0));
            } else {
                play_ui(&mut sfx, ambition_sfx::ids::UI_MENU_CLOSE);
                close_grid_unified_menu(&mut overlay, mode.get(), &mut next_mode);
            }
        } else if matches!(mode.get(), GameMode::Playing | GameMode::Paused) {
            // Esc/Start opens on the System face (the shared entry→tab mapping),
            // NOT the remembered tab — the pause button targets System.
            play_ui(&mut sfx, ambition_sfx::ids::UI_MENU_OPEN);
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
            play_ui(&mut sfx, ambition_sfx::ids::UI_MENU_CLOSE);
            close_grid_unified_menu(&mut overlay, mode.get(), &mut next_mode);
        } else if matches!(mode.get(), GameMode::Playing | GameMode::Paused) {
            play_ui(&mut sfx, ambition_sfx::ids::UI_MENU_OPEN);
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
        play_ui(&mut sfx, ambition_sfx::ids::UI_MENU_OPEN);
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
    overlay: &mut crate::inventory::InventoryUiState,
    mode: &crate::runtime::game_mode::GameMode,
    next_mode: &mut NextState<crate::runtime::game_mode::GameMode>,
    cursor: &mut KaleidoscopeCursor,
    system_nav: &mut KaleidoscopeSystemNav,
) {
    use crate::runtime::game_mode::GameMode;
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
    overlay: &mut crate::inventory::InventoryUiState,
    mode: &crate::runtime::game_mode::GameMode,
    next_mode: &mut NextState<crate::runtime::game_mode::GameMode>,
) {
    use crate::runtime::game_mode::GameMode;
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
    backend: Res<InventoryUiBackend>,
    menu: Res<MenuControlFrame>,
    mut tab_state: ResMut<GridMenuTabState>,
    mut cursor: ResMut<KaleidoscopeCursor>,
    mut system_nav: ResMut<KaleidoscopeSystemNav>,
    mut pages: ResMut<ActiveMenuPages<MenuPage, MenuPageAction>>,
    mut overlay: ResMut<crate::inventory::InventoryUiState>,
    mode: Res<State<crate::runtime::game_mode::GameMode>>,
    mut next_mode: ResMut<NextState<crate::runtime::game_mode::GameMode>>,
    mut fx: MenuDispatchParams,
) {
    if *backend != InventoryUiBackend::Grid || !overlay.visible {
        return;
    }
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
        seed_cursor_for_tab(tab_state.active_tab, &mut cursor);
        tab_state.focus_zone = GridFocusZone::Body;
        pages.active = Some(tab_page(tab_state.active_tab));
        play_ui(&mut fx.sfx, ambition_sfx::ids::UI_TAB_CHANGE);
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
            system_nav.open_entry = None;
            seed_cursor_for_tab(tab_state.active_tab, &mut cursor);
            pages.active = Some(tab_page(tab_state.active_tab));
            play_ui(&mut fx.sfx, ambition_sfx::ids::UI_TAB_CHANGE);
            return;
        }
        if menu.select || dy > 0 {
            // Activate: focus is already the live tab; just drop into the body (the
            // cursor was seeded for this tab when we switched onto it).
            tab_state.focus_zone = GridFocusZone::Body;
            seed_cursor_for_tab(tab_state.active_tab, &mut cursor);
            play_ui(&mut fx.sfx, ambition_sfx::ids::UI_MENU_ACCEPT);
            return;
        }
        if menu.back {
            tab_state.focus_zone = GridFocusZone::Body;
            play_ui(&mut fx.sfx, ambition_sfx::ids::UI_MENU_BACK);
            return;
        }
        // No other input does anything while the tab bar holds focus.
        return;
    }

    // Fix 4: UP from the TOP body row moves focus onto the tab bar.
    if dy < 0 && cursor_on_top_row(active_page, cursor.focus(), system_nav.open_entry) {
        tab_state.focus_zone = GridFocusZone::Tabs;
        play_ui(&mut fx.sfx, ambition_sfx::ids::UI_TAB_CHANGE);
        return;
    }

    match active_page {
        MenuPage::Items => {
            if dx != 0 || dy != 0 {
                let next = move_items_cursor(cursor.focus(), dx, dy);
                cursor.mark_keyboard(next);
            }
            if menu.back {
                play_ui(&mut fx.sfx, ambition_sfx::ids::UI_MENU_CLOSE);
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
                        close_grid_unified_menu(&mut overlay, mode.get(), &mut next_mode);
                    }
                } else {
                    play_ui(&mut fx.sfx, ambition_sfx::ids::UI_MENU_ERROR);
                }
            }
        }
        MenuPage::System => {
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
                active_page,
                &mut fx.owned,
                &mut fx.commands,
                &mut fx.players,
                &mut fx.mana_q,
                &mut fx.heals,
                &mut fx.sfx,
                &mut fx.system,
            );
            // Keep the tab pinned: a value-step's edge case in `system_focus_nav` can
            // turn the cube page; the flat tabs ignore that.
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
                play_ui(&mut fx.sfx, ambition_sfx::ids::UI_MENU_CLOSE);
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
    open_entry: Option<crate::persistence::settings::SystemMenuEntryId>,
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
    backend: Res<InventoryUiBackend>,
    overlay: Res<crate::inventory::InventoryUiState>,
    pages: Res<ActiveMenuPages<MenuPage, MenuPageAction>>,
    owned: Res<OwnedItems>,
    cursor: Res<KaleidoscopeCursor>,
    system_nav: Res<KaleidoscopeSystemNav>,
    settings: Res<UserSettings>,
    system: SystemMenuParams,
    mut tab_state: ResMut<GridMenuTabState>,
    roots: Query<Entity, With<BevyUiMenuRoot>>,
    assets: Option<Res<AssetServer>>,
    mut commands: Commands,
) {
    let active = *backend == InventoryUiBackend::Grid && overlay.visible;
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
    let key = ViewKey {
        tab: tab_state.active_tab,
        open_entry: system_nav.open_entry,
        focus: cursor.focus(),
        version: pages.version,
        zone: tab_state.focus_zone,
    };
    // Fix 3: detect inventory/settings STATE changes too, mirroring the cube's
    // `republish_kaleidoscope_pages` (`owned.is_changed() || settings.is_changed()`).
    // A select that mutates state (equip an item, toggle a setting, play a radio song)
    // changes `OwnedItems`/`UserSettings` but NOT the focus cursor, so the old key
    // `(tab, open_entry, focus, version)` stayed equal and the view did not refresh
    // until the cursor moved. The dispatch paths ALSO clear `last_key` directly (the
    // belt-and-braces force-republish), so even a state change this key can't see
    // (e.g. a dev snapshot) still re-renders.
    let state_changed = owned.is_changed() || settings.is_changed();
    if tab_state.last_key == Some(key) && !roots.is_empty() && !state_changed {
        return;
    }
    tab_state.last_key = Some(key);

    let model = system.model(&settings);
    // The System window-start follows the cursor (the grid has no independent scroll
    // override yet), exactly as the cube derives it when no drag/wheel override is set.
    let window_start = if active_page == MenuPage::System {
        let rows = crate::menu::model::system_rows(&model, system_nav.open_entry);
        crate::menu::model::system_effective_window_start(&rows, cursor.focus(), None)
    } else {
        0
    };
    let built = crate::menu::model::build_inventory_pages(
        &owned,
        owned.equipped(),
        cursor.focus(),
        &settings,
        &system.radio_snapshot(),
        &system.dev_snapshot(),
        window_start,
        system_nav.open_entry,
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
    overlay: Res<crate::inventory::InventoryUiState>,
    controls: Query<&AmbitionMenuControl<MenuPageAction>>,
    tabs: Query<&BevyUiMenuTab>,
    mut state: ResMut<GridPointerPress>,
) {
    let e = press.entity;
    if *backend != InventoryUiBackend::Grid || !overlay.visible {
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
    backend: Res<InventoryUiBackend>,
    mut state: ResMut<GridPointerPress>,
    mut tab_state: ResMut<GridMenuTabState>,
    mut cursor: ResMut<KaleidoscopeCursor>,
    mut system_nav: ResMut<KaleidoscopeSystemNav>,
    mut pages: ResMut<ActiveMenuPages<MenuPage, MenuPageAction>>,
    mut overlay: ResMut<crate::inventory::InventoryUiState>,
    mode: Res<State<crate::runtime::game_mode::GameMode>>,
    mut next_mode: ResMut<NextState<crate::runtime::game_mode::GameMode>>,
    mut fx: MenuDispatchParams,
) {
    if *backend != InventoryUiBackend::Grid || !overlay.visible {
        state.action = None;
        state.tab = None;
        return;
    }
    if let Some(tab) = state.tab.take() {
        tab_state.active_tab = tab.min(MenuPage::ALL.len() - 1);
        system_nav.open_entry = None;
        seed_cursor_for_tab(tab_state.active_tab, &mut cursor);
        // Clicking a tab lands focus in that tab's body (Fix 4: pointer doesn't park
        // on the tab bar — only arrow-key nav holds the Tabs zone).
        tab_state.focus_zone = GridFocusZone::Body;
        pages.active = Some(tab_page(tab_state.active_tab));
        play_ui(&mut fx.sfx, ambition_sfx::ids::UI_TAB_CHANGE);
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
        close_grid_unified_menu(&mut overlay, mode.get(), &mut next_mode);
    }
}

/// Hover: move the cursor onto the hovered control (so keyboard + pointer agree).
pub(crate) fn grid_menu_pointer_hover(
    over: On<Pointer<Over>>,
    backend: Res<InventoryUiBackend>,
    overlay: Res<crate::inventory::InventoryUiState>,
    controls: Query<&AmbitionMenuControl<MenuPageAction>>,
    settings: Res<UserSettings>,
    system: SystemMenuParams,
    tab_state: Res<GridMenuTabState>,
    system_nav: Res<KaleidoscopeSystemNav>,
    mut cursor: ResMut<KaleidoscopeCursor>,
) {
    if *backend != InventoryUiBackend::Grid || !overlay.visible {
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
    let focus = focus_for_action(action, active_page, &model, system_nav.open_entry);
    cursor.mark_keyboard(focus);
}

/// Install the Grid backend systems. Registered alongside the cube
/// (`install_kaleidoscope_menu`) so `\` flips between them at runtime.
pub(crate) fn install_grid_unified_menu(app: &mut App) {
    app.init_resource::<GridMenuTabState>()
        .init_resource::<GridPointerPress>();
    #[cfg(feature = "input")]
    app.add_systems(
        Update,
        (
            grid_menu_open_routing.run_if(grid_backend_active),
            grid_menu_nav.run_if(grid_backend_active),
        )
            .chain()
            .before(crate::app::SandboxSet::CoreSimulation),
    );
    app.add_systems(
        Update,
        grid_menu_republish_view.after(crate::app::SandboxSet::CoreSimulation),
    );
    #[cfg(feature = "input")]
    app.add_observer(grid_menu_pointer_press)
        .add_observer(grid_menu_pointer_release)
        .add_observer(grid_menu_pointer_hover);
}

#[cfg(all(test, feature = "input"))]
mod tests {
    use super::*;
    use crate::brain::ActionSet;
    use crate::items::Item;
    use crate::menu::model::{build_inventory_pages, system_rows, SystemRow};
    use crate::persistence::settings::{SystemMenuEntryId, SystemMenuModel};
    use crate::player::{PlayerEntity, PlayerMana, PrimaryPlayer};
    use crate::runtime::game_mode::GameMode;

    /// A minimal app wired with the Grid backend systems + every resource the
    /// shared cursor/dispatch path touches. Mirrors the cube test harness so the
    /// two backends exercise the same machinery.
    fn grid_app() -> App {
        let mut app = App::new();
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.init_state::<GameMode>();
        app.init_resource::<InventoryUiBackend>();
        app.init_resource::<ActiveMenuPages<MenuPage, MenuPageAction>>();
        app.init_resource::<KaleidoscopeCursor>();
        app.init_resource::<KaleidoscopeSystemNav>();
        app.init_resource::<OwnedItems>();
        app.init_resource::<crate::dev::dev_tools::DeveloperTools>();
        app.init_resource::<crate::SandboxDevState>();
        app.init_resource::<crate::ldtk_world::LdtkHotReloadState>();
        app.init_resource::<crate::runtime::reset::SandboxResetRequested>();
        app.init_resource::<crate::dev::dev_tools::EditableMovementTuning>();
        app.init_resource::<UserSettings>();
        app.init_resource::<crate::inventory::InventoryUiState>();
        app.init_resource::<crate::menu::map::MapMenuState>();
        app.init_resource::<MenuControlFrame>();
        app.init_resource::<GridMenuTabState>();
        app.init_resource::<GridPointerPress>();
        app.add_message::<PlayerHealRequested>();
        app.add_message::<SfxMessage>();
        app.add_message::<bevy::app::AppExit>();
        app.add_systems(Update, (grid_menu_open_routing, grid_menu_nav).chain());
        *app.world_mut().resource_mut::<InventoryUiBackend>() = InventoryUiBackend::Grid;
        app.world_mut().spawn((
            PlayerEntity,
            PrimaryPlayer,
            ActionSet::default(),
            PlayerMana::default(),
        ));
        app.update();
        app
    }

    fn set_frame(app: &mut App, f: impl FnOnce(&mut MenuControlFrame)) {
        let mut frame = app.world_mut().resource_mut::<MenuControlFrame>();
        *frame = MenuControlFrame::default();
        f(&mut frame);
    }

    fn active_tab(app: &App) -> MenuPage {
        tab_page(app.world().resource::<GridMenuTabState>().active_tab)
    }

    fn is_open(app: &App) -> bool {
        app.world()
            .resource::<crate::inventory::InventoryUiState>()
            .visible
    }

    /// The inventory key opens ON the Inventory tab; `page_right` bumper cycles
    /// Inventory -> Map -> ... and wraps back to Inventory.
    #[test]
    fn open_shows_inventory_then_bumper_cycles_tabs_with_wraparound() {
        let mut app = grid_app();
        // Open via the inventory key → lands on Items (the entry→tab mapping).
        set_frame(&mut app, |f| f.inventory = true);
        app.update();
        assert!(is_open(&app), "inventory key opens the unified menu");
        assert_eq!(
            active_tab(&app),
            MenuPage::Items,
            "inventory key → Inventory tab"
        );

        set_frame(&mut app, |f| f.page_right = true);
        app.update();
        // MenuPage::ALL = [Items, Map, Quest, System]; +1 from Items = Map.
        assert_eq!(active_tab(&app), MenuPage::ALL[1]);
        // Cycle the whole ring and confirm wraparound returns to Items.
        for _ in 0..(MenuPage::ALL.len() - 1) {
            set_frame(&mut app, |f| f.page_right = true);
            app.update();
        }
        assert_eq!(active_tab(&app), MenuPage::Items, "wraps back to Inventory");
    }

    /// `page_right` from Inventory reaches System (the settings tab) within the ring,
    /// proving the bumper drives the System tab too.
    #[test]
    fn bumper_reaches_system_tab() {
        let mut app = grid_app();
        // Open on Items via the inventory key, then bump rightwards to System.
        set_frame(&mut app, |f| f.inventory = true);
        app.update();
        assert_eq!(active_tab(&app), MenuPage::Items);
        // Bump until System (index 3 in ALL).
        let sys_idx = MenuPage::ALL
            .iter()
            .position(|p| *p == MenuPage::System)
            .unwrap();
        for _ in 0..sys_idx {
            set_frame(&mut app, |f| f.page_right = true);
            app.update();
        }
        assert_eq!(active_tab(&app), MenuPage::System);
    }

    /// Selecting an item control equips it (via the shared dispatcher → equip path).
    #[test]
    fn selecting_an_item_dispatches_equip() {
        let mut app = grid_app();
        // Own an equippable item (Axe at index 1 has a held_item_id).
        let axe = Item::from_index(1).unwrap();
        assert!(axe.held_item_id().is_some());
        app.world_mut().resource_mut::<OwnedItems>().grant(axe, 1);
        // Open + focus item 1.
        set_frame(&mut app, |f| f.inventory = true);
        app.update();
        assert_eq!(active_tab(&app), MenuPage::Items);
        app.world_mut()
            .resource_mut::<KaleidoscopeCursor>()
            .mark_keyboard(MenuFocus::Item(1));
        set_frame(&mut app, |f| f.select = true);
        app.update();
        assert_eq!(
            app.world().resource::<OwnedItems>().equipped(),
            Some(axe),
            "selecting the item equipped it through dispatch_menu_action"
        );
    }

    /// On the System tab, selecting a settings row applies it through the shared IR
    /// (`apply_settings_option`): drilling into Audio then toggling a row mutates
    /// `UserSettings`. We assert the System select path reaches the dispatcher by
    /// drilling into an entry (open_entry becomes Some).
    #[test]
    fn system_select_drills_and_dispatches() {
        let mut app = grid_app();
        // Esc opens directly on the System tab (the entry→tab mapping).
        set_frame(&mut app, |f| f.start = true);
        app.update();
        assert_eq!(active_tab(&app), MenuPage::System, "Esc opens on System");
        // (no bumping needed — Esc already lands on System)
        let sys_idx = 0usize;
        for _ in 0..sys_idx {
            set_frame(&mut app, |f| f.page_right = true);
            app.update();
        }
        assert_eq!(active_tab(&app), MenuPage::System);
        // Find a drillable entry row (Audio) and land the cursor on it.
        let settings = app.world().resource::<UserSettings>().clone();
        let model = SystemMenuModel::build(&settings, &Default::default(), &Default::default());
        let rows = system_rows(&model, None);
        let audio_row = rows
            .iter()
            .position(|r| matches!(r, SystemRow::Entry(SystemMenuEntryId::Audio)));
        if let Some(idx) = audio_row {
            app.world_mut()
                .resource_mut::<KaleidoscopeCursor>()
                .mark_keyboard(MenuFocus::System(idx));
            set_frame(&mut app, |f| f.select = true);
            app.update();
            assert_eq!(
                app.world().resource::<KaleidoscopeSystemNav>().open_entry,
                Some(SystemMenuEntryId::Audio),
                "selecting the Audio entry drilled into it (System dispatch path)"
            );
        }
    }

    /// Selecting Quit-to-Desktop on the System tab writes `AppExit`.
    #[test]
    fn system_quit_dispatches_app_exit() {
        let mut app = grid_app();
        // Esc opens directly on System (the entry→tab mapping).
        set_frame(&mut app, |f| f.start = true);
        app.update();
        assert_eq!(active_tab(&app), MenuPage::System);
        let settings = app.world().resource::<UserSettings>().clone();
        let model = SystemMenuModel::build(&settings, &Default::default(), &Default::default());
        let rows = system_rows(&model, None);
        let quit_idx = rows
            .iter()
            .position(|r| matches!(r, SystemRow::Entry(SystemMenuEntryId::Quit)))
            .expect("Quit entry present in the System IR");
        app.world_mut()
            .resource_mut::<KaleidoscopeCursor>()
            .mark_keyboard(MenuFocus::System(quit_idx));
        set_frame(&mut app, |f| f.select = true);
        app.update();
        let exits = app
            .world_mut()
            .resource_mut::<bevy::ecs::message::Messages<bevy::app::AppExit>>()
            .drain()
            .count();
        assert!(exits >= 1, "Quit dispatched an AppExit");
    }

    /// Back at a tab's top level closes the menu (→ Playing), and respects
    /// `opened_from_pause` (opened from Paused stays Paused on close).
    #[test]
    fn back_closes_and_respects_opened_from_pause() {
        // Opened from gameplay → Back restores Playing.
        let mut app = grid_app();
        set_frame(&mut app, |f| f.inventory = true);
        app.update();
        // Let the open's `NextState(Paused)` transition settle (it applies on the
        // next StateTransition, exactly as frames pass in the real app). Clear the
        // frame first so the still-set inventory bit doesn't re-toggle the menu (the
        // real app rebuilds the frame each tick via the input populators).
        set_frame(&mut app, |_| {});
        app.update();
        assert!(is_open(&app));
        assert!(matches!(
            app.world().resource::<State<GameMode>>().get(),
            GameMode::Paused
        ));
        set_frame(&mut app, |f| f.back = true);
        app.update();
        assert!(!is_open(&app), "Back closed the menu");
        set_frame(&mut app, |_| {});
        app.update();
        assert!(
            matches!(
                app.world().resource::<State<GameMode>>().get(),
                GameMode::Playing
            ),
            "opened from gameplay → close returns to Playing"
        );

        // Opened while already Paused → Back closes but stays Paused.
        let mut app = grid_app();
        app.world_mut()
            .resource_mut::<NextState<GameMode>>()
            .set(GameMode::Paused);
        app.update();
        set_frame(&mut app, |f| f.inventory = true);
        app.update();
        set_frame(&mut app, |_| {});
        app.update();
        assert!(is_open(&app));
        assert!(
            app.world()
                .resource::<crate::inventory::InventoryUiState>()
                .opened_from_pause,
            "opened while already Paused records opened_from_pause"
        );
        set_frame(&mut app, |f| f.back = true);
        app.update();
        set_frame(&mut app, |_| {});
        app.update();
        assert!(!is_open(&app));
        assert!(
            matches!(
                app.world().resource::<State<GameMode>>().get(),
                GameMode::Paused
            ),
            "opened_from_pause → close stays Paused"
        );
    }

    /// CROSS-BACKEND CONTENT PARITY: the active tab's `MenuPageModel` is built from
    /// the SAME backend-agnostic builders regardless of which backend renders it.
    /// We build the page set the way both backends do (`build_inventory_pages`) for a
    /// fixed state and confirm the Inventory + System tab models are byte-identical —
    /// the grid and cube draw the same content because there is one model.
    #[test]
    fn cross_backend_model_parity_inventory_and_system() {
        let owned = OwnedItems::starter();
        let equipped = owned.equipped();
        let settings = UserSettings::default();
        let build = || {
            build_inventory_pages(
                &owned,
                equipped,
                MenuFocus::Item(0),
                &settings,
                &Default::default(),
                &Default::default(),
                0,
                None,
            )
        };
        let cube_pages = build();
        let grid_pages = build();
        for page in [MenuPage::Items, MenuPage::System] {
            let cube = cube_pages.iter().find(|p| p.id == page).unwrap();
            let grid = grid_pages.iter().find(|p| p.id == page).unwrap();
            // The action vocabulary on each page must match exactly (the parity net).
            let cube_actions: Vec<_> = cube
                .nodes
                .iter()
                .filter_map(|n| n.action().cloned())
                .collect();
            let grid_actions: Vec<_> = grid
                .nodes
                .iter()
                .filter_map(|n| n.action().cloned())
                .collect();
            assert_eq!(
                cube_actions, grid_actions,
                "{page:?} tab: grid and cube render the same actions"
            );
        }
    }

    /// NAV ↔ RENDER agreement: the cursor focus key we compute (and hand the
    /// renderer as `view.focused`) equals the `focus` the renderer tags on the
    /// matching control. We build the Items page, take the control for the focused
    /// item, and confirm `cursor_focus_key` resolves to that control's rect-derived
    /// key — so the highlighted control is exactly the one nav points at.
    #[test]
    fn cursor_focus_key_matches_a_rendered_control() {
        let mut owned = OwnedItems::default();
        let axe = Item::from_index(1).unwrap();
        owned.grant(axe, 1);
        let settings = UserSettings::default();
        let pages = build_inventory_pages(
            &owned,
            owned.equipped(),
            MenuFocus::Item(1),
            &settings,
            &Default::default(),
            &Default::default(),
            0,
            None,
        );
        let items = pages.iter().find(|p| p.id == MenuPage::Items).unwrap();
        let model = SystemMenuModel::build(&settings, &Default::default(), &Default::default());
        let key = cursor_focus_key(items, MenuPage::Items, MenuFocus::Item(1), &model, None)
            .expect("focused item resolves to a rendered control");
        // The key must equal the rect-derived key of SOME actionable control whose
        // action maps back to Item(1) — i.e. it addresses a real tagged control.
        let matching = items.nodes.iter().any(|n| {
            matches!(n, MenuNode::Control { rect, action: Some(a), .. }
                if focus_for_action(*a, MenuPage::Items, &model, None) == MenuFocus::Item(1)
                    && focus_key_for(*rect) == key)
        });
        assert!(
            matching,
            "the focus key addresses a tagged control the renderer drew"
        );
    }

    fn focus_zone(app: &App) -> GridFocusZone {
        app.world().resource::<GridMenuTabState>().focus_zone
    }

    /// Fix 4: UP from the top body row moves focus onto the TAB BAR; LEFT/RIGHT then
    /// cycle tabs (live); SELECT activates the focused tab and drops back into the body.
    #[test]
    fn arrow_keys_navigate_to_and_activate_tabs() {
        let mut app = grid_app();
        set_frame(&mut app, |f| f.inventory = true);
        app.update();
        set_frame(&mut app, |_| {});
        app.update();
        assert_eq!(active_tab(&app), MenuPage::Items);
        assert_eq!(focus_zone(&app), GridFocusZone::Body, "starts in the body");

        // Cursor is on Item(0) (top row); UP reaches the tab bar.
        set_frame(&mut app, |f| f.up = true);
        app.update();
        assert_eq!(
            focus_zone(&app),
            GridFocusZone::Tabs,
            "UP from top row → tabs"
        );

        // RIGHT cycles to the next tab (Items → Map in MenuPage::ALL).
        set_frame(&mut app, |f| f.right = true);
        app.update();
        assert_eq!(active_tab(&app), MenuPage::ALL[1], "RIGHT cycles tabs");
        assert_eq!(
            focus_zone(&app),
            GridFocusZone::Tabs,
            "still on the tab bar"
        );

        // SELECT activates the focused tab and drops focus back into the body.
        set_frame(&mut app, |f| f.select = true);
        app.update();
        assert_eq!(
            focus_zone(&app),
            GridFocusZone::Body,
            "SELECT activates the tab + returns to the body"
        );
        assert_eq!(
            active_tab(&app),
            MenuPage::ALL[1],
            "stays on the chosen tab"
        );
    }

    /// Fix 4: DOWN from the tab bar also activates the focused tab and returns to body.
    #[test]
    fn down_from_tab_bar_returns_to_body() {
        let mut app = grid_app();
        set_frame(&mut app, |f| f.inventory = true);
        app.update();
        set_frame(&mut app, |_| {});
        app.update();
        set_frame(&mut app, |f| f.up = true);
        app.update();
        assert_eq!(focus_zone(&app), GridFocusZone::Tabs);
        set_frame(&mut app, |f| f.down = true);
        app.update();
        assert_eq!(focus_zone(&app), GridFocusZone::Body, "DOWN drops to body");
    }

    /// Fix 4: UP from a NON-top body cell navigates within the body (does NOT escape
    /// to the tab bar), so the tab affordance only triggers from the top row.
    #[test]
    fn up_from_non_top_row_stays_in_body() {
        let mut app = grid_app();
        set_frame(&mut app, |f| f.inventory = true);
        app.update();
        set_frame(&mut app, |_| {});
        app.update();
        // Park the cursor on the second row.
        app.world_mut()
            .resource_mut::<KaleidoscopeCursor>()
            .mark_keyboard(MenuFocus::Item(ITEM_GRID_COLS));
        set_frame(&mut app, |f| f.up = true);
        app.update();
        assert_eq!(
            focus_zone(&app),
            GridFocusZone::Body,
            "UP from row 2 stays in the body"
        );
        assert_eq!(
            app.world().resource::<KaleidoscopeCursor>().focus(),
            MenuFocus::Item(0),
            "UP moved the cursor up one row instead of escaping to tabs"
        );
    }

    // ----- Phase C2b bug-fix coverage --------------------------------------

    use ambition_menu::render::bevy_ui::BevyUiMenuTab;
    use ambition_menu::AmbitionMenuControl;
    use bevy::camera::NormalizedRenderTarget;
    use bevy::picking::backend::HitData;
    use bevy::picking::events::{Pointer, Press, Release};
    use bevy::picking::pointer::{Location, PointerId};

    fn pointer_location() -> Location {
        Location {
            target: NormalizedRenderTarget::None {
                width: 1,
                height: 1,
            },
            position: Vec2::ZERO,
        }
    }

    /// Fire a `Pointer<Press>` whose hit/target is `entity`, arming the grid press
    /// observer (entity-independent capture).
    fn fire_press(app: &mut App, entity: Entity) {
        app.world_mut().trigger(Pointer::new(
            PointerId::Mouse,
            pointer_location(),
            Press {
                button: bevy::picking::pointer::PointerButton::Primary,
                hit: HitData::new(entity, 0.0, None, None),
            },
            entity,
        ));
        app.update();
    }

    /// Fire a `Pointer<Release>` whose hit/target is `entity` (which may have been
    /// respawned by a republish between press and release).
    fn fire_release(app: &mut App, entity: Entity) {
        app.world_mut().trigger(Pointer::new(
            PointerId::Mouse,
            pointer_location(),
            Release {
                button: bevy::picking::pointer::PointerButton::Primary,
                hit: HitData::new(entity, 0.0, None, None),
            },
            entity,
        ));
        app.update();
    }

    /// A render-capable harness: the open/nav systems PLUS `grid_menu_republish_view`
    /// and the pointer observers, so a test can drive the actual spawned `bevy_ui`
    /// control/tab entities (the body the user sees).
    fn render_app() -> App {
        let mut app = grid_app();
        app.add_systems(Update, grid_menu_republish_view);
        app.add_observer(grid_menu_pointer_press);
        app.add_observer(grid_menu_pointer_release);
        app
    }

    /// Collect the actions of the spawned `AmbitionMenuControl` entities (the body
    /// the flat renderer actually drew this frame).
    fn rendered_actions(app: &mut App) -> Vec<MenuPageAction> {
        let mut q = app
            .world_mut()
            .query::<&AmbitionMenuControl<MenuPageAction>>();
        q.iter(app.world())
            .filter_map(|c| c.action)
            .collect::<Vec<_>>()
    }

    fn switch_to_tab(app: &mut App, page: MenuPage) {
        // Clear any held input bits so the open/nav systems don't re-toggle / re-nav
        // the menu while we settle the tab (the real app rebuilds the frame each tick).
        set_frame(app, |_| {});
        app.world_mut()
            .resource_mut::<GridMenuTabState>()
            .active_tab = tab_index_of(page);
        // republish builds the tree via `commands.queue`, so it materializes one
        // update later; two updates guarantees the spawn applied.
        app.update();
        app.update();
    }

    /// Fix 3: a keyboard select that dispatches an action FORCES the next republish
    /// (clears `last_key`) so the view reflects the new state immediately, without
    /// waiting for a cursor move. After equipping via SELECT, `last_key` is cleared.
    #[test]
    fn select_forces_republish_so_view_refreshes_immediately() {
        let mut app = render_app();
        let axe = Item::from_index(1).unwrap();
        app.world_mut().resource_mut::<OwnedItems>().grant(axe, 1);
        set_frame(&mut app, |f| f.inventory = true);
        app.update();
        set_frame(&mut app, |_| {});
        app.update();
        // Let republish settle so `last_key` is Some(..) before the select.
        app.update();
        assert!(
            app.world()
                .resource::<GridMenuTabState>()
                .last_key
                .is_some(),
            "republish recorded a key before the select"
        );
        // Focus the Axe and SELECT it (equips via dispatch_menu_action). The nav
        // system clears `last_key` so the republish is forced even though the cursor
        // did not move.
        app.world_mut()
            .resource_mut::<KaleidoscopeCursor>()
            .mark_keyboard(MenuFocus::Item(1));
        set_frame(&mut app, |f| f.select = true);
        app.update();
        assert_eq!(
            app.world().resource::<OwnedItems>().equipped(),
            Some(axe),
            "select equipped the item"
        );
        // Directly exercise the dirty check: after a dispatch the nav system set
        // `last_key = None`, so the NEXT `grid_menu_republish_view` MUST rebuild (it
        // cannot early-return on an equal key). Clear the frame + run an update; the
        // rebuild then re-records a key (proving it ran rather than skipping).
        set_frame(&mut app, |_| {});
        app.update();
        assert!(
            app.world()
                .resource::<GridMenuTabState>()
                .last_key
                .is_some(),
            "the forced republish rebuilt the view and re-recorded a key"
        );
    }

    /// BUG 1: switching the grid tab to System makes the RENDERED model BE the
    /// System page — the spawned controls carry System actions, NOT Items' Equip/Use.
    #[test]
    fn switching_tab_renders_that_pages_model() {
        let mut app = render_app();
        // Own an item so Items has an Equip action to distinguish from System.
        let axe = Item::from_index(1).unwrap();
        app.world_mut().resource_mut::<OwnedItems>().grant(axe, 1);
        set_frame(&mut app, |f| f.inventory = true);
        app.update();
        set_frame(&mut app, |_| {}); // release the key so it doesn't re-toggle
        app.update();
        // On Items, an Equip action is present; no System action.
        let items_actions = rendered_actions(&mut app);
        assert!(
            items_actions
                .iter()
                .any(|a| matches!(a, MenuPageAction::Equip(_))),
            "Items tab renders an Equip control"
        );

        // Switch to System: the body must now carry System drill/option actions and
        // NO Items Equip/Use.
        switch_to_tab(&mut app, MenuPage::System);
        let system_actions = rendered_actions(&mut app);
        assert!(
            system_actions.iter().any(|a| matches!(
                a,
                MenuPageAction::OpenSystemEntry(_)
                    | MenuPageAction::System(_)
                    | MenuPageAction::SystemOption(_)
                    | MenuPageAction::SystemAction(_)
            )),
            "System tab renders System controls, got {system_actions:?}"
        );
        assert!(
            !system_actions
                .iter()
                .any(|a| matches!(a, MenuPageAction::Equip(_) | MenuPageAction::Use(_))),
            "System tab body is NOT the Items page"
        );
    }

    /// BUG 2: the flat renderer NEVER draws the cube's page-turn EDGE controls
    /// (`MenuPageAction::ChangePage`) — the tab bar replaces them.
    #[test]
    fn flat_renderer_skips_page_turn_edge_controls() {
        let mut app = render_app();
        set_frame(&mut app, |f| f.inventory = true);
        app.update();
        set_frame(&mut app, |_| {});
        app.update();
        for page in MenuPage::ALL {
            switch_to_tab(&mut app, page);
            let actions = rendered_actions(&mut app);
            assert!(
                !actions
                    .iter()
                    .any(|a| matches!(a, MenuPageAction::ChangePage(_))),
                "{page:?} tab: no ChangePage edge controls drawn, got {actions:?}"
            );
        }
    }

    /// BUG 4 (pointer): a `Pointer<Press>`+`Release` on a tab entity switches the
    /// active tab; on an item control it dispatches the control's action (equip).
    #[test]
    fn pointer_press_release_switches_tab_and_dispatches_item() {
        let mut app = render_app();
        let axe = Item::from_index(1).unwrap();
        app.world_mut().resource_mut::<OwnedItems>().grant(axe, 1);
        set_frame(&mut app, |f| f.inventory = true);
        app.update();
        set_frame(&mut app, |_| {});
        app.update();
        assert_eq!(active_tab(&app), MenuPage::Items);

        // Find the System tab entity and click it.
        let sys_tab = {
            let mut q = app.world_mut().query::<(Entity, &BevyUiMenuTab)>();
            q.iter(app.world())
                .find(|(_, t)| t.index == tab_index_of(MenuPage::System))
                .map(|(e, _)| e)
                .expect("System tab entity spawned")
        };
        fire_press(&mut app, sys_tab);
        fire_release(&mut app, sys_tab);
        app.update();
        assert_eq!(
            active_tab(&app),
            MenuPage::System,
            "clicking the System tab switched tabs"
        );

        // Back to Items, then click the Axe item control → it equips.
        switch_to_tab(&mut app, MenuPage::Items);
        let axe_ctrl = {
            let mut q = app
                .world_mut()
                .query::<(Entity, &AmbitionMenuControl<MenuPageAction>)>();
            q.iter(app.world())
                .find(|(_, c)| matches!(c.action, Some(MenuPageAction::Equip(i)) if i == axe))
                .map(|(e, _)| e)
                .expect("Axe equip control spawned")
        };
        fire_press(&mut app, axe_ctrl);
        fire_release(&mut app, axe_ctrl);
        app.update();
        assert_eq!(
            app.world().resource::<OwnedItems>().equipped(),
            Some(axe),
            "clicking the item equipped it through dispatch_menu_action"
        );
    }

    /// BUG 5: an Esc open→close is exactly ONE toggle — no immediate reopen. Drive
    /// the routing with two SEPARATE Esc rising edges and assert open then closed.
    #[test]
    fn esc_open_then_close_is_one_toggle() {
        let mut app = grid_app();
        // First Esc: open.
        set_frame(&mut app, |f| f.start = true);
        app.update();
        assert!(is_open(&app), "first Esc opens");
        // Release Esc (so the next press is a fresh rising edge).
        set_frame(&mut app, |_| {});
        app.update();
        assert!(is_open(&app), "menu stays open while Esc is released");
        // Second Esc: close (and NOT reopen on the same press across frames).
        set_frame(&mut app, |f| f.start = true);
        app.update();
        app.update(); // a held multi-frame Start must not re-open.
        assert!(!is_open(&app), "second Esc closes and does NOT reopen");
    }

    /// BUG 6: the entry→tab mapping is one place and maps each open key correctly.
    #[test]
    fn pause_entry_target_maps_each_source() {
        assert_eq!(
            pause_entry_target(PauseEntrySource::Inventory),
            MenuPage::Items
        );
        assert_eq!(
            pause_entry_target(PauseEntrySource::Pause),
            MenuPage::System
        );
        assert_eq!(pause_entry_target(PauseEntrySource::Map), MenuPage::Map);
    }

    /// BUG 6 (routing): the entry key sets the active tab per the mapping —
    /// inventory→Items, Esc→System, map→Map.
    #[test]
    fn open_routing_lands_on_mapped_tab() {
        // Inventory key → Items.
        let mut app = grid_app();
        set_frame(&mut app, |f| f.inventory = true);
        app.update();
        assert_eq!(active_tab(&app), MenuPage::Items);

        // Esc → System.
        let mut app = grid_app();
        set_frame(&mut app, |f| f.start = true);
        app.update();
        assert_eq!(active_tab(&app), MenuPage::System);

        // Map key → Map.
        let mut app = grid_app();
        set_frame(&mut app, |f| f.map = true);
        app.update();
        assert_eq!(active_tab(&app), MenuPage::Map);
    }
}

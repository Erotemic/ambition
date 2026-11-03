//! Flat tabbed menu backend (`InventoryUiBackend::Grid`).
//!
//! Wires `ambition_platformer2d::menu::render::bevy_ui::spawn_bevy_ui_menu` into the same menu
//! model, actions and menu-owned cursor used by the 3D kaleidoscope backend. Only the
//! presentation differs: a flat tab bar and body instead of cube faces.
//!
//! Owned systems, gated to `backend == Grid`:
//! - tab switching and remembered active tab;
//! - keyboard/gamepad focus navigation and action dispatch;
//! - dirty republish of the active page model through the Bevy-UI renderer;
//! - shared Bevy picking activation for touch/mouse rows and tabs;
//! - pointer hover handling for mouse focus ownership.

use bevy::prelude::*;

use ambition_platformer2d::menu::render::bevy_ui::{
    install_bevy_ui_menu_actions, install_bevy_ui_menu_tabs, BevyUiMenuInteractionSet,
    BevyUiMenuRoot, BevyUiMenuTabSpec, BevyUiMenuView,
};
use ambition_platformer2d::menu::{
    ActiveMenuPages, MenuActionActivated, MenuFocusKey, MenuNode, MenuRect, MenuTabActivated,
};

use crate::menu::effects::MenuBodySource;
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
#[cfg(test)]
use ambition_platformer2d::input::MenuControlFrame;
use ambition_platformer2d::items::OwnedItems;
use ambition_platformer2d::menu::backend::{InventoryUiBackend, BEVY_UI_MENU_BACKEND_ENABLED};
use ambition_platformer2d::persistence::settings::{UserSettings, VisualQualityProfile};
use ambition_platformer2d::settings_menu::system::{SystemMenuEntryId, SystemMenuModel};
use ambition_platformer2d::sfx::SfxWriter;

/// Item and settings effects used by Grid navigation.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct MenuNavParams<'w, 's> {
    owned: Res<'w, OwnedItems>,
    body_source: MenuBodySource<'w, 's>,
    settings: ResMut<'w, UserSettings>,
    sfx: SfxWriter<'w>,
    system: SystemMenuParams<'w, 's>,
}

/// Run condition: the Grid backend is the active inventory frontend.
pub(crate) fn grid_backend_active(backend: Res<InventoryUiBackend>) -> bool {
    BEVY_UI_MENU_BACKEND_ENABLED && backend.effective() == InventoryUiBackend::Grid
}

/// Grid-only state: republish bookkeeping and the flat menu's own focus zone.
/// Shared cursor and drill state stay on [`KaleidoscopeCursor`] /
/// [`KaleidoscopeSystemNav`].
///
/// ⛔⛤ **IT NO LONGER REMEMBERS THE TAB, AND THAT WAS A SECOND AUTHORITY.** This
/// held `active_tab: usize` — an index into [`MenuPage::ALL`] — while
/// `ActiveMenuPages.active` held the same fact as a [`MenuPage`], and the two were
/// kept in step by hand: `grid_menu_nav` assigned the index and mirrored it into
/// `pages.active` three lines later, and a whole system
/// (`sync_menu_page_across_backend_switch`) existed to bridge them across a
/// backend switch, carrying a THIRD copy in a `Local` *"so a switch can carry it
/// even after `grid_menu_nav` clobbers the live `pages.active`"*.
///
/// ⇒ `ActiveMenuPages.active` is the sole owner now, and the index is DERIVED at
/// the two places that need one. Nothing can disagree, so the bridge, its
/// `Local`s and its ordering edge are gone — and the cross-backend parity test
/// that could never see the disagreement is gone with them. MAKE IT IMPOSSIBLE,
/// NOT CHECKED.
#[derive(Component)]
pub(crate) struct GridMenuTabState {
    rendered_font: Option<bevy::text::FontSource>,
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

impl GridMenuTabState {
    pub(crate) fn invalidate(&mut self) {
        self.last_key = None;
    }

    pub(crate) fn reset_navigation(&mut self) {
        self.focus_zone = GridFocusZone::Body;
        self.system_window_start = None;
    }
}

impl Default for GridMenuTabState {
    fn default() -> Self {
        Self {
            rendered_font: None,
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
    layout: crate::menu::model::InventoryLayout,
    system_revision: u64,
    tab: usize,
    open_entry: Option<SystemMenuEntryId>,
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
    /// What the menu owner has equipped: an
    /// equip changes a body component, not the bag, so the bag's change tick
    /// cannot stand in for it.
    equipped: ambition_platformer2d::items::EquippedItems,
    room: Option<ambition_platformer2d::platformer::lifecycle::LiveRoomInstance>,
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
    use ambition_platformer2d::sfx::ids;
    use ambition_platformer2d::sfx::SfxId;

    pub const CLOSE: SfxId = ids::UI_PAUSE_CLOSE;
    pub const ACCEPT: SfxId = ids::UI_ACCEPT;
    pub const BACK: SfxId = ids::UI_BACK;
    pub const ERROR: SfxId = ids::UI_ERROR;
    pub const TAB_CHANGE: SfxId = ids::UI_TAB_CHANGE;
}

/// The active tab's [`MenuPage`].
/// The page `ActiveMenuPages` names, or the landing tab before anything published.
///
/// ⭐ `Items` IS THE OLD DEFAULT, NOT A NEW CHOICE: `MenuPage::ALL[0]` is
/// `MenuPage::Items`, so this agrees exactly with the `active_tab: 0` initializer
/// it replaces. `active` is only `None` before the first `replace_pages`, which
/// always assigns `Some`.
fn active_page(pages: &ActiveMenuPages<MenuPage, MenuPageAction>) -> MenuPage {
    pages.active.unwrap_or(MenuPage::Items)
}

/// The active page as an index, for the two places that need to do arithmetic on
/// it (bumper/arrow tab cycling, and the republish view key).
fn active_tab_index(pages: &ActiveMenuPages<MenuPage, MenuPageAction>) -> usize {
    tab_index_of(active_page(pages))
}

fn tab_page(active_tab: usize) -> MenuPage {
    MenuPage::ALL[active_tab.min(MenuPage::ALL.len() - 1)]
}

/// The [`MenuPage::ALL`] index of a page (for seeding `active_tab`).
fn tab_index_of(page: MenuPage) -> usize {
    MenuPage::ALL.iter().position(|p| *p == page).unwrap_or(0)
}

// ⛔⛤ **`sync_menu_page_across_backend_switch` WAS DELETED HERE ON 2026-09-12,
// AND EVERY LINE OF IT WAS A CONSEQUENCE OF ONE FACT BEING STORED TWICE.** It
// carried the active page across an inventory-backend switch, because the cube
// kept it in `ActiveMenuPages.active` and the grid kept it in
// `GridMenuTabState.active_tab`. To do that it needed a `Local` SNAPSHOT of the
// page — a THIRD copy — re-taken every stable frame, *"so a switch can carry it
// even after `grid_menu_nav` clobbers the live `pages.active`"*; a second `Local`
// to detect the switch edge; a `match` to write into whichever encoding the
// ARRIVING backend treated as authoritative; and an ordering constraint placing
// it before BOTH republish systems.
//
// ⇒ With `ActiveMenuPages.active` as the sole owner there is nothing to carry:
// both backends read the field the other wrote, so the page survives a switch BY
// CONSTRUCTION. Its test went too — `backend_switch_carries_the_active_page` added
// this very system and asserted the BRIDGE worked, not that a user lands on the
// same page, so it could not outlive the bridge. The behaviour is still guarded,
// by the arms that assert which tab the user sees after input
// (`bumper_reaches_system_tab`, `arrow_keys_navigate_to_and_activate_tabs`,
// `system_tab_left_right_never_turns_the_page`), none of which names a field.

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
        MenuPage::Items => "Equipment",
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
    page: &ambition_platformer2d::menu::MenuPageModel<MenuPage, MenuPageAction>,
    active_page: MenuPage,
    cursor: MenuFocus,
    model: &SystemMenuModel,
    open_entry: Option<SystemMenuEntryId>,
    pending_quality: Option<VisualQualityProfile>,
) -> Option<MenuFocusKey> {
    // ⛔ BUILT ONCE, OUTSIDE THE LOOP. This asks every actionable node "is your
    // focus the cursor's?", and for a System row the answer is an index into this
    // list. Rebuilding it per node made one cursor lookup cost N list builds — an
    // allocation and a copy of every row id each time, to read one `.position()`.
    let rows = system_rows_with_quality_prompt(model, open_entry, pending_quality);
    for node in &page.nodes {
        let MenuNode::Control {
            rect,
            action: Some(action),
            ..
        } = node
        else {
            continue;
        };
        if focus_for_action(*action, active_page, &rows) == cursor {
            return Some(focus_key_for(*rect));
        }
    }
    None
}

/// Seed a sensible cursor for the tab being shown.
fn seed_cursor_for_tab(active_tab: usize, cursor: &mut KaleidoscopeCursor) {
    cursor.mark_keyboard(match tab_page(active_tab) {
        MenuPage::Items => MenuFocus::Item(0),
        MenuPage::System => MenuFocus::System(0),
        MenuPage::Map | MenuPage::Quest => MenuFocus::EdgeLeft,
    });
}

/// Keyboard / gamepad navigation for the Grid backend. Bumpers switch tabs
/// (wraparound); up/down/left/right move the focus cursor over the active page;
/// `select` dispatches the focused control's action; `back` pops a System drill
/// else closes. The Esc toggle is owned by [`crate::menu::lifecycle::route_menu_lifecycle`] (it bails on
/// `menu.start`), so this never fights it.
#[cfg(feature = "input")]
#[allow(clippy::too_many_arguments)]
pub(crate) fn grid_menu_nav(
    access: ambition_platformer2d::input::ParticipantContextAccess,
    regions: Query<&super::regions::MenuRegion>,
    mut menu_frames: ResMut<ambition_platformer2d::input::SeatMenuFrames>,
    mut menus: Query<(
        Entity,
        &ambition_platformer2d::inventory_ui::InventoryMenu,
        &mut ActiveMenuPages<MenuPage, MenuPageAction>,
        &mut GridMenuTabState,
        &mut KaleidoscopeCursor,
        &mut KaleidoscopeSystemNav,
        &mut VisualQualityConfirmState,
        &mut ambition_platformer2d::inventory_ui::InventoryUiState,
    )>,
    // The ONE activation event. A controller/keyboard submit publishes the same
    // message the renderer's pointer bridge does, so `crate::menu::dispatch::menu_action_activated`
    // is the single place a chosen action is dispatched.
    mut activated: MessageWriter<MenuActionActivated<MenuPageAction>>,
    mut fx: MenuNavParams,
) {
    for (
        menu_entity,
        menu_owner,
        mut pages,
        mut tab_state,
        mut cursor,
        mut system_nav,
        mut quality_confirm,
        mut overlay,
    ) in &mut menus
    {
        // Read the backend from `fx.system` (it owns the resource); a separate `Res`
        // here would be a B0002 conflict with that `ResMut`.
        if fx.system.backend() != InventoryUiBackend::Grid || !overlay.visible {
            // Not the active backend — leave the frame for whichever nav owns it.
            continue;
        }
        if !super::inventory_input_allowed(&access, menu_owner.owner) { continue; }
        // Consume only this seat's edges before a backend switch can expose them again.
        let menu = menu_frames.for_seat(menu_owner.owner.slot());
        menu_frames.consume_nav_edges(menu_owner.owner.slot());
        // The Esc/Start toggle is owned by `route_menu_lifecycle`; bail so a single Esc
        // can't both close here and reopen there (the cube's Esc-Esc reopen guard).
        if menu.start {
            continue;
        }

        // Bumpers cycle tabs with wraparound (the shared MenuControlFrame contract).
        // Bumpers act regardless of focus zone and leave focus in the body (they're the
        // "fast" tab switch); arrow-key tab nav is the discoverable alternative (Fix 4).
        let bump = (menu.page_right as i32) - (menu.page_left as i32);
        if bump != 0 {
            let n = MenuPage::ALL.len() as i32;
            let next = ((active_tab_index(&pages) as i32 + bump).rem_euclid(n)) as usize;
            system_nav.open_entry = None;
            tab_state.system_window_start = None;
            seed_cursor_for_tab(next, &mut cursor);
            tab_state.focus_zone = GridFocusZone::Body;
            pages.active = Some(tab_page(next));
            play_ui(&mut fx.sfx, grid_sfx::TAB_CHANGE);
            continue;
        }

        let active_page = active_page(&pages);
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
                let next = ((active_tab_index(&pages) as i32 + dx).rem_euclid(n)) as usize;
                quality_confirm.cancel();
                system_nav.open_entry = None;
                tab_state.system_window_start = None;
                seed_cursor_for_tab(next, &mut cursor);
                pages.active = Some(tab_page(next));
                play_ui(&mut fx.sfx, grid_sfx::TAB_CHANGE);
                continue;
            }
            if menu.select || dy > 0 {
                // Activate: focus is already the live tab; just drop into the body (the
                // cursor was seeded for this tab when we switched onto it).
                tab_state.focus_zone = GridFocusZone::Body;
                seed_cursor_for_tab(active_tab_index(&pages), &mut cursor);
                play_ui(&mut fx.sfx, grid_sfx::ACCEPT);
                continue;
            }
            if menu.back {
                tab_state.focus_zone = GridFocusZone::Body;
                play_ui(&mut fx.sfx, grid_sfx::BACK);
                continue;
            }
            // No other input does anything while the tab bar holds focus.
            continue;
        }

        // Fix 4: UP from the TOP body row moves focus onto the tab bar.
        if dy < 0 && cursor_on_top_row(active_page, cursor.focus(), system_nav.open_entry, crate::menu::model::InventoryLayout::for_region(regions.get(menu_entity).ok()).cols) {
            tab_state.focus_zone = GridFocusZone::Tabs;
            play_ui(&mut fx.sfx, grid_sfx::TAB_CHANGE);
            continue;
        }

        match active_page {
            MenuPage::Items => {
                if dx != 0 || dy != 0 {
                    let next = move_items_cursor(cursor.focus(), dx, dy, crate::menu::model::InventoryLayout::for_region(regions.get(menu_entity).ok()));
                    cursor.mark_keyboard(next);
                }
                if menu.back {
                    play_ui(&mut fx.sfx, grid_sfx::CLOSE);
                    quality_confirm.cancel();
                    overlay.close();
                    continue;
                }
                if menu.select {
                    // The ERROR bark stays here because it is NOT an activation:
                    // an empty cell has no action to publish, and the consumer
                    // could not tell "nothing there" from "nobody pressed".
                    let idx = cursor.focus().item_index();
                    match owned_item_action(
                        &ambition_platformer2d::items::Inventory::new(
                            &fx.owned,
                            fx.body_source
                                .read(menu_owner.owner)
                                .and_then(|body| body.in_hand),
                        ),
                        fx.body_source.items(),
                        idx,
                    ) {
                        Some(action) => {
                            activated.write(MenuActionActivated {
                                menu: menu_entity,
                                action,
                            });
                        }
                        None => play_ui(&mut fx.sfx, grid_sfx::ERROR),
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
                    menu_entity,
                    &menu,
                    dx,
                    dy,
                    &mut cursor,
                    &mut system_nav,
                    &mut pages,
                    &mut fx.settings,
                    &mut quality_confirm,
                    active_page,
                    &mut activated,
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
                    quality_confirm.cancel();
                    overlay.close();
                }
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
    open_entry: Option<SystemMenuEntryId>,
    cols: usize,
) -> bool {
    match page {
        MenuPage::Items => match focus {
            MenuFocus::Item(i) => i < cols,
            _ => true,
        },
        MenuPage::System => open_entry.is_none() && matches!(focus, MenuFocus::System(0)),
        MenuPage::Map | MenuPage::Quest => true,
    }
}

/// Move within the current grid dimensions. Non-item focus starts at slot zero.
fn move_items_cursor(focus: MenuFocus, dx: i32, dy: i32, layout: crate::menu::model::InventoryLayout) -> MenuFocus {
    let cols = layout.cols as i32;
    let rows = layout.rows() as i32;
    let idx = match focus {
        MenuFocus::Item(i) => i as i32,
        _ => 0,
    };
    let row = (idx / cols + dy).clamp(0, rows - 1);
    let col = (idx % cols + dx).clamp(0, cols - 1);
    MenuFocus::Item((row * cols + col) as usize)
}

/// Run while any menu is open or still has a Grid tree to remove.
fn a_menu_is_open_or_a_grid_tree_was(
    menus: Query<(
        &GridMenuTabState,
        &ambition_platformer2d::inventory_ui::InventoryUiState,
    )>,
) -> bool {
    menus
        .iter()
        .any(|(tab, overlay)| overlay.visible || tab.was_open)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn grid_menu_republish_view(
    regions: Query<&super::regions::MenuRegion>,
    mut menus: Query<(
        Entity,
        &ambition_platformer2d::inventory_ui::InventoryMenu,
        &ambition_platformer2d::inventory_ui::InventoryUiState,
        &ActiveMenuPages<MenuPage, MenuPageAction>,
        &KaleidoscopeCursor,
        &KaleidoscopeSystemNav,
        &VisualQualityConfirmState,
        &mut GridMenuTabState,
        &super::system_cache::CachedSystemMenu,
    )>,
    owned: Res<OwnedItems>,
    body_source: MenuBodySource,
    backend: Res<InventoryUiBackend>,
    map_page: crate::menu::map_page::MapPageSource,
    roots: Query<
        (Entity, &ambition_platformer2d::menu::MenuSurface),
        (With<BevyUiMenuRoot>, With<GridMenuRoot>),
    >,
    assets: Option<Res<AssetServer>>,
    menu_font: Option<Res<ambition_platformer2d::menu::render::bevy_ui::MenuFont>>,
    mut commands: Commands,
) {
    for (
        menu_entity,
        menu_owner,
        overlay,
        pages,
        cursor,
        system_nav,
        quality_confirm,
        mut tab_state,
        cache,
    ) in &mut menus
    {
        let has_root = roots.iter().any(|(_, owner)| owner.0 == menu_entity);
        let active = backend.effective() == InventoryUiBackend::Grid && overlay.visible;
        if !active {
            // Tear the tree down when not the active+open backend, and forget the key so
            // a reopen always rebuilds.
            if tab_state.was_open || has_root {
                for (e, _) in roots.iter().filter(|(_, owner)| owner.0 == menu_entity) {
                    commands.entity(e).despawn();
                }
                tab_state.was_open = false;
                tab_state.last_key = None;
            }
            continue;
        }
        tab_state.was_open = true;
        let body = body_source.read(menu_owner.owner);
        let inventory = ambition_platformer2d::items::Inventory::new(
            &owned,
            body.and_then(|body| body.in_hand),
        );

        // The selected tab uses the same System model as navigation and the Cube.
        let active_page = active_page(&pages);
        let Some(model) = cache.model.as_ref() else {
            continue;
        };
        // The EFFECTIVE System window start: an explicit wheel/drag override wins
        // (Features C/D), otherwise it follows the cursor — exactly the cube's rule via
        // the shared `system_effective_window_start`. Hovering moves the cursor but, with
        // an override set, does NOT shift the window, so hovering no longer scrolls.
        let window_start = if active_page == MenuPage::System {
            crate::menu::model::system_effective_window_start(
                &cache.rows,
                cursor.focus(),
                tab_state.system_window_start,
            )
        } else {
            0
        };
        let key = ViewKey {
            layout: crate::menu::model::InventoryLayout::for_region(regions.get(menu_entity).ok()),
            system_revision: cache.model_builds,
            tab: active_tab_index(&pages),
            open_entry: system_nav.open_entry,
            focus: cursor.focus(),
            version: pages.version,
            zone: tab_state.focus_zone,
            window_start,
            pending_quality: quality_confirm.pending(),
            room: if active_page == MenuPage::Map {
                map_page.room(body.map(|body| body.entity))
            } else {
                None
            },
            equipped: body.map(|body| body.equipped).unwrap_or_default(),
        };
        // The dispatch paths ALSO clear `last_key` directly (the belt-and-braces force-republish),
        // so even a state change this key can't see (e.g. a dev snapshot) still re-renders.
        let font = menu_font.as_deref().and_then(|font| font.0.as_ref());
        let state_changed = tab_state.rendered_font.as_ref() != font
            || owned.is_changed()
            || body_source.items_changed()
            || map_page.changed();
        if tab_state.last_key == Some(key) && has_root && !state_changed {
            continue;
        }
        tab_state.last_key = Some(key);
        tab_state.rendered_font = font.cloned();
        let built = build_inventory_pages_with_quality_prompt(
            &inventory,
            body_source.items(),
            key.equipped,
            cursor.focus(),
            &model,
            &cache.radio,
            window_start,
            system_nav.open_entry,
            quality_confirm.pending(),
            map_page.page(map_page.view(body.map(|body| body.entity)).as_ref()),
            key.layout,
        );
        let Some(page) = built
            .iter()
            .find(|p| p.id == active_page)
            .or_else(|| built.first())
        else {
            continue;
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

        // The page builders bake `< Prev` / `> Next` edge buttons for the cube; the flat tabbed
        // renderer replaces them with the tab bar, so they must NOT be drawn (they leaked in as "<
        // Items" / "> Quest" flashes, and keyboard/gamepad nav could land on them).
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

        // Despawn the previous tree, then respawn ON THE SAME command buffer so both land in the same
        // flush. Spawning directly with the system's own `Commands` closes that gap.)
        for (e, _) in roots.iter().filter(|(_, owner)| owner.0 == menu_entity) {
            commands.entity(e).despawn();
        }
        let tabs = tab_specs();
        // Fix 4: when focus is on the tab bar, tell the renderer which tab to ring; when
        // in the body, no tab is focused (only the active tab is highlighted).
        let focused_tab = match tab_state.focus_zone {
            GridFocusZone::Tabs => Some(active_tab_index(&pages)),
            GridFocusZone::Body => None,
        };
        let view = BevyUiMenuView {
            menu: menu_entity,
            tabs: &tabs,
            active_tab: active_tab_index(&pages),
            page: &page,
            focused,
            focused_tab,
        };
        // Fix 3: hand the `AssetServer` to the renderer so the Items tab shows its
        // sprite ICONS (the model's per-cell icon path), like the cube does.
        let root = ambition_platformer2d::menu::render::bevy_ui::spawn_bevy_ui_menu_with_font(
            &mut commands,
            &view,
            assets.as_deref(),
            font,
        );
        // Identity, not species: the teardown above must only claim the GRID's own
        // tree. Other `BevyUiMenuRoot` producers (the shell launcher/title screen)
        // coexist in the same App.
        commands.entity(root).insert(GridMenuRoot);
    }
}

/// Marker on the grid backend's own menu root, so its open/close teardown
/// never despawns another producer's `BevyUiMenuRoot` (the shell launcher).
#[derive(Component)]
pub(crate) struct GridMenuRoot;

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
    mut menus: Query<(
        Entity,
        &ambition_platformer2d::inventory_ui::InventoryUiState,
        &ActiveMenuPages<MenuPage, MenuPageAction>,
        &mut GridMenuTabState,
        &KaleidoscopeSystemNav,
        &VisualQualityConfirmState,
        &KaleidoscopeCursor,
    )>,
    settings: Res<UserSettings>,
    system: SystemMenuParams,
    mut wheel: super::regions::MenuWheelInput,
) {
    // Backend read from `system` (it owns the resource); a separate `Res` would
    // B0002-conflict with that `ResMut`.
    if system.backend() != InventoryUiBackend::Grid {
        wheel.clear();
        return;
    }
    wheel.read();
    for (menu, overlay, pages, mut tab_state, system_nav, quality_confirm, cursor) in &mut menus {
        if !overlay.visible {
            continue;
        }
        let steps = wheel.steps_for(menu);
        if steps == 0 {
            continue;
        }
        let active_page = active_page(&pages);
        let model = system.model(&settings, menu, quality_confirm.pending());
        let total =
            grid_system_row_count(active_page, &system_nav, &model, quality_confirm.pending());
        if total <= SYSTEM_VISIBLE_ROWS {
            continue; // nothing to scroll
        }
        let max = system_max_window_start(total) as i32;
        // Seed from the effective start so the first notch moves relative to what is
        // currently shown (cursor-derived window) rather than jumping to 0.
        let rows = system_rows_with_quality_prompt(
            &model,
            system_nav.open_entry,
            quality_confirm.pending(),
        );
        let current = crate::menu::model::system_effective_window_start(
            &rows,
            cursor.focus(),
            tab_state.system_window_start,
        ) as i32;
        let next = (current + steps).clamp(0, max) as usize;
        tab_state.system_window_start = Some(next);
    }
}

/// Feature C (Grid): apply the engine's backend-agnostic scrollbar-drag signal
/// (`ambition_platformer2d::menu::MenuScrollDragged`, emitted by the
/// `bevy_ui` scrollbar observers) to the Grid's scroll override — the mirror of the
/// cube's `kaleidoscope_apply_scroll_drag`. The neutral `0..=1` fraction maps across
/// the scrollable range to a window-start row. Selection-independent, like the wheel.
#[cfg(feature = "input")]
pub(crate) fn grid_menu_apply_scroll_drag(
    mut menus: Query<(
        &ambition_platformer2d::inventory_ui::InventoryUiState,
        &ActiveMenuPages<MenuPage, MenuPageAction>,
        &mut GridMenuTabState,
        &KaleidoscopeSystemNav,
        &VisualQualityConfirmState,
    )>,
    settings: Res<UserSettings>,
    system: SystemMenuParams,
    mut dragged: MessageReader<ambition_platformer2d::menu::MenuScrollDragged>,
) {
    if system.backend() != InventoryUiBackend::Grid {
        dragged.clear();
        return;
    }
    for drag in dragged.read() {
        let Ok((overlay, pages, mut tab_state, system_nav, quality_confirm)) =
            menus.get_mut(drag.menu)
        else {
            continue;
        };
        if !overlay.visible {
            continue;
        }
        let model = system.model(&settings, drag.menu, quality_confirm.pending());
        let total = grid_system_row_count(
            active_page(pages),
            system_nav,
            &model,
            quality_confirm.pending(),
        );
        if let Some(start) = scroll_fraction_to_window_start(total, drag.fraction.clamp(0.0, 1.0)) {
            tab_state.system_window_start = Some(start);
        }
    }
}

/// Switch flat-menu tabs from the renderer-neutral tab activation message.
#[cfg(feature = "input")]
pub(crate) fn grid_menu_tab_activated(
    mut activated: MessageReader<MenuTabActivated>,
    backend: Res<InventoryUiBackend>,
    mut menus: Query<(
        Entity,
        &ambition_platformer2d::inventory_ui::InventoryMenu,
        &mut ActiveMenuPages<MenuPage, MenuPageAction>,
        &mut GridMenuTabState,
        &mut KaleidoscopeCursor,
        &mut KaleidoscopeSystemNav,
        &mut VisualQualityConfirmState,
        &ambition_platformer2d::inventory_ui::InventoryUiState,
    )>,
    mut sfx: SfxWriter,
) {
    if backend.effective() != InventoryUiBackend::Grid {
        activated.clear();
        return;
    }
    for activation in activated.read() {
        let Ok((
            _menu_entity,
            _menu_owner,
            mut pages,
            mut tab_state,
            mut cursor,
            mut system_nav,
            mut quality_confirm,
            overlay,
        )) = menus.get_mut(activation.menu)
        else {
            continue;
        };
        if !overlay.visible {
            continue;
        }
        let next = activation.index.min(MenuPage::ALL.len() - 1);
        quality_confirm.cancel();
        system_nav.open_entry = None;
        tab_state.system_window_start = None;
        seed_cursor_for_tab(next, &mut cursor);
        // Pointer/touch tab activation lands focus in the body. Only explicit
        // keyboard navigation parks focus on the tab bar.
        tab_state.focus_zone = GridFocusZone::Body;
        pages.active = Some(tab_page(next));
        tab_state.last_key = None;
        play_ui(&mut sfx, grid_sfx::TAB_CHANGE);
    }
}

/// Install the flat Bevy-UI/Grid backend systems. Registered independently from
/// the cube backend so builds can omit this presentation without installing its
/// Bevy-UI tree, picking observers, or scroll systems.
pub fn install_grid_unified_menu(app: &mut App) {
    // registered HERE, beside the system that publishes it, not only in the
    // `install_bevy_ui_menu_actions` block below. `grid_menu_nav` now writes this
    // message, so a composition that installs nav without the pointer bridge
    // (every `grid_app()` test fixture) would panic on an uninitialised
    // `MessageWriter` — and the fix for that panic is NOT to wrap the parameter in
    // `Option`, which answers "may this be absent" when the real question is
    // "who owns registering it". `add_message` is a no-op when already present.
    #[cfg(feature = "input")]
    app.add_message::<MenuActionActivated<MenuPageAction>>();
    #[cfg(feature = "input")]
    app.add_systems(
        Update,
        (
            grid_menu_nav
                .run_if(grid_backend_active)
                // Join the shared menu-nav consume set so the touch-joystick
                // fold (mobile_input) can pin `.before(MenuNavConsume)` and
                // land its directional intent before this reads the frame.
                .in_set(ambition_platformer2d::input::MenuNavConsume)
                // ORDERED against the cube backend's nav; see `menu::GridMenuNav`.
                .in_set(crate::menu::GridMenuNav),
        )
            .chain()
            // CONDITIONAL on the host, like the preset-map sync in
            // `app/plugins.rs`. `CoreSimulation` lives in `app.sim_schedule()`,
            // which is this literal `Update` only under `RenderFrame`; the shipped
            // `dev_tools` build is `Ggrs`, where measurement puts 0 systems in
            // `Update`'s CoreSimulation node and 242 in `GgrsSchedule`. Kept
            // because `RenderFrame` is the default host — but do NOT read this
            // pin as proof that menu nav consumes the frame before the sim does
            // under rollback: there the sim has already run, in `PreUpdate`.
            // What actually keeps the menu and the sim from fighting over a press
            // is the `simulation_authorized` + `in_base_mode` gating above, not
            // this edge. See `tests/sim_phase_pins.rs`.
            .before(ambition_platformer2d::platformer::schedule::Platformer2dSimulationPhase::CoreSimulation),
    );
    app.add_systems(
        Update,
        grid_menu_republish_view
            .after(ambition_platformer2d::platformer::schedule::Platformer2dSimulationPhase::CoreSimulation)
            // Closed and torn down, it has nothing to build or take down.
            .run_if(a_menu_is_open_or_a_grid_tree_was)
            .after(super::system_cache::cache_system_menu),
    );
    // ⛔ NOTHING IS REGISTERED HERE ANY MORE, and the ordering edge that used to
    // be went with it. Carrying the active page across a backend switch needed a
    // system only while the two backends kept that page in two different fields;
    // they now read one, so the carry is an identity and the `MenuNavConsume`
    // edge it needed for same-frame settling is moot.
    // Features C/D: the wheel + scrollbar-drag scroll appliers run BEFORE republish so
    // a scroll set this frame rebuilds the windowed rows the same frame. The drag
    // signal comes from the engine's `bevy_ui` scrollbar observers
    // (`install_bevy_ui_menu_scroll`), which also registers the `MenuScrollDragged`
    // message the applier reads.
    #[cfg(feature = "input")]
    {
        install_bevy_ui_menu_actions::<MenuPageAction>(app);
        install_bevy_ui_menu_tabs(app);
        ambition_platformer2d::menu::render::bevy_ui::install_bevy_ui_menu_scroll(app);
        app.add_systems(
            Update,
            (
                grid_menu_tab_activated,
                grid_menu_scroll_wheel,
                grid_menu_apply_scroll_drag,
            )
                .run_if(grid_backend_active)
                .after(BevyUiMenuInteractionSet)
                // AND after nav, which now publishes activations too. Without
                // this the two sets are unordered siblings in `Update`, so a
                // controller submit would be dispatched a frame late — breaking the
                // `InputSet` contract that an edge produced this frame is consumed
                // this frame.
                .after(ambition_platformer2d::input::MenuNavConsume)
                .before(super::system_cache::cache_system_menu)
                .before(grid_menu_republish_view),
        );
    }
}

#[cfg(all(test, feature = "input"))]
mod tests;

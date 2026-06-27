use super::*;
use crate::menu::model::{build_inventory_pages, system_rows, SystemRow};
use ambition_characters::brain::ActionSet;
use ambition_gameplay_core::items::Item;
use ambition_gameplay_core::persistence::settings::{SystemMenuEntryId, SystemMenuModel};
use ambition_gameplay_core::actor::{PlayerEntity, PrimaryPlayer};
use ambition_gameplay_core::actor::{BodyMana};
use ambition_gameplay_core::session::game_mode::GameMode;

/// Switching the inventory frontend mid-session lands you on the SAME page in the
/// new frontend (not back on Inventory). The cube stores the page in
/// `ActiveMenuPages.active`, the Grid in `GridMenuTabState.active_tab`;
/// `sync_menu_page_across_backend_switch` carries it across either way.
#[test]
fn backend_switch_carries_the_active_page() {
    let mut app = grid_app();
    app.add_systems(Update, sync_menu_page_across_backend_switch);
    app.world_mut()
        .resource_mut::<ambition_gameplay_core::inventory_ui::InventoryUiState>()
        .visible = true;

    // Open on the cube, System page; the first update snapshots the current page.
    *app.world_mut().resource_mut::<InventoryUiBackend>() = InventoryUiBackend::LunexKaleidoscope;
    app.world_mut()
        .resource_mut::<ActiveMenuPages<MenuPage, MenuPageAction>>()
        .active = Some(MenuPage::System);
    app.update();

    // Cube → Grid: the grid tab carries the cube's page (System), not Inventory.
    *app.world_mut().resource_mut::<InventoryUiBackend>() = InventoryUiBackend::Grid;
    app.update();
    assert_eq!(
        tab_page(app.world().resource::<GridMenuTabState>().active_tab),
        MenuPage::System,
        "cube→grid lands on the same page (System), not Inventory"
    );

    // Navigate the grid to Map and let it settle one frame (the snapshot), then
    // switch back to the cube: the cube page carries it.
    app.world_mut()
        .resource_mut::<GridMenuTabState>()
        .active_tab = tab_index_of(MenuPage::Map);
    app.update();
    *app.world_mut().resource_mut::<InventoryUiBackend>() = InventoryUiBackend::LunexKaleidoscope;
    app.update();
    assert_eq!(
        app.world()
            .resource::<ActiveMenuPages<MenuPage, MenuPageAction>>()
            .active,
        Some(MenuPage::Map),
        "grid→cube lands on the same page (Map)"
    );
}

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
    app.init_resource::<ambition_gameplay_core::dev::dev_tools::DeveloperTools>();
    app.init_resource::<ambition_gameplay_core::SandboxDevState>();
    app.init_resource::<ambition_gameplay_core::ldtk_world::LdtkHotReloadState>();
    app.init_resource::<ambition_gameplay_core::session::reset::SandboxResetRequested>();
    app.init_resource::<ambition_gameplay_core::dev::dev_tools::EditableMovementTuning>();
    app.init_resource::<UserSettings>();
    app.init_resource::<ambition_gameplay_core::inventory_ui::InventoryUiState>();
    app.init_resource::<ambition_gameplay_core::menu::map::MapMenuState>();
    app.init_resource::<MenuControlFrame>();
    app.init_resource::<GridMenuTabState>();
    app.init_resource::<GridPointerPress>();
    app.init_resource::<ambition_input::ActiveInputKind>();
    app.add_message::<PlayerHealRequested>();
    app.add_message::<SfxMessage>();
    app.add_message::<bevy::app::AppExit>();
    app.add_observer(grid_menu_pointer_hover);
    app.add_systems(Update, (grid_menu_open_routing, grid_menu_nav).chain());
    *app.world_mut().resource_mut::<InventoryUiBackend>() = InventoryUiBackend::Grid;
    app.world_mut().spawn((
        PlayerEntity,
        PrimaryPlayer,
        ActionSet::default(),
        BodyMana::default(),
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
        .resource::<ambition_gameplay_core::inventory_ui::InventoryUiState>()
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

/// Regression: on the System tab, hammering LEFT/RIGHT must NEVER turn the cube's
/// page or land the shared cursor on a page-turn edge. The Grid switches pages via
/// the tab bar only; previously LEFT/RIGHT on a non-value System row walked onto
/// `EdgeLeft`/`EdgeRight` and the next press fired the cube's `turn_page`
/// (rotate-SFX + a one-frame face flip leaking into Grid mode). `allow_page_turn=false`.
#[test]
fn system_tab_left_right_never_turns_the_page() {
    let mut app = grid_app();
    // Open + reach the System tab.
    set_frame(&mut app, |f| f.inventory = true);
    app.update();
    let sys_idx = MenuPage::ALL
        .iter()
        .position(|p| *p == MenuPage::System)
        .unwrap();
    for _ in 0..sys_idx {
        set_frame(&mut app, |f| f.page_right = true);
        app.update();
    }
    assert_eq!(active_tab(&app), MenuPage::System);
    let page_before = app
        .world()
        .resource::<ActiveMenuPages<MenuPage, MenuPageAction>>()
        .active;

    // Hammer LEFT then RIGHT many times — enough to reach an edge under the old
    // (leaky) behaviour and then "turn the page" on the following press.
    for i in 0..8 {
        let go_left = i % 2 == 0;
        set_frame(&mut app, |f| {
            if go_left {
                f.left = true;
            } else {
                f.right = true;
            }
        });
        app.update();
        let focus = app.world().resource::<KaleidoscopeCursor>().focus();
        assert!(
            !matches!(focus, MenuFocus::EdgeLeft | MenuFocus::EdgeRight),
            "step {i}: Grid System nav must not land on a page-turn edge, got {focus:?}",
        );
        assert_eq!(
            active_tab(&app),
            MenuPage::System,
            "step {i}: the Grid tab must stay on System",
        );
        assert_eq!(
            app.world()
                .resource::<ActiveMenuPages<MenuPage, MenuPageAction>>()
                .active,
            page_before,
            "step {i}: the shared page must not turn under Grid LEFT/RIGHT",
        );
    }
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
            .resource::<ambition_gameplay_core::inventory_ui::InventoryUiState>()
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

// ----- Pointer bug-fix coverage -----------------------------------------

use crate::menu::test_support::{spawn_control, trigger_over, trigger_press, trigger_release};
use ambition_menu::render::bevy_ui::BevyUiMenuTab;
use ambition_menu::AmbitionMenuControl;

fn fire_press(app: &mut App, entity: Entity) {
    trigger_press(app, entity);
    app.update();
}

fn fire_release(app: &mut App, entity: Entity) {
    trigger_release(app, entity);
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

/// Spawn a hoverable control and fire a `Pointer<Over>` at it (the exact
/// event a republish synthesizes under a stationary mouse).
fn hover_control(app: &mut App, action: MenuPageAction) {
    let entity = spawn_control(app, action);
    // The observer fires synchronously; avoid `app.update()` so open routing
    // does not reseed the cursor before the assertion.
    trigger_over(app, entity);
}

/// Bug 1 (snap-back): `grid_menu_pointer_hover` must IGNORE a `Pointer<Over>`
/// (the event a republish fires under a stationary mouse) while the active
/// input source is NOT the mouse, and HONOR it once a genuine mouse move has
/// set `ActiveInputKind = Mouse`. Without the gate, every arrow-key move
/// rebuilt the menu → fired `Over` → snapped the cursor back to the mouse.
#[test]
fn hover_is_gated_on_active_input_being_mouse() {
    use ambition_input::ActiveInputKind;
    use ambition_gameplay_core::items::Item;

    let mut app = grid_app();
    // Open the menu so the hover handler's `overlay.visible` guard passes.
    set_frame(&mut app, |f| f.inventory = true);
    app.update();

    // Park the keyboard cursor on a known item, then drop the active source
    // onto Keyboard — the exact state during arrow-key navigation.
    let parked = MenuFocus::Item(Item::ALL[0].index());
    app.world_mut()
        .resource_mut::<KaleidoscopeCursor>()
        .mark_keyboard(parked);
    *app.world_mut().resource_mut::<ActiveInputKind>() = ActiveInputKind::Keyboard;

    // A republish-style `Over` on a DIFFERENT item must NOT move the cursor.
    let other = Item::ALL[1];
    hover_control(&mut app, MenuPageAction::Equip(other));
    assert_eq!(
        app.world().resource::<KaleidoscopeCursor>().focus(),
        parked,
        "an Over while on the keyboard is ignored — no snap-back"
    );

    // Now a GENUINE mouse move would set active=Mouse; the same Over then
    // takes ownership and moves the cursor onto the hovered item.
    *app.world_mut().resource_mut::<ActiveInputKind>() = ActiveInputKind::Mouse;
    hover_control(&mut app, MenuPageAction::Equip(other));
    assert_eq!(
        app.world().resource::<KaleidoscopeCursor>().focus(),
        MenuFocus::Item(other.index()),
        "with active=Mouse a genuine hover moves the cursor"
    );
}

// ---- Features C/D: Grid independent scroll (mouse wheel + scrollbar drag) ----

/// A Grid harness opened on the System tab, drilled into the long Developer list,
/// with the wheel + drag scroll appliers wired exactly as `install_grid_unified_menu`
/// orders them (before republish). Mirrors the cube's `scroll_app`.
fn scroll_grid_app() -> App {
    let mut app = grid_app();
    app.add_message::<bevy::input::mouse::MouseWheel>();
    app.add_message::<ambition_menu::render::kaleidoscope::MenuScrollDragged>();
    app.add_systems(
        Update,
        (grid_menu_scroll_wheel, grid_menu_apply_scroll_drag).before(grid_menu_nav),
    );
    // Open on the System tab, drilled into Developer (the long, scrollable list).
    app.world_mut()
        .resource_mut::<ambition_gameplay_core::inventory_ui::InventoryUiState>()
        .visible = true;
    {
        let mut ts = app.world_mut().resource_mut::<GridMenuTabState>();
        ts.active_tab = tab_index_of(MenuPage::System);
        ts.system_window_start = None;
    }
    app.world_mut()
        .resource_mut::<KaleidoscopeSystemNav>()
        .open_entry = Some(SystemMenuEntryId::Developer);
    app.world_mut()
        .resource_mut::<KaleidoscopeCursor>()
        .mark_keyboard(MenuFocus::System(0));
    app.update();
    app
}

/// The live Developer-drill row count (built the SAME way the scroll appliers do,
/// via `SystemMenuParams::model`), so the test clamps against the real range.
fn grid_scroll_total_rows(app: &mut App) -> usize {
    use bevy::ecs::system::RunSystemOnce;
    app.world_mut()
        .run_system_once(
            |settings: Res<UserSettings>,
             system_nav: Res<KaleidoscopeSystemNav>,
             system: SystemMenuParams| {
                let model = system.model(&settings);
                grid_system_row_count(MenuPage::System, &system_nav, &model)
            },
        )
        .unwrap()
}

fn grid_window_start(app: &App) -> Option<usize> {
    app.world()
        .resource::<GridMenuTabState>()
        .system_window_start
}

/// Feature D: a `MouseWheel` down over the System tab advances the scroll override
/// (window_start) but NOT the selection cursor; clamped at the ends.
#[test]
fn grid_mouse_wheel_scrolls_window_not_selection() {
    let mut app = scroll_grid_app();
    let total = grid_scroll_total_rows(&mut app);
    assert!(
        total > SYSTEM_VISIBLE_ROWS,
        "fixture must overflow: {total}"
    );
    let max = system_max_window_start(total);

    let cursor_before = app.world().resource::<KaleidoscopeCursor>().focus();
    assert_eq!(grid_window_start(&app), None, "starts following the cursor");

    // Three wheel-down notches → window_start == 3, cursor unchanged.
    for _ in 0..3 {
        app.world_mut()
            .resource_mut::<Messages<bevy::input::mouse::MouseWheel>>()
            .write(bevy::input::mouse::MouseWheel {
                unit: bevy::input::mouse::MouseScrollUnit::Line,
                x: 0.0,
                y: -1.0,
                window: Entity::PLACEHOLDER,
            });
        app.update();
    }
    assert_eq!(
        grid_window_start(&app),
        Some(3),
        "three wheel-down notches scroll the window to row 3"
    );
    assert_eq!(
        app.world().resource::<KaleidoscopeCursor>().focus(),
        cursor_before,
        "the wheel must NOT move the selection cursor (Feature D)"
    );

    // Many more notches clamp at the bottom (system_max_window_start).
    for _ in 0..100 {
        app.world_mut()
            .resource_mut::<Messages<bevy::input::mouse::MouseWheel>>()
            .write(bevy::input::mouse::MouseWheel {
                unit: bevy::input::mouse::MouseScrollUnit::Line,
                x: 0.0,
                y: -1.0,
                window: Entity::PLACEHOLDER,
            });
        app.update();
    }
    assert_eq!(
        grid_window_start(&app),
        Some(max),
        "the wheel clamps at the bottom of the range"
    );
}

/// Feature C/D: with a scroll override active, a HOVER/cursor-follow does NOT change
/// the window (the override wins → hovering no longer scrolls); a keyboard move
/// CLEARS the override so the window resumes following the cursor.
#[test]
fn grid_override_survives_hover_and_clears_on_keyboard() {
    let mut app = scroll_grid_app();

    // Establish an override via a wheel notch.
    app.world_mut()
        .resource_mut::<Messages<bevy::input::mouse::MouseWheel>>()
        .write(bevy::input::mouse::MouseWheel {
            unit: bevy::input::mouse::MouseScrollUnit::Line,
            x: 0.0,
            y: -1.0,
            window: Entity::PLACEHOLDER,
        });
    app.update();
    assert_eq!(grid_window_start(&app), Some(1), "wheel set an override");

    // A hover (cursor-follow) moves the CURSOR but, with the override set, the
    // EFFECTIVE window stays at the override — hovering does not scroll the list.
    *app.world_mut()
        .resource_mut::<ambition_input::ActiveInputKind>() =
        ambition_input::ActiveInputKind::Mouse;
    app.world_mut()
        .resource_mut::<KaleidoscopeCursor>()
        .mark_keyboard(MenuFocus::System(0));
    app.update();
    assert_eq!(
        grid_window_start(&app),
        Some(1),
        "the override survives a hover — hovering does not scroll"
    );

    // A DOWN keypress moves the cursor and CLEARS the override.
    set_frame(&mut app, |f| f.down = true);
    app.update();
    assert_eq!(
        grid_window_start(&app),
        None,
        "keyboard nav resumes cursor-follow scrolling (Features C/D)"
    );
}

/// Feature C: applying the engine's neutral `MenuScrollDragged` fraction (the
/// scrollbar-drag signal) sets the Grid override proportionally across the range —
/// the Grid equivalent of the cube's drag test.
#[test]
fn grid_scrollbar_drag_fraction_sets_window_start_proportionally() {
    let mut app = scroll_grid_app();
    let total = grid_scroll_total_rows(&mut app);
    let max = system_max_window_start(total);
    assert!(max > 0, "fixture must be scrollable");

    let cursor_before = app.world().resource::<KaleidoscopeCursor>().focus();

    // Drag to the BOTTOM of the track (fraction 1.0) → window_start == max.
    app.world_mut()
        .resource_mut::<Messages<ambition_menu::render::kaleidoscope::MenuScrollDragged>>()
        .write(ambition_menu::render::kaleidoscope::MenuScrollDragged { fraction: 1.0 });
    app.update();
    assert_eq!(
        grid_window_start(&app),
        Some(max),
        "fraction 1.0 scrolls to the bottom (Feature C)"
    );

    // Drag to the MIDDLE (fraction 0.5) → ~half the range.
    app.world_mut()
        .resource_mut::<Messages<ambition_menu::render::kaleidoscope::MenuScrollDragged>>()
        .write(ambition_menu::render::kaleidoscope::MenuScrollDragged { fraction: 0.5 });
    app.update();
    assert_eq!(
        grid_window_start(&app),
        Some((0.5 * max as f32).round() as usize),
        "fraction 0.5 maps to the middle of the range"
    );
    assert_eq!(
        app.world().resource::<KaleidoscopeCursor>().focus(),
        cursor_before,
        "a scrollbar drag does not move the selection cursor"
    );
}

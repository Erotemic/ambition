//! Behaviour tests for the cube's interaction seams, driven through the real
//! systems/observers as the app wires them.
use super::*;
use crate::menu::model::{build_inventory_pages, system_rows};
use crate::menu::test_support::{
    click_control, pointer_location, spawn_control, trigger_move, trigger_press, trigger_release,
};
use ambition::actors::actor::BodyMana;
use ambition::actors::actor::{PlayerEntity, PrimaryPlayer};
use ambition::characters::brain::ActionSet;
use ambition::platformer::schedule::GameMode;

/// The cube's System list wraps vertically (closed list); the Grid clamps (its
/// rows sit below the tab bar, a real UP target). Pins `step_system_row` so a
/// future edit can't silently revert the cube wrap or leak it into the Grid.
#[test]
fn system_row_wrap_is_cube_only() {
    // count = 4 rows (indices 0..=3).
    // Cube (wrap): UP off the top → bottom, DOWN off the bottom → top.
    assert_eq!(step_system_row(0, -1, 4, true), 3, "cube UP off top wraps");
    assert_eq!(
        step_system_row(3, 1, 4, true),
        0,
        "cube DOWN off bottom wraps"
    );
    // Cube interior moves are unchanged.
    assert_eq!(step_system_row(1, 1, 4, true), 2);
    assert_eq!(step_system_row(2, -1, 4, true), 1);
    // Grid (clamp): the ends hold.
    assert_eq!(
        step_system_row(0, -1, 4, false),
        0,
        "grid UP off top clamps"
    );
    assert_eq!(
        step_system_row(3, 1, 4, false),
        3,
        "grid DOWN off bottom clamps"
    );
    // Single-row list: wrap is a no-op, never a divide-by-zero.
    assert_eq!(step_system_row(0, -1, 1, true), 0);
}
use bevy::camera::NormalizedRenderTarget;
use bevy::picking::backend::HitData;
use bevy::picking::events::{Move, Pointer, Press, Release};
use bevy::picking::pointer::{Location, PointerId};

fn base_kaleidoscope_test_app() -> App {
    let mut app = App::new();
    app.init_resource::<VisualQualityConfirmState>();
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.init_state::<GameMode>();
    app.init_resource::<InventoryUiBackend>();
    app.init_resource::<ActiveMenuPages<MenuPage, MenuPageAction>>();
    app.init_resource::<KaleidoscopeCursor>();
    app.init_resource::<ambition::input::ActiveInputKind>();
    app.init_resource::<KaleidoscopeSystemNav>();
    app.init_resource::<KaleidoscopeScroll>();
    app.init_resource::<CachedSystemMenu>();
    app.init_resource::<KaleidoscopePointerPress>();
    app.init_resource::<OwnedItems>();
    app.init_resource::<ambition::dev_tools::dev_tools::DeveloperTools>();
    app.init_resource::<ambition::dev_tools::SandboxDevState>();
    app.init_resource::<ambition::actors::ldtk_world::LdtkHotReloadState>();
    app.init_resource::<ambition::actors::session::reset::SandboxResetRequested>();
    app.init_resource::<ambition::dev_tools::dev_tools::EditableMovementTuning>();
    app.init_resource::<UserSettings>();
    app.init_resource::<ambition::inventory_ui::InventoryUiState>();
    app.init_resource::<ambition::menu::map::MapMenuState>();
    app.init_resource::<MenuControlFrame>();
    app.add_message::<PlayerHealRequested>();
    app.add_message::<ambition::sfx::OwnedSfxMessage>();
    *app.world_mut().resource_mut::<InventoryUiBackend>() = InventoryUiBackend::LunexKaleidoscope;
    app
}

fn set_kaleidoscope_visible(app: &mut App, visible: bool) {
    app.world_mut()
        .resource_mut::<ambition::inventory_ui::InventoryUiState>()
        .visible = visible;
}

fn spawn_kaleidoscope_test_player(app: &mut App) -> Entity {
    app.world_mut()
        .spawn((
            PlayerEntity,
            PrimaryPlayer,
            ActionSet::default(),
            BodyMana::default(),
        ))
        .id()
}

// ---- Developer-resource toggles ------------------------------------------

/// Dispatching the resource-backed Developer rows flips the right resource:
/// `DebugOverlay` → `SandboxDevState::debug`, `SlowMotion` →
/// `SandboxDevState::slowmo`, `LdtkAutoApply` → `LdtkHotReloadState::auto_apply`
/// — none of which live on `DeveloperTools`. Driven through the real
/// `apply_dev_toggle` path so the cube and pause menu can't drift.
#[test]
fn extra_dev_toggles_flip_their_non_developer_resources() {
    let mut dev = ambition::dev_tools::dev_tools::DeveloperTools::default();
    let mut dev_state = ambition::dev_tools::SandboxDevState::default();
    let mut ldtk_reload = ambition::actors::ldtk_world::LdtkHotReloadState::default();
    let mut backend = InventoryUiBackend::default();

    let debug_before = dev_state.debug;
    let slowmo_before = dev_state.slowmo;
    let auto_before = ldtk_reload.auto_apply;

    for id in [
        DevToggleId::DebugOverlay,
        DevToggleId::SlowMotion,
        DevToggleId::LdtkAutoApply,
    ] {
        apply_dev_toggle(
            DevToggleWrite {
                dev: &mut dev,
                dev_state: &mut dev_state,
                ldtk_reload: &mut ldtk_reload,
                backend: &mut backend,
                #[cfg(feature = "portal_render")]
                portal_effect: None,
                #[cfg(feature = "portal_render")]
                portal_camera: None,
                base_gravity: None,
            },
            id,
            0,
        );
    }

    assert_eq!(
        dev_state.debug, !debug_before,
        "the debug-overlay row flips SandboxDevState.debug"
    );
    assert_eq!(
        dev_state.slowmo, !slowmo_before,
        "the slow-motion row flips SandboxDevState.slowmo"
    );
    assert_eq!(
        ldtk_reload.auto_apply, !auto_before,
        "the LDtk row flips LdtkHotReloadState.auto_apply"
    );
    // The snapshot mirrors the live state for all three (no field drift).
    let snap = dev_snapshot(DevToggleRead {
        dev: &dev,
        dev_state: &dev_state,
        ldtk_reload: &ldtk_reload,
        backend: InventoryUiBackend::default(),
        #[cfg(feature = "portal_render")]
        portal_effect: None,
        #[cfg(feature = "portal_render")]
        portal_camera: None,
        base_gravity: None,
    });
    let read = |id: DevToggleId| snap.values.iter().find(|(d, _, _)| *d == id).unwrap().1;
    assert_eq!(read(DevToggleId::DebugOverlay), dev_state.debug);
    assert_eq!(read(DevToggleId::SlowMotion), dev_state.slowmo);
    assert_eq!(read(DevToggleId::LdtkAutoApply), ldtk_reload.auto_apply);
}

/// The Developer "Menu Backend" row exists and dispatching it cycles
/// `InventoryUiBackend` (Grid ↔ Cube) — the in-menu equivalent of the `\`
/// hotkey, wired through the shared `apply_dev_toggle` path so BOTH backends can
/// trigger it. Its snapshot value label reflects the active frontend.
#[test]
fn menu_backend_dev_row_cycles_inventory_backend() {
    // The row is part of the Developer vocabulary and is a cycle (shows a label).
    assert!(DevToggleId::ALL.contains(&DevToggleId::MenuBackend));
    assert!(DevToggleId::MenuBackend.is_cycle());

    let mut dev = ambition::dev_tools::dev_tools::DeveloperTools::default();
    let mut dev_state = ambition::dev_tools::SandboxDevState::default();
    let mut ldtk_reload = ambition::actors::ldtk_world::LdtkHotReloadState::default();
    let mut backend = InventoryUiBackend::Grid;

    // The snapshot label reflects the live backend.
    let label = |b: InventoryUiBackend| {
        dev_snapshot(DevToggleRead {
            dev: &dev,
            dev_state: &dev_state,
            ldtk_reload: &ldtk_reload,
            backend: b,
            #[cfg(feature = "portal_render")]
            portal_effect: None,
            #[cfg(feature = "portal_render")]
            portal_camera: None,
            base_gravity: None,
        })
        .values
        .iter()
        .find(|(d, _, _)| *d == DevToggleId::MenuBackend)
        .unwrap()
        .2
        .clone()
    };
    assert_eq!(label(InventoryUiBackend::Grid), "Grid");
    assert_eq!(label(InventoryUiBackend::LunexKaleidoscope), "Cube");

    // Dispatching the row flips the backend Grid → Cube …
    apply_dev_toggle(
        DevToggleWrite {
            dev: &mut dev,
            dev_state: &mut dev_state,
            ldtk_reload: &mut ldtk_reload,
            backend: &mut backend,
            #[cfg(feature = "portal_render")]
            portal_effect: None,
            #[cfg(feature = "portal_render")]
            portal_camera: None,
            base_gravity: None,
        },
        DevToggleId::MenuBackend,
        0,
    );
    assert_eq!(backend, InventoryUiBackend::LunexKaleidoscope);
    // … and back Cube → Grid.
    apply_dev_toggle(
        DevToggleWrite {
            dev: &mut dev,
            dev_state: &mut dev_state,
            ldtk_reload: &mut ldtk_reload,
            backend: &mut backend,
            #[cfg(feature = "portal_render")]
            portal_effect: None,
            #[cfg(feature = "portal_render")]
            portal_camera: None,
            base_gravity: None,
        },
        DevToggleId::MenuBackend,
        0,
    );
    assert_eq!(backend, InventoryUiBackend::Grid);
}

/// ShowHitboxes from the System menu toggles the SAME field(s) the pause menu
/// does: BOTH `show_feature_hitboxes` and `show_player_hitbox`, and the
/// snapshot reads `show_feature_hitboxes` (matching the pause menu's source).
#[test]
fn show_hitboxes_toggles_feature_and_player_fields_like_pause() {
    let mut dev = ambition::dev_tools::dev_tools::DeveloperTools::default();
    let mut dev_state = ambition::dev_tools::SandboxDevState::default();
    let mut ldtk_reload = ambition::actors::ldtk_world::LdtkHotReloadState::default();
    dev.show_feature_hitboxes = false;
    dev.show_player_hitbox = false;
    let mut backend = InventoryUiBackend::default();

    apply_dev_toggle(
        DevToggleWrite {
            dev: &mut dev,
            dev_state: &mut dev_state,
            ldtk_reload: &mut ldtk_reload,
            backend: &mut backend,
            #[cfg(feature = "portal_render")]
            portal_effect: None,
            #[cfg(feature = "portal_render")]
            portal_camera: None,
            base_gravity: None,
        },
        DevToggleId::ShowHitboxes,
        0,
    );
    assert!(dev.show_feature_hitboxes, "feature hitboxes flip on");
    assert!(
        dev.show_player_hitbox,
        "player hitbox flips together (pause-menu parity)"
    );

    let snap = dev_snapshot(DevToggleRead {
        dev: &dev,
        dev_state: &dev_state,
        ldtk_reload: &ldtk_reload,
        backend: InventoryUiBackend::default(),
        #[cfg(feature = "portal_render")]
        portal_effect: None,
        #[cfg(feature = "portal_render")]
        portal_camera: None,
        base_gravity: None,
    });
    let on = snap
        .values
        .iter()
        .find(|(d, _, _)| *d == DevToggleId::ShowHitboxes)
        .unwrap()
        .1;
    assert!(on, "snapshot reads show_feature_hitboxes, now ON");
}

// ---- Fix 1: back-edge seeding --------------------------------------------

#[test]
fn back_edge_lands_opposite_the_direction_travelled() {
    // Turning RIGHT brings the viewer-right page to front; to go BACK you turn
    // left, so the cursor lands on the LEFT edge button — and vice-versa.
    let from = MenuPage::Items;
    let right = from.on_viewer_right();
    assert_eq!(back_edge_focus(Some(from), right), MenuFocus::EdgeLeft);
    let left = from.on_viewer_left();
    assert_eq!(back_edge_focus(Some(from), left), MenuFocus::EdgeRight);
    // First open (no prior page) defaults to a highlighted left edge button.
    assert_eq!(back_edge_focus(None, MenuPage::Map), MenuFocus::EdgeLeft);
}

// ---- Fix 4: System-page pointer clicks -----------------------------------

fn click_app() -> (App, Entity) {
    let mut app = base_kaleidoscope_test_app();
    // Feature E: the tap/drag-cancel guard needs the press + move observers in
    // addition to the release-dispatch observer.
    app.add_observer(kaleidoscope_pointer_press);
    app.add_observer(kaleidoscope_pointer_move);
    app.add_observer(kaleidoscope_pointer_release);
    set_kaleidoscope_visible(&mut app, true);
    let player = spawn_kaleidoscope_test_player(&mut app);
    app.update();
    (app, player)
}

fn open_app() -> App {
    let mut app = base_kaleidoscope_test_app();
    app.add_systems(Update, kaleidoscope_menu_open_routing);
    app.add_observer(kaleidoscope_pointer_move);
    set_kaleidoscope_visible(&mut app, false);
    spawn_kaleidoscope_test_player(&mut app);
    app.update();
    app
}

fn move_control(app: &mut App, action: MenuPageAction) {
    app.world_mut()
        .insert_resource(ambition::input::ActiveInputKind::Mouse);
    let entity = spawn_control(app, action);
    trigger_move(app, entity, Vec2::new(1.0, 0.0));
    app.update();
}

// ---- Fix 2: shoulder-bumper page turns -----------------------------------

fn nav_app() -> App {
    let mut app = base_kaleidoscope_test_app();
    app.add_systems(Update, kaleidoscope_focus_nav);
    set_kaleidoscope_visible(&mut app, true);
    app.world_mut()
        .resource_mut::<ActiveMenuPages<MenuPage, MenuPageAction>>()
        .active = Some(MenuPage::Items);
    spawn_kaleidoscope_test_player(&mut app);
    app.update();
    app
}

fn system_nav_app(focus: MenuFocus) -> App {
    let mut app = base_kaleidoscope_test_app();
    app.add_systems(Update, kaleidoscope_focus_nav);
    set_kaleidoscope_visible(&mut app, true);
    app.world_mut()
        .resource_mut::<ActiveMenuPages<MenuPage, MenuPageAction>>()
        .active = Some(MenuPage::System);
    app.world_mut().resource_mut::<KaleidoscopeCursor>().focus = focus;
    spawn_kaleidoscope_test_player(&mut app);
    app.update();
    app
}

fn press_bumper(app: &mut App, right: bool) {
    let mut frame = MenuControlFrame::default();
    if right {
        frame.page_right = true;
    } else {
        frame.page_left = true;
    }
    app.insert_resource(frame);
    app.update();
}

fn press_select(app: &mut App) {
    app.insert_resource(MenuControlFrame {
        select: true,
        ..Default::default()
    });
    app.update();
}

/// REGRESSION: switching Cube→Grid while the cube is open must keep the cube's
/// render set ticking until the fold-shut `amount` decays, so the camera can turn
/// off. Before the fix, a non-cube backend short-circuited render to `false`
/// immediately, freezing `amount` mid-fold → the cube stayed stuck on top.
#[test]
fn cube_render_keeps_folding_shut_after_backend_switch() {
    // Actively open on the cube backend → render.
    assert!(cube_render_needed(true, true, true, 1.0, 1.0));
    // Switched to Grid mid-fold (amount still high): MUST still render so the fold
    // completes and the camera deactivates. This is the bug fix.
    assert!(cube_render_needed(true, false, false, 0.0, 1.0));
    assert!(cube_render_needed(true, false, true, 0.0, 0.5));
    // Fully folded shut on a non-cube backend → stop (no churn, camera already off).
    assert!(!cube_render_needed(true, false, false, 0.0, 0.07));
    // Cube backend selected but menu closed and fully decayed → stop.
    assert!(!cube_render_needed(true, true, false, 0.0, 0.0));
    // Backend feature disabled → never render.
    assert!(!cube_render_needed(false, true, true, 1.0, 1.0));
}

fn press_dir(app: &mut App, left: bool) {
    let mut frame = MenuControlFrame::default();
    if left {
        frame.left = true;
    } else {
        frame.right = true;
    }
    app.insert_resource(frame);
    app.update();
}

fn placeholder_nav_app(page: MenuPage, focus: MenuFocus) -> App {
    let mut app = base_kaleidoscope_test_app();
    app.add_systems(Update, kaleidoscope_focus_nav);
    set_kaleidoscope_visible(&mut app, true);
    app.world_mut()
        .resource_mut::<ActiveMenuPages<MenuPage, MenuPageAction>>()
        .active = Some(page);
    app.world_mut().resource_mut::<KaleidoscopeCursor>().focus = focus;
    spawn_kaleidoscope_test_player(&mut app);
    app.update();
    app
}

fn active_page(app: &App) -> Option<MenuPage> {
    app.world()
        .resource::<ActiveMenuPages<MenuPage, MenuPageAction>>()
        .active
}

/// REGRESSION: the active backend's nav must CONSUME the frame's select edge, so
/// the OTHER inventory backend's nav (which shares `Res<MenuControlFrame>` in the
/// same frame) can't re-process it. Without this, selecting the "Menu Backend" row
/// flipped `InventoryUiBackend` mid-frame and the second nav flipped it back —
/// keyboard select on that row appeared to do nothing while a mouse click (a
/// one-shot observer) worked.
#[test]
fn active_backend_nav_consumes_the_select_edge() {
    let mut app = system_nav_app(MenuFocus::System(0));
    app.insert_resource(MenuControlFrame {
        select: true,
        ..Default::default()
    });
    app.update();
    assert!(
        !app.world().resource::<MenuControlFrame>().select,
        "the cube nav (active backend) consumed the select edge"
    );
}

/// The System face routes its `>`/`<` buttons through the SAME `edge_button_nav`
/// as the placeholder faces: an OUTWARD arrow off an edge rotates (landing on the
/// new face's back-edge), an INWARD arrow enters the row list.
#[test]
fn system_edge_outward_arrow_rotates() {
    let mut app = system_nav_app(MenuFocus::EdgeLeft);
    press_dir(&mut app, /* left = */ true); // outward off the < edge
    assert_eq!(
        active_page(&app),
        Some(MenuPage::System.on_viewer_left()),
        "outward arrow off the < edge rotates to the viewer-left page"
    );
}

/// The placeholder faces (Map/Quest) — which have only the two edge buttons — go
/// through the shared `edge_button_nav` too: INWARD crosses to the opposite edge
/// (no rotation), OUTWARD rotates, SELECT rotates.
#[test]
fn placeholder_edge_nav_matches_other_faces() {
    // INWARD (right, from the left edge) crosses to the opposite edge, no rotate.
    let mut inward = placeholder_nav_app(MenuPage::Quest, MenuFocus::EdgeLeft);
    press_dir(&mut inward, /* left = */ false);
    assert_eq!(
        inward.world().resource::<KaleidoscopeCursor>().focus,
        MenuFocus::EdgeRight,
        "inward arrow crosses to the opposite edge"
    );
    assert_eq!(
        active_page(&inward),
        Some(MenuPage::Quest),
        "inward arrow does NOT rotate"
    );

    // OUTWARD (right, from the right edge) rotates to the viewer-right page.
    let mut outward = placeholder_nav_app(MenuPage::Quest, MenuFocus::EdgeRight);
    press_dir(&mut outward, /* left = */ false);
    assert_eq!(
        active_page(&outward),
        Some(MenuPage::Quest.on_viewer_right()),
        "outward arrow rotates"
    );

    // SELECT on an edge rotates.
    let mut selected = placeholder_nav_app(MenuPage::Quest, MenuFocus::EdgeLeft);
    press_select(&mut selected);
    assert_eq!(
        active_page(&selected),
        Some(MenuPage::Quest.on_viewer_left()),
        "select on the < edge rotates to the viewer-left page"
    );
}

/// REGRESSION: on the System face, SELECT while the cursor sits on a `>`/`<`
/// page-turn edge button must ROTATE to that neighbour — exactly like every other
/// face. The System branch used to fall through to the row dispatch with `current`
/// normalised to `rows[0]`, so selecting `>Quest` wrongly activated the first
/// System row instead of turning the page.
#[test]
fn select_on_system_edge_button_turns_the_page() {
    let mut right = system_nav_app(MenuFocus::EdgeRight);
    press_select(&mut right);
    assert_eq!(
        right
            .world()
            .resource::<ActiveMenuPages<MenuPage, MenuPageAction>>()
            .active,
        Some(MenuPage::System.on_viewer_right()),
        "SELECT on the right edge rotates to the viewer-right page"
    );

    let mut left = system_nav_app(MenuFocus::EdgeLeft);
    press_select(&mut left);
    assert_eq!(
        left.world()
            .resource::<ActiveMenuPages<MenuPage, MenuPageAction>>()
            .active,
        Some(MenuPage::System.on_viewer_left()),
        "SELECT on the left edge rotates to the viewer-left page"
    );
}

#[test]
fn right_bumper_turns_to_the_viewer_right_page() {
    let mut app = nav_app();
    press_bumper(&mut app, true);
    assert_eq!(
        app.world()
            .resource::<ActiveMenuPages<MenuPage, MenuPageAction>>()
            .active,
        Some(MenuPage::Items.on_viewer_right()),
        "right bumper rotates to the viewer-right page (Fix 2)"
    );
    // The cursor lands on the new page's back-edge button (Fix 1): arriving from
    // the right edge means the LEFT edge button turns back home.
    assert_eq!(
        app.world().resource::<KaleidoscopeCursor>().focus,
        MenuFocus::EdgeLeft,
        "cursor seeds onto the back (left) edge button"
    );
}

#[test]
fn left_bumper_turns_to_the_viewer_left_page() {
    let mut app = nav_app();
    press_bumper(&mut app, false);
    assert_eq!(
        app.world()
            .resource::<ActiveMenuPages<MenuPage, MenuPageAction>>()
            .active,
        Some(MenuPage::Items.on_viewer_left()),
        "left bumper rotates to the viewer-left page (Fix 2)"
    );
    assert_eq!(
        app.world().resource::<KaleidoscopeCursor>().focus,
        MenuFocus::EdgeRight,
        "cursor seeds onto the back (right) edge button"
    );
}

#[test]
fn clicking_a_system_entry_drills_in() {
    let (mut app, _player) = click_app();
    app.world_mut()
        .resource_mut::<ActiveMenuPages<MenuPage, MenuPageAction>>()
        .active = Some(MenuPage::System);
    assert!(app
        .world()
        .resource::<KaleidoscopeSystemNav>()
        .open_entry
        .is_none());
    click_control(
        &mut app,
        MenuPageAction::OpenSystemEntry(SystemMenuEntryId::Audio),
    );
    assert_eq!(
        app.world().resource::<KaleidoscopeSystemNav>().open_entry,
        Some(SystemMenuEntryId::Audio),
        "clicking a System entry drills into it (Fix 4)"
    );
}

#[test]
fn clicking_a_system_setting_applies_it() {
    let (mut app, _player) = click_app();
    app.world_mut()
        .resource_mut::<ActiveMenuPages<MenuPage, MenuPageAction>>()
        .active = Some(MenuPage::System);
    app.world_mut()
        .resource_mut::<KaleidoscopeSystemNav>()
        .open_entry = Some(SystemMenuEntryId::Video);
    let before = app.world().resource::<UserSettings>().video.show_fps;
    click_control(&mut app, MenuPageAction::System(SettingsOptionId::ShowFps));
    let after = app.world().resource::<UserSettings>().video.show_fps;
    assert_ne!(before, after, "clicking a setting toggles it (Fix 4)");
}

#[test]
fn clicking_back_drills_out_to_the_entry_list() {
    let (mut app, _player) = click_app();
    app.world_mut()
        .resource_mut::<ActiveMenuPages<MenuPage, MenuPageAction>>()
        .active = Some(MenuPage::System);
    app.world_mut()
        .resource_mut::<KaleidoscopeSystemNav>()
        .open_entry = Some(SystemMenuEntryId::Audio);
    click_control(&mut app, MenuPageAction::CloseSystemEntry);
    assert!(
        app.world()
            .resource::<KaleidoscopeSystemNav>()
            .open_entry
            .is_none(),
        "clicking Back drills out to the entry list (Fix 4)"
    );
}

#[test]
fn clicking_a_radio_station_keeps_the_menu_open() {
    // Selecting a radio station auditions it WITHOUT closing the cube, so the
    // user can keep browsing. Audio is absent in this minimal fixture, so the
    // apply no-ops, but the menu must still stay open.
    let (mut app, _player) = click_app();
    app.world_mut()
        .resource_mut::<ActiveMenuPages<MenuPage, MenuPageAction>>()
        .active = Some(MenuPage::System);
    app.world_mut()
        .resource_mut::<KaleidoscopeSystemNav>()
        .open_entry = Some(SystemMenuEntryId::Radio);
    click_control(
        &mut app,
        MenuPageAction::SystemOption(SystemOptionId::Radio(0)),
    );
    assert!(
        app.world()
            .resource::<ambition::inventory_ui::InventoryUiState>()
            .visible,
        "auditioning a station keeps the cube open"
    );
}

#[test]
fn reset_sandbox_action_closes_and_unpauses() {
    // Reset Sandbox closes the cube via a dispatched action (`close_menu = true`).
    // When the menu was opened from gameplay (paused, not opened-from-pause), the
    // action-close must ALSO restore `GameMode::Playing` — exactly like a normal
    // Esc-close — instead of leaving the sim paused with the menu hidden. Before the
    // fix the close path only did `ui_state.visible = false`, so this stayed Paused.
    let (mut app, _player) = click_app();
    app.world_mut()
        .resource_mut::<ActiveMenuPages<MenuPage, MenuPageAction>>()
        .active = Some(MenuPage::System);
    // Open the menu from gameplay: paused, but NOT nested under the pause menu.
    app.world_mut()
        .resource_mut::<ambition::inventory_ui::InventoryUiState>()
        .opened_from_pause = false;
    app.world_mut()
        .resource_mut::<NextState<GameMode>>()
        .set(GameMode::Paused);
    app.update();
    assert_eq!(
        *app.world().resource::<State<GameMode>>().get(),
        GameMode::Paused,
        "precondition: menu opened from gameplay leaves the sim paused"
    );

    // Dispatch Reset Sandbox through the real pointer release/dispatch path.
    click_control(
        &mut app,
        MenuPageAction::SystemAction(SystemMenuAction::ResetSandbox),
    );

    assert!(
        !app.world()
            .resource::<ambition::inventory_ui::InventoryUiState>()
            .visible,
        "Reset Sandbox hides the cube"
    );
    // The action-close set NextState(Playing); apply the transition and confirm the
    // sim is unpaused (the bug left it stuck on Paused).
    app.update();
    assert_eq!(
        *app.world().resource::<State<GameMode>>().get(),
        GameMode::Playing,
        "Reset Sandbox closes the menu AND unpauses (back to Playing)"
    );
}

#[test]
fn reset_all_settings_action_resets_settings_and_closes() {
    // The cube's System menu surfaces Reset All Settings as a top-level Action
    // entry; dispatching it resets every persisted settings/dev resource to
    // defaults (the same set the pause menu's ResetAllSettings restores) and
    // folds the menu shut (close, which also unpauses).
    let (mut app, _player) = click_app();
    app.world_mut()
        .resource_mut::<ActiveMenuPages<MenuPage, MenuPageAction>>()
        .active = Some(MenuPage::System);

    // The IR surfaces Reset All Settings as an always-present Action entry.
    let model = SystemMenuModel::build(
        &UserSettings::default(),
        &RadioSnapshot::default(),
        &DevSnapshot::default(),
    );
    assert!(
        model.entry(SystemMenuEntryId::ResetAllSettings).is_some(),
        "Reset All Settings is surfaced as a top-level entry"
    );

    // Mutate persisted resources away from their defaults.
    app.world_mut()
        .resource_mut::<UserSettings>()
        .audio
        .master_volume = 0.123;
    app.world_mut()
        .resource_mut::<ambition::dev_tools::dev_tools::DeveloperTools>()
        .inspector_visible = true;

    // Dispatch Reset All Settings through the real pointer release/dispatch path.
    click_control(
        &mut app,
        MenuPageAction::SystemAction(SystemMenuAction::ResetAllSettings),
    );

    // UserSettings + DeveloperTools are back to defaults.
    assert_eq!(
        *app.world().resource::<UserSettings>(),
        UserSettings::default(),
        "Reset All Settings restores UserSettings to defaults"
    );
    assert!(
        !app.world()
            .resource::<ambition::dev_tools::dev_tools::DeveloperTools>()
            .inspector_visible,
        "Reset All Settings restores DeveloperTools to defaults"
    );
    // The cube folds shut (same close as Reset Sandbox).
    assert!(
        !app.world()
            .resource::<ambition::inventory_ui::InventoryUiState>()
            .visible,
        "Reset All Settings closes the cube"
    );
}

#[test]
fn quit_action_writes_app_exit_and_closes() {
    // The cube's System menu surfaces Quit to Desktop as a top-level Action
    // entry; dispatching it writes `AppExit::Success` and folds the menu shut.
    let (mut app, _player) = click_app();
    app.add_message::<bevy::app::AppExit>();
    app.world_mut()
        .resource_mut::<ActiveMenuPages<MenuPage, MenuPageAction>>()
        .active = Some(MenuPage::System);

    // The IR surfaces Quit as an always-present Action entry.
    let model = SystemMenuModel::build(
        &UserSettings::default(),
        &RadioSnapshot::default(),
        &DevSnapshot::default(),
    );
    assert_eq!(
        model.entry(SystemMenuEntryId::Quit).map(|e| &e.target),
        Some(&SystemMenuTarget::Action(SystemMenuAction::Quit)),
        "Quit is surfaced as a top-level Action entry"
    );

    // Dispatch Quit through the real pointer release/dispatch path.
    click_control(
        &mut app,
        MenuPageAction::SystemAction(SystemMenuAction::Quit),
    );

    // An AppExit::Success was written.
    let messages = app.world().resource::<Messages<bevy::app::AppExit>>();
    let mut reader = messages.get_cursor();
    let exits: Vec<_> = reader.read(messages).collect();
    assert_eq!(
        exits,
        vec![&bevy::app::AppExit::Success],
        "Quit dispatches a single AppExit::Success"
    );

    // The cube folds shut (same close as the other immediate actions).
    assert!(
        !app.world()
            .resource::<ambition::inventory_ui::InventoryUiState>()
            .visible,
        "Quit closes the cube"
    );
}

#[test]
fn system_edge_left_moves_inward_to_the_row_list() {
    let mut app = system_nav_app(MenuFocus::EdgeLeft);
    let mut frame = MenuControlFrame::default();
    frame.right = true;
    app.insert_resource(frame);
    app.update();

    assert_eq!(
        app.world().resource::<KaleidoscopeCursor>().focus,
        MenuFocus::System(0),
        "moving right from the < Items button enters the System row list instead of rotating"
    );
    assert_eq!(
        app.world()
            .resource::<ActiveMenuPages<MenuPage, MenuPageAction>>()
            .active,
        Some(MenuPage::System),
        "the cube stays on the System face while moving into the rows"
    );
}

#[test]
fn system_row_horizontal_moves_to_the_edge_buttons() {
    let mut app = system_nav_app(MenuFocus::System(1));
    let mut frame = MenuControlFrame::default();
    frame.left = true;
    app.insert_resource(frame);
    app.update();

    assert_eq!(
        app.world().resource::<KaleidoscopeCursor>().focus,
        MenuFocus::EdgeLeft,
        "horizontal motion from a row should land on the left edge button"
    );
}

#[test]
fn pointer_motion_selects_a_kaleidoscope_control() {
    let mut app = open_app();
    app.world_mut()
        .resource_mut::<ambition::inventory_ui::InventoryUiState>()
        .visible = true;
    app.world_mut()
        .resource_mut::<ActiveMenuPages<MenuPage, MenuPageAction>>()
        .active = Some(MenuPage::Items);
    app.world_mut().resource_mut::<KaleidoscopeCursor>().focus = MenuFocus::EdgeRight;

    move_control(
        &mut app,
        MenuPageAction::ChangePage(MenuPage::Items.on_viewer_left()),
    );

    assert_eq!(
        app.world().resource::<KaleidoscopeCursor>().focus,
        MenuFocus::EdgeLeft,
        "actual pointer motion updates the cube cursor"
    );
    assert_eq!(
        app.world()
            .resource::<KaleidoscopeCursor>()
            .last_pointer_focus,
        Some(MenuFocus::EdgeLeft),
        "the hovered focus is remembered for later move dedup"
    );
}

/// Faithful reproduction of the real app's input wiring: a leafwing player with
/// Esc bound to BOTH `Start` (pause) and `MenuBack`, the menu-frame populate
/// system, AND the cube routing — registered in the SAME default Update set so
/// the scheduler is free to order them as it does in the real app.
///
/// Fix 1 behaviour: while the menu is open, Esc BACKS OUT one level when inside a
/// nested System screen (`open_entry.is_some()`) and only CLOSES at the top level.
/// So from a drilled-in category: first Esc → back to the entry list (still open),
/// second Esc → close. There must be no double-trigger (Esc fires both
/// `menu.start` and `menu.back`).
#[test]
fn esc_backs_out_then_closes_the_kaleidoscope_via_real_input() {
    use ambition::input::SandboxAction;
    use ambition::render::rendering::PlayerVisual;
    use leafwing_input_manager::prelude::*;

    let mut app = App::new();
    app.init_resource::<VisualQualityConfirmState>();
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.add_plugins(bevy::time::TimePlugin);
    app.add_plugins(bevy::input::InputPlugin);
    app.add_plugins(InputManagerPlugin::<SandboxAction>::default());
    app.init_state::<GameMode>();
    app.init_resource::<InventoryUiBackend>();
    app.init_resource::<ActiveMenuPages<MenuPage, MenuPageAction>>();
    app.init_resource::<KaleidoscopeCursor>();
    app.init_resource::<ambition::input::ActiveInputKind>();
    app.init_resource::<KaleidoscopeSystemNav>();
    app.init_resource::<KaleidoscopeScroll>();
    app.init_resource::<CachedSystemMenu>();
    app.init_resource::<KaleidoscopePointerPress>();
    app.init_resource::<OwnedItems>();
    app.init_resource::<ambition::dev_tools::dev_tools::DeveloperTools>();
    app.init_resource::<ambition::dev_tools::SandboxDevState>();
    app.init_resource::<ambition::actors::ldtk_world::LdtkHotReloadState>();
    app.init_resource::<ambition::actors::session::reset::SandboxResetRequested>();
    app.init_resource::<ambition::dev_tools::dev_tools::EditableMovementTuning>();
    app.init_resource::<UserSettings>();
    app.init_resource::<ambition::inventory_ui::InventoryUiState>();
    app.init_resource::<ambition::menu::map::MapMenuState>();
    app.init_resource::<MenuControlFrame>();
    app.init_resource::<ambition::input::MenuInputState>();
    app.add_message::<PlayerHealRequested>();
    app.add_message::<ambition::sfx::OwnedSfxMessage>();
    app.add_systems(
        Update,
        (
            ambition::actors::schedule::populate_menu_control_frame_from_actions,
            kaleidoscope_menu_open_routing,
            kaleidoscope_focus_nav,
        )
            .chain(),
    );
    *app.world_mut().resource_mut::<InventoryUiBackend>() = InventoryUiBackend::LunexKaleidoscope;

    // Esc → both Start (pause) and MenuBack, exactly like the keyboard preset.
    // Device state lives on the persistent participant, never on the player
    // entity — the same split the real host boots with.
    let mut map = InputMap::<SandboxAction>::default();
    map.insert(SandboxAction::Start, KeyCode::Escape);
    map.insert(SandboxAction::MenuBack, KeyCode::Escape);
    app.world_mut().spawn((
        ambition::input::InputParticipant::primary(),
        ambition::input::ParticipantContexts::default(),
        ActionState::<SandboxAction>::default(),
        map,
    ));
    app.world_mut().spawn((
        PlayerVisual,
        PlayerEntity,
        PrimaryPlayer,
        ActionSet::default(),
        BodyMana::default(),
    ));
    app.update();

    let press_esc = |app: &mut App, down: bool| {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        if down {
            keys.press(KeyCode::Escape);
        } else {
            keys.release(KeyCode::Escape);
        }
    };
    let visible = |app: &App| {
        app.world()
            .resource::<ambition::inventory_ui::InventoryUiState>()
            .visible
    };

    // First Esc press → opens the cube.
    press_esc(&mut app, true);
    app.update();
    press_esc(&mut app, false);
    app.update();
    assert!(visible(&app), "first Esc opens the cube");

    // Drill INTO a System-page category. The close path is page-dependent: inside
    // a category Esc must BACK OUT one level (not close), and that drill-out is
    // owned by the start branch (we consume the co-firing `menu.back` so the nav
    // systems never see this Esc).
    app.world_mut()
        .resource_mut::<ActiveMenuPages<MenuPage, MenuPageAction>>()
        .active = Some(MenuPage::System);
    app.world_mut()
        .resource_mut::<KaleidoscopeSystemNav>()
        .open_entry = Some(SystemMenuEntryId::Audio);
    app.world_mut().resource_mut::<KaleidoscopeCursor>().focus = MenuFocus::System(0);

    // Second Esc press → backs OUT to the entry list (menu stays open).
    press_esc(&mut app, true);
    app.update();
    press_esc(&mut app, false);
    app.update();
    assert!(
        visible(&app),
        "second Esc (nested) backs out one level, keeping the menu open"
    );
    assert!(
        app.world()
            .resource::<KaleidoscopeSystemNav>()
            .open_entry
            .is_none(),
        "second Esc drilled out of the open System entry"
    );

    // Third Esc press → now at the top level, CLOSES the cube.
    press_esc(&mut app, true);
    app.update();
    press_esc(&mut app, false);
    app.update();
    assert!(!visible(&app), "third Esc (top level) closes the cube");
}

#[test]
fn opening_the_kaleidoscope_clears_stale_pointer_hover_state() {
    let mut app = open_app();
    app.world_mut()
        .resource_mut::<KaleidoscopeCursor>()
        .last_pointer_focus = Some(MenuFocus::Item(7));
    app.world_mut().resource_mut::<MenuControlFrame>().start = true;
    app.world_mut()
        .resource_mut::<ambition::inventory_ui::InventoryUiState>()
        .visible = false;
    app.update();

    assert_eq!(
        app.world().resource::<KaleidoscopeCursor>().last_pointer_focus,
        None,
        "opening the cube clears stale pointer hover state so parked hover cannot select immediately"
    );
}

// ---- Bug 2: click/tap activation survives a hover-driven republish ---------
//
// Root cause (now fixed): a `Pointer<Move>` changed `cursor.focus`, which the
// republish baked into its dirty key, so it rewrote `ActiveMenuPages`; the lib's
// `rebuild_cube_faces` then despawned + respawned every control. When that
// happened BETWEEN a pointer press and release, Bevy dropped the `Pointer<Click>`
// (the press entity no longer existed), so clicking a control did NOTHING while
// mouse-over highlight worked. The fix moves the highlight + detail text in place
// (no rebuild on a cursor-only move). These tests reproduce the drop and assert
// the click now dispatches.

/// A faithful stand-in for the lib's `rebuild_cube_faces`: whenever
/// `ActiveMenuPages` is `Changed` (which the OLD republish did on every cursor
/// move), despawn every `AmbitionMenuControl` and respawn the actionable controls
/// from `pages.pages`. This reproduces the exact entity-id churn that dropped the
/// click — the real renderer is too heavy to run headless.
fn fake_rebuild_cube_faces(
    mut commands: Commands,
    pages: Res<ActiveMenuPages<MenuPage, MenuPageAction>>,
    existing: Query<Entity, With<AmbitionMenuControl<MenuPageAction>>>,
    mut built: Local<bool>,
) {
    if !pages.is_changed() && *built {
        return;
    }
    *built = true;
    for entity in &existing {
        commands.entity(entity).despawn();
    }
    for page in &pages.pages {
        for node in &page.nodes {
            if let ambition::menu::MenuNode::Control {
                kind,
                action: Some(action),
                ..
            } = node
            {
                commands.spawn((
                    AmbitionMenuControl::<MenuPageAction> {
                        kind: *kind,
                        action: Some(action.clone()),
                        focus: ambition::menu::MenuFocusKey::default(),
                    },
                    MenuVisualState::default(),
                ));
            }
        }
    }
}

/// A full Bug-2 fixture: the REAL republish + in-place highlight/detail systems +
/// the `fake_rebuild` (mirroring the lib) + the real pointer observers, on the
/// given active page. Drives the genuine despawn-on-republish path.
fn bug2_app(active: MenuPage) -> App {
    let mut app = base_kaleidoscope_test_app();
    app.add_systems(
        Update,
        (
            cache_system_menu,
            republish_kaleidoscope_pages,
            kaleidoscope_sync_focus_visuals,
            kaleidoscope_sync_detail_text,
            fake_rebuild_cube_faces,
        )
            .chain(),
    );
    app.add_observer(kaleidoscope_pointer_press);
    app.add_observer(kaleidoscope_pointer_move);
    app.add_observer(kaleidoscope_pointer_release);
    set_kaleidoscope_visible(&mut app, true);
    app.world_mut()
        .resource_mut::<ActiveMenuPages<MenuPage, MenuPageAction>>()
        .active = Some(active);
    spawn_kaleidoscope_test_player(&mut app);
    // First update: republish builds the page data, fake_rebuild spawns controls.
    app.update();
    app
}

/// The live control entity carrying `action` (the one the renderer spawned).
fn control_entity(app: &mut App, action: MenuPageAction) -> Entity {
    let mut q = app
        .world_mut()
        .query::<(Entity, &AmbitionMenuControl<MenuPageAction>)>();
    q.iter(app.world())
        .find(|(_, c)| c.action.as_ref() == Some(&action))
        .map(|(e, _)| e)
        .unwrap_or_else(|| panic!("no live control for {action:?}"))
}

/// Reproduce Bug 2 on the NEW release-dispatch path: PRESS the original
/// `click_target` (arming its action), then hover-move onto `move_to` (which
/// rebuilds the face and DESPAWNS the pressed control), then RELEASE. The action
/// must still dispatch because it was captured at press time — entity-independent.
///
/// Under the OLD `Pointer<Click>` path this dropped the activation: the press
/// entity was gone by release, so the compound click never resolved.
fn hover_then_click(app: &mut App, move_to: MenuPageAction, click_target: MenuPageAction) {
    // The entity a real pointer press latches onto, captured BEFORE the rebuild.
    let target = control_entity(app, click_target);
    // 1. PRESS the target: arms the action in `KaleidoscopePointerPress`.
    app.world_mut().trigger(Pointer::new(
        PointerId::Mouse,
        pointer_location(),
        Press {
            button: bevy::picking::pointer::PointerButton::Primary,
            hit: HitData::new(target, 0.0, None, None),
        },
        target,
    ));
    // 2. Hover-move onto a different control: changes `cursor.focus`, which the
    //    republish bakes into pages → fake_rebuild despawns `target`.
    let move_target = control_entity(app, move_to);
    app.world_mut().trigger(Pointer::new(
        PointerId::Mouse,
        pointer_location(),
        Move {
            hit: HitData::new(move_target, 0.0, None, None),
            delta: Vec2::new(2.0, 0.0),
        },
        move_target,
    ));
    app.update();
    // 3. RELEASE. The release entity (`target`) may now be despawned, but the
    //    handler dispatches the action STORED at press time, not the release
    //    entity — so the activation survives the rebuild (the fix).
    app.world_mut().trigger(Pointer::new(
        PointerId::Mouse,
        pointer_location(),
        Release {
            button: bevy::picking::pointer::PointerButton::Primary,
            hit: HitData::new(target, 0.0, None, None),
        },
        target,
    ));
    app.update();
}

#[test]
fn bug2_item_equip_click_survives_a_hover_republish() {
    let mut app = bug2_app(MenuPage::Items);
    // Two owned, equippable (held-item) weapons so both an equip target and a
    // distinct hover target exist as live controls.
    {
        let mut owned = app.world_mut().resource_mut::<OwnedItems>();
        owned.grant(Item::Blink, 1);
        owned.grant(Item::Axe, 1);
    }
    app.update();
    assert!(
        !app.world()
            .resource::<OwnedItems>()
            .is_equipped(Item::Blink),
        "precondition: Blink not equipped yet"
    );
    // Hover Axe (moves focus → old rebuild), then click Blink (was despawned).
    hover_then_click(
        &mut app,
        MenuPageAction::Equip(Item::Axe),
        MenuPageAction::Equip(Item::Blink),
    );
    assert!(
        app.world()
            .resource::<OwnedItems>()
            .is_equipped(Item::Blink),
        "clicking an item after a hover-move must still equip it (Bug 2)"
    );
}

#[test]
fn bug2_page_turn_click_survives_a_hover_republish() {
    let mut app = bug2_app(MenuPage::Items);
    app.update();
    let target_page = MenuPage::Items.on_viewer_right();
    // Hover the LEFT edge (moves focus), then click the RIGHT edge (page turn).
    hover_then_click(
        &mut app,
        MenuPageAction::ChangePage(MenuPage::Items.on_viewer_left()),
        MenuPageAction::ChangePage(target_page),
    );
    assert_eq!(
        app.world()
            .resource::<ActiveMenuPages<MenuPage, MenuPageAction>>()
            .active,
        Some(target_page),
        "clicking a page-turn edge after a hover-move must still turn the page (Bug 2)"
    );
}

#[test]
fn bug2_system_row_click_survives_a_hover_republish() {
    let mut app = bug2_app(MenuPage::System);
    app.update();
    assert!(
        app.world()
            .resource::<KaleidoscopeSystemNav>()
            .open_entry
            .is_none(),
        "precondition: no System entry open"
    );
    // Hover the Video entry (moves focus), then click the Audio entry (drill in).
    hover_then_click(
        &mut app,
        MenuPageAction::OpenSystemEntry(SystemMenuEntryId::Video),
        MenuPageAction::OpenSystemEntry(SystemMenuEntryId::Audio),
    );
    assert_eq!(
        app.world().resource::<KaleidoscopeSystemNav>().open_entry,
        Some(SystemMenuEntryId::Audio),
        "clicking a System row after a hover-move must still drill in (Bug 2)"
    );
}

// ---- Features C/D/E: scroll position + tap/drag cancel --------------------

/// A System fixture drilled into Developer (16 toggles + Back = a list LONGER
/// than `SYSTEM_VISIBLE_ROWS`, so it is scrollable) running the real scroll
/// chain: keyboard nav, the mouse-wheel scroller, the scrollbar-drag applier,
/// and the page republish. No audio resources needed (dev toggles overflow).
fn scroll_app() -> App {
    let mut app = base_kaleidoscope_test_app();
    app.add_message::<bevy::input::mouse::MouseWheel>();
    app.add_message::<ambition::menu::MenuScrollDragged>();
    app.add_systems(
        Update,
        (
            kaleidoscope_focus_nav,
            kaleidoscope_scroll_wheel.run_if(kaleidoscope_menu_visible),
            kaleidoscope_apply_scroll_drag.run_if(kaleidoscope_menu_visible),
            cache_system_menu,
            republish_kaleidoscope_pages,
        )
            .chain(),
    );
    set_kaleidoscope_visible(&mut app, true);
    app.world_mut()
        .resource_mut::<ActiveMenuPages<MenuPage, MenuPageAction>>()
        .active = Some(MenuPage::System);
    app.world_mut()
        .resource_mut::<KaleidoscopeSystemNav>()
        .open_entry = Some(SystemMenuEntryId::Developer);
    app.world_mut().resource_mut::<KaleidoscopeCursor>().focus = MenuFocus::System(0);
    spawn_kaleidoscope_test_player(&mut app);
    app.update();
    app
}

/// The live Developer row count for the scroll fixture (18 toggles + Back).
fn scroll_total_rows(app: &App) -> usize {
    let settings = app.world().resource::<UserSettings>();
    let dev = app
        .world()
        .resource::<ambition::dev_tools::dev_tools::DeveloperTools>();
    let dev_state = app
        .world()
        .resource::<ambition::dev_tools::SandboxDevState>();
    let ldtk_reload = app
        .world()
        .resource::<ambition::actors::ldtk_world::LdtkHotReloadState>();
    let backend = *app.world().resource::<InventoryUiBackend>();
    let snap = dev_snapshot(DevToggleRead {
        dev,
        dev_state,
        ldtk_reload,
        backend,
        #[cfg(feature = "portal_render")]
        portal_effect: None,
        #[cfg(feature = "portal_render")]
        portal_camera: None,
        base_gravity: None,
    });
    let model = SystemMenuModel::build(settings, &RadioSnapshot::default(), &snap);
    system_rows(&model, Some(SystemMenuEntryId::Developer)).len()
}

/// Feature D: the mouse wheel scrolls the System window (window_start) WITHOUT
/// moving the keyboard selection cursor.
#[test]
fn mouse_wheel_scrolls_window_not_selection() {
    let mut app = scroll_app();
    let total = scroll_total_rows(&app);
    assert!(
        total > SYSTEM_VISIBLE_ROWS,
        "fixture list must overflow: {total}"
    );

    let cursor_before = app.world().resource::<KaleidoscopeCursor>().focus;
    assert_eq!(
        app.world()
            .resource::<KaleidoscopeScroll>()
            .system_window_start,
        None,
        "starts following the cursor (no override)"
    );

    // Wheel DOWN three notches (negative y = scroll down).
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

    let scroll = app
        .world()
        .resource::<KaleidoscopeScroll>()
        .system_window_start;
    assert_eq!(
        scroll,
        Some(3),
        "three wheel-down notches scroll the window to row 3"
    );
    assert_eq!(
        app.world().resource::<KaleidoscopeCursor>().focus,
        cursor_before,
        "the wheel must NOT move the selection cursor (Feature D)"
    );
}

/// Feature C: applying a scrollbar drag fraction (the lib's neutral signal) moves
/// the window_start proportionally across the scrollable range.
#[test]
fn scrollbar_drag_fraction_sets_window_start_proportionally() {
    let mut app = scroll_app();
    let total = scroll_total_rows(&app);
    let max = system_max_window_start(total);
    assert!(max > 0, "fixture must be scrollable");

    let cursor_before = app.world().resource::<KaleidoscopeCursor>().focus;

    // Drag to the BOTTOM of the track (fraction 1.0) -> window_start == max.
    app.world_mut()
        .resource_mut::<Messages<ambition::menu::MenuScrollDragged>>()
        .write(ambition::menu::MenuScrollDragged { fraction: 1.0 });
    app.update();
    assert_eq!(
        app.world()
            .resource::<KaleidoscopeScroll>()
            .system_window_start,
        Some(max),
        "fraction 1.0 scrolls to the bottom (Feature C)"
    );

    // Drag to the MIDDLE (fraction 0.5) -> ~half the range.
    app.world_mut()
        .resource_mut::<Messages<ambition::menu::MenuScrollDragged>>()
        .write(ambition::menu::MenuScrollDragged { fraction: 0.5 });
    app.update();
    let expected_mid = (0.5 * max as f32).round() as usize;
    assert_eq!(
        app.world()
            .resource::<KaleidoscopeScroll>()
            .system_window_start,
        Some(expected_mid),
        "fraction 0.5 maps to the middle of the range"
    );
    assert_eq!(
        app.world().resource::<KaleidoscopeCursor>().focus,
        cursor_before,
        "a scrollbar drag does not move the selection cursor"
    );
}

/// Feature C/D: a keyboard move after a wheel/drag scroll CLEARS the override so
/// the window snaps back to following the selection cursor.
#[test]
fn keyboard_nav_clears_the_scroll_override() {
    let mut app = scroll_app();
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
    assert!(
        app.world()
            .resource::<KaleidoscopeScroll>()
            .system_window_start
            .is_some(),
        "wheel set an override"
    );

    // A DOWN keypress moves the cursor and clears the override.
    let mut frame = MenuControlFrame::default();
    frame.down = true;
    app.insert_resource(frame);
    app.update();
    assert_eq!(
        app.world()
            .resource::<KaleidoscopeScroll>()
            .system_window_start,
        None,
        "keyboard nav resumes cursor-follow scrolling (Features C/D)"
    );
}

// ---- Feature E: tap activates, drag-away cancels --------------------------

/// Build a control + fire a Press at `press_pos`, a Move at `move_pos`, then a
/// Release — exactly the mouse/touch sequence Bevy picking produces. Returns the
/// `KaleidoscopeSystemNav.open_entry` after, so the test can see whether the
/// release's drill-in action fired (activated) or was cancelled by a drag.
fn press_move_click(app: &mut App, press_pos: Vec2, move_pos: Vec2) -> Entity {
    let entity = app
        .world_mut()
        .spawn(AmbitionMenuControl::<MenuPageAction> {
            kind: ambition::menu::MenuControlKind::OptionToggle,
            action: Some(MenuPageAction::OpenSystemEntry(SystemMenuEntryId::Video)),
            focus: ambition::menu::MenuFocusKey::default(),
        })
        .id();
    let loc = |p: Vec2| Location {
        target: NormalizedRenderTarget::None {
            width: 1,
            height: 1,
        },
        position: p,
    };
    app.world_mut().trigger(Pointer::new(
        PointerId::Mouse,
        loc(press_pos),
        Press {
            button: bevy::picking::pointer::PointerButton::Primary,
            hit: HitData::new(entity, 0.0, None, None),
        },
        entity,
    ));
    app.world_mut().trigger(Pointer::new(
        PointerId::Mouse,
        loc(move_pos),
        Move {
            hit: HitData::new(entity, 0.0, None, None),
            delta: move_pos - press_pos,
        },
        entity,
    ));
    app.world_mut().trigger(Pointer::new(
        PointerId::Mouse,
        loc(move_pos),
        Release {
            button: bevy::picking::pointer::PointerButton::Primary,
            hit: HitData::new(entity, 0.0, None, None),
        },
        entity,
    ));
    app.update();
    entity
}

/// Feature E: a clean tap (press + tiny move + release under the drag threshold)
/// ACTIVATES the control; a press + drag-away beyond the threshold CANCELS it.
#[test]
fn tap_activates_drag_away_cancels() {
    // Clean tap: tiny move -> drill into Video.
    let (mut app, _player) = click_app();
    // The control's drill-in action needs an active System page for the click
    // dispatch to resolve OpenSystemEntry against the live model.
    app.world_mut()
        .resource_mut::<ActiveMenuPages<MenuPage, MenuPageAction>>()
        .active = Some(MenuPage::System);
    press_move_click(&mut app, Vec2::new(10.0, 10.0), Vec2::new(12.0, 11.0));
    assert_eq!(
        app.world().resource::<KaleidoscopeSystemNav>().open_entry,
        Some(SystemMenuEntryId::Video),
        "a clean tap activates the control (Feature E)"
    );

    // Drag away: a large move past the threshold -> NO activation.
    let (mut app, _player) = click_app();
    app.world_mut()
        .resource_mut::<ActiveMenuPages<MenuPage, MenuPageAction>>()
        .active = Some(MenuPage::System);
    press_move_click(&mut app, Vec2::new(10.0, 10.0), Vec2::new(200.0, 200.0));
    assert_eq!(
        app.world().resource::<KaleidoscopeSystemNav>().open_entry,
        None,
        "a press-then-drag-away is cancelled, not activated (Feature E)"
    );
}

/// Spawn a real control carrying `action` and fire a `Pointer<Press>` on it
/// (arming the guard via the real press handler), returning its entity.
fn arm_press(app: &mut App, action: MenuPageAction) -> Entity {
    let entity = spawn_control(app, action);
    trigger_press(app, entity);
    app.update();
    entity
}

/// Fire a `Pointer<Release>` whose hit/target is `entity` (which may be despawned).
fn fire_release(app: &mut App, entity: Entity) {
    trigger_release(app, entity);
    app.update();
}

/// THE KEY TEST. The GUI failure exactly: a press is armed on a control, then the
/// perspective cube REBUILDS its faces (despawns + respawns every control) BEFORE
/// the release lands. With the old `Pointer<Click>` observer this dropped the
/// activation (press/release no longer resolved to the same live entity). The new
/// release-dispatch path stores the action at PRESS time, so it survives the
/// rebuild: the release still equips the item.
#[test]
fn release_dispatch_survives_a_control_rebuild_between_press_and_release() {
    let (mut app, _player) = click_app();
    {
        let mut owned = app.world_mut().resource_mut::<OwnedItems>();
        owned.grant(Item::Blink, 1);
    }
    app.world_mut()
        .resource_mut::<ActiveMenuPages<MenuPage, MenuPageAction>>()
        .active = Some(MenuPage::Items);
    app.update();
    assert!(
        !app.world()
            .resource::<OwnedItems>()
            .is_equipped(Item::Blink),
        "precondition: Blink not equipped yet"
    );

    // 1. Arm a press on the Blink control.
    let pressed = arm_press(&mut app, MenuPageAction::Equip(Item::Blink));
    assert_eq!(
        app.world().resource::<KaleidoscopePointerPress>().action,
        Some(MenuPageAction::Equip(Item::Blink)),
        "the press armed the control's action in the guard"
    );

    // 2. Simulate a face rebuild: despawn EVERY control (incl. the pressed one)
    //    and respawn a fresh one with a NEW entity id, exactly like the cube does
    //    on a hover-driven republish.
    {
        let to_despawn: Vec<Entity> = app
            .world_mut()
            .query_filtered::<Entity, With<AmbitionMenuControl<MenuPageAction>>>()
            .iter(app.world())
            .collect();
        for e in to_despawn {
            app.world_mut().entity_mut(e).despawn();
        }
        app.world_mut()
            .spawn(AmbitionMenuControl::<MenuPageAction> {
                kind: ambition::menu::MenuControlKind::OptionToggle,
                action: Some(MenuPageAction::Equip(Item::Blink)),
                focus: ambition::menu::MenuFocusKey::default(),
            });
    }
    assert!(
        app.world().get_entity(pressed).is_err(),
        "the pressed entity is gone after the rebuild (this is what broke Pointer<Click>)"
    );

    // 3. Release on the now-DEAD pressed entity. The handler dispatches the action
    //    stored at press time, not the release entity — so it still equips.
    fire_release(&mut app, pressed);
    assert!(
        app.world()
            .resource::<OwnedItems>()
            .is_equipped(Item::Blink),
        "release dispatches the action armed at press time even after the control \
         was despawned + respawned between press and release (the GUI mouse-click fix)"
    );
}

/// A plain press→release on a live control activates (the common case).
#[test]
fn press_then_release_equips_an_item() {
    let (mut app, _player) = click_app();
    {
        let mut owned = app.world_mut().resource_mut::<OwnedItems>();
        owned.grant(Item::Blink, 1);
    }
    app.world_mut()
        .resource_mut::<ActiveMenuPages<MenuPage, MenuPageAction>>()
        .active = Some(MenuPage::Items);
    app.update();
    let entity = arm_press(&mut app, MenuPageAction::Equip(Item::Blink));
    fire_release(&mut app, entity);
    assert!(
        app.world()
            .resource::<OwnedItems>()
            .is_equipped(Item::Blink),
        "a clean press→release on an item control equips it"
    );
}

// ---- CURSOR HIGHLIGHT regression -----------------------------------------

/// Build an app with the real lib cube plugin so `rebuild_cube_faces` spawns
/// REAL controls (with their `MenuVisualState`, `KaleidoscopeControlStyle`, and
/// HIDDEN `SelectionCorner` children), wire the sandbox focus writer + the lib
/// focus readers, publish the Items page with one owned item, and grant that item.
fn highlight_app(owned_item: Item) -> App {
    highlight_app_ordered(owned_item, true)
}

/// `writer_first = true` mirrors a correctly-ordered chain (writer before the
/// lib `Changed` readers). `writer_first = false` reproduces the REAL app's
/// hazard: the lib readers (added by the plugin as plain unordered `Update`
/// systems) can run BEFORE the sandbox writer, so the `Changed<MenuVisualState>`
/// the writer raises is consumed one frame too late — and the writer is
/// change-detection-gated, so it never re-raises it. The highlight never shows.
fn highlight_app_ordered(owned_item: Item, writer_first: bool) -> App {
    use ambition_menu_kaleidoscope::{sync_control_focus_visuals, sync_selection_corner_visuals};
    // The icon asset loads (`AssetServer::load`) need the IO task pool.
    bevy::tasks::IoTaskPool::get_or_init(Default::default);
    let mut app = App::new();
    app.init_resource::<VisualQualityConfirmState>();
    app.add_plugins(bevy::asset::AssetPlugin::default());
    app.init_asset::<StandardMaterial>();
    app.init_asset::<Mesh>();
    app.init_asset::<Image>();
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.init_state::<GameMode>();
    // Resources the host systems read.
    app.init_resource::<InventoryUiBackend>();
    app.init_resource::<ActiveMenuPages<MenuPage, MenuPageAction>>();
    app.init_resource::<KaleidoscopeCursor>();
    app.init_resource::<ambition::input::ActiveInputKind>();
    app.init_resource::<KaleidoscopeSystemNav>();
    app.init_resource::<KaleidoscopeScroll>();
    app.init_resource::<CachedSystemMenu>();
    app.init_resource::<KaleidoscopePointerPress>();
    let mut owned = OwnedItems::default();
    owned.grant(owned_item, 1);
    app.insert_resource(owned);
    app.init_resource::<ambition::dev_tools::dev_tools::DeveloperTools>();
    app.init_resource::<ambition::dev_tools::SandboxDevState>();
    app.init_resource::<ambition::actors::ldtk_world::LdtkHotReloadState>();
    app.init_resource::<ambition::actors::session::reset::SandboxResetRequested>();
    app.init_resource::<ambition::dev_tools::dev_tools::EditableMovementTuning>();
    app.init_resource::<UserSettings>();
    app.init_resource::<ambition::inventory_ui::InventoryUiState>();
    app.add_message::<ambition::sfx::OwnedSfxMessage>();
    *app.world_mut().resource_mut::<InventoryUiBackend>() = InventoryUiBackend::LunexKaleidoscope;
    app.world_mut()
        .resource_mut::<ambition::inventory_ui::InventoryUiState>()
        .visible = true;

    // The lib's ring root that `rebuild_cube_faces` parents faces under. We spawn
    // it directly (the plugin's `setup_cube` would also add a Camera3d we don't
    // need headlessly).
    app.world_mut().spawn((
        ambition::menu::AmbitionMenuRoot,
        ambition_menu_kaleidoscope::MenuRing,
        Transform::default(),
        Visibility::Visible,
    ));
    app.insert_resource(KaleidoscopeMenuConfig {
        draw_nav_arrows: false,
        pickable_controls: true,
        ..Default::default()
    });

    // Wire it like the REAL app does. The sandbox writer lives in its own chain;
    // the lib `Changed<MenuVisualState>` readers + the rebuild are added as plain,
    // UNORDERED `Update` systems (exactly as `KaleidoscopeMenuPlugin::build` adds
    // them). `writer_first` forces the writer to run before the readers (the fixed
    // ordering); `!writer_first` leaves them unordered so the readers may be
    // scheduled BEFORE the writer (the regression hazard).
    app.add_systems(
        Update,
        ambition_menu_kaleidoscope::rebuild_cube_faces::<MenuPage, MenuPageAction>,
    );
    if writer_first {
        // The FIX: republish + the host focus writer run AFTER the lib rebuild (so
        // the writer always writes to the freshly (re)spawned controls), and the
        // lib `Changed` readers run AFTER the writer (so they see the flipped flags
        // the same frame). This is the ordering `install_kaleidoscope_menu` +
        // `KaleidoscopeMenuPlugin` declare on the real app.
        app.add_systems(
            Update,
            (
                cache_system_menu,
                republish_kaleidoscope_pages,
                kaleidoscope_sync_focus_visuals,
            )
                .chain()
                .after(ambition_menu_kaleidoscope::rebuild_cube_faces::<MenuPage, MenuPageAction>),
        );
        app.add_systems(
            Update,
            (sync_control_focus_visuals, sync_selection_corner_visuals)
                .after(kaleidoscope_sync_focus_visuals),
        );
    } else {
        // The REGRESSION wiring: nothing orders the host writer against the lib
        // rebuild, so `rebuild_cube_faces` can despawn+respawn controls (resetting
        // `MenuVisualState` to focused:false) AFTER the writer flipped them, and the
        // `Changed` readers run before the writer. The highlight is dropped.
        app.add_systems(
            Update,
            (
                cache_system_menu,
                republish_kaleidoscope_pages,
                kaleidoscope_sync_focus_visuals,
            )
                .chain(),
        );
        app.add_systems(
            Update,
            (sync_control_focus_visuals, sync_selection_corner_visuals)
                .before(kaleidoscope_sync_focus_visuals),
        );
    }

    // Publish the Items page (one frame to spawn the controls/corners).
    app.world_mut()
        .resource_mut::<ActiveMenuPages<MenuPage, MenuPageAction>>()
        .active = Some(MenuPage::Items);
    let pages = build_inventory_pages(
        &app.world().resource::<OwnedItems>().clone(),
        None,
        MenuFocus::Item(owned_item.index()),
        &app.world().resource::<UserSettings>().clone(),
        &RadioSnapshot::default(),
        &DevSnapshot::default(),
        0,
        None,
    );
    app.world_mut()
        .resource_mut::<ActiveMenuPages<MenuPage, MenuPageAction>>()
        .replace_pages(pages, MenuPage::Items);
    app.update();
    app
}

/// REGRESSION pin: the cursor highlight stays on the ACTIVE face. The cube spawns
/// every face's controls at once, so a control built `selected` (an equipped item
/// / active station) on a rotated-away face spawns with `focused` pre-set. The
/// writer must RESET it (the "flash"/stuck-lit bug), while still highlighting the
/// same-focus control on the active face. We spawn two controls with the SAME
/// matching action — one marked `KaleidoscopeActiveFaceControl`, one not — both
/// pre-lit, and assert only the active-face one survives.
#[test]
fn highlight_resets_inactive_face_controls() {
    let item = Item::PortalGun;
    let mut app = base_kaleidoscope_test_app();
    app.add_systems(
        Update,
        (cache_system_menu, kaleidoscope_sync_focus_visuals).chain(),
    );
    app.world_mut()
        .resource_mut::<ActiveMenuPages<MenuPage, MenuPageAction>>()
        .active = Some(MenuPage::Items);
    app.world_mut().resource_mut::<KaleidoscopeCursor>().focus = MenuFocus::Item(item.index());

    let pre_lit = || MenuVisualState {
        focused: true,
        selected: true,
        ..Default::default()
    };
    let control = |with_marker: bool| {
        (
            AmbitionMenuControl::<MenuPageAction> {
                kind: ambition::menu::MenuControlKind::Action,
                action: Some(MenuPageAction::Equip(item)),
                focus: ambition::menu::MenuFocusKey::default(),
            },
            pre_lit(),
            with_marker,
        )
    };
    // Active-face control (marked) — must stay lit; inactive (unmarked) — reset.
    let (a_ctrl, a_vis, _) = control(true);
    let active = app
        .world_mut()
        .spawn((a_ctrl, a_vis, KaleidoscopeActiveFaceControl))
        .id();
    let (i_ctrl, i_vis, _) = control(false);
    let inactive = app.world_mut().spawn((i_ctrl, i_vis)).id();

    app.update();

    assert!(
        app.world().get::<MenuVisualState>(active).unwrap().focused,
        "the active-face control matching the cursor stays highlighted"
    );
    assert!(
        !app.world()
            .get::<MenuVisualState>(inactive)
            .unwrap()
            .focused,
        "an inactive-face control (no marker) is reset, even though its action \
         matches the cursor focus — fixes the rotate 'flash'/stuck-lit highlight"
    );
}

/// REGRESSION pin: setting the cursor onto an owned item's focus must (a) flip
/// that control's `MenuVisualState.focused`, (b) make its `SelectionCorner`
/// children VISIBLE, and (c) leave a non-focused control's corners HIDDEN.
#[test]
fn cursor_focus_highlights_the_control_and_reveals_its_corners() {
    let item = Item::PortalGun;
    let mut app = highlight_app(item);
    set_focus_and_step(&mut app, item, 1);
    assert_highlight_visible(&mut app, item);
}

/// REGRESSION reproduction: when the host republishes (a hover, a late texture
/// load, an inventory change — all common in-game), `rebuild_cube_faces` despawns
/// and respawns every control with a fresh `MenuVisualState { focused: false }`.
/// With the UN-ordered wiring (lib rebuild + `Changed` readers added as plain
/// `Update` systems, nothing ordering them against the host focus writer), that
/// rebuild can run AFTER the writer flipped the focus flag, wiping it — and the
/// `Changed` readers run before the writer — so the corners never show. The FIXED
/// ordering (`cursor_focus_*`) keeps the writer after the rebuild and the readers
/// after the writer, so the highlight survives a same-frame republish.
#[test]
fn republish_during_focus_keeps_the_highlight_under_fixed_ordering() {
    let item = Item::PortalGun;

    // Fixed ordering: a republish on the focus frame must NOT drop the highlight.
    let mut fixed = highlight_app_ordered(item, /* writer_first */ true);
    force_republish_and_focus(&mut fixed, item);
    assert_highlight_visible(&mut fixed, item);

    // Do not assert the negative fixture: Bevy may still choose a lucky order in
    // the intentionally under-constrained schedule. The durable invariant is the
    // fixed ordering above.
}

/// Set the cursor onto `item` AND force a host republish the same frame (bump the
/// page version so `rebuild_cube_faces` despawns+respawns the controls), then run
/// one frame — exactly the in-game hover / texture-load / inventory-change churn.
fn force_republish_and_focus(app: &mut App, item: Item) {
    app.world_mut().resource_mut::<KaleidoscopeCursor>().focus = MenuFocus::Item(item.index());
    // Mark the inventory changed so `republish_kaleidoscope_pages` rebuilds.
    app.world_mut().resource_mut::<OwnedItems>().set_changed();
    app.update();
}

/// Set the cursor onto `item`'s focus and run `frames` updates.
fn set_focus_and_step(app: &mut App, item: Item, frames: usize) {
    let focus = MenuFocus::Item(item.index());
    app.world_mut().resource_mut::<KaleidoscopeCursor>().focus = focus;
    for _ in 0..frames {
        app.update();
    }
}

/// Assert the highlight is visible for `item`: (a) its control's
/// `MenuVisualState.focused`, (b) its corners Visible, (c) others' corners Hidden.
fn assert_highlight_visible(app: &mut App, item: Item) {
    let focus = MenuFocus::Item(item.index());
    // Find the control whose action maps to the focused item.
    let active_page = MenuPage::Items;
    let model = SystemMenuModel::build(
        &app.world().resource::<UserSettings>().clone(),
        &RadioSnapshot::default(),
        &DevSnapshot::default(),
    );
    let world = app.world_mut();
    let mut focused_control = None;
    let mut other_control = None;
    let mut q = world.query::<(
        Entity,
        &AmbitionMenuControl<MenuPageAction>,
        &MenuVisualState,
    )>();
    let rows: Vec<(Entity, bool, bool)> = q
        .iter(world)
        .filter_map(|(e, c, vis)| {
            let action = c.action?;
            let f = focus_for_action(action, active_page, &model, None, None);
            Some((e, f == focus, vis.focused))
        })
        .collect();
    for (e, is_focused, vis_focused) in rows {
        if is_focused {
            focused_control = Some((e, vis_focused));
        } else if other_control.is_none() {
            other_control = Some(e);
        }
    }
    let (focused_entity, vis_focused) =
        focused_control.expect("a control maps to the focused item");
    assert!(
        vis_focused,
        "(a) the focused control's MenuVisualState.focused must be true"
    );

    // (b) the focused control's selection corners are VISIBLE.
    let corners_visible = corner_visibilities(world, focused_entity);
    assert!(
        !corners_visible.is_empty(),
        "the focused control must have SelectionCorner children"
    );
    assert!(
        corners_visible.iter().all(|v| *v == Visibility::Visible),
        "(b) focused control's corners must be Visible, got {corners_visible:?}"
    );

    // (c) a non-focused control's corners stay HIDDEN.
    let other = other_control.expect("a non-focused control exists");
    let other_corners = corner_visibilities(world, other);
    assert!(
        other_corners.iter().all(|v| *v == Visibility::Hidden),
        "(c) non-focused control's corners must be Hidden, got {other_corners:?}"
    );
}

/// Collect the `Visibility` of the `SelectionCorner`-style children of a control.
/// Corners are the lib's hidden bracket meshes; identify them as children that are
/// neither text nor icon (they carry a `UiMeshPlane3d` + `Visibility` and no
/// `Text3d`). We match on the lib-set Name "selection corner".
fn corner_visibilities(world: &mut World, control: Entity) -> Vec<Visibility> {
    let children: Vec<Entity> = world
        .get::<Children>(control)
        .map(|c| c.iter().collect())
        .unwrap_or_default();
    children
        .into_iter()
        .filter(|&c| {
            world
                .get::<Name>(c)
                .map(|n| n.as_str() == "selection corner")
                .unwrap_or(false)
        })
        .filter_map(|c| world.get::<Visibility>(c).copied())
        .collect()
}

/// Proves the host's `kaleidoscope_render_needed` gate disables cube rendering
/// for Grid or settled-closed menus, while still allowing visible cube menus.
#[test]
fn render_set_is_gated_off_under_the_grid_backend() {
    #[derive(Resource, Default)]
    struct RenderRan(u32);

    fn build(backend: InventoryUiBackend, menu_visible: bool) -> App {
        let mut app = App::new();
        app.init_resource::<VisualQualityConfirmState>();
        app.init_resource::<InventoryUiBackend>();
        app.init_resource::<RenderRan>();
        *app.world_mut().resource_mut::<InventoryUiBackend>() = backend;
        let mut ui_state = ambition::inventory_ui::InventoryUiState::default();
        ui_state.visible = menu_visible;
        app.insert_resource(ui_state);
        // Exactly the host's gating from `install_kaleidoscope_menu`.
        app.configure_sets(
            Update,
            KaleidoscopeRender.run_if(kaleidoscope_render_needed),
        );
        // A stand-in for the lib's cube render systems (rebuild/animate/fade…),
        // which all live in `KaleidoscopeRender`.
        app.add_systems(
            Update,
            (|mut ran: ResMut<RenderRan>| ran.0 += 1).in_set(KaleidoscopeRender),
        );
        app
    }

    let mut grid = build(InventoryUiBackend::Grid, true);
    grid.update();
    grid.update();
    assert_eq!(
        grid.world().resource::<RenderRan>().0,
        0,
        "cube render set must NOT run when the Grid backend is active"
    );

    let mut cube = build(InventoryUiBackend::LunexKaleidoscope, true);
    cube.update();
    cube.update();
    assert_eq!(
        cube.world().resource::<RenderRan>().0,
        2,
        "cube render set runs every frame while the cube menu is visible"
    );

    // P4b settle early-out: closed + no fade in flight = the set is skipped.
    let mut closed = build(InventoryUiBackend::LunexKaleidoscope, false);
    closed.update();
    closed.update();
    assert_eq!(
        closed.world().resource::<RenderRan>().0,
        0,
        "cube render set must NOT run when the menu is closed and settled"
    );
}

/// Gate 6 (GPT-5.6 review): the focused inventory item's verb resolves correctly
/// — the pure decision the provider publishes. A HELD item focus → "Equip"; a
/// consumable → "Use"; a page-turn / closed menu / absent roster / unowned slot →
/// None (the prompt then uses its generic verb).
#[test]
fn menu_confirm_label_resolves_the_focused_item_verb() {
    let axe_idx = Item::ALL.iter().position(|&i| i == Item::Axe).unwrap();
    let cell_idx = Item::ALL
        .iter()
        .position(|&i| i == Item::HealthCell)
        .unwrap();
    assert!(
        Item::Axe.held_item_id().is_some(),
        "Axe is a held item -> Equip"
    );
    assert!(
        Item::HealthCell.held_item_id().is_none(),
        "HealthCell is a consumable -> Use"
    );

    let mut owned = OwnedItems::default();
    owned.grant(Item::Axe, 1);
    owned.grant(Item::HealthCell, 1);

    assert_eq!(
        menu_confirm_label(true, MenuFocus::Item(axe_idx), Some(&owned)).as_deref(),
        Some("Equip")
    );
    assert_eq!(
        menu_confirm_label(true, MenuFocus::Item(cell_idx), Some(&owned)).as_deref(),
        Some("Use")
    );
    // A page-turn focus carries no item verb (must NOT mislabel slot 0).
    assert_eq!(
        menu_confirm_label(true, MenuFocus::EdgeLeft, Some(&owned)),
        None
    );
    // Closed menu / absent roster / unowned slot -> None.
    assert_eq!(
        menu_confirm_label(false, MenuFocus::Item(axe_idx), Some(&owned)),
        None
    );
    assert_eq!(
        menu_confirm_label(true, MenuFocus::Item(axe_idx), None),
        None
    );
    assert_eq!(
        menu_confirm_label(
            true,
            MenuFocus::Item(cell_idx),
            Some(&OwnedItems::default())
        ),
        None,
        "an unowned slot has no verb"
    );
}

/// Gate 6 end-to-end through the REAL provider path AND its production schedule
/// registration: the app provider (`publish_menu_confirm_prompt`) reads the live
/// cursor + owned items + open overlay and publishes into `MenuConfirmPrompt`; the
/// sim-side `rebuild_control_prompt` folds that into `ControlPrompt.menu_confirm`.
///
/// Crucially, the two systems are wired by the REAL `install_menu_confirm_provider`
/// — the same helper `install_unified_menu_shared` calls — into the SIM schedule's
/// `FeatureViewSync` set, `.before` the reader. The test does NOT hand-chain them
/// in `Update`; a single sim tick must carry the focused Axe's verb all the way to
/// the prompt, which only holds if the writer is ordered before the reader in the
/// same schedule (the cross-schedule staleness bug this registration fixes). We
/// pin the sim schedule to `Update` so one `app.update()` is one deterministic sim
/// tick.
#[test]
fn the_provider_publishes_the_focused_item_verb_into_the_control_prompt() {
    use ambition::platformer::schedule::{SandboxSet, SimScheduleExt};
    use ambition::sim_view::{ControlContextKind, ControlPrompt, MenuConfirmPrompt};
    use bevy::prelude::IntoScheduleConfigs;

    let axe_idx = Item::ALL.iter().position(|&i| i == Item::Axe).unwrap();

    let mut app = App::new();
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.init_state::<GameMode>();
    app.init_resource::<KaleidoscopeCursor>();
    app.init_resource::<ControlPrompt>();
    app.init_resource::<MenuConfirmPrompt>();
    let mut owned = OwnedItems::default();
    owned.grant(Item::Axe, 1);
    app.insert_resource(owned);
    app.insert_resource(ambition::inventory_ui::InventoryUiState {
        visible: true,
        ..Default::default()
    });
    // Focus the Axe and enter a menu (paused) mode.
    app.world_mut()
        .resource_mut::<KaleidoscopeCursor>()
        .mark_keyboard(MenuFocus::Item(axe_idx));
    app.world_mut()
        .resource_mut::<NextState<GameMode>>()
        .set(GameMode::Paused);

    // Pin the sim schedule to `Update` so one update == one sim tick, then wire the
    // reader into the same schedule/set production uses and register the provider
    // through the REAL helper (not a manual `.chain()`).
    app.set_sim_schedule(Update);
    app.add_systems(
        Update,
        ambition::sim_view::rebuild_control_prompt.in_set(SandboxSet::FeatureViewSync),
    );
    super::install_menu_confirm_provider(&mut app);
    app.update();

    let prompt = app.world().resource::<ControlPrompt>();
    assert_eq!(prompt.context, ControlContextKind::Menu);
    assert_eq!(
        prompt.menu_confirm.as_deref(),
        Some("Equip"),
        "the focused Axe's real verb flows app-provider -> MenuConfirmPrompt -> ControlPrompt \
         in ONE sim tick, because install_menu_confirm_provider orders the writer .before the reader"
    );
}

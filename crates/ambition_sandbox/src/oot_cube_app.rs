//! Game-side hookup for the 3D-cube OoT pause menu (#31): adds the lib's reusable
//! cube renderer ([`ambition_inventory_ui::cube::CubeMenuPlugin`]) and feeds it our
//! live 24-item inventory (via [`crate::oot_cube`]). Runtime-toggleable vs the
//! existing Bevy-UI grid through [`InventoryUiBackend`].
//!
//! The cube is pause-gated ([`gate_cube_menu`]): its order-8 `Camera3d` + ring are
//! only active while the inventory is open, so it never clears the screen to black
//! during play. Routing nav/selection input to it is the next step — see
//! `dev/journals/oot-cube-integration-plan.md`.

use ambition_inventory_ui::cube::{CubeMenuConfig, CubeMenuPlugin};
use ambition_inventory_ui::{
    ActiveMenuPages, AmbitionInventoryUiPlugin, AmbitionMenuControl,
};
use bevy::prelude::*;

use crate::input::MenuControlFrame;
use crate::items::{Item, OwnedItems, ITEM_GRID_COLS, ITEM_GRID_ROWS};
use crate::oot_cube::{
    build_inventory_pages, system_rows, CubeAction, CubeFocus, CubePage, SystemCategory,
    SystemOption, SystemRow,
};
use crate::oot_menu::input::{dispatch_item_confirm, MenuEffectManaQuery, MenuEffectPlayers};
use crate::persistence::settings::{AudioSettings, UserSettings};
use crate::player::PlayerHealRequested;

/// Which inventory frontend renders. Runtime toggle (both compiled in); defaults to
/// the 3D `Cube` (#31), with `\` flipping to the proven Bevy-UI `Grid` (see
/// [`toggle_inventory_backend`]).
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum InventoryUiBackend {
    Grid,
    #[default]
    Cube,
}

/// Peak opacity of the readability dim-scrim (black) when the cube is fully open.
/// The game runs the cube as an Option-1 overlay (cube camera clears `None`, so the
/// live world shows through); that busy world wrecks the cube text contrast. A
/// full-screen translucent-black `bevy_ui` Node on the order-0 `Camera2d` renders
/// UNDER the order-8 cube but OVER the world, dimming the world so the cube text
/// reads. The demo doesn't need this (it has a dark `ClearColor`).
const SCRIM_PEAK_ALPHA: f32 = 0.7;

/// Marks the full-screen readability dim-scrim node (game overlay only).
#[derive(Component)]
struct CubeScrim;

/// Wire the 3D-cube menu into the app: the lib plugins + our page-feed system.
pub fn install_cube_menu(app: &mut App) {
    // The game uses Bevy picking on the cube controls AND draws its own real L/R
    // edge buttons (see `oot_cube::add_edge_buttons`), so it inserts its own
    // `CubeMenuConfig` (lib overlay defaults, but `draw_nav_arrows = false` so the
    // decorative arrows don't double-draw and `pickable_controls = true` so
    // `Pointer<*>` events fire) BEFORE the plugin (which only inserts a default
    // if the host hasn't).
    if !app.world().contains_resource::<CubeMenuConfig>() {
        app.insert_resource(CubeMenuConfig {
            draw_nav_arrows: false,
            pickable_controls: true,
            ..Default::default()
        });
    }
    app.init_resource::<InventoryUiBackend>()
        .init_resource::<ActiveMenuPages<CubePage, CubeAction>>()
        .init_resource::<CubeCursor>()
        .init_resource::<CubeSystemNav>()
        .add_plugins(AmbitionInventoryUiPlugin)
        .add_plugins(CubeMenuPlugin::<CubePage, CubeAction>::default())
        .add_systems(Startup, spawn_cube_scrim)
        .add_systems(
            Update,
            (
                // Fix 3: when Cube is the backend, the game's menu-open inputs
                // (pause/Esc, inventory, map) open the cube on the matching page
                // instead of the old Bevy-UI menus. Runs before nav so the page is
                // set the same frame the cube opens.
                cube_menu_open_routing,
                // Nav first (mutates the cursor), then republish (reads the cursor +
                // inventory) so the highlight + detail panel reflect this frame's move.
                cube_focus_nav,
                republish_cube_pages,
                gate_cube_menu,
                toggle_inventory_backend,
                retarget_cube_scrim,
                fade_cube_scrim,
            )
                .chain(),
        )
        .add_observer(cube_pointer_over)
        .add_observer(cube_pointer_click);
}

/// Which input source currently owns the cube cursor. Mirrors the grid's
/// [`crate::ui_nav::MenuFocusOwner`]: keyboard/gamepad nav claims focus and keeps
/// it until the pointer moves to a DIFFERENT control. A stationary hover must not
/// keep reasserting itself over newer directional navigation (the "can't move away
/// from the hovered option" bug).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
enum FocusSource {
    #[default]
    Keyboard,
    Pointer,
}

/// The directional-focus cursor for the items page: which item slot or edge
/// (page-turn) button the cursor sits on. Mirrors the demo's selection state
/// (`MockDemo::selected`). [`cube_focus_nav`] moves it with `move_spatial`-style
/// rules; [`republish_cube_pages`] republishes the page model whenever its
/// SEMANTIC focus changes so the highlight + detail panel follow it.
#[derive(Resource, Default)]
struct CubeCursor {
    focus: CubeFocus,
    /// Which input source last moved the cursor (keyboard nav vs pointer hover).
    owner: FocusSource,
    /// The last focus the POINTER hovered. A parked mouse re-fires `Pointer<Over>`
    /// every model rebuild (new entities spawn under the cursor); gating on this
    /// means a stationary hover over the same logical focus is a no-op, so it can't
    /// (a) loop the rebuild or (b) override a later keyboard move.
    last_pointer_focus: Option<CubeFocus>,
}

impl CubeCursor {
    /// Keyboard/gamepad nav took the cursor to `focus` (claims ownership).
    fn mark_keyboard(&mut self, focus: CubeFocus) {
        self.focus = focus;
        self.owner = FocusSource::Keyboard;
    }
}

/// Drill-down state for the System face. `None` = the top-level category list is
/// shown (Video / Audio / Controls / Gameplay + Close Menu); `Some(category)` = the
/// open category's option rows + a Back row are shown. Mirrors the Bevy-UI pause
/// menu's settings page stack. `republish_cube_pages` feeds this into
/// `build_system_page`, and changing it republishes (the System cursor resets to
/// row 0). B0002-safe: only `cube_focus_nav` / `cube_pointer_click` mutate it (both
/// `ResMut`); `republish_cube_pages` reads it as `Res`.
#[derive(Resource, Default)]
struct CubeSystemNav {
    open_category: Option<SystemCategory>,
}

/// Spawn the readability dim-scrim node (full-screen, starts fully transparent).
///
/// The scrim DIMS THE WORLD, so it must render BEHIND the order-8 cube. Since the
/// default UI camera is now the order-9 [`FrontHudCamera`] (which draws in front of
/// the cube), the scrim is explicitly retargeted onto the order-0 main camera via
/// [`retarget_cube_scrim`] (the `MainCameraEntity` resource isn't guaranteed to
/// exist yet at this Startup point, so the target is attached from an Update guard).
/// [`fade_cube_scrim`] drives its alpha.
fn spawn_cube_scrim(mut commands: Commands) {
    commands.spawn((
        CubeScrim,
        Name::new("Cube readability scrim"),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
        // Never eat clicks meant for the world/cube; purely a visual dimmer.
        GlobalZIndex(-1),
        Pickable::IGNORE,
    ));
}

/// Retarget the dim-scrim onto the order-0 main camera so it renders BEHIND the cube.
///
/// The default UI camera is the order-9 front HUD camera (so the HUD draws in front
/// of the cube); without this retarget the scrim would inherit that default and dim
/// the cube itself. Runs once, as soon as both the scrim and the `MainCameraEntity`
/// resource exist (Startup ordering between them is not guaranteed, so this Update
/// guard does it on the first frame both are present). `Option<Res<_>>` keeps it
/// B0002-safe and never panics on an uninserted resource.
fn retarget_cube_scrim(
    mut commands: Commands,
    main_camera: Option<Res<crate::runtime::camera_layers::MainCameraEntity>>,
    scrim: Query<Entity, (With<CubeScrim>, Without<UiTargetCamera>)>,
    mut done: Local<bool>,
) {
    if *done {
        return;
    }
    let Some(main_camera) = main_camera else {
        return;
    };
    let mut any = false;
    for entity in &scrim {
        commands.entity(entity).insert(UiTargetCamera(main_camera.0));
        any = true;
    }
    if any {
        *done = true;
    }
}

/// Fade the dim-scrim's alpha with the cube's eased open `amount`, so the world
/// dims in/out exactly with the fold. Fully transparent when the cube is shut.
fn fade_cube_scrim(
    open_state: Res<ambition_inventory_ui::cube::CubeOpenState>,
    mut scrim: Query<&mut BackgroundColor, With<CubeScrim>>,
) {
    let alpha = open_state.amount.clamp(0.0, 1.0) * SCRIM_PEAK_ALPHA;
    for mut bg in &mut scrim {
        bg.0 = Color::srgba(0.0, 0.0, 0.0, alpha);
    }
}

/// Directional focus navigation for the cube (keyboard / gamepad), porting the
/// demo's `MockDemo::move_spatial` (`crates/ambition_mock_demo/src/app/state.rs`).
/// The cursor lives on the [`CubeCursor`] resource as a [`CubeFocus`], and the
/// 6×4 item grid is flanked by two edge (page-turn) buttons. The exact rules
/// (user spec) honoured here:
///
/// 1. From a side arrow, moving toward centre lands on the nearest ITEM in the
///    adjacent column — not across the screen to the other arrow.
/// 2. UP/DOWN never reach a side arrow (vertical stays within the item columns).
/// 3. On a side arrow, moving further outward ROTATES to that page (same as a
///    click).
/// 4. From the leftmost / rightmost column, LEFT/RIGHT moves onto the arrow.
///
/// `select` on an item dispatches its `CubeAction`; `select` on an arrow turns the
/// page; `back` closes the menu. The republish runs after this in the chain.
#[allow(clippy::too_many_arguments)]
fn cube_focus_nav(
    backend: Res<InventoryUiBackend>,
    menu: Res<MenuControlFrame>,
    mut cursor: ResMut<CubeCursor>,
    mut system_nav: ResMut<CubeSystemNav>,
    mut pages: ResMut<ActiveMenuPages<CubePage, CubeAction>>,
    // Single mutable access to the overlay state — also read `.visible` from it (a
    // separate `Res<InventoryUiState>` would be a B0002 conflict with this `ResMut`).
    mut overlay: ResMut<crate::inventory::InventoryUiState>,
    mut owned: ResMut<OwnedItems>,
    mut settings: ResMut<UserSettings>,
    mut commands: Commands,
    mut players: MenuEffectPlayers,
    mut mana_q: MenuEffectManaQuery,
    mut heals: MessageWriter<PlayerHealRequested>,
) {
    if *backend != InventoryUiBackend::Cube || !overlay.visible {
        return;
    }
    let Some(active_page) = pages.active else {
        return;
    };

    // Directional intent (one step; the menu frame already debounces repeat).
    let dx = (menu.right as i32) - (menu.left as i32);
    let dy = (menu.down as i32) - (menu.up as i32);

    // Fix 2: the L/R shoulder bumpers turn the page DIRECTLY (same target as the
    // on-screen L/R edge buttons), independent of the arrow/d-pad item cursor. Left
    // bumper rotates to the viewer-left page, right bumper to the viewer-right page.
    // Handled before the per-face nav so a bumper press always rotates regardless of
    // where the item cursor sits. The cursor lands on the new page's back-edge button.
    let bump = (menu.page_right as i32) - (menu.page_left as i32);
    if bump < 0 {
        turn_page_seeded(&mut pages, &mut cursor, active_page.on_viewer_left());
        return;
    } else if bump > 0 {
        turn_page_seeded(&mut pages, &mut cursor, active_page.on_viewer_right());
        return;
    }

    // The System face is an interactive option list: UP/DOWN move the cursor
    // between rows, LEFT/RIGHT at the column edges turn the page (or step a
    // value), and SELECT applies the focused option.
    if active_page == CubePage::System {
        system_focus_nav(
            &menu, dx, dy, &mut cursor, &mut system_nav, &mut pages, &mut overlay, &mut settings,
            active_page, &mut owned, &mut commands, &mut players, &mut mana_q, &mut heals,
        );
        return;
    }

    // Other non-items faces (Map / Quest placeholders) respond to horizontal page
    // turns; arrows rotate, landing the cursor on the new page's back-edge button
    // (Fix 1). The L/R bumpers (Fix 2) are already handled above for every face.
    if active_page != CubePage::Items {
        if dx < 0 {
            turn_page_seeded(&mut pages, &mut cursor, active_page.on_viewer_left());
        } else if dx > 0 {
            turn_page_seeded(&mut pages, &mut cursor, active_page.on_viewer_right());
        }
        if menu.select {
            // The only selectable controls on a placeholder are the edge buttons.
            match cursor.focus {
                CubeFocus::EdgeLeft => {
                    turn_page_seeded(&mut pages, &mut cursor, active_page.on_viewer_left())
                }
                CubeFocus::EdgeRight => {
                    turn_page_seeded(&mut pages, &mut cursor, active_page.on_viewer_right())
                }
                _ => {}
            }
        }
        if menu.back {
            overlay.visible = false;
        }
        return;
    }

    if dx != 0 || dy != 0 {
        match move_spatial(cursor.focus, dx, dy, active_page) {
            SpatialMove::Focus(next) => cursor.mark_keyboard(next),
            SpatialMove::TurnLeft => {
                turn_page(&mut pages, active_page.on_viewer_left());
                // Land the cursor on the new face's right arrow (so pressing back
                // toward centre re-enters the grid) — demo's turn_page_from_edge.
                cursor.mark_keyboard(CubeFocus::EdgeRight);
            }
            SpatialMove::TurnRight => {
                turn_page(&mut pages, active_page.on_viewer_right());
                cursor.mark_keyboard(CubeFocus::EdgeLeft);
            }
        }
    }

    if menu.back {
        overlay.visible = false;
        return;
    }

    if menu.select {
        let action = match cursor.focus {
            CubeFocus::EdgeLeft => Some(CubeAction::ChangePage(active_page.on_viewer_left())),
            CubeFocus::EdgeRight => Some(CubeAction::ChangePage(active_page.on_viewer_right())),
            CubeFocus::Item(idx) => owned_item_action(&owned, idx),
            // System focus is handled by the System branch above; never reached here.
            CubeFocus::System(_) => None,
        };
        if let Some(action) = action {
            let mut close_menu = false;
            dispatch_cube_action(
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
            );
            if close_menu {
                overlay.visible = false;
            }
        }
    }
}

/// Directional navigation + select for the System face. UP/DOWN move the cursor
/// over [`SystemOption::ALL`]; from the leftmost/rightmost edge LEFT/RIGHT turns
/// the page; SELECT applies the focused option (volume/zoom rows also respond to
/// LEFT/RIGHT to step). `back` closes the menu. Mutations go through
/// [`apply_system_option`] so persistence stays in one place.
#[allow(clippy::too_many_arguments)]
fn system_focus_nav(
    menu: &MenuControlFrame,
    dx: i32,
    dy: i32,
    cursor: &mut CubeCursor,
    system_nav: &mut CubeSystemNav,
    pages: &mut ActiveMenuPages<CubePage, CubeAction>,
    overlay: &mut crate::inventory::InventoryUiState,
    settings: &mut UserSettings,
    active_page: CubePage,
    owned: &mut OwnedItems,
    commands: &mut Commands,
    players: &mut MenuEffectPlayers,
    mana_q: &mut MenuEffectManaQuery,
    heals: &mut MessageWriter<PlayerHealRequested>,
) {
    // The rows shown for the current drill-down state: categories (+ Close Menu) at
    // the top level, or the open category's options + a Back row.
    let rows = system_rows(system_nav.open_category);
    let count = rows.len() as i32;
    // Normalise the cursor onto a System row (it may arrive as an items/edge focus
    // after a page turn).
    let mut row = match cursor.focus {
        CubeFocus::System(idx) => (idx as i32).min(count - 1),
        _ => 0,
    };

    if dy != 0 {
        row = (row + dy).clamp(0, count - 1);
        cursor.mark_keyboard(CubeFocus::System(row as usize));
    }

    let current = rows[row.max(0).min(count - 1) as usize];

    if dx != 0 {
        // LEFT/RIGHT step value OPTION rows in place; otherwise turn the page.
        let value_option = match current {
            SystemRow::Option(o) if is_value_option(o) => Some(o),
            _ => None,
        };
        if let Some(option) = value_option {
            apply_system_option_step(option, dx, settings);
        } else if dx < 0 {
            turn_page(pages, active_page.on_viewer_left());
            cursor.mark_keyboard(CubeFocus::System(0));
        } else {
            turn_page(pages, active_page.on_viewer_right());
            cursor.mark_keyboard(CubeFocus::System(0));
        }
    }

    if menu.back {
        // Inside a category, Back drills OUT to the category list; at the top level
        // Back closes the menu (matching the items face).
        if system_nav.open_category.is_some() {
            close_system_category(system_nav, cursor);
        } else {
            overlay.visible = false;
        }
        return;
    }

    if menu.select {
        if let Some(action) = system_row_action_for(current) {
            let mut close_menu = false;
            dispatch_cube_action(
                action,
                pages,
                system_nav,
                cursor,
                owned,
                settings,
                &mut close_menu,
                commands,
                players,
                mana_q,
                heals,
            );
            if close_menu {
                overlay.visible = false;
            }
        }
    }
}

/// True for OPTION rows whose value steps with LEFT/RIGHT (volume + camera zoom).
fn is_value_option(option: SystemOption) -> bool {
    matches!(
        option,
        SystemOption::CycleMasterVolume
            | SystemOption::CycleMusicVolume
            | SystemOption::CycleSfxVolume
            | SystemOption::CycleCameraZoom
    )
}

/// The `CubeAction` a System row dispatches on select (categories drill in, options
/// apply, Back drills out).
fn system_row_action_for(row: SystemRow) -> Option<CubeAction> {
    match row {
        SystemRow::Category(c) => Some(CubeAction::OpenSystemCategory(c)),
        SystemRow::Option(o) => Some(CubeAction::System(o)),
        SystemRow::Back => Some(CubeAction::CloseSystemCategory),
    }
}

/// Drill OUT of an open System category back to the category list, resetting the
/// cursor to the first (category) row so the highlight lands sensibly.
fn close_system_category(system_nav: &mut CubeSystemNav, cursor: &mut CubeCursor) {
    system_nav.open_category = None;
    cursor.mark_keyboard(CubeFocus::System(0));
}

/// Apply a signed LEFT/RIGHT step to a value-style System option (volume up/down,
/// camera-zoom prev/next). Toggle/close rows ignore stepping (they only respond
/// to SELECT). Persistence is automatic via `UserSettings` change detection.
fn apply_system_option_step(option: SystemOption, dx: i32, settings: &mut UserSettings) {
    match option {
        SystemOption::CycleMasterVolume => {
            settings.audio.nudge_master(step_sign(dx) * AudioSettings::VOLUME_STEP);
        }
        SystemOption::CycleMusicVolume => {
            settings.audio.nudge_music(step_sign(dx) * AudioSettings::VOLUME_STEP);
        }
        SystemOption::CycleSfxVolume => {
            settings.audio.nudge_sfx(step_sign(dx) * AudioSettings::VOLUME_STEP);
        }
        SystemOption::CycleCameraZoom => {
            settings.video.camera_zoom = if dx < 0 {
                settings.video.camera_zoom.prev()
            } else {
                settings.video.camera_zoom.next()
            };
        }
        _ => {}
    }
}

fn step_sign(dx: i32) -> f32 {
    if dx < 0 {
        -1.0
    } else {
        1.0
    }
}

/// Outcome of a spatial cursor move on the items page.
enum SpatialMove {
    /// The cursor moves to a new focus (item or arrow) on the same page.
    Focus(CubeFocus),
    /// The cursor was on the left arrow and pressed further left → rotate left.
    TurnLeft,
    /// The cursor was on the right arrow and pressed further right → rotate right.
    TurnRight,
}

/// Port of the demo's `MockDemo::move_spatial` for the items grid + flanking
/// arrows. Pure (no ECS) so it's unit-testable and easy to reason about. See
/// [`cube_focus_nav`] for the rule list.
fn move_spatial(focus: CubeFocus, dx: i32, dy: i32, _page: CubePage) -> SpatialMove {
    let cols = ITEM_GRID_COLS as i32;
    let rows = ITEM_GRID_ROWS as i32;

    // Rule 3: on an arrow, moving further OUTWARD rotates the page; UP/DOWN never
    // reach/leave an arrow (rule 2); moving INWARD enters the adjacent column.
    match focus {
        CubeFocus::EdgeLeft => {
            if dx < 0 {
                return SpatialMove::TurnLeft;
            }
            if dx > 0 {
                // Rule 1: enter the LEFTMOST item column (col 0), keep the row band.
                return SpatialMove::Focus(CubeFocus::Item(0));
            }
            // Up/Down on an arrow: stay put (rule 2).
            return SpatialMove::Focus(focus);
        }
        CubeFocus::EdgeRight => {
            if dx > 0 {
                return SpatialMove::TurnRight;
            }
            if dx < 0 {
                // Rule 1: enter the RIGHTMOST item column.
                return SpatialMove::Focus(CubeFocus::Item((cols - 1) as usize));
            }
            return SpatialMove::Focus(focus);
        }
        CubeFocus::Item(idx) => {
            let idx = idx as i32;
            let row = idx / cols;
            let col = idx % cols;
            // Rule 4: stepping off the left/right column lands on the arrow.
            if dx < 0 && col == 0 {
                return SpatialMove::Focus(CubeFocus::EdgeLeft);
            }
            if dx > 0 && col == cols - 1 {
                return SpatialMove::Focus(CubeFocus::EdgeRight);
            }
            // Rule 2: UP/DOWN stays within the columns (never reaches an arrow).
            let next_col = (col + dx).clamp(0, cols - 1);
            let next_row = (row + dy).clamp(0, rows - 1);
            SpatialMove::Focus(CubeFocus::Item((next_row * cols + next_col) as usize))
        }
        // `move_spatial` is only invoked on the Items face; a System focus here
        // would be a logic error — re-enter the grid at slot 0 to stay safe.
        CubeFocus::System(_) => SpatialMove::Focus(CubeFocus::Item(0)),
    }
}

/// The `CubeAction` for an owned item slot, or `None` if the slot is empty/unowned
/// (so confirming an empty cell is a no-op, matching the grid backend).
fn owned_item_action(owned: &OwnedItems, idx: usize) -> Option<CubeAction> {
    let item = Item::from_index(idx)?;
    if !owned.has(item) {
        return None;
    }
    Some(if item.held_item_id().is_some() {
        CubeAction::Equip(item)
    } else {
        CubeAction::Use(item)
    })
}

/// The edge-button focus on `to` that turns BACK toward `from` (Fix 1). After a page
/// turn the cursor lands here, so the arriving control is highlighted and an immediate
/// rotate/select returns to the page we came from. On `to`, the LEFT edge button
/// targets `to.on_viewer_left()` and the RIGHT targets `to.on_viewer_right()`; we pick
/// whichever points back at `from`. When `from` is unknown (first open) we default to
/// the left edge button so there is always a highlighted control.
fn back_edge_focus(from: Option<CubePage>, to: CubePage) -> CubeFocus {
    match from {
        Some(from) if to.on_viewer_right() == from => CubeFocus::EdgeRight,
        Some(from) if to.on_viewer_left() == from => CubeFocus::EdgeLeft,
        _ => CubeFocus::EdgeLeft,
    }
}

/// Set the active page (the lib rotates that face to the camera), landing the cursor
/// on the new page's back-edge button (Fix 1) via [`back_edge_focus`].
fn turn_page_seeded(
    pages: &mut ActiveMenuPages<CubePage, CubeAction>,
    cursor: &mut CubeCursor,
    page: CubePage,
) {
    let from = pages.active;
    turn_page(pages, page);
    cursor.mark_keyboard(back_edge_focus(from, page));
}

/// Set the active page (the lib rotates that face to the camera).
fn turn_page(pages: &mut ActiveMenuPages<CubePage, CubeAction>, page: CubePage) {
    if pages.active != Some(page) {
        pages.active = Some(page);
        info!("cube page \u{2192} {:?}", page);
    }
}

/// Dispatch a [`CubeAction`]. Item Equip/Use reuse the grid's shared
/// [`dispatch_item_confirm`] (no portal/equip/heal duplication); page-change sets
/// the active page so the lib rotates that face to the camera.
#[allow(clippy::too_many_arguments)]
fn dispatch_cube_action(
    action: CubeAction,
    pages: &mut ActiveMenuPages<CubePage, CubeAction>,
    system_nav: &mut CubeSystemNav,
    cursor: &mut CubeCursor,
    owned: &mut OwnedItems,
    settings: &mut UserSettings,
    close_menu: &mut bool,
    commands: &mut Commands,
    players: &mut MenuEffectPlayers,
    mana_q: &mut MenuEffectManaQuery,
    heals: &mut MessageWriter<PlayerHealRequested>,
) {
    match action {
        CubeAction::Equip(item) | CubeAction::Use(item) => {
            let decided = dispatch_item_confirm(item, owned, commands, players, mana_q, heals);
            info!("cube action: {:?} \u{2192} {:?}", item, decided);
        }
        CubeAction::ChangePage(page) => {
            let from = pages.active;
            pages.active = Some(page);
            // Fix 1: land the cursor on the new page's "back" edge button — the one
            // that turns BACK toward the page we came from — so an immediate select /
            // rotate goes home and the arriving control is highlighted.
            cursor.mark_keyboard(back_edge_focus(from, page));
            info!("cube page \u{2192} {:?}", page);
        }
        CubeAction::System(option) => {
            apply_system_option(option, settings, close_menu);
        }
        CubeAction::OpenSystemCategory(category) => {
            // Drill INTO a category: show its option rows, land the cursor on the
            // first option. The republish picks up the new drill state + cursor.
            system_nav.open_category = Some(category);
            cursor.mark_keyboard(CubeFocus::System(0));
            info!("cube system category \u{2192} {:?}", category);
        }
        CubeAction::CloseSystemCategory => {
            close_system_category(system_nav, cursor);
            info!("cube system category \u{2192} (list)");
        }
    }
}

/// Apply a System-face option by mutating `UserSettings` (toggles flip the bool;
/// volume rows step via the audio settings' own `nudge_*` clamping helpers; the
/// camera-zoom row cycles the preset enum). Persistence is NOT re-implemented
/// here: the existing `save_settings_on_change` system writes `settings.ron`
/// whenever `UserSettings` changes, so mutating the resource is the whole job.
/// `CloseMenu` raises `close_menu` for the caller to fold back into the overlay.
fn apply_system_option(option: SystemOption, settings: &mut UserSettings, close_menu: &mut bool) {
    match option {
        SystemOption::ToggleFps => settings.video.show_fps = !settings.video.show_fps,
        SystemOption::ToggleDebugHud => {
            settings.gameplay.debug_hud_visible = !settings.gameplay.debug_hud_visible;
        }
        SystemOption::ToggleQuestHud => {
            settings.gameplay.quest_hud_visible = !settings.gameplay.quest_hud_visible;
        }
        SystemOption::ToggleTouchControls => {
            settings.controls.touch_controls_visible = !settings.controls.touch_controls_visible;
        }
        SystemOption::ToggleMute => settings.audio.toggle_mute(),
        // Volume rows confirm-cycle UP by one step (wrapping at the ceiling), so a
        // single select/tap keeps stepping the value the way a slider would. The
        // audio settings' own `nudge_*` helpers do the clamping + mute coupling.
        SystemOption::CycleMasterVolume => {
            settings.audio.master_volume = step_volume(settings.audio.master_volume);
            // Raising master while muted unmutes, matching `nudge_master`.
            if settings.audio.master_volume > 0.0 && settings.audio.muted {
                settings.audio.muted = false;
            }
        }
        SystemOption::CycleMusicVolume => {
            settings.audio.music_volume = step_volume(settings.audio.music_volume);
        }
        SystemOption::CycleSfxVolume => {
            settings.audio.sfx_volume = step_volume(settings.audio.sfx_volume);
        }
        SystemOption::CycleCameraZoom => {
            settings.video.camera_zoom = settings.video.camera_zoom.next();
        }
        SystemOption::CloseMenu => *close_menu = true,
    }
    info!("cube system option: {:?}", option);
}

/// Step a 0..=1 volume up by one `AudioSettings::VOLUME_STEP`, wrapping back to
/// 0 once it passes the ceiling. Single-select "cycle" behaviour for the System
/// face's volume rows (no L/R needed).
fn step_volume(current: f32) -> f32 {
    let step = AudioSettings::VOLUME_STEP;
    let next = current + step;
    if next > 1.0 + step * 0.5 {
        0.0
    } else {
        next.clamp(0.0, 1.0)
    }
}

/// Map a control's `CubeAction` back to the cursor focus it represents, so pointer
/// hover/click and the keyboard cursor share one model. `Equip`/`Use` carry the
/// item (→ its slot); `ChangePage` is an edge arrow — left vs right is decided by
/// whether its target is the active page's viewer-left neighbour.
fn focus_for_action(
    action: CubeAction,
    active_page: CubePage,
    open_category: Option<SystemCategory>,
) -> CubeFocus {
    // System rows are positional: the focus index is the action's row in the
    // currently-displayed System row list (categories+Close, or the open category's
    // options+Back), so hover/click and the keyboard cursor agree on the row.
    let system_row = |want: SystemRow| {
        let idx = system_rows(open_category)
            .iter()
            .position(|r| *r == want)
            .unwrap_or(0);
        CubeFocus::System(idx)
    };
    match action {
        CubeAction::Equip(item) | CubeAction::Use(item) => CubeFocus::Item(item.index()),
        CubeAction::ChangePage(target) => {
            if target == active_page.on_viewer_left() {
                CubeFocus::EdgeLeft
            } else {
                CubeFocus::EdgeRight
            }
        }
        CubeAction::System(option) => system_row(SystemRow::Option(option)),
        CubeAction::OpenSystemCategory(category) => system_row(SystemRow::Category(category)),
        CubeAction::CloseSystemCategory => system_row(SystemRow::Back),
    }
}

/// Pointer hover (mouse/touch) over a cube control: move the focus cursor to it —
/// but ONLY on a genuine pointer move to a DIFFERENT control. Bevy picking fires
/// this for mouse AND touch uniformly.
///
/// Two guards (both essential), mirroring the grid's `MenuFocusState`:
///
/// 1. **Semantic dedup.** Every model rebuild despawns/respawns the controls, so a
///    parked mouse re-fires `Pointer<Over>` on a NEW entity that maps to the SAME
///    logical [`CubeFocus`]. We compare the hovered focus against `last_pointer_focus`
///    and bail when unchanged → no `CubeCursor` write → no rebuild → the
///    "rebuilding 4 faces" loop is broken.
/// 2. **Pointer-vs-keyboard ownership.** When the hovered focus equals the last one
///    the pointer reported, the mouse hasn't moved; we leave the cursor alone even
///    if keyboard nav has since taken it elsewhere. The pointer only re-claims the
///    cursor when it moves onto a genuinely different control. This fixes "can't
///    move away from the hovered option."
fn cube_pointer_over(
    over: On<Pointer<Over>>,
    controls: Query<&AmbitionMenuControl<CubeAction>>,
    pages: Res<ActiveMenuPages<CubePage, CubeAction>>,
    system_nav: Res<CubeSystemNav>,
    mut cursor: ResMut<CubeCursor>,
) {
    let Some(active_page) = pages.active else {
        return;
    };
    if let Ok(control) = controls.get(over.entity) {
        if let Some(action) = control.action {
            let next = focus_for_action(action, active_page, system_nav.open_category);
            // The pointer hasn't moved to a new control (same logical focus, just a
            // freshly-rebuilt entity under a parked mouse): do nothing. This is the
            // single guard that breaks the rebuild loop AND prevents the parked
            // mouse from locking the cursor against keyboard nav.
            if cursor.last_pointer_focus == Some(next) {
                return;
            }
            cursor.last_pointer_focus = Some(next);
            if cursor.focus != next {
                cursor.focus = next;
                cursor.owner = FocusSource::Pointer;
            }
        }
    }
}

/// Pointer click (mouse/touch) on a cube control: dispatch its `CubeAction`.
#[allow(clippy::too_many_arguments)]
fn cube_pointer_click(
    click: On<Pointer<Click>>,
    backend: Res<InventoryUiBackend>,
    mut ui_state: Option<ResMut<crate::inventory::InventoryUiState>>,
    controls: Query<&AmbitionMenuControl<CubeAction>>,
    mut pages: ResMut<ActiveMenuPages<CubePage, CubeAction>>,
    mut cursor: ResMut<CubeCursor>,
    mut system_nav: ResMut<CubeSystemNav>,
    mut owned: ResMut<OwnedItems>,
    mut settings: ResMut<UserSettings>,
    mut commands: Commands,
    mut players: MenuEffectPlayers,
    mut mana_q: MenuEffectManaQuery,
    mut heals: MessageWriter<PlayerHealRequested>,
) {
    let open = ui_state.as_deref().map(|s| s.visible).unwrap_or(false);
    if *backend != InventoryUiBackend::Cube || !open {
        return;
    }
    if let Ok(control) = controls.get(click.entity) {
        if let Some(action) = control.action {
            if let Some(active_page) = pages.active {
                let next = focus_for_action(action, active_page, system_nav.open_category);
                cursor.focus = next;
                cursor.owner = FocusSource::Pointer;
                cursor.last_pointer_focus = Some(next);
            }
            let mut close_menu = false;
            dispatch_cube_action(
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
            );
            if close_menu {
                if let Some(ui_state) = ui_state.as_deref_mut() {
                    ui_state.visible = false;
                }
            }
        }
    }
}

/// Fix 3: route the game's menu-open inputs to the CUBE when it is the active
/// backend, opening it on the page that matches the requested menu:
///
/// * pause / `Esc` (`menu.start`) → open on [`CubePage::System`] (replacing the old
///   pause/system menu); pressing it again while the cube is open CLOSES the cube.
/// * inventory key (`menu.inventory`) → open on [`CubePage::Items`].
/// * map key (`menu.map`) → open on [`CubePage::Map`].
///
/// Opening pauses the sim (`GameMode::Paused`) and raises `InventoryUiState.visible`,
/// exactly like the inventory open path — which makes the existing pause-menu UI
/// auto-suppress (`Paused && !inventory.visible`). The old `pause_menu_toggle` and
/// `handle_map_menu_hotkeys` are gated to no-op under the Cube backend (see their
/// `cube_backend_active` guards), so nothing double-fires the `GameMode` toggle and
/// the map panel never opens behind the cube.
///
/// `Esc`-to-close is owned HERE (not by `cube_focus_nav`'s `menu.back`) so the close
/// also restores `GameMode::Playing`; the routing runs before `cube_focus_nav`, and
/// consuming the open/close intent keeps the two from fighting over the same frame.
#[cfg(feature = "input")]
#[allow(clippy::too_many_arguments)]
fn cube_menu_open_routing(
    backend: Res<InventoryUiBackend>,
    menu: Res<MenuControlFrame>,
    mut overlay: ResMut<crate::inventory::InventoryUiState>,
    mode: Res<State<crate::runtime::game_mode::GameMode>>,
    mut next_mode: ResMut<NextState<crate::runtime::game_mode::GameMode>>,
    mut pages: ResMut<ActiveMenuPages<CubePage, CubeAction>>,
    mut cursor: ResMut<CubeCursor>,
    mut system_nav: ResMut<CubeSystemNav>,
    mut map: ResMut<crate::map_menu::MapMenuState>,
) {
    use crate::runtime::game_mode::GameMode;
    if *backend != InventoryUiBackend::Cube {
        return;
    }

    // pause / Esc: toggle the cube on the System page.
    if menu.start {
        if overlay.visible {
            close_cube_menu(&mut overlay, mode.get(), &mut next_mode);
        } else if matches!(mode.get(), GameMode::Playing | GameMode::Paused) {
            open_cube_menu(
                CubePage::System,
                &mut overlay,
                mode.get(),
                &mut next_mode,
                &mut pages,
                &mut cursor,
                &mut system_nav,
                &mut map,
            );
        }
        return;
    }

    // inventory key: the shared open/close TOGGLE stays in `oot_menu_input` (it raises
    // `visible` + pauses for both backends); here we only point the cube at the Items
    // page + seed the cursor whenever that key fires. Closing is handled there too —
    // when the key closes the overlay this just sets a page that won't be shown.
    if menu.inventory {
        pages.active = Some(CubePage::Items);
        system_nav.open_category = None;
        cursor.mark_keyboard(CubeFocus::Item(0));
        map.open = false;
        return;
    }

    // map key: open on the Map page (suppressing the standalone map panel).
    if menu.map && matches!(mode.get(), GameMode::Playing | GameMode::Paused) {
        if overlay.visible {
            pages.active = Some(CubePage::Map);
            cursor.mark_keyboard(CubeFocus::EdgeLeft);
        } else {
            open_cube_menu(
                CubePage::Map,
                &mut overlay,
                mode.get(),
                &mut next_mode,
                &mut pages,
                &mut cursor,
                &mut system_nav,
                &mut map,
            );
        }
    }
}

/// Open the cube overlay on `page`, pausing the sim and seeding the cursor. Mirrors
/// the inventory open path (`oot_menu_input`): raise `visible`, switch to
/// `GameMode::Paused` when coming from gameplay, and make sure the standalone map
/// panel stays shut so it can't render behind the cube.
#[cfg(feature = "input")]
#[allow(clippy::too_many_arguments)]
fn open_cube_menu(
    page: CubePage,
    overlay: &mut crate::inventory::InventoryUiState,
    mode: &crate::runtime::game_mode::GameMode,
    next_mode: &mut NextState<crate::runtime::game_mode::GameMode>,
    pages: &mut ActiveMenuPages<CubePage, CubeAction>,
    cursor: &mut CubeCursor,
    system_nav: &mut CubeSystemNav,
    map: &mut crate::map_menu::MapMenuState,
) {
    use crate::runtime::game_mode::GameMode;
    overlay.visible = true;
    overlay.opened_from_pause = matches!(mode, GameMode::Paused);
    pages.active = Some(page);
    // Seed a sensible cursor for the opening page.
    system_nav.open_category = None;
    cursor.mark_keyboard(match page {
        CubePage::Items => CubeFocus::Item(0),
        CubePage::System => CubeFocus::System(0),
        CubePage::Map | CubePage::Quest => CubeFocus::EdgeLeft,
    });
    // Never leave the standalone map panel open underneath the cube.
    map.open = false;
    if matches!(mode, GameMode::Playing) {
        next_mode.set(GameMode::Paused);
    }
}

/// Close the cube overlay (Esc while open), restoring `GameMode::Playing` when the
/// cube was opened directly from gameplay (matching `close_oot_menu`).
#[cfg(feature = "input")]
fn close_cube_menu(
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

/// Dev runtime toggle (#31): `\` flips the inventory frontend between the Bevy-UI
/// grid and the 3D cube. Logs the new backend so it's visible in the console.
fn toggle_inventory_backend(
    keys: Res<ButtonInput<KeyCode>>,
    mut backend: ResMut<InventoryUiBackend>,
) {
    if keys.just_pressed(KeyCode::Backslash) {
        *backend = match *backend {
            InventoryUiBackend::Grid => InventoryUiBackend::Cube,
            InventoryUiBackend::Cube => InventoryUiBackend::Grid,
        };
        info!("inventory backend → {:?}", *backend);
    }
}

/// Pause-gate the cube: its order-8 `Camera3d` clears the whole screen every frame,
/// so it must be active only while the inventory is open (and the Cube backend is
/// selected). Off otherwise → the lower-order game cameras render normally.
fn gate_cube_menu(
    backend: Res<InventoryUiBackend>,
    ui_state: Option<Res<crate::inventory::InventoryUiState>>,
    mut open_state: ResMut<ambition_inventory_ui::cube::CubeOpenState>,
    mut cameras: Query<(&mut Camera, Has<ambition_inventory_ui::cube::CubePauseCamera>)>,
    mut rings: Query<&mut Visibility, With<ambition_inventory_ui::cube::MenuRing>>,
    mut last_show: Local<Option<bool>>,
) {
    let open = ui_state.map(|s| s.visible).unwrap_or(false);
    let show = *backend == InventoryUiBackend::Cube && open;
    if *last_show != Some(show) {
        info!("cube gate: show={show} backend={:?} menu_open={open}", *backend);
        *last_show = Some(show);
    }
    // Drive the lib's open/close fold: it eases `amount` toward this target each
    // frame (see `animate_cube_open`). We gate the camera/visibility off the eased
    // AMOUNT (not the binary `show`) so the close-fold animation stays on-screen
    // until the cube has fully folded shut.
    open_state.target = if show { 1.0 } else { 0.0 };
    // Hide the camera/ring once the close-fold has decayed past a sizable cutoff
    // (not a near-zero `0.002`) so the slow fold/scrim TAIL is cut and the menu
    // clears snappily. Combined with the lib's faster close decay
    // (`close_speed_scale`), the scrim (which follows `amount`) reads as a quick
    // fade-out. The cutoff only matters while CLOSING; opening crosses it instantly.
    let shown = open_state.amount > 0.08;
    // Option 1 overlay experiment: toggle ONLY the cube camera and LEAVE the game's
    // 2D camera active, so the live world renders behind the cube (which now clears
    // None). This is the configuration we previously avoided (sole-camera) to dodge
    // the 2D/3D share bug — but that bug's real cause was the camera-drag (now fixed
    // via With<Camera2d>) plus an MSAA mismatch (now matched), so it's worth a try.
    for (mut cam, is_cube) in &mut cameras {
        if is_cube && cam.is_active != shown {
            cam.is_active = shown;
        }
    }
    let want = if shown {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    for mut vis in &mut rings {
        if *vis != want {
            *vis = want;
        }
    }
}

/// Republish the cube's faces from our live inventory + the focus cursor (the
/// host-owned data seam — the cube renderer treats `ActiveMenuPages` as read-only).
///
/// Runs after [`cube_focus_nav`] in the chain so this frame's cursor move is
/// reflected in the rebuilt page (highlight + detail panel). To avoid an infinite
/// rebuild loop (writing `pages.pages` marks the resource changed), it republishes
/// only when something it depends on actually changed: the inventory, the focus
/// cursor, the active page, the just-opened edge, or the very first publish.
fn republish_cube_pages(
    backend: Res<InventoryUiBackend>,
    ui_state: Option<Res<crate::inventory::InventoryUiState>>,
    owned: Option<Res<OwnedItems>>,
    // Read-only here. The mutators (`cube_focus_nav`, `cube_pointer_click`) take
    // `ResMut<UserSettings>` in SEPARATE systems, so this `Res` is not a B0002
    // conflict; `UserSettings` is inserted at startup so the `Res` never panics.
    settings: Res<UserSettings>,
    cursor: Res<CubeCursor>,
    // Read-only here; the mutators (`cube_focus_nav`, `cube_pointer_click`) take
    // `ResMut<CubeSystemNav>` in SEPARATE systems/observers, so this `Res` is not a
    // B0002 conflict. Inserted at startup (`init_resource`) so it never panics.
    system_nav: Res<CubeSystemNav>,
    mut pages: ResMut<ActiveMenuPages<CubePage, CubeAction>>,
    mut was_open: Local<bool>,
    mut last: Local<Option<(CubeFocus, Option<CubePage>, Option<SystemCategory>)>>,
) {
    if *backend != InventoryUiBackend::Cube {
        return;
    }
    let Some(owned) = owned else {
        return;
    };
    let open = ui_state.map(|s| s.visible).unwrap_or(false);
    let just_opened = open && !*was_open;
    *was_open = open;

    // The drill-down state is part of the page key, so drilling into/out of a
    // System category republishes the (now different) System rows.
    let key = (cursor.focus, pages.active, system_nav.open_category);
    // Republish on: catalog change, settings change (so a toggled System option's
    // label updates immediately), first publish, menu-open (textures that loaded
    // after the initial build get picked up), cursor move, page change, or a
    // System drill in/out. The open case fixes icons rendering blank until the
    // first rotate.
    let dirty = owned.is_changed()
        || settings.is_changed()
        || pages.pages.is_empty()
        || just_opened
        || *last != Some(key);
    if !dirty {
        return;
    }
    *last = Some(key);

    let active = pages.active.unwrap_or(CubePage::Items);
    pages.pages = build_inventory_pages(
        &owned,
        owned.equipped(),
        cursor.focus,
        &settings,
        system_nav.open_category,
    );
    pages.active = Some(active);
}

#[cfg(test)]
mod oot_cube_app_tests {
    //! Behaviour tests for the cube's interaction seams, driven through the real
    //! systems / observers exactly as the app wires them.
    //!
    //! * Fix 1 — [`back_edge_focus`] lands the cursor on the "back" edge button.
    //! * Fix 4 — `cube_pointer_click` dispatches System-page clicks (drill in,
    //!   apply an option, Close) at parity with keyboard select.
    use super::*;
    use crate::brain::ActionSet;
    use crate::game_mode::GameMode;
    use crate::player::{PlayerEntity, PlayerMana, PrimaryPlayer};
    use bevy::picking::pointer::{Location, PointerId};
    use bevy::picking::backend::HitData;
    use bevy::picking::events::{Click, Pointer};
    use bevy::camera::NormalizedRenderTarget;
    use core::time::Duration;

    // ---- Fix 1: back-edge seeding --------------------------------------------

    #[test]
    fn back_edge_lands_opposite_the_direction_travelled() {
        // Turning RIGHT brings the viewer-right page to front; to go BACK you turn
        // left, so the cursor lands on the LEFT edge button — and vice-versa.
        let from = CubePage::Items;
        let right = from.on_viewer_right();
        assert_eq!(back_edge_focus(Some(from), right), CubeFocus::EdgeLeft);
        let left = from.on_viewer_left();
        assert_eq!(back_edge_focus(Some(from), left), CubeFocus::EdgeRight);
        // First open (no prior page) defaults to a highlighted left edge button.
        assert_eq!(back_edge_focus(None, CubePage::Map), CubeFocus::EdgeLeft);
    }

    // ---- Fix 4: System-page pointer clicks -----------------------------------

    fn click_app() -> (App, Entity) {
        let mut app = App::new();
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.init_state::<GameMode>();
        app.init_resource::<InventoryUiBackend>();
        app.init_resource::<ActiveMenuPages<CubePage, CubeAction>>();
        app.init_resource::<CubeCursor>();
        app.init_resource::<CubeSystemNav>();
        app.init_resource::<OwnedItems>();
        app.init_resource::<UserSettings>();
        app.init_resource::<crate::inventory::InventoryUiState>();
        app.add_message::<PlayerHealRequested>();
        app.add_observer(cube_pointer_click);
        *app.world_mut().resource_mut::<InventoryUiBackend>() = InventoryUiBackend::Cube;
        app.world_mut()
            .resource_mut::<crate::inventory::InventoryUiState>()
            .visible = true;
        let player = app
            .world_mut()
            .spawn((PlayerEntity, PrimaryPlayer, ActionSet::default(), PlayerMana::default()))
            .id();
        app.update();
        (app, player)
    }

    /// Spawn a cube control carrying `action` and fire a real `Pointer<Click>` at it,
    /// exactly as Bevy picking would.
    fn click_control(app: &mut App, action: CubeAction) {
        let entity = app
            .world_mut()
            .spawn(AmbitionMenuControl::<CubeAction> {
                kind: ambition_inventory_ui::MenuControlKind::OptionToggle,
                action: Some(action),
                focus: ambition_inventory_ui::MenuFocusKey::default(),
            })
            .id();
        // The observer only reads `click.entity`; any render target works for the
        // location, so the simplest no-render target keeps the fixture minimal.
        let location = Location {
            target: NormalizedRenderTarget::None { width: 1, height: 1 },
            position: Vec2::ZERO,
        };
        let click = Pointer::new(
            PointerId::Mouse,
            location,
            Click {
                button: bevy::picking::pointer::PointerButton::Primary,
                hit: HitData::new(entity, 0.0, None, None),
                duration: Duration::ZERO,
            },
            entity,
        );
        app.world_mut().trigger(click);
        app.update();
    }

    // ---- Fix 2: shoulder-bumper page turns -----------------------------------

    fn nav_app() -> App {
        let mut app = App::new();
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.init_state::<GameMode>();
        app.init_resource::<InventoryUiBackend>();
        app.init_resource::<ActiveMenuPages<CubePage, CubeAction>>();
        app.init_resource::<CubeCursor>();
        app.init_resource::<CubeSystemNav>();
        app.init_resource::<OwnedItems>();
        app.init_resource::<UserSettings>();
        app.init_resource::<crate::inventory::InventoryUiState>();
        app.init_resource::<MenuControlFrame>();
        app.add_message::<PlayerHealRequested>();
        app.add_systems(Update, cube_focus_nav);
        *app.world_mut().resource_mut::<InventoryUiBackend>() = InventoryUiBackend::Cube;
        app.world_mut()
            .resource_mut::<crate::inventory::InventoryUiState>()
            .visible = true;
        app.world_mut()
            .resource_mut::<ActiveMenuPages<CubePage, CubeAction>>()
            .active = Some(CubePage::Items);
        app.world_mut()
            .spawn((PlayerEntity, PrimaryPlayer, ActionSet::default(), PlayerMana::default()));
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

    #[test]
    fn right_bumper_turns_to_the_viewer_right_page() {
        let mut app = nav_app();
        press_bumper(&mut app, true);
        assert_eq!(
            app.world().resource::<ActiveMenuPages<CubePage, CubeAction>>().active,
            Some(CubePage::Items.on_viewer_right()),
            "right bumper rotates to the viewer-right page (Fix 2)"
        );
        // The cursor lands on the new page's back-edge button (Fix 1): arriving from
        // the right edge means the LEFT edge button turns back home.
        assert_eq!(
            app.world().resource::<CubeCursor>().focus,
            CubeFocus::EdgeLeft,
            "cursor seeds onto the back (left) edge button"
        );
    }

    #[test]
    fn left_bumper_turns_to_the_viewer_left_page() {
        let mut app = nav_app();
        press_bumper(&mut app, false);
        assert_eq!(
            app.world().resource::<ActiveMenuPages<CubePage, CubeAction>>().active,
            Some(CubePage::Items.on_viewer_left()),
            "left bumper rotates to the viewer-left page (Fix 2)"
        );
        assert_eq!(
            app.world().resource::<CubeCursor>().focus,
            CubeFocus::EdgeRight,
            "cursor seeds onto the back (right) edge button"
        );
    }

    #[test]
    fn clicking_a_system_category_drills_in() {
        let (mut app, _player) = click_app();
        app.world_mut()
            .resource_mut::<ActiveMenuPages<CubePage, CubeAction>>()
            .active = Some(CubePage::System);
        assert!(app.world().resource::<CubeSystemNav>().open_category.is_none());
        click_control(&mut app, CubeAction::OpenSystemCategory(SystemCategory::Audio));
        assert_eq!(
            app.world().resource::<CubeSystemNav>().open_category,
            Some(SystemCategory::Audio),
            "clicking a System category drills into it (Fix 4)"
        );
    }

    #[test]
    fn clicking_a_system_option_applies_it() {
        let (mut app, _player) = click_app();
        app.world_mut()
            .resource_mut::<ActiveMenuPages<CubePage, CubeAction>>()
            .active = Some(CubePage::System);
        app.world_mut().resource_mut::<CubeSystemNav>().open_category = Some(SystemCategory::Video);
        let before = app.world().resource::<UserSettings>().video.show_fps;
        click_control(&mut app, CubeAction::System(SystemOption::ToggleFps));
        let after = app.world().resource::<UserSettings>().video.show_fps;
        assert_ne!(before, after, "clicking an option toggles the setting (Fix 4)");
    }

    #[test]
    fn clicking_back_drills_out_to_the_category_list() {
        let (mut app, _player) = click_app();
        app.world_mut()
            .resource_mut::<ActiveMenuPages<CubePage, CubeAction>>()
            .active = Some(CubePage::System);
        app.world_mut().resource_mut::<CubeSystemNav>().open_category = Some(SystemCategory::Audio);
        click_control(&mut app, CubeAction::CloseSystemCategory);
        assert!(
            app.world().resource::<CubeSystemNav>().open_category.is_none(),
            "clicking Back drills out to the category list (Fix 4)"
        );
    }

    #[test]
    fn clicking_close_menu_closes_the_overlay() {
        let (mut app, _player) = click_app();
        app.world_mut()
            .resource_mut::<ActiveMenuPages<CubePage, CubeAction>>()
            .active = Some(CubePage::System);
        click_control(&mut app, CubeAction::System(SystemOption::CloseMenu));
        assert!(
            !app.world().resource::<crate::inventory::InventoryUiState>().visible,
            "clicking Close Menu folds the cube shut (Fix 4)"
        );
    }
}

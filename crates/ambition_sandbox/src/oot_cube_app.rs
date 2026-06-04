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
use crate::oot_cube::{build_inventory_pages, CubeAction, CubeFocus, CubePage};
use crate::oot_menu::input::{dispatch_item_confirm, MenuEffectManaQuery, MenuEffectPlayers};
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
        .add_plugins(AmbitionInventoryUiPlugin)
        .add_plugins(CubeMenuPlugin::<CubePage, CubeAction>::default())
        .add_systems(Startup, spawn_cube_scrim)
        .add_systems(
            Update,
            (
                // Nav first (mutates the cursor), then republish (reads the cursor +
                // inventory) so the highlight + detail panel reflect this frame's move.
                cube_focus_nav,
                republish_cube_pages,
                gate_cube_menu,
                toggle_inventory_backend,
                fade_cube_scrim,
            )
                .chain(),
        )
        .add_observer(cube_pointer_over)
        .add_observer(cube_pointer_click);
}

/// The directional-focus cursor for the items page: which item slot or edge
/// (page-turn) button the cursor sits on. Mirrors the demo's selection state
/// (`MockDemo::selected`). [`cube_focus_nav`] moves it with `move_spatial`-style
/// rules; [`republish_cube_pages`] republishes the page model whenever it changes
/// so the highlight + detail panel follow it.
#[derive(Resource, Default)]
struct CubeCursor {
    focus: CubeFocus,
}

/// Spawn the readability dim-scrim node (full-screen, starts fully transparent).
/// It lives on the `Camera2d` (which carries `IsDefaultUiCamera`), so it renders
/// between the world and the order-8 cube. [`fade_cube_scrim`] drives its alpha.
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
    mut pages: ResMut<ActiveMenuPages<CubePage, CubeAction>>,
    // Single mutable access to the overlay state — also read `.visible` from it (a
    // separate `Res<InventoryUiState>` would be a B0002 conflict with this `ResMut`).
    mut overlay: ResMut<crate::inventory::InventoryUiState>,
    mut owned: ResMut<OwnedItems>,
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

    // Non-items faces only respond to horizontal page turns (matches the demo's
    // early branch in move_spatial).
    if active_page != CubePage::Items {
        if dx < 0 {
            turn_page(&mut pages, active_page.on_viewer_left());
        } else if dx > 0 {
            turn_page(&mut pages, active_page.on_viewer_right());
        }
        if menu.back {
            overlay.visible = false;
        }
        return;
    }

    if dx != 0 || dy != 0 {
        match move_spatial(cursor.focus, dx, dy, active_page) {
            SpatialMove::Focus(next) => cursor.focus = next,
            SpatialMove::TurnLeft => {
                turn_page(&mut pages, active_page.on_viewer_left());
                // Land the cursor on the new face's right arrow (so pressing back
                // toward centre re-enters the grid) — demo's turn_page_from_edge.
                cursor.focus = CubeFocus::EdgeRight;
            }
            SpatialMove::TurnRight => {
                turn_page(&mut pages, active_page.on_viewer_right());
                cursor.focus = CubeFocus::EdgeLeft;
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
        };
        if let Some(action) = action {
            dispatch_cube_action(
                action,
                &mut pages,
                &mut owned,
                &mut commands,
                &mut players,
                &mut mana_q,
                &mut heals,
            );
        }
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
    owned: &mut OwnedItems,
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
            pages.active = Some(page);
            info!("cube page \u{2192} {:?}", page);
        }
    }
}

/// Map a control's `CubeAction` back to the cursor focus it represents, so pointer
/// hover/click and the keyboard cursor share one model. `Equip`/`Use` carry the
/// item (→ its slot); `ChangePage` is an edge arrow — left vs right is decided by
/// whether its target is the active page's viewer-left neighbour.
fn focus_for_action(action: CubeAction, active_page: CubePage) -> CubeFocus {
    match action {
        CubeAction::Equip(item) | CubeAction::Use(item) => CubeFocus::Item(item.index()),
        CubeAction::ChangePage(target) => {
            if target == active_page.on_viewer_left() {
                CubeFocus::EdgeLeft
            } else {
                CubeFocus::EdgeRight
            }
        }
    }
}

/// Pointer hover (mouse/touch) over a cube control: move the focus cursor to it.
/// Bevy picking fires this for mouse AND touch uniformly. Republish then follows
/// the cursor (highlight + detail panel).
fn cube_pointer_over(
    over: On<Pointer<Over>>,
    controls: Query<&AmbitionMenuControl<CubeAction>>,
    pages: Res<ActiveMenuPages<CubePage, CubeAction>>,
    mut cursor: ResMut<CubeCursor>,
) {
    let Some(active_page) = pages.active else {
        return;
    };
    if let Ok(control) = controls.get(over.entity) {
        if let Some(action) = control.action {
            let next = focus_for_action(action, active_page);
            if cursor.focus != next {
                cursor.focus = next;
            }
        }
    }
}

/// Pointer click (mouse/touch) on a cube control: dispatch its `CubeAction`.
#[allow(clippy::too_many_arguments)]
fn cube_pointer_click(
    click: On<Pointer<Click>>,
    backend: Res<InventoryUiBackend>,
    ui_state: Option<Res<crate::inventory::InventoryUiState>>,
    controls: Query<&AmbitionMenuControl<CubeAction>>,
    mut pages: ResMut<ActiveMenuPages<CubePage, CubeAction>>,
    mut cursor: ResMut<CubeCursor>,
    mut owned: ResMut<OwnedItems>,
    mut commands: Commands,
    mut players: MenuEffectPlayers,
    mut mana_q: MenuEffectManaQuery,
    mut heals: MessageWriter<PlayerHealRequested>,
) {
    let open = ui_state.map(|s| s.visible).unwrap_or(false);
    if *backend != InventoryUiBackend::Cube || !open {
        return;
    }
    if let Ok(control) = controls.get(click.entity) {
        if let Some(action) = control.action {
            if let Some(active_page) = pages.active {
                cursor.focus = focus_for_action(action, active_page);
            }
            dispatch_cube_action(
                action,
                &mut pages,
                &mut owned,
                &mut commands,
                &mut players,
                &mut mana_q,
                &mut heals,
            );
        }
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
    let shown = open_state.amount > 0.002;
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
    cursor: Res<CubeCursor>,
    mut pages: ResMut<ActiveMenuPages<CubePage, CubeAction>>,
    mut was_open: Local<bool>,
    mut last: Local<Option<(CubeFocus, Option<CubePage>)>>,
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

    let key = (cursor.focus, pages.active);
    // Republish on: catalog change, first publish, menu-open (textures that loaded
    // after the initial build get picked up), cursor move, or page change. The
    // open case fixes icons rendering blank until the first rotate.
    let dirty = owned.is_changed()
        || pages.pages.is_empty()
        || just_opened
        || *last != Some(key);
    if !dirty {
        return;
    }
    *last = Some(key);

    let active = pages.active.unwrap_or(CubePage::Items);
    pages.pages = build_inventory_pages(&owned, owned.equipped(), cursor.focus);
    pages.active = Some(active);
}

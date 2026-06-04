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
    ActiveMenuPages, AmbitionInventoryUiPlugin, AmbitionMenuControl, AmbitionMenuPage, MenuFocusKey,
    MenuVisualState,
};
use bevy::prelude::*;

use crate::input::MenuControlFrame;
use crate::items::OwnedItems;
use crate::oot_cube::{build_inventory_pages, CubeAction, CubePage};
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
        .init_resource::<CubeFocus>()
        .add_plugins(AmbitionInventoryUiPlugin)
        .add_plugins(CubeMenuPlugin::<CubePage, CubeAction>::default())
        .add_systems(Startup, spawn_cube_scrim)
        .add_systems(
            Update,
            (
                sync_cube_pages,
                gate_cube_menu,
                toggle_inventory_backend,
                cube_focus_nav,
                cube_apply_focus_visuals,
                fade_cube_scrim,
            ),
        )
        .add_observer(cube_pointer_over)
        .add_observer(cube_pointer_click);
}

/// The directional-focus cursor for the cube. Holds the [`MenuFocusKey`] of the
/// currently-focused control on the active page. `None` until first navigation /
/// hover, at which point [`cube_focus_nav`] seeds it from the first control.
#[derive(Resource, Default)]
struct CubeFocus {
    key: Option<MenuFocusKey>,
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

/// Walk up the `ChildOf` chain from `entity` to the `AmbitionMenuPage` face it
/// belongs to, returning that page id. Cube controls are grandchildren of the
/// face, so we climb until we hit the entity carrying the page component.
fn page_of(
    mut entity: Entity,
    parents: &Query<&ChildOf>,
    faces: &Query<&AmbitionMenuPage<CubePage>>,
) -> Option<CubePage> {
    loop {
        if let Ok(face) = faces.get(entity) {
            return Some(face.id);
        }
        match parents.get(entity) {
            Ok(child_of) => entity = child_of.parent(),
            Err(_) => return None,
        }
    }
}

/// Directional focus navigation for the cube (keyboard / gamepad), matching the
/// demo's spatial convention: on the active page, Up/Down/Left/Right move the
/// item cursor through the 6×4 grid; Left at the left column / Right at the right
/// column lands on the flanking edge button, and `select` there turns the page.
/// `select` on an item dispatches its `CubeAction`; `back` closes the menu.
///
/// Page-change is therefore reachable two ways: walk the cursor off the grid edge
/// onto an edge button and confirm, or click an edge button with the pointer.
#[allow(clippy::too_many_arguments)]
fn cube_focus_nav(
    backend: Res<InventoryUiBackend>,
    ui_state: Option<Res<crate::inventory::InventoryUiState>>,
    menu: Res<MenuControlFrame>,
    mut focus: ResMut<CubeFocus>,
    mut pages: ResMut<ActiveMenuPages<CubePage, CubeAction>>,
    mut overlay: ResMut<crate::inventory::InventoryUiState>,
    controls: Query<(Entity, &AmbitionMenuControl<CubeAction>)>,
    parents: Query<&ChildOf>,
    faces: Query<&AmbitionMenuPage<CubePage>>,
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
    let Some(active_page) = pages.active else {
        return;
    };

    // The focusable controls on the CURRENTLY-active page, with their grid keys.
    let mut page_controls: Vec<(MenuFocusKey, CubeAction)> = controls
        .iter()
        .filter(|(e, _)| page_of(*e, &parents, &faces) == Some(active_page))
        .filter_map(|(_, c)| c.action.map(|a| (c.focus, a)))
        .collect();
    if page_controls.is_empty() {
        return;
    }
    // Stable reading order (top-to-bottom, left-to-right) so seeding + the edge
    // buttons resolve deterministically.
    page_controls.sort_by_key(|(k, _)| (k.row, k.col));

    // Seed the cursor onto the first control if it isn't on this page yet.
    let cur_key = focus
        .key
        .filter(|k| page_controls.iter().any(|(ck, _)| ck == k))
        .unwrap_or(page_controls[0].0);

    // Directional move: pick the nearest control in the requested direction using
    // the focus-key grid coords (col≈x*10, row≈y*10).
    let (mut dx, mut dy) = (0i32, 0i32);
    if menu.left {
        dx -= 1;
    }
    if menu.right {
        dx += 1;
    }
    if menu.up {
        dy -= 1;
    }
    if menu.down {
        dy += 1;
    }
    let mut next_key = cur_key;
    if dx != 0 || dy != 0 {
        let mut best: Option<(i64, MenuFocusKey)> = None;
        for (k, _) in &page_controls {
            if *k == cur_key {
                continue;
            }
            let ddx = (k.col - cur_key.col) as i64;
            let ddy = (k.row - cur_key.row) as i64;
            // Must move in the requested direction on the dominant axis.
            let aligned = (dx > 0 && ddx > 0)
                || (dx < 0 && ddx < 0)
                || (dy > 0 && ddy > 0)
                || (dy < 0 && ddy < 0);
            if !aligned {
                continue;
            }
            // Prefer the axis we're moving along; penalise cross-axis drift.
            let (along, across) = if dx != 0 { (ddx.abs(), ddy.abs()) } else { (ddy.abs(), ddx.abs()) };
            let cost = along + across * 100;
            if best.map(|(c, _)| cost < c).unwrap_or(true) {
                best = Some((cost, *k));
            }
        }
        if let Some((_, k)) = best {
            next_key = k;
        }
    }
    focus.key = Some(next_key);

    if menu.back {
        overlay.visible = false;
        return;
    }

    if menu.select {
        if let Some((_, action)) = page_controls.iter().find(|(k, _)| *k == next_key) {
            dispatch_cube_action(
                *action,
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

/// Mirror the focus cursor onto the controls' [`MenuVisualState`]: the focused
/// control reads `focused`, everything else clears. The lib renders selection
/// corners from this flag (`draw_selection_corners`). Scoped to the active page.
fn cube_apply_focus_visuals(
    backend: Res<InventoryUiBackend>,
    ui_state: Option<Res<crate::inventory::InventoryUiState>>,
    focus: Res<CubeFocus>,
    pages: Res<ActiveMenuPages<CubePage, CubeAction>>,
    parents: Query<&ChildOf>,
    faces: Query<&AmbitionMenuPage<CubePage>>,
    mut controls: Query<(Entity, &AmbitionMenuControl<CubeAction>, &mut MenuVisualState)>,
) {
    let open = ui_state.map(|s| s.visible).unwrap_or(false);
    if *backend != InventoryUiBackend::Cube || !open {
        return;
    }
    let active_page = pages.active;
    for (e, control, mut vis) in &mut controls {
        let on_active = page_of(e, &parents, &faces) == active_page;
        let want = on_active && control.action.is_some() && focus.key == Some(control.focus);
        if vis.focused != want {
            vis.focused = want;
        }
    }
}

/// Pointer hover (mouse/touch) over a cube control: move the focus cursor to it
/// and mark it hovered. Bevy picking fires this for mouse AND touch uniformly.
fn cube_pointer_over(
    over: On<Pointer<Over>>,
    controls: Query<&AmbitionMenuControl<CubeAction>>,
    mut focus: ResMut<CubeFocus>,
) {
    if let Ok(control) = controls.get(over.entity) {
        if control.action.is_some() {
            focus.key = Some(control.focus);
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

/// Republish the cube's faces from our live inventory when it changes (the
/// host-owned data seam — the cube renderer treats `ActiveMenuPages` as read-only).
fn sync_cube_pages(
    backend: Res<InventoryUiBackend>,
    ui_state: Option<Res<crate::inventory::InventoryUiState>>,
    owned: Option<Res<OwnedItems>>,
    mut pages: ResMut<ActiveMenuPages<CubePage, CubeAction>>,
    mut was_open: Local<bool>,
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
    // Republish on catalog change, first publish, OR each time the menu opens. The
    // open case fixes icons rendering blank until the first rotate: faces are built
    // once at startup, but item icon textures that finish loading after that build
    // are only picked up on a rebuild — so force one when the menu is shown.
    if !owned.is_changed() && !pages.pages.is_empty() && !just_opened {
        return;
    }
    pages.pages = build_inventory_pages(&owned, owned.equipped(), None);
    if pages.active.is_none() {
        pages.active = Some(CubePage::Items);
    }
    info!("cube sync: published {} page(s) to the cube", pages.pages.len());
}

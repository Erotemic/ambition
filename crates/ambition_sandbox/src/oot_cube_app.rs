//! Game-side hookup for the 3D-cube OoT pause menu (#31): adds the lib's reusable
//! cube renderer ([`ambition_inventory_ui::cube::CubeMenuPlugin`]) and feeds it our
//! live 24-item inventory (via [`crate::oot_cube`]). Runtime-toggleable vs the
//! existing Bevy-UI grid through [`InventoryUiBackend`].
//!
//! The cube is pause-gated ([`gate_cube_menu`]): its order-8 `Camera3d` + ring are
//! only active while the inventory is open, so it never clears the screen to black
//! during play. Routing nav/selection input to it is the next step — see
//! `dev/journals/oot-cube-integration-plan.md`.

use ambition_inventory_ui::cube::CubeMenuPlugin;
use ambition_inventory_ui::{ActiveMenuPages, AmbitionInventoryUiPlugin};
use bevy::prelude::*;

use crate::items::OwnedItems;
use crate::oot_cube::{build_inventory_pages, CubeAction, CubePage};

/// Which inventory frontend renders. Runtime toggle (both compiled in); defaults to
/// the 3D `Cube` (#31), with `\` flipping to the proven Bevy-UI `Grid` (see
/// [`toggle_inventory_backend`]).
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum InventoryUiBackend {
    Grid,
    #[default]
    Cube,
}

/// Wire the 3D-cube menu into the app: the lib plugins + our page-feed system.
pub fn install_cube_menu(app: &mut App) {
    app.init_resource::<InventoryUiBackend>()
        .init_resource::<ActiveMenuPages<CubePage, CubeAction>>()
        .add_plugins(AmbitionInventoryUiPlugin)
        .add_plugins(CubeMenuPlugin::<CubePage, CubeAction>::default())
        .add_systems(
            Update,
            (
                sync_cube_pages,
                gate_cube_menu,
                toggle_inventory_backend,
                cube_page_nav,
            ),
        );
}

/// Rotate the cube: while it's open, Left/Right (or A/D) cycle the active page and
/// the lib's snap-to-active rotation turns that face to the camera (#31 nav).
fn cube_page_nav(
    backend: Res<InventoryUiBackend>,
    ui_state: Option<Res<crate::inventory::InventoryUiState>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut pages: ResMut<ActiveMenuPages<CubePage, CubeAction>>,
) {
    let open = ui_state.map(|s| s.visible).unwrap_or(false);
    if *backend != InventoryUiBackend::Cube || !open {
        return;
    }
    let dir = if keys.just_pressed(KeyCode::ArrowRight) || keys.just_pressed(KeyCode::KeyD) {
        1
    } else if keys.just_pressed(KeyCode::ArrowLeft) || keys.just_pressed(KeyCode::KeyA) {
        -1
    } else {
        return;
    };
    let all = CubePage::ALL;
    let cur = pages
        .active
        .and_then(|a| all.iter().position(|p| *p == a))
        .unwrap_or(0);
    let next = (cur as isize + dir).rem_euclid(all.len() as isize) as usize;
    pages.active = Some(all[next]);
    info!("cube page \u{2192} {:?}", all[next]);
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

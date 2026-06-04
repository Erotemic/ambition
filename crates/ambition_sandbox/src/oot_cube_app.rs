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

/// Which inventory frontend renders. Runtime toggle (both are compiled in); the
/// cube is default during #31 bring-up so it's visible to iterate on.
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
        .add_systems(Update, (sync_cube_pages, gate_cube_menu));
}

/// Pause-gate the cube: its order-8 `Camera3d` clears the whole screen every frame,
/// so it must be active only while the inventory is open (and the Cube backend is
/// selected). Off otherwise → the lower-order game cameras render normally.
fn gate_cube_menu(
    backend: Res<InventoryUiBackend>,
    ui_state: Option<Res<crate::inventory::InventoryUiState>>,
    mut cameras: Query<&mut Camera, With<ambition_inventory_ui::cube::CubePauseCamera>>,
    mut rings: Query<&mut Visibility, With<ambition_inventory_ui::cube::MenuRing>>,
) {
    let open = ui_state.map(|s| s.visible).unwrap_or(false);
    let show = *backend == InventoryUiBackend::Cube && open;
    for mut cam in &mut cameras {
        if cam.is_active != show {
            cam.is_active = show;
        }
    }
    let want = if show {
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
    owned: Option<Res<OwnedItems>>,
    mut pages: ResMut<ActiveMenuPages<CubePage, CubeAction>>,
) {
    if *backend != InventoryUiBackend::Cube {
        return;
    }
    let Some(owned) = owned else {
        return;
    };
    if !owned.is_changed() && !pages.pages.is_empty() {
        return;
    }
    pages.pages = build_inventory_pages(&owned, owned.equipped(), None);
    if pages.active.is_none() {
        pages.active = Some(CubePage::Items);
    }
}

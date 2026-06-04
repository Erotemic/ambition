//! Game-side hookup for the 3D-cube OoT pause menu (#31): adds the lib's reusable
//! cube renderer ([`ambition_inventory_ui::cube::CubeMenuPlugin`]) and feeds it our
//! live 24-item inventory (via [`crate::oot_cube`]). Runtime-toggleable vs the
//! existing Bevy-UI grid through [`InventoryUiBackend`].
//!
//! First bring-up (#31): the cube renders from `ActiveMenuPages`; pause-gating the
//! cube camera + routing input to it (so it only shows while paused) is the next
//! visual-iteration step — see `dev/journals/oot-cube-integration-plan.md`.

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
        .add_systems(Update, sync_cube_pages);
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

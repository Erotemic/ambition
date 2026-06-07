use bevy::prelude::*;

use super::model::MapMenuState;

#[cfg(feature = "input")]
use crate::input::MenuControlFrame;

#[cfg(feature = "input")]
pub fn handle_map_menu_hotkeys(
    keys: Res<bevy::input::ButtonInput<bevy::input::keyboard::KeyCode>>,
    menu: Res<MenuControlFrame>,
    mut map: ResMut<MapMenuState>,
    // Fix 3: under the 3D-cube inventory backend, the map key opens the cube on its
    // Map page (`lunex_kaleidoscope_app::kaleidoscope_menu_open_routing`) instead of this standalone
    // panel, so suppress the `menu.map` open here. The `M`/`N`/zoom keys still drive
    // the panel directly for users who toggle back to the Grid backend.
    #[cfg(feature = "oot_inventory")] kaleidoscope_backend: Option<
        Res<crate::lunex_kaleidoscope_app::InventoryUiBackend>,
    >,
) {
    use bevy::input::keyboard::KeyCode;
    #[cfg(feature = "oot_inventory")]
    let kaleidoscope = kaleidoscope_backend
        .map(|b| *b == crate::lunex_kaleidoscope_app::InventoryUiBackend::LunexKaleidoscope)
        .unwrap_or(false);
    #[cfg(not(feature = "oot_inventory"))]
    let kaleidoscope = false;
    // The `menu.map` intent routes to the cube under the Cube backend; the `M` key
    // keeps toggling the standalone panel so it stays reachable for debugging.
    if keys.just_pressed(KeyCode::KeyM) || (menu.map && !kaleidoscope) {
        map.toggle_open();
    }
    if keys.just_pressed(KeyCode::KeyN) {
        map.toggle_minimap();
    }
    if map.open {
        if menu.back || menu.start {
            map.open = false;
            return;
        }
        let zoom_in = keys.just_pressed(KeyCode::Equal)
            || keys.just_pressed(KeyCode::NumpadAdd)
            || menu.right
            || menu.scroll_y > 0.5;
        let zoom_out = keys.just_pressed(KeyCode::Minus)
            || keys.just_pressed(KeyCode::NumpadSubtract)
            || menu.left
            || menu.scroll_y < -0.5;
        let zoom_reset = keys.just_pressed(KeyCode::Digit0) || keys.just_pressed(KeyCode::Numpad0);
        if zoom_in {
            map.zoom_in();
        }
        if zoom_out {
            map.zoom_out();
        }
        if zoom_reset {
            map.zoom_reset();
        }
    }
}

#[cfg(not(feature = "input"))]
pub fn handle_map_menu_hotkeys() {}

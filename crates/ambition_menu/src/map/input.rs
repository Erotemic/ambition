//! Map-screen keyboard/menu-control input: `handle_map_menu_hotkeys` toggles
//! the map (the bound `menu.map` intent) and the minimap (`N`), the developer
//! map and changes zoom. The inventory menu handles the map shortcut when its
//! backend is available.

// both are used only by the `input`-gated system below, so they carry the
// SAME gate. Without it a build with the feature off warns on two unused
// imports, and a crate that warns in its ordinary configuration is a crate
// whose real warnings are invisible.
#[cfg(feature = "input")]
use bevy::prelude::*;

#[cfg(feature = "input")]
use super::MapMenuState;

#[cfg(feature = "input")]
use ambition_input::MenuControlFrame;

#[cfg(feature = "input")]
pub fn handle_map_menu_hotkeys(
    keys: Res<bevy::input::ButtonInput<bevy::input::keyboard::KeyCode>>,
    menu: Res<MenuControlFrame>,
    mut map: ResMut<MapMenuState>,
    backend: Option<Res<crate::backend::InventoryUiBackend>>,
) {
    use bevy::input::keyboard::KeyCode;
    let unified_menu = backend.is_some_and(|backend| backend.effective().is_available());
    if menu.map && !unified_menu {
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
        // A key no preset binds: gameplay runs under the open map, and `F` is
        // `interact` on two presets.
        if keys.just_pressed(KeyCode::Backquote) {
            map.toggle_reveal_all();
        }
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

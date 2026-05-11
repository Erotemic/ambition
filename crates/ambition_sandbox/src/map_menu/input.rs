use bevy::prelude::*;

use super::model::MapMenuState;

#[cfg(feature = "input")]
use crate::input::MenuControlFrame;

#[cfg(feature = "input")]
pub fn handle_map_menu_hotkeys(
    keys: Res<bevy::input::ButtonInput<bevy::input::keyboard::KeyCode>>,
    menu: Res<MenuControlFrame>,
    mut map: ResMut<MapMenuState>,
) {
    use bevy::input::keyboard::KeyCode;
    if keys.just_pressed(KeyCode::KeyM) || menu.map {
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

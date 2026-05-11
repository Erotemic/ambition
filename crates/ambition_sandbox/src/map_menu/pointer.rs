use bevy::prelude::*;

use super::model::MapMenuState;
use super::ui::MapMenuRoot;

#[cfg(feature = "input")]
pub fn map_menu_pointer_dismiss(
    mut map: ResMut<MapMenuState>,
    interactions: Query<&Interaction, (With<MapMenuRoot>, Changed<Interaction>)>,
) {
    if !map.open {
        return;
    }
    for interaction in &interactions {
        if matches!(interaction, Interaction::Pressed) {
            map.open = false;
        }
    }
}

#[cfg(not(feature = "input"))]
pub fn map_menu_pointer_dismiss() {}

//! Pointer dismissal for the full-map panel: `map_menu_pointer_dismiss` closes
//! the open map when its `MapMenuRoot` node is clicked/tapped. No-op stub when
//! the `input` feature is off.

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

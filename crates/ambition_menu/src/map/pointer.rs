//! Pointer dismissal for the full-map panel: `map_menu_pointer_dismiss` closes
//! the open map when its `MapMenuRoot` node is clicked/tapped. No-op stub when
//! the `input` feature is off.

// all three are used only by the `input`-gated system below; same gate, same
// reason as `input.rs`.
#[cfg(feature = "input")]
use bevy::prelude::*;

#[cfg(feature = "input")]
use super::ui::MapMenuRoot;
#[cfg(feature = "input")]
use super::MapMenuState;

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

//! Map / minimap state AND the Map tab that renders it.
//!
//! `MapMenuState` holds the visited-room set, per-room geometry
//! (`MapRoomNode`), open/minimap toggles, and the clamped zoom level
//! (`MAP_ZOOM_MIN`..`MAP_ZOOM_MAX`). `summary_lines` produces the HUD text; a
//! host-owned UI adapter can render it as a full map, minimap, or menu tab.

use std::collections::BTreeSet;

use bevy::prelude::*;

#[derive(Clone, Debug)]
pub struct MapRoomNode {
    pub id: String,
    pub world_min: Vec2,
    pub world_size: Vec2,
}

#[derive(Resource)]
pub struct MapMenuState {
    pub open: bool,
    pub minimap_enabled: bool,
    pub visited: BTreeSet<String>,
    pub rooms: Vec<MapRoomNode>,
    pub zoom: f32,
}

impl Default for MapMenuState {
    fn default() -> Self {
        Self {
            open: false,
            minimap_enabled: false,
            visited: BTreeSet::new(),
            rooms: Vec::new(),
            zoom: 1.0,
        }
    }
}

pub const MAP_ZOOM_STEP: f32 = 1.25;
pub const MAP_ZOOM_MIN: f32 = 0.5;
pub const MAP_ZOOM_MAX: f32 = 4.0;

impl MapMenuState {
    pub fn toggle_open(&mut self) {
        self.open = !self.open;
    }

    pub fn toggle_minimap(&mut self) {
        self.minimap_enabled = !self.minimap_enabled;
    }

    pub fn zoom_in(&mut self) {
        self.zoom = (self.zoom * MAP_ZOOM_STEP).clamp(MAP_ZOOM_MIN, MAP_ZOOM_MAX);
    }

    pub fn zoom_out(&mut self) {
        self.zoom = (self.zoom / MAP_ZOOM_STEP).clamp(MAP_ZOOM_MIN, MAP_ZOOM_MAX);
    }

    pub fn zoom_reset(&mut self) {
        self.zoom = 1.0;
    }

    pub fn record_visit(&mut self, room_id: &str) {
        self.visited.insert(room_id.to_string());
    }

    pub fn summary_lines(&self, current_room: &str) -> Vec<String> {
        if !self.open {
            if self.minimap_enabled {
                return vec![format!(
                    "minimap: {} visited / current = {}",
                    self.visited.len(),
                    current_room
                )];
            }
            return Vec::new();
        }
        let mut lines = vec![format!("MAP — {} visited", self.visited.len())];
        for id in &self.visited {
            let marker = if id == current_room { "→" } else { " " };
            lines.push(format!("{marker} {id}"));
        }
        lines
    }
}

/// The set [`populate_map_rooms`] runs in, published so a composition can
/// order against the phase instead of naming the function. Same pattern as
/// `AudioInitSet` in `ambition_app`; the app's `after_map_menu_spawn` profile
/// mark orders after this set.
#[cfg(feature = "ldtk")]
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MapMenuSpawnSet;

/// The map-menu domain's sim-state plugin (track 6, decision #9): the crate
/// owns its visited-rooms/map state, and the sim assembly only adds the
/// plugin.
pub struct MapStatePlugin;

impl bevy::prelude::Plugin for MapStatePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.init_resource::<MapMenuState>();

        // Vocabulary only. The systems are in [`install_map_menu_systems`],
        // a function, because a plugin in a plugin group runs in every
        // composition, and `handle_map_menu_hotkeys` panics in headless apps
        // with no `ButtonInput`.
    }
}

/// Install the map menu's systems.
///
/// A function, not a plugin. [`MapStatePlugin`] declares the vocabulary
/// ([`MapMenuState`]) unconditionally; whether a composition runs these
/// systems is the composition's choice. The caller names one function
/// instead of three private systems.
///
/// Contract: call this only from a composition that has input. These systems
/// take `Res<ButtonInput<KeyCode>>` and `Res<MenuControlFrame>` (from
/// `HostInputBindingsPlugin`) strictly. There is no `resource_exists` guard:
/// that would be runtime feature detection in place of a composition
/// contract. Calling this without input is a composition error and panics.
///
/// The systems also `.run_if(session_world_exists)`, so the panic comes on the
/// first update where a session world exists without the input resources. A
/// host that never opens a session is not proven correct by staying quiet.
pub fn install_map_menu_systems(app: &mut bevy::prelude::App) {
    // Startup half. `populate_map_rooms` reads `Res<ActiveLdtkProject>` and
    // writes the room list once. Its real prerequisite is that resource, so a
    // `run_if` is enough.
    #[cfg(feature = "ldtk")]
    app.add_systems(
        bevy::prelude::Startup,
        systems::populate_map_rooms.in_set(MapMenuSpawnSet).run_if(
            bevy::prelude::resource_exists::<ambition_platformer2d_ldtk::ActiveLdtkProject>,
        ),
    );
    app.add_systems(
            bevy::prelude::Update,
            (
                // No `run_if` for input: calling this installer without
                // `ButtonInput` and `MenuControlFrame` is a composition error
                // (see the doc above). `map_menu_pointer_dismiss` and
                // `sync_map_menu` need only `MapMenuState` and queries, so
                // only the hotkey system reaches outside this plugin.
                input::handle_map_menu_hotkeys,
                pointer::map_menu_pointer_dismiss,
                // The view uses the same ordering as the hotkey: after the
                // simulation phase, while a session world exists.
                ui::sync_map_menu,
            )
                .after(
                    ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith::CoreSimulation,
                )
                .run_if(ambition_platformer2d_shared_tangle::lifecycle::session_world_exists),
        );
}

// Nothing in the simulation calls these; only the runtime's progression
// schedule and the app's shell host use them.
mod input;
mod pointer;
mod systems;
mod ui;

#[cfg(test)]
mod tests;

pub use input::handle_map_menu_hotkeys;
pub use pointer::map_menu_pointer_dismiss;
#[cfg(feature = "ldtk")]
pub use systems::populate_map_rooms;
pub use systems::{
    room_from_visited_flag, room_visited_flag, sync_map_from_save, track_room_visits,
    ROOM_VISITED_FLAG_PREFIX,
};
pub use ui::{spawn_map_menu_with_scope, sync_map_menu, MapMenuRoot};

#[cfg(test)]
use ui::short_room_label;

/// Install the map's simulation half: `track_room_visits` records where the
/// player has been, and `sync_map_from_save` reconciles that with the save.
///
/// Separate from [`install_map_menu_systems`] on purpose: that half needs a
/// windowed host with `ButtonInput` and `MenuControlFrame`, and this half
/// needs neither. A headless simulation still records map facts.
///
/// The caller passes the schedule, because the caller owns where its
/// simulation runs. `ProgressionSet::Map` is from `shared_tangle`, which this
/// crate already depends on.
pub fn install_map_simulation_systems(
    app: &mut bevy::prelude::App,
    schedule: impl bevy::ecs::schedule::ScheduleLabel,
) {
    use bevy::prelude::IntoScheduleConfigs as _;

    app.add_systems(
        schedule,
        (systems::track_room_visits, systems::sync_map_from_save)
            .chain()
            .in_set(ambition_platformer2d_shared_tangle::schedule::ProgressionSet::Map),
    );
}

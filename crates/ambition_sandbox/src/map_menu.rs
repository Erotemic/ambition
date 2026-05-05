//! Map / minimap state.
//!
//! Tracks which rooms the player has visited (write `room_visited_<id>`
//! flag whenever the active room changes — already done by
//! `quest::push_room_entered_quest_events` via `RoomEntered`).
//! Surfaces the visited set + room dimensions / connections to the HUD
//! so a future map UI can render them.
//!
//! Right now there's no full-screen map UI; the data is exposed
//! through `MapMenuState::summary_lines` which the existing HUD picks
//! up. Pressing `M` toggles `MapMenuState::open`. When a richer UI
//! lands, this resource is the source of truth.

use std::collections::BTreeSet;

use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct MapMenuState {
    pub open: bool,
    pub minimap_enabled: bool,
    pub visited: BTreeSet<String>,
}

impl MapMenuState {
    /// Toggle the full map.
    pub fn toggle_open(&mut self) {
        self.open = !self.open;
    }

    /// Toggle the corner minimap.
    pub fn toggle_minimap(&mut self) {
        self.minimap_enabled = !self.minimap_enabled;
    }

    pub fn record_visit(&mut self, room_id: &str) {
        self.visited.insert(room_id.to_string());
    }

    /// Lines suitable for the HUD's quest log section.
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

/// Bevy system: sync the map's visited set with the live save flags.
/// We record the room id whenever the active room changes; the
/// `RoomEntered` quest event fires from the same frame.
pub fn track_room_visits(
    room_set: Res<crate::rooms::RoomSet>,
    mut map: ResMut<MapMenuState>,
    mut last: Local<Option<String>>,
    mut save: ResMut<crate::save::SandboxSave>,
) {
    let current = room_set.active_spec().id.clone();
    if last.as_deref() == Some(current.as_str()) {
        return;
    }
    *last = Some(current.clone());
    map.record_visit(&current);
    // Also write a save flag so the visited set persists across saves.
    save.data_mut()
        .set_flag(format!("room_visited_{current}"), true);
}

/// Bevy system: keep the in-memory visited set in sync with the save
/// resource on first load. Idempotent.
pub fn sync_map_from_save(
    save: Res<crate::save::SandboxSave>,
    mut map: ResMut<MapMenuState>,
    mut hydrated: Local<bool>,
) {
    if *hydrated {
        return;
    }
    *hydrated = true;
    for flag in &save.data().flags {
        if let Some(room_id) = flag.id.strip_prefix("room_visited_") {
            map.record_visit(room_id);
        }
    }
}

/// Toggle systems for keyboard input. `M` opens the map menu, `N`
/// toggles the minimap. Hidden behind dev_tools so the main game can
/// override the bindings later.
#[cfg(feature = "input")]
pub fn handle_map_menu_hotkeys(
    keys: Res<bevy::input::ButtonInput<bevy::input::keyboard::KeyCode>>,
    mut map: ResMut<MapMenuState>,
) {
    if keys.just_pressed(bevy::input::keyboard::KeyCode::KeyM) {
        map.toggle_open();
    }
    if keys.just_pressed(bevy::input::keyboard::KeyCode::KeyN) {
        map.toggle_minimap();
    }
}

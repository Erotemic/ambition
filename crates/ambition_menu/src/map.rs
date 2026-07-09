//! Map / minimap state — the renderer-agnostic source of truth the Map tab renders.
//!
//! `MapMenuState` holds the visited-room set, per-room geometry
//! (`MapRoomNode`), open/minimap toggles, and the clamped zoom level
//! (`MAP_ZOOM_MIN`..`MAP_ZOOM_MAX`). `summary_lines` produces the text the HUD
//! shows; a host-owned UI adapter can render it as a full map, minimap, or menu tab.

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

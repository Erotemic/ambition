//! Map-state hydration systems that feed `MapMenuState`: `track_room_visits`
//! records the active room (and persists a `room_visited_<id>` save flag),
//! `sync_map_from_save` replays those flags into the visited set on load, and
//! `populate_map_rooms` fills room geometry from the LDtk project levels.

use bevy::prelude::*;

use super::model::{MapMenuState, MapRoomNode};

pub fn track_room_visits(
    room_set: Res<crate::rooms::RoomSet>,
    mut map: ResMut<MapMenuState>,
    mut last: Local<Option<String>>,
    mut save: ResMut<ambition_persistence::save::SandboxSave>,
) {
    let current = room_set.active_spec().id.clone();
    if last.as_deref() == Some(current.as_str()) {
        return;
    }
    *last = Some(current.clone());
    map.record_visit(&current);
    save.data_mut()
        .set_flag(format!("room_visited_{current}"), true);
}

pub fn sync_map_from_save(
    save: Res<ambition_persistence::save::SandboxSave>,
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

pub fn populate_map_rooms(
    project: Res<crate::ldtk_world::SandboxLdtkProject>,
    mut map: ResMut<MapMenuState>,
) {
    if !map.rooms.is_empty() {
        return;
    }
    for level in &project.0.levels {
        map.rooms.push(MapRoomNode {
            id: level.identifier.clone(),
            world_min: Vec2::new(level.world_x as f32, level.world_y as f32),
            world_size: Vec2::new(level.px_wid as f32, level.px_hei as f32),
        });
    }
}

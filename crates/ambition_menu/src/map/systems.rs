//! Map-state hydration systems that feed `MapMenuState`: `track_room_visits`
//! records the active room (and persists a `room_visited_<id>` save flag),
//! `sync_map_from_save` replays those flags into the visited set on load, and
//! `populate_map_rooms` fills room geometry from the LDtk project levels.

use bevy::prelude::*;

use super::MapMenuState;
// only `populate_map_rooms` builds one, and that is behind `ldtk`.
#[cfg(feature = "ldtk")]
use super::MapRoomNode;

/// The one spelling of "the player has been in this room", as a save flag.
///
/// ⭐ IT WAS TWO, ELEVEN LINES APART. `track_room_visits` wrote
/// `format!("room_visited_{id}")` and `sync_map_from_save` read
/// `strip_prefix("room_visited_")`, so renaming either one was SILENT in the other
/// direction: the writer would stamp a flag the reader no longer recognised, the
/// map would come back empty from every load, and both functions would still read
/// correctly on their own. The agreement was the fact worth guarding, and it now
/// cannot disagree.
pub const ROOM_VISITED_FLAG_PREFIX: &str = "room_visited_";

/// The save flag id that records a visit to `room_id`.
pub fn room_visited_flag(room_id: &str) -> String {
    format!("{ROOM_VISITED_FLAG_PREFIX}{room_id}")
}

/// The room a visit flag names, or `None` for any other flag.
pub fn room_from_visited_flag(flag_id: &str) -> Option<&str> {
    flag_id.strip_prefix(ROOM_VISITED_FLAG_PREFIX)
}

pub fn track_room_visits(
    room_set: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
        ambition_platformer2d_world::rooms::RoomSet,
    >,
    mut map: ResMut<MapMenuState>,
    mut last: Local<Option<String>>,
    mut save: ResMut<ambition_persistence::save::AmbitionGameSave>,
) {
    let current = room_set.active_spec().id.clone();
    if last.as_deref() == Some(current.as_str()) {
        return;
    }
    *last = Some(current.clone());
    map.record_visit(&current);
    save.data_mut().set_flag(room_visited_flag(&current), true);
}

pub fn sync_map_from_save(
    save: Res<ambition_persistence::save::AmbitionGameSave>,
    mut map: ResMut<MapMenuState>,
    mut hydrated: Local<bool>,
) {
    if *hydrated {
        return;
    }
    *hydrated = true;
    for flag in save.data().flags() {
        if let Some(room_id) = room_from_visited_flag(&flag.id) {
            map.record_visit(room_id);
        }
    }
}

/// Fill room geometry from the LDtk project levels. Behind `ldtk` because
/// the map is drawable without a backend — only the room RECTANGLES need one.
#[cfg(feature = "ldtk")]
pub fn populate_map_rooms(
    project: Res<ambition_platformer2d_ldtk::ActiveLdtkProject>,
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

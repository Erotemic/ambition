//! Map-state hydration systems that feed `MapMenuState`: `track_room_visits`
//! records the active room (and persists a `room_visited_<id>` save flag),
//! `sync_map_from_save` replays those flags into the visited set on load, and
//! `populate_map_rooms` fills room geometry from the LDtk project levels.

use bevy::prelude::*;

use super::MapMenuState;
// Only `populate_map_rooms` builds one, and it is behind `ldtk`.
#[cfg(feature = "ldtk")]
use super::MapRoomNode;

/// The one spelling of "the player has been in this room", as a save flag.
///
/// The writer (`track_room_visits`) and reader (`sync_map_from_save`) share
/// this prefix, so renaming it cannot make them disagree.
pub const ROOM_VISITED_FLAG_PREFIX: &str = "room_visited_";

/// The save flag id that records a visit to `room_id`.
pub fn room_visited_flag(room_id: &str) -> String {
    format!("{ROOM_VISITED_FLAG_PREFIX}{room_id}")
}

/// The room a visit flag names, or `None` for any other flag.
pub fn room_from_visited_flag(flag_id: &str) -> Option<&str> {
    flag_id.strip_prefix(ROOM_VISITED_FLAG_PREFIX)
}

/// Record the active room on the save, once per room per save.
///
/// The edge comes from the save ("is this room flagged"), not a `Local`, so
/// it has the save's lifetime and re-derives after a new game, a rewind, or a
/// second session.
///
/// Writes only on the edge and otherwise reads through `Deref`: a `ResMut`
/// deref-mut marks the save changed for every reader, including autosave.
pub fn track_room_visits(
    room_set: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
        ambition_platformer2d_world::rooms::RoomSet,
    >,
    mut save: ResMut<ambition_persistence::save::AmbitionGameSave>,
) {
    let flag = room_visited_flag(&room_set.active_spec().id);
    if save.data().flag(&flag) {
        return;
    }
    save.data_mut().set_flag(flag, true);
}

/// Keep the map's visited set equal to what the save says.
///
/// The visited set is a projection of the save, with the save's lifetime.
/// `MapMenuState` is process-lived UI state; only this field mirrors the
/// save, and it is rebuilt whenever the save changes (reset, load, new
/// visit). So a new game does not inherit the old map, and a second save is
/// read.
///
/// Assigned only when different: the map view rebuilds on
/// `map.is_changed()`, and the save changes often for other reasons.
pub fn sync_map_from_save(
    save: Res<ambition_persistence::save::AmbitionGameSave>,
    mut map: ResMut<MapMenuState>,
) {
    if !save.is_changed() {
        return;
    }
    let visited: std::collections::BTreeSet<String> = save
        .data()
        .flags()
        .iter()
        .filter_map(|flag| room_from_visited_flag(&flag.id).map(str::to_string))
        .collect();
    if map.visited != visited {
        map.visited = visited;
    }
}

/// Fill room geometry from the LDtk project levels. Behind `ldtk`: the map
/// draws without a backend; only the room rectangles need one.
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

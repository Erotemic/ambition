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

/// Record the active room on the save, once per room per SAVE.
///
/// ⛔⛔ THE EDGE IS DERIVED FROM THE SAVE, NOT FROM A `Local`. This kept
/// `Local<Option<String>>` of the last room and wrote nothing while it matched
/// -- process-lived memory standing in for a fact the save already holds. After
/// a new game the player stands in the same room they were in, the local still
/// says so, and the fresh save never learns they are there. Asking the save
/// "is this room flagged" is the same edge with the save's lifetime, and it
/// re-derives correctly after a reset, a rewind or a second session. A GPT
/// review found the lifetime 2026-09-07.
///
/// ⚠ WRITES ONLY ON THE EDGE, and reads through `Deref` otherwise: a `ResMut`
/// deref-mut marks the save changed for every reader downstream, and the
/// autosave is one of them.
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
/// ⛔⛔ THE VISITED SET IS A PROJECTION OF THE SAVE, with the save's lifetime,
/// and it used to be hydrated ONCE PER PROCESS behind a `Local<bool>`. Two
/// failures followed: a new game inherited the old game's map (the save was
/// wiped, the set was not), and a second save activated in the same process was
/// never read at all (the local already said "done"). `MapMenuState` is
/// process-lived UI state; only this field mirrors the save, and it is rebuilt
/// whenever the save CHANGES -- which is what a reset, a load and a fresh visit
/// all are -- rather than keyed to a moment nothing else remembers.
///
/// ⚠ ASSIGNED ONLY WHEN DIFFERENT. The map view rebuilds on `map.is_changed()`,
/// and a save that changes every tick for other reasons must not redraw a map
/// that did not.
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

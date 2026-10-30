//! What the Map page shows: the current room's region of the world.
//!
//! The base is the LDtk layout. Rooms joined by an edge exit (a side exit or a
//! vertical one) are one region: they are placed on a faithful flat map, so
//! their world rectangles are a map as they stand. A door enters a building,
//! whose inside is placed anywhere in the world, so a door does not join
//! regions. A building's inside is a region of its own. A big dungeon's rooms,
//! joined by edge exits, are a region too.
//!
//! Inside a building of one room, the map shows the region its door is in,
//! and marks that door.
//!
//! [`map_view`] is pure: the menu pages build from its [`MapView`], a value,
//! so a renderer compares it by value to know when to rebuild.

use std::collections::{BTreeSet, VecDeque};

use bevy::math::Vec2;

use super::{MapMenuState, MapRoomNode};

/// A door in a room, and the room it leads to.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MapDoor {
    /// The door's middle, in the room's own coordinates.
    pub at: Vec2,
    pub target_room: String,
}

/// One room as the map draws it.
#[derive(Clone, Debug, PartialEq)]
pub struct MapViewRoom {
    pub id: String,
    pub title: String,
    /// The room's rectangle as fractions `0..=1` of the map's frame (x right,
    /// y down). The frame keeps the region's proportions at [`MapView::aspect`].
    pub min: Vec2,
    pub size: Vec2,
    pub visited: bool,
    /// The room the player is in, or (inside a building) the room whose door
    /// they went in by.
    pub here: bool,
}

/// The map of one region.
#[derive(Clone, Debug, PartialEq)]
pub struct MapView {
    /// The title of the room the player is in.
    pub here_title: String,
    /// The building the player is inside, when the map shows the region
    /// outside it.
    pub inside: Option<String>,
    pub rooms: Vec<MapViewRoom>,
    /// Doors to show, in map fractions. Inside a building, its own door is the
    /// first and is marked [`MapView::marked_door`].
    pub doors: Vec<Vec2>,
    pub marked_door: Option<usize>,
    /// Width over height of the frame the fractions are of.
    pub aspect: f32,
}

/// The room `id` names: its level identifier or its area.
fn find(rooms: &[MapRoomNode], id: &str) -> Option<usize> {
    rooms.iter().position(|room| room.id == id || room.area == id)
}

/// The rooms joined to `start` by edge exits, `start` first. An exit joins
/// both ways, whichever side names it.
pub fn region(rooms: &[MapRoomNode], start: usize) -> Vec<usize> {
    let mut seen = BTreeSet::from([start]);
    let mut queue = VecDeque::from([start]);
    let mut order = Vec::new();
    while let Some(index) = queue.pop_front() {
        order.push(index);
        let room = &rooms[index];
        let forward = room.edge_exits.iter().filter_map(|target| find(rooms, target));
        let backward = rooms.iter().enumerate().filter(|(_, other)| {
            other.world == room.world && other.edge_exits.iter().any(|target| *target == room.id || *target == room.area)
        });
        for next in forward.chain(backward.map(|(i, _)| i)).collect::<Vec<_>>() {
            if rooms[next].world == room.world && seen.insert(next) {
                queue.push_back(next);
            }
        }
    }
    order
}

/// The map of the region the player is in, or `None` when the room is not on
/// the map.
///
/// Fog of war: a visited room is drawn; an unvisited room an edge exit leads
/// to from a visited one is drawn as not yet visited; the rest of the region
/// is not drawn. The current room counts as visited.
pub fn map_view(state: &MapMenuState, current: &str, aspect: f32) -> Option<MapView> {
    let rooms = &state.rooms;
    let here = find(rooms, current)?;
    let here_title = rooms[here].title.clone();
    let mut region_rooms = region(rooms, here);
    let mut anchor = here;
    let mut inside = None;
    let mut door_at = None;
    // Inside a building of one room: the region outside, by its door.
    if region_rooms.len() == 1 {
        let outside = rooms.iter().enumerate().find_map(|(i, room)| {
            (i != here).then_some(())?;
            let door = room.doors.iter().find(|door| door.target_room == rooms[here].id || door.target_room == rooms[here].area)?;
            Some((i, door.at))
        });
        if let Some((outside, at)) = outside {
            region_rooms = region(rooms, outside);
            anchor = outside;
            inside = Some(here_title.clone());
            door_at = Some(rooms[outside].world_min + at);
        }
    }
    let visited = |i: usize| i == here || i == anchor || state.visited.contains(&rooms[i].id) || state.visited.contains(&rooms[i].area);
    // An edge exit joins both ways, whichever side names it.
    let names = |from: usize, to: usize| rooms[from].edge_exits.iter().any(|t| *t == rooms[to].id || *t == rooms[to].area);
    let shown: Vec<usize> = region_rooms
        .iter()
        .copied()
        .filter(|&i| visited(i) || region_rooms.iter().any(|&j| visited(j) && (names(j, i) || names(i, j))))
        .collect();
    let lo = shown.iter().map(|&i| rooms[i].world_min).reduce(Vec2::min)?;
    let hi = shown.iter().map(|&i| rooms[i].world_min + rooms[i].world_size).reduce(Vec2::max)?;
    // Fit the region in a frame of `aspect`, centred, its proportions kept.
    let extent = (hi - lo).max(Vec2::ONE);
    let scale = (aspect / extent.x).min(1.0 / extent.y);
    let used = extent * scale;
    let offset = Vec2::new((aspect - used.x) * 0.5, (1.0 - used.y) * 0.5);
    let to_frame = |p: Vec2| {
        let q = (p - lo) * scale + offset;
        Vec2::new(q.x / aspect, q.y)
    };
    let mut doors = Vec::new();
    let mut marked_door = None;
    if let Some(at) = door_at {
        marked_door = Some(0);
        doors.push(to_frame(at));
    }
    for &i in &shown {
        if visited(i) {
            for door in &rooms[i].doors {
                let at = rooms[i].world_min + door.at;
                if Some(at) != door_at {
                    doors.push(to_frame(at));
                }
            }
        }
    }
    let view_rooms = shown
        .iter()
        .map(|&i| {
            let room = &rooms[i];
            let min = to_frame(room.world_min);
            let max = to_frame(room.world_min + room.world_size);
            MapViewRoom {
                id: room.id.clone(),
                title: room.title.clone(),
                min,
                size: max - min,
                visited: visited(i),
                here: i == anchor,
            }
        })
        .collect();
    Some(MapView { here_title, inside, rooms: view_rooms, doors, marked_door, aspect })
}

/// A readable name for a room with no title of its own: its id, words
/// capitalised (`pirate_wreckers_reef` -> `Pirate Wreckers Reef`).
pub fn title_from_id(id: &str) -> String {
    id.split('_')
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            chars.next().map(|c| c.to_uppercase().chain(chars).collect::<String>()).unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

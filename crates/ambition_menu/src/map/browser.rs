//! Map hierarchy and navigation for menu renderers.

use std::collections::{BTreeMap, BTreeSet};

use bevy::prelude::*;

use super::{
    view::{title_from_id, MapPin},
    MapRoomNode, MAP_ZOOM_MIN, MAP_ZOOM_STEP,
};

/// Optional names and room groups supplied by a game. Group paths are relative to a world.
#[derive(Resource, Default)]
pub struct MapHierarchy {
    pub root_title: Option<String>,
    pub world_titles: BTreeMap<String, String>,
    /// Group path for each playable room ID.
    pub room_groups: BTreeMap<String, Vec<String>>,
}

/// The discovery and travel rules for the menu owner.
#[derive(Clone, Copy)]
pub struct MapAccess<'a> {
    pub current_room: Option<&'a str>,
    pub visited: &'a BTreeSet<String>,
    pub reveal_all: bool,
    pub can_travel: bool,
}

/// The identity of an entry in the current atlas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MapEntryId(usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MapAction {
    Open(MapEntryId),
    Move(i8, i8),
    Confirm,
    Up,
    Browse(i8),
    Zoom(i8),
    Pan(i8, i8),
    Fit,
    Travel,
    Canvas,
}

#[derive(Debug, PartialEq, Eq)]
pub enum MapOutcome<'a> {
    Stay,
    Close,
    TravelTo(&'a str),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MapEntryKind {
    Group,
    Landmark,
    Room,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MapEntryView {
    pub id: MapEntryId,
    pub kind: MapEntryKind,
    pub title: String,
    /// Visible pieces in viewport fractions, with Y down.
    pub pieces: Vec<Rect>,
    pub here: bool,
    pub visited: bool,
    pub selected: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MapBrowserView {
    pub title: String,
    pub parent_title: Option<String>,
    pub entries: Vec<MapEntryView>,
    pub room_detail: bool,
    pub focused_tool: Option<MapAction>,
    pub tools: Vec<MapAction>,
    pub doors: Vec<Vec2>,
    /// Passages between visible rooms, in viewport coordinates.
    pub links: Vec<[Vec2; 2]>,
    pub pins: Vec<MapPin>,
}

struct Entry<'a> {
    parent: MapEntryId,
    children: Vec<MapEntryId>,
    title: String,
    pieces: Vec<&'a MapRoomNode>,
    entrances: Vec<Entrance<'a>>,
    key: Vec<&'a str>,
}

struct Entrance<'a> {
    outside: &'a MapRoomNode,
    destination: MapEntryId,
    at: Vec2,
}

impl<'a> Entry<'a> {
    fn room(&self) -> Option<&'a MapRoomNode> {
        self.pieces.first().copied()
    }

    fn room_bounds(&self) -> Option<Rect> {
        self.pieces
            .iter()
            .map(|room| room_rect(room))
            .reduce(|a, b| a.union(b))
    }
}

fn room_id(room: &MapRoomNode) -> &str {
    if room.area.is_empty() {
        &room.id
    } else {
        &room.area
    }
}

/// Build the map from room definitions. A game can supply any number of group levels.
pub struct MapAtlas<'a> {
    entries: Vec<Entry<'a>>,
    room_ids: BTreeMap<&'a str, MapEntryId>,
    piece_ids: BTreeMap<&'a str, MapEntryId>,
}

const ROOT: MapEntryId = MapEntryId(0);
const MAP_BROWSER_ZOOM_MAX: f32 = 16.0;
const OPENING_ZOOM_MAX: f32 = 6.0;
const ROOM_CONTEXT: f32 = 2.5;

impl<'a> MapAtlas<'a> {
    pub fn new(
        rooms: impl IntoIterator<Item = &'a MapRoomNode>,
        hierarchy: Option<&'a MapHierarchy>,
    ) -> Self {
        let mut atlas = Self {
            entries: vec![Entry {
                parent: ROOT,
                children: Vec::new(),
                title: hierarchy
                    .and_then(|h| h.root_title.clone())
                    .unwrap_or_else(|| "Dimensions".into()),
                pieces: Vec::new(),
                entrances: Vec::new(),
                key: Vec::new(),
            }],
            room_ids: BTreeMap::new(),
            piece_ids: BTreeMap::new(),
        };
        let mut groups = BTreeMap::new();
        for room in rooms {
            let room_id = room_id(room);
            if let Some(id) = atlas.room_ids.get(room_id) {
                atlas.entries[id.0].pieces.push(room);
                atlas.piece_ids.insert(&room.id, *id);
                continue;
            }
            let mut key = vec![room.world.as_str()];
            let mut parent = *groups.entry(key.clone()).or_insert_with(|| {
                let title = hierarchy
                    .and_then(|h| h.world_titles.get(&room.world))
                    .cloned()
                    .unwrap_or_else(|| world_title(&room.world));
                atlas.insert(ROOT, title, None, key.clone())
            });
            let path = hierarchy
                .and_then(|h| h.room_groups.get(room_id))
                .unwrap_or(&room.group_path);
            for group in path {
                key.push(group);
                parent = *groups
                    .entry(key.clone())
                    .or_insert_with(|| atlas.insert(parent, group.clone(), None, key.clone()));
            }
            key.push(room_id);
            let id = atlas.insert(parent, room.title.clone(), Some(room), key);
            atlas.room_ids.insert(room_id, id);
            atlas.piece_ids.insert(&room.id, id);
        }
        atlas.place_entrances();
        atlas
    }

    fn place_entrances(&mut self) {
        let mut entrances = Vec::new();
        for (&id, entry) in self.room_ids.values().map(|id| (id, &self.entries[id.0])) {
            for &room in &entry.pieces {
                for door in &room.doors {
                    if let Some(destination) = self.find_room(&door.target_room) {
                        if let Some(group) = self.entered_group(id, destination) {
                            entrances.push((group, Entrance { outside: room, destination, at: room.world_min + door.at }));
                        }
                    }
                }
            }
        }
        for (group, entrance) in entrances {
            self.entries[group.0].entrances.push(entrance);
        }
    }

    fn entered_group(&self, outside: MapEntryId, target: MapEntryId) -> Option<MapEntryId> {
        let parent = self.entries[outside.0].parent;
        let mut child = self.entries[target.0].parent;
        while child != ROOT && child != parent {
            let ancestor = self.entries[child.0].parent;
            if ancestor == parent {
                return Some(child);
            }
            child = ancestor;
        }
        None
    }

    fn insert(
        &mut self,
        parent: MapEntryId,
        title: String,
        room: Option<&'a MapRoomNode>,
        key: Vec<&'a str>,
    ) -> MapEntryId {
        let id = MapEntryId(self.entries.len());
        self.entries.push(Entry {
            parent,
            children: Vec::new(),
            title,
            pieces: room.into_iter().collect(),
            entrances: Vec::new(),
            key,
        });
        self.entries[parent.0].children.push(id);
        id
    }

    fn is_here(&self, id: MapEntryId, current: Option<MapEntryId>) -> bool {
        let Some(mut current) = current else {
            return false;
        };
        loop {
            if id == current {
                return true;
            }
            if current == ROOT {
                return false;
            }
            current = self.entries[current.0].parent;
        }
    }

    fn find_room(&self, id: &str) -> Option<MapEntryId> {
        self.room_ids.get(id).or_else(|| self.piece_ids.get(id)).copied()
    }
}

fn world_title(id: &str) -> String {
    let id = id.strip_prefix("world.").unwrap_or(id);
    if id.is_empty() {
        return "World".into();
    }
    title_from_id(id.strip_suffix("_ldtk").unwrap_or(id))
}

/// Camera, location, and selection for one menu. A new browser centers on the owner's room.
#[derive(Component, Default)]
pub struct MapBrowser {
    location: Option<Location>,
    selected: Option<Location>,
    tool: Option<MapAction>,
    viewport: Viewport,
}

#[derive(Default)]
enum Viewport {
    #[default]
    FollowRoom,
    Fit,
    Manual {
        centre: Vec2,
        zoom: f32,
    },
}

struct Frame {
    rect: Rect,
    zoom: f32,
}

struct Location {
    key: Vec<String>,
    is_room: bool,
}

impl Location {
    fn of(entry: &Entry) -> Self {
        Self {
            key: entry.key.iter().map(|part| (*part).to_owned()).collect(),
            is_room: entry.room().is_some(),
        }
    }

    fn matches(&self, entry: &Entry) -> bool {
        entry.room().is_some() == self.is_room && entry.key == self.key
    }
}

struct Level<'a> {
    id: MapEntryId,
    bounds: Rect,
    entries: Vec<PlacedEntry>,
    current: Option<MapEntryId>,
    atlas: &'a MapAtlas<'a>,
}

struct PlacedEntry {
    id: MapEntryId,
    bounds: Rect,
    visited: bool,
    markers: Vec<Rect>,
}

impl PlacedEntry {
    fn room(id: MapEntryId, bounds: Rect, visited: bool) -> Self {
        Self { id, bounds, visited, markers: Vec::new() }
    }
}

fn entrance_marker(entrance: &Entrance) -> Rect {
    let width = (entrance.outside.world_size.x * 0.32).clamp(260.0, 800.0);
    let height = (entrance.outside.world_size.y * 0.30).clamp(120.0, 280.0);
    Rect::from_center_size(entrance.at - Vec2::Y * height, Vec2::new(width, height))
}

impl MapBrowser {
    /// Get the focused tool. A menu can use this to leave the map's controls.
    pub fn focused_tool(&self) -> Option<MapAction> {
        self.tool
    }

    fn level<'a>(&self, atlas: &'a MapAtlas<'a>, access: MapAccess<'_>) -> Level<'a> {
        let current = access.current_room.and_then(|room| atlas.find_room(room));
        let id = match &self.location {
            Some(location) => atlas
                .entries
                .iter()
                .position(|entry| location.matches(entry))
                .map(MapEntryId),
            None => current.map(|room| atlas.entries[room.0].parent),
        }
        .unwrap_or(ROOT);
        let level = &atlas.entries[id.0];
        let visited = |room: &MapRoomNode| {
            access.reveal_all
                || current == atlas.room_ids.get(room_id(room)).copied()
                || access.visited.contains(&room.id)
                || access.visited.contains(&room.area)
        };
        let visible = |room: &MapRoomNode| {
            visited(room)
                || atlas
                    .entries
                    .iter()
                    .flat_map(|entry| entry.pieces.iter().copied())
                    .any(|other| {
                        other.world == room.world
                            && visited(other)
                            && (super::view::open_edge(other, room)
                                || super::view::open_edge(room, other)
                                || (!level.key.is_empty()
                                    && level.parent != ROOT
                                    && atlas.entries[atlas.room_ids[room_id(other)].0].parent == id
                                    && other.doors.iter().any(|door| atlas.find_room(&door.target_room)
                                        == atlas.room_ids.get(room_id(room)).copied())))
                    })
        };
        let mut entries = Vec::new();
        let mut bounds: Option<Rect> = None;
        if let Some(rect) = level.room_bounds() {
            entries.push(PlacedEntry::room(id, rect, level.pieces.iter().any(|piece| visited(piece))));
            bounds = Some(rect);
        }
        for &child in &level.children {
            let item = &atlas.entries[child.0];
            if item.pieces.iter().any(|piece| visible(piece))
                || level.entrances.iter().any(|entrance| entrance.destination == child && visited(entrance.outside))
            {
                let rect = item.room_bounds().expect("a visible room has a piece");
                entries.push(PlacedEntry::room(child, rect, item.pieces.iter().any(|piece| visited(piece))));
                bounds = Some(bounds.map_or(rect, |b| b.union(rect)));
            }
        }
        // Give each group a card in its parent's coordinate space.
        let card_origin = bounds.map_or(Vec2::ZERO, |b| Vec2::new(b.min.x, b.min.y - 480.0));
        let card_size = bounds.map_or(200.0, |b| (b.width() / 4.0).clamp(200.0, 1400.0));
        for (i, child) in level
            .children
            .iter()
            .copied()
            .filter(|child| {
                let group = &atlas.entries[child.0];
                group.pieces.is_empty() && group.entrances.is_empty()
            })
            .enumerate()
        {
            let row = (i / 4) as f32;
            let column = (i % 4) as f32;
            let spacing = Vec2::new(
                card_size * 1.08,
                if bounds.is_none() { 116.0 } else { -116.0 },
            );
            let min = card_origin + Vec2::new(column, row) * spacing;
            let rect = Rect::from_corners(min, min + Vec2::new(card_size, 100.0));
            entries.push(PlacedEntry { id: child, bounds: rect, visited: true, markers: vec![rect] });
        }
        for &child in &level.children {
            let group = &atlas.entries[child.0];
            let here = atlas.is_here(child, current);
            let mut markers: Vec<_> = group.entrances.iter()
                .filter(|entrance| visited(entrance.outside)
                    || atlas.entries[entrance.destination.0].pieces.iter().any(|piece| visited(piece)))
                .map(entrance_marker)
                .collect();
            if markers.is_empty() && here {
                markers.extend(group.entrances.first().map(entrance_marker));
            }
            if let Some(bounds) = markers.iter().copied().reduce(|a, b| a.union(b)) {
                entries.push(PlacedEntry { id: child, bounds, visited: true, markers });
            }
        }
        let bounds = entries
            .iter()
            .map(|entry| entry.bounds)
            .reduce(|a, b| a.union(b))
            .unwrap_or(Rect::from_corners(Vec2::ZERO, Vec2::ONE));
        Level {
            id,
            bounds,
            entries,
            current,
            atlas,
        }
    }

    fn selected(&self, level: &Level) -> Option<MapEntryId> {
        self.selected
            .as_ref()
            .and_then(|location| {
                level
                    .entries
                    .iter()
                    .find(|entry| location.matches(&level.atlas.entries[entry.id.0]))
                    .map(|entry| entry.id)
            })
            .or_else(|| {
                level
                    .entries
                    .iter()
                    .find(|entry| level.atlas.is_here(entry.id, level.current))
                    .map(|entry| entry.id)
            })
            .or_else(|| level.entries.first().map(|entry| entry.id))
    }

    fn frame(&self, level: &Level, aspect: f32) -> Frame {
        let aspect = if aspect.is_finite() {
            aspect.clamp(0.1, 10.0)
        } else {
            1.0
        };
        let bounds = level.bounds;
        let level_height = fit_height(bounds, aspect);
        let fitted = (bounds.center(), 1.0);
        let (centre, zoom) = match self.viewport {
            Viewport::Fit => fitted,
            Viewport::FollowRoom => {
                level
                    .current
                    .and_then(|id| level.entry_bounds(id))
                    .map_or(fitted, |room| {
                        let room_height = fit_height(room, aspect) * ROOM_CONTEXT;
                        let zoom = (level_height / room_height).clamp(1.0, OPENING_ZOOM_MAX);
                        if zoom > 1.0 {
                            (room.center(), zoom)
                        } else {
                            fitted
                        }
                    })
            }
            Viewport::Manual { centre, zoom } => (centre, zoom),
        };
        let extent = Vec2::new(level_height * aspect, level_height) / zoom;
        Frame {
            rect: Rect::from_center_size(clamp_centre(bounds, centre, extent), extent),
            zoom,
        }
    }

    fn tools(&self, level: &Level, travel_allowed: bool) -> Vec<MapAction> {
        let mut tools = Vec::new();
        if level.id != ROOT {
            tools.push(MapAction::Up);
        }
        tools.extend([
            MapAction::Zoom(-1),
            MapAction::Fit,
            MapAction::Zoom(1),
            MapAction::Pan(-1, 0),
            MapAction::Pan(0, -1),
            MapAction::Pan(0, 1),
            MapAction::Pan(1, 0),
        ]);
        if level.entries.len() > 1 {
            tools.extend([MapAction::Browse(-1), MapAction::Browse(1)]);
        }
        if travel_allowed && level.atlas.entries[level.id.0].room().is_some() {
            tools.push(MapAction::Travel);
        }
        tools
    }

    /// Project this level to a viewport with the specified aspect ratio.
    pub fn view(&self, atlas: &MapAtlas, access: MapAccess<'_>, aspect: f32) -> MapBrowserView {
        let level = self.level(atlas, access);
        let entry = &atlas.entries[level.id.0];
        let frame = self.frame(&level, aspect).rect;
        let point = |p| (p - frame.min) / frame.size();
        let selected = self.selected(&level);
        let tools = self.tools(&level, access.can_travel);
        let mut doors = Vec::new();
        let mut pins = Vec::new();
        let links = if entry.parent != ROOT && entry.room().is_none() {
            level.links().map(|[from, to]| [point(from), point(to)]).collect()
        } else {
            Vec::new()
        };
        let entries = level
            .entries
            .iter()
            .map(|placed| {
                let id = placed.id;
                let visited = placed.visited;
                let item = &atlas.entries[id.0];
                let project = |rect: Rect| {
                    let clip = rect.intersect(frame);
                    (!clip.is_empty()).then(|| Rect::from_corners(point(clip.min), point(clip.max)))
                };
                let mut pieces: Vec<_> = placed.markers.iter().copied().filter_map(project).collect();
                for room in &item.pieces {
                    let Some(piece) = project(room_rect(room)) else {
                        continue;
                    };
                    pieces.push(piece);
                    // Leave space between markers at the scale of the visible piece.
                    if visited && piece.size().cmpge(Vec2::new(0.035, 0.045)).all() {
                        doors.extend(
                            room.doors
                                .iter()
                                .filter(|door| atlas.find_room(&door.target_room)
                                    .is_none_or(|target| atlas.entered_group(id, target).is_none()))
                                .map(|door| room.world_min + door.at)
                                .filter(|&at| frame.contains(at))
                                .map(point),
                        );
                        pins.extend(room.pins.iter().filter_map(|pin| {
                            let at = room.world_min + pin.at;
                            frame.contains(at).then(|| MapPin {
                                at: point(at),
                                kind: pin.kind,
                            })
                        }));
                    }
                }
                MapEntryView {
                    id,
                    kind: if item.room().is_some() {
                        MapEntryKind::Room
                    } else if !item.entrances.is_empty() {
                        MapEntryKind::Landmark
                    } else {
                        MapEntryKind::Group
                    },
                    title: item.title.clone(),
                    pieces,
                    here: atlas.is_here(id, level.current),
                    visited,
                    selected: selected == Some(id),
                }
            })
            .collect();
        MapBrowserView {
            title: entry.title.clone(),
            parent_title: (level.id != ROOT).then(|| atlas.entries[entry.parent.0].title.clone()),
            entries,
            room_detail: entry.room().is_some(),
            focused_tool: self.tool.filter(|tool| tools.contains(tool)),
            tools,
            doors,
            links,
            pins,
        }
    }

    /// Pan by fractions of the viewport and zoom by steps. A step can be fractional.
    pub fn adjust_viewport(
        &mut self,
        atlas: &MapAtlas,
        access: MapAccess<'_>,
        aspect: f32,
        pan: Vec2,
        zoom_steps: f32,
    ) {
        let level = self.level(atlas, access);
        if zoom_steps.is_finite() && zoom_steps != 0.0 {
            self.zoom_level(&level, aspect, zoom_steps);
        }
        if pan.is_finite() && pan != Vec2::ZERO {
            self.pan_level(&level, aspect, pan);
        }
    }

    /// Scale the viewport by the change in distance between two touch points.
    /// Keep the map point at `from` under `to`. Both positions are viewport fractions.
    pub fn pinch_viewport(
        &mut self,
        atlas: &MapAtlas,
        access: MapAccess<'_>,
        aspect: f32,
        from: Vec2,
        to: Vec2,
        scale: f32,
    ) {
        if !from.is_finite()
            || !to.is_finite()
            || !scale.is_finite()
            || scale <= 0.0
            || (from == to && scale == 1.0)
        {
            return;
        }
        let level = self.level(atlas, access);
        let Frame { rect, zoom } = self.frame(&level, aspect);
        let new_zoom = (zoom * scale).clamp(MAP_ZOOM_MIN, MAP_BROWSER_ZOOM_MAX);
        let extent = rect.size() * (zoom / new_zoom);
        let anchor = rect.min + rect.size() * from;
        let centre = clamp_centre(
            level.bounds,
            anchor - extent * (to - Vec2::splat(0.5)),
            extent,
        );
        self.set_viewport(
            &level,
            Viewport::Manual {
                centre,
                zoom: new_zoom,
            },
        );
    }

    fn zoom_level(&mut self, level: &Level, aspect: f32, steps: f32) {
        let Frame { rect, zoom } = self.frame(level, aspect);
        let centre =
            if steps > 0.0 && zoom == 1.0 && !matches!(self.viewport, Viewport::Manual { .. }) {
                self.selected(level)
                    .and_then(|id| level.entry_bounds(id))
                    .map_or(rect.center(), |bounds| bounds.center())
            } else {
                rect.center()
            };
        let zoom = (zoom * MAP_ZOOM_STEP.powf(steps)).clamp(MAP_ZOOM_MIN, MAP_BROWSER_ZOOM_MAX);
        self.set_viewport(level, Viewport::Manual { centre, zoom });
    }

    fn pan_level(&mut self, level: &Level, aspect: f32, delta: Vec2) {
        let Frame { rect, zoom } = self.frame(level, aspect);
        let centre = clamp_centre(
            level.bounds,
            rect.center() + rect.size() * delta,
            rect.size(),
        );
        self.set_viewport(level, Viewport::Manual { centre, zoom });
    }

    fn set_viewport(&mut self, level: &Level, viewport: Viewport) {
        self.location
            .get_or_insert_with(|| Location::of(&level.atlas.entries[level.id.0]));
        self.viewport = viewport;
    }

    /// Apply one input. The host owns the effect of an accepted travel request.
    pub fn apply<'a>(
        &mut self,
        atlas: &MapAtlas<'a>,
        access: MapAccess<'_>,
        aspect: f32,
        action: MapAction,
    ) -> MapOutcome<'a> {
        let level = self.level(atlas, access);
        let tools = self.tools(&level, access.can_travel);
        self.tool = self.tool.filter(|tool| tools.contains(tool));
        let selected = self.selected(&level);
        let detail = atlas.entries[level.id.0].room();
        let action = match action {
            MapAction::Confirm => match self
                .tool
                .or_else(|| selected.filter(|_| detail.is_none()).map(MapAction::Open))
            {
                Some(action) => action,
                None => return MapOutcome::Stay,
            },
            action => action,
        };
        match action {
            MapAction::Up if level.id == ROOT => return MapOutcome::Close,
            MapAction::Up => {
                self.open(atlas, atlas.entries[level.id.0].parent);
                let parent = self.level(atlas, access);
                self.reveal(&parent, level.id, aspect);
            }
            MapAction::Open(id)
                if detail.is_none() && level.entries.iter().any(|entry| entry.id == id) =>
            {
                self.open(atlas, id);
            }
            MapAction::Move(dx, dy) => {
                let direction = Vec2::new(f32::from(dx), f32::from(dy));
                if let Some(tool) = self.tool {
                    if dy > 0 {
                        self.tool = None;
                    } else if let Some(index) = tools.iter().position(|item| *item == tool) {
                        let next = (index as i32 + i32::from(dx)).clamp(0, tools.len() as i32 - 1);
                        self.tool = Some(tools[next as usize]);
                    }
                } else if let Some(next) = level.directional(selected, direction) {
                    self.reveal(&level, next, aspect);
                } else if dy < 0 {
                    self.tool = tools.first().copied();
                } else if dx != 0 || dy != 0 {
                    self.pan_level(&level, aspect, direction * 0.18);
                }
            }
            MapAction::Browse(step) if !level.entries.is_empty() => {
                let index = level
                    .entries
                    .iter()
                    .position(|entry| Some(entry.id) == selected)
                    .unwrap_or(0);
                let index = (index as i32 + i32::from(step)).rem_euclid(level.entries.len() as i32);
                self.reveal(&level, level.entries[index as usize].id, aspect);
            }
            MapAction::Zoom(step) => self.zoom_level(&level, aspect, f32::from(step)),
            MapAction::Pan(x, y) => {
                self.pan_level(&level, aspect, Vec2::new(f32::from(x), f32::from(y)) * 0.18)
            }
            MapAction::Fit => self.set_viewport(&level, Viewport::Fit),
            MapAction::Travel if access.can_travel => {
                if let Some(room) = detail {
                    return MapOutcome::TravelTo(room_id(room));
                }
            }
            _ => {}
        }
        MapOutcome::Stay
    }

    fn open(&mut self, atlas: &MapAtlas, id: MapEntryId) {
        let entry = &atlas.entries[id.0];
        *self = Self {
            location: Some(Location::of(entry)),
            viewport: Viewport::Fit,
            ..Self::default()
        };
    }

    fn reveal(&mut self, level: &Level, id: MapEntryId, aspect: f32) {
        self.selected = Some(Location::of(&level.atlas.entries[id.0]));
        let Some(bounds) = level.entry_bounds(id) else {
            return;
        };
        let Frame { rect, zoom } = self.frame(level, aspect);
        let centre = bounds.center();
        if !rect.contains(centre) {
            self.set_viewport(level, Viewport::Manual { centre, zoom });
        } else if self.location.is_none() {
            self.set_viewport(
                level,
                Viewport::Manual {
                    centre: rect.center(),
                    zoom,
                },
            );
        }
    }
}

impl Level<'_> {
    fn links(&self) -> impl Iterator<Item = [Vec2; 2]> + '_ {
        let rooms: BTreeSet<_> = self.entries.iter()
            .filter(|entry| entry.markers.is_empty())
            .map(|entry| entry.id.0)
            .collect();
        let mut linked = BTreeSet::new();
        let mut links = Vec::new();
        for visible in self.entries.iter().filter(|entry| entry.visited) {
            let source = &self.atlas.entries[visible.id.0];
            for room in &source.pieces {
                for door in &room.doors {
                    let Some(target) = self.atlas.find_room(&door.target_room) else { continue };
                    let (a, b) = (visible.id.0.min(target.0), visible.id.0.max(target.0));
                    if a == b || !rooms.contains(&target.0) || !linked.insert((a, b)) {
                        continue;
                    }
                    let other = &self.atlas.entries[target.0];
                    let from = room.world_min + door.at;
                    let to = other.pieces.iter().flat_map(|piece| piece.doors.iter().filter_map(|door| {
                        (self.atlas.find_room(&door.target_room) == Some(visible.id))
                            .then_some(piece.world_min + door.at)
                    }))
                    .min_by(|a, b| a.distance_squared(from).total_cmp(&b.distance_squared(from)))
                    .or_else(|| other.room_bounds().map(|rect| rect.center()));
                    links.extend(to.map(|to| [from, to]));
                }
            }
        }
        links.into_iter()
    }

    fn entry_bounds(&self, id: MapEntryId) -> Option<Rect> {
        self.entries
            .iter()
            .find(|entry| entry.id == id)
            .map(|entry| entry.markers.first().copied().unwrap_or(entry.bounds))
    }

    fn directional(&self, selected: Option<MapEntryId>, direction: Vec2) -> Option<MapEntryId> {
        let origin = self.entry_bounds(selected?)?.center();
        let direction = direction.normalize_or_zero();
        self.entries
            .iter()
            .filter_map(|entry| {
                let offset = entry.markers.first().copied().unwrap_or(entry.bounds).center() - origin;
                let ahead = offset.dot(direction);
                (ahead > f32::EPSILON).then(|| {
                    let sideways = offset.perp_dot(direction).abs();
                    (entry.id, ahead + 2.0 * sideways)
                })
            })
            .min_by(|(_, left), (_, right)| left.total_cmp(right))
            .map(|(id, _)| id)
    }
}

fn fit_height(bounds: Rect, aspect: f32) -> f32 {
    let size = bounds.size().max(Vec2::ONE);
    size.y.max(size.x / aspect) * 1.10
}

fn clamp_centre(bounds: Rect, centre: Vec2, extent: Vec2) -> Vec2 {
    let overscroll = extent * 0.25;
    centre.clamp(bounds.min - overscroll, bounds.max + overscroll)
}

fn room_rect(room: &MapRoomNode) -> Rect {
    Rect::from_corners(
        room.world_min,
        room.world_min + room.world_size.abs().max(Vec2::ONE),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::MapMenuState;

    fn room(id: &str, world: &str, min: Vec2) -> MapRoomNode {
        MapRoomNode {
            id: id.into(),
            title: id.into(),
            world: world.into(),
            world_min: min,
            world_size: Vec2::splat(300.0),
            ..Default::default()
        }
    }

    #[test]
    fn nested_groups_keep_overlapping_rooms_accessible() {
        let state = MapMenuState {
            rooms: vec![
                room("home", "world.sandbox_ldtk", Vec2::ZERO),
                room("entrance", "world.art_ldtk", Vec2::ZERO),
                room("crypt", "world.art_ldtk", Vec2::ZERO),
            ],
            reveal_all: true,
            ..Default::default()
        };
        let hierarchy = MapHierarchy {
            room_groups: BTreeMap::from([
                (
                    "entrance".into(),
                    vec!["Overworld".into(), "Dungeon".into()],
                ),
                ("crypt".into(), vec!["Overworld".into(), "Dungeon".into()]),
            ]),
            ..Default::default()
        };
        let atlas = MapAtlas::new(&state.rooms, Some(&hierarchy));
        let mut browser = MapBrowser::default();
        let access = MapAccess {
            current_room: Some("home"),
            visited: &state.visited,
            reveal_all: true,
            can_travel: true,
        };
        let view = |browser: &MapBrowser| browser.view(&atlas, access, 1.7);
        let apply = |browser: &mut MapBrowser, action| browser.apply(&atlas, access, 1.7, action);
        let initial = view(&browser);
        assert_eq!(initial.title, "Sandbox");
        assert!(initial.entries[0].selected && initial.entries[0].here);
        assert_eq!(apply(&mut browser, MapAction::Up), MapOutcome::Stay);
        assert_eq!(
            view(&browser)
                .entries
                .iter()
                .map(|item| (item.title.as_str(), item.here))
                .collect::<Vec<_>>(),
            [("Sandbox", true), ("Art", false)]
        );
        assert_eq!(apply(&mut browser, MapAction::Move(1, 0)), MapOutcome::Stay);
        for title in ["Art", "Overworld", "Dungeon"] {
            assert_eq!(apply(&mut browser, MapAction::Confirm), MapOutcome::Stay);
            assert_eq!(view(&browser).title, title);
        }
        assert_eq!(
            view(&browser).entries[0].pieces,
            view(&browser).entries[1].pieces
        );
        apply(&mut browser, MapAction::Browse(1));
        apply(&mut browser, MapAction::Confirm);
        assert_eq!(view(&browser).title, "crypt");
        assert_eq!(
            browser.apply(
                &atlas,
                MapAccess {
                    can_travel: false,
                    ..access
                },
                1.7,
                MapAction::Travel
            ),
            MapOutcome::Stay
        );
        assert_eq!(
            apply(&mut browser, MapAction::Travel),
            MapOutcome::TravelTo("crypt")
        );
        apply(&mut browser, MapAction::Up);
        assert_eq!(
            view(&browser)
                .entries
                .iter()
                .find(|item| item.selected)
                .unwrap()
                .title,
            "crypt"
        );
    }

    #[test]
    fn a_dungeon_chart_opens_at_its_entrance_and_owns_its_rooms() {
        use crate::map::view::MapDoor;
        let mut moor = room("Moor", "Country", Vec2::ZERO);
        moor.doors.push(MapDoor { at: Vec2::new(172.0, 205.0), target_room: "Vestibule".into() });
        let mut vestibule = room("Vestibule", "Country", Vec2::new(9000.0, 500.0));
        vestibule.group_path = vec!["Barrows".into()];
        vestibule.doors.push(MapDoor { at: Vec2::new(200.0, 210.0), target_room: "Crypt".into() });
        let mut crypt = room("Crypt", "Country", Vec2::new(9400.0, 950.0));
        crypt.group_path = vestibule.group_path.clone();
        let rooms = [moor, vestibule, crypt, room("Hub", "Sandbox", Vec2::ZERO)];
        let atlas = MapAtlas::new(&rooms, None);
        let visited = BTreeSet::new();
        let outside = MapAccess { current_room: Some("Moor"), visited: &visited, reveal_all: false, can_travel: false };
        let mut browser = MapBrowser::default();
        let world = browser.view(&atlas, outside, 1.7);
        assert_eq!(world.title, "Country");
        assert_eq!(world.entries.iter().map(|e| e.title.as_str()).collect::<Vec<_>>(), ["Moor", "Barrows"]);
        let barrows = &world.entries[1];
        assert_eq!(barrows.kind, MapEntryKind::Landmark);
        assert!(!barrows.pieces[0].intersect(world.entries[0].pieces[0]).is_empty());
        assert!(world.doors.is_empty());

        browser.apply(&atlas, outside, 1.7, MapAction::Open(barrows.id));
        let map = browser.view(&atlas, outside, 1.7);
        assert_eq!(map.title, "Barrows");
        assert_eq!(map.parent_title.as_deref(), Some("Country"));
        assert_eq!(map.entries.len(), 1);
        assert_eq!(map.entries[0].title, "Vestibule");
        assert!(!map.entries[0].visited);

        let inside = MapAccess { current_room: Some("Vestibule"), ..outside };
        let mut browser = MapBrowser::default();
        let map = browser.view(&atlas, inside, 1.7);
        assert!(map.entries[0].here && map.entries[0].selected);
        assert_eq!(map.entries[1].title, "Crypt");
        assert_eq!(map.links.len(), 1);
        browser.apply(&atlas, inside, 1.7, MapAction::Up);
        assert!(browser.view(&atlas, inside, 1.7).entries.iter().any(|e| e.id == barrows.id && e.here));
        browser.apply(&atlas, inside, 1.7, MapAction::Up);
        let root = browser.view(&atlas, inside, 1.7);
        assert_eq!(root.title, "Dimensions");
        assert_eq!(root.entries.iter().map(|e| e.title.as_str()).collect::<Vec<_>>(), ["Country", "Sandbox"]);
    }

    #[test]
    fn stitched_pieces_share_one_selection_and_travel_destination() {
        let mut lobby = room("hub_lobby", "sandbox", Vec2::ZERO);
        let mut basement = room("hub_basement", "sandbox", Vec2::new(0.0, 300.0));
        lobby.area = "hub".into();
        basement.area = lobby.area.clone();
        let rooms = [lobby, basement];
        let atlas = MapAtlas::new(&rooms, None);
        let mut browser = MapBrowser::default();
        let visits = BTreeSet::new();
        let access = MapAccess {
            current_room: Some("hub_basement"),
            visited: &visits,
            reveal_all: false,
            can_travel: true,
        };
        let world = browser.view(&atlas, access, 1.7);
        assert!(!world.room_detail);
        assert_eq!(world.entries.len(), 1);
        assert!(world.entries[0].here);
        assert_eq!(world.entries[0].pieces.len(), 2);
        browser.apply(&atlas, access, 1.7, MapAction::Open(world.entries[0].id));
        assert!(browser.view(&atlas, access, 1.7).room_detail);
        assert_eq!(
            browser.apply(&atlas, access, 1.7, MapAction::Travel),
            MapOutcome::TravelTo("hub")
        );
    }

    #[test]
    fn zoom_and_pan_use_each_levels_bounds() {
        let state = MapMenuState {
            rooms: vec![
                room("home", "sandbox", Vec2::ZERO),
                room("away", "sandbox", Vec2::new(2050.0, 500.0)),
                room("start", "another", Vec2::ZERO),
            ],
            reveal_all: true,
            ..Default::default()
        };
        let atlas = MapAtlas::new(&state.rooms, None);
        let mut browser = MapBrowser::default();
        let access = MapAccess {
            current_room: Some("home"),
            visited: &state.visited,
            reveal_all: true,
            can_travel: false,
        };
        let view = |browser: &MapBrowser| browser.view(&atlas, access, 1.7);
        browser.apply(&atlas, access, 1.7, MapAction::Confirm);
        for title in ["home", "Sandbox", "Dimensions"] {
            let fitted = view(&browser);
            assert_eq!(fitted.title, title);
            browser.apply(&atlas, access, 1.7, MapAction::Zoom(4));
            assert_ne!(fitted.entries, view(&browser).entries);
            let zoomed = view(&browser);
            browser.adjust_viewport(&atlas, access, 1.7, Vec2::new(0.35, 0.2), 0.0);
            assert_ne!(zoomed.entries, view(&browser).entries);
            browser.apply(&atlas, access, 1.7, MapAction::Fit);
            assert_eq!(fitted, view(&browser));
            browser.apply(&atlas, access, 1.7, MapAction::Up);
        }
    }

    #[test]
    fn pinch_tracks_its_midpoint_and_clamps_zoom() {
        let rooms = [
            room("home", "sandbox", Vec2::ZERO),
            room("away", "sandbox", Vec2::new(2050.0, 500.0)),
        ];
        let atlas = MapAtlas::new(&rooms, None);
        let visits = BTreeSet::new();
        let access = MapAccess {
            current_room: None,
            visited: &visits,
            reveal_all: true,
            can_travel: false,
        };
        let mut browser = MapBrowser::default();
        assert_eq!(browser.view(&atlas, access, 1.7).title, "Dimensions");
        let access = MapAccess {
            current_room: Some("home"),
            ..access
        };
        let frame = |browser: &MapBrowser| browser.frame(&browser.level(&atlas, access), 1.7).rect;
        let zoom = |browser: &MapBrowser| browser.frame(&browser.level(&atlas, access), 1.7).zoom;
        let from = Vec2::new(0.4, 0.55);
        let to = Vec2::new(0.65, 0.7);
        let point = |frame: Rect, at: Vec2| frame.min + frame.size() * at;
        let initial = frame(&browser);
        let anchor = point(initial, from);

        browser.pinch_viewport(&atlas, access, 1.7, from, to, 2.0);
        let zoomed = frame(&browser);
        assert!((point(zoomed, to) - anchor).length() < 0.01);
        assert!((zoomed.size() * 2.0 - initial.size()).length() < 0.01);

        browser.pinch_viewport(&atlas, access, 1.7, to, from, 0.5);
        assert!((frame(&browser).min - initial.min).length() < 0.01);
        browser.pinch_viewport(&atlas, access, 1.7, from, from, 100.0);
        assert_eq!(zoom(&browser), MAP_BROWSER_ZOOM_MAX);
        browser.pinch_viewport(&atlas, access, 1.7, from, from, 0.001);
        assert_eq!(zoom(&browser), MAP_ZOOM_MIN);
    }

    #[test]
    fn new_map_centres_on_the_room_in_its_group_and_zoom_out_shows_more() {
        let rooms = [
            room("home", "frontier", Vec2::ZERO),
            room("near", "frontier", Vec2::new(350.0, 0.0)),
            room("away", "frontier", Vec2::new(4600.0, 0.0)),
        ];
        let hierarchy = MapHierarchy {
            room_groups: rooms
                .iter()
                .map(|room| (room.id.clone(), vec!["Railway".into()]))
                .collect(),
            ..Default::default()
        };
        let atlas = MapAtlas::new(&rooms, Some(&hierarchy));
        let visits = BTreeSet::new();
        let access = MapAccess {
            current_room: Some("home"),
            visited: &visits,
            reveal_all: true,
            can_travel: false,
        };
        for aspect in [1.2, 2.0] {
            let mut browser = MapBrowser::default();
            let view = |browser: &MapBrowser| browser.view(&atlas, access, aspect);
            let local = view(&browser);
            assert_eq!(local.title, "Railway");
            assert_eq!(local.parent_title.as_deref(), Some("Frontier"));
            assert!(!local.room_detail);
            let home = &local.entries[0];
            assert!(home.here && home.selected);
            assert!((home.pieces[0].center() - Vec2::splat(0.5)).length() < 0.001);
            assert!(!local.entries[1].pieces.is_empty());
            assert!(local.entries[2].pieces.is_empty());

            browser.apply(&atlas, access, aspect, MapAction::Zoom(-1));
            let zoomed_out = view(&browser);
            assert!(zoomed_out.entries[0].pieces[0].width() < home.pieces[0].width());
            assert!(zoomed_out.entries[1].pieces[0].max.x < local.entries[1].pieces[0].max.x);

            let mut browser = MapBrowser::default();
            browser.apply(&atlas, access, aspect, MapAction::Fit);
            let fitted = view(&browser);
            assert!(fitted.entries.iter().all(|entry| !entry.pieces.is_empty()));
            assert!(fitted.entries[0].selected);
        }
    }
}

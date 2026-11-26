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
    pub pins: Vec<MapPin>,
}

struct Entry<'a> {
    parent: MapEntryId,
    children: Vec<MapEntryId>,
    title: String,
    pieces: Vec<&'a MapRoomNode>,
    key: Vec<&'a str>,
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
}

const ROOT: MapEntryId = MapEntryId(0);

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
                key: Vec::new(),
            }],
            room_ids: BTreeMap::new(),
        };
        let mut groups = BTreeMap::new();
        for room in rooms {
            let room_id = room_id(room);
            if let Some(id) = atlas.room_ids.get(room_id) {
                atlas.entries[id.0].pieces.push(room);
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
            if let Some(path) = hierarchy.and_then(|h| h.room_groups.get(room_id)) {
                for group in path {
                    key.push(group);
                    parent = *groups
                        .entry(key.clone())
                        .or_insert_with(|| atlas.insert(parent, group.clone(), None, key.clone()));
                }
            }
            key.push(room_id);
            let id = atlas.insert(parent, room.title.clone(), Some(room), key);
            atlas.room_ids.insert(room_id, id);
        }
        atlas
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
}

fn world_title(id: &str) -> String {
    let id = id.strip_prefix("world.").unwrap_or(id);
    if id.is_empty() {
        return "World".into();
    }
    title_from_id(id.strip_suffix("_ldtk").unwrap_or(id))
}

/// Camera, location and selection for one menu. Keep this component with the menu owner.
#[derive(Component, Default)]
pub struct MapBrowser {
    location: Location,
    selected: Option<Location>,
    tool: Option<MapAction>,
    centre: Option<Vec2>,
    zoom: Option<f32>,
}

#[derive(Default)]
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
    entries: Vec<(MapEntryId, Rect, bool)>,
    current: Option<MapEntryId>,
    atlas: &'a MapAtlas<'a>,
}

impl MapBrowser {
    /// Get the focused tool. A menu can use this to leave the map's controls.
    pub fn focused_tool(&self) -> Option<MapAction> {
        self.tool
    }

    fn level<'a>(&self, atlas: &'a MapAtlas<'a>, access: MapAccess<'_>) -> Level<'a> {
        let id = atlas
            .entries
            .iter()
            .position(|entry| self.location.matches(entry))
            .map_or(ROOT, MapEntryId);
        let level = &atlas.entries[id.0];
        let current = access
            .current_room
            .and_then(|room| atlas.room_ids.get(room).copied());
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
                                || super::view::open_edge(room, other))
                    })
        };
        let mut entries = Vec::new();
        let mut bounds: Option<Rect> = None;
        if let Some(rect) = level.room_bounds() {
            entries.push((id, rect, level.pieces.iter().any(|piece| visited(piece))));
            bounds = Some(rect);
        }
        for &child in &level.children {
            let item = &atlas.entries[child.0];
            if item.pieces.iter().any(|piece| visible(piece)) {
                let rect = item.room_bounds().expect("a visible room has a piece");
                entries.push((child, rect, item.pieces.iter().any(|piece| visited(piece))));
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
            .filter(|child| atlas.entries[child.0].pieces.is_empty())
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
            entries.push((child, rect, true));
        }
        let bounds = entries
            .iter()
            .map(|(_, rect, _)| *rect)
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
                    .find(|(entry, _, _)| location.matches(&level.atlas.entries[entry.0]))
                    .map(|(id, _, _)| *id)
            })
            .or_else(|| {
                level
                    .entries
                    .iter()
                    .find(|(id, _, _)| level.atlas.is_here(*id, level.current))
                    .map(|(id, _, _)| *id)
            })
            .or_else(|| level.entries.first().map(|(id, _, _)| *id))
    }

    fn frame(&self, level: &Level, aspect: f32) -> Rect {
        let aspect = if aspect.is_finite() {
            aspect.clamp(0.1, 10.0)
        } else {
            1.0
        };
        let bounds = level.bounds;
        let size = bounds.size().max(Vec2::ONE);
        let height = size.y.max(size.x / aspect) * 1.10;
        let extent = Vec2::new(height * aspect, height) / self.zoom.unwrap_or(1.0);
        let overscroll = extent * 0.25;
        let centre = self
            .centre
            .unwrap_or_else(|| bounds.center())
            .clamp(bounds.min - overscroll, bounds.max + overscroll);
        Rect::from_center_size(centre, extent)
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
        let frame = self.frame(&level, aspect);
        let point = |p| (p - frame.min) / frame.size();
        let selected = self.selected(&level);
        let tools = self.tools(&level, access.can_travel);
        let mut doors = Vec::new();
        let mut pins = Vec::new();
        let entries = level
            .entries
            .iter()
            .map(|&(id, rect, visited)| {
                let item = &atlas.entries[id.0];
                let project = |rect: Rect| {
                    let clip = rect.intersect(frame);
                    (!clip.is_empty()).then(|| Rect::from_corners(point(clip.min), point(clip.max)))
                };
                let mut pieces = Vec::new();
                if item.pieces.is_empty() {
                    pieces.extend(project(rect));
                }
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

    fn zoom_level(&mut self, level: &Level, aspect: f32, steps: f32) {
        let frame = self.frame(level, aspect);
        let zoom = self.zoom.unwrap_or(1.0);
        let focus = (self.centre.is_none() && steps > 0.0)
            .then(|| self.selected(level))
            .flatten();
        self.centre = Some(
            level
                .entries
                .iter()
                .find(|(id, _, _)| Some(*id) == focus)
                .map_or(frame.center(), |(_, rect, _)| rect.center()),
        );
        self.zoom = Some((zoom * MAP_ZOOM_STEP.powf(steps)).clamp(MAP_ZOOM_MIN, 16.0));
    }

    fn pan_level(&mut self, level: &Level, aspect: f32, delta: Vec2) {
        let frame = self.frame(level, aspect);
        self.centre = Some(frame.center() + frame.size() * delta);
        self.centre = Some(self.frame(level, aspect).center());
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
                if detail.is_none() && level.entries.iter().any(|(entry, _, _)| *entry == id) =>
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
                    .position(|(id, _, _)| Some(*id) == selected)
                    .unwrap_or(0);
                let index = (index as i32 + i32::from(step)).rem_euclid(level.entries.len() as i32);
                self.reveal(&level, level.entries[index as usize].0, aspect);
            }
            MapAction::Zoom(step) => self.zoom_level(&level, aspect, f32::from(step)),
            MapAction::Pan(x, y) => {
                self.pan_level(&level, aspect, Vec2::new(f32::from(x), f32::from(y)) * 0.18)
            }
            MapAction::Fit => {
                self.centre = None;
                self.zoom = None;
            }
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
            location: Location::of(entry),
            ..Self::default()
        };
    }

    fn reveal(&mut self, level: &Level, id: MapEntryId, aspect: f32) {
        self.selected = Some(Location::of(&level.atlas.entries[id.0]));
        let Some((_, rect, _)) = level.entries.iter().find(|(entry, _, _)| *entry == id) else {
            return;
        };
        let frame = self.frame(level, aspect);
        if !frame.contains(rect.center()) {
            self.centre = Some(rect.center());
        }
    }
}

impl Level<'_> {
    fn directional(&self, selected: Option<MapEntryId>, direction: Vec2) -> Option<MapEntryId> {
        let (_, origin, _) = self
            .entries
            .iter()
            .find(|(entry, _, _)| Some(*entry) == selected)?;
        let direction = direction.normalize_or_zero();
        self.entries
            .iter()
            .filter_map(|(id, rect, _)| {
                let offset = rect.center() - origin.center();
                let ahead = offset.dot(direction);
                (ahead > f32::EPSILON).then(|| {
                    let sideways = offset.perp_dot(direction).abs();
                    (*id, ahead + 2.0 * sideways)
                })
            })
            .min_by(|(_, left), (_, right)| left.total_cmp(right))
            .map(|(id, _)| id)
    }
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
            current_room: Some("hub"),
            visited: &visits,
            reveal_all: false,
            can_travel: true,
        };
        browser.apply(&atlas, access, 1.7, MapAction::Confirm);
        let world = browser.view(&atlas, access, 1.7);
        assert_eq!(world.entries.len(), 1);
        assert!(world.entries[0].here);
        assert_eq!(world.entries[0].pieces.len(), 2);
        browser.apply(&atlas, access, 1.7, MapAction::Open(world.entries[0].id));
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
        for title in ["Dimensions", "Sandbox", "home"] {
            let fitted = view(&browser);
            assert_eq!(fitted.title, title);
            browser.apply(&atlas, access, 1.7, MapAction::Zoom(4));
            assert_ne!(fitted.entries, view(&browser).entries);
            let zoomed = view(&browser);
            browser.adjust_viewport(&atlas, access, 1.7, Vec2::new(0.35, 0.2), 0.0);
            assert_ne!(zoomed.entries, view(&browser).entries);
            browser.apply(&atlas, access, 1.7, MapAction::Fit);
            assert_eq!(fitted, view(&browser));
            browser.apply(&atlas, access, 1.7, MapAction::Confirm);
        }
    }
}

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

#[cfg(feature = "input")]
use crate::input::MenuControlFrame;

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
    save.data_mut()
        .set_flag(format!("room_visited_{current}"), true);
}

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

#[cfg(feature = "input")]
pub fn map_menu_pointer_dismiss(
    mut map: ResMut<MapMenuState>,
    interactions: Query<&Interaction, (With<MapMenuRoot>, Changed<Interaction>)>,
) {
    if !map.open {
        return;
    }
    for interaction in &interactions {
        if matches!(interaction, Interaction::Pressed) {
            map.open = false;
        }
    }
}

#[cfg(not(feature = "input"))]
pub fn map_menu_pointer_dismiss() {}

#[cfg(feature = "input")]
pub fn handle_map_menu_hotkeys(
    keys: Res<bevy::input::ButtonInput<bevy::input::keyboard::KeyCode>>,
    menu: Res<MenuControlFrame>,
    mut map: ResMut<MapMenuState>,
) {
    use bevy::input::keyboard::KeyCode;
    if keys.just_pressed(KeyCode::KeyM) || menu.map {
        map.toggle_open();
    }
    if keys.just_pressed(KeyCode::KeyN) {
        map.toggle_minimap();
    }
    if map.open {
        if menu.back || menu.start {
            map.open = false;
            return;
        }
        let zoom_in = keys.just_pressed(KeyCode::Equal)
            || keys.just_pressed(KeyCode::NumpadAdd)
            || menu.right
            || menu.scroll_y > 0.5;
        let zoom_out = keys.just_pressed(KeyCode::Minus)
            || keys.just_pressed(KeyCode::NumpadSubtract)
            || menu.left
            || menu.scroll_y < -0.5;
        let zoom_reset = keys.just_pressed(KeyCode::Digit0) || keys.just_pressed(KeyCode::Numpad0);
        if zoom_in {
            map.zoom_in();
        }
        if zoom_out {
            map.zoom_out();
        }
        if zoom_reset {
            map.zoom_reset();
        }
    }
}

#[cfg(not(feature = "input"))]
pub fn handle_map_menu_hotkeys() {}

const MAP_PANEL_WIDTH: f32 = 720.0;
const MAP_PANEL_HEIGHT: f32 = 480.0;
const MAP_PADDING: f32 = 24.0;
const MINIMAP_WIDTH: f32 = 200.0;
const MINIMAP_HEIGHT: f32 = 140.0;
const MINIMAP_PADDING: f32 = 6.0;

#[derive(Component)]
pub struct MapMenuRoot;

#[derive(Component)]
pub struct MapMenuCanvas;

#[derive(Component)]
pub struct MapMenuStatus;

#[derive(Component)]
pub struct MinimapRoot;

#[derive(Component)]
pub struct MinimapCanvas;

#[derive(Component)]
pub struct MapRoomBox {
    pub room_id: String,
}

pub fn spawn_map_menu(mut commands: Commands) {
    let root = commands
        .spawn((
            Button,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(50.0),
                top: Val::Percent(50.0),
                margin: UiRect {
                    left: Val::Px(-MAP_PANEL_WIDTH * 0.5),
                    top: Val::Px(-MAP_PANEL_HEIGHT * 0.5),
                    ..default()
                },
                width: Val::Px(MAP_PANEL_WIDTH),
                height: Val::Px(MAP_PANEL_HEIGHT),
                padding: UiRect::all(Val::Px(MAP_PADDING)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.04, 0.06, 0.10, 0.96)),
            BorderColor::all(Color::srgba(0.42, 0.78, 1.00, 0.85)),
            ZIndex(60),
            Visibility::Hidden,
            MapMenuRoot,
            Name::new("Map menu root"),
        ))
        .id();
    let title = commands
        .spawn((
            Text::new("MAP"),
            TextFont {
                font_size: 22.0,
                ..default()
            },
            TextColor(Color::srgba(0.92, 0.96, 1.0, 0.98)),
            Name::new("Map title"),
        ))
        .id();
    let status = commands
        .spawn((
            Text::new("0 rooms visited"),
            TextFont {
                font_size: 14.0,
                ..default()
            },
            TextColor(Color::srgba(0.78, 0.86, 0.96, 0.9)),
            MapMenuStatus,
            Name::new("Map status"),
        ))
        .id();
    let canvas = commands
        .spawn((
            Node {
                position_type: PositionType::Relative,
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.03, 0.06, 0.65)),
            MapMenuCanvas,
            Name::new("Map canvas"),
        ))
        .id();
    commands.entity(root).add_children(&[title, status, canvas]);

    let minimap_root = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(12.0),
                top: Val::Px(12.0),
                width: Val::Px(MINIMAP_WIDTH),
                height: Val::Px(MINIMAP_HEIGHT),
                padding: UiRect::all(Val::Px(MINIMAP_PADDING)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.04, 0.06, 0.10, 0.78)),
            BorderColor::all(Color::srgba(0.42, 0.78, 1.00, 0.65)),
            ZIndex(40),
            Visibility::Hidden,
            MinimapRoot,
            Name::new("Minimap root"),
        ))
        .id();
    let minimap_canvas = commands
        .spawn((
            Node {
                position_type: PositionType::Relative,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            MinimapCanvas,
            Name::new("Minimap canvas"),
        ))
        .id();
    commands
        .entity(minimap_root)
        .add_children(&[minimap_canvas]);
}

pub fn sync_map_menu(
    mut commands: Commands,
    map: Res<MapMenuState>,
    room_set: Res<crate::rooms::RoomSet>,
    mut roots: Query<&mut Visibility, (With<MapMenuRoot>, Without<MinimapRoot>)>,
    mut minimap_roots: Query<&mut Visibility, (With<MinimapRoot>, Without<MapMenuRoot>)>,
    canvases: Query<Entity, With<MapMenuCanvas>>,
    minimap_canvases: Query<Entity, With<MinimapCanvas>>,
    mut status: Query<&mut Text, With<MapMenuStatus>>,
    existing_boxes: Query<Entity, With<MapRoomBox>>,
) {
    if let Ok(mut visibility) = roots.single_mut() {
        *visibility = if map.open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if let Ok(mut visibility) = minimap_roots.single_mut() {
        *visibility = if map.minimap_enabled {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if let Ok(mut text) = status.single_mut() {
        **text = format!(
            "{} of {} rooms visited — {} active   |   zoom {:.2}x   (+ / − adjust, 0 reset)",
            map.visited.len(),
            map.rooms.len(),
            room_set.active_spec().id,
            map.zoom,
        );
    }

    if !map.open && !map.minimap_enabled {
        return;
    }
    if map.rooms.is_empty() {
        return;
    }

    for entity in &existing_boxes {
        commands.entity(entity).despawn();
    }

    let active_id = room_set.active_spec().id.clone();

    if map.open {
        if let Ok(canvas) = canvases.single() {
            paint_room_boxes(
                &mut commands,
                canvas,
                &map.rooms,
                &map.visited,
                &active_id,
                MAP_PANEL_WIDTH - MAP_PADDING * 2.0,
                MAP_PANEL_HEIGHT - MAP_PADDING * 2.0 - 60.0,
                MapLabelStyle::Full,
                map.zoom,
            );
        }
    }
    if map.minimap_enabled {
        if let Ok(canvas) = minimap_canvases.single() {
            paint_room_boxes(
                &mut commands,
                canvas,
                &map.rooms,
                &map.visited,
                &active_id,
                MINIMAP_WIDTH - MINIMAP_PADDING * 2.0,
                MINIMAP_HEIGHT - MINIMAP_PADDING * 2.0,
                MapLabelStyle::None,
                1.0,
            );
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MapLabelStyle {
    Full,
    #[allow(dead_code)]
    Short,
    None,
}

fn paint_room_boxes(
    commands: &mut Commands,
    canvas: Entity,
    rooms: &[MapRoomNode],
    visited: &BTreeSet<String>,
    active: &str,
    canvas_w: f32,
    canvas_h: f32,
    label_style: MapLabelStyle,
    zoom: f32,
) {
    if rooms.is_empty() {
        return;
    }
    let min_x = rooms
        .iter()
        .map(|r| r.world_min.x)
        .fold(f32::INFINITY, f32::min);
    let max_x = rooms
        .iter()
        .map(|r| r.world_min.x + r.world_size.x)
        .fold(f32::NEG_INFINITY, f32::max);
    let min_y = rooms
        .iter()
        .map(|r| r.world_min.y)
        .fold(f32::INFINITY, f32::min);
    let max_y = rooms
        .iter()
        .map(|r| r.world_min.y + r.world_size.y)
        .fold(f32::NEG_INFINITY, f32::max);
    let span_x = (max_x - min_x).max(1.0);
    let span_y = (max_y - min_y).max(1.0);
    let fit_scale = (canvas_w / span_x).min(canvas_h / span_y);
    let scale = fit_scale * zoom.clamp(MAP_ZOOM_MIN, MAP_ZOOM_MAX);
    let active_room = rooms.iter().find(|r| r.id == active);
    let (offset_x, offset_y) = if zoom > 1.0001 {
        if let Some(active_room) = active_room {
            let active_cx = active_room.world_min.x + active_room.world_size.x * 0.5;
            let active_cy = active_room.world_min.y + active_room.world_size.y * 0.5;
            let world_cx = (min_x + max_x) * 0.5;
            let world_cy = (min_y + max_y) * 0.5;
            (
                (world_cx - active_cx) * scale,
                (world_cy - active_cy) * scale,
            )
        } else {
            (0.0, 0.0)
        }
    } else {
        (0.0, 0.0)
    };

    for room in rooms {
        let visited_now = visited.contains(&room.id);
        let is_active = room.id == active;
        let color = if is_active {
            Color::srgba(0.55, 0.92, 0.62, 0.95)
        } else if visited_now {
            Color::srgba(0.42, 0.78, 1.00, 0.78)
        } else {
            Color::srgba(0.30, 0.32, 0.42, 0.45)
        };
        let border = if is_active {
            Color::srgba(1.0, 1.0, 1.0, 1.0)
        } else if visited_now {
            Color::srgba(0.78, 0.92, 1.00, 0.85)
        } else {
            Color::srgba(0.55, 0.58, 0.66, 0.55)
        };
        let left = (room.world_min.x - min_x) * scale + offset_x;
        let top = (room.world_min.y - min_y) * scale + offset_y;
        let width = room.world_size.x * scale;
        let height = room.world_size.y * scale;
        let mut entity = commands.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(left),
                top: Val::Px(top),
                width: Val::Px(width.max(8.0)),
                height: Val::Px(height.max(8.0)),
                padding: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(color),
            BorderColor::all(border),
            MapRoomBox {
                room_id: room.id.clone(),
            },
            Name::new(format!("MapRoom {}", room.id)),
        ));
        let entity_id = entity.id();
        match label_style {
            MapLabelStyle::None => {}
            MapLabelStyle::Short => {
                entity.with_children(|parent| {
                    parent.spawn((
                        Text::new(short_room_label(&room.id)),
                        TextFont {
                            font_size: 10.0,
                            ..default()
                        },
                        TextColor(Color::srgba(0.04, 0.06, 0.10, 0.95)),
                    ));
                });
            }
            MapLabelStyle::Full => {
                let label = if width >= 80.0 {
                    room.id.clone()
                } else {
                    short_room_label(&room.id)
                };
                let font_size = if width >= 120.0 { 12.0 } else { 9.0 };
                entity.with_children(|parent| {
                    parent.spawn((
                        Text::new(label),
                        TextFont {
                            font_size,
                            ..default()
                        },
                        TextColor(Color::srgba(0.04, 0.06, 0.10, 0.95)),
                    ));
                });
            }
        }
        commands.entity(canvas).add_child(entity_id);
    }
}

fn short_room_label(id: &str) -> String {
    let parts: Vec<&str> = id.split('_').collect();
    if parts.len() <= 1 {
        id.chars().take(8).collect::<String>().to_uppercase()
    } else {
        parts
            .iter()
            .filter_map(|p| p.chars().next())
            .map(|c| c.to_ascii_uppercase())
            .collect::<String>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_zoom_in_clamps_to_max() {
        let mut map = MapMenuState::default();
        for _ in 0..20 {
            map.zoom_in();
        }
        assert!(map.zoom <= MAP_ZOOM_MAX + 1e-4);
        assert!(map.zoom > 1.0);
    }

    #[test]
    fn map_zoom_out_clamps_to_min() {
        let mut map = MapMenuState::default();
        for _ in 0..20 {
            map.zoom_out();
        }
        assert!(map.zoom >= MAP_ZOOM_MIN - 1e-4);
        assert!(map.zoom < 1.0);
    }

    #[test]
    fn map_zoom_reset_returns_to_one() {
        let mut map = MapMenuState::default();
        map.zoom_in();
        map.zoom_in();
        map.zoom_reset();
        assert_eq!(map.zoom, 1.0);
    }

    #[test]
    fn map_zoom_step_is_round_trip_friendly() {
        let mut map = MapMenuState::default();
        let initial = map.zoom;
        map.zoom_in();
        let zoomed = map.zoom;
        map.zoom_out();
        assert!(
            (map.zoom - initial).abs() < 1e-3,
            "zoom_in then zoom_out should return near 1.0 (got {} from {})",
            map.zoom,
            zoomed
        );
    }

    #[test]
    fn short_room_label_initializes_underscore_id() {
        assert_eq!(short_room_label("central_hub_complex"), "CHC");
        assert_eq!(short_room_label("water_world"), "WW");
        assert_eq!(short_room_label("mob_lab"), "ML");
    }

    #[test]
    fn short_room_label_uppercase_truncates_single_word() {
        assert_eq!(short_room_label("alpha"), "ALPHA");
        assert_eq!(short_room_label("verylongname"), "VERYLONG");
    }
}

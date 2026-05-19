use std::collections::BTreeSet;

use bevy::prelude::*;

use super::model::{MapMenuState, MapRoomNode, MAP_ZOOM_MAX, MAP_ZOOM_MIN};

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
    #[allow(dead_code)] // Carried for future "click room → highlight" lookup.
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
        // Map and minimap both off — drop any leftover boxes once, then
        // skip rebuild until the player toggles either back on.
        for entity in &existing_boxes {
            commands.entity(entity).despawn();
        }
        return;
    }
    if map.rooms.is_empty() {
        return;
    }

    // Skip the despawn-and-repaint pass when nothing material changed
    // this frame. `MapMenuState` mutates on toggle / visit / zoom and
    // `RoomSet` mutates on room transitions, so a frame with neither
    // changed produces an identical paint — repainting it is wasted
    // work. The visibility / status text branches above stay
    // unconditional (cheap, and the status string depends on inputs
    // that aren't all covered by `is_changed`).
    if !map.is_changed() && !room_set.is_changed() {
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

pub(super) fn short_room_label(id: &str) -> String {
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

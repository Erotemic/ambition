//! Bevy-UI for the map: spawns and syncs the full-screen map panel
//! (`spawn_map_menu` / `sync_map_menu`, rooted at `MapMenuRoot`) and the corner
//! minimap, drawing visited rooms from `MapMenuState` room geometry. Owns the
//! panel/minimap layout constants and the `short_room_label` helper.

use std::collections::{BTreeSet, HashMap, HashSet};

use bevy::prelude::*;

use ambition_menu::map::{MapMenuState, MapRoomNode, MAP_ZOOM_MAX, MAP_ZOOM_MIN};
use ambition_platformer_primitives::lifecycle::SessionSpawnScope;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MapRoomBoxKind {
    Map,
    Minimap,
}

/// Persistent UI entity for a single room on the map or minimap.
///
/// One [`MapRoomBox`] per `(MapRoomBoxKind, room_id)` pair lives as long
/// as the room is in [`MapMenuState::rooms`] and its canvas is enabled.
/// `sync_map_menu` mutates this entity's `Node` and color components in
/// place when zoom / visit / active state changes, rather than the
/// pre-refactor pattern of despawning + respawning the whole subtree
/// every frame the state mutated.
#[derive(Component)]
pub struct MapRoomBox {
    pub room_id: String,
    pub kind: MapRoomBoxKind,
    /// Most recently rendered label state for the child
    /// [`MapRoomLabel`] entity, so we can skip respawning the text
    /// when zoom / size thresholds didn't change it. `None` for
    /// canvases that render no label (minimap).
    current_label: Option<String>,
    current_font_size: f32,
}

#[derive(Component)]
pub struct MapRoomLabel;

pub fn spawn_map_menu(mut commands: Commands) {
    spawn_map_menu_with_scope(&mut commands, SessionSpawnScope::UNSCOPED);
}

/// Spawn the map and minimap under an explicit gameplay-session owner.
///
/// The process-resident direct-entry host uses [`spawn_map_menu`]. Shell hosts
/// call this function when an Ambition gameplay session activates so the roots
/// are absent at the title and are retired by the exact session cleanup.
pub fn spawn_map_menu_with_scope(commands: &mut Commands, scope: SessionSpawnScope) {
    let mut root_commands = commands.spawn((
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
        ));
    scope.apply_to(&mut root_commands);
    let root = root_commands.id();
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

    let mut minimap_root_commands = commands.spawn((
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
        ));
    scope.apply_to(&mut minimap_root_commands);
    let minimap_root = minimap_root_commands.id();
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

/// Visual produced for a single room on a single canvas this frame.
/// `paint_room_boxes` builds these and `sync_map_menu` reconciles them
/// against the live [`MapRoomBox`] entities.
struct RoomVisual {
    left: f32,
    top: f32,
    width: f32,
    height: f32,
    color: Color,
    border: Color,
    /// `None` when the canvas style asks for no labels.
    label: Option<RoomLabel>,
}

struct RoomLabel {
    text: String,
    font_size: f32,
}

#[allow(clippy::too_many_arguments)]
pub fn sync_map_menu(
    mut commands: Commands,
    map: Res<MapMenuState>,
    room_set: Res<crate::rooms::RoomSet>,
    mut roots: Query<&mut Visibility, (With<MapMenuRoot>, Without<MinimapRoot>)>,
    mut minimap_roots: Query<&mut Visibility, (With<MinimapRoot>, Without<MapMenuRoot>)>,
    canvases: Query<Entity, With<MapMenuCanvas>>,
    minimap_canvases: Query<Entity, With<MinimapCanvas>>,
    mut status: Query<&mut Text, With<MapMenuStatus>>,
    mut boxes: Query<(
        Entity,
        &mut MapRoomBox,
        &mut Node,
        &mut BackgroundColor,
        &mut BorderColor,
        Option<&Children>,
    )>,
    mut labels: Query<(&mut Text, &mut TextFont), (With<MapRoomLabel>, Without<MapMenuStatus>)>,
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
        // Both canvases off — drop every persistent box once, then skip
        // rebuild until the player toggles either back on.
        for (entity, _, _, _, _, _) in &boxes {
            commands.entity(entity).despawn();
        }
        return;
    }
    if map.rooms.is_empty() {
        return;
    }

    // Skip the reconciliation pass when nothing material changed this
    // frame. Visibility / status text above stay unconditional because
    // they're cheap and depend on inputs not all covered by `is_changed`.
    if !map.is_changed() && !room_set.is_changed() {
        return;
    }

    let active_id = room_set.active_spec().id.clone();

    // Compute desired (kind, room_id) → RoomVisual for every enabled canvas.
    let mut desired: HashMap<(MapRoomBoxKind, String), RoomVisual> = HashMap::new();
    if map.open {
        compute_canvas_visuals(
            &mut desired,
            MapRoomBoxKind::Map,
            &map.rooms,
            &map.visited,
            &active_id,
            MAP_PANEL_WIDTH - MAP_PADDING * 2.0,
            MAP_PANEL_HEIGHT - MAP_PADDING * 2.0 - 60.0,
            MapLabelStyle::Full,
            map.zoom,
        );
    }
    if map.minimap_enabled {
        compute_canvas_visuals(
            &mut desired,
            MapRoomBoxKind::Minimap,
            &map.rooms,
            &map.visited,
            &active_id,
            MINIMAP_WIDTH - MINIMAP_PADDING * 2.0,
            MINIMAP_HEIGHT - MINIMAP_PADDING * 2.0,
            MapLabelStyle::None,
            1.0,
        );
    }

    // Pass 1: walk existing boxes, mutate matches in place, despawn stragglers.
    let mut seen: HashSet<(MapRoomBoxKind, String)> = HashSet::new();
    for (entity, mut room_box, mut node, mut bg, mut border, children) in &mut boxes {
        let key = (room_box.kind, room_box.room_id.clone());
        if let Some(visual) = desired.get(&key) {
            apply_visual_to_node(&mut node, visual);
            *bg = BackgroundColor(visual.color);
            *border = BorderColor::all(visual.border);
            reconcile_label(
                &mut commands,
                entity,
                children,
                &mut room_box,
                visual,
                &mut labels,
            );
            seen.insert(key);
        } else {
            commands.entity(entity).despawn();
        }
    }

    // Pass 2: spawn boxes for desired entries we did not find above.
    let Ok(map_canvas) = canvases.single() else {
        return;
    };
    let minimap_canvas = minimap_canvases.single().ok();
    for (key, visual) in desired {
        if seen.contains(&key) {
            continue;
        }
        let canvas = match key.0 {
            MapRoomBoxKind::Map => map_canvas,
            MapRoomBoxKind::Minimap => {
                let Some(canvas) = minimap_canvas else {
                    continue;
                };
                canvas
            }
        };
        let (kind, room_id) = key;
        spawn_room_box(&mut commands, canvas, kind, room_id, visual);
    }
}

fn apply_visual_to_node(node: &mut Node, visual: &RoomVisual) {
    node.position_type = PositionType::Absolute;
    node.left = Val::Px(visual.left);
    node.top = Val::Px(visual.top);
    node.width = Val::Px(visual.width.max(8.0));
    node.height = Val::Px(visual.height.max(8.0));
    node.padding = UiRect::all(Val::Px(2.0));
}

fn reconcile_label(
    commands: &mut Commands,
    box_entity: Entity,
    children: Option<&Children>,
    room_box: &mut MapRoomBox,
    visual: &RoomVisual,
    labels: &mut Query<(&mut Text, &mut TextFont), (With<MapRoomLabel>, Without<MapMenuStatus>)>,
) {
    match &visual.label {
        Some(label) => {
            if room_box.current_label.as_deref() == Some(label.text.as_str())
                && (room_box.current_font_size - label.font_size).abs() < 0.01
            {
                return;
            }
            // Find the existing label child, if any, and mutate it.
            if let Some(children) = children {
                for child in children.iter() {
                    if let Ok((mut text, mut font)) = labels.get_mut(child) {
                        **text = label.text.clone();
                        font.font_size = label.font_size;
                        room_box.current_label = Some(label.text.clone());
                        room_box.current_font_size = label.font_size;
                        return;
                    }
                }
            }
            // No existing label child — spawn one.
            let label_entity = commands
                .spawn((
                    Text::new(label.text.clone()),
                    TextFont {
                        font_size: label.font_size,
                        ..default()
                    },
                    TextColor(Color::srgba(0.04, 0.06, 0.10, 0.95)),
                    MapRoomLabel,
                ))
                .id();
            commands.entity(box_entity).add_child(label_entity);
            room_box.current_label = Some(label.text.clone());
            room_box.current_font_size = label.font_size;
        }
        None => {
            // No label desired (minimap). Despawn any existing label child.
            if let Some(children) = children {
                for child in children.iter() {
                    if labels.get(child).is_ok() {
                        commands.entity(child).despawn();
                    }
                }
            }
            room_box.current_label = None;
            room_box.current_font_size = 0.0;
        }
    }
}

fn spawn_room_box(
    commands: &mut Commands,
    canvas: Entity,
    kind: MapRoomBoxKind,
    room_id: String,
    visual: RoomVisual,
) {
    let mut node = Node::default();
    apply_visual_to_node(&mut node, &visual);
    let mut entity = commands.spawn((
        node,
        BackgroundColor(visual.color),
        BorderColor::all(visual.border),
        MapRoomBox {
            room_id: room_id.clone(),
            kind,
            current_label: visual.label.as_ref().map(|l| l.text.clone()),
            current_font_size: visual.label.as_ref().map(|l| l.font_size).unwrap_or(0.0),
        },
        Name::new(format!("MapRoom {}", room_id)),
    ));
    if let Some(label) = &visual.label {
        let text = label.text.clone();
        let font_size = label.font_size;
        entity.with_children(|parent| {
            parent.spawn((
                Text::new(text),
                TextFont {
                    font_size,
                    ..default()
                },
                TextColor(Color::srgba(0.04, 0.06, 0.10, 0.95)),
                MapRoomLabel,
            ));
        });
    }
    let entity_id = entity.id();
    commands.entity(canvas).add_child(entity_id);
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MapLabelStyle {
    Full,
    #[allow(dead_code)]
    Short,
    None,
}

#[allow(clippy::too_many_arguments)]
fn compute_canvas_visuals(
    desired: &mut HashMap<(MapRoomBoxKind, String), RoomVisual>,
    kind: MapRoomBoxKind,
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

        let label = match label_style {
            MapLabelStyle::None => None,
            MapLabelStyle::Short => Some(RoomLabel {
                text: short_room_label(&room.id),
                font_size: 10.0,
            }),
            MapLabelStyle::Full => {
                let text = if width >= 80.0 {
                    room.id.clone()
                } else {
                    short_room_label(&room.id)
                };
                let font_size = if width >= 120.0 { 12.0 } else { 9.0 };
                Some(RoomLabel { text, font_size })
            }
        };

        desired.insert(
            (kind, room.id.clone()),
            RoomVisual {
                left,
                top,
                width,
                height,
                color,
                border,
                label,
            },
        );
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

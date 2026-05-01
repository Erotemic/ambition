//! Data-driven sandbox room-set and loading-zone graph.
//!
//! Rooms are runtime graph nodes built from LDtk-authored runtime data. This
//! module owns transition graph assembly and arrival validation, while LDtk owns
//! sandbox world authoring.
//! Loading-zone links point at destination zones by name, so authoring no longer
//! requires brittle hand-written spawn coordinates.

use std::collections::HashMap;

use ambition_engine as ae;
use ambition_engine::AabbExt;
use bevy::prelude::Resource;
use petgraph::graph::{Graph, NodeIndex};
use petgraph::visit::EdgeRef;
use petgraph::Direction;


/// How a loading zone should be activated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadingZoneActivation {
    EdgeExit,
    Door,
}

impl LoadingZoneActivation {
    pub fn label(self) -> &'static str {
        match self {
            Self::EdgeExit => "edge exit",
            Self::Door => "door",
        }
    }
}

/// A non-colliding rectangular trigger that swaps the active room.
#[derive(Clone, Debug)]
pub struct LoadingZone {
    pub id: String,
    pub name: String,
    pub activation: LoadingZoneActivation,
    pub aabb: ae::Aabb,
}

impl LoadingZone {
    pub fn is_ready(&self, wants_interact: bool) -> bool {
        match self.activation {
            LoadingZoneActivation::EdgeExit => true,
            LoadingZoneActivation::Door => wants_interact,
        }
    }

    pub fn hint(&self, _flying: bool) -> String {
        match self.activation {
            LoadingZoneActivation::EdgeExit => format!("{}: {}", self.activation.label(), self.name),
            LoadingZoneActivation::Door => {
                format!("{}: {} (Interact / double-tap up)", self.activation.label(), self.name)
            }
        }
    }
}

/// Complete room data used by the Bevy sandbox.
#[derive(Clone, Debug)]
pub struct RoomSpec {
    pub id: String,
    pub world: ae::World,
    pub loading_zones: Vec<LoadingZone>,
}

#[derive(Clone, Debug)]
struct TransitionEdge {
    from_zone: String,
    to_zone: String,
}

/// Authored directed connection between loading zones in runtime rooms.
///
/// This is intentionally independent from the retired RON world manifest so
/// LDtk and future generators can build `RoomSet` directly from runtime room
/// data.
#[derive(Clone, Debug)]
pub struct RoomLink {
    pub from_room: String,
    pub from_zone: String,
    pub to_room: String,
    pub to_zone: String,
    pub bidirectional: bool,
}

/// Resolved transition from the active room to a graph-linked destination room.
#[derive(Clone, Debug)]
pub struct RoomTransition {
    pub zone: LoadingZone,
    pub target_room: usize,
    pub arrival: ae::Vec2,
}

/// Small room graph for early loading-zone tests.
#[derive(Resource, Clone, Debug)]
pub struct RoomSet {
    pub rooms: Vec<RoomSpec>,
    pub active: usize,
    graph: Graph<String, TransitionEdge>,
    room_nodes: Vec<NodeIndex>,
}

impl RoomSet {
    /// Build a runtime room graph from already-materialized runtime rooms.
    ///
    /// LDtk uses this path directly so it can own authored world data without
    /// passing through a legacy RON world manifest.
    pub fn from_parts(start_room: impl AsRef<str>, rooms: Vec<RoomSpec>, links: Vec<RoomLink>) -> Self {
        let mut graph = Graph::<String, TransitionEdge>::new();
        let mut room_nodes = Vec::new();
        let mut by_id = HashMap::new();
        for (index, room) in rooms.iter().enumerate() {
            let node = graph.add_node(room.id.clone());
            room_nodes.push(node);
            by_id.insert(room.id.clone(), (index, node));
        }

        for link in &links {
            let Some((_, from_node)) = by_id.get(&link.from_room).copied() else {
                eprintln!("room graph warning: unknown source room '{}'", link.from_room);
                continue;
            };
            let Some((_, to_node)) = by_id.get(&link.to_room).copied() else {
                eprintln!("room graph warning: unknown target room '{}'", link.to_room);
                continue;
            };
            graph.add_edge(
                from_node,
                to_node,
                TransitionEdge {
                    from_zone: link.from_zone.clone(),
                    to_zone: link.to_zone.clone(),
                },
            );
            if link.bidirectional {
                graph.add_edge(
                    to_node,
                    from_node,
                    TransitionEdge {
                        from_zone: link.to_zone.clone(),
                        to_zone: link.from_zone.clone(),
                    },
                );
            }
        }

        let active = by_id.get(start_room.as_ref()).map(|(index, _)| *index).unwrap_or(0);
        Self { rooms, active, graph, room_nodes }
    }

    pub fn active_spec(&self) -> &RoomSpec {
        &self.rooms[self.active]
    }

    pub fn active_world(&self) -> &ae::World {
        &self.active_spec().world
    }

    pub fn active_loading_zones(&self) -> &[LoadingZone] {
        &self.active_spec().loading_zones
    }

    pub fn set_active(&mut self, index: usize) -> &RoomSpec {
        self.active = index.min(self.rooms.len().saturating_sub(1));
        self.active_spec()
    }

    pub fn transition_for_player(&self, player: &ae::Player, wants_interact: bool) -> Option<RoomTransition> {
        let body = player.aabb();
        let zone = self
            .active_loading_zones()
            .iter()
            .find(|zone| body.strict_intersects(zone.aabb) && zone.is_ready(wants_interact))?
            .clone();
        self.transition_from_zone(zone)
    }

    fn transition_from_zone(&self, zone: LoadingZone) -> Option<RoomTransition> {
        let active_node = *self.room_nodes.get(self.active)?;
        for edge in self.graph.edges_directed(active_node, Direction::Outgoing) {
            let weight = edge.weight();
            if weight.from_zone != zone.id {
                continue;
            }
            let target_room = edge.target().index();
            let target_zone = self.zone_by_id(target_room, &weight.to_zone)?;
            let arrival = arrival_from_target_zone(&self.rooms[target_room].world, target_zone);
            return Some(RoomTransition {
                zone,
                target_room,
                arrival,
            });
        }
        None
    }

    fn zone_by_id(&self, room_index: usize, id: &str) -> Option<&LoadingZone> {
        self.rooms
            .get(room_index)?
            .loading_zones
            .iter()
            .find(|zone| zone.id == id)
    }

    pub fn nearby_zone_hints(&self, player: &ae::Player, flying: bool) -> Vec<String> {
        let body = player.aabb();
        self.active_loading_zones()
            .iter()
            .filter(|zone| body.strict_intersects(zone.aabb))
            .map(|zone| zone.hint(flying))
            .collect()
    }

    /// Return non-fatal authoring warnings for room specs.
    pub fn layout_warnings(&self) -> Vec<String> {
        let mut warnings = Vec::new();
        for (room_index, room) in self.rooms.iter().enumerate() {
            for zone in &room.loading_zones {
                for block in &room.world.blocks {
                    let active_fixture = matches!(
                        block.kind,
                        ae::BlockKind::Rebound { .. } | ae::BlockKind::PogoOrb | ae::BlockKind::Hazard
                    );
                    if active_fixture && block.aabb.strict_intersects(zone.aabb) {
                        warnings.push(format!(
                            "room {room_index} '{}' has {} '{}' overlapping loading zone '{}'",
                            room.world.name,
                            block_kind_label(block.kind),
                            block.name,
                            zone.name,
                        ));
                    }
                }
            }
        }

        for edge in self.graph.edge_references() {
            let source_room = edge.source().index();
            let target_room = edge.target().index();
            let weight = edge.weight();
            if self.zone_by_id(source_room, &weight.from_zone).is_none() {
                warnings.push(format!(
                    "room graph edge from room {source_room} references missing source zone '{}'",
                    weight.from_zone,
                ));
                continue;
            }
            let Some(target_zone) = self.zone_by_id(target_room, &weight.to_zone) else {
                warnings.push(format!(
                    "room graph edge into room {target_room} references missing target zone '{}'",
                    weight.to_zone,
                ));
                continue;
            };
            let target_world = &self.rooms[target_room].world;
            let arrival = arrival_from_target_zone(target_world, target_zone);
            let repaired = validated_spawn(target_world, arrival, ae::Vec2::new(PLAYER_HALF_W * 2.0, PLAYER_HALF_H * 2.0));
            let delta = repaired - arrival;
            if delta.length() > 0.5 {
                warnings.push(format!(
                    "room graph edge room {source_room}:{} -> room {target_room}:{} repairs arrival by ({:+.1}, {:+.1})",
                    weight.from_zone,
                    weight.to_zone,
                    delta.x,
                    delta.y,
                ));
            }
        }
        warnings
    }
}

fn block_kind_label(kind: ae::BlockKind) -> &'static str {
    match kind {
        ae::BlockKind::Solid => "solid",
        ae::BlockKind::BlinkWall { .. } => "blink wall",
        ae::BlockKind::OneWay => "one-way platform",
        ae::BlockKind::Hazard => "hazard",
        ae::BlockKind::PogoOrb => "pogo orb",
        ae::BlockKind::Rebound { .. } => "rebound pad",
    }
}

const WALL: f32 = 36.0;
const EDGE_ARRIVAL_INSET: f32 = 92.0;
const PLAYER_HALF_W: f32 = 14.0;
const PLAYER_HALF_H: f32 = 23.0;
const SPAWN_MARGIN: f32 = 3.0;

fn arrival_from_target_zone(world: &ae::World, zone: &LoadingZone) -> ae::Vec2 {
    match zone.activation {
        LoadingZoneActivation::Door => door_arrival(zone.aabb),
        LoadingZoneActivation::EdgeExit => edge_arrival(world, zone.aabb),
    }
}

fn edge_arrival(world: &ae::World, zone: ae::Aabb) -> ae::Vec2 {
    let x = if zone.left() <= WALL + 1.0 {
        EDGE_ARRIVAL_INSET
    } else if zone.right() >= world.size.x - WALL - 1.0 {
        world.size.x - EDGE_ARRIVAL_INSET
    } else {
        zone.center().x
    };
    ae::Vec2::new(x, zone.center().y)
}

fn door_arrival(zone: ae::Aabb) -> ae::Vec2 {
    ae::Vec2::new(zone.center().x, zone.bottom() - PLAYER_HALF_H - SPAWN_MARGIN)
}

/// Clamp and repair a proposed player spawn so transitions never place the
/// player outside the room or embedded in solids.
pub fn validated_spawn(world: &ae::World, desired: ae::Vec2, player_size: ae::Vec2) -> ae::Vec2 {
    let half = player_size * 0.5;
    let base = clamp_spawn_to_room(world, desired, half);
    if player_body_clear(world, base, half) {
        return base;
    }

    const STEP: f32 = 8.0;
    for y_step in 0..=96 {
        let dy = -(y_step as f32) * STEP;
        for x_step in 0..=96 {
            if x_step == 0 {
                let candidate = clamp_spawn_to_room(world, ae::Vec2::new(base.x, base.y + dy), half);
                if player_body_clear(world, candidate, half) {
                    return candidate;
                }
            } else {
                for sign in [-1.0_f32, 1.0] {
                    let dx = sign * x_step as f32 * STEP;
                    let candidate = clamp_spawn_to_room(world, ae::Vec2::new(base.x + dx, base.y + dy), half);
                    if player_body_clear(world, candidate, half) {
                        return candidate;
                    }
                }
            }
        }
    }

    base
}

fn clamp_spawn_to_room(world: &ae::World, pos: ae::Vec2, half: ae::Vec2) -> ae::Vec2 {
    ae::Vec2::new(
        pos.x.clamp(half.x + SPAWN_MARGIN, world.size.x - half.x - SPAWN_MARGIN),
        pos.y.clamp(half.y + SPAWN_MARGIN, world.size.y - half.y - SPAWN_MARGIN),
    )
}

fn player_body_clear(world: &ae::World, center: ae::Vec2, half: ae::Vec2) -> bool {
    let body = ae::Aabb::new(center, half);
    !world.body_overlaps_any(body, |block| {
        matches!(
            block.kind,
            ae::BlockKind::Solid
                | ae::BlockKind::BlinkWall { .. }
                | ae::BlockKind::OneWay
                | ae::BlockKind::Hazard
                | ae::BlockKind::Rebound { .. }
        )
    })
}

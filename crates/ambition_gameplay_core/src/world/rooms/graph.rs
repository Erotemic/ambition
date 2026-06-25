//! `RoomSet` graph assembly + queries (petgraph-backed transition graph).
//!
//! `impl RoomSet` block: builds the node/edge graph from runtime rooms
//! (`from_parts`), exposes active-room accessors (`active_spec`/`active_world`/
//! `active_loading_zones`/…), and resolves player transitions
//! (`transition_for_player`, `nearby_zone_hints`, `layout_warnings`). The
//! `RoomSet` type itself lives in sibling `room_graph`; spawn/arrival math is in
//! sibling `spawn`.

use std::collections::HashMap;

use ambition_engine_core::AabbExt;
use petgraph::graph::Graph;
use petgraph::visit::EdgeRef;
use petgraph::Direction;

use super::spawn::{
    arrival_from_target_zone, block_kind_label, validated_spawn, PLAYER_HALF_H, PLAYER_HALF_W,
};
use super::*;

impl RoomSet {
    /// Build a runtime room graph from already-materialized runtime rooms.
    ///
    /// LDtk uses this path directly so it can own authored world data without
    /// passing through a legacy RON world manifest.
    pub fn from_parts(
        start_room: impl AsRef<str>,
        rooms: Vec<RoomSpec>,
        links: Vec<RoomLink>,
    ) -> Self {
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
                eprintln!(
                    "room graph warning: unknown source room '{}'",
                    link.from_room
                );
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

        let active = by_id
            .get(start_room.as_ref())
            .map(|(index, _)| *index)
            .unwrap_or(0);
        Self {
            rooms,
            active,
            start: active,
            graph,
            room_nodes,
        }
    }

    pub fn room_index_by_id(&self, id: &str) -> Option<usize> {
        self.rooms
            .iter()
            .position(|room| room.id == id || room.world.name == id)
    }

    pub fn set_start_by_id(&mut self, id: &str) -> bool {
        let Some(index) = self.room_index_by_id(id) else {
            return false;
        };
        self.active = index;
        self.start = index;
        true
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

    pub fn active_props(&self) -> &[PropSpec] {
        &self.active_spec().props
    }

    pub fn active_metadata(&self) -> &RoomMetadata {
        &self.active_spec().metadata
    }

    pub fn set_active(&mut self, index: usize) -> &RoomSpec {
        self.active = index.min(self.rooms.len().saturating_sub(1));
        self.active_spec()
    }

    pub fn transition_for_player(
        &self,
        player_aabb: ae::Aabb,
        wants_interact: bool,
    ) -> Option<RoomTransition> {
        let zone = self
            .active_loading_zones()
            .iter()
            .find(|zone| player_aabb.strict_intersects(zone.aabb) && zone.is_ready(wants_interact))?
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

    pub fn nearby_zone_hints(&self, player_aabb: ae::Aabb, flying: bool) -> Vec<String> {
        self.active_loading_zones()
            .iter()
            .filter(|zone| player_aabb.strict_intersects(zone.aabb))
            .map(|zone| zone.hint(flying))
            .collect()
    }

    /// Return non-fatal authoring warnings for room specs.
    ///
    /// Catches authoring jank that compiles but plays badly:
    /// - active fixtures (hazards / pogo orbs / rebound pads)
    ///   overlapping loading zones (player teleports into damage),
    /// - door zones that aren't door-sized (height < player + jump
    ///   buffer),
    /// - door zones too close to a wall to fit the player,
    /// - paired door zones with mismatched sizes,
    /// - rooms whose only entrance is also their only exit (player
    ///   gets stuck once it triggers),
    /// - dangling room graph edges,
    /// - arrival points that need repair to land safely.
    pub fn layout_warnings(&self) -> Vec<String> {
        let mut warnings = Vec::new();
        for (room_index, room) in self.rooms.iter().enumerate() {
            for zone in &room.loading_zones {
                // Active fixtures inside a zone teleport the player
                // straight into damage / a bounce.
                for block in &room.world.blocks {
                    let active_fixture = matches!(
                        block.kind,
                        ae::BlockKind::Rebound { .. }
                            | ae::BlockKind::PogoOrb
                            | ae::BlockKind::Hazard
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
                // Door zones must clear the player's body.
                if matches!(zone.activation, LoadingZoneActivation::Door) {
                    let min_door_h = PLAYER_HALF_H * 2.0 + 6.0;
                    let min_door_w = PLAYER_HALF_W * 2.0 + 6.0;
                    let zone_h = zone.aabb.height();
                    let zone_w = zone.aabb.width();
                    if zone_h < min_door_h {
                        warnings.push(format!(
                            "room {room_index} '{}' door '{}' is too short ({:.0}px; need {:.0}+ for player)",
                            room.world.name, zone.name, zone_h, min_door_h
                        ));
                    }
                    if zone_w < min_door_w {
                        warnings.push(format!(
                            "room {room_index} '{}' door '{}' is too narrow ({:.0}px; need {:.0}+ for player)",
                            room.world.name, zone.name, zone_w, min_door_w
                        ));
                    }
                }
                // Door zones that overlap a Solid block of the same
                // room shouldn't exist — the door-arrival ends inside
                // a wall.
                if matches!(zone.activation, LoadingZoneActivation::Door) {
                    for block in &room.world.blocks {
                        if matches!(block.kind, ae::BlockKind::Solid)
                            && block.aabb.strict_intersects(zone.aabb)
                        {
                            warnings.push(format!(
                                "room {room_index} '{}' door '{}' overlaps solid '{}'",
                                room.world.name, zone.name, block.name,
                            ));
                        }
                    }
                }
            }
        }

        // Per-room: verify there's at least one outgoing edge if the
        // room has any incoming edges (otherwise the room is a trap).
        for (room_index, _room) in self.rooms.iter().enumerate() {
            if room_index >= self.room_nodes.len() {
                continue;
            }
            let node = self.room_nodes[room_index];
            let outgoing = self.graph.edges_directed(node, Direction::Outgoing).count();
            let incoming = self.graph.edges_directed(node, Direction::Incoming).count();
            if incoming > 0 && outgoing == 0 {
                warnings.push(format!(
                    "room {room_index} '{}' has no outgoing edges — it's a one-way trap",
                    self.rooms[room_index].world.name,
                ));
            }
        }

        // Paired-door size consistency: if A→B is via doors, the door
        // sizes should roughly match so the player's mental model
        // ("the door I came through is the door I leave through")
        // holds.
        for edge in self.graph.edge_references() {
            let source_room = edge.source().index();
            let target_room = edge.target().index();
            let weight = edge.weight();
            let Some(from_zone) = self.zone_by_id(source_room, &weight.from_zone) else {
                continue;
            };
            let Some(to_zone) = self.zone_by_id(target_room, &weight.to_zone) else {
                continue;
            };
            if matches!(from_zone.activation, LoadingZoneActivation::Door)
                && matches!(to_zone.activation, LoadingZoneActivation::Door)
            {
                let from_w = from_zone.aabb.width();
                let from_h = from_zone.aabb.height();
                let to_w = to_zone.aabb.width();
                let to_h = to_zone.aabb.height();
                let dw = (from_w - to_w).abs();
                let dh = (from_h - to_h).abs();
                if dw > 12.0 || dh > 12.0 {
                    warnings.push(format!(
                        "room graph edge room {source_room}:{} -> room {target_room}:{} doors mismatch ({}x{} vs {}x{})",
                        weight.from_zone,
                        weight.to_zone,
                        from_w as i32,
                        from_h as i32,
                        to_w as i32,
                        to_h as i32,
                    ));
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
            let repaired = validated_spawn(
                target_world,
                arrival,
                ae::Vec2::new(PLAYER_HALF_W * 2.0, PLAYER_HALF_H * 2.0),
            );
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

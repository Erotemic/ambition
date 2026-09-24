//! `RoomSet` graph assembly + queries (petgraph-backed transition graph).
//!
//! `impl RoomSet` block: builds the node/edge graph from runtime rooms
//! (`from_parts`), exposes active-room accessors (`active_spec`/`active_world`/
//! `active_loading_zones`/…), and resolves player transitions
//! (`transition_for_player`, `nearby_zone_hints`, `layout_warnings`). The
//! `RoomSet` type itself lives in sibling `room_graph`; spawn/arrival math is in
//! sibling `spawn`.

use std::collections::HashMap;

use ambition_platformer2d_core::AabbExt;
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
    /// Fixture use only. Production uses [`Self::try_from_parts`]. This
    /// function is `try_from_parts` plus an `expect`: it panics on a world the
    /// type cannot represent, and a shipped game needs a refusal it can report.
    pub fn from_parts_or_panic(
        start_room: impl AsRef<str>,
        rooms: Vec<RoomSpec>,
        links: Vec<RoomLink>,
    ) -> Self {
        let start = start_room.as_ref().to_string();
        Self::try_from_parts(start_room, rooms, links)
            .unwrap_or_else(|why| panic!("fixture room set for start room `{start}`: {why}"))
    }

    /// Build a runtime room graph from already-materialized runtime rooms, or
    /// refuse.
    ///
    /// There is no fallback. An unresolvable start room refuses, so the caller
    /// never gets a session in a different room. An empty `rooms` refuses,
    /// because `active = start = 0` would index nothing and make
    /// [`RoomSet::active_spec`] and the room-set rollback checksum panic.
    /// Fixtures use [`Self::from_parts_or_panic`].
    pub fn try_from_parts(
        start_room: impl AsRef<str>,
        rooms: Vec<RoomSpec>,
        links: Vec<RoomLink>,
    ) -> Result<Self, RoomSetRefused> {
        // Checked first and separately, because the two diagnoses differ: a set
        // with no rooms is a caller that built nothing, and there is no
        // "registered kinds"-style list to print back at them.
        if rooms.is_empty() {
            return Err(RoomSetRefused::NoRooms {
                start_room: start_room.as_ref().to_string(),
            });
        }
        let mut graph = Graph::<String, TransitionEdge>::new();
        let mut room_nodes = Vec::new();
        let mut by_id = HashMap::new();
        for (index, room) in rooms.iter().enumerate() {
            // Refuse before the insert. `by_id` keeps the last duplicate and
            // `RoomSet::room_index_by_id` returns the first, so a duplicate
            // gives two lookups that name two rooms. See
            // `RoomSetRefused::DuplicateRoomId`.
            if let Some((first, _)) = by_id.get(&room.id) {
                return Err(RoomSetRefused::DuplicateRoomId {
                    id: room.id.clone(),
                    first: *first,
                    second: index,
                });
            }
            let node = graph.add_node(room.id.clone());
            room_nodes.push(node);
            by_id.insert(room.id.clone(), (index, node));
        }

        // Unknown rooms are reported here. Unknown zones are owned by
        // `layout_warnings`, which checks graph/zone consistency.
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

        let Some(&(active, _)) = by_id.get(start_room.as_ref()) else {
            return Err(RoomSetRefused::UnknownStartRoom {
                start_room: start_room.as_ref().to_string(),
                rooms: rooms.into_iter().map(|room| room.id).collect(),
            });
        };
        Ok(Self {
            rooms,
            active,
            start: active,
            graph,
            room_nodes,
        })
    }

    /// Canonical directed room links for fingerprinting and inspection.
    /// Bidirectional authored links are represented by their two effective
    /// directed edges, so behavior rather than source insertion order is hashed.
    pub fn canonical_links(&self) -> Vec<RoomLink> {
        let mut links = self
            .graph
            .edge_references()
            .filter_map(|edge| {
                let from_room = self.graph.node_weight(edge.source())?.clone();
                let to_room = self.graph.node_weight(edge.target())?.clone();
                Some(RoomLink {
                    from_room,
                    from_zone: edge.weight().from_zone.clone(),
                    to_room,
                    to_zone: edge.weight().to_zone.clone(),
                    bidirectional: false,
                })
            })
            .collect::<Vec<_>>();
        links.sort_by(|a, b| {
            (&a.from_room, &a.from_zone, &a.to_room, &a.to_zone).cmp(&(
                &b.from_room,
                &b.from_zone,
                &b.to_room,
                &b.to_zone,
            ))
        });
        links
    }

    /// A room is found by its authored id only.
    ///
    /// `room.world.name` is a display title (the LDtk converter builds
    /// `format!("Ambition: {}", area_id.replace('_', " "))`), not a name.
    /// Do not match it here. `same_destination`, the room transition dedup
    /// key, compares raw `target_room()` values. If a room had two names, two
    /// intents for one room would open two transactions into it.
    pub fn room_index_by_id(&self, id: &str) -> Option<usize> {
        self.rooms.iter().position(|room| room.id == id)
    }

    /// `#[must_use]`: false means the start room was not set. A caller that
    /// ignores it silently keeps the room the world already had.
    #[must_use = "false means the id matched no room and the start was NOT \
                  changed: report it, or the session silently starts elsewhere"]
    pub fn set_start_by_id(&mut self, id: &str) -> bool {
        // Go through `set_active` so that one function owns all writes
        // to `self.active`.
        if self.set_active_by_id(id).is_none() {
            return false;
        }
        self.start = self.active;
        true
    }

    /// Sorted, duplicate-free room indices reachable from the active room.
    ///
    /// This is the presentation-neutral prefetch seam: callers can prepare
    /// likely next-room data without inspecting the private petgraph or
    /// reimplementing loading-zone link resolution. It deliberately reports
    /// graph adjacency only; choosing how much speculative work to perform is
    /// owned by the loading/asset layer.
    pub fn neighboring_room_indices(&self) -> Vec<usize> {
        self.neighboring_room_indices_of(self.active)
    }

    /// The rooms one hop out from `room` — what a transition INTO `room` will
    /// prefetch, and therefore what a room commit keeps resident.
    pub fn neighboring_room_indices_of(&self, room: usize) -> Vec<usize> {
        let Some(&active_node) = self.room_nodes.get(room) else {
            return Vec::new();
        };
        let mut neighbors = self
            .graph
            .edges_directed(active_node, Direction::Outgoing)
            .map(|edge| edge.target().index())
            .filter(|&index| index < self.rooms.len())
            .collect::<Vec<_>>();
        neighbors.sort_unstable();
        neighbors.dedup();
        neighbors
    }

    pub fn active_spec(&self) -> &RoomSpec {
        &self.rooms[self.active]
    }

    /// The spec at `index`, if in range. Resolves a transition's target index to
    /// its authored room id (Track B records the id in a deferred intent).
    pub fn spec_at(&self, index: usize) -> Option<&RoomSpec> {
        self.rooms.get(index)
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

    /// Which room of [`Self::rooms`] is live right now.
    pub fn active(&self) -> usize {
        self.active
    }

    /// Which room of [`Self::rooms`] a fresh sandbox starts in.
    pub fn start(&self) -> usize {
        self.start
    }

    /// Seat the session in room `index`, or refuse.
    ///
    /// Do not clamp. A clamp moves the session to the last room while the
    /// caller publishes the geometry of the room it asked for.
    ///
    /// `None` means the index named no room and nothing was written: the
    /// previously active room is still active. The staging check
    /// `StagedWorldViolation::TargetRoomOutOfRange` also refuses such a plan
    /// earlier.
    #[must_use = "`None` means the index named no room and the active room did                   NOT change: report it, or the session silently stays put"]
    pub fn set_active(&mut self, index: usize) -> Option<&RoomSpec> {
        if index >= self.rooms.len() {
            return None;
        }
        self.active = index;
        Some(&self.rooms[index])
    }

    /// Seat the session in the room with this authored id, or refuse.
    ///
    /// This is the one id-to-active road. Callers must not resolve the id and
    /// write the private field themselves.
    #[must_use = "`None` means the id matched no room and the active room did                   NOT change: report it, or the session silently stays put"]
    pub fn set_active_by_id(&mut self, id: &str) -> Option<&RoomSpec> {
        let index = self.room_index_by_id(id)?;
        self.set_active(index)
    }

    /// Find the loading zone the controlled body's frame path enters this tick.
    ///
    /// CC2 (the sweep law, docs/concepts/movement-collision.md): loading-zone
    /// entry is path-dependent. The reader sweeps the body's `delta` path
    /// through the zone with the one swept primitive (`aabb_path_contacts`),
    /// so a fast body (blink / dash / Sanic run) cannot tunnel an
    /// overlap-fire `Walk` zone. With `delta == 0` this is the discrete
    /// overlap. `Door` zones stay button-gated (`is_ready` requires
    /// `wants_interact`), so the sweep only helps them. `EdgeExit` bands are
    /// backed by the world edge, so a tunnel past one is an OOB that the CC3
    /// oracle catches.
    pub fn transition_for_player(
        &self,
        player_aabb: ae::Aabb,
        delta: ae::Vec2,
        wants_interact: bool,
    ) -> Option<RoomTransition> {
        let zone = self
            .active_loading_zones()
            .iter()
            .find(|zone| {
                ae::cast::aabb_path_contacts(
                    player_aabb.center(),
                    player_aabb.half_size(),
                    delta,
                    zone.aabb,
                ) && zone.is_ready(wants_interact)
            })?
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
            // CC2 §3.3: flag water/climbable state regions thin enough for a
            // fast body to tunnel between frames (the authoring half of the
            // sweep law — the per-frame reads stay discrete).
            for w in room.world.thin_region_warnings() {
                warnings.push(format!("room {room_index} '{}' {w}", room.world.name));
            }
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

#[cfg(test)]
mod room_identity_tests {
    use super::*;

    /// The authored id is a room's only name; its display title is not a
    /// second one.
    ///
    /// The title matches the `ambition_platformer2d_ldtk` converter format,
    /// `format!("Ambition: {}", area_id.replace('_', " "))`. That crate
    /// depends on this one, so the format cannot be imported here. The first
    /// assertion checks that the title differs from the id.
    ///
    /// If a caption resolved, `same_destination` would read one room named
    /// two ways as two destinations. This test fails if a title alias returns.
    #[test]
    fn a_rooms_display_title_is_not_a_second_name_for_it() {
        let id = "lab_genesis";
        let title = format!("Ambition: {}", id.replace('_', " "));
        // Anti-vacuity: if these were equal the rest would pass for the wrong
        // reason, and the converter's format is exactly what keeps them apart.
        assert_ne!(
            id, title,
            "the converter's title format no longer differs from the id, so this \
             test would prove nothing"
        );

        let world = ae::World::new(
            title.clone(),
            ae::Vec2::new(320.0, 240.0),
            ae::Vec2::new(16.0, 16.0),
            Vec::new(),
        );
        let rooms = vec![RoomSpec::new(id, world)];
        let set = RoomSet::from_parts_or_panic(id, rooms, Vec::new());

        assert_eq!(
            set.room_index_by_id(id),
            Some(0),
            "a room must still resolve by its authored id"
        );
        assert_eq!(
            set.room_index_by_id(&title),
            None,
            "a room's display title resolved as its id: the second name is back, \
             and `same_destination` can now read one room as two destinations"
        );
    }

    fn two_rooms() -> RoomSet {
        let world = |name: &str| {
            ae::World::new(
                name.to_string(),
                ae::Vec2::new(320.0, 240.0),
                ae::Vec2::new(16.0, 16.0),
                Vec::new(),
            )
        };
        RoomSet::from_parts_or_panic(
            "hub",
            vec![
                RoomSpec::new("hub", world("hub")),
                RoomSpec::new("cellar", world("cellar")),
            ],
            Vec::new(),
        )
    }

    /// An index that names no room moves nobody.
    ///
    /// The second assertion checks that the room did not move; a clamp that
    /// returns `None` after it moves the room fails it.
    ///
    /// Every refusal is made from room 0. A clamp goes to `len - 1` (room 1),
    /// so a refusal from room 1 would not detect it.
    #[test]
    fn an_index_that_names_no_room_is_refused_and_nothing_moves() {
        let mut set = two_rooms();
        assert!(
            set.set_active(1).is_some(),
            "a valid index must still seat the session"
        );
        assert_eq!(set.active_spec().id, "cellar");
        assert!(set.set_active(0).is_some());

        assert!(
            set.set_active(7).is_none(),
            "room 7 of a set of two was accepted"
        );
        assert_eq!(
            set.active(),
            0,
            "the refusal moved the active room anyway — the clamp is back, only              now it reports itself as a refusal"
        );
        assert_eq!(set.active_spec().id, "hub");

        // `usize::MAX` is the value the 2026-09-14 poison staged by accident.
        assert!(set.set_active(usize::MAX).is_none());
        assert_eq!(set.active(), 0);
    }

    /// A set that cannot seat anybody is not built.
    ///
    /// An unresolvable start id and an empty `rooms` both refuse. The second
    /// half of each arm checks that the caller gets no set back with the
    /// `Err`.
    #[test]
    fn a_set_that_cannot_seat_anybody_is_refused_rather_than_built() {
        let world = || {
            ae::World::new(
                "w".to_string(),
                ae::Vec2::new(320.0, 240.0),
                ae::Vec2::new(16.0, 16.0),
                Vec::new(),
            )
        };

        // An id no room carries. The old fallback ran the caller's session in
        // `hub` and said nothing.
        let refused = RoomSet::try_from_parts(
            "a_room_this_world_does_not_have",
            vec![
                RoomSpec::new("hub", world()),
                RoomSpec::new("cellar", world()),
            ],
            Vec::new(),
        );
        assert_eq!(
            refused
                .map(|_| "BUILT A SET")
                .map_err(|why| why)
                .err()
                .expect("an unresolvable start room built a set anyway"),
            RoomSetRefused::UnknownStartRoom {
                start_room: "a_room_this_world_does_not_have".to_string(),
                rooms: vec!["hub".to_string(), "cellar".to_string()],
            },
        );

        // No rooms at all. The old constructor returned a set whose
        // `active_spec()` panics, at whatever unrelated moment first read it.
        assert_eq!(
            RoomSet::try_from_parts("hub", Vec::new(), Vec::new())
                .map(|_| ())
                .err()
                .expect("a set holding no rooms was built with an index naming nothing"),
            RoomSetRefused::NoRooms {
                start_room: "hub".to_string()
            },
        );

        // Anti-vacuity: the same call with a resolvable id must SUCCEED, or
        // both assertions above would hold for a constructor that refuses
        // everything.
        let built = RoomSet::try_from_parts("cellar", vec![
            RoomSpec::new("hub", world()),
            RoomSpec::new("cellar", world()),
        ], Vec::new())
        .expect("a set holding `cellar` can start in it");
        assert_eq!(built.active(), 1);
        assert_eq!(built.start(), 1);
        assert_eq!(built.active_spec().id, "cellar");
    }

    /// Two rooms under one id give two answers to "which room is this".
    ///
    /// `by_id` is a `HashMap`, so a duplicate insert keeps the last room; it
    /// drives start resolution and authored links. [`RoomSet::room_index_by_id`]
    /// is a linear `position()`, so it finds the first; it drives
    /// `set_active_by_id`. The test asserts this disagreement as well as the
    /// refusal, so it shows why `DuplicateRoomId` is a correctness rule.
    #[test]
    fn two_rooms_with_one_id_are_refused_because_the_two_lookup_roads_disagree() {
        let world = || {
            ae::World::new(
                "w".to_string(),
                ae::Vec2::new(320.0, 240.0),
                ae::Vec2::new(16.0, 16.0),
                Vec::new(),
            )
        };
        let duplicated = || vec![RoomSpec::new("lab", world()), RoomSpec::new("lab", world())];

        assert_eq!(
            RoomSet::try_from_parts("lab", duplicated(), Vec::new())
                .map(|_| ())
                .err()
                .expect("a set with two `lab`s was built"),
            RoomSetRefused::DuplicateRoomId {
                id: "lab".to_string(),
                first: 0,
                second: 1,
            },
        );

        // Show the disagreement: the `HashMap` road keeps the last duplicate
        // (1) and the `position` road finds the first (0).
        let mut by_id = std::collections::HashMap::new();
        for (index, room) in duplicated().iter().enumerate() {
            by_id.insert(room.id.clone(), index);
        }
        let hashmap_road = by_id["lab"];
        let position_road = duplicated().iter().position(|room| room.id == "lab").unwrap();
        assert_ne!(
            hashmap_road, position_road,
            "the two roads agree on a duplicate id, so this refusal is guarding nothing \
             and the reason recorded on `DuplicateRoomId` is wrong"
        );

        // Anti-vacuity: distinct ids still build.
        let built = RoomSet::try_from_parts(
            "lab",
            vec![RoomSpec::new("lab", world()), RoomSpec::new("cellar", world())],
            Vec::new(),
        )
        .expect("two distinctly named rooms are a perfectly ordinary set");
        assert_eq!(built.active_spec().id, "lab");
    }

    /// The start road and the active road use one mutation law.
    ///
    /// `set_start_by_id` must go through `set_active`, so a rule added to
    /// `set_active` also applies to it.
    #[test]
    fn setting_the_start_room_moves_the_active_room_through_the_same_road() {
        let mut set = two_rooms();
        assert!(set.set_start_by_id("cellar"));
        assert_eq!(set.active(), 1);
        assert_eq!(set.start(), 1);

        assert!(
            !set.set_start_by_id("a_room_this_world_does_not_have"),
            "an unresolvable id reported success"
        );
        assert_eq!(
            (set.active(), set.start()),
            (1, 1),
            "a refused start moved the session anyway"
        );
    }

    /// The id road refuses the same way, and it is the road two callers outside
    /// this crate used to hand-roll by resolving an id and assigning the field.
    #[test]
    fn an_id_that_names_no_room_is_refused_and_nothing_moves() {
        let mut set = two_rooms();
        assert_eq!(
            set.set_active_by_id("cellar").map(|room| room.id.as_str()),
            Some("cellar"),
            "an authored id must seat the session in that room and hand it back"
        );

        assert!(
            set.set_active_by_id("a_room_this_world_does_not_have")
                .is_none()
        );
        assert_eq!(
            set.active(),
            1,
            "a refused id moved the active room, so a caller that ignores the              `None` runs in a room nobody asked for"
        );
    }
}

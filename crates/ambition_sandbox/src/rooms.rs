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
use bevy::prelude::{Res, ResMut, Resource};
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
            LoadingZoneActivation::EdgeExit => {
                format!("{}: {}", self.activation.label(), self.name)
            }
            LoadingZoneActivation::Door => {
                format!(
                    "{}: {} (Interact / double-tap up)",
                    self.activation.label(),
                    self.name
                )
            }
        }
    }
}

/// Track the music identifier the active room would like to play.
///
/// Written by `sync_room_music_request` from `ActiveRoomMetadata`,
/// consumed by `audio::apply_encounter_music` as the "default track"
/// when no encounter override is active. The encounter system retains
/// priority — `EncounterMusicRequest::desired_track = Some(...)`
/// overrides this resource the same way it overrides the sandbox-wide
/// default music track. Empty/absent room music falls back to
/// `sandbox_data.audio.default_music_track`.
#[derive(Resource, Clone, Debug, Default)]
pub struct RoomMusicRequest {
    pub desired_track: Option<String>,
}

/// Mirrors `RoomSet::active_metadata()` as a standalone Bevy resource.
///
/// Synced by `sync_active_room_metadata` each frame the active room
/// changes. Consumers (room music selection, ambient layer selection,
/// renderer palette swaps) can subscribe via `Res<ActiveRoomMetadata>`
/// + change detection without importing the larger `RoomSet` type.
#[derive(Resource, Clone, Debug, Default)]
pub struct ActiveRoomMetadata(pub RoomMetadata);

/// Optional declarative room metadata authored on LDtk levels.
///
/// LDtk level fields `biome` / `music_track` / `ambient_profile` /
/// `visual_theme` (added by `tools/add_biome_level_fields.py`) land
/// here. Every field is optional so existing levels keep working
/// without a value. The first non-empty value among an active area's
/// member levels wins; future systems can refine this if needed
/// (e.g. dominant-vote, level-position weighted).
///
/// Consumers: room music selection, ambient layer selection,
/// renderer palette/theme variants. This struct is intentionally
/// non-exhaustive — adding a metadata seam is cheaper than adding a
/// new resource per consumer.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RoomMetadata {
    pub biome: Option<String>,
    pub music_track: Option<String>,
    pub ambient_profile: Option<String>,
    pub visual_theme: Option<String>,
}

impl RoomMetadata {
    pub fn is_empty(&self) -> bool {
        self.biome.is_none()
            && self.music_track.is_none()
            && self.ambient_profile.is_none()
            && self.visual_theme.is_none()
    }

    /// Fold `other` into `self`, preferring values already set.
    /// LDtk active areas can span multiple levels; the first level
    /// with a non-empty value wins so author intent is predictable.
    pub fn merge(&mut self, other: RoomMetadata) {
        if self.biome.is_none() {
            self.biome = other.biome;
        }
        if self.music_track.is_none() {
            self.music_track = other.music_track;
        }
        if self.ambient_profile.is_none() {
            self.ambient_profile = other.ambient_profile;
        }
        if self.visual_theme.is_none() {
            self.visual_theme = other.visual_theme;
        }
    }
}

/// Complete room data used by the Bevy sandbox.
#[derive(Clone, Debug)]
pub struct RoomSpec {
    pub id: String,
    pub world: ae::World,
    pub loading_zones: Vec<LoadingZone>,
    pub metadata: RoomMetadata,
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
    /// Index of the room the player starts in on a fresh sandbox.
    /// Captured at `from_parts` time so the "reset sandbox" flow can
    /// warp the player back without round-tripping through LDtk.
    pub start: usize,
    graph: Graph<String, TransitionEdge>,
    room_nodes: Vec<NodeIndex>,
}

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

    pub fn active_metadata(&self) -> &RoomMetadata {
        &self.active_spec().metadata
    }

    pub fn set_active(&mut self, index: usize) -> &RoomSpec {
        self.active = index.min(self.rooms.len().saturating_sub(1));
        self.active_spec()
    }

    pub fn transition_for_player(
        &self,
        player: &ae::Player,
        wants_interact: bool,
    ) -> Option<RoomTransition> {
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
    ae::Vec2::new(
        zone.center().x,
        zone.bottom() - PLAYER_HALF_H - SPAWN_MARGIN,
    )
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
                let candidate =
                    clamp_spawn_to_room(world, ae::Vec2::new(base.x, base.y + dy), half);
                if player_body_clear(world, candidate, half) {
                    return candidate;
                }
            } else {
                for sign in [-1.0_f32, 1.0] {
                    let dx = sign * x_step as f32 * STEP;
                    let candidate =
                        clamp_spawn_to_room(world, ae::Vec2::new(base.x + dx, base.y + dy), half);
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
        pos.x
            .clamp(half.x + SPAWN_MARGIN, world.size.x - half.x - SPAWN_MARGIN),
        pos.y
            .clamp(half.y + SPAWN_MARGIN, world.size.y - half.y - SPAWN_MARGIN),
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

/// Mirror `RoomSet::active_metadata()` into the `ActiveRoomMetadata`
/// resource, but only when the metadata actually changes. The
/// PartialEq guard means change-detection consumers (e.g. a future
/// room-music selector) only fire when the active room's biome /
/// music_track / ambient / theme really differ — not on every frame.
pub fn sync_active_room_metadata(room_set: Res<RoomSet>, mut active: ResMut<ActiveRoomMetadata>) {
    let current = room_set.active_metadata().clone();
    if current != active.0 {
        active.0 = current;
    }
}

/// Push the active room's `music_track` into `RoomMusicRequest` so the
/// audio system knows the room-default track when no encounter
/// override is active. Empty values clear the request, falling back to
/// `sandbox_data.audio.default_music_track`.
pub fn sync_room_music_request(
    active: Res<ActiveRoomMetadata>,
    mut request: ResMut<RoomMusicRequest>,
) {
    let next = active.0.music_track.clone();
    if next != request.desired_track {
        request.desired_track = next;
    }
}

#[cfg(test)]
mod metadata_tests {
    use super::*;

    fn empty_world(name: &str) -> ae::World {
        ae::World::new(
            name,
            ae::Vec2::new(640.0, 480.0),
            ae::Vec2::new(96.0, 96.0),
            Vec::new(),
        )
    }

    fn spec_with(meta: RoomMetadata, id: &str) -> RoomSpec {
        RoomSpec {
            id: id.into(),
            world: empty_world(id),
            loading_zones: Vec::new(),
            metadata: meta,
        }
    }

    #[test]
    fn active_metadata_returns_active_room_metadata() {
        let m1 = RoomMetadata {
            biome: Some("hub".into()),
            music_track: Some("hub_loop".into()),
            ambient_profile: None,
            visual_theme: None,
        };
        let m2 = RoomMetadata {
            biome: Some("cave".into()),
            music_track: Some("cave_loop".into()),
            ambient_profile: Some("damp".into()),
            visual_theme: None,
        };
        let mut set = RoomSet::from_parts(
            "first",
            vec![
                spec_with(m1.clone(), "first"),
                spec_with(m2.clone(), "second"),
            ],
            Vec::new(),
        );
        assert_eq!(set.active_metadata(), &m1);
        set.set_active(1);
        assert_eq!(set.active_metadata(), &m2);
    }

    #[test]
    fn sync_room_music_request_mirrors_metadata_music_track() {
        use bevy::prelude::*;
        let mut app = App::new();
        app.insert_resource(ActiveRoomMetadata(RoomMetadata {
            biome: Some("cave".into()),
            music_track: Some("cave_loop".into()),
            ambient_profile: None,
            visual_theme: None,
        }));
        app.insert_resource(RoomMusicRequest::default());
        app.add_systems(Update, sync_room_music_request);
        app.update();
        assert_eq!(
            app.world().resource::<RoomMusicRequest>().desired_track,
            Some("cave_loop".into())
        );

        // Empty active metadata clears the request.
        app.world_mut()
            .resource_mut::<ActiveRoomMetadata>()
            .0
            .music_track = None;
        app.update();
        assert_eq!(
            app.world().resource::<RoomMusicRequest>().desired_track,
            None
        );
    }

    #[test]
    fn sync_active_room_metadata_publishes_active_value() {
        use bevy::prelude::*;
        let mut app = App::new();
        let m_hub = RoomMetadata {
            biome: Some("hub".into()),
            music_track: Some("hub_loop".into()),
            ambient_profile: None,
            visual_theme: None,
        };
        let m_lab = RoomMetadata {
            biome: Some("lab".into()),
            music_track: Some("lab_loop".into()),
            ambient_profile: None,
            visual_theme: None,
        };
        let set = RoomSet::from_parts(
            "hub",
            vec![
                spec_with(m_hub.clone(), "hub"),
                spec_with(m_lab.clone(), "lab"),
            ],
            Vec::new(),
        );
        app.insert_resource(set);
        app.insert_resource(ActiveRoomMetadata::default());
        app.add_systems(Update, sync_active_room_metadata);
        app.update();
        assert_eq!(&app.world().resource::<ActiveRoomMetadata>().0, &m_hub);

        app.world_mut().resource_mut::<RoomSet>().set_active(1);
        app.update();
        assert_eq!(&app.world().resource::<ActiveRoomMetadata>().0, &m_lab);
    }

    #[test]
    fn room_metadata_is_empty_default_is_true() {
        let m = RoomMetadata::default();
        assert!(m.is_empty());
    }

    #[test]
    fn room_metadata_is_empty_false_when_any_field_set() {
        let mut m = RoomMetadata::default();
        m.biome = Some("hub".into());
        assert!(!m.is_empty());

        let m = RoomMetadata {
            biome: None,
            music_track: Some("loop".into()),
            ambient_profile: None,
            visual_theme: None,
        };
        assert!(!m.is_empty());
    }

    #[test]
    fn room_metadata_merge_preserves_existing_values() {
        let mut a = RoomMetadata {
            biome: Some("hub".into()),
            music_track: None,
            ambient_profile: None,
            visual_theme: Some("blue".into()),
        };
        let b = RoomMetadata {
            biome: Some("CONFLICT".into()),        // ignored — a.biome wins
            music_track: Some("hub_loop".into()),  // takes effect — a.music_track was None
            ambient_profile: Some("damp".into()),  // takes effect
            visual_theme: Some("CONFLICT".into()), // ignored
        };
        a.merge(b);
        assert_eq!(a.biome.as_deref(), Some("hub"));
        assert_eq!(a.music_track.as_deref(), Some("hub_loop"));
        assert_eq!(a.ambient_profile.as_deref(), Some("damp"));
        assert_eq!(a.visual_theme.as_deref(), Some("blue"));
    }

    #[test]
    fn loading_zone_activation_label_is_non_empty() {
        assert!(!LoadingZoneActivation::EdgeExit.label().is_empty());
        assert!(!LoadingZoneActivation::Door.label().is_empty());
    }

    #[test]
    fn loading_zone_is_ready_respects_activation() {
        let edge = LoadingZone {
            id: "x".into(),
            name: "x".into(),
            aabb: ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(1.0, 1.0)),
            activation: LoadingZoneActivation::EdgeExit,
        };
        // EdgeExit is always ready (auto-fires on overlap).
        assert!(edge.is_ready(false));
        assert!(edge.is_ready(true));

        let door = LoadingZone {
            id: "y".into(),
            name: "y".into(),
            aabb: ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(1.0, 1.0)),
            activation: LoadingZoneActivation::Door,
        };
        // Door requires interact press.
        assert!(!door.is_ready(false));
        assert!(door.is_ready(true));
    }

    #[test]
    fn loading_zone_hint_includes_door_prompt() {
        let door = LoadingZone {
            id: "lab_door".into(),
            name: "lab door".into(),
            aabb: ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(1.0, 1.0)),
            activation: LoadingZoneActivation::Door,
        };
        let hint = door.hint(false);
        assert!(hint.contains("door"));
        assert!(hint.contains("Interact") || hint.contains("interact"));
        assert!(hint.contains("lab door"));
    }

    #[test]
    fn loading_zone_hint_for_edge_exit_skips_prompt() {
        let edge = LoadingZone {
            id: "east_exit".into(),
            name: "east exit".into(),
            aabb: ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(1.0, 1.0)),
            activation: LoadingZoneActivation::EdgeExit,
        };
        let hint = edge.hint(false);
        assert!(hint.contains("east exit"));
        // Auto-firing edge exits don't need an Interact prompt.
        assert!(!hint.contains("Interact"));
    }
}

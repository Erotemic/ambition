//! Data-driven sandbox room-set and loading-zone graph.
//!
//! Rooms can be authored by RON fixtures or by the LDtk adapter. This module
//! turns the resolved room manifest into engine worlds plus a directed `petgraph` room graph.
//! Loading-zone links point at destination zones by name, so authoring no longer
//! requires brittle hand-written spawn coordinates.

use std::collections::HashMap;

use ambition_engine as ae;
use ambition_engine::AabbExt;
use bevy::prelude::Resource;
use petgraph::graph::{Graph, NodeIndex};
use petgraph::visit::EdgeRef;
use petgraph::Direction;

use crate::data::{self, BlockSpec, BlinkWallTierSpec, LoadingZoneActivationSpec, RoomManifestSpec, WallSideSpec};

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

impl From<LoadingZoneActivationSpec> for LoadingZoneActivation {
    fn from(value: LoadingZoneActivationSpec) -> Self {
        match value {
            LoadingZoneActivationSpec::EdgeExit => Self::EdgeExit,
            LoadingZoneActivationSpec::Door => Self::Door,
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
    pub fn from_manifest(manifest: &RoomManifestSpec) -> Self {
        let rooms = manifest.rooms.iter().map(build_room_from_data).collect::<Vec<_>>();
        let mut graph = Graph::<String, TransitionEdge>::new();
        let mut room_nodes = Vec::new();
        let mut by_id = HashMap::new();
        for (index, room) in rooms.iter().enumerate() {
            let node = graph.add_node(room.id.clone());
            room_nodes.push(node);
            by_id.insert(room.id.clone(), (index, node));
        }

        for link in &manifest.links {
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

        let active = by_id.get(&manifest.start_room).map(|(index, _)| *index).unwrap_or(0);
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
const FLOOR: f32 = 48.0;
const CEILING: f32 = 24.0;
const EDGE_ARRIVAL_INSET: f32 = 92.0;
const PLAYER_HALF_W: f32 = 14.0;
const PLAYER_HALF_H: f32 = 23.0;
const SPAWN_MARGIN: f32 = 3.0;

fn build_room_from_data(room: &data::RoomSpecData) -> RoomSpec {
    let mut blocks = Vec::new();
    let size = data::vec2(room.size);
    if room.shell.enabled {
        shell_with_openings(&mut blocks, size.x, size.y, &room.shell.openings);
    }
    for block in &room.blocks {
        blocks.push(build_block(block));
    }
    let loading_zones = room
        .zones
        .iter()
        .map(|zone| LoadingZone {
            id: zone.id.clone(),
            name: zone.name.clone(),
            activation: zone.activation.into(),
            aabb: ae::aabb_from_min_size(data::vec2(zone.min), data::vec2(zone.size)),
        })
        .collect();
    let objects = build_room_objects(&room.objects);

    RoomSpec {
        id: room.id.clone(),
        world: ae::World {
            name: room.name.clone(),
            size,
            spawn: data::vec2(room.spawn),
            blocks,
            objects,
        },
        loading_zones,
    }
}

fn build_block(block: &BlockSpec) -> ae::Block {
    match block {
        BlockSpec::Solid { name, min, size } => ae::Block::solid(name.clone(), data::vec2(*min), data::vec2(*size)),
        BlockSpec::BlinkWall { name, min, size, tier } => {
            let tier = match tier {
                BlinkWallTierSpec::Soft => ae::BlinkWallTier::Soft,
                BlinkWallTierSpec::Hard => ae::BlinkWallTier::Hard,
            };
            ae::Block::blink_wall(name.clone(), data::vec2(*min), data::vec2(*size), tier)
        }
        BlockSpec::OneWay { name, min, size } => ae::Block::one_way(name.clone(), data::vec2(*min), data::vec2(*size)),
        BlockSpec::Hazard { name, min, size } => ae::Block::hazard(name.clone(), data::vec2(*min), data::vec2(*size)),
        BlockSpec::PogoOrb { name, center, radius } => ae::Block::pogo_orb(name.clone(), data::vec2(*center), *radius),
        BlockSpec::Rebound { name, min, size, impulse } => {
            ae::Block::rebound(name.clone(), data::vec2(*min), data::vec2(*size), data::vec2(*impulse))
        }
    }
}

fn build_room_objects(objects: &[data::RoomObjectSpec]) -> Vec<ae::RoomObject> {
    objects.iter().map(build_room_object).collect()
}

fn build_room_object(object: &data::RoomObjectSpec) -> ae::RoomObject {
    match object {
        data::RoomObjectSpec::DamageVolume { id, name, min, size, damage, path } => {
            let aabb = object_aabb(*min, *size);
            let mut volume = ae::DamageVolume::new(id.clone(), aabb, *damage);
            volume.motion = path.as_ref().map(kinematic_path_spec);
            ae::RoomObject::new(id.clone(), name.clone(), aabb, ae::RoomObjectKind::DamageVolume(volume))
        }
        data::RoomObjectSpec::Interactable { id, name, prompt, min, size, kind } => {
            let aabb = object_aabb(*min, *size);
            let interactable = ae::Interactable::new(id.clone(), prompt.clone(), aabb, interaction_kind(kind));
            ae::RoomObject::new(id.clone(), name.clone(), aabb, ae::RoomObjectKind::Interactable(interactable))
        }
        data::RoomObjectSpec::Pickup { id, name, min, size, kind } => {
            let aabb = object_aabb(*min, *size);
            let pickup = ae::Pickup::new(id.clone(), pickup_kind(kind));
            ae::RoomObject::new(id.clone(), name.clone(), aabb, ae::RoomObjectKind::Pickup(pickup))
        }
        data::RoomObjectSpec::Chest { id, name, min, size, reward } => {
            let aabb = object_aabb(*min, *size);
            let chest = ae::Chest::new(id.clone(), reward.as_ref().map(pickup_kind));
            ae::RoomObject::new(id.clone(), name.clone(), aabb, ae::RoomObjectKind::Chest(chest))
        }
        data::RoomObjectSpec::Breakable { id, name, min, size, max_hp, respawn, solid } => {
            let aabb = object_aabb(*min, *size);
            let mut breakable = ae::Breakable::new(id.clone(), *max_hp);
            if let Some(respawn) = respawn {
                breakable.respawn = respawn_policy(*respawn);
            }
            breakable.solid = *solid;
            ae::RoomObject::new(id.clone(), name.clone(), aabb, ae::RoomObjectKind::Breakable(breakable))
        }
        data::RoomObjectSpec::EnemySpawn { id, name, min, size, brain } => {
            let aabb = object_aabb(*min, *size);
            ae::RoomObject::new(id.clone(), name.clone(), aabb, ae::RoomObjectKind::EnemySpawn(enemy_brain(brain)))
        }
        data::RoomObjectSpec::BossSpawn { id, name, min, size, brain } => {
            let aabb = object_aabb(*min, *size);
            ae::RoomObject::new(id.clone(), name.clone(), aabb, ae::RoomObjectKind::BossSpawn(boss_brain(brain)))
        }
        data::RoomObjectSpec::KinematicPath { id, name, min, size, points, speed, mode } => {
            let aabb = object_aabb(*min, *size);
            let path = ae::KinematicPath {
                points: points.iter().copied().map(data::vec2).collect(),
                speed: *speed,
                mode: kinematic_path_mode(*mode),
                start_offset_seconds: 0.0,
            };
            ae::RoomObject::new(id.clone(), name.clone(), aabb, ae::RoomObjectKind::KinematicPath(path))
        }
        data::RoomObjectSpec::DebugLabel { id, name, position, text, category } => {
            let pos = data::vec2(*position);
            let aabb = ae::Aabb::new(pos, ae::Vec2::splat(1.0));
            let label = ae::DebugLabel::new(text.clone(), pos, debug_label_kind(*category));
            ae::RoomObject::new(id.clone(), name.clone(), aabb, ae::RoomObjectKind::DebugLabel(label))
        }
    }
}

fn object_aabb(min: [f32; 2], size: [f32; 2]) -> ae::Aabb {
    ae::aabb_from_min_size(data::vec2(min), data::vec2(size))
}

fn interaction_kind(kind: &data::InteractionKindSpec) -> ae::InteractionKind {
    match kind {
        data::InteractionKindSpec::Door { target } => ae::InteractionKind::Door { target: target.clone() },
        data::InteractionKindSpec::Npc { dialogue_id } => ae::InteractionKind::Npc { dialogue_id: dialogue_id.clone() },
        data::InteractionKindSpec::Chest => ae::InteractionKind::Chest,
        data::InteractionKindSpec::Pickup => ae::InteractionKind::Pickup,
        data::InteractionKindSpec::Breakable => ae::InteractionKind::Breakable,
        data::InteractionKindSpec::Custom(value) => ae::InteractionKind::Custom(value.clone()),
    }
}

fn pickup_kind(kind: &data::PickupKindSpec) -> ae::PickupKind {
    match kind {
        data::PickupKindSpec::Health { amount } => ae::PickupKind::Health { amount: *amount },
        data::PickupKindSpec::Currency { amount } => ae::PickupKind::Currency { amount: *amount },
        data::PickupKindSpec::Ability { ability_id } => ae::PickupKind::Ability { ability_id: ability_id.clone() },
        data::PickupKindSpec::StoryFlag { flag } => ae::PickupKind::StoryFlag { flag: flag.clone() },
        data::PickupKindSpec::Custom(value) => ae::PickupKind::Custom(value.clone()),
    }
}

fn respawn_policy(policy: data::RespawnPolicySpec) -> ae::RespawnPolicy {
    match policy {
        data::RespawnPolicySpec::Never => ae::RespawnPolicy::Never,
        data::RespawnPolicySpec::AfterSeconds(seconds) => ae::RespawnPolicy::AfterSeconds(seconds),
        data::RespawnPolicySpec::OnRoomReload => ae::RespawnPolicy::OnRoomReload,
        data::RespawnPolicySpec::Persistent => ae::RespawnPolicy::Persistent,
    }
}

fn kinematic_path_spec(spec: &data::KinematicPathSpec) -> ae::KinematicPath {
    ae::KinematicPath {
        points: spec.points.iter().copied().map(data::vec2).collect(),
        speed: spec.speed,
        mode: kinematic_path_mode(spec.mode),
        start_offset_seconds: 0.0,
    }
}

fn kinematic_path_mode(mode: data::KinematicPathModeSpec) -> ae::KinematicPathMode {
    match mode {
        data::KinematicPathModeSpec::Once => ae::KinematicPathMode::Once,
        data::KinematicPathModeSpec::Loop => ae::KinematicPathMode::Loop,
        data::KinematicPathModeSpec::PingPong => ae::KinematicPathMode::PingPong,
    }
}

fn debug_label_kind(kind: data::DebugLabelKindSpec) -> ae::DebugLabelKind {
    match kind {
        data::DebugLabelKindSpec::Room => ae::DebugLabelKind::Room,
        data::DebugLabelKindSpec::LoadingZone => ae::DebugLabelKind::LoadingZone,
        data::DebugLabelKindSpec::Hazard => ae::DebugLabelKind::Hazard,
        data::DebugLabelKindSpec::Enemy => ae::DebugLabelKind::Enemy,
        data::DebugLabelKindSpec::Boss => ae::DebugLabelKind::Boss,
        data::DebugLabelKindSpec::Interactable => ae::DebugLabelKind::Interactable,
        data::DebugLabelKindSpec::Pickup => ae::DebugLabelKind::Pickup,
        data::DebugLabelKindSpec::Custom => ae::DebugLabelKind::Custom,
    }
}

fn enemy_brain(brain: &data::EnemyBrainSpec) -> ae::EnemyBrain {
    match brain {
        data::EnemyBrainSpec::Passive => ae::EnemyBrain::Passive,
        data::EnemyBrainSpec::Patrol { path_id } => ae::EnemyBrain::Patrol { path_id: path_id.clone() },
        data::EnemyBrainSpec::Guard { leash_radius } => ae::EnemyBrain::Guard { leash_radius: *leash_radius },
        data::EnemyBrainSpec::Custom(value) => ae::EnemyBrain::Custom(value.clone()),
    }
}

fn boss_brain(brain: &data::BossBrainSpec) -> ae::BossBrain {
    match brain {
        data::BossBrainSpec::Dormant => ae::BossBrain::Dormant,
        data::BossBrainSpec::PhaseScript { script_id } => ae::BossBrain::PhaseScript { script_id: script_id.clone() },
        data::BossBrainSpec::Custom(value) => ae::BossBrain::Custom(value.clone()),
    }
}


fn shell_with_openings(blocks: &mut Vec<ae::Block>, w: f32, h: f32, openings: &[data::WallOpeningSpec]) {
    blocks.push(ae::Block::solid("floor", ae::Vec2::new(0.0, h - FLOOR), ae::Vec2::new(w, FLOOR)));
    blocks.push(ae::Block::solid("ceiling", ae::Vec2::new(0.0, 0.0), ae::Vec2::new(w, CEILING)));

    for (side, x, name) in [
        (WallSideSpec::Left, 0.0, "left wall"),
        (WallSideSpec::Right, w - WALL, "right wall"),
    ] {
        let mut spans = vec![(0.0, h)];
        for opening in openings.iter().filter(|opening| opening.side == side) {
            let open_min = opening.y.max(CEILING);
            let open_max = (opening.y + opening.height).min(h - FLOOR);
            if open_min >= open_max {
                continue;
            }
            let mut next = Vec::new();
            for (span_min, span_max) in spans {
                if open_max <= span_min || open_min >= span_max {
                    next.push((span_min, span_max));
                } else {
                    if span_min < open_min {
                        next.push((span_min, open_min));
                    }
                    if open_max < span_max {
                        next.push((open_max, span_max));
                    }
                }
            }
            spans = next;
        }
        for (index, (span_min, span_max)) in spans.into_iter().enumerate() {
            let height = span_max - span_min;
            if height > 0.5 {
                let block_name = if index == 0 { name } else { "wall segment" };
                blocks.push(ae::Block::solid(block_name, ae::Vec2::new(x, span_min), ae::Vec2::new(WALL, height)));
            }
        }
    }
}

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

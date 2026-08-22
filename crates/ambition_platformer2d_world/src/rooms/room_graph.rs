//! RoomSpec + the transition graph types.

use super::*;

/// Complete room data used by the Bevy sandbox.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct RoomSpec {
    pub id: String,
    pub world: ae::World,
    pub loading_zones: Vec<LoadingZone>,
    pub metadata: RoomMetadata,
    pub camera_zones: Vec<CameraZoneSpec>,
    /// LDtk-authored path index for platforms, hazards, NPC patrols,
    /// camera rails, and future scripted room beats.
    pub kinematic_paths: Vec<KinematicPathSpec>,
    /// LDtk-authored moving platforms for this area. This is the
    /// complete platform set for gameplay: empty means the room has
    /// no moving platforms.
    pub moving_platforms: Vec<crate::platforms::MovingPlatformState>,
    /// LDtk-authored decorative props. Render-only — see [`PropSpec`].
    pub props: Vec<PropSpec>,
    /// LDtk-authored ground held-items (gauntlet / weapon pickups). See
    /// [`GroundItemSpec`].
    pub ground_items: Vec<GroundItemSpec>,
    /// LDtk-authored portal-gun pickups. See [`PortalGunSpawnSpec`].
    pub portal_gun_spawns: Vec<PortalGunSpawnSpec>,
    /// LDtk-authored heal/save shrines. See [`ShrineSpec`].
    pub shrines: Vec<ShrineSpec>,
    /// LDtk-authored localized-gravity zones. See [`GravityZoneSpec`].
    pub gravity_zones: Vec<GravityZoneSpec>,

    // Generic placement families lower through `placements`; the typed vectors
    // below are domain-specific room facets that still need direct access here.
    pub enemy_spawns: Vec<Authored<crate::rooms::EnemySpawnSpec>>,
    pub boss_spawns: Vec<Authored<ambition_entity_catalog::placements::BossBrain>>,
    pub debug_labels: Vec<Authored<crate::debug_label::DebugLabel>>,
    /// ADR 0020 authored mount links: `(rider_id, mount_id)` pairs. A rider
    /// `EnemySpawn` with a `mounted_on` entity-ref emits one; after the room's
    /// actors spawn, the room construction planner turns each pair into a planned
    /// `ambition.mount` relation, matched by
    /// `FeatureId` and installs the `RidingOn`/`MountSlot` link.
    pub mount_links: Vec<(String, String)>,
    /// Authored placement records consumed by the lowering registry.
    pub placements: Vec<crate::placements::PlacementRecord>,
    /// Authored encounter trigger volumes in this room (at most one today).
    /// Carried so `load_encounter_specs` can read a ROOM rather than an
    /// `LdtkProject` — see [`crate::rooms::EncounterTriggerSpec`].
    pub encounter_triggers: Vec<crate::rooms::EncounterTriggerSpec>,
    /// Authored encounter lock walls in this room (at most one today).
    pub lock_walls: Vec<crate::rooms::EncounterLockWallSpec>,
    /// Authored `Switch` command lines in this room. Most switches have none.
    pub switch_commands: Vec<crate::rooms::SwitchCommandSpec>,
}

impl RoomSpec {
    /// A room with the given geometry and no authored entities. The starting
    /// point for generated rooms, fixtures, and demo shells; authored paths
    /// (LDtk) fill every list from the map instead.
    pub fn new(id: impl Into<String>, world: ae::World) -> Self {
        Self {
            id: id.into(),
            world,
            loading_zones: Vec::new(),
            metadata: RoomMetadata::default(),
            camera_zones: Vec::new(),
            kinematic_paths: Vec::new(),
            moving_platforms: Vec::new(),
            props: Vec::new(),
            ground_items: Vec::new(),
            portal_gun_spawns: Vec::new(),
            shrines: Vec::new(),
            gravity_zones: Vec::new(),
            enemy_spawns: Vec::new(),
            boss_spawns: Vec::new(),
            debug_labels: Vec::new(),
            mount_links: Vec::new(),
            placements: Vec::new(),
            encounter_triggers: Vec::new(),
            lock_walls: Vec::new(),
            switch_commands: Vec::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct TransitionEdge {
    pub(crate) from_zone: String,
    pub(crate) to_zone: String,
}

/// Authored directed connection between loading zones in runtime rooms.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
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

/// Presentation-neutral SFX cue reference carried by room IR and room messages.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RoomSfxId(String);

impl RoomSfxId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Bevy message emitted when a room's contents finish STAGING — written by
/// the registry-aware room-placement choke point every staging path uses
/// (initial session build, room transitions, sandbox reset, and LDtk
/// hot-reload restage). The JD4 seam for imperative per-room content
/// staging: a content system reads this instead of change-detecting the
/// active room id or hooking the engine's spawn internals.
///
/// Written via `Commands`, so readers observe it once the staging commands
/// have applied — the room's feature entities are already live.
#[derive(Message, Clone, Debug)]
pub struct RoomLoaded {
    /// The staged room's id (`RoomSpec::id` — the LDtk active-area id).
    pub room_id: String,
}

/// Small room graph for early loading-zone tests.
#[derive(Component, Clone, Debug)]
pub struct RoomSet {
    pub rooms: Vec<RoomSpec>,
    pub active: usize,
    /// Index of the room the player starts in on a fresh sandbox.
    /// Captured at `from_parts` time so the "reset sandbox" flow can
    /// warp the player back without round-tripping through LDtk.
    pub start: usize,
    pub(crate) graph: Graph<String, TransitionEdge>,
    pub(crate) room_nodes: Vec<NodeIndex>,
}

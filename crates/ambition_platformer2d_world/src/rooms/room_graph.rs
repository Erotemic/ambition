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

/// Why a room set could not be built.
///
/// An unresolvable start room refuses; there is no fallback to room 0. An empty
/// `rooms` refuses, because `active = start = 0` would index nothing and make
/// [`RoomSet::active_spec`] and the room-set rollback checksum panic later.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RoomSetRefused {
    /// No rooms at all, so no index can name a live one.
    NoRooms { start_room: String },
    /// The named start room is not in the set. The ids that ARE present are
    /// carried because the usual cause is a typo or a stale id, and a refusal
    /// that does not say what WAS there sends its reader back to the map.
    UnknownStartRoom {
        start_room: String,
        rooms: Vec<String>,
    },
    /// Two rooms answer to one id, so the set holds two answers to *"which
    /// room is this"*.
    ///
    /// The constructor's `by_id` is a `HashMap`, so a duplicate insert keeps
    /// the last room; [`RoomSet::room_index_by_id`] is a linear `position()`,
    /// so it returns the first. With two rooms named `"lab"`, start and
    /// authored links would resolve to room 1 while `set_active_by_id("lab")`
    /// would go to room 0.
    ///
    /// No shipped content has a duplicate. OW1 separates the room definition
    /// from the live occurrence, and a definition id that names two
    /// definitions cannot be the stable half of that pair.
    DuplicateRoomId {
        id: String,
        first: usize,
        second: usize,
    },
}

impl std::fmt::Display for RoomSetRefused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoRooms { start_room } => write!(
                f,
                "a room set holding no rooms cannot start in `{start_room}`, or \
                 anywhere else"
            ),
            Self::UnknownStartRoom { start_room, rooms } => write!(
                f,
                "no room is named `{start_room}`; this set holds {rooms:?}"
            ),
            Self::DuplicateRoomId { id, first, second } => write!(
                f,
                "two rooms are named `{id}` (indices {first} and {second}); the id lookup \
                 would answer {first} and the link/start resolution would answer {second}, \
                 so the set holds two answers to which room this is"
            ),
        }
    }
}

impl std::error::Error for RoomSetRefused {}

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
    /// Still public, which limits the invariant below: a caller with
    /// `&mut RoomSet` can shorten this vector under the private indices. No
    /// caller mutates it after construction. Making it private would move 94
    /// read sites.
    pub rooms: Vec<RoomSpec>,
    /// Which room of [`Self::rooms`] is live, read through [`RoomSet::active`]
    /// and written through [`RoomSet::set_active`] / [`RoomSet::set_active_by_id`].
    ///
    /// Private because it is an invariant, not a free value: it must index
    /// `rooms`. All writes go through the setters, which refuse an
    /// out-of-range index instead of clamping.
    ///
    /// It answers which definition is live, not which live instance this is;
    /// two instances of one room would share this index. OW1 in
    /// `docs/planning/engine/open-world-runtime-and-residency.md` separates
    /// those questions and needs this field to be trustworthy.
    pub(crate) active: usize,
    /// Index of the room the player starts in on a fresh sandbox, read through
    /// [`RoomSet::start`] and written through [`RoomSet::set_start_by_id`].
    /// Captured at `from_parts` time so the "reset sandbox" flow can
    /// warp the player back without round-tripping through LDtk.
    ///
    /// Private for the same reason as [`Self::active`]: it must index `rooms`.
    pub(crate) start: usize,
    pub(crate) graph: Graph<String, TransitionEdge>,
    pub(crate) room_nodes: Vec<NodeIndex>,
}

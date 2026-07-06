//! RoomSpec + the transition graph types.
//!
//! Split out of the former 823-line `rooms/mod.rs` (2026-06-15); the
//! parent re-exports every type so `rooms::*` paths are unchanged.

use super::*;

/// Complete room data used by the Bevy sandbox.
#[derive(Clone, Debug)]
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
    pub moving_platforms: Vec<crate::world::platforms::MovingPlatformState>,
    /// LDtk-authored decorative props. Render-only — see [`PropSpec`].
    pub props: Vec<PropSpec>,
    /// LDtk-authored ground held-items (gauntlet / weapon pickups). See
    /// [`GroundItemSpec`].
    pub ground_items: Vec<GroundItemSpec>,
    /// LDtk-authored portal-gun pickups. See [`PortalGunSpawnSpec`].
    #[cfg(feature = "portal")]
    pub portal_gun_spawns: Vec<PortalGunSpawnSpec>,
    /// LDtk-authored static portals (pre-placed linked pairs). See [`PortalSpec`].
    #[cfg(feature = "portal")]
    pub portals: Vec<PortalSpec>,
    /// LDtk-authored heal/save shrines. See [`ShrineSpec`].
    pub shrines: Vec<ShrineSpec>,
    /// LDtk-authored localized-gravity zones. See [`GravityZoneSpec`].
    pub gravity_zones: Vec<GravityZoneSpec>,

    // --- Per-family authored entity lists; each family spawns through ECS.
    pub hazards: Vec<Authored<crate::combat::DamageVolume>>,
    pub interactables: Vec<Authored<ambition_interaction::Interactable>>,
    pub pickups: Vec<Authored<ambition_interaction::Pickup>>,
    pub chests: Vec<Authored<ambition_interaction::Chest>>,
    pub breakables: Vec<Authored<ambition_interaction::Breakable>>,
    pub enemy_spawns: Vec<Authored<ambition_entity_catalog::placements::CharacterBrain>>,
    pub boss_spawns: Vec<Authored<ambition_entity_catalog::placements::BossBrain>>,
    pub debug_labels: Vec<Authored<crate::debug_label::DebugLabel>>,
    /// ADR 0020 authored mount links: `(rider_id, mount_id)` pairs. A rider
    /// `EnemySpawn` with a `mounted_on` entity-ref emits one; after the room's
    /// actors spawn, `resolve_pending_mount_links` matches each pair by
    /// `FeatureId` and installs the `RidingOn`/`MountSlot` link.
    pub mount_links: Vec<(String, String)>,
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
            #[cfg(feature = "portal")]
            portal_gun_spawns: Vec::new(),
            #[cfg(feature = "portal")]
            portals: Vec::new(),
            shrines: Vec::new(),
            gravity_zones: Vec::new(),
            hazards: Vec::new(),
            interactables: Vec::new(),
            pickups: Vec::new(),
            chests: Vec::new(),
            breakables: Vec::new(),
            enemy_spawns: Vec::new(),
            boss_spawns: Vec::new(),
            debug_labels: Vec::new(),
            mount_links: Vec::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct TransitionEdge {
    pub(crate) from_zone: String,
    pub(crate) to_zone: String,
}

/// Authored directed connection between loading zones in runtime rooms.
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

/// Bevy message emitted when a room transition is triggered (player walks through
/// a loading zone or door). The actual `load_room` call happens in
/// `apply_room_transition_system`, which runs after the player tick in the
/// `CoreSimulation` chain.
///
/// Carries the resolved `RoomTransition` payload and the optional SFX id for the
/// zone type so the apply system can emit the sound at the correct player position
/// after repositioning.
#[derive(Message, Clone, Debug)]
pub struct RoomTransitionRequested {
    pub transition: RoomTransition,
    /// SFX id to play at the new player position after the room loads.
    pub zone_sfx: Option<ambition_sfx::SfxId>,
}

impl RoomTransitionRequested {
    pub fn new(transition: RoomTransition, zone_sfx: Option<ambition_sfx::SfxId>) -> Self {
        Self {
            transition,
            zone_sfx,
        }
    }
}

/// Bevy message emitted when a room's contents finish STAGING — written by
/// `spawn_room_feature_entities`, the one choke point every staging path
/// flows through (initial session build, room transitions, sandbox reset,
/// LDtk hot-reload restage). The JD4 seam for imperative per-room content
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
#[derive(Resource, Clone, Debug)]
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

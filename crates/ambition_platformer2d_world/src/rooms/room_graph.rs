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
/// [`RoomSet::activation_spec`] and the room-set rollback checksum panic later.
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
    /// authored links would resolve to room 1 while `set_activation_by_id("lab")`
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

/// The end of a [`RoomLink`] that names nothing in a set of rooms.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnresolvedEnd {
    SourceRoom,
    SourceZone,
    TargetRoom,
    TargetZone,
}

/// A [`RoomLink`] with an end that does not resolve, and which end.
#[derive(Clone, Debug, PartialEq)]
pub struct UnresolvedLink {
    pub link: RoomLink,
    pub end: UnresolvedEnd,
}

impl std::fmt::Display for UnresolvedLink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let RoomLink {
            from_room,
            from_zone,
            to_room,
            to_zone,
            ..
        } = &self.link;
        match self.end {
            UnresolvedEnd::SourceRoom => write!(
                f,
                "a link to '{to_room}:{to_zone}' starts in unknown room '{from_room}'"
            ),
            UnresolvedEnd::SourceZone => write!(
                f,
                "a link to '{to_room}:{to_zone}' starts in missing zone '{from_room}:{from_zone}'"
            ),
            UnresolvedEnd::TargetRoom => write!(
                f,
                "LoadingZone '{from_room}:{from_zone}' targets unknown room '{to_room}'"
            ),
            UnresolvedEnd::TargetZone => write!(
                f,
                "LoadingZone '{from_room}:{from_zone}' targets missing zone '{to_room}:{to_zone}'"
            ),
        }
    }
}

/// The links whose ends do not resolve in `rooms`, in the order of `links`.
///
/// This is the one judge of "does this link name a room and a zone that
/// exist". A source end is checked before a target end, and a room before its
/// zone, so each link gives one answer for each end.
///
/// An unresolved link is not always an error. A partial set (one room alone,
/// or a world without the rooms that a different provider adds) keeps the
/// exits of its rooms, and [`RoomSet::try_from_parts`] drops them with a
/// warning. A caller that holds the complete game makes each one an error.
pub fn unresolved_links(rooms: &[RoomSpec], links: &[RoomLink]) -> Vec<UnresolvedLink> {
    let room = |id: &str| rooms.iter().find(|room| room.id == id);
    let has_zone =
        |room: &RoomSpec, zone: &str| room.loading_zones.iter().any(|candidate| candidate.id == zone);
    let mut unresolved = Vec::new();
    for link in links {
        let mut report = |end| {
            unresolved.push(UnresolvedLink {
                link: link.clone(),
                end,
            })
        };
        match room(&link.from_room) {
            None => report(UnresolvedEnd::SourceRoom),
            Some(from) if !has_zone(from, &link.from_zone) => report(UnresolvedEnd::SourceZone),
            Some(_) => {}
        }
        match room(&link.to_room) {
            None => report(UnresolvedEnd::TargetRoom),
            Some(to) if !has_zone(to, &link.to_zone) => report(UnresolvedEnd::TargetZone),
            Some(_) => {}
        }
    }
    unresolved
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

/// Which definition of the session's [`RoomSet`] one live room instantiates.
///
/// A component on the live room root, beside its geometry (OW1 cut 5e). It is
/// the one authority for "which room is this": two live rooms of one session
/// can be two different rooms, or two instances of one. Minted only by the
/// set ([`RoomSet::definition`]), so it indexes the set that minted it.
/// Publication writes the root it replaces.
///
/// Rollback state (`root.live_room_definition`): a rewind across a
/// publication returns the root to the room it was.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LiveRoomDefinition(pub(crate) usize);

impl LiveRoomDefinition {
    /// The index of the definition in its [`RoomSet`].
    pub fn index(self) -> usize {
        self.0
    }

    /// Rebuild from an index, for snapshot decode only. It restores a
    /// definition a set already minted; it does not choose one.
    pub fn from_index(index: usize) -> Self {
        Self(index)
    }
}

/// Small room graph for early loading-zone tests.
#[derive(Component, Clone, Debug)]
pub struct RoomSet {
    /// Still public, which limits the invariant below: a caller with
    /// `&mut RoomSet` can shorten this vector under the private indices. No
    /// caller mutates it after construction. Making it private would move 94
    /// read sites.
    pub rooms: Vec<RoomSpec>,
    /// Which room of [`Self::rooms`] a session activates into, read through
    /// [`RoomSet::activation`] and written through [`RoomSet::set_activation`]
    /// / [`RoomSet::set_activation_by_id`].
    ///
    /// ⛔ A PREPARED FACT, NOT A LIVE ONE. Which definition a live room
    /// instantiates is that room's own [`LiveRoomDefinition`], on its root:
    /// two live rooms can instantiate two definitions (OW1 cut 5e). This
    /// field is read only when a session activates, and hot reload
    /// normalizes it to the room it reloads from (it is not `start`).
    ///
    /// Private because it is an invariant, not a free value: it must index
    /// `rooms`. All writes go through the setters, which refuse an
    /// out-of-range index instead of clamping.
    pub(crate) activation: usize,
    /// Index of the room the player starts in on a fresh sandbox, read through
    /// [`RoomSet::start`] and written through [`RoomSet::set_start_by_id`].
    /// Captured at `from_parts` time so the "reset sandbox" flow can
    /// warp the player back without round-tripping through LDtk.
    ///
    /// Private for the same reason as [`Self::activation`]: it must index `rooms`.
    pub(crate) start: usize,
    /// The live room the session's next publication mints (OW1 cut 5a).
    ///
    /// One counter for the session, not one per live room root. A root that
    /// advanced its own instance would mint #1 while another root already is
    /// #1, and two live rooms would share an identity. It only moves forward,
    /// so a retired live room's identity is never reused. Read through
    /// [`RoomSet::next_live_room`], moved by [`RoomSet::mint_live_room`].
    pub(crate) next_live_room: ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance,
    pub(crate) graph: Graph<String, TransitionEdge>,
    pub(crate) room_nodes: Vec<NodeIndex>,
}

/// The definition of THE live room: [`SoleLiveRoom`]'s twin for the room set.
///
/// ⚠ **THE ONE-LIVE-ROOM READ, NAMED AS THE DEBT IT IS.** Its root half is a
/// `Single`, so the system does not run while two rooms are live. That is
/// right for a reader that has not said WHICH room it means. A reader with a
/// subject reads the subject's root's [`LiveRoomDefinition`] and asks
/// [`RoomSet::spec`] (OW1 cut 5e).
///
/// [`SoleLiveRoom`]: ambition_platformer2d_shared_tangle::lifecycle::SoleLiveRoom
#[derive(bevy_ecs::system::SystemParam)]
pub struct SoleLiveRoomSpec<'w, 's> {
    rooms: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<'w, 's, RoomSet>,
    live: ambition_platformer2d_shared_tangle::lifecycle::SoleLiveRoom<'w, 's, LiveRoomDefinition>,
}

impl SoleLiveRoomSpec<'_, '_> {
    /// The live room's definition.
    pub fn spec(&self) -> &RoomSpec {
        self.rooms.spec(**self.live)
    }

    /// Which definition the live room instantiates.
    pub fn definition(&self) -> LiveRoomDefinition {
        **self.live
    }

    /// The session's room set.
    pub fn rooms(&self) -> &RoomSet {
        &self.rooms
    }

    /// Whether the live room became another room, or the set was replaced,
    /// since this system last ran.
    pub fn is_changed(&self) -> bool {
        use bevy_ecs::change_detection::DetectChanges;
        self.live.is_changed() || self.rooms.is_changed()
    }
}

/// The definition of the live room an entity is in: the reader WITH a
/// subject, where [`SoleLiveRoomSpec`] is the reader without one.
///
/// The entity's live room comes from [`LiveRooms::of`]: its
/// `InRoomInstance` stamp, or, for an unstamped entity, the sole live room.
/// The definition is on that room's root. So two live rooms answer for each
/// body, and one live room answers as the sole read did (OW1 cut 6a).
///
/// [`LiveRooms::of`]: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms::of
#[derive(bevy_ecs::system::SystemParam)]
pub struct LiveRoomSpecs<'w, 's> {
    rooms: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<'w, 's, RoomSet>,
    live: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms<'w, 's>,
    roots: bevy_ecs::system::Query<
        'w,
        's,
        (
            &'static ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance,
            &'static LiveRoomDefinition,
        ),
        bevy_ecs::query::With<ambition_platformer2d_shared_tangle::lifecycle::RoomInstanceRoot>,
    >,
}

impl LiveRoomSpecs<'_, '_> {
    /// Which definition the live room `entity` is in instantiates. `None`
    /// when its live room cannot be told or has no definition seated.
    pub fn definition_of(&self, entity: bevy_ecs::entity::Entity) -> Option<LiveRoomDefinition> {
        let room = self.live.of(entity)?;
        self.roots
            .iter()
            .find(|(live, _)| **live == room)
            .map(|(_, definition)| *definition)
    }

    /// The authored spec of the live room `entity` is in. `None` when
    /// [`Self::definition_of`] is `None`.
    pub fn spec_of(&self, entity: bevy_ecs::entity::Entity) -> Option<&RoomSpec> {
        self.definition_of(entity).map(|definition| self.rooms.spec(definition))
    }

    /// The session's room set.
    pub fn rooms(&self) -> &RoomSet {
        &self.rooms
    }

    /// Which live room an entity is in, by the rule every reader here uses.
    pub fn live(&self) -> &ambition_platformer2d_shared_tangle::lifecycle::LiveRooms<'_, '_> {
        &self.live
    }

    /// Every live room and the spec it instantiates.
    pub fn live_specs(&self) -> Vec<(ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance, &RoomSpec)> {
        self.roots
            .iter()
            .map(|(live, definition)| (*live, self.rooms.spec(*definition)))
            .collect()
    }

    /// Which definition live room `room` instantiates. `None` when no live
    /// room is `room`.
    pub fn definition_in(
        &self,
        room: ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance,
    ) -> Option<LiveRoomDefinition> {
        self.roots
            .iter()
            .find(|(live, _)| **live == room)
            .map(|(_, definition)| *definition)
    }

    /// The room a crossing leaves: the live room its subject was recorded in
    /// (OW1 cut 6b). A crossing with no subject, or a subject in no live
    /// room, leaves the sole live room, and there is none when two are live.
    pub fn left_by(
        &self,
        subject: Option<&ambition_platformer2d_shared_tangle::lifecycle::LiveBodyId>,
    ) -> Option<LiveRoomDefinition> {
        self.definition_named(subject.and_then(|subject| subject.room))
    }

    /// Which definition live room `room` instantiates; with no room named,
    /// the sole live room's, and none when two are live.
    pub fn definition_named(
        &self,
        room: Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>,
    ) -> Option<LiveRoomDefinition> {
        match room {
            Some(room) => self.definition_in(room),
            None => self.roots.single().ok().map(|(_, definition)| *definition),
        }
    }

    /// The definition every live room instantiates, one per live room.
    pub fn live_definitions(&self) -> impl Iterator<Item = LiveRoomDefinition> + '_ {
        self.live_rooms().map(|(_, definition)| definition)
    }

    /// Every live room, and the definition it instantiates.
    pub fn live_rooms(
        &self,
    ) -> impl Iterator<Item = (ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance, LiveRoomDefinition)> + '_
    {
        self.roots.iter().map(|(live, definition)| (*live, *definition))
    }

    /// Whether the room set was replaced since this system last ran.
    pub fn is_changed(&self) -> bool {
        use bevy_ecs::change_detection::DetectChanges;
        self.rooms.is_changed()
    }
}

/// The live room of the primary seat and the definition it instantiates: the
/// reader for presentation that has one viewer, such as the map and the
/// developer HUD.
///
/// The room is [`PrimaryLiveRoom`]: the room of the primary body, and with no
/// primary body, the sole live room. So two live rooms answer with the
/// primary body's room, where [`SoleLiveRoomSpec`] answers with none.
///
/// [`PrimaryLiveRoom`]: ambition_platformer2d_shared_tangle::lifecycle::PrimaryLiveRoom
#[derive(bevy_ecs::system::SystemParam)]
pub struct PrimaryLiveRoomSpec<'w, 's> {
    specs: LiveRoomSpecs<'w, 's>,
    primary: ambition_platformer2d_shared_tangle::lifecycle::PrimaryLiveRoom<'w, 's>,
}

impl PrimaryLiveRoomSpec<'_, '_> {
    /// The live room of the primary seat. `None` when no room can be told.
    pub fn room(&self) -> Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance> {
        self.primary.get()
    }

    /// The definition the primary seat's live room instantiates.
    pub fn definition(&self) -> Option<LiveRoomDefinition> {
        self.room().and_then(|room| self.specs.definition_in(room))
    }

    /// The authored spec of the primary seat's live room.
    pub fn spec(&self) -> Option<&RoomSpec> {
        self.definition().map(|definition| self.specs.rooms().spec(definition))
    }

    /// The session's room set.
    pub fn rooms(&self) -> &RoomSet {
        self.specs.rooms()
    }

    /// Whether the room set was replaced since this system last ran. A
    /// crossing of the primary body changes no resource, so a reader that
    /// skips work keeps the room it last showed.
    pub fn is_changed(&self) -> bool {
        self.specs.is_changed()
    }
}

/// [`LiveRoomSpecs::left_by`] at an exclusive-world boundary: the definition
/// of the live room a crossing by `subject` leaves.
pub fn live_room_definition_left_by(
    world: &bevy_ecs::world::World,
    subject: Option<&ambition_platformer2d_shared_tangle::lifecycle::LiveBodyId>,
) -> Option<LiveRoomDefinition> {
    live_room_definition_in(world, subject.and_then(|subject| subject.room))
}

/// Which definition live room `room` instantiates, at an exclusive-world
/// boundary. With no room named, the sole live room's, and none when two
/// are live.
pub fn live_room_definition_in(
    world: &bevy_ecs::world::World,
    room: Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>,
) -> Option<LiveRoomDefinition> {
    let Some(room) = room else {
        return sole_live_room_definition(world);
    };
    let mut roots = world.try_query_filtered::<
        (
            &ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance,
            &LiveRoomDefinition,
        ),
        bevy_ecs::query::With<ambition_platformer2d_shared_tangle::lifecycle::RoomInstanceRoot>,
    >()?;
    roots
        .iter(world)
        .find(|(live, _)| **live == room)
        .map(|(_, definition)| *definition)
}

/// The live room that instantiates the room `id`, at an exclusive-world
/// boundary. A room has at most one live room (`DefinitionAlreadyLive`), so
/// the answer is unique. `None` when no live room stands in `id`.
///
/// The keyed read for a question about a named room: with one live room it is
/// [`sole_live_room_definition`]'s answer when that room is `id`, and with two
/// it still answers.
pub fn live_room_standing_in(
    world: &bevy_ecs::world::World,
    id: &str,
) -> Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance> {
    let definition = ambition_platformer2d_shared_tangle::lifecycle::session_world_component::<RoomSet>(world)?
        .definition_by_id(id)?;
    let mut roots = world.try_query_filtered::<
        (
            &ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance,
            &LiveRoomDefinition,
        ),
        bevy_ecs::query::With<ambition_platformer2d_shared_tangle::lifecycle::RoomInstanceRoot>,
    >()?;
    roots
        .iter(world)
        .find(|(_, live)| **live == definition)
        .map(|(instance, _)| *instance)
}

/// The ids of the rooms every live room instantiates, in instance order, at an
/// exclusive-world boundary.
pub fn live_room_ids(world: &bevy_ecs::world::World) -> Vec<String> {
    let Some(rooms) = ambition_platformer2d_shared_tangle::lifecycle::session_world_component::<RoomSet>(world)
    else {
        return Vec::new();
    };
    let Some(mut roots) = world.try_query_filtered::<
        (
            &ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance,
            &LiveRoomDefinition,
        ),
        bevy_ecs::query::With<ambition_platformer2d_shared_tangle::lifecycle::RoomInstanceRoot>,
    >() else {
        return Vec::new();
    };
    let mut live: Vec<_> = roots.iter(world).map(|(instance, definition)| (*instance, *definition)).collect();
    live.sort_by_key(|(instance, _)| *instance);
    live.into_iter()
        .map(|(_, definition)| rooms.spec(definition).id.clone())
        .collect()
}

/// The sole live room's definition, at an exclusive-world boundary. `None`
/// with no live session, no live room, or two live rooms. The same debt as
/// [`SoleLiveRoomSpec`].
pub fn sole_live_room_definition(world: &bevy_ecs::world::World) -> Option<LiveRoomDefinition> {
    ambition_platformer2d_shared_tangle::lifecycle::sole_live_room_component::<LiveRoomDefinition>(
        world,
    )
    .copied()
}

/// The spec of the live room `entity` is in, at an exclusive-world boundary:
/// the room of its `InRoomInstance` stamp. An unstamped entity is in the
/// sole live room, and in none when two are live.
pub fn live_room_spec_of(world: &bevy_ecs::world::World, entity: bevy_ecs::entity::Entity) -> Option<&RoomSpec> {
    let room = world
        .get::<ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance>(entity)
        .map(|stamp| stamp.0);
    let definition = live_room_definition_in(world, room)?;
    ambition_platformer2d_shared_tangle::lifecycle::session_world_component::<RoomSet>(world)
        .map(|rooms| rooms.spec(definition))
}

/// The sole live room's spec, at an exclusive-world boundary. See
/// [`sole_live_room_definition`].
pub fn sole_live_room_spec(world: &bevy_ecs::world::World) -> Option<&RoomSpec> {
    let definition = sole_live_room_definition(world)?;
    ambition_platformer2d_shared_tangle::lifecycle::session_world_component::<RoomSet>(world)
        .map(|rooms| rooms.spec(definition))
}

/// Put `rooms` on the session root and seat its activation room as the
/// definition of the sole live room, creating either root when absent. The
/// one road for a direct host or a fixture: a set with no live room seated
/// answers no "which room is this" question.
pub fn insert_room_set(world: &mut bevy_ecs::world::World, rooms: RoomSet) -> bevy_ecs::entity::Entity {
    let definition = rooms.activation_definition();
    let root = ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(world, rooms);
    ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(world, definition);
    root
}

/// Seat the sole live room as the session's room with this authored id, for
/// a direct host or a fixture. `None` when the session has no room set or no
/// room with that id; nothing is written then.
pub fn seat_sole_live_room_by_id(
    world: &mut bevy_ecs::world::World,
    id: &str,
) -> Option<LiveRoomDefinition> {
    let definition =
        ambition_platformer2d_shared_tangle::lifecycle::session_world_component::<RoomSet>(world)?
            .definition_by_id(id)?;
    ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(world, definition);
    Some(definition)
}

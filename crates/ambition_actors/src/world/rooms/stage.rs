//! Canonical prepared room construction.
//!
//! Every room lifecycle path prepares the same mutation-free
//! [`RoomConstructionPlan`] before it retires live entities. The plan freezes
//! target identity, authored geometry, resolved placement interpreters,
//! content-staged actor requests, catalogs, moving-platform starts, and the
//! expected authoritative roster. Startup, reset, ordinary transition, LDtk
//! hot reload, and snapshot reconstruction execute this one artifact.
//!
//! The file retains its historical `stage.rs` path, but there is no longer a
//! restore-only staging API. Snapshot reconstruction is a use of canonical construction,
//! not a second construction authority.

use std::collections::BTreeSet;
use std::hash::{Hash, Hasher};

use bevy::ecs::entity::Entity;
use bevy::ecs::query::With;
use bevy::ecs::world::World;
use bevy::prelude::{Commands, Resource};

use super::{transaction, RespawnRoomVisualsRequested, RoomSet, RoomSpec};
use crate::features::{self, RoomFeatureConstructionPlan};
use crate::platformer_runtime::lifecycle::RoomScopedEntity;
use crate::world::physics::{self, PhysicsRoomEntity};
use crate::world::placements::PlacementLoweringRegistry;
use crate::world::platforms::{self, MovingPlatformState};
use ambition_platformer_primitives::lifecycle::{
    session_world_component, session_world_component_mut, ActiveSessionScope, SessionSpawnScope,
};

/// Stable same-build identity for one prepared construction artifact.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct RoomConstructionPlanId(String);

impl RoomConstructionPlanId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Why canonical room construction could not be prepared. Every variant is
/// detected before any live-room mutation.
#[derive(Clone, Debug, PartialEq)]
pub enum RoomConstructionError {
    UnknownRoom {
        room: String,
    },
    MissingService {
        service: &'static str,
    },
    InvalidFeatures {
        room: String,
        reason: features::RoomFeatureConstructionError,
    },
}

impl std::fmt::Display for RoomConstructionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownRoom { room } => {
                write!(f, "no room named `{room}` in the prepared RoomSet")
            }
            Self::MissingService { service } => write!(
                f,
                "room construction needs `{service}`, which this world does not provide"
            ),
            Self::InvalidFeatures { room, reason } => {
                write!(f, "room `{room}` construction is invalid: {reason}")
            }
        }
    }
}

impl std::error::Error for RoomConstructionError {}

/// Last successfully scheduled room-construction commit.
///
/// This is developer evidence, not simulation authority: the active RoomSet and
/// spawned ECS entities remain authoritative. It lets diagnostics and tests join
/// a committed room to the exact immutable plan and root roster that produced it.
#[derive(Resource, Clone, Debug, PartialEq, Eq)]
pub struct LastRoomConstructionCommit {
    pub plan_id: RoomConstructionPlanId,
    pub room_id: String,
    pub authoritative_ids: BTreeSet<String>,
    pub moving_platform_count: usize,
}

/// The one prepared artifact for a room's authoritative simulation contents.
#[derive(Clone)]
pub struct RoomConstructionPlan {
    id: RoomConstructionPlanId,
    target_index: usize,
    features: RoomFeatureConstructionPlan,
    platform_states: Vec<MovingPlatformState>,
    session_scope: SessionSpawnScope,
}

impl std::fmt::Debug for RoomConstructionPlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RoomConstructionPlan")
            .field("id", &self.id)
            .field("target_index", &self.target_index)
            .field("room", &self.features.room().id)
            .field(
                "expected_authoritative_ids",
                self.features.expected_authoritative_ids(),
            )
            .field("platform_count", &self.platform_states.len())
            .field("session_scope", &self.session_scope)
            .finish()
    }
}

impl RoomConstructionPlan {
    /// Prepare from the canonical resources in an exclusive `World`.
    pub fn prepare(world: &World, target_room_id: &str) -> Result<Self, RoomConstructionError> {
        let missing = |service| RoomConstructionError::MissingService { service };
        let rooms = session_world_component::<RoomSet>(world).ok_or(missing("session RoomSet"))?;
        let target_index = rooms.room_index_by_id(target_room_id).ok_or_else(|| {
            RoomConstructionError::UnknownRoom {
                room: target_room_id.to_string(),
            }
        })?;
        session_world_component::<ambition_engine_core::RoomGeometry>(world)
            .ok_or(missing("session RoomGeometry"))?;
        if world
            .get_resource::<ambition_world::collision::MovingPlatformSet>()
            .is_none()
        {
            return Err(missing("MovingPlatformSet"));
        }
        let session_scope = SessionSpawnScope::for_optional_active_session(
            world.get_resource::<ActiveSessionScope>(),
        )
        .ok_or(missing("ActiveSessionScope"))?;
        let content_staging = world
            .get_resource::<features::RoomContentStagingRegistry>()
            .cloned()
            .unwrap_or_default();
        Self::prepare_from_parts(
            rooms,
            target_index,
            world
                .get_resource::<PlacementLoweringRegistry>()
                .ok_or(missing("PlacementLoweringRegistry"))?,
            &content_staging,
            world
                .get_resource::<ambition_characters::actor::character_catalog::CharacterCatalog>()
                .ok_or(missing("CharacterCatalog"))?,
            world
                .get_resource::<features::CharacterRoster>()
                .ok_or(missing("CharacterRoster"))?,
            world
                .get_resource::<crate::boss_encounter::BossCatalog>()
                .ok_or(missing("BossCatalog"))?,
            session_scope,
            features::ActorConstructionContext::new(
                world
                    .get_resource::<crate::construction::ActorConstructionRegistry>()
                    .ok_or(missing("ActorConstructionRegistry"))?,
                // The activation generation this world is running, published on
                // the session root beside the prepared content it identifies.
                // A world with no prepared session states none.
                session_world_component::<ambition_engine_core::ContentEpoch>(world)
                    .copied()
                    .unwrap_or_default(),
            ),
        )
    }

    /// Prepare from already-borrowed services. This is the system-facing seam
    /// used by activation, reset, transition, and hot reload.
    #[allow(clippy::too_many_arguments)]
    pub fn prepare_from_parts(
        rooms: &RoomSet,
        target_index: usize,
        placement_lowering: &PlacementLoweringRegistry,
        content_staging: &features::RoomContentStagingRegistry,
        character_catalog: &ambition_characters::actor::character_catalog::CharacterCatalog,
        character_roster: &features::CharacterRoster,
        boss_catalog: &crate::boss_encounter::BossCatalog,
        session_scope: SessionSpawnScope,
        construction: features::ActorConstructionContext<'_>,
    ) -> Result<Self, RoomConstructionError> {
        let spec = rooms.rooms.get(target_index).cloned().ok_or_else(|| {
            RoomConstructionError::UnknownRoom {
                room: format!("<room-index-{target_index}>"),
            }
        })?;
        Self::prepare_spec(
            target_index,
            spec,
            placement_lowering,
            content_staging,
            character_catalog,
            character_roster,
            boss_catalog,
            session_scope,
            construction,
        )
    }

    /// Prepare a room whose containing `RoomSet` is itself a candidate artifact,
    /// as in transactional LDtk hot reload.
    #[allow(clippy::too_many_arguments)]
    pub fn prepare_spec(
        target_index: usize,
        spec: RoomSpec,
        placement_lowering: &PlacementLoweringRegistry,
        content_staging: &features::RoomContentStagingRegistry,
        character_catalog: &ambition_characters::actor::character_catalog::CharacterCatalog,
        character_roster: &features::CharacterRoster,
        boss_catalog: &crate::boss_encounter::BossCatalog,
        session_scope: SessionSpawnScope,
        construction: features::ActorConstructionContext<'_>,
    ) -> Result<Self, RoomConstructionError> {
        let feature_plan = RoomFeatureConstructionPlan::prepare(
            &spec,
            placement_lowering,
            content_staging,
            character_catalog,
            character_roster,
            boss_catalog,
            construction,
        )
        .map_err(|reason| RoomConstructionError::InvalidFeatures {
            room: spec.id.clone(),
            reason,
        })?;
        let platform_states = platforms::moving_platforms_for_room(&spec);
        let id = construction_plan_id(&spec, feature_plan.construction());
        Ok(Self {
            id,
            target_index,
            features: feature_plan,
            platform_states,
            session_scope,
        })
    }

    pub fn id(&self) -> &RoomConstructionPlanId {
        &self.id
    }

    pub fn target_index(&self) -> usize {
        self.target_index
    }

    pub fn room_id(&self) -> &str {
        &self.features.room().id
    }

    pub fn spec(&self) -> &RoomSpec {
        self.features.room()
    }

    /// Whether a currently installed room definition is byte-for-byte the
    /// authored spec this plan prepared. This rejects a same-id hot reload from
    /// committing a stale in-flight transition.
    pub fn matches_room_spec(&self, candidate: &RoomSpec) -> bool {
        let prepared =
            serde_json::to_vec(self.spec()).expect("prepared RoomSpec must remain serializable");
        let current =
            serde_json::to_vec(candidate).expect("candidate RoomSpec must remain serializable");
        prepared == current
    }

    pub fn platform_states(&self) -> &[MovingPlatformState] {
        &self.platform_states
    }

    pub fn predicted_authoritative_ids(&self) -> &BTreeSet<String> {
        self.features.expected_authoritative_ids()
    }

    pub fn content_staged_names(&self) -> Vec<String> {
        self.features.content_staged_names()
    }

    /// Rebuild one authored authoritative root through this plan's frozen
    /// interpreter/catalog decisions.
    pub fn respawn_authoritative_entity(&self, commands: &mut Commands, authored_id: &str) -> bool {
        self.features
            .respawn_authoritative_entity(commands, self.session_scope, authored_id)
    }

    /// Rebuild one PLANNED root by its stable identity — the only form that can
    /// name a derived row like a giant's hand (`SimId::spawned`), which no
    /// authored-id spelling reaches.
    pub fn respawn_authoritative_sim_id(
        &self,
        commands: &mut Commands,
        sim_id: &ambition_platformer_primitives::sim_id::SimId,
    ) -> bool {
        self.features
            .respawn_authoritative_sim_id(commands, self.session_scope, sim_id)
    }

    pub fn session_scope(&self) -> SessionSpawnScope {
        self.session_scope
    }

    /// Enqueue the prepared room contents without changing active-room
    /// resources. Session startup uses this after those resources are installed.
    ///
    /// **This is the room transaction boundary.** Everything the room is made of
    /// is queued between [`transaction::open`] and [`transaction::close`], so
    /// the verification that publishes `RoomLoaded` runs after ALL of it: the
    /// feature families, the planned roots, the planned relationships, the
    /// moving-platform bodies, and the last-commit receipt. Active room
    /// selection, room geometry, moving-platform resource state, and carried-
    /// player handling are applied by every caller before this is reached, so
    /// they precede publication too.
    ///
    /// The bracket sits HERE rather than inside the feature plan because the
    /// feature plan does not know when the room is complete — it is one
    /// participant. When it owned the bracket, the platform bodies and the
    /// commit receipt below were queued after its verification had already run
    /// and published, so `RoomLoaded` described a room that was still being
    /// built.
    pub fn spawn_contents(&self, commands: &mut Commands) {
        transaction::open(commands);
        let receipt = features::spawn_room_feature_entities_from_plan(
            commands,
            &self.features,
            self.session_scope,
        );
        debug_assert_eq!(
            receipt.authoritative_ids(),
            self.predicted_authoritative_ids(),
            "room construction execution diverged from its prepared root roster",
        );
        platforms::spawn_moving_platforms(
            commands,
            self.session_scope,
            &self.spec().world,
            &self.platform_states,
        );
        commands.insert_resource(LastRoomConstructionCommit {
            plan_id: self.id.clone(),
            room_id: self.room_id().to_string(),
            authoritative_ids: receipt.authoritative_ids().clone(),
            moving_platform_count: self.platform_states.len(),
        });
        transaction::close(
            commands,
            self.features.construction(),
            receipt.construction(),
            self.room_id().to_string(),
            self.session_scope,
        );
    }

    /// Retire the outgoing room's scoped entities. The transiting possessed
    /// body may be carried across the boundary instead of being retired.
    pub fn retire_outgoing<'a>(
        &self,
        commands: &mut Commands,
        outgoing: impl IntoIterator<Item = (Entity, bool)> + 'a,
        carry_body: Option<Entity>,
    ) {
        for (entity, is_physics) in outgoing {
            if carry_body == Some(entity) {
                continue;
            }
            if is_physics {
                physics::retire_physics_entity(commands, entity);
            } else {
                commands.entity(entity).despawn();
            }
        }
    }

    /// Publish target geometry/platform state and enqueue the exact frozen room
    /// contents. Call only after every preflight has succeeded.
    pub fn commit_deferred(
        &self,
        commands: &mut Commands,
        rooms: &mut RoomSet,
        geometry: &mut ambition_engine_core::RoomGeometry,
        moving_platforms: &mut Vec<MovingPlatformState>,
    ) {
        rooms.set_active(self.target_index);
        geometry.0 = self.spec().world.clone();
        *moving_platforms = self.platform_states.clone();
        self.spawn_contents(commands);
    }

    /// Exclusive-world execution for snapshot reconstruction. Preparation is
    /// mutation-free; after it returns, this application has no fallible lookup.
    ///
    /// `carry_body` is the transiting controlled body (a possessed room-scoped
    /// actor) that must ride along into the target room rather than be despawned
    /// with the old room scope — the same exemption
    /// `commit_room_transition_geometry` gives it. `None` for the ordinary
    /// primary player, which is not room-scoped and so is never in `outgoing`.
    pub fn apply_to_world(self, world: &mut World, carry_body: Option<Entity>) {
        if let Some(mut pending) =
            world.get_resource_mut::<bevy::ecs::message::Messages<features::SpawnActorRequest>>()
        {
            pending.clear();
        }

        let outgoing: Vec<(Entity, bool)> = match world
            .try_query_filtered::<(Entity, Option<&PhysicsRoomEntity>), With<RoomScopedEntity>>()
        {
            Some(mut query) => query
                .iter(world)
                .map(|(entity, physics)| (entity, physics.is_some()))
                .collect(),
            None => Vec::new(),
        };
        {
            let mut commands = world.commands();
            self.retire_outgoing(&mut commands, outgoing, carry_body);
        }
        world.flush();

        if let Some(mut rooms) = session_world_component_mut::<RoomSet>(world) {
            rooms.set_active(self.target_index);
        }
        if let Some(mut geometry) =
            session_world_component_mut::<ambition_engine_core::RoomGeometry>(world)
        {
            geometry.0 = self.spec().world.clone();
        }
        if let Some(mut set) =
            world.get_resource_mut::<ambition_world::collision::MovingPlatformSet>()
        {
            set.0 = self.platform_states.clone();
        }
        {
            let mut commands = world.commands();
            self.spawn_contents(&mut commands);
        }
        world.flush();
        let _ = bevy::ecs::system::RunSystemOnce::run_system_once(
            &mut *world,
            features::apply_spawn_actor_requests,
        );
        world.flush();
        if let Some(mut messages) =
            world.get_resource_mut::<bevy::ecs::message::Messages<RespawnRoomVisualsRequested>>()
        {
            messages.write(RespawnRoomVisualsRequested);
        }
    }
}

/// Identity of one prepared room-construction artifact, from EVERY frozen
/// world-defining preparation product — not just the authored source.
///
/// The previous form hashed the `RoomSpec` plus a hand-built id set, which made
/// two materially different prepared worlds collide: giant hand rows, limb
/// relation payloads (slots, home offsets), recipe identities, and the content
/// epoch are all derived from the character roster and registry — data OUTSIDE
/// the `RoomSpec` — so a roster change that moved a hand's slot produced a
/// different world under the SAME plan id. `deterministic_dump()` is the
/// canonical rendering of exactly that derived surface (schema version, content
/// binding/epoch, every plan row with recipe + origin + parameter summary, every
/// relation with its canonical payload), so folding it in makes the id a function
/// of the complete frozen plan.
///
/// Moving platforms and kinematic paths are pure functions of the spec, so the
/// spec JSON already covers them. Deliberately EXCLUDED: `SessionSpawnScope` /
/// `TransactionId` (commit-time, not frozen-plan), `Entity` values, and anything
/// process-local. `DefaultHasher::new()` uses fixed keys, so the id is stable
/// across runs and replays.
fn construction_plan_id(
    spec: &RoomSpec,
    construction: &crate::construction::ActorConstructionPlan,
) -> RoomConstructionPlanId {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    spec.id.hash(&mut hasher);
    // RoomSpec is the canonical authored room artifact; JSON avoids depending
    // on map insertion order because its fields are vectors/ordered values.
    serde_json::to_vec(spec)
        .expect("RoomSpec serialization must succeed for construction identity")
        .hash(&mut hasher);
    construction.deterministic_dump().hash(&mut hasher);
    RoomConstructionPlanId(format!("room-plan:{:016x}", hasher.finish()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_engine_core as ae;

    fn empty_spec(id: &str) -> RoomSpec {
        RoomSpec::new(
            id,
            ae::World::new(
                id,
                ae::Vec2::new(640.0, 480.0),
                ae::Vec2::new(96.0, 96.0),
                Vec::new(),
            ),
        )
    }

    fn prepare(spec: RoomSpec) -> Result<RoomConstructionPlan, RoomConstructionError> {
        let recipes = crate::construction::engine_construction_registry();
        RoomConstructionPlan::prepare_spec(
            0,
            spec,
            &PlacementLoweringRegistry::default(),
            &features::RoomContentStagingRegistry::default(),
            &ambition_characters::actor::character_catalog::CharacterCatalog::empty(),
            &features::CharacterRoster::default(),
            &crate::boss_encounter::BossCatalog::default(),
            SessionSpawnScope::UNSCOPED,
            features::ActorConstructionContext::new(&recipes, Default::default()),
        )
    }

    #[test]
    fn equivalent_room_construction_has_stable_identity() {
        let a = prepare(empty_spec("same")).expect("first plan");
        let b = prepare(empty_spec("same")).expect("second plan");
        assert_eq!(a.id(), b.id());
        assert_eq!(
            a.predicted_authoritative_ids(),
            b.predicted_authoritative_ids()
        );
    }

    /// As [`prepare`], but with an explicit roster and content epoch — the two
    /// preparation inputs OUTSIDE the `RoomSpec` that shape the derived plan.
    fn prepare_with(
        spec: RoomSpec,
        roster: &features::CharacterRoster,
        epoch: ae::ContentEpoch,
    ) -> Result<RoomConstructionPlan, RoomConstructionError> {
        let recipes = crate::construction::engine_construction_registry();
        RoomConstructionPlan::prepare_spec(
            0,
            spec,
            &PlacementLoweringRegistry::default(),
            &features::RoomContentStagingRegistry::default(),
            &ambition_characters::actor::character_catalog::CharacterCatalog::empty(),
            roster,
            &crate::boss_encounter::BossCatalog::default(),
            SessionSpawnScope::UNSCOPED,
            features::ActorConstructionContext::new(&recipes, epoch),
        )
    }

    /// A minimal roster whose `"giant_gnu"` is a `"giant"`-class limbed host
    /// with the given body size. The size drives `giant_hand_plans` geometry —
    /// hand boxes and `home_offset` relation payloads — which lives NOWHERE in
    /// the `RoomSpec`.
    fn giant_roster(default_size: f32) -> features::CharacterRoster {
        features::CharacterRoster::from_ron(&format!(
            r#"{{
                "combatant": (
                    max_health: 2, patrol_speed: 0.0, chase_speed: 0.0,
                    aggro_radius: 0.0, attack_range: 0.0, contact_strength: 0.0,
                    damage_amount: 0, brain_template: StandStill, move_style: Walk,
                ),
                "giant_gnu": (
                    max_health: 42, patrol_speed: 0.0, chase_speed: 0.0,
                    aggro_radius: 0.0, attack_range: 0.0, contact_strength: 0.0,
                    damage_amount: 0, brain_template: StandStill, move_style: Walk,
                    mount_class: Some("giant"),
                    default_size: Some(({default_size}, {default_size})),
                ),
                "giant_gnu_hands": (
                    max_health: 42, patrol_speed: 0.0, chase_speed: 0.0,
                    aggro_radius: 0.0, attack_range: 0.0, contact_strength: 0.0,
                    damage_amount: 0, brain_template: StandStill, move_style: Walk,
                ),
            }}"#
        ))
    }

    fn giant_spec(id: &str) -> RoomSpec {
        let mut spec = empty_spec(id);
        spec.enemy_spawns.push(crate::rooms::Authored::new(
            "gnu",
            "Giant GNU",
            ae::Aabb::new(ae::Vec2::new(100.0, 100.0), ae::Vec2::splat(60.0)),
            ambition_entity_catalog::placements::CharacterBrain::Custom("giant_gnu".into()),
        ));
        spec
    }

    /// **The plan id tracks the DERIVED construction surface, not just the
    /// authored spec.** Two rosters that differ only in the giant's body size
    /// produce byte-identical `RoomSpec`s but different hand `home_offset`
    /// relation payloads — materially different prepared worlds. The previous id
    /// (spec JSON + authored id set) collided them.
    #[test]
    fn the_plan_id_tracks_the_derived_relation_payloads() {
        let small = prepare_with(
            giant_spec("arena"),
            &giant_roster(120.0),
            ae::ContentEpoch(4),
        )
        .expect("small-giant plan");
        let large = prepare_with(
            giant_spec("arena"),
            &giant_roster(140.0),
            ae::ContentEpoch(4),
        )
        .expect("large-giant plan");
        assert_ne!(
            small.id(),
            large.id(),
            "different hand offsets are different prepared worlds"
        );
    }

    /// The id also tracks the giant-vs-ordinary shape of the plan itself: the
    /// same spec whose brain key stops resolving as a `"giant"`-class host loses
    /// its host/hand rows AND their relations.
    #[test]
    fn the_plan_id_tracks_the_giant_expansion() {
        let giant = prepare_with(
            giant_spec("arena"),
            &giant_roster(120.0),
            ae::ContentEpoch(4),
        )
        .expect("giant plan");
        // Same spec, but the roster has no idea "giant_gnu" is a giant.
        let plain = prepare_with(
            giant_spec("arena"),
            &features::CharacterRoster::default(),
            ae::ContentEpoch(4),
        )
        .expect("plain plan");
        assert_ne!(giant.id(), plain.id());
    }

    /// The id tracks the prepared-content epoch: the same room prepared against
    /// re-prepared content is a different transaction target.
    #[test]
    fn the_plan_id_tracks_the_content_epoch() {
        let four = prepare_with(
            giant_spec("arena"),
            &giant_roster(120.0),
            ae::ContentEpoch(4),
        )
        .expect("epoch-4 plan");
        let five = prepare_with(
            giant_spec("arena"),
            &giant_roster(120.0),
            ae::ContentEpoch(5),
        )
        .expect("epoch-5 plan");
        assert_ne!(four.id(), five.id());
    }

    /// Frozen room path content reaches the id (through the spec AND through the
    /// giant host row that now carries the paths).
    #[test]
    fn the_plan_id_tracks_frozen_path_content() {
        let bare = prepare_with(
            giant_spec("arena"),
            &giant_roster(120.0),
            ae::ContentEpoch(4),
        )
        .expect("pathless plan");
        let mut with_path = giant_spec("arena");
        with_path
            .kinematic_paths
            .push(ambition_world::rooms::KinematicPathSpec::new(
                "patrol",
                "patrol",
                ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::splat(8.0)),
                ae::KinematicPath::line(ae::Vec2::ZERO, ae::Vec2::new(64.0, 0.0), 24.0),
            ));
        let pathed = prepare_with(with_path, &giant_roster(120.0), ae::ContentEpoch(4))
            .expect("pathed plan");
        assert_ne!(bare.id(), pathed.id());
    }

    /// **One giant, every roster surface, one answer.** The prepared plan, the
    /// predicted outer roster, the commit receipt, and the boundary verifier all
    /// name the same three-cluster — and the hands are welcome plan rows, not
    /// unexpected or legacy findings.
    #[test]
    fn a_giant_rooms_rosters_agree_from_plan_to_receipt_to_verifier() {
        let plan = prepare_with(
            giant_spec("arena"),
            &giant_roster(120.0),
            ae::ContentEpoch(4),
        )
        .expect("giant plan");

        let host = ambition_platformer_primitives::sim_id::SimId::placement("gnu");
        let cluster: BTreeSet<String> = [
            host.to_string(),
            ambition_platformer_primitives::sim_id::SimId::spawned(&host, 0).to_string(),
            ambition_platformer_primitives::sim_id::SimId::spawned(&host, 1).to_string(),
        ]
        .into();

        // Prepared plan: three giant-cluster identities.
        let planned: BTreeSet<String> = plan
            .features
            .construction()
            .planned_ids()
            .iter()
            .map(std::string::ToString::to_string)
            .collect();
        assert_eq!(planned, cluster, "host + two hands are the plan rows");
        // Predicted outer roster: the same three (nothing else in this room).
        assert_eq!(plan.predicted_authoritative_ids(), &cluster);

        let expected_plan_id = plan.id().clone();
        let mut app = bevy::prelude::App::new();
        app.add_message::<crate::rooms::RoomLoaded>();
        {
            let mut commands = app.world_mut().commands();
            plan.spawn_contents(&mut commands);
        }
        app.world_mut().flush();

        // Commit receipt: the same three.
        let commit = app.world().resource::<LastRoomConstructionCommit>();
        assert_eq!(commit.plan_id, expected_plan_id);
        assert_eq!(commit.authoritative_ids, cluster);

        // Boundary verifier: published, and NOTHING flagged — a hand read as
        // unexpected or legacy would appear here.
        let verification = app
            .world()
            .resource::<crate::world::rooms::LastConstructionVerification>();
        assert!(
            verification.published,
            "the giant room publishes: {:?}",
            verification.violations
        );
        assert_eq!(
            verification.violations,
            Vec::new(),
            "no hand is unexpected, legacy, or malformed"
        );
    }

    #[test]
    fn duplicate_authoritative_roots_fail_before_commit() {
        let mut spec = empty_spec("duplicate");
        let aabb = ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::splat(16.0));
        spec.enemy_spawns.push(crate::rooms::Authored::new(
            "same-id",
            "first",
            aabb,
            ambition_entity_catalog::placements::CharacterBrain::Custom("combatant".into()),
        ));
        spec.enemy_spawns.push(crate::rooms::Authored::new(
            "same-id",
            "second",
            aabb,
            ambition_entity_catalog::placements::CharacterBrain::Custom("combatant".into()),
        ));
        let error = prepare(spec).expect_err("duplicate roots must fail preparation");
        assert!(matches!(
            error,
            RoomConstructionError::InvalidFeatures {
                reason: features::RoomFeatureConstructionError::DuplicateAuthoritativeId { .. },
                ..
            }
        ));
    }

    /// **`RoomLoaded` is published only after the WHOLE room is applied.**
    ///
    /// The transaction boundary is `spawn_contents`, not the feature plan.
    /// Command queues apply in insertion order, so the verify-and-publish queued
    /// at the tail of `spawn_contents` runs after the moving-platform bodies and
    /// the last-commit receipt at its middle — the ordering that failed when the
    /// feature plan owned the bracket, publishing before its caller had queued
    /// the platforms. An observer reads the world the instant `RoomLoaded` is
    /// delivered and proves the platforms, the commit receipt, and the
    /// authoritative bodies are already present.
    #[test]
    fn room_loaded_observes_a_fully_committed_room() {
        let mut spec = empty_spec("published");
        spec.moving_platforms
            .push(MovingPlatformState::from_authored(
                ae::Vec2::new(0.0, 200.0),
                ae::Vec2::new(96.0, 16.0),
                120.0,
                60.0,
            ));
        // A CONTENT-STAGED actor, so it is a plan row: the executor stamps its
        // `SimId` during construction, which is what lets an observer at
        // publication time see it. An `enemy_spawn` gets its id from
        // `ensure_sim_id` in a later system that this minimal app does not run.
        let mut staging = features::RoomContentStagingRegistry::default();
        staging
            .register("published", "test_provider", "occ", "occ.v1", |_room| {
                vec![features::SpawnActorRequest {
                    id: "occupant".into(),
                    name: "occupant".into(),
                    pos: ae::Vec2::ZERO,
                    half_size: ae::Vec2::splat(10.0),
                    faction: features::ActorFaction::Npc,
                    grudge_against: None,
                    kind: features::SpawnActorKind::Enemy {
                        brain: ambition_entity_catalog::placements::CharacterBrain::Custom(
                            "combatant".into(),
                        ),
                    },
                }]
            })
            .expect("stager registers");
        let recipes = crate::construction::engine_construction_registry();
        let plan = RoomConstructionPlan::prepare_spec(
            0,
            spec,
            &PlacementLoweringRegistry::default(),
            &staging,
            &ambition_characters::actor::character_catalog::CharacterCatalog::empty(),
            &crate::features::enemies::test_roster(),
            &crate::boss_encounter::BossCatalog::default(),
            SessionSpawnScope::UNSCOPED,
            features::ActorConstructionContext::new(&recipes, Default::default()),
        )
        .expect("plan");

        let mut app = bevy::prelude::App::new();
        app.add_message::<crate::rooms::RoomLoaded>();
        app.add_message::<features::SpawnActorRequest>();

        let observed = std::sync::Arc::new(std::sync::Mutex::new(None));
        let sink = observed.clone();
        app.add_systems(
            bevy::prelude::Update,
            move |mut reader: bevy::ecs::message::MessageReader<crate::rooms::RoomLoaded>,
                  commit: Option<bevy::prelude::Res<LastRoomConstructionCommit>>,
                  platforms: bevy::prelude::Query<
                &crate::world::platforms::MovingPlatformVisual,
            >,
                  ids: bevy::prelude::Query<
                &ambition_platformer_primitives::sim_id::SimId,
            >| {
                if reader.read().next().is_some() {
                    *sink.lock().unwrap() = Some((
                        commit.map(|c| c.moving_platform_count),
                        platforms.iter().count(),
                        ids.iter().any(|id| id.as_str() == "placement:occupant"),
                    ));
                }
            },
        );

        {
            let mut commands = app.world_mut().commands();
            plan.spawn_contents(&mut commands);
        }
        app.update();

        let (commit_platforms, platform_bodies, saw_occupant) = observed
            .lock()
            .unwrap()
            .expect("RoomLoaded must have published for a valid room");
        assert_eq!(
            commit_platforms,
            Some(1),
            "the last-commit receipt existed before RoomLoaded"
        );
        assert_eq!(
            platform_bodies, 1,
            "the moving-platform body was spawned before RoomLoaded"
        );
        assert!(
            saw_occupant,
            "the authoritative occupant existed before RoomLoaded"
        );
    }

    #[test]
    fn commit_receipt_matches_the_prepared_root_roster() {
        let mut spec = empty_spec("receipt");
        spec.enemy_spawns.push(crate::rooms::Authored::new(
            "enemy-1",
            "enemy",
            ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::splat(16.0)),
            ambition_entity_catalog::placements::CharacterBrain::Custom("combatant".into()),
        ));
        let plan = prepare(spec).expect("plan");
        let expected = plan.predicted_authoritative_ids().clone();
        let expected_id = plan.id().clone();

        let mut app = bevy::prelude::App::new();
        app.add_message::<crate::rooms::RoomLoaded>();
        app.add_message::<features::SpawnActorRequest>();
        {
            let mut commands = app.world_mut().commands();
            plan.spawn_contents(&mut commands);
        }
        app.world_mut().flush();

        let receipt = app.world().resource::<LastRoomConstructionCommit>();
        assert_eq!(receipt.plan_id, expected_id);
        assert_eq!(receipt.room_id, "receipt");
        assert_eq!(receipt.authoritative_ids, expected);

        // The roster speaks the `SimId` namespace now (it is derived from the
        // construction plan, whose derived rows have no authored spelling). A
        // family-loop enemy's body only receives its `SimId` from `ensure_sim_id`
        // AFTER verification, so map its authored `FeatureId` through the same
        // `placement:` spelling the roster uses for authored roots.
        let actual = {
            let mut query = app
                .world_mut()
                .query::<&ambition_combat::components::FeatureId>();
            query
                .iter(app.world())
                .map(|feature| {
                    ambition_platformer_primitives::sim_id::SimId::placement(&feature.0).to_string()
                })
                .collect::<BTreeSet<_>>()
        };
        assert_eq!(
            actual, expected,
            "the committed authoritative roots must match the prepared roster",
        );
    }

    #[test]
    fn plan_rejects_a_same_id_room_spec_changed_after_preparation() {
        let plan = prepare(empty_spec("mutable")).expect("plan");
        let mut changed = empty_spec("mutable");
        changed.world.spawn.x += 1.0;
        assert!(plan.matches_room_spec(plan.spec()));
        assert!(!plan.matches_room_spec(&changed));
    }
}

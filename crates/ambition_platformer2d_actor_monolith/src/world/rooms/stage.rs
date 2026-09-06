//! Canonical prepared room construction.
//!
//! Every room lifecycle path prepares the same mutation-free
//! [`RoomConstructionPlan`] before it retires live entities. The plan freezes
//! target identity, authored geometry, resolved placement interpreters,
//! content-staged actor requests, catalogs, moving-platform starts, and the
//! expected authoritative roster. Startup, reset, ordinary transition, LDtk
//! hot reload, and snapshot reconstruction execute this one artifact.
//!
//! Snapshot reconstruction also executes this canonical construction plan; it
//! is not a second construction authority.

use ambition_combat::components::ActorFaction;
use std::collections::BTreeSet;
use std::hash::{Hash, Hasher};

use bevy::ecs::entity::Entity;
use bevy::ecs::query::With;
use bevy::prelude::{Commands, Resource};

use super::transaction;
use crate::features::{self, RoomFeatureConstructionPlan};
use crate::world::physics::{self, PhysicsRoomEntity};
use crate::world::placements::PlacementLoweringRegistry;
use ambition_platformer2d_shared_tangle::lifecycle::RoomScopedEntity;
use ambition_platformer2d_shared_tangle::lifecycle::{
    session_world_component_mut, SessionSpawnScope,
};
use ambition_platformer2d_world::platforms::MovingPlatformState;
use ambition_platformer2d_world::rooms::{RespawnRoomVisualsRequested, RoomSet, RoomSpec};

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
    /// Prepare from already-borrowed services. This is the system-facing seam
    /// used by activation, reset, transition, and hot reload.
    #[allow(clippy::too_many_arguments)]
    pub fn prepare_from_parts(
        rooms: &RoomSet,
        target_index: usize,
        placement_lowering: &PlacementLoweringRegistry,
        content_staging: &features::RoomContentStagingRegistry,
        character_catalog: &ambition_characters::actor::character_catalog::CharacterCatalog,
        authored_sheets: &ambition_sprite_sheet::character::sheets::AuthoredSheets,
        boss_catalog: &ambition_boss_encounter::BossCatalog,
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
            authored_sheets,
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
        authored_sheets: &ambition_sprite_sheet::character::sheets::AuthoredSheets,
        boss_catalog: &ambition_boss_encounter::BossCatalog,
        session_scope: SessionSpawnScope,
        construction: features::ActorConstructionContext<'_>,
    ) -> Result<Self, RoomConstructionError> {
        let feature_plan = RoomFeatureConstructionPlan::prepare(
            &spec,
            placement_lowering,
            content_staging,
            character_catalog,
            authored_sheets,
            boss_catalog,
            construction,
        )
        .map_err(|reason| RoomConstructionError::InvalidFeatures {
            room: spec.id.clone(),
            reason,
        })?;
        let platform_states =
            ambition_platformer2d_world::platforms::moving_platforms_for_room(&spec);
        let id = construction_plan_id(&spec, &feature_plan);
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

    /// The occurrence dispositions this plan was prepared against.
    ///
    /// A plan is frozen against a world that remembered exactly this much. A
    /// cached plan prepared while an authored object was in somebody's hands
    /// left that object out; committing it into a world where the object has
    /// since been put down and destroyed would leave the room permanently
    /// short. Anything that holds a plan across frames compares this before
    /// promoting it.
    ///
    /// it is the whole outlook, not a set of suppressed identities. A plan
    /// that placed a relocated object at one position is not the plan a world
    /// wants once that object rests at another, and an identity set cannot tell
    /// those two apart.
    pub fn occurrence_outlook(
        &self,
    ) -> &ambition_platformer2d_shared_tangle::lifecycle::RoomOccurrenceOutlook {
        self.features.occurrence_outlook()
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
        sim_id: &ambition_platformer2d_shared_tangle::sim_id::SimId,
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
    /// This is the room transaction boundary. Everything the room is made of is queued
    /// between [`transaction::open`] and [`transaction::close`], so the verification that
    /// publishes `RoomLoaded` runs after ALL of it: the feature families, the planned roots,
    /// the planned relationships, the moving-platform bodies, and the last-commit receipt.
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
        // no platform VISUAL is spawned here any more. The commit installs
        // platform STATE (the receipt below counts it); the picture is
        // reconciled by a render family from `MovingPlatformSet`, like every
        // other room feature. That is what let the visual adapter leave the
        // actor monolith at all — see `world::platforms`.
        commands.insert_resource(LastRoomConstructionCommit {
            plan_id: self.id.clone(),
            room_id: self.room_id().to_string(),
            authoritative_ids: receipt.authoritative_ids().clone(),
            moving_platform_count: self.platform_states.len(),
        });
        transaction::close(
            commands,
            &self.features,
            &receipt,
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
                // `try_despawn`: the outgoing roster is collected BEFORE the
                // frame's commands flush, so an entity in it can already have
                // been despawned by something else in the same frame — an actor
                // death, a session teardown racing a transition. Retiring a room
                // entity that is already gone is the outcome this wants, so
                // failing on it turns a success into a crash.
                //
                // This is the honest residue of Task 5's transactionality
                // question: `apply_to_world` promises no fallible
                // LOOKUP, and promised nothing about the commands it queues.
                // This was the only command in the construction path that could
                // fail — everything else spawns.
                commands.entity(entity).try_despawn();
            }
        }
    }

    /// Publish target geometry/platform state and enqueue the exact frozen room
    /// contents. Call only after every preflight has succeeded.
    pub fn commit_deferred(
        &self,
        commands: &mut Commands,
        rooms: &mut RoomSet,
        geometry: &mut ambition_platformer2d_core::RoomGeometry,
        moving_platforms: &mut Vec<MovingPlatformState>,
    ) {
        rooms.set_active(self.target_index);
        geometry.0 = self.spec().world.clone();
        *moving_platforms = self.platform_states.clone();
        self.spawn_contents(commands);
    }
}

/// Identity of one prepared room-construction artifact, from EVERY frozen
/// world-defining preparation product — not just the authored source.
///
/// `deterministic_dump()` is the canonical rendering of exactly that derived surface (schema
/// version, content binding/epoch, every plan row with recipe + origin + parameter summary,
/// every relation with its canonical payload), so folding it in makes the id a function of the
/// complete frozen plan.
///
/// Moving platforms and kinematic paths are pure functions of the spec, so the spec JSON
/// already covers them. Deliberately EXCLUDED: `SessionSpawnScope` / `TransactionId`
/// (commit-time, not frozen-plan), `Entity` values, and anything process-local.
fn construction_plan_id(
    spec: &RoomSpec,
    features: &RoomFeatureConstructionPlan,
) -> RoomConstructionPlanId {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    spec.id.hash(&mut hasher);
    // RoomSpec is the canonical authored room artifact; JSON avoids depending
    // on map insertion order because its fields are vectors/ordered values.
    serde_json::to_vec(spec)
        .expect("RoomSpec serialization must succeed for construction identity")
        .hash(&mut hasher);
    features.construction_deterministic_dump().hash(&mut hasher);
    RoomConstructionPlanId(format!("room-plan:{:016x}", hasher.finish()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d_core as ae;

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

    /// THE FIXTURE CAST, because every body is built from a character (AC6)
    /// and a placement that names none is refused at construction.
    ///
    /// `'static`: `ActorConstructionContext` BORROWS the cast, so a per-test
    /// local would not outlive the plan it is handed to.
    fn fixture_cast() -> &'static ambition_characters::prepared::PreparedCharacterRegistry {
        static CAST: std::sync::OnceLock<ambition_characters::prepared::PreparedCharacterRegistry> =
            std::sync::OnceLock::new();
        CAST.get_or_init(|| {
            let mut registry = ambition_characters::prepared::PreparedCharacterRegistry::default();
            // `npc_giant_gnu_hands` is minted by `giant_cluster_rows` for the two
            // limb rows, so a cast without it refuses the whole giant cluster.
            for id in ["combatant", "npc_giant_gnu_hands"] {
                let mut definition =
                    ambition_characters::actor::definition::CharacterDefinition::new(
                        id, id, "test",
                    )
                    .with_locomotion(
                        ambition_characters::actor::CharacterLocomotion {
                            run_speed: 155.0,
                            ..Default::default()
                        },
                    );
                definition.vitals.max_health = Some(4);
                let finalized = crate::character_runtime::prepare_and_finalize_for_test(
                    definition,
                    &ambition_characters::prepared::CharacterBindings::default(),
                );
                registry.insert_prepared(finalized.prepared);
            }
            registry
        })
    }

    fn prepare(spec: RoomSpec) -> Result<RoomConstructionPlan, RoomConstructionError> {
        let recipes = crate::construction::engine_construction_registry();
        RoomConstructionPlan::prepare_spec(
            0,
            spec,
            &PlacementLoweringRegistry::default(),
            &features::RoomContentStagingRegistry::default(),
            &ambition_characters::actor::character_catalog::CharacterCatalog::empty(),
            &Default::default(),
            &ambition_boss_encounter::BossCatalog::default(),
            SessionSpawnScope::UNSCOPED,
            features::ActorConstructionContext::new(&recipes, Default::default())
                .with_prepared(fixture_cast()),
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

    #[cfg(feature = "portal")]
    #[test]
    fn portal_gun_is_a_capability_owned_construction_lane() {
        let mut spec = empty_spec("portal-lane");
        spec.portal_gun_spawns
            .push(ambition_platformer2d_world::rooms::PortalGunSpawnSpec {
                id: "gun".to_string(),
                name: "Aperture Device".to_string(),
                pos: ae::Vec2::new(120.0, 80.0),
                half_extent: ae::Vec2::new(8.0, 6.0),
                pair: 0,
            });
        let plan = prepare(spec).expect("portal-gun room plan");
        let gun = ambition_platformer2d_shared_tangle::sim_id::SimId::placement("gun");

        assert!(
            plan.features.construction().get(&gun).is_none(),
            "portal-gun vocabulary must not re-enter ActorConstructionParams",
        );
        assert!(
            plan.features.portal_construction().get(&gun).is_some(),
            "the portal capability owns the authored pickup row",
        );
        assert_eq!(
            plan.features.portal_construction().lane().as_str(),
            ambition_portal2d::PORTAL_GUN_CONSTRUCTION_DOMAIN,
        );

        let mut app = bevy::prelude::App::new();
        app.add_message::<ambition_platformer2d_world::rooms::RoomLoaded>();
        {
            let mut commands = app.world_mut().commands();
            plan.spawn_contents(&mut commands);
        }
        app.world_mut().flush();

        let entity = {
            let mut query = app.world_mut().query::<(
                bevy::prelude::Entity,
                &ambition_platformer2d_shared_tangle::sim_id::SimId,
            )>();
            query
                .iter(app.world())
                .find_map(|(entity, id)| (id == &gun).then_some(entity))
                .expect("constructed portal-gun root")
        };
        assert!(app
            .world()
            .get::<ambition_portal2d::PortalGunPickup>(entity)
            .is_some());
        assert!(
            app.world()
                .resource::<crate::features::LastConstructionVerification>()
                .published,
            "all typed construction lanes must verify before RoomLoaded",
        );
    }

    /// As [`prepare`], but with an explicit CAST and content epoch — the two
    /// preparation inputs OUTSIDE the `RoomSpec` that shape the derived plan.
    ///
    /// it took a `&CharacterRoster` and the giant tests below handed it
    /// rows declaring `mount_class: Some("giant")`. A character states its mount
    /// now (AC6), so the cast is the input that decides whether a placement
    /// lowers to a limbed host.
    fn prepare_with(
        spec: RoomSpec,
        cast: &ambition_characters::prepared::PreparedCharacterRegistry,
        epoch: ae::ContentEpoch,
    ) -> Result<RoomConstructionPlan, RoomConstructionError> {
        let recipes = crate::construction::engine_construction_registry();
        RoomConstructionPlan::prepare_spec(
            0,
            spec,
            &PlacementLoweringRegistry::default(),
            &features::RoomContentStagingRegistry::default(),
            &ambition_characters::actor::character_catalog::CharacterCatalog::empty(),
            &Default::default(),
            &ambition_boss_encounter::BossCatalog::default(),
            SessionSpawnScope::UNSCOPED,
            features::ActorConstructionContext::new(&recipes, epoch).with_prepared(cast),
        )
    }

    /// A cast whose `"giant_gnu"` is a `"giant"`-class limbed host.
    fn giant_cast(
        mount_class: Option<&str>,
    ) -> ambition_characters::prepared::PreparedCharacterRegistry {
        let mut definition = ambition_characters::actor::definition::CharacterDefinition::new(
            "giant_gnu",
            "Giant GNU",
            "test",
        )
        .with_locomotion(ambition_characters::actor::CharacterLocomotion {
            run_speed: 0.0,
            ..Default::default()
        });
        definition.vitals.max_health = Some(42);
        if let Some(class) = mount_class {
            definition.mount = Some(ambition_characters::actor::CharacterMount {
                class: Some(class.to_string()),
                ..Default::default()
            });
        }
        let finalized = crate::character_runtime::prepare_and_finalize_for_test(
            definition,
            &ambition_characters::prepared::CharacterBindings::default(),
        );
        // The hands travel with the host: `giant_cluster_rows` mints two limb
        // rows naming `npc_giant_gnu_hands`.
        let mut registry = fixture_cast().clone();
        registry.insert_prepared(finalized.prepared);
        registry
    }

    fn giant_spec_sized(id: &str, half: f32) -> RoomSpec {
        let mut spec = empty_spec(id);
        let payload = ambition_platformer2d_world::rooms::EnemySpawnSpec::new(
            ambition_entity_catalog::placements::CharacterBrain::Custom("giant_gnu".into()),
            "giant_gnu",
        );
        spec.enemy_spawns
            .push(ambition_platformer2d_world::rooms::Authored::new(
                "gnu",
                "Giant GNU",
                ae::Aabb::new(ae::Vec2::new(100.0, 100.0), ae::Vec2::splat(half)),
                payload,
            ));
        spec
    }

    fn giant_spec(id: &str) -> RoomSpec {
        giant_spec_sized(id, 60.0)
    }

    /// The plan id tracks the DERIVED construction surface, not just the
    /// authored spec. Two rosters that differ only in the giant's body size
    /// produce byte-identical `RoomSpec`s but different hand `home_offset`
    /// relation payloads — materially different prepared worlds. The previous id
    /// (spec JSON + authored id set) collided them.
    #[test]
    fn the_plan_id_tracks_the_derived_relation_payloads() {
        let small = prepare_with(
            giant_spec_sized("arena", 60.0),
            &giant_cast(Some("giant")),
            ae::ContentEpoch(4),
        )
        .expect("small-giant plan");
        let large = prepare_with(
            giant_spec_sized("arena", 70.0),
            &giant_cast(Some("giant")),
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
            &giant_cast(Some("giant")),
            ae::ContentEpoch(4),
        )
        .expect("giant plan");
        // Same spec, but the cast has no idea "giant_gnu" is a giant.
        let plain = prepare_with(giant_spec("arena"), &giant_cast(None), ae::ContentEpoch(4))
            .expect("plain plan");
        assert_ne!(giant.id(), plain.id());
    }

    /// The id tracks the prepared-content epoch: the same room prepared against
    /// re-prepared content is a different transaction target.
    #[test]
    fn the_plan_id_tracks_the_content_epoch() {
        let four = prepare_with(
            giant_spec("arena"),
            &giant_cast(Some("giant")),
            ae::ContentEpoch(4),
        )
        .expect("epoch-4 plan");
        let five = prepare_with(
            giant_spec("arena"),
            &giant_cast(Some("giant")),
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
            &giant_cast(Some("giant")),
            ae::ContentEpoch(4),
        )
        .expect("pathless plan");
        let mut with_path = giant_spec("arena");
        with_path
            .kinematic_paths
            .push(ambition_platformer2d_world::rooms::KinematicPathSpec::new(
                "patrol",
                "patrol",
                ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::splat(8.0)),
                ae::KinematicPath::line(ae::Vec2::ZERO, ae::Vec2::new(64.0, 0.0), 24.0),
            ));
        let pathed = prepare_with(with_path, &giant_cast(Some("giant")), ae::ContentEpoch(4))
            .expect("pathed plan");
        assert_ne!(bare.id(), pathed.id());
    }

    /// One giant, every roster surface, one answer. The prepared plan, the
    /// predicted outer roster, the commit receipt, and the boundary verifier all
    /// name the same three-cluster — and the hands are welcome plan rows, not
    /// unexpected or legacy findings.
    #[test]
    fn a_giant_rooms_rosters_agree_from_plan_to_receipt_to_verifier() {
        let plan = prepare_with(
            giant_spec("arena"),
            &giant_cast(Some("giant")),
            ae::ContentEpoch(4),
        )
        .expect("giant plan");

        let host = ambition_platformer2d_shared_tangle::sim_id::SimId::placement("gnu");
        let cluster: BTreeSet<String> = [
            host.to_string(),
            ambition_platformer2d_shared_tangle::sim_id::SimId::spawned(&host, 0).to_string(),
            ambition_platformer2d_shared_tangle::sim_id::SimId::spawned(&host, 1).to_string(),
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
        app.add_message::<ambition_platformer2d_world::rooms::RoomLoaded>();
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
        spec.enemy_spawns
            .push(ambition_platformer2d_world::rooms::Authored::new(
                "same-id",
                "first",
                aabb,
                ambition_platformer2d_world::rooms::EnemySpawnSpec::new(
                    ambition_entity_catalog::placements::CharacterBrain::Custom("combatant".into()),
                    "combatant",
                ),
            ));
        spec.enemy_spawns
            .push(ambition_platformer2d_world::rooms::Authored::new(
                "same-id",
                "second",
                aabb,
                ambition_platformer2d_world::rooms::EnemySpawnSpec::new(
                    ambition_entity_catalog::placements::CharacterBrain::Custom("combatant".into()),
                    "combatant",
                ),
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

    /// `RoomLoaded` is published only after the WHOLE room is applied.
    ///
    /// The transaction boundary is `spawn_contents`, not the feature plan. An observer reads
    /// the world the instant `RoomLoaded` is delivered and proves the platforms, the commit
    /// receipt, and the authoritative bodies are already present.
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
                    faction: ambition_combat::components::ActorFaction::Npc,
                    grudge_against: None,
                    kind: features::SpawnActorKind::Enemy {
                        brain: ambition_entity_catalog::placements::CharacterBrain::Custom(
                            "combatant".into(),
                        ),
                        character: ambition_entity_catalog::CharacterId::from("combatant"),
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
            &Default::default(),
            &ambition_boss_encounter::BossCatalog::default(),
            SessionSpawnScope::UNSCOPED,
            features::ActorConstructionContext::new(&recipes, Default::default())
                .with_prepared(fixture_cast()),
        )
        .expect("plan");

        let mut app = bevy::prelude::App::new();
        app.add_message::<ambition_platformer2d_world::rooms::RoomLoaded>();
        app.add_message::<features::SpawnActorRequest>();

        let observed = std::sync::Arc::new(std::sync::Mutex::new(None));
        let sink = observed.clone();
        app.add_systems(
            bevy::prelude::Update,
            move |mut reader: bevy::ecs::message::MessageReader<
                ambition_platformer2d_world::rooms::RoomLoaded,
            >,
                  commit: Option<bevy::prelude::Res<LastRoomConstructionCommit>>,
                  ids: bevy::prelude::Query<
                &ambition_platformer2d_shared_tangle::sim_id::SimId,
            >| {
                if reader.read().next().is_some() {
                    *sink.lock().unwrap() = Some((
                        commit.map(|c| c.moving_platform_count),
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

        let (commit_platforms, saw_occupant) = observed
            .lock()
            .unwrap()
            .expect("RoomLoaded must have published for a valid room");
        assert_eq!(
            commit_platforms,
            Some(1),
            "the last-commit receipt existed before RoomLoaded"
        );
        // What the test defends is unchanged — the receipt, and therefore the room's STATE, is
        // complete before `RoomLoaded` publishes.
        assert!(
            saw_occupant,
            "the authoritative occupant existed before RoomLoaded"
        );
    }

    #[test]
    fn commit_receipt_matches_the_prepared_root_roster() {
        let mut spec = empty_spec("receipt");
        spec.enemy_spawns
            .push(ambition_platformer2d_world::rooms::Authored::new(
                "enemy-1",
                "enemy",
                ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::splat(16.0)),
                ambition_platformer2d_world::rooms::EnemySpawnSpec::new(
                    ambition_entity_catalog::placements::CharacterBrain::Custom("combatant".into()),
                    "combatant",
                ),
            ));
        let plan = prepare(spec).expect("plan");
        let expected = plan.predicted_authoritative_ids().clone();
        let expected_id = plan.id().clone();

        let mut app = bevy::prelude::App::new();
        app.add_message::<ambition_platformer2d_world::rooms::RoomLoaded>();
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
                    ambition_platformer2d_shared_tangle::sim_id::SimId::placement(&feature.0)
                        .to_string()
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

    /// Prepare room `index` of a WORLD, against what that world remembers about
    /// its occurrences. The pair is one argument on purpose — see
    /// [`features::OccurrenceContinuity`].
    fn prepare_in_world(
        world: &[RoomSpec],
        index: usize,
        remembered: &ambition_platformer2d_shared_tangle::lifecycle::AuthoredOccurrences,
    ) -> Result<RoomConstructionPlan, RoomConstructionError> {
        let recipes = crate::construction::engine_construction_registry();
        let mut construction =
            features::ActorConstructionContext::new(&recipes, Default::default())
                .with_prepared(fixture_cast());
        construction.continuity = Some(features::OccurrenceContinuity {
            remembered,
            world,
            // no checkpoint behind this planner: it prepares a spec from the
            // world's own records, and `None` is the honest answer for a
            // composition that has taken none.
            minted: None,
        });
        RoomConstructionPlan::prepare_spec(
            index,
            world[index].clone(),
            &PlacementLoweringRegistry::default(),
            &features::RoomContentStagingRegistry::default(),
            &ambition_characters::actor::character_catalog::CharacterCatalog::empty(),
            &Default::default(),
            &ambition_boss_encounter::BossCatalog::default(),
            SessionSpawnScope::UNSCOPED,
            construction,
        )
    }

    /// Where one planned row would put the occurrence it builds.
    fn planned_position(
        plan: &RoomConstructionPlan,
        sim_id: &ambition_platformer2d_shared_tangle::sim_id::SimId,
    ) -> Option<ae::Vec2> {
        plan.features
            .construction()
            .entities()
            .iter()
            .find(|entity| entity.sim_id() == sim_id)
            .map(|entity| match entity.parameters() {
                crate::construction::ActorConstructionParams::GroundItem { spec, .. } => spec.pos,
                other => {
                    panic!("the planned row is not the ground item it was authored as: {other:?}")
                }
            })
    }

    /// A ROOM REBUILDS AN OCCURRENCE WHOSE RECORD LIVES NEXT DOOR — and the
    /// room that owns the record does NOT rebuild it. One row, both halves.
    ///
    /// This is room construction ceasing to be a pure function of one
    /// `RoomSpec`: what a room owes the world is its current RESIDENCY, derived
    /// from the world's definitions plus the authoritative disposition of every
    /// occurrence, and an occurrence carried out of the room that minted it and
    /// put down elsewhere belongs to the room it is lying in.
    #[test]
    fn a_room_reinstates_an_occurrence_whose_record_lives_next_door() {
        let mut home = empty_spec("blink_run");
        home.ground_items
            .push(ambition_platformer2d_world::rooms::GroundItemSpec {
                id: "axe".into(),
                name: "Axe".into(),
                held_item: "gun_sword".into(),
                pos: ae::Vec2::new(10.0, 20.0),
                half_extent: ae::Vec2::splat(8.0),
            });
        let world = vec![home, empty_spec("portal_bridge")];
        let axe = ambition_platformer2d_shared_tangle::sim_id::SimId::placement("axe");
        let left_at = ae::Vec2::new(300.0, 64.0);

        // ── NOBODY HAS TOUCHED IT: the home room authors it where it says ────
        // The baseline that makes the two claims below changes rather than
        // coincidences.
        let untouched = Default::default();
        let plan = prepare_in_world(&world, 0, &untouched).expect("home plan");
        assert_eq!(
            planned_position(&plan, &axe),
            Some(ae::Vec2::new(10.0, 20.0)),
            "an untouched record is authored at its own coordinates"
        );
        assert!(prepare_in_world(&world, 1, &untouched)
            .expect("neighbour plan")
            .predicted_authoritative_ids()
            .is_empty());

        // ── IT WAS CARRIED NEXT DOOR AND PUT DOWN ───────────────────────────
        let mut remembered =
            ambition_platformer2d_shared_tangle::lifecycle::AuthoredOccurrences::default();
        // ⚠ CARRIED, then put down — in that order, because that is the road.
        // The ledger refuses a placement for an id it does not already hold as
        // a live occurrence, so a fixture that jumps straight to `Placed` is
        // modelling a relocation that cannot happen.
        remembered.republish_custody([axe.clone()].into_iter().collect());
        assert!(
            remembered
                .republish_placements(
                    "portal_bridge",
                    [(axe.clone(), left_at)].into_iter().collect(),
                )
                .is_empty(),
            "an occurrence that passed through custody may be put down"
        );

        // HALF ONE: the room it is lying in builds it, at the position it was
        // left, from a record that room does not own.
        let away = prepare_in_world(&world, 1, &remembered).expect("destination plan");
        assert_eq!(
            planned_position(&away, &axe),
            Some(left_at),
            "'portal_bridge' owes the world this occurrence: it is lying there, \
             and the only record that can rebuild it belongs to 'blink_run'"
        );
        let row = away
            .features
            .construction()
            .entities()
            .iter()
            .find(|entity| entity.sim_id() == &axe)
            .expect("the row asserted above");
        assert!(
            matches!(
                row.origin(),
                ambition_platformer2d_shared_tangle::construction::SpawnOrigin::Authored { source, .. }
                    if source.as_str() == "blink_run"
            ),
            "and its PROVENANCE still names the room that authored it — it was \
             moved, not re-created somewhere else: {:?}",
            row.origin(),
        );

        // HALF TWO: the room that authors the record does not build it.
        let home_again = prepare_in_world(&world, 0, &remembered).expect("home plan");
        assert_eq!(
            planned_position(&home_again, &axe),
            None,
            "'blink_run' must not mint a second occurrence of a record whose \
             first one is lying in 'portal_bridge'"
        );
    }
}

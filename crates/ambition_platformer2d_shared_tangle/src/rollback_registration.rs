//! Rollback declaration owned by `ambition_platformer2d_shared_tangle`.
//!
//! Shared body/lifecycle primitives name their own rewindable state here. The
//! host supplies storage and GGRS integration through [`RollbackRegistrar`].

use ambition_platformer2d_core::snapshot::RollbackRegistrar;

const OWNER: &str = env!("CARGO_PKG_NAME");

/// Register everything the primitives domain needs rewound.
pub fn register_rollback_state<R>(registrar: &mut R)
where
    R: RollbackRegistrar,
{
    // ⛔⛔ THE SAFE-POSITION MECHANIC'S DECLARATIONS, AND THEY ARRIVED A COMMIT
    // LATE. `6dca281` moved the types, the snapshot codecs and every consumer
    // here and left these two rows behind in the actor monolith — whose header
    // says it "names only state defined in this crate" — which is the SAME
    // incomplete carve already fixed for mount and for rooms earlier the same
    // day. Moving a type is four things, and the registration is the one that
    // compiles fine where it is.
    //
    // ⛔ THE STABLE NAMES DO NOT MOVE. `resource.sandbox_sim_state` and
    // `player.safety_state` are identities on the wire; the schema fingerprint
    // deliberately excludes owner labels so an ownership repoint is not a
    // wire-format event.
    registrar.rollback_resource_canonical::<crate::safe_position::RoomTransitionCooldown>(
        OWNER,
        "resource.sandbox_sim_state",
    );
    registrar.rollback_component_canonical::<crate::safe_position::PlayerSafetyState>(
        OWNER,
        "player.safety_state",
    );
    registrar.require_rollback::<crate::body::BodyKinematics>(OWNER, "entity:body_kinematics");
    registrar
        .require_rollback::<crate::lifecycle::FeatureSimEntity>(OWNER, "entity:feature_sim_entity");
    registrar.rollback_resource_canonical::<crate::time::SimDt>(OWNER, "resource.sim_dt");
    registrar
        .rollback_resource_canonical::<crate::gravity::BaseGravity>(OWNER, "resource.base_gravity");
    registrar.rollback_resource_canonical::<crate::gravity::GravityField>(
        OWNER,
        "resource.gravity_field",
    );
    // ⛔⛔ A TEMPORARY WELL IS SPAWNED MID-MATCH, so the room-load re-derivation
    // that covers an AUTHORED gravity zone covers nothing here. A gravity
    // grenade's fuse spawns this pair, `tick_temporary_zones` counts `remaining`
    // down, and the zone despawns at zero — existence, position and countdown
    // are all authoritative simulation state on an entity nothing anchored.
    //
    // ⭐ THE ANCHOR IS `TemporaryZone`, NOT `GravityZone`, and that is the whole
    // distinction: an authored column is room geometry a room load rebuilds, and
    // enlisting every one of them in the rollback sweep would pay for state that
    // never changes. Only the zone with a lifetime is dynamic, so only it carries
    // the anchor — and `GravityZone` is registered as state because a restored
    // temporary entity that came back without its aabb would pull nothing.
    registrar.require_rollback::<crate::gravity::TemporaryZone>(OWNER, "entity:temporary_zone");
    registrar.rollback_component_clone_probed::<crate::gravity::TemporaryZone>(
        OWNER,
        "gravity.temporary_zone",
        |zone| zone.remaining.to_bits() as u64,
    );
    registrar.rollback_component_clone_probed::<crate::gravity::GravityZone>(
        OWNER,
        "gravity.zone",
        |zone| {
            ((zone.aabb.min.x.to_bits() as u64) << 32)
                ^ (zone.aabb.min.y.to_bits() as u64)
                ^ ((zone.aabb.max.x.to_bits() as u64) << 16)
                ^ (zone.aabb.max.y.to_bits() as u64)
                ^ ((zone.dir.x.to_bits() as u64) << 48)
                ^ (zone.dir.y.to_bits() as u64)
        },
    );
    registrar.rollback_component_canonical::<crate::sim_id::SimId>(OWNER, "entity.sim_id");
    registrar.rollback_component_canonical::<crate::body::BodyKinematics>(OWNER, "body.kinematics");
    registrar
        .rollback_component_canonical::<crate::sim_id::SimIdCounter>(OWNER, "body.sim_id_counter");
    registrar.rollback_component_canonical::<crate::construction::TransactionId>(
        OWNER,
        "component.construction_transaction_id",
    );
    registrar.rollback_component_canonical::<crate::construction::SpawnOrigin>(
        OWNER,
        "entity.spawn_origin",
    );
    registrar.rollback_component_canonical::<crate::orientation::ActorRoll>(OWNER, "actor.roll");
    registrar
        .rollback_component_clone::<crate::lifecycle::RoomVisual>(OWNER, "lifecycle.room_visual");
    registrar.rollback_component_clone::<crate::lifecycle::PlayerVisual>(
        OWNER,
        "lifecycle.player_visual",
    );
    registrar.rollback_component_clone::<crate::body::PrimaryBody>(OWNER, "marker.primary_body");
    registrar.rollback_component_clone::<crate::lifecycle::FeatureSimEntity>(
        OWNER,
        "marker.feature_sim_entity",
    );
    // A construction-time marker on a simulated body, exactly like `PlayerVisual`
    // beside it: it decides whether the pose read model is published for this
    // entity, and a rewind that dropped it would stop publishing mid-match.
    registrar
        .rollback_component_clone::<crate::lifecycle::PosedBody>(OWNER, "marker.posed_body");
    registrar
        .rollback_component_clone::<crate::markers::PlayerEntity>(OWNER, "marker.player_entity");
    registrar
        .rollback_component_clone::<crate::markers::PrimaryPlayer>(OWNER, "marker.primary_player");
    registrar.declare_rollback_derived_resource::<crate::markers::ControlledSubject>(
        OWNER,
        "derived.controlled_subject",
        "resolved from the entity holding DrivingParticipant for the active slot",
    );
    registrar.declare_rollback_derived_resource::<crate::class_b::ClassBRemapLog>(
        OWNER,
        "derived.class_b_remap_log",
        "frame-local diagnostic ledger cleared before every simulation step",
    );
    registrar.declare_rollback_derived_resource::<crate::gravity::GravityZones>(
        OWNER,
        "derived.gravity_zones",
        "rebuilt from authoritative GravityZone components before body integration",
    );
    registrar.declare_rollback_derived_resource::<crate::body::BodyContactSnapshot>(
        OWNER,
        "derived.body_contact_snapshot",
        "cleared and refilled from authoritative BodyKinematics every tick, immediately \
         before the movement phase that reads it",
    );
    registrar.rollback_component_clone_probed::<crate::body::BodyContact>(
        OWNER,
        "body.contact",
        // probed, not presence-only. The capability is a NUMBER: two peers
        // can agree a body is solid and disagree about how hard, which is a
        // divergence in every step that body takes beside another one.
        |contact| u64::from(contact.resistance.to_bits()),
    );
    registrar
        .rollback_component_canonical::<crate::lifecycle::RoomScopedEntity>(OWNER, "scope.room");
    registrar.rollback_component_canonical::<crate::lifecycle::SessionScopedEntity>(
        OWNER,
        "scope.session",
    );
    // `InCustodyOf` is fully reprojected each tick from rollback-authoritative
    // custody/possession state. It contains an entity handle, so duplicating it
    // in the snapshot would add a second, less stable encoding of the same fact.
    registrar.declare_rollback_derived_component::<crate::lifecycle::InCustodyOf>(
        OWNER,
        "derived.custody_residency",
        "room residency reprojected from ItemCustody every tick",
    );
    // Loaded authored occurrences are republished from live state. Unloaded
    // placement rows only disappear at confirmed room transitions, which a
    // rewind cannot cross.
    registrar.declare_rollback_derived_resource::<crate::lifecycle::AuthoredOccurrences>(
        OWNER,
        "derived.placement_continuity",
        "authored-occurrence whereabouts; republished from live state while its room is loaded",
    );
    // Checkpoint-baseline values and their message cursors are declared beside the lifecycle
    // horizon that owns them.
    crate::lifecycle::horizon::register_checkpoint_rollback_state(registrar);
    registrar.rollback_component_canonical::<crate::projectile::ProjectileGameplay>(
        OWNER,
        "projectile.gameplay",
    );
    // ⛔ THE SHOT'S OWN VICTIM LEDGER, and it is the same shape and the same
    // registrar path as `combat.hitbox_hits` because it is the same rule: a
    // continuous stretch of contact owns ONE per-victim answer, and a returning
    // shot's second leg is a second stretch. An entity set cannot be snapshot by
    // value — bevy_ggrs destroys and recreates what it names — so it clones and
    // probes through the targets' stable sim identities.
    registrar.rollback_component_clone_entity_set::<crate::projectile::ProjectileHits>(
        OWNER,
        "projectile.hits",
        |hits| hits.hit.iter().copied().collect(),
    );
    registrar
        .rollback_map_entities::<crate::projectile::ProjectileHits>(OWNER, "map.projectile_hits");
    registrar.declare_rollback_derived_component::<crate::frame_env::ResolvedMotionFrame>(
        OWNER,
        "derived.resolved_motion_frame",
        "published every tick from the live environment",
    );
    registrar.declare_rollback_derived_component::<crate::orientation::SurfaceUpright>(
        OWNER,
        "derived.surface_upright",
        "republished from support facts every movement step",
    );
    registrar.declare_rollback_derived_resource::<crate::frame_env::ForceZones>(
        OWNER,
        "derived.force_zones",
        "per-tick zone snapshot rebuilt by collect_force_zones",
    );
    registrar.declare_rollback_derived_resource::<crate::feature_overlay::FeatureEcsWorldOverlay>(
        OWNER,
        "derived.feature_ecs_world_overlay",
        "collision contributions rebuilt from ECS feature state every tick",
    );
    registrar.declare_dynamic_anchor::<crate::projectile::ProjectileGameplay>(
        OWNER,
        "dynamic.projectile",
        "Rollback entity recreation plus the complete projectile component family",
    );
    // Authored-command delivery is an in-tick channel: a rewind clears the
    // abandoned branch and the narrative ledger republishes the request.
    registrar.clear_message_on_rollback::<crate::authored_logic::RunAuthoredCommand>(
        OWNER,
        "message.run_authored_command",
    );

    // The deterministic round-scope allocator is optional because games without
    // a match never install it.
    registrar.rollback_resource_optional_canonical::<crate::lifecycle::ActiveRoundScope>(
        OWNER,
        "resource.active_round_scope",
    );

    registrar.rollback_component_clone::<crate::camera_ease::PlayerBlinkCameraState>(
        OWNER,
        "player.blink_camera_state",
    );
    registrar.clear_message_on_rollback::<crate::body::MountDied>(OWNER, "message.mount_died");
    registrar.clear_message_on_rollback::<crate::camera_ease::CameraShakeRequest>(
        OWNER,
        "message.camera_shake_request",
    );
}

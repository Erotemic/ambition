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
        // ⚠ **probed, not presence-only.** The capability is a NUMBER: two peers
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
    // ⚠ **RESIDENCY IS DERIVED, the SCOPES above are not.** `InCustodyOf` is a
    // pure projection of `ItemCustody` (registered as `item.item_custody`, with
    // its holder handle remapped): `project_custody_onto_residency` recomputes
    // the whole thing every tick from live state, with no "already applied"
    // gate, so a rewind that restores custody restores residency with it on the
    // next step. It is declared rather than registered because snapshotting it
    // would be storing an answer the sim recomputes anyway — and left
    // undeclared it is exactly the behaviour-gating component the coverage
    // census exists to catch.
    //
    // ⛔ **deliberately NOT `declare_rollback_derived_component_state`**, which
    // would demand a `SnapshotState` value projection. The only value here is an
    // entity HANDLE, and hashing a raw handle is the determinism hazard
    // `item.item_custody` already answers properly — its `_entity_set` probe
    // measures the same holder through that body's stable `SimId`. A second,
    // WORSE projection of the same fact is not more coverage.
    //
    // ⛔⛔ **THE REASON BELOW IS A PROMISE, AND IT IS OWED BY EVERY POPULATION
    // THAT WEARS THIS COMPONENT (2026-08-19).** "Reprojected from `ItemCustody`"
    // is what excuses this from the snapshot — so a population `ItemCustody`
    // cannot see is a population nothing reprojects, and a rewind drops the
    // marker with nothing to put it back.
    //
    // ⇒ that is exactly what happened when a POSSESSED BODY started wearing it:
    // a body has no `ItemCustody`. It is covered by a second deriver,
    // `abilities::traversal::possession::project_possession_onto_custody`,
    // reading `PossessionState` — which IS snapshot state — so the excuse holds
    // for both. ⚠ **a third population owes a third deriver**, and the poison is
    // cheap: delete the component and step, because that is what a restore does.
    // Writing it at the site that causes it passes every other test.
    //
    // ⛔⛔ **AND THE REASON STRING BELOW IS DELIBERATELY NOT UPDATED TO NAME THE
    // SECOND DERIVER.** It reads as a comment and it is not one: `detail` reaches
    // `RollbackRegistry::schema_dump`, which is hashed into
    // `schema_fingerprint`, which the schema baseline pins and a peer compares.
    // Editing this prose to be more accurate would be a WIRE FORMAT CHANGE, paid
    // for in save/peer compatibility, to improve a sentence. The accurate version
    // lives in the block above, where it costs nothing.
    registrar.declare_rollback_derived_component::<crate::lifecycle::InCustodyOf>(
        OWNER,
        "derived.custody_residency",
        "room residency reprojected from ItemCustody every tick",
    );
    // ⚠ **WHY THE WHEREABOUTS LEDGER IS STILL DERIVED, stated sharply because the
    // old reason went stale the day it gained a second producer.** It used to say
    // "custody is the only producer, and custody is republished every tick". That
    // is no longer true: `Placed { room, at }` rows are stamped by the item domain
    // and do not self-retract.
    //
    // ⭐ the argument that replaces it, and it is the one written at
    // `AuthoredOccurrences::rewind_argument`: **every row is republished from live
    // state while that state is loaded.** The single value that cannot be
    // recomputed is a `Placed` row whose room is UNLOADED — and a room unloads
    // only at a CONFIRMED transition, which a rewind never crosses. So the
    // unrecomputable rows are exactly the ones no rewind can reach.
    //
    // ⛔⛔ **the day `OccurrenceWhereabouts::Consumed` gains a producer this
    // declaration becomes a LIE.** That leg accumulates: it records that an
    // occurrence is gone for good, which is precisely a fact no live component
    // still carries, so nothing re-derives it and a rewind past the destruction
    // would not un-record it. It owes a registration with a real VALUE
    // projection (the id set, not a presence probe) and a durable-save
    // representation at the same time. There is no producer as of 2026-08-15.
    registrar.declare_rollback_derived_resource::<crate::lifecycle::AuthoredOccurrences>(
        OWNER,
        "derived.placement_continuity",
        "authored-occurrence whereabouts; republished from live state while its room is loaded",
    );
    // Checkpoint-baseline values and their message cursors are declared beside
    // the lifecycle horizon that owns them. The crate-wide registrar composes
    // the offer; it no longer enumerates the horizon's concrete types.
    crate::lifecycle::horizon::register_checkpoint_rollback_state(registrar);
    registrar.rollback_component_canonical::<crate::projectile::ProjectileGameplay>(
        OWNER,
        "projectile.gameplay",
    );
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

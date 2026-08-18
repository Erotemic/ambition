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
    registrar.require_rollback::<crate::body::BodyKinematics>(
        OWNER,
        "entity:body_kinematics",
    );
    registrar.require_rollback::<crate::lifecycle::FeatureSimEntity>(
        OWNER,
        "entity:feature_sim_entity",
    );
    registrar.rollback_resource_canonical::<crate::time::SimDt>(
        OWNER,
        "resource.sim_dt",
    );
    registrar.rollback_resource_canonical::<crate::gravity::BaseGravity>(
        OWNER,
        "resource.base_gravity",
    );
    registrar.rollback_resource_canonical::<crate::gravity::GravityField>(
        OWNER,
        "resource.gravity_field",
    );
    registrar.rollback_component_canonical::<crate::sim_id::SimId>(
        OWNER,
        "entity.sim_id",
    );
    registrar.rollback_component_canonical::<crate::body::BodyKinematics>(
        OWNER,
        "body.kinematics",
    );
    registrar.rollback_component_canonical::<crate::sim_id::SimIdCounter>(
        OWNER,
        "body.sim_id_counter",
    );
    registrar.rollback_component_canonical::<crate::construction::TransactionId>(
        OWNER,
        "component.construction_transaction_id",
    );
    registrar.rollback_component_canonical::<crate::construction::SpawnOrigin>(
        OWNER,
        "entity.spawn_origin",
    );
    registrar.rollback_component_canonical::<crate::orientation::ActorRoll>(
        OWNER,
        "actor.roll",
    );
    registrar.rollback_component_clone::<crate::lifecycle::RoomVisual>(
        OWNER,
        "lifecycle.room_visual",
    );
    registrar.rollback_component_clone::<crate::lifecycle::PlayerVisual>(
        OWNER,
        "lifecycle.player_visual",
    );
    registrar.rollback_component_clone::<crate::body::PrimaryBody>(
        OWNER,
        "marker.primary_body",
    );
    registrar.rollback_component_clone::<crate::lifecycle::FeatureSimEntity>(
        OWNER,
        "marker.feature_sim_entity",
    );
    registrar.rollback_component_clone::<crate::markers::PlayerEntity>(
        OWNER,
        "marker.player_entity",
    );
    registrar.rollback_component_clone::<crate::markers::PrimaryPlayer>(
        OWNER,
        "marker.primary_player",
    );
    registrar.declare_rollback_derived_resource::<crate::markers::ControlledSubject>(
        OWNER,
        "derived.controlled_subject",
        "resolved from the entity carrying Brain::Player for the active slot",
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
    registrar.rollback_component_canonical::<crate::lifecycle::RoomScopedEntity>(
        OWNER,
        "scope.room",
    );
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
    // ⭐⭐ **THE TWO BASELINES ARE THE OPPOSITE CASE, AND THAT CONTRAST IS THE
    // WHOLE REASON THEY SIT HERE.** The ledger above is derived because live
    // state republishes every row of it. Nothing republishes a baseline: it is
    // written once, at a checkpoint commit, which is a shrine touched MID-FRAME
    // and therefore squarely inside the rollback window. Rewind across that
    // commit with these declared derived and the world keeps a baseline taken
    // from a future that got un-happened — so the next death restores a world
    // that never existed.
    //
    // ⚠ this is exactly the trap the note above predicts for
    // `OccurrenceWhereabouts::Consumed`, arrived at from the other direction: an
    // accumulating value needs a real VALUE projection, not a presence probe.
    // ⛔ **CHECKSUMMED, not plain clones.** `rollback_resource_clone` records
    // "state checksum supplied by another authoritative projection", and for
    // these that sentence would be false: nothing else projects a baseline, so a
    // plain clone would put them in the snapshot and leave desync detection
    // blind to them. The projections live on the values themselves.
    registrar.rollback_resource_clone_checksum::<crate::lifecycle::OccurrenceBaseline>(
        OWNER,
        "resource.occurrence_baseline",
        "entity-free remembered-whereabouts checksum projection",
        crate::lifecycle::OccurrenceBaseline::checksum,
    );
    registrar.rollback_resource_clone_checksum::<crate::lifecycle::CustodyBaseline>(
        OWNER,
        "resource.custody_baseline",
        "entity-free remembered-custody checksum projection",
        crate::lifecycle::CustodyBaseline::checksum,
    );
    // ⭐ **AND THE TWO CHANNELS THAT DRIVE THEM.** A reader's cursor is `Local`
    // state GGRS never rewinds, so a rewind past a commit leaves the capture's
    // cursor beyond a message the new timeline has not sent — or before one it
    // already consumed. Either direction is a baseline recorded for a world that
    // did not happen, and the value is not self-correcting: nothing republishes
    // a baseline, so the mistake survives until the next checkpoint.
    //
    // ⚠ a WAIVER would have been available and would have been wrong. The
    // question a waiver answers is *"can a stale cursor change the
    // simulation?"*, and here it decides what a later death restores.
    registrar.clear_message_on_rollback::<crate::lifecycle::CheckpointCommitted>(
        OWNER,
        "message.checkpoint_committed",
    );
    registrar.clear_message_on_rollback::<crate::lifecycle::ResetToCheckpoint>(
        OWNER,
        "message.reset_to_checkpoint",
    );
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

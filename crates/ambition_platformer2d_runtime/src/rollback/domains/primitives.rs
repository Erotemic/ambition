//! **The primitives domain's rollback schema** (Campaign 2, R3).
//!
//! The primitives crate's own rollback state: body kinematics, lifecycle scope tags, orientation, and the projectile gameplay carrier.
//!
//! ⚠ **relocation only.** The registrations were extracted mechanically and the
//! schema baseline verifies the result is byte-identical — a retyped call is
//! exactly the mistake that would slip through review and not through the
//! baseline.
//!
//! ⚠ the owner label stays `ambition_platformer2d_runtime` because this module is in it, and
//! must be: `ambition_platformer2d_shared_tangle` sits below the runtime in the crate graph. R1's
//! recorded decision is that this is the right shape for every domain below the
//! runtime; crates above it own their schemas directly.

use bevy::prelude::App;

use super::super::AmbitionRollbackApp;

const OWNER: &str = "ambition_platformer2d_runtime";

/// Register everything the primitives domain needs rewound.
pub(in crate::rollback) fn register(app: &mut App) {
    app.require_rollback::<ambition_platformer2d_shared_tangle::body::BodyKinematics>(
        OWNER,
        "entity:body_kinematics",
    );
    app.require_rollback::<ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity>(
        OWNER,
        "entity:feature_sim_entity",
    );
    app.rollback_resource_canonical::<ambition_platformer2d_shared_tangle::time::SimDt>(
        OWNER,
        "resource.sim_dt",
    );
    app.rollback_resource_canonical::<ambition_platformer2d_shared_tangle::gravity::BaseGravity>(
        OWNER,
        "resource.base_gravity",
    );
    app.rollback_resource_canonical::<ambition_platformer2d_shared_tangle::gravity::GravityField>(
        OWNER,
        "resource.gravity_field",
    );
    app.rollback_component_canonical::<ambition_platformer2d_shared_tangle::sim_id::SimId>(
        OWNER,
        "entity.sim_id",
    );
    app.rollback_component_canonical::<ambition_platformer2d_shared_tangle::body::BodyKinematics>(
        OWNER,
        "body.kinematics",
    );
    app.rollback_component_canonical::<ambition_platformer2d_shared_tangle::sim_id::SimIdCounter>(
        OWNER,
        "body.sim_id_counter",
    );
    app.rollback_component_canonical::<ambition_platformer2d_shared_tangle::construction::TransactionId>(
        OWNER,
        "component.construction_transaction_id",
    );
    app.rollback_component_canonical::<ambition_platformer2d_shared_tangle::construction::SpawnOrigin>(
        OWNER,
        "entity.spawn_origin",
    );
    app.rollback_component_canonical::<ambition_platformer2d_shared_tangle::orientation::ActorRoll>(
        OWNER,
        "actor.roll",
    );
    app.rollback_component_clone::<ambition_platformer2d_shared_tangle::lifecycle::RoomVisual>(
        OWNER,
        "lifecycle.room_visual",
    );
    app.rollback_component_clone::<ambition_platformer2d_shared_tangle::lifecycle::PlayerVisual>(
        OWNER,
        "lifecycle.player_visual",
    );
    app.rollback_component_clone::<ambition_platformer2d_shared_tangle::body::PrimaryBody>(
        OWNER,
        "marker.primary_body",
    );
    app.rollback_component_clone::<ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity>(
        OWNER,
        "marker.feature_sim_entity",
    );
    app.rollback_component_clone::<ambition_platformer2d_shared_tangle::markers::PlayerEntity>(
        OWNER,
        "marker.player_entity",
    );
    app.rollback_component_clone::<ambition_platformer2d_shared_tangle::markers::PrimaryPlayer>(
        OWNER,
        "marker.primary_player",
    );
    app.declare_rollback_derived_resource::<ambition_platformer2d_shared_tangle::markers::ControlledSubject>(
        OWNER,
        "derived.controlled_subject",
        "resolved from the entity carrying Brain::Player for the active slot",
    );
    app.declare_rollback_derived_resource::<ambition_platformer2d_shared_tangle::class_b::ClassBRemapLog>(
        OWNER,
        "derived.class_b_remap_log",
        "frame-local diagnostic ledger cleared before every simulation step",
    );
    app.declare_rollback_derived_resource::<ambition_platformer2d_shared_tangle::gravity::GravityZones>(
        OWNER,
        "derived.gravity_zones",
        "rebuilt from authoritative GravityZone components before body integration",
    );
    app.rollback_component_canonical::<ambition_platformer2d_shared_tangle::lifecycle::RoomScopedEntity>(
        OWNER,
        "scope.room",
    );
    app.rollback_component_canonical::<ambition_platformer2d_shared_tangle::lifecycle::SessionScopedEntity>(
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
    app.declare_rollback_derived_component::<ambition_platformer2d_shared_tangle::lifecycle::InCustodyOf>(
        OWNER,
        "derived.custody_residency",
        "room residency reprojected from ItemCustody every tick",
    );
    app.rollback_component_canonical::<ambition_platformer2d_shared_tangle::projectile::ProjectileGameplay>(
        OWNER,
        "projectile.gameplay",
    );
    app.declare_rollback_derived_component::<ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame>(
        OWNER,
        "derived.resolved_motion_frame",
        "published every tick from the live environment",
    );
    app.declare_rollback_derived_component::<ambition_platformer2d_shared_tangle::orientation::SurfaceUpright>(
        OWNER,
        "derived.surface_upright",
        "republished from support facts every movement step",
    );
    app.declare_rollback_derived_resource::<ambition_platformer2d_shared_tangle::frame_env::ForceZones>(
        OWNER,
        "derived.force_zones",
        "per-tick zone snapshot rebuilt by collect_force_zones",
    );
    app.declare_rollback_derived_resource::<ambition_platformer2d_shared_tangle::feature_overlay::FeatureEcsWorldOverlay>(
        OWNER,
        "derived.feature_ecs_world_overlay",
        "collision contributions rebuilt from ECS feature state every tick",
    );
    app.declare_dynamic_anchor::<ambition_platformer2d_shared_tangle::projectile::ProjectileGameplay>(
        OWNER,
        "dynamic.projectile",
        "Rollback entity recreation plus the complete projectile component family",
    );
}

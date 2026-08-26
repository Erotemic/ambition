//! Backend-neutral rollback schema composition.
//!
//! Gameplay domains own their concrete declarations through
//! [`RollbackRegistrar`].  This module composes those declarations into stable
//! schema metadata for every simulation host. A concrete rollback backend may
//! call the same composition function with its own registrar to install storage,
//! checksums, mapping, and load behavior without making the generic runtime
//! depend on that backend.

use ambition_platformer2d_core::snapshot::{checksum_bytes, RollbackRegistrar};
use bevy::prelude::*;

pub mod registrar;
pub mod registry;

pub use registrar::SchemaRollbackRegistrar;
pub use registry::*;

/// Install the host-independent typed rollback schema used by prepared-content
/// identity. This plugin records metadata only; rollback hosts install their
/// backend machinery through the same declarations in their owning crate.
pub struct AmbitionRollbackSchemaPlugin;

impl Plugin for AmbitionRollbackSchemaPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RollbackRegistry>();
        let mut registrar = SchemaRollbackRegistrar::new(app);
        register_engine_rollback_state(&mut registrar);
    }
}

const ENGINE: &str = "ambition_platformer2d_runtime";

pub fn register_engine_rollback_state(registrar: &mut impl RollbackRegistrar) {
    use ambition_platformer2d_core::body_clusters as bc;

    // DOMAIN-OWNED ROLLBACK DECLARATIONS. The composition supplies one backend-neutral
    // registrar; each capability names its own concrete types and projections.
    // This is composition, not a type census: adding state to an existing domain
    // edits only that domain, and the runtime contains no gameplay type paths.
    ambition_encounter::register_rollback_state(registrar);
    ambition_combat::register_rollback_state(registrar);
    ambition_platformer2d_actor_monolith::register_rollback_state(registrar);
    ambition_mount::register_rollback_state(registrar);
    ambition_characters::register_rollback_state(registrar);
    ambition_time::register_rollback_state(registrar);
    ambition_boss_encounter::register_rollback_state(registrar);
    ambition_conversation::register_rollback_state(registrar);
    ambition_sprite_sheet::register_rollback_state(registrar);
    ambition_platformer2d_shared_tangle::register_rollback_state(registrar);
    ambition_vfx::register_rollback_state(registrar);
    ambition_items::register_rollback_state(registrar);
    // Portal rollback state exists only when the portal capability is enabled.
    #[cfg(feature = "portal")]
    ambition_portal2d::register_rollback_state(registrar);
    ambition_cutscene::register_rollback_state(registrar);
    ambition_persistence::register_rollback_state(registrar);
    ambition_projectiles::register_rollback_state(registrar);
    ambition_sim_view::register_rollback_state(registrar);
    ambition_platformer2d_world::rooms::register_rollback_state(registrar);
    ambition_platformer2d_world::rooms::register_gate_portal_rollback_state(registrar);

    // Rollback participation. These anchors cover the canonical session root,
    // every simulated body, projectile-only entities, encounter authorities,
    // and any semantic-identity entity that does not fit those families.
    //
    //  actor-owned anchors now live in
    // `ambition_platformer2d_actor_monolith::register_rollback_state`; only
    // foundation/runtime-owned rows remain below. In-flight strike volumes (moveset melee
    // windows, DamageBox effects, world AOEs).

    // Canonical live-session root. Authored definitions are immutable and bound
    // by PreparedContentIdentity; only mutable selection/cursor state rewinds.
    //
    //  actor-owned members moved to
    // `ambition_platformer2d_actor_monolith::register_rollback_state`; the
    // geometry is `ambition_platformer2d_core`'s and stays.
    registrar.rollback_component_clone::<ambition_platformer2d_core::RoomGeometry>(
        ENGINE,
        "root.geometry",
    );

    // Global authoritative resources.
    // ⭐ THE CLOCK'S OWN STATE IS NOT HERE ANY MORE. `SimTick`, `WorldTime`,
    // `ClockState` and `ProperTimeScale` are declared by `ambition_time`, which
    // was already declaring four other resources of its own from
    // `register_rollback_state` at the top of this function — so the same crate's
    // state had two owners. Stable names unchanged; the wire did not move.
    //
    // Match activation is rollback state. Rewinding before activation must restore the
    // resource's absence so seating can reconstruct the roster from authored inputs.
    registrar
        .rollback_resource_canonical::<ambition_platformer2d_world::collision::MovingPlatformSet>(
            ENGINE,
            "resource.moving_platform_set",
        )
        // The gate portals' live phase is registered too — but NOT here, and
        // not by this crate. `ambition_platformer2d_world` owns both halves of it
        // now: `GatePortalPhases` documents why an integrator whose input rewinds
        // must rewind with it, and `register_gate_portal_rollback_state` (called
        // with the domain-owned declarations at the top of this function) performs the
        // registration through the floor's `RollbackRegistrar` vocabulary.  it
        // carries a VALUE projection, not a presence probe — see that function.
        .rollback_resource_clone::<crate::InputStreamRecorder>(
            ENGINE,
            "resource.input_stream_recorder",
        );

    // Core body state.
    registrar
        .rollback_component_canonical::<bc::BodyAbilities>(ENGINE, "body.abilities")
        .rollback_component_canonical::<bc::BodyGroundState>(ENGINE, "body.ground")
        .rollback_component_canonical::<bc::BodyWallState>(ENGINE, "body.wall")
        .rollback_component_canonical::<bc::BodyJumpState>(ENGINE, "body.jump")
        .rollback_component_canonical::<bc::BodyDashState>(ENGINE, "body.dash")
        .rollback_component_canonical::<bc::BodyFlightState>(ENGINE, "body.flight")
        .rollback_component_canonical::<bc::BodyBlinkState>(ENGINE, "body.blink")
        .rollback_component_canonical::<bc::BodyDodgeState>(ENGINE, "body.dodge")
        .rollback_component_canonical::<bc::BodyShieldState>(ENGINE, "body.shield")
        .rollback_component_canonical::<bc::BodyOffense>(ENGINE, "body.offense")
        .rollback_component_canonical::<bc::BodyLifetime>(ENGINE, "body.lifetime")
        .rollback_component_canonical::<bc::BodyActionBuffer>(ENGINE, "body.action_buffer")
        .rollback_component_canonical::<bc::BodyBaseSize>(ENGINE, "body.base_size")
        .rollback_component_canonical::<bc::SweepSample>(ENGINE, "body.sweep_sample")
        .rollback_component_canonical::<bc::BodyMana>(ENGINE, "body.mana");
    // In-flight strike volumes — the components on the `entity:hitbox` family (see the
    // require_rollback anchor above). G2b: probed through the OWNER's stable identity, paired with
    // the hitbox's own — the same treatment `ProjectileOwner` has. A strike volume remapped onto
    // the wrong body damages the wrong faction's targets, and a presence count could not tell that
    // from a correct restore. The victims this strike has ALREADY hit. Losing one from the set is a
    // sustained overlap re-hitting a body it already hit, which is exactly the kind of one-frame
    // difference the aggregate reports as a desync with no name attached. G2b: probed through the
    // fired victims' stable identities. A presence count sees the component and nothing of WHO is
    // in the set, so a remap redirecting one victim to the wrong body changes no census — and the
    // visible consequence is a sustained overlap re-firing an on-hit at a body it has already fired
    // at.

    // A live match's per-body state. Registered together because they are one decision — match
    // activation — landing on a body, and a rewind that kept some and dropped others would
    // produce a fighter that is half in the match. S4 — the stocks loop's own state. A stock
    // count that is NOT rollback state un-spends itself on a rewind: the body comes back and
    // the count does not, so a fighter loses the same stock twice or never loses it at all.
    // Elimination is the same fact one step later, and a rewind that restores a fighter while
    // leaving it eliminated is a body standing in a match nothing will ever let it play. The
    // "already announced" latch for a stocks match's outcome.
    registrar
        .rollback_component_canonical::<ambition_platformer2d_core::geometry::CenteredAabb>(
            ENGINE,
            "actor.centered_aabb",
        )
        .rollback_component_canonical::<bc::BodyModeState>(ENGINE, "actor.body_mode")
        .rollback_component_canonical::<bc::BodyLedgeState>(ENGINE, "actor.ledge")
        .rollback_component_canonical::<ambition_platformer2d_core::MotionModel>(
            ENGINE,
            "actor.motion_model",
        )
        .rollback_component_canonical::<bc::BodyComboTrace>(ENGINE, "actor.combo_trace");

    // Register values a recreated rollback entity cannot safely reconstruct from another
    // authoritative source. This includes identity/projection memos, rig/custody maps, authored
    // presentation bindings, mutable world-feature state, and pickup latches. Value-bearing
    // bookkeeping is probed by value rather than presence/count so swaps and stale derivation
    // records cannot pass a census unchanged.
    registrar
        .rollback_component_clone::<ambition_platformer2d_core::body_clusters::AbilityBase>(
            ENGINE,
            "body.ability_base",
        )
        // Runtime-staged actors need this marker after restore so presentation can
        // rediscover them. `SfxSource` must also survive for projectiles because it
        // is stamped at spawn and may outlive the firing body; probe it by value.
        .rollback_component_clone_probed::<ambition_sfx::BodyPresentationSource>(
            ENGINE,
            "presentation.body_source",
            |source| checksum_bytes(source.id().as_str().as_bytes()),
        )
        // The marker that says the per-tick derivation OWNS that source and may retract
        // it. Losing it across a restore would strand a body's source: the derivation
        // stops maintaining what it can no longer recognise as its own.
        .rollback_component_clone::<ambition_sfx::DerivedPresentationSource>(
            ENGINE,
            "presentation.body_source_derived",
        )
        // Portal-gun cooldown, in-flight shot, and pickup arm timers affect future
        // transit/grab decisions and therefore must rewind with portal state.
        .rollback_component_clone::<bevy::prelude::Name>(ENGINE, "entity.name")
        .rollback_component_clone::<bevy::prelude::Transform>(ENGINE, "entity.transform");

    // These values are guaranteed to be republished before any downstream
    // consumer in each GGRS frame, so storing them would duplicate authority.
    // A per-tick MIRROR of the item's own body, not a second authority:
    // `sync_ground_items_to_transitable` overwrites pos/vel/half_extent from the
    // authoritative `GroundItem` (registered state) before portal core reads it, and
    // `sync_transitable_to_ground_items` mirrors the possibly-teleported result
    // straight back. Snapshotting it would give one body two restorable positions.
    //
    //  this DECLARED-DERIVED group lost its actor-owned head to
    // `ambition_platformer2d_actor_monolith::register_rollback_state`; the rest belongs to
    // `ambition_characters`.
    registrar.declare_rollback_derived_component::<bevy::prelude::GlobalTransform>(
        ENGINE,
        "derived.global_transform",
        "Bevy transform propagation rebuilds it from Transform and hierarchy",
    )
    // AE6. Derived, not state: `project_combat_rules` rebuilds it in WorldPrep
    // every tick from the match's declaration folded over the world's baseline,
    // both of which outlive any rollback window — the declaration is route
    // lifecycle (`Update`, outside the sim) and the baseline is authored tuning.
    // Registering it as STATE would be the borrow again: a rewind would restore
    // a rules value independently of the declaration that produced it, and the
    // two could then disagree for a frame.
;
    // `ProjectileOwner(Entity)` is authoritative entity-bearing rollback state
    // and is restored with entity remapping; not every projectile carries a
    // `SpawnOrigin` from which ownership could be re-derived.
    //
    // Boss simulation animation state is also covered here so rollback restores
    // the cursor that drives derived hurtbox samples.

    // G2: probed through the OWNER's stable `SimId`, not by counting carriers.
    registrar
        .declare_rollback_derived_component::<ambition_platformer2d_core::body_clusters::BodyEnvironmentContact>(
            ENGINE,
            "derived.body_environment_contact",
            "rewritten every movement step from body geometry and the live world",
        )
        .declare_rollback_derived_component::<ambition_platformer2d_core::BodyMotionFacts>(
            ENGINE,
            "derived.body_motion_facts",
            "republished from MotionModel every movement step",
        )
        .declare_rollback_derived_component::<ambition_sim_view::BodyPoseView>(
            ENGINE,
            "derived.body_pose_view",
            "SimView projection rebuilt every tick",
        )
        .declare_rollback_derived_component::<ambition_sim_view::ProjectileView>(
            ENGINE,
            "derived.projectile_view",
            "SimView projection rebuilt every tick",
        )
        // Frame-derived RESOURCES (Phase 5 resource-coverage pass): each is
        // republished by its ordinary maintenance system before anything reads it,
        // so a rewind that keeps a stale value is overwritten before it matters.
        .declare_rollback_derived_resource::<ambition_platformer2d_core::control_frame::ControlFrame>(
            ENGINE,
            "derived.control_frame",
            "per-tick input frame regenerated from the synchronized input stream",
        );

    // Abandoned-future transient ingress must be empty after LoadWorld. Replayed inputs and
    // deterministic systems regenerate the correct messages. S4 — the stocks loop's two
    // messages.
    registrar.clear_message_on_rollback::<ambition_platformer2d_world::rooms::RoomLoaded>(
        ENGINE,
        "message.room_loaded",
    )
    // ⚠ `ambition_sfx` CANNOT DECLARE ITS OWN. It is the one domain crate left in
    // this list that cannot see `RollbackRegistrar`: its dependencies are
    // `ambition_sfx_bank` plus an OPTIONAL `bevy_ecs`, deliberately near-leaf, and
    // giving it `ambition_platformer2d_core` to federate two rows would buy a
    // dependency edge with a declaration. So these stay here, owned by the
    // composition, and that is the answer rather than an omission.
    .clear_message_on_rollback::<ambition_sfx::OwnedSfxMessage>(ENGINE, "message.owned_sfx")
    // A same-tick handshake: the reset processor announces it, and the teardown
    // systems chained after it read it. A cursor GGRS did not rewind would let
    // that teardown fire for a reset the resimulation never committed to — the
    // held items and portals of a session that was, on this timeline, never
    // reset.
    .clear_message_on_rollback::<ambition_platformer2d_world::rooms::RespawnRoomVisualsRequested>(
        ENGINE,
        "message.respawn_room_visuals",
    );
}

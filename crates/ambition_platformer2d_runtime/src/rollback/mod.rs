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

pub mod authority;
pub mod registrar;
pub mod registry;

pub use authority::{
    ActiveRollbackAuthority, RollbackConfirmationState, RollbackDiagnostic,
    RollbackDiagnosticHistory, RollbackTimelineContract, RollbackTimelineGeneration,
    RollbackTimelineStatus, SessionRollbackConfirmation,
};
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
    // ⭐ NO `body_clusters` ALIAS ANY MORE, and its absence is the measurement:
    // this function no longer names a single one of the floor's types.

    // DOMAIN-OWNED ROLLBACK DECLARATIONS. The composition supplies one backend-neutral
    // registrar; each capability names its own concrete types and projections.
    // This is composition, not a type census: adding state to an existing domain
    // edits only that domain, and the runtime contains no gameplay type paths.
    ambition_encounter::register_rollback_state(registrar);
    ambition_combat::register_rollback_state(registrar);
    ambition_platformer2d_actor_monolith::register_rollback_state(registrar);
    ambition_mount::register_rollback_state(registrar);
    ambition_characters::register_rollback_state(registrar);
    // ⭐ THE FLOOR DECLARES ITS OWN NOW, and the precedent that settled it is the
    // line right below: `ambition_time` is equally a floor crate and has done so
    // since 2026-08-26. Two floor crates answering the question differently was
    // the last split left in the federation.
    ambition_platformer2d_core::register_rollback_state(registrar);
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
    // `ambition_platformer2d_actor_monolith::register_rollback_state`, and the
    // geometry to `ambition_platformer2d_core`'s own.

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

    // ⭐ CORE BODY STATE IS NOT HERE ANY MORE — fifteen body-cluster components,
    // declared by `ambition_platformer2d_core` beside the types themselves.
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
    // ⭐ the five rows this comment used to introduce — `actor.centered_aabb`,
    // `actor.body_mode`, `actor.ledge`, `actor.motion_model`, `actor.combo_trace`
    // — are `ambition_platformer2d_core`'s and are declared there.

    // Register values a recreated rollback entity cannot safely reconstruct from another
    // authoritative source. This includes identity/projection memos, rig/custody maps, authored
    // presentation bindings, mutable world-feature state, and pickup latches. Value-bearing
    // bookkeeping is probed by value rather than presence/count so swaps and stale derivation
    // records cannot pass a census unchanged.
    registrar
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
        // ⚠ the frame-derived RESOURCE that used to close this chain,
        // `derived.control_frame`, is the floor's and is declared there.
        ;

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

//! Rollback declaration owned by `ambition_characters`.
//!
//! This module names this domain's concrete rewindable state while the host
//! supplies the backend through [`RollbackRegistrar`]. It deliberately contains
//! no `bevy_ggrs` dependency and no host/composition logic.

use ambition_platformer2d_core::snapshot::RollbackRegistrar;

const OWNER: &str = env!("CARGO_PKG_NAME");

/// Register everything the characters domain needs rewound.
pub fn register_rollback_state<R>(registrar: &mut R)
where
    R: RollbackRegistrar,
{
    registrar.rollback_component_canonical::<crate::actor::BodyHealth>(OWNER, "body.health");
    registrar.rollback_component_canonical::<crate::actor::pose::ActorPose>(OWNER, "actor.pose");
    registrar
        .rollback_component_canonical::<crate::actor::WornCharacter>(OWNER, "actor.worn_character");
    registrar
        .rollback_component_clone::<crate::equipment::WornEquipment>(OWNER, "actor.worn_equipment");
    registrar
        .rollback_component_canonical::<crate::actor::body::BodyCombat>(OWNER, "actor.body_combat");
    registrar.rollback_component_canonical::<crate::brain::boss_pattern::BossAttackState>(
        OWNER,
        "boss.attack_state",
    );
    registrar.rollback_component_canonical::<crate::brain::boss_pattern::BossAttackIntent>(
        OWNER,
        "boss.attack_intent",
    );
    registrar.rollback_component_cursor::<crate::brain::Brain>(OWNER, "actor.brain");
    registrar.rollback_component_canonical::<crate::actor::character_catalog::BrainBinding>(
        OWNER,
        "actor.brain_binding",
    );
    registrar
        .rollback_component_canonical::<crate::actor::character_catalog::AuthoredBrainContext>(
            OWNER,
            "actor.authored_brain_context",
        );
    registrar.rollback_component_canonical::<crate::brain::ActorControl>(OWNER, "actor.control");
    registrar.rollback_component_canonical::<crate::actor::attack_gesture::AttackGestureState>(
        OWNER,
        "actor.attack_gesture_state",
    );
    registrar.rollback_component_canonical::<crate::actor::attack_gesture::AttackGestureTuning>(
        OWNER,
        "actor.attack_gesture_tuning",
    );
    registrar.rollback_component_canonical::<crate::actor::BodyWallet>(OWNER, "body.wallet");
    registrar
        .rollback_component_clone::<crate::brain::ScriptedControl>(OWNER, "actor.scripted_control");
    // **WHO DRIVES THIS BODY, and it is DERIVED rather than snapshot state.**
    // `control::project_driving_participant` reprojects it every tick from
    // `Brain::Player` — which IS registered, above — plus `PossessionState`,
    // which is registered rollback state in the actor domain. Both inputs rewind,
    // so a restore rebuilds this on the next tick with nothing to put back.
    //
    // ⛔⛔ **"it is a derive" is not a thing the coverage guard can infer, and I
    // learned that the expensive way.** `rollback_coverage` offers exactly three
    // outcomes — registered, DECLARED derived, or waived — and a component that
    // is genuinely reprojected but says so nowhere fails all three. Eight
    // coverage tests plus `rollback_exit_oracle` went red on main for one missing
    // declaration.
    //
    // ⛔ **and the string below is WIRE.** `detail` reaches
    // `RollbackRegistry::schema_dump` and is hashed into `schema_fingerprint`,
    // which the baseline pins and a peer compares — so declaring a derive is a
    // schema change even though the component itself encodes nothing. That is
    // why this slice took a version bump after claiming it needed none. Do not
    // edit this prose to read better; the accurate long version is this comment.
    registrar.declare_rollback_derived_component::<crate::brain::DrivingParticipant>(
        OWNER,
        "derived.driving_participant",
        "the driving seat reprojected from Brain::Player and possession every tick",
    );
    // `ScriptedControl` is the projection; this set records which authority owns it.
    registrar.rollback_component_clone_probed::<crate::brain::ControlHolds>(
        OWNER,
        "actor.control_holds",
        |holds| u64::from(holds.bits()),
    );
    registrar
        .rollback_component_clone::<crate::actor::BodyAnimFacts>(OWNER, "actor.animation_facts");
    registrar.rollback_component_clone::<crate::actor::ActorFaction>(OWNER, "actor.faction");
    registrar.rollback_component_clone::<crate::brain::ChargesProjectiles>(
        OWNER,
        "actor.charges_projectiles",
    );
    registrar.rollback_component_clone::<crate::brain::PlayerSlot>(OWNER, "actor.player_slot");
    registrar.rollback_component_clone::<crate::brain::ActionSet>(OWNER, "actor.action_set");
    registrar.rollback_component_clone::<crate::brain::action_set::IdentityKit>(
        OWNER,
        "actor.identity_kit",
    );
    registrar.rollback_component_clone::<crate::brain::BossCapability>(OWNER, "boss.capability");
    registrar
        .rollback_component_clone::<crate::brain::MovesetRanged>(OWNER, "actor.moveset_ranged");
    registrar.declare_rollback_derived_component::<crate::action_scheme::ActorActionScheme>(
        OWNER,
        "derived.actor_action_scheme",
        "reconciled from abilities, moveset, and action set",
    );
    registrar.declare_rollback_derived_component::<crate::action_scheme::ResolvedTechniqueEdges>(
        OWNER,
        "derived.resolved_technique_edges",
        "cleared and republished from current input every frame",
    );
    registrar
        .declare_rollback_derived_component::<crate::actor::attack_gesture::ResolvedAttackGesture>(
        OWNER,
        "derived.resolved_attack_gesture",
        "republished from ActorControl and rollback-backed gesture history before move triggering",
    );
    registrar.declare_rollback_derived_resource::<crate::brain::SlotControls>(
        OWNER,
        "derived.slot_controls",
        "republished from GGRS PlayerInputs at the head of every frame",
    );
    registrar.clear_message_on_rollback::<crate::brain::ActorActionMessage>(
        OWNER,
        "message.actor_action",
    );
    // ⭐ **the PLATFORM-FIGHTER half of a capture** — pummel count, hold age and
    // escape progress. Authoritative sim state for exactly the reason its
    // relation half is: a rewind past a pummel must undo the pummel, and a
    // rewind past a mash must undo the progress it bought.
    //
    // ⚠ it joined this domain on 2026-08-19 when those three fields left
    // `ambition_combat::capture::CapturedBy`. The relation stays registered by
    // `ambition_combat`; this is a second, separately-owned row rather than a
    // widening of that one, which is the whole point of the split.
    registrar.rollback_component_canonical::<crate::smash_capture::SmashHoldState>(
        OWNER,
        "smash.hold_state",
    );
}

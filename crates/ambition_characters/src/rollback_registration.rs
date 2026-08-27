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
    registrar.rollback_component_clone_entity_map::<crate::actor::limb::LimbRig>(
        OWNER,
        "limb.rig",
        |rig| {
            rig.limbs
                .iter()
                .map(|(slot, limb)| (slot.probe_key(), *limb))
                .collect()
        },
    );
    registrar.rollback_map_entities::<crate::actor::limb::LimbRig>(OWNER, "map.limb_rig");
    registrar.rollback_component_clone_entity_ref::<crate::actor::limb::Limb>(
        OWNER,
        "limb.member",
        |limb| limb.of,
    );
    registrar.rollback_map_entities::<crate::actor::limb::Limb>(OWNER, "map.limb_member");
    registrar
        .rollback_component_clone::<crate::actor::limb::LimbRouteState>(OWNER, "limb.route_state");
    registrar.rollback_component_clone::<crate::actor::limb::LimbIntents>(OWNER, "limb.intents");
    registrar.rollback_component_canonical::<crate::actor::character_catalog::BrainBinding>(
        OWNER,
        "actor.brain_binding",
    );
    registrar
        .rollback_component_canonical::<crate::actor::character_catalog::AuthoredBrainContext>(
            OWNER,
            "actor.authored_brain_context",
        );
    registrar.rollback_component_canonical::<crate::control::ActorControl>(OWNER, "actor.control");
    registrar.rollback_component_canonical::<crate::actor::attack_gesture::AttackGestureState>(
        OWNER,
        "actor.attack_gesture_state",
    );
    registrar.rollback_component_canonical::<crate::actor::attack_gesture::AttackGestureTuning>(
        OWNER,
        "actor.attack_gesture_tuning",
    );
    registrar.rollback_component_canonical::<crate::actor::BodyWallet>(OWNER, "body.wallet");
    registrar.rollback_component_clone::<crate::control::ScriptedControl>(
        OWNER,
        "actor.scripted_control",
    );
    // `DrivingParticipant` is authoritative rollback state. Possession may
    // reconcile its value from registered `PossessionState`, but there is no
    // independent source from which it can be fully reprojected.
    //
    // Probe the slot value, not only component presence, because the slot number
    // determines who is driving and must participate in desync localization.
    registrar.rollback_component_clone_probed::<crate::control::DrivingParticipant>(
        OWNER,
        "actor.driving_participant",
        |driver| u64::from(driver.0 .0),
    );
    // `ScriptedControl` is the projection; this set records which authority owns it.
    registrar.rollback_component_clone_probed::<crate::control::ControlHolds>(
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
    registrar.rollback_component_clone::<crate::control::PlayerSlot>(OWNER, "actor.player_slot");
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
    registrar.rollback_resource_canonical::<crate::control::SlotInteractionState>(
        OWNER,
        "resource.slot_interaction_state",
    );
    registrar.declare_rollback_derived_resource::<crate::control::SlotControls>(
        OWNER,
        "derived.slot_controls",
        "republished from GGRS PlayerInputs at the head of every frame",
    );
    registrar.clear_message_on_rollback::<crate::brain::ActorActionMessage>(
        OWNER,
        "message.actor_action",
    );
    // the PLATFORM-FIGHTER half of a capture — pummel count, hold age and
    // escape progress. Authoritative sim state for exactly the reason its
    // relation half is: a rewind past a pummel must undo the pummel, and a
    // rewind past a mash must undo the progress it bought.
    registrar.rollback_component_canonical::<crate::smash_capture::SmashHoldState>(
        OWNER,
        "smash.hold_state",
    );
    // ⭐ MOVED WITH ITS TYPE 2026-08-26. `ActorStatus` is one f32 and one
    // `CharacterAiMode`, so it belongs beside the mode; its declaration belongs
    // with it, which is the obligation nothing enforces — a declaration compiles
    // fine in the crate its type just left. ⛔ the STABLE NAME is unchanged, so
    // the wire did not move; only the OWNER string did.
    registrar.rollback_component_canonical::<crate::actor::ai::ActorStatus>(OWNER, "actor.status");
}

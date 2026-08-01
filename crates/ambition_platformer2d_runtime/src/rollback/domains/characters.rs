//! **The characters domain's rollback schema** (Campaign 2, R3).
//!
//! Character state: the body's own components, the brain, poses, worn identity and the action scheme's declared derivations.
//!
//! ⚠ **relocation only.** The registrations were extracted mechanically and the
//! schema baseline verifies the result is byte-identical — a retyped call is
//! exactly the mistake that would slip through review and not through the
//! baseline.
//!
//! ⚠ the owner label stays `ambition_platformer2d_runtime` because this module is in it, and
//! must be: `ambition_characters` sits below the runtime in the crate graph. R1's
//! recorded decision is that this is the right shape for every domain below the
//! runtime; crates above it own their schemas directly.

use bevy::prelude::App;

use super::super::AmbitionRollbackApp;

const OWNER: &str = "ambition_platformer2d_runtime";

/// Register everything the characters domain needs rewound.
pub(in crate::rollback) fn register(app: &mut App) {
    app.rollback_component_canonical::<ambition_characters::actor::BodyHealth>(
        OWNER,
        "body.health",
    );
    app.rollback_component_canonical::<ambition_characters::actor::pose::ActorPose>(
        OWNER,
        "actor.pose",
    );
    app.rollback_component_canonical::<ambition_characters::actor::WornCharacter>(
        OWNER,
        "actor.worn_character",
    );
    app.rollback_component_clone::<ambition_characters::equipment::WornEquipment>(
        OWNER,
        "actor.worn_equipment",
    );
    app.rollback_component_canonical::<ambition_characters::actor::body::BodyCombat>(
        OWNER,
        "actor.body_combat",
    );
    app.rollback_component_canonical::<ambition_characters::brain::boss_pattern::BossAttackState>(
        OWNER,
        "boss.attack_state",
    );
    app.rollback_component_canonical::<ambition_characters::brain::boss_pattern::BossAttackIntent>(
        OWNER,
        "boss.attack_intent",
    );
    app.rollback_component_cursor::<ambition_characters::brain::Brain>(OWNER, "actor.brain");
    app.rollback_component_canonical::<ambition_characters::actor::character_catalog::BrainBinding>(
            OWNER,
            "actor.brain_binding",
        );
    app.rollback_component_canonical::<ambition_characters::actor::character_catalog::AuthoredBrainContext>(
            OWNER,
            "actor.authored_brain_context",
        );
    app.rollback_component_canonical::<ambition_characters::brain::ActorControl>(
        OWNER,
        "actor.control",
    );
    app.rollback_component_canonical::<ambition_characters::actor::attack_gesture::AttackGestureState>(
            OWNER,
            "actor.attack_gesture_state",
        );
    app.rollback_component_canonical::<ambition_characters::actor::attack_gesture::AttackGestureTuning>(
            OWNER,
            "actor.attack_gesture_tuning",
        );
    app.rollback_component_canonical::<ambition_characters::actor::BodyWallet>(
        OWNER,
        "body.wallet",
    );
    app.rollback_component_clone::<ambition_characters::brain::ScriptedControl>(
        OWNER,
        "actor.scripted_control",
    );
    app.rollback_component_clone::<ambition_characters::actor::BodyAnimFacts>(
        OWNER,
        "actor.animation_facts",
    );
    app.rollback_component_clone::<ambition_characters::actor::ActorFaction>(
        OWNER,
        "actor.faction",
    );
    app.rollback_component_clone::<ambition_characters::brain::ChargesProjectiles>(
        OWNER,
        "actor.charges_projectiles",
    );
    app.rollback_component_clone::<ambition_characters::brain::PlayerSlot>(
        OWNER,
        "actor.player_slot",
    );
    app.rollback_component_clone::<ambition_characters::brain::ActionSet>(
        OWNER,
        "actor.action_set",
    );
    app.rollback_component_clone::<ambition_characters::brain::action_set::IdentityKit>(
        OWNER,
        "actor.identity_kit",
    );
    app.rollback_component_clone::<ambition_characters::brain::BossCapability>(
        OWNER,
        "boss.capability",
    );
    app.rollback_component_clone::<ambition_characters::brain::MovesetRanged>(
        OWNER,
        "actor.moveset_ranged",
    );
    app.declare_rollback_derived_component::<ambition_characters::action_scheme::ActorActionScheme>(
        OWNER,
        "derived.actor_action_scheme",
        "reconciled from abilities, moveset, and action set",
    );
    app.declare_rollback_derived_component::<ambition_characters::action_scheme::ResolvedTechniqueEdges>(
        OWNER,
        "derived.resolved_technique_edges",
        "cleared and republished from current input every frame",
    );
    app.declare_rollback_derived_component::<ambition_characters::actor::attack_gesture::ResolvedAttackGesture>(
        OWNER,
        "derived.resolved_attack_gesture",
        "republished from ActorControl and rollback-backed gesture history before move triggering",
    );
    app.declare_rollback_derived_resource::<ambition_characters::brain::SlotControls>(
        OWNER,
        "derived.slot_controls",
        "republished from GGRS PlayerInputs at the head of every frame",
    );
    app.clear_message_on_rollback::<ambition_characters::brain::ActorActionMessage>(
        OWNER,
        "message.actor_action",
    );
}

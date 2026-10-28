//! Rollback declaration owned by the actor runtime.
//!
//! The actor runtime names only state defined in this crate. The host supplies GGRS machinery
//! through [`RollbackRegistrar`].

use ambition_platformer2d_core::snapshot::RollbackRegistrar;

const OWNER: &str = env!("CARGO_PKG_NAME");

/// Register everything the actors domain needs rewound.
pub fn register_rollback_state<R>(registrar: &mut R)
where
    R: RollbackRegistrar,
{
    // Causal readers keep non-rollback `Local` cursors, so their message
    // channels must be cleared rather than replay rows from an abandoned future.
    //
    // ⛔⛤ REGISTERED AS INSTRUMENT CHANNELS, WHICH IS WHAT KEEPS THIS FEATURE
    // OUT OF THE PEER IDENTITY. Until 2026-09-16 these were ordinary
    // `clear_message_on_rollback` rows, so compiling the recorder moved
    // `schema_fingerprint()` — 494 schema rows became 497 — for a simulation
    // that is mechanically identical to the one without it. Two such peers
    // would have computed the same snapshots and the same checksums and then
    // refused to play each other.
    #[cfg(feature = "causal")]
    {
        registrar.clear_instrument_message_on_rollback::<crate::causal::BodyMovementOps>(
            OWNER,
            "message.causal_body_movement_ops",
        );

    }

    registrar.require_rollback::<crate::body::transform_beat::TransformBeat>(
        OWNER,
        "entity:transform_beat",
    );

    registrar.rollback_resource_clone_entity_set::<crate::control::possession::PossessionState>(
            OWNER,
            "resource.possession_state",
            |state| state.possessed.into_iter().chain(state.home).collect(),
        );
    registrar
        .rollback_resource_map_entities::<crate::control::possession::PossessionState>(
            OWNER,
            "map.resource.possession_state",
        );

    // A pet is a script that runs across frames: the walk to the petted
    // body, then the gesture, each with its time left. A rewind into the walk
    // must resume the walk, and one before the request must not keep a beat
    // the resimulation has not started yet. The holds and the two poses are
    // derived from the beat, so the beat is the only row.
    registrar.rollback_component_clone_probed::<crate::body::PetBeat>(
        OWNER,
        "actor.pet_beat",
        |beat| {
            let stage = match beat.stage {
                crate::body::PetStage::Walk { remaining } => remaining.to_bits() as u64,
                crate::body::PetStage::Gesture { remaining } => {
                    (1 << 63) | u64::from(remaining.to_bits())
                }
            };
            (u64::from(beat.mark.to_bits()) << 32) ^ stage ^ u64::from(beat.side.to_bits())
        },
    );
    // A conversation's spacing: where the initiator steps to and the time
    // the step has left. A rewind into the step must resume it, and one
    // before the conversation must not keep a record the resimulation has
    // not made. It is the one record of the step: no second decision is
    // made while it is present.
    registrar.rollback_component_clone_probed::<crate::body::TalkSpacing>(
        OWNER,
        "actor.talk_spacing",
        |spacing| {
            (u64::from(spacing.mark.to_bits()) << 32)
                ^ u64::from(spacing.walking.to_bits())
                ^ (u64::from(spacing.side.to_bits()) << 1)
        },
    );
    // The frame a held movement stick was pressed in: a rewind into a hold
    // must read the stick in the frame it was pressed in, or a resimulated
    // turn of gravity reads it the other way.
    registrar.rollback_component_clone_probed::<crate::avatar::systems::HeldControlFrame>(
        OWNER,
        "actor.held_control_frame",
        |hold| {
            let mode = match hold.mode {
                ambition_platformer2d_core::InputFrameMode::ScreenRelative => 1u64,
                ambition_platformer2d_core::InputFrameMode::BodyRelativeStrict => 2,
                ambition_platformer2d_core::InputFrameMode::BodyRelativeAssist => 3,
            };
            (u64::from(hold.down.x.to_bits()) << 32)
                ^ u64::from(hold.down.y.to_bits())
                ^ (u64::from(hold.raw.x.to_bits()) << 16)
                ^ (u64::from(hold.raw.y.to_bits()) << 8)
                ^ (u64::from(hold.slot.0) << 56)
                ^ (mode << 60)
        },
    );
    registrar.rollback_component_clone_probed::<crate::body::transform_beat::TransformBeat>(
        OWNER,
        "actor.transform_beat",
        |beat| beat.remaining.to_bits() as u64,
    );
    // Probed by value: the request carries the policy of the beat it asks for.
    registrar.rollback_component_clone_probed::<crate::body::transform_beat::TransformBeatRequested>(
        OWNER,
        "actor.transform_beat_requested",
        |request| {
            (u64::from(request.0.duration.to_bits()) << 32)
                | u64::from(request.0.clock_scale.to_bits())
        },
    );

    // An ARMED challenge, counting down to a fight.
    //
    // it was not rollback state, and it is the `SaveRestored` failure
    // in another domain. `tick_pending_challenges` REMOVES it in the sim
    // schedule; a rewind past that removal restored everything the removal
    // implied and left the removal itself standing, so the fight the narrative
    // armed was quietly disarmed by a rollback. The insert is a simulation
    // decision now (`arm_requested_challenges`), so it belongs in the snapshot
    // like every other simulation decision.
    registrar.rollback_component_clone_entity_set::<crate::body::PendingChallenge>(
        OWNER,
        "actor.pending_challenge",
        |pending| pending.challenger.into_iter().collect(),
    );
    registrar
        .rollback_map_entities::<crate::body::PendingChallenge>(OWNER, "map.pending_challenge");

    // What a body was sent to fetch, and how that ended. Probed by value: a
    // restore that brought back another item or another outcome is a mismatch.
    registrar.rollback_component_clone_probed::<crate::body::errand::Errand>(
        OWNER,
        "actor.errand",
        |errand| errand.probe(),
    );

    // The pickup's ATTRACTION POLICY rides the same entity as the pickup, so a
    // rewind that recreates a dropped coin has to recreate whether it comes to
    // you. Authored at spawn and never mutated — but "never mutated" is not
    // "never needs restoring" when the entity itself is rollback state.
    //
    // caught by `rollback_exit_oracle`'s PER-FRAME census within an hour of
    // the component existing, because a dropped coin is transient — spawned and
    // despawned inside the route — and the one-shot sweep in `rollback_coverage`
    // cannot see it. That is B3b's first blind spot, demonstrating itself.

    // Presence is authoritative because gameplay edge-triggers on this marker;
    // rollback must restore whether the actor was dormant, not merely re-derive it.
    registrar.rollback_component_clone::<crate::body::dormancy::Dormant>(
        OWNER,
        "actor.dormant",
    );

    // AN ENGINE COMPONENT IS REGISTERED ONCE, BY THE ENGINE. `Empowered`
    // lives in `features::empowerment`, and Mary-O and Sanic each registered it
    // from their own plugin — which is fine in a composition holding one demo
    // and a PANIC in the app, which holds both: bevy_ggrs refuses a second
    // `ComponentSnapshotPlugin` for one type ("plugin was already added"), and
    // 56 app tests died on that one line. Two games owning one engine type is
    // duplicate authority; the engine owns it.
    registrar.rollback_component_clone_probed::<crate::body::empowerment::Empowered>(
        OWNER,
        "feature.empowered",
        |empowered| {
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            // The REMAINING clock matters as much as the traits: an empowerment
            // restored with the wrong time left expires on a different frame,
            // and expiry is what flips invulnerability back off.
            empowered.remaining.map(f32::to_bits).hash(&mut hasher);
            empowered.traits.bits().hash(&mut hasher);
            hasher.finish()
        },
    );

    registrar.declare_rollback_derived_component::<crate::avatar::body_integration::PlayerBodyFrameOutput>(
        OWNER,
        "derived.player_body_frame_output",
        "republished by body integration every simulation frame",
    );

    registrar.declare_rollback_derived_resource::<crate::body::ActorDecisionFacts>(
        OWNER,
        "derived.actor_decision_facts",
        "rebuilt from authoritative body state before every autonomous decision phase",
    );
    registrar.declare_rollback_derived_resource::<crate::body::ActorDecisionFrames>(
        OWNER,
        "derived.actor_decision_frames",
        "rebuilt by autonomous decision and consumed by same-tick control publication",
    );
    registrar.declare_rollback_derived_resource::<crate::body::navigation::NavigationAdvice>(
        OWNER,
        "derived.navigation_advice",
        "rebuilt in the Observe phase from the room's surface graph, each body and its brain's goal",
    );
    registrar.declare_rollback_derived_resource::<crate::body::navigation::RoomNavigation>(
        OWNER,
        "derived.room_navigation",
        "a cache of surface graphs, each a pure function of a room's authored geometry and a body's tuning",
    );
    registrar.declare_rollback_derived_resource::<crate::body::ActorSteering>(
        OWNER,
        "derived.actor_steering",
        "rebuilt from the authoritative actor population before movement",
    );

    registrar
        .declare_rollback_derived_resource::<crate::body::perception::PerceptionPeers>(
            OWNER,
            "derived.perception_peers",
            "perception snapshot rebuilt every tick before brains read it",
        );
    registrar.declare_rollback_derived_resource::<crate::body::perception::PerceptionProjectiles>(
        OWNER,
        "derived.perception_projectiles",
        "perception snapshot rebuilt every tick before brains read it",
    );
    registrar
        .clear_message_on_rollback::<crate::body::BrainCommand>(OWNER, "message.brain_command");
    // What a conversation asked the simulation for, released by the
    // narrative ledger at the head of the tick it was stamped for. Cleared on
    // load for the same reason as every other released fact: the resimulated
    // tick is handed it again from the ledger rather than remembering it from
    // the branch that was abandoned.
    registrar.clear_message_on_rollback::<crate::body::ChallengeRequested>(
        OWNER,
        "message.challenge_requested",
    );
    registrar.clear_message_on_rollback::<crate::body::ReleaseProvocation>(
        OWNER,
        "message.release_provocation",
    );
    registrar.clear_message_on_rollback::<crate::body::PetRequested>(
        OWNER,
        "message.pet_requested");
    registrar.clear_message_on_rollback::<crate::body::WearRequested>(
        OWNER,
        "message.wear_requested",
    );
    // Written and recorded inside one tick; a resimulated tick re-announces
    // the transition that caused it.


    registrar.clear_message_on_rollback::<crate::avatar::trail::TrailContinuityBreak>(
        OWNER,
        "message.trail_continuity_break",
    );

    // ✔ THE PARALLEL PROJECTILE IS GONE (K2, 2026-09-02). `item.held_projectile`
    // stood here: `HeldProjectile` rode on `BodyKinematics` while its own
    // damage, range and splash were not rollback state, so a rewind restored
    // where a held shot WAS and kept what it would do when it landed. The row
    // was added as "registered, not endorsed", naming the real repair — fold
    // held shots into `ProjectileSpawnRequest` and delete the second projectile
    // simulation. That is done: a held item's shot is a `LiveProjectile` whose
    // `ProjectileGameplay` (registered by the projectile domain, splash
    // included) is the only in-flight state it has.

    // The puppy-slug gun's ally marker is `actor.summoned_companion` now,
    // registered by the combat domain beside `actor.summoned_to_the_fight`.

    // Item checkpoint baselines declare their rewind/checksum obligations beside
    // the item horizon that owns capture, restore and durable adoption.

}

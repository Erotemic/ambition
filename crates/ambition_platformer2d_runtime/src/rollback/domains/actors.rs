//! **The actors domain's rollback schema** (Campaign 2, R3).
//!
//! Actor state: features, rooms, the character runtime's seating and match bookkeeping, perception, time control, and the session reset's own record.
//!
//! ⚠ **relocation only.** The registrations were extracted mechanically and the
//! schema baseline verifies the result is byte-identical — a retyped call is
//! exactly the mistake that would slip through review and not through the
//! baseline.
//!
//! ⚠ the owner label stays `ambition_platformer2d_runtime` because this module is in it, and
//! must be: `ambition_platformer2d_actor_monolith` sits below the runtime in the crate graph. R1's
//! recorded decision is that this is the right shape for every domain below the
//! runtime; crates above it own their schemas directly.

use bevy::prelude::App;

use super::super::AmbitionRollbackApp;
// The byte-writer vocabulary these projections are built from.
use ambition_platformer2d_core::snapshot::{checksum_bytes, put_str, put_u64};
// Bespoke checksum projections these registrations name. They stayed beside the
// central function because the helpers predate the domain split; a projection
// used by exactly one domain should follow it, and that is a tidy rather than
// part of a relocation (R3: a move moves nothing else).

const OWNER: &str = "ambition_platformer2d_runtime";

/// Register everything the actors domain needs rewound.
pub(in crate::rollback) fn register(app: &mut App) {
    // ⭐ **THE INSTRUMENT'S OWN CHANNELS REWIND TOO** — and they were invisible
    // until the whole workspace was compiled with every feature at once. These
    // three exist only under `causal`, so the default job never sees them and
    // both rollback oracles reported green over channels that did not exist in
    // their build. The union graph (Front 1 of the test-cost campaign) is what
    // surfaced them, which is the argument for that job in one example.
    //
    // Clearing is the right answer rather than a waiver: a reader's cursor is
    // `Local` state GGRS never rewinds, so a recorder resuming after a load
    // would consume rows written in an abandoned future and print them as fact.
    // An instrument that reports a future that did not happen is worse than one
    // that reports nothing.
    #[cfg(feature = "causal")]
    {
        app.clear_message_on_rollback::<ambition_platformer2d_actor_monolith::causal::BodyMovementOps>(
            OWNER,
            "message.causal_body_movement_ops",
        );
        app.clear_message_on_rollback::<ambition_platformer2d_actor_monolith::features::ecs::damage_apply::BodyHitResolved>(
            OWNER,
            "message.causal_body_hit_resolved",
        );
        app.clear_message_on_rollback::<ambition_platformer2d_actor_monolith::features::ecs::damage_apply::BodyReactionApplied>(
            OWNER,
            "message.causal_body_reaction_applied",
        );
    }

    app.require_rollback::<ambition_platformer2d_actor_monolith::features::transform_beat::TransformBeat>(
        OWNER,
        "entity:transform_beat",
    );
    app.require_rollback::<ambition_platformer2d_actor_monolith::rooms::RoomSet>(
        OWNER,
        "root:room_set",
    );
    app.require_rollback::<ambition_platformer2d_actor_monolith::items::pickup::GroundItem>(
        OWNER,
        "entity:ground_item",
    );
    // ⚠ **a MOVING world item is the same kind of thing as a ground item, and
    // was not registered.** `spawn_moving_world_item` uses `spawn_room_scoped`,
    // and `RoomScopedEntity` is a LIFETIME marker — it says when the entity
    // dies with its room, and nothing about whether a rewind can reproduce it.
    // A block bonked on a mispredicted frame therefore left an item standing in
    // a future that was abandoned. (GPT review of 5cc4337..47d7de3, finding 1.)
    app.require_rollback::<ambition_platformer2d_actor_monolith::items::world_item::WorldItem>(
        OWNER,
        "entity:world_item",
    );
    app.require_rollback::<ambition_platformer2d_actor_monolith::gravity::GravityFlipSwitch>(
        OWNER,
        "entity:gravity_flip_switch",
    );
    // ⚠ **the heal shrine, for the same reason as the portal gun pickup**
    // (2026-08-06, K2b edit 2). It carries `SimId`, `SpawnOrigin` and
    // `TransactionId`, had no anchor, and so those registrations were inert on
    // it. Its own component is waived as authored geometry — the heal reads it
    // and never writes it — but the anchor is not about the shrine's data; it is
    // about whether GGRS reproduces the ENTITY on a resimulated timeline.
    app.require_rollback::<ambition_platformer2d_actor_monolith::shrine::HealShrine>(
        OWNER,
        "entity:heal_shrine",
    );
    app.rollback_component_clone_checksum::<ambition_platformer2d_actor_monolith::rooms::RoomSet>(
        OWNER,
        "root.room_set",
        "bevy_ggrs clone snapshot + active/start room identity checksum",
        room_set_checksum,
    );
    app.rollback_component_clone::<ambition_platformer2d_actor_monolith::rooms::ActiveRoomMetadata>(
        OWNER,
        "root.active_room_metadata",
    );
    app.rollback_component_clone_checksum::<ambition_platformer2d_ldtk::LdtkRuntimeIndex>(
        OWNER,
        "root.ldtk_runtime_index",
        "bevy_ggrs clone snapshot + active LDtk area checksum",
        ldtk_runtime_index_checksum,
    );
    app.rollback_component_clone::<ambition_platformer2d_actor_monolith::rooms::RoomMusicRequest>(
        OWNER,
        "root.room_music_request",
    );
    app.rollback_resource_optional_canonical::<ambition_platformer2d_actor_monolith::character_runtime::ActiveMatch>(
        OWNER,
        "resource.active_match",
    );
    app.rollback_resource_canonical::<ambition_platformer2d_actor_monolith::features::GameplayElapsed>(
        OWNER,
        "resource.gameplay_elapsed",
    );
    app.rollback_resource_canonical::<ambition_platformer2d_actor_monolith::time::time_control::RequestedClockScale>(
        OWNER,
        "resource.requested_clock_scale",
    );
    app.rollback_resource_canonical::<ambition_platformer2d_actor_monolith::time::time_control::RegimePolicy>(
        OWNER,
        "resource.clock_regime_policy",
    );
    app.rollback_resource_canonical::<ambition_platformer2d_actor_monolith::control::SlotInteractionState>(
        OWNER,
        "resource.slot_interaction_state",
    );
    app.rollback_resource_canonical::<ambition_platformer2d_actor_monolith::session::reset::NewGameResetRequested>(
        OWNER,
        "resource.sandbox_reset_requested",
    );
    app.rollback_resource_canonical::<ambition_platformer2d_actor_monolith::session::lifecycle_commit::PendingLifecycleCommit>(
            OWNER,
            "resource.pending_lifecycle_commit",
        );
    app.rollback_resource_canonical::<ambition_platformer2d_actor_monolith::RoomTransitionCooldown>(
        OWNER,
        "resource.sandbox_sim_state",
    );
    app.rollback_resource_clone_entity_set::<ambition_platformer2d_actor_monolith::abilities::traversal::possession::PossessionState>(
            OWNER,
            "resource.possession_state",
            |state| state.possessed.into_iter().chain(state.home).collect(),
        );
    app.rollback_resource_map_entities::<ambition_platformer2d_actor_monolith::abilities::traversal::possession::PossessionState>(
            OWNER,
            "map.resource.possession_state",
        );
    // **The conversation the simulation is having.**
    //
    // ⛔ registered because the continuity rules RESIMULATE: they run in the sim
    // schedule, which under a rollback host is the GGRS schedule. Before this
    // existed those rules read `ambition_dialog::DialogState`, which is not
    // rewound, and the hold they applied was half rollback state
    // (`ScriptedControl`) and half not (`HeldByConversation`) — so a rewind could
    // leave a body held by the dialogue's account and free by the simulation's.
    //
    // ⚠ **two registrations, exactly as `PossessionState` above.** The clone is
    // the snapshot; the entity map is what fixes the two body handles when
    // `LoadWorld` remaps. Registering only the first would restore a conversation
    // pointing at whatever those entity ids mean AFTER the load.
    // ⛔ **the entity set alone reported two DIVERGENT conversations as
    // identical.** It localizes through the two bodies' stable sim identities,
    // which is right for the half it covers and silent about the rest — and the
    // rest is authoritative: `input_owner` decides whose controls the box
    // captures, and the instance id decides which node runs and whether a
    // stamped narrative record applies at all. A peer disagreeing about any of
    // them looked like agreement.
    //
    // ⭐ **the instance id is hashed WHOLE rather than field by field**, so a new
    // ingredient of conversation identity joins the fingerprint by construction.
    // Listing its parts here was a second place to keep in step, and the kind
    // that goes quietly stale.
    //
    // ⭐ **which is what happened to the dialogue context, hashed separately here
    // until D29.** Hashing it was a correct reading of the DESYNC question and a
    // standing signal that the type whose whole job is identity did not carry it;
    // it does now, so the context arrives inside `live.instance` and the three
    // lines are gone rather than kept as a second opinion that can only agree.
    //
    // ⚠ **`input_owner` stays, and that is not the same judgement.** It is
    // deliberately NOT part of conversation identity — it publishes nothing into
    // Yarn and a correction re-derives it every tick — but a peer disagreeing
    // about which seats the box captures is resimulating a different game. This
    // probe asks "do two peers agree about the live conversation", which is
    // broader than "is this the same conversation".
    //
    // ⚠ **no raw entity numbers in the fingerprint.** Those differ across a load
    // by design, which is exactly why the entity half is probed through
    // identities. This is the complement, not a second answer to it.
    app.rollback_resource_clone_entity_set_probed::<ambition_platformer2d_actor_monolith::conversation::ActiveConversation>(
            OWNER,
            "resource.active_conversation",
            |conversation| conversation.referenced_entities(),
            |conversation| {
                use std::hash::{Hash, Hasher};
                let Some(live) = conversation.live() else {
                    // Distinct from a live conversation that hashes to nothing:
                    // "nobody is talking" is a state the probe must be able to
                    // name.
                    return 0;
                };
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                live.instance.hash(&mut hasher);
                match live.input_owner {
                    ambition_platformer2d_actor_monolith::conversation::ConversationInputOwner::Participant(id) => {
                        (1u8, id.slot()).hash(&mut hasher)
                    }
                    ambition_platformer2d_actor_monolith::conversation::ConversationInputOwner::Primary => {
                        (2u8, 0u8).hash(&mut hasher)
                    }
                    ambition_platformer2d_actor_monolith::conversation::ConversationInputOwner::AllParticipants => {
                        (3u8, 0u8).hash(&mut hasher)
                    }
                }
                // ⚠ `speaker_name` is deliberately absent: it is a DISPLAY
                // string, and a localization changing it is not a desync.
                hasher.finish()
            },
        );
    app.rollback_resource_map_entities::<ambition_platformer2d_actor_monolith::conversation::ActiveConversation>(
            OWNER,
            "map.resource.active_conversation",
        );
    // **The narrative end, as the conversation ledger RELEASES it.**
    //
    // ⛔ **clearing this message was once the whole bug, and clearing it is now
    // correct — because the producer changed.** It used to be written by
    // presentation watching the live Yarn runner, a system that does not execute
    // between resimulated ticks: clearing it on load dropped the end entirely,
    // and every replayed tick ran with a conversation the original timeline had
    // already finished. It is now written by `release_narrative_inputs`, a SIM
    // system, from a ledger that is deliberately not rollback state. So the
    // resimulated tick is handed the fact again rather than remembering it —
    // which is exactly what clearing a message on load is for.
    //
    // ⭐ **the durable record is the ledger; the message is its delivery.** The
    // ledger stays out of the schema for the same reason the device input stream
    // does: a rewind restores what the simulation decided, never what it was
    // told.
    app.clear_message_on_rollback::<
        ambition_platformer2d_actor_monolith::conversation::ConversationEnded,
    >(OWNER, "message.conversation_ended");
    // **The continuity → cast port.** The break rule asks for a bark; the cast
    // answers on the same tick. Cleared on load like every other in-tick sim
    // channel: a resimulated break asks again, and a request from a branch the
    // host abandoned must not reach the cast at all.
    app.clear_message_on_rollback::<
        ambition_platformer2d_actor_monolith::conversation::ConversationCutBark,
    >(OWNER, "message.conversation_cut_bark");
    app.rollback_resource_clone::<ambition_platformer2d_actor_monolith::encounter::SwitchActivationQueue>(
        OWNER,
        "resource.switch_activation_queue",
    );
    app.rollback_component_canonical::<ambition_platformer2d_actor_monolith::character_runtime::MatchSeat>(
        OWNER,
        "actor.match_seat",
    );
    app.rollback_component_cursor::<ambition_platformer2d_actor_monolith::features::ActorMotionPath>(
        OWNER,
        "actor.motion_path",
    );
    app.rollback_component_canonical::<ambition_platformer2d_actor_monolith::features::ActorStatus>(
        OWNER,
        "actor.status",
    );
    app.rollback_component_canonical::<ambition_platformer2d_actor_monolith::features::ecs::perception::Perception>(
        OWNER,
        "actor.perception",
    );
    app.rollback_component_canonical::<ambition_platformer2d_actor_monolith::features::ecs::perception::PerceptionMemory>(
            OWNER,
            "actor.perception_memory",
        );
    app.rollback_component_canonical::<ambition_platformer2d_actor_monolith::features::TemporaryControl>(
        OWNER,
        "actor.temporary_control",
    );
    app.rollback_component_canonical::<ambition_platformer2d_actor_monolith::features::ActorSurfaceState>(
        OWNER,
        "actor.surface_state",
    );
    app.rollback_component_cursor::<ambition_platformer2d_actor_monolith::features::BossEncounter>(
        OWNER,
        "boss.encounter",
    );
    app.rollback_component_canonical::<ambition_platformer2d_actor_monolith::avatar::PlayerSafetyState>(
        OWNER,
        "player.safety_state",
    );
    app.rollback_component_clone::<ambition_platformer2d_shared_tangle::camera_ease::PlayerBlinkCameraState>(
        OWNER,
        "player.blink_camera_state",
    );
    app.rollback_component_clone::<ambition_platformer2d_actor_monolith::control::LocalPlayer>(
        OWNER,
        "player.local_marker",
    );
    app.rollback_component_clone::<ambition_platformer2d_actor_monolith::features::ActorConfig>(
        OWNER,
        "actor.config",
    );
    app.rollback_component_clone_probed::<ambition_platformer2d_actor_monolith::features::transform_beat::TransformBeat>(
        OWNER,
        "actor.transform_beat",
        |beat| beat.remaining.to_bits() as u64,
    );
    app.rollback_component_clone::<ambition_platformer2d_actor_monolith::features::transform_beat::TransformBeatRequested>(
        OWNER,
        "actor.transform_beat_requested",
    );
    app.rollback_component_clone::<ambition_sprite_sheet::character::ActorAnimOverride>(
        OWNER,
        "actor.anim_override",
    );
    app.rollback_component_clone::<ambition_platformer2d_actor_monolith::features::BossConfig>(
        OWNER,
        "boss.config",
    );
    app.rollback_component_clone_entity_map::<ambition_platformer2d_actor_monolith::features::LimbRig>(
        OWNER,
        "limb.rig",
        |rig| {
            rig.limbs
                .iter()
                .map(|(slot, limb)| (*slot as u64, *limb))
                .collect()
        },
    );
    app.rollback_map_entities::<ambition_platformer2d_actor_monolith::features::LimbRig>(
        OWNER,
        "map.limb_rig",
    );
    app.rollback_component_clone_entity_ref::<ambition_platformer2d_actor_monolith::features::Limb>(
        OWNER,
        "limb.member",
        |limb| limb.of,
    );
    app.rollback_map_entities::<ambition_platformer2d_actor_monolith::features::Limb>(
        OWNER,
        "map.limb_member",
    );
    app.rollback_component_clone::<ambition_platformer2d_actor_monolith::features::LimbRouteState>(
        OWNER,
        "limb.route_state",
    );
    app.rollback_component_clone::<ambition_platformer2d_actor_monolith::features::LimbIntents>(
        OWNER,
        "limb.intents",
    );
    app.rollback_component_clone::<ambition_platformer2d_actor_monolith::features::CanPilot>(
        OWNER,
        "mount.can_pilot",
    );
    app.rollback_component_clone::<ambition_platformer2d_actor_monolith::features::Mass>(
        OWNER,
        "mount.mass",
    );
    app.rollback_component_clone_entity_set::<ambition_platformer2d_actor_monolith::features::MountSlot>(
        OWNER,
        "mount.slot",
        |slot| slot.rider.into_iter().collect(),
    );
    app.rollback_map_entities::<ambition_platformer2d_actor_monolith::features::MountSlot>(
        OWNER,
        "map.mount_slot",
    );
    app.rollback_component_clone::<ambition_platformer2d_actor_monolith::features::Mountable>(
        OWNER,
        "mount.mountable",
    );
    app.rollback_component_clone::<ambition_platformer2d_actor_monolith::features::Mounted>(
        OWNER,
        "mount.mounted",
    );
    app.rollback_component_clone_entity_ref::<ambition_platformer2d_actor_monolith::features::RidingOn>(
        OWNER,
        "mount.riding_on",
        |riding| riding.mount,
    );
    app.rollback_map_entities::<ambition_platformer2d_actor_monolith::features::RidingOn>(
        OWNER,
        "map.riding_on",
    );
    // **An ARMED challenge, counting down to a fight.**
    //
    // ⛔ **it was not rollback state, and it is the `SaveRestored` failure
    // in another domain.** `tick_pending_challenges` REMOVES it in the sim
    // schedule; a rewind past that removal restored everything the removal
    // implied and left the removal itself standing, so the fight the narrative
    // armed was quietly disarmed by a rollback. The insert is a simulation
    // decision now (`arm_requested_challenges`), so it belongs in the snapshot
    // like every other simulation decision.
    app.rollback_component_clone_entity_set::<ambition_platformer2d_actor_monolith::features::PendingChallenge>(
        OWNER,
        "actor.pending_challenge",
        |pending| pending.challenger.into_iter().collect(),
    );
    app.rollback_map_entities::<ambition_platformer2d_actor_monolith::features::PendingChallenge>(
        OWNER,
        "map.pending_challenge",
    );
    app.rollback_component_clone::<ambition_platformer2d_actor_monolith::features::BossOverrides>(
        OWNER,
        "boss.overrides",
    );
    app.rollback_component_clone::<ambition_platformer2d_actor_monolith::items::pickup::StashedActionSet>(
        OWNER,
        "actor.stashed_action_set",
    );
    app.rollback_component_clone_probed::<ambition_platformer2d_actor_monolith::avatar::PersonaBaseline>(
        OWNER,
        "actor.persona_baseline",
        |baseline| {
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            baseline.id.hash(&mut hasher);
            // The STANDING physicals are in the probe for the same reason
            // `ProjectedCharacterKit.granted` is: retraction is driven from them,
            // so a rewind that restores the record without them retracts a
            // replacement to the wrong numbers and nothing reads wrong until a
            // character swap. Mass is hashed by its bit pattern — this is a
            // checksum, not an arithmetic comparison, so `to_bits` is exactly
            // right and `f32`'s missing `Hash` is not an obstacle.
            baseline.displaced.max_health.hash(&mut hasher);
            baseline
                .displaced
                .mass
                .map(|mass| mass.map(f32::to_bits))
                .hash(&mut hasher);
            hasher.finish() ^ baseline.generation.get().rotate_left(32)
        },
    );
    app.rollback_component_clone_probed::<ambition_platformer2d_actor_monolith::character_runtime::ProjectedCharacterKit>(
        OWNER,
        "actor.projected_character_kit",
        |projected| {
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            projected.id.hash(&mut hasher);
            projected.granted.hurtboxes.hash(&mut hasher);
            projected.granted.movement_tuning.hash(&mut hasher);
            projected.granted.posed_body.hash(&mut hasher);
            hasher.finish() ^ projected.generation.get().rotate_left(32)
        },
    );
    app.rollback_component_clone_probed::<ambition_platformer2d_actor_monolith::character_runtime::BodyPoseClock>(
        OWNER,
        "actor.body_pose_clock",
        |clock| checksum_bytes(clock.pose.as_bytes()) ^ clock.elapsed_s.to_bits() as u64,
    );
    app.rollback_component_clone::<ambition_platformer2d_actor_monolith::character_runtime::AuthoredHurtboxes>(
        OWNER,
        "actor.authored_hurtboxes",
    );
    app.rollback_component_clone::<ambition_sprite_sheet::character::SpritePosedBody>(
        OWNER,
        "actor.sprite_posed_body",
    );
    app.rollback_component_clone_probed::<ambition_platformer2d_actor_monolith::encounter::SwitchOn>(
        OWNER,
        "feature.switch_on",
        |on| u64::from(on.0),
    );
    app.rollback_component_clone::<ambition_platformer2d_actor_monolith::encounter::SwitchFeature>(
        OWNER,
        "feature.switch",
    );
    app.rollback_component_clone::<ambition_platformer2d_actor_monolith::features::PickupCollectLock>(
        OWNER,
        "feature.pickup_collect_lock",
    );
    app.rollback_component_clone::<ambition_platformer2d_actor_monolith::features::PickupArt>(
        OWNER,
        "feature.pickup_art",
    );
    app.rollback_component_clone::<ambition_platformer2d_actor_monolith::items::pickup::GroundItem>(
        OWNER,
        "item.ground_item",
    );
    // ⚠ **CUSTODY IS SIMULATION STATE, not a cache.** It decides on every later
    // frame whether the item is drawn, stepped by `ground_item_physics`, and
    // grabbable — so a rewind that restored the wrong value leaves the same axe
    // both in a hand and on the floor, or makes a carried axe fall out of it.
    // It replaced a despawn/spawn pair, which GGRS reproduced through the
    // entity anchor; the state that took over the same job owes the same
    // coverage.
    //
    // `_entity_set` rather than `_entity_ref`: `InWorld` names no body at all,
    // so the handle is a zero-or-one set. The probe therefore measures WHICH
    // body is holding it through that body's stable identity — a restore that
    // hands the item to the wrong holder changes this census and would not
    // change a presence count.
    app.rollback_component_clone_entity_set::<ambition_platformer2d_actor_monolith::items::pickup::ItemCustody>(
        OWNER,
        "item.item_custody",
        |custody| match custody {
            ambition_platformer2d_actor_monolith::items::pickup::ItemCustody::InWorld => Vec::new(),
            ambition_platformer2d_actor_monolith::items::pickup::ItemCustody::Held { holder } => {
                vec![*holder]
            }
        },
    );
    app.rollback_map_entities::<ambition_platformer2d_actor_monolith::items::pickup::ItemCustody>(
        OWNER,
        "map.item_custody",
    );
    // The pickup's ATTRACTION POLICY rides the same entity as the pickup, so a
    // rewind that recreates a dropped coin has to recreate whether it comes to
    // you. Authored at spawn and never mutated — but "never mutated" is not
    // "never needs restoring" when the entity itself is rollback state.
    //
    // ⚠ caught by `rollback_exit_oracle`'s PER-FRAME census within an hour of
    // the component existing, because a dropped coin is transient — spawned and
    // despawned inside the route — and the one-shot sweep in `rollback_coverage`
    // cannot see it. That is B3b's first blind spot, demonstrating itself.
    app.rollback_component_clone_probed::<ambition_platformer2d_actor_monolith::features::ecs::pickups::PickupMagnet>(
        OWNER,
        "item.pickup_magnet",
        |magnet| {
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            magnet.range.to_bits().hash(&mut hasher);
            magnet.speed.to_bits().hash(&mut hasher);
            hasher.finish()
        },
    );
    // ⚠ PROBED, not merely cloned. A presence-only probe satisfies the coverage
    // sweep while seeing nothing of the value, so a restore that put the item
    // back at the wrong PLACE would checksum identical.
    // ⛔ this used to add *"and where a pickup is is the entire content of a
    // pickup"*, which was never true — a pickup is also WHAT IT GRANTS and, when
    // it moves, the PLAN it is following, and the probes below omitted both
    // (found in review, 2026-08-03). The sentence read as a justification for
    // stopping at position, which is exactly how a probe ends up narrower than
    // the value it certifies. (`rollback_exit_oracle` refuses a bare
    // presence probe by name, which is how these three were caught the hour they
    // were added.)
    // WHO SPAWNED IT — a marker, so presence IS the value. A rewind that
    // recreates a dropped coin must recreate the fact that the attempt produced
    // it, or a later reset leaves loot standing that should have gone with the
    // attempt. (Flagged by the coverage sweep the same run the marker landed.)
    // ⛔ **WHO IS ASLEEP IS ROLLBACK STATE, because falling asleep has an EDGE.**
    // `Dormant`'s own doc said "derived every tick; never authored, never
    // persisted", and that was true right up until dormancy started RETRACTING
    // the brain's last intent on the transition — `ActorControl` is restored by a
    // rewind, and the marker that decides whether the retraction fires was not.
    //
    // So a rewind across the moment an actor fell asleep restored a body that was
    // already marked dormant, the transition did not re-fire, the retraction never
    // happened, and the resimulated timeline drove an actor the original had
    // stopped. `mary_o_it` reported it as a sync-test checksum mismatch at frames
    // [2, 3, 4] and a `git bisect` over 160 commits named the dormancy commit —
    // after seven one-at-a-time reverts had failed to.
    //
    // ⭐ the general rule this is an instance of: **a derived marker stops being
    // derived the moment something EDGE-TRIGGERS off it.** "Recomputed every tick"
    // is only safe while nothing remembers the previous tick's answer.
    app.rollback_component_clone::<ambition_platformer2d_actor_monolith::features::ecs::dormancy::Dormant>(
        OWNER,
        "actor.dormant",
    );
    // ⛔ **the POLICY beside the marker, and its absence was invisible until a
    // third game used it.** `Dormant` was registered here the day dormancy
    // landed; `DormancyPolicy` was not, because nothing in any `app_it` room
    // carried one — Mary-O and Sanic declare theirs in their own demo binaries,
    // which the coverage sweep does not visit. The moment `ambition_content`
    // declared stances for its bosses and cast, seven sweep populations went red
    // at once and named it.
    //
    // ⚠ it is content's DECLARATION, written once at spawn and never mutated by
    // simulation — so a waiver could be argued. Registering is cheaper than the
    // argument: it is a `Copy` enum, and the alternative is a waiver whose
    // reasoning has to be re-checked every time the tagger's schedule moves.
    // ⭐ this is the lesson the comment above already records, one level out: a
    // family is not covered when its principal member is.
    // ⚠ **PROBED, not presence-only, and a second guard insisted.** Registering
    // it with `rollback_component_clone` satisfied the coverage sweep and the
    // exit oracle immediately refused it: *"a presence probe satisfies the
    // coverage test above while seeing nothing of the value"*. That is exactly
    // right here — the value IS a radius, and a rewind that restored the
    // component's presence while losing its distance would put a different world
    // to sleep. Two guards, one shallower than the other, and the deeper one
    // caught a registration that looked complete.
    app.rollback_component_clone_probed::<ambition_platformer2d_actor_monolith::features::ecs::dormancy::DormancyPolicy>(
        OWNER,
        "actor.dormancy_policy",
        |policy| {
            use ambition_platformer2d_actor_monolith::features::ecs::dormancy::DormancyPolicy;
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            match policy {
                DormancyPolicy::Never => 0u8.hash(&mut hasher),
                DormancyPolicy::AwakeNearObservers { radius } => {
                    1u8.hash(&mut hasher);
                    // Bit pattern: this is a checksum, not an arithmetic
                    // comparison.
                    radius.to_bits().hash(&mut hasher);
                }
            }
            hasher.finish()
        },
    );
    app.rollback_component_clone::<ambition_platformer2d_actor_monolith::features::ecs::SpawnedThisAttempt>(
        OWNER,
        "lifecycle.spawned_this_attempt",
    );
    app.rollback_component_clone_probed::<ambition_platformer2d_actor_monolith::items::world_item::WorldItem>(
        OWNER,
        "item.world_item",
        |item| {
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            // Bit patterns: this is a checksum, not an arithmetic comparison.
            item.pos.x.to_bits().hash(&mut hasher);
            item.pos.y.to_bits().hash(&mut hasher);
            item.half_extent.x.to_bits().hash(&mut hasher);
            item.half_extent.y.to_bits().hash(&mut hasher);
            item.sprite.hash(&mut hasher);
            // WHAT COLLECTING IT DOES, which the box and the sprite do not say.
            // Two pickups can sit in the same place looking the same and equip
            // different rows, and only this tells them apart.
            //
            // ⚠ the ROW IDENTITY, not the row's authored numbers. `EquipmentRow`
            // carries modifiers, grants and an on-hit rule, and those are
            // CONTENT — read from the same files, identical for a given id in
            // one build, and therefore incapable of differing between two
            // timelines of one session. The id and the exclusive slot are what
            // a divergent spawn would change.
            match &item.payload {
                ambition_platformer2d_actor_monolith::items::world_item::WorldItemPayload::Equip(
                    row,
                ) => {
                    row.id.hash(&mut hasher);
                    row.exclusive_slot.hash(&mut hasher);
                }
            }
            hasher.finish()
        },
    );
    // ⛔ **AN ENGINE COMPONENT IS REGISTERED ONCE, BY THE ENGINE.** `Empowered`
    // lives in `features::empowerment`, and Mary-O and Sanic each registered it
    // from their own plugin — which is fine in a composition holding one demo
    // and a PANIC in the app, which holds both: bevy_ggrs refuses a second
    // `ComponentSnapshotPlugin` for one type ("plugin was already added"), and
    // 56 app tests died on that one line. Two games owning one engine type is
    // duplicate authority; the engine owns it.
    app.rollback_component_clone_probed::<ambition_platformer2d_actor_monolith::features::empowerment::Empowered>(
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
    // The motion PLAN and its cursor travel together — `ItemMotion`'s own doc
    // says a cursor without its plan is meaningless — so one registration
    // restores both halves of where the pickup is in its arc.
    app.rollback_component_clone_probed::<ambition_platformer2d_actor_monolith::items::item_motion::ItemMotion>(
        OWNER,
        "item.motion",
        |motion| {
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            // WHERE IT IS IN ITS ARC: how far the rise has got, how fast it is
            // travelling, and which way it turned last.
            motion.emerged_for().to_bits().hash(&mut hasher);
            motion.velocity().x.to_bits().hash(&mut hasher);
            motion.velocity().y.to_bits().hash(&mut hasher);
            motion.facing().to_bits().hash(&mut hasher);
            // AND THE PLAN IT IS FOLLOWING. The cursor above is meaningless
            // without it — `emerged_for` is read against `emerge.seconds`, and
            // the same cursor under a different plan is a different pickup
            // mid-arc. The plan is authored and does not normally change, which
            // is exactly why a timeline where it DID is worth catching.
            let plan = &motion.plan;
            plan.emerge.map(|e| (e.distance.to_bits(), e.seconds.to_bits())).hash(&mut hasher);
            plan.speed.to_bits().hash(&mut hasher);
            plan.facing.to_bits().hash(&mut hasher);
            plan.gravity.to_bits().hash(&mut hasher);
            plan.bounce.to_bits().hash(&mut hasher);
            plan.turns_at_walls.hash(&mut hasher);
            hasher.finish()
        },
    );
    app.rollback_component_clone::<ambition_platformer2d_actor_monolith::gravity::GravityFlipSwitch>(
        OWNER,
        "gravity.flip_switch",
    );
    app.rollback_component_clone::<ambition_platformer2d_actor_monolith::boss_encounter::EncounterDef>(
        OWNER,
        "encounter.definition",
    );
    app.rollback_component_clone::<ambition_platformer2d_actor_monolith::features::MountedBrainCache>(
        OWNER,
        "mount.brain_cache",
    );
    app.rollback_component_clone::<ambition_platformer2d_actor_monolith::features::MountedSize>(
        OWNER,
        "mount.authored_size",
    );
    app.declare_rollback_derived_component::<ambition_platformer2d_actor_monolith::avatar::body_integration::PlayerBodyFrameOutput>(
        OWNER,
        "derived.player_body_frame_output",
        "republished by body integration every simulation frame",
    );
    app.declare_rollback_derived_component::<ambition_platformer2d_actor_monolith::body_mode::BodyModeCapabilities>(
        OWNER,
        "derived.body_mode_capabilities",
        "projected from the active body mode each frame",
    );
    app.declare_rollback_derived_component::<ambition_platformer2d_actor_monolith::control::PlayerInputFrame>(
        OWNER,
        "derived.player_input_frame",
        "copied from GGRS PlayerInputs at the head of every frame",
    );
    app.declare_rollback_derived_component::<ambition_platformer2d_actor_monolith::character_runtime::ResolvedHurtboxes>(
        OWNER,
        "derived.resolved_hurtboxes",
        "recomputed from AuthoredHurtboxes plus the move and pose clocks each tick",
    );
    app.declare_rollback_derived_resource::<ambition_platformer2d_actor_monolith::features::ActorSteering>(
        OWNER,
        "derived.actor_steering",
        "rebuilt from the authoritative actor population before movement",
    );
    app.rollback_component_cursor::<ambition_platformer2d_actor_monolith::boss_encounter::sprites::BossAnimFrame>(
        OWNER,
        "component.boss_anim_frame",
    );
    app.declare_rollback_derived_component::<ambition_platformer2d_actor_monolith::features::BossAnimationFrameSample>(
        OWNER,
        "derived.boss_animation_frame_sample",
        "republished every tick by drive_boss_animators from the rewound BossAnimFrame cursor",
    );
    app.declare_rollback_derived_component::<ambition_platformer2d_actor_monolith::boss_encounter::EncounterProgress>(
        OWNER,
        "derived.encounter_progress",
        "recomputed from lifecycle and participant health every tick",
    );
    app.declare_rollback_derived_resource::<ambition_platformer2d_actor_monolith::features::ecs::perception::PerceptionPeers>(
        OWNER,
        "derived.perception_peers",
        "perception snapshot rebuilt every tick before brains read it",
    );
    app.declare_rollback_derived_resource::<ambition_platformer2d_actor_monolith::features::ecs::perception::PerceptionProjectiles>(
        OWNER,
        "derived.perception_projectiles",
        "perception snapshot rebuilt every tick before brains read it",
    );
    app.declare_rollback_derived_resource::<ambition_platformer2d_actor_monolith::encounter::EncounterSwitchIndex>(
        OWNER,
        "derived.encounter_switch_index",
        "rebuilt from SwitchFeature + SwitchOn components each frame",
    );
    app.declare_rollback_derived_resource::<ambition_platformer2d_actor_monolith::affordances::PlayerAffordances>(
        OWNER,
        "derived.player_affordances",
        "affordance read model recomputed per frame from body state",
    );
    app.declare_rollback_derived_resource::<ambition_platformer2d_actor_monolith::affordances::intent::PlayerIntent>(
        OWNER,
        "derived.player_intent",
        "affordance read model recomputed per frame from control input",
    );
    app.declare_rollback_derived_resource::<ambition_platformer2d_actor_monolith::affordances::interactable_proximity::NearestInteractable>(
        OWNER,
        "derived.nearest_interactable",
        "proximity read model recomputed per frame",
    );
    app.declare_rollback_derived_resource::<ambition_platformer2d_actor_monolith::affordances::pogo_proximity::PogoTargetBelow>(
        OWNER,
        "derived.pogo_target_below",
        "proximity read model recomputed per frame",
    );
    app.clear_message_on_rollback::<ambition_platformer2d_actor_monolith::features::BrainCommand>(
        OWNER,
        "message.brain_command",
    );
    // **What a conversation asked the simulation for**, released by the
    // narrative ledger at the head of the tick it was stamped for. Cleared on
    // load for the same reason as every other released fact: the resimulated
    // tick is handed it again from the ledger rather than remembering it from
    // the branch that was abandoned.
    app.clear_message_on_rollback::<ambition_platformer2d_actor_monolith::features::ChallengeRequested>(
        OWNER,
        "message.challenge_requested",
    );
    app.clear_message_on_rollback::<ambition_platformer2d_actor_monolith::features::ReleaseProvocation>(
        OWNER,
        "message.release_provocation",
    );
    app.clear_message_on_rollback::<ambition_platformer2d_actor_monolith::features::SpawnActorRequest>(
        OWNER,
        "message.spawn_actor_request",
    );
    app.clear_message_on_rollback::<ambition_platformer2d_actor_monolith::ActorDiedMessage>(
        OWNER,
        "message.actor_died",
    );
    app.clear_message_on_rollback::<ambition_platformer2d_actor_monolith::session::reset::NewGameResetCommitted>(
        OWNER,
        "message.sandbox_reset_committed",
    );
    app.clear_message_on_rollback::<ambition_platformer2d_actor_monolith::features::ecs::damage_apply::WalletShieldSpent>(
        OWNER,
        "message.wallet_shield_spent",
    );
    app.clear_message_on_rollback::<ambition_platformer2d_actor_monolith::avatar::PlayerHealRequested>(
        OWNER,
        "message.player_heal_requested",
    );
    app.clear_message_on_rollback::<ambition_platformer2d_actor_monolith::avatar::trail::TrailContinuityBreak>(
        OWNER,
        "message.trail_continuity_break",
    );
    app.clear_message_on_rollback::<ambition_platformer2d_actor_monolith::boss_encounter::PayloadReleased>(
        OWNER,
        "message.payload_released",
    );
    app.clear_message_on_rollback::<ambition_platformer2d_actor_monolith::encounter::SwitchActivated>(
        OWNER,
        "message.switch_activated",
    );
    app.clear_message_on_rollback::<ambition_platformer2d_actor_monolith::features::MountDied>(
        OWNER,
        "message.mount_died",
    );
    // P0.2: the boss phase-transition edge, same shape as `MountDied` directly
    // above — announced by `update_boss_encounters` in `BossAdvance` and
    // consumed by `boss_phase_transition_feedback` in `BossHazards`, a same-frame
    // handshake inside one sim schedule. A cursor GGRS did not rewind could let
    // the feedback fire for a transition the resimulation never committed to,
    // which here means a `DamageBox` shockwave on a timeline the boss never
    // phased on.
    app.clear_message_on_rollback::<ambition_platformer2d_actor_monolith::boss_encounter::BossPhaseChanged>(
        OWNER,
        "message.boss_phase_changed",
    );
    app.clear_message_on_rollback::<ambition_platformer2d_actor_monolith::session::reset::RoomReplayRequested>(
        OWNER,
        "message.room_replay_requested",
    );
    app.clear_message_on_rollback::<ambition_platformer2d_actor_monolith::time::time_control::ClockResetRequest>(
        OWNER,
        "message.clock_reset_request",
    );
    app.clear_message_on_rollback::<ambition_platformer2d_actor_monolith::time::time_control::ClockScaleRequest>(
        OWNER,
        "message.clock_scale_request",
    );
    app.clear_message_on_rollback::<ambition_platformer2d_actor_monolith::encounter::SwitchActivated>(
        OWNER,
        "message.switch_activated",
    );
    app.clear_message_on_rollback::<ambition_platformer2d_actor_monolith::features::MountDied>(
        OWNER,
        "message.mount_died",
    );
    app.clear_message_on_rollback::<ambition_platformer2d_actor_monolith::session::reset::RoomReplayRequested>(
        OWNER,
        "message.room_replay_requested",
    );
    app.clear_message_on_rollback::<ambition_platformer2d_actor_monolith::time::time_control::ClockResetRequest>(
        OWNER,
        "message.clock_reset_request",
    );
    app.clear_message_on_rollback::<ambition_platformer2d_actor_monolith::time::time_control::ClockScaleRequest>(
        OWNER,
        "message.clock_scale_request",
    );
}

fn room_set_checksum(rooms: &ambition_platformer2d_actor_monolith::rooms::RoomSet) -> u64 {
    let mut bytes = Vec::new();
    put_u64(&mut bytes, rooms.active as u64);
    put_u64(&mut bytes, rooms.start as u64);
    put_str(&mut bytes, &rooms.active_spec().id);
    checksum_bytes(&bytes)
}

fn ldtk_runtime_index_checksum(
    index: &ambition_platformer2d_ldtk::LdtkRuntimeIndex,
) -> u64 {
    checksum_bytes(index.active_area().as_bytes())
}

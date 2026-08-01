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
    app.require_rollback::<ambition_platformer2d_actor_monolith::features::transform_beat::TransformBeat>(
        OWNER,
        "entity:transform_beat",
    );
    app.require_rollback::<ambition_platformer2d_actor_monolith::rooms::RoomSet>(OWNER, "root:room_set");
    app.require_rollback::<ambition_platformer2d_actor_monolith::items::pickup::GroundItem>(OWNER, "entity:ground_item");
    app.require_rollback::<ambition_platformer2d_actor_monolith::gravity::GravityFlipSwitch>(
        OWNER,
        "entity:gravity_flip_switch",
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
    app.rollback_component_clone_checksum::<ambition_platformer2d_actor_monolith::ldtk_world::LdtkRuntimeIndex>(
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
    app.rollback_resource_canonical::<ambition_platformer2d_actor_monolith::session::reset::SandboxResetRequested>(
        OWNER,
        "resource.sandbox_reset_requested",
    );
    app.rollback_resource_canonical::<ambition_platformer2d_actor_monolith::session::lifecycle_commit::PendingLifecycleCommit>(
            OWNER,
            "resource.pending_lifecycle_commit",
        );
    app.rollback_resource_canonical::<ambition_platformer2d_actor_monolith::SandboxSimState>(
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
    app.rollback_component_clone::<ambition_platformer2d_actor_monolith::avatar::PlayerBlinkCameraState>(
        OWNER,
        "player.blink_camera_state",
    );
    app.rollback_component_clone::<ambition_platformer2d_actor_monolith::control::LocalPlayer>(
        OWNER,
        "player.local_marker",
    );
    app.rollback_component_clone::<ambition_platformer2d_actor_monolith::features::ActorConfig>(OWNER, "actor.config");
    app.rollback_component_clone_probed::<ambition_platformer2d_actor_monolith::features::transform_beat::TransformBeat>(
        OWNER,
        "actor.transform_beat",
        |beat| beat.remaining.to_bits() as u64 ^ u64::from(beat.was_invulnerable) << 32,
    );
    app.rollback_component_clone::<ambition_platformer2d_actor_monolith::features::transform_beat::TransformBeatRequested>(
        OWNER,
        "actor.transform_beat_requested",
    );
    app.rollback_component_clone::<ambition_platformer2d_actor_monolith::features::ActorAnimOverride>(
        OWNER,
        "actor.anim_override",
    );
    app.rollback_component_clone::<ambition_platformer2d_actor_monolith::features::BossConfig>(OWNER, "boss.config");
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
    app.rollback_map_entities::<ambition_platformer2d_actor_monolith::features::LimbRig>(OWNER, "map.limb_rig");
    app.rollback_component_clone_entity_ref::<ambition_platformer2d_actor_monolith::features::Limb>(
        OWNER,
        "limb.member",
        |limb| limb.of,
    );
    app.rollback_map_entities::<ambition_platformer2d_actor_monolith::features::Limb>(OWNER, "map.limb_member");
    app.rollback_component_clone::<ambition_platformer2d_actor_monolith::features::LimbRouteState>(
        OWNER,
        "limb.route_state",
    );
    app.rollback_component_clone::<ambition_platformer2d_actor_monolith::features::LimbIntents>(OWNER, "limb.intents");
    app.rollback_component_clone::<ambition_platformer2d_actor_monolith::features::CanPilot>(OWNER, "mount.can_pilot");
    app.rollback_component_clone::<ambition_platformer2d_actor_monolith::features::Mass>(OWNER, "mount.mass");
    app.rollback_component_clone_entity_set::<ambition_platformer2d_actor_monolith::features::MountSlot>(
        OWNER,
        "mount.slot",
        |slot| slot.rider.into_iter().collect(),
    );
    app.rollback_map_entities::<ambition_platformer2d_actor_monolith::features::MountSlot>(OWNER, "map.mount_slot");
    app.rollback_component_clone::<ambition_platformer2d_actor_monolith::features::Mountable>(OWNER, "mount.mountable");
    app.rollback_component_clone::<ambition_platformer2d_actor_monolith::features::Mounted>(OWNER, "mount.mounted");
    app.rollback_component_clone_entity_ref::<ambition_platformer2d_actor_monolith::features::RidingOn>(
        OWNER,
        "mount.riding_on",
        |riding| riding.mount,
    );
    app.rollback_map_entities::<ambition_platformer2d_actor_monolith::features::RidingOn>(OWNER, "map.riding_on");
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
    app.rollback_component_clone::<ambition_platformer2d_actor_monolith::character_sprites::SpritePosedBody>(
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
    app.clear_message_on_rollback::<ambition_platformer2d_actor_monolith::features::ReleaseProvocation>(
        OWNER,
        "message.release_provocation",
    );
    app.clear_message_on_rollback::<ambition_platformer2d_actor_monolith::features::SpawnActorRequest>(
        OWNER,
        "message.spawn_actor_request",
    );
    app.clear_message_on_rollback::<ambition_platformer2d_actor_monolith::ActorDiedMessage>(OWNER, "message.actor_died");
    app.clear_message_on_rollback::<ambition_platformer2d_actor_monolith::session::reset::SandboxResetCommitted>(
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

fn ldtk_runtime_index_checksum(index: &ambition_platformer2d_actor_monolith::ldtk_world::LdtkRuntimeIndex) -> u64 {
    checksum_bytes(index.active_area().as_bytes())
}

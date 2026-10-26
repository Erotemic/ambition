//! The body, control, and action kernel.
//!
//! Domain owners supply construction, sessions, items, and world policy.
//! This crate executes accepted control on live bodies. Runtime composes
//! domain schedules and rollback declarations.

// External API surface — bins, tests, and Android/wasm entry points reach
// into these modules. Everything else stays `pub(crate)` so the compiler
// can tell us what's actually depended on from outside.
/// Body integration and capabilities for controlled bodies.
pub mod avatar;
pub mod brain_tick;
#[cfg(feature = "causal")]
pub mod causal;
#[cfg(test)]
mod character_roster;
/// The local control seam: device frame -> slot -> the body carrying that slot's
/// player brain. See `control/mod.rs`.
pub mod control;

pub mod abilities;
pub mod body_custody;
pub mod body_mode;
#[cfg(test)]
mod character_integration_tests;
pub mod character_projection;
/// Swings and shots arbitrated in ONE contest, before either damage road runs.
pub mod clash;
#[cfg(test)]
pub mod enemy_projectile;
/// The `ParticipantId` ↔ `PlayerSlot` correspondence, in one place, so the
/// eventual split of "a person" from "a seat" is localized.
pub mod participant_seat;
pub mod repertoire;

// Public re-exports double as the external API for bins, tests, and docs.
mod features;

// Keep generic time vocabulary in `ambition_time`; export only the local bridge.

#[cfg(test)]
mod clock_discipline_tests;
#[cfg(test)]
mod safe_pos_tests;

// Domain-owned rollback declaration; the host supplies the backend registrar.
mod rollback_registration;
pub use rollback_registration::register_rollback_state;

pub use features::{
    ActorSteering, BodyIdentityCensus, BodySimulationPlugin, BrainCommand, BrainCommandKind,
    BrainCommandPlugin, CHALLENGE_GRACE_S, ChallengeRequested, PendingChallenge, PetRequested,
    ReleaseProvocation, WearRequested, advance_body_anim_overlay_clocks,
    apply_actor_contact_damage, apply_actor_stimuli, apply_brain_commands,
    apply_feature_hit_events, apply_release_provocations, apply_summon_effects,
    arm_ground_contact_anim_overlay, arm_movement_anim_overlays, arm_requested_challenges,
    brain_command, derive_pogo_target_volumes, emit_movement_fx, empowerment, handle_player_events,
    install_body_identity_census, install_dismounted_rider_rebuild, integrate_boss_bodies,
    integrate_sim_bodies, interact_ecs_actors_and_switches, ledge_grab, movement_fx,
    npc_talk_dialogue_id, refresh_body_damageable_volumes, refresh_boss_damageable_volumes,
    route_boss_strikes_to_limbs, snapshot_body_contact, spawn_projectiles_from_brain_actions,
    speak_conversation_cut_barks, swim, tick_actor_brains, tick_npc_idle_barks,
    tick_pending_challenges, track_body_life_stats, transform_beat, wallet_shield,
};

pub use features::ecs::{
    FADE_ROW_WRITER, PET_SECONDS, PET_WALK_SPEED, PetBeat, PetStage, REST_ROW_WRITER, TALK_GAP,
    TalkSpacing, TalkableBodies, advance_pet_beats, apply_pet_requests, apply_wear_requests,
    death_beat, dormancy, errand, fighter_ladder, follow_drawn_ledges, navigation, perception,
    pin_fade_rows, pin_rest_rows, project_pet_holds, space_the_talkers, withdraw_faded_contact,
};

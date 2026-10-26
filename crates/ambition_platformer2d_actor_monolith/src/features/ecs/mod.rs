//! Body decisions, contact effects, and action execution.

use super::*;
use ambition_characters::actor::BodyCombat;
use bevy::prelude::{Commands, Entity, MessageWriter, Query, Res, ResMut, With, Without};

use ambition_time::WorldTime;

pub(crate) mod actors;
mod aggression;
pub mod anim_helpers;
pub mod body_identity;
mod boss_bodies;
#[cfg(test)]
mod boss_scripted_pattern_tests;
pub(crate) use ambition_platformer2d_actor_spawn::brain_builders::enemy_default_brain;
/// The ladder projection, registered in the actor pipeline beside the brain tick.
/// The dismount reaction: mount announces, this rebuilds. See its own note.
pub(crate) mod autonomous_reconcile;
mod brain_effects;
pub(crate) mod character_policy;
mod damage;
pub mod death_beat;
mod death_effects;
pub mod dismounted_rider;
pub mod dormancy;
mod drawn_ledges;
pub mod errand;
mod fade_rows;
#[cfg(test)]
mod fighter_harness;
pub mod fighter_ladder;
mod interact;
pub mod navigation;
pub mod perception;
mod pet;
mod rest_row;
mod summon;
mod talk_spacing;
mod target_volumes;
mod wear;

pub(crate) use actors::{
    ActorDecisionFacts, ActorDecisionFrames, maintain_actor_pre_decision_state,
    observe_actor_decision_inputs, publish_actor_decision_frames,
};

pub use actors::{
    ActorSteering, apply_actor_contact_damage, integrate_sim_bodies, route_boss_strikes_to_limbs,
    snapshot_body_contact, tick_actor_brains, tick_npc_idle_barks,
};
pub use aggression::{
    CHALLENGE_GRACE_S, ChallengeRequested, PendingChallenge, apply_actor_stimuli,
    arm_requested_challenges, tick_pending_challenges,
};
use ambition_platformer2d_core::movement::MotionModel;
pub use boss_bodies::integrate_boss_bodies;
pub use brain_effects::spawn_projectiles_from_brain_actions;
pub use damage::apply_feature_hit_events;
pub use drawn_ledges::follow_drawn_ledges;
pub use fade_rows::{FADE_ROW_WRITER, pin_fade_rows, withdraw_faded_contact};
pub use interact::{TalkableBodies, interact_ecs_actors_and_switches};
pub use pet::{
    PET_SECONDS, PET_WALK_SPEED, PetBeat, PetRequested, PetStage, advance_pet_beats,
    apply_pet_requests, project_pet_holds,
};
pub use rest_row::{REST_ROW_WRITER, pin_rest_rows};
pub use talk_spacing::{TALK_GAP, TalkSpacing, space_the_talkers};
pub use wear::{WearRequested, apply_wear_requests};
#[cfg(test)]
mod mount_pair_tests;

pub use summon::apply_summon_effects;
pub use target_volumes::{
    derive_pogo_target_volumes, refresh_body_damageable_volumes, refresh_boss_damageable_volumes,
};

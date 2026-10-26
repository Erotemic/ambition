//! ECS backbone of the actor / world-object simulation.
//!
//! Despite the `features/` name, this is NOT a set of toggleable feature
//! slices — it is the enemy / NPC / boss ACTOR SIMULATION plus the authored
//! room objects they share a world with (pickups, chests, breakables,
//! switches, hazards, mounts). Every one is a Bevy entity spawned and ticked
//! here; this is the authoritative implementation.
//!
//! Module map (each sibling owns one slice of that sim):
//! - `spawn*` — spawn authored room objects, encounter mobs, mounts/riders;
//! - `actors` / `actor_clusters` / `bosses` — the per-frame actor tick over the
//!   unified ECS cluster components that hold actor state (NPCs + enemies share
//!   one cluster; bosses are their own);
//! - `damage*` / `aggression` / `interact` — hit routing, provocation, and
//!   player interactions;
//! - `encounter_rewards` / `reset` — reward chests, room reset,
//!   and save-state mirroring;
//! - `view_index` / `anim_helpers` / `target_volumes` — per-frame read models
//!   and overlays consumed by presentation, engine, and combat code.
//!
//! Facade: many `ecs::<name>` paths re-export from the reusable
//! `combat` kit (`banner`, `breakables`, `chests`, `hazards`,
//! `hitbox`, `overlay`, `pickups`, ...) so call sites stay
//! stable while the generic mechanics live DOWN in that kit (ADR 0019).

use super::*;
use ambition_combat::{banner, hazards, held_items, hitbox, targeting};
// `BodyCombat`/`BodyHealth` live on the reusable actor crate. This module surfaces them to the
// `ecs/` submodules that name `super:BodyCombat` — the `super:*` glob no longer carries them
// since the `features` facade stopped re-exporting the shared body vocabulary.
use ambition_characters::actor::BodyCombat;
use ambition_platformer2d_shared_tangle::lifecycle::RoomVisual;
#[cfg(test)]
use ambition_vfx::vfx::VfxInRoom;
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
pub mod fighter_ladder;
// ⛔ `damage_apply` LEFT FOR `crates/ambition_damage`, 2026-08-26. It named no
// monolith type by the time it went — five re-export facades had been hiding
// what it actually depended on — so the carve was a move rather than a design.
pub mod dormancy;
pub mod effect_bus;
pub mod errand;
mod fade_rows;
#[cfg(test)]
mod fighter_harness;
mod interact;
pub mod navigation;
pub mod perception;
mod pet;
mod rest_row;
pub(crate) mod spawn;
pub mod spawn_static;
mod summon;
mod talk_spacing;
mod target_volumes;
mod wear;

pub(crate) use actors::{
    ActorDecisionFacts, ActorDecisionFrames, maintain_actor_pre_decision_state,
    observe_actor_decision_inputs, publish_actor_decision_frames,
};

pub use actors::{
    ActorSteering, AxisSweptMotion, MomentumMotion, MotionModel, apply_actor_contact_damage, integrate_sim_bodies, route_boss_strikes_to_limbs,
    snapshot_body_contact,
    tick_actor_brains, tick_npc_idle_barks,
};
pub use aggression::{
    CHALLENGE_GRACE_S, ChallengeRequested, PendingChallenge, apply_actor_stimuli, arm_requested_challenges, tick_pending_challenges,
};
pub use ambition_boss_encounter::anim::{
    boss_anim_state_for, ecs_boss_anim_state, ecs_boss_anim_state_and_entity,
    ecs_boss_animation_frame_sample,
};
pub use banner::{apply_gameplay_banner_requests, tick_gameplay_banner};
// `boss_component_snapshot` is pub: the observation-boundary contract tests
// (ambition_sim_view) build boss read-model components from a scratch boss.
pub mod world_overlay;
pub use world_overlay::rebuild_feature_ecs_world_overlay;
// ⭐ THE BOSS ECS MODULE LIVES IN `ambition_boss_encounter` NOW, beside the boss
// profiles, catalog and anim helpers it was already calling. These stay as the
// monolith's façade rows so its schedule registration and its callers did not
// have to move with it; the definitions are one hop away and named as such.
pub use ambition_boss_encounter::ecs::boss_component_snapshot;
pub use ambition_boss_encounter::ecs::{
    boss_spawn_hurtboxes, drive_boss_animators,
    project_boss_attack_state_from_move, tick_boss_brains_system, trigger_boss_attack_moves, update_ecs_bosses,
};
// ⛔ EXCEPT THIS ONE, which did not go: see `boss_bodies`.
pub use boss_bodies::integrate_boss_bodies;
pub use brain_effects::spawn_projectiles_from_brain_actions;
pub use damage::apply_feature_hit_events;
pub use fade_rows::{FADE_ROW_WRITER, pin_fade_rows, withdraw_faded_contact};
pub use hazards::{HazardTickSet, advance_hazards, apply_hazard_contacts};
pub use held_items::HeldItem;
pub use hitbox::{
    Hitbox, HitboxAnchor, HitboxHits,
    HitboxKnockback, HitboxLifetime, apply_hitbox_damage,
    tick_and_despawn_hitboxes,
};
pub use interact::{TalkableBodies, interact_ecs_actors_and_switches};
pub use pet::{
    PET_SECONDS, PET_WALK_SPEED, PetBeat, PetRequested, PetStage, advance_pet_beats,
    apply_pet_requests, project_pet_holds,
};
pub use rest_row::{REST_ROW_WRITER, pin_rest_rows};
pub use talk_spacing::{TALK_GAP, TalkSpacing, space_the_talkers};
pub use wear::{WearRequested, apply_wear_requests};
// ⭐ THE MOUNT PAIR'S TESTS STAYED, because their fixtures are this crate's
// construction road. They exercise `ambition_mount` from the composition.
#[cfg(test)]
mod mount_pair_tests;

// ⛔ THE MOUNT PAIR LEFT THIS CRATE (D33, 2026-08-26) and is NOT re-exported.
// `ambition_mount` owns it; a `pub use` here would let every caller keep
// spelling it `ambition_mount::MountSlot` and hide whose domain it is — the same rule
// `Mass`, `MountDied` and `ControlClaims` each moved under.
pub use spawn::{
    ActorConstructionContext,
    OccurrenceContinuity, RoomContentStagingError, RoomContentStagingRegistrationError,
    RoomContentStagingRegistry, RoomFeatureConstructionError, RoomFeatureConstructionPlan, serve_encounter_spawn_commands, spawn_encounter_mob,
};
// ⛔ The room construction boundary and the receipt it produces are consumed
// only by the room coordinator (`ambition_platformer2d_publication`), which
// spawns a plan inside a publication transaction and verifies the receipt.
// The absence contract `only-the-room-coordinator-spawns-a-room-plan` keeps
// every other crate off it.
pub use spawn::{RoomFeatureConstructionReceipt, spawn_room_feature_entities_from_plan};
// ⭐ THE SUMMON ROAD IS RE-EXPORTED FROM ITS OWN MODULE, and the path callers use
// is unchanged (`features::apply_summon_effects`) — the schedule in
// `ambition_platformer2d_runtime` and an ordering in `ambition_demo_smash` both
// name it, so a move that renamed the path would be a carve with a blast radius.
pub use summon::apply_summon_effects;
// ⛔ THE SPAWN-PRIMITIVE RE-EXPORTS THAT USED TO LIVE HERE ARE GONE, AND THEIR
// DEATH IS THE EVIDENCE THE INVERSION IS REAL. They existed so `construction`
// could reach these names through `crate::features`; now that it names
// the extracted `ambition_platformer2d_actor_spawn` crate directly, nothing in this module needs them and the
// compiler said so. A carve that leaves its shims behind has only moved files.
pub use target_volumes::{
    derive_pogo_target_volumes, refresh_body_damageable_volumes, refresh_boss_damageable_volumes,
};
pub use targeting::{
    FactionRelations, FriendlyFire, can_damage, damage_lands, dissolve_settled_grudges, select_actor_targets,
};

#[cfg(test)]
mod tests;

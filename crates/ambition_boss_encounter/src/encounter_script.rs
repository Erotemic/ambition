//! Encounter-script execution and its actor-specific mechanics.
//!
//! The generic timeline vocabulary ([`EncounterGate`], [`EncounterTrigger`],
//! [`EncounterEffect`], [`EncounterBeat`], [`EncounterScript`]) and the generic
//! beat advance (`EncounterScript::advance`) live in `ambition_encounter`, the
//! one timeline authority. This module owns what touches actor bodies: it
//! executes each script's advanced effects (defeat a member, command a
//! member's brain, drop a hazard, banner, music), plus the two generic
//! mechanics an effect spawns: [`CommandedMove`] (a "walk the boss to a spot"
//! brain override) and [`FallingHazard`] (a "hang, wait for alignment, fall,
//! fire the impact gate" hazard). Member indices address the encounter's
//! generic [`EncounterParticipants`].
//!
//! The cut-rope fight is a script: `Gate("rope_cut")` →
//! [`EncounterEffect::CommandMoveTo`] (lure the behemoth under the drop) +
//! [`EncounterEffect::DropHazard`] (a [`FallingHazard`]) → `ForceKill`. The
//! swallowed-NPC release comes from the generic
//! [`ReleaseOnDeath`](super::encounter_entity::ReleaseOnDeath).

use bevy::prelude::*;

use crate::{BossClusterRef, BossEncounter};
use ambition_combat::CenteredAabb;
use ambition_encounter::{EncounterEffect, EncounterGate, EncounterParticipants, EncounterScript};
use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::AabbExt;
/// A scripted encounter beat's claim on the priority music tier.
pub const SCRIPT_MUSIC_OWNER: &str = "encounter_script";

use ambition_platformer2d_shared_tangle::lifecycle::{
    SessionScopedEntity, SessionSpawnScope, SpawnSessionScopedExt,
};

/// Advance every encounter script and execute the effects it yields this
/// tick. The trigger evaluation and cursor logic are generic
/// (`EncounterScript::advance`, reading fired gates and participant deadness);
/// this system supplies the actor-touching execution. Runs in the Progression
/// set after `update_encounter_progress` (which refreshes participant
/// `alive`).
pub fn tick_encounter_scripts(
    mut commands: Commands,
    world_time: Res<ambition_time::WorldTime>,
    mut gates: MessageReader<EncounterGate>,
    // The encounter's own identity and counter: what it drops is minted under
    // it, so two hazards of one script are two identified objects.
    mut scripts: Query<(
        &EncounterParticipants,
        &mut EncounterScript,
        Option<&ambition_platformer2d_shared_tangle::sim_id::SimId>,
        Option<&mut ambition_platformer2d_shared_tangle::sim_id::SimIdCounter>,
    )>,
    mut members: Query<(
        &mut BossEncounter,
        &mut ambition_characters::actor::BodyHealth,
    )>,
    session_owners: Query<&SessionScopedEntity>,
    mut banner: ResMut<ambition_combat::GameplayBanner>,
    mut music: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldMut<
        ambition_encounter::EncounterMusicRequest,
    >,
) {
    let dt = world_time.sim_dt();
    let fired: Vec<String> = gates.read().map(|g| g.gate.clone()).collect();

    // No script means no script music. `SetMusic` is an effect, fired once
    // when a beat reaches it, so `release_priority` is reached only when a
    // live script emits `SetMusic(None)`. An encounter that ends without that
    // beat, or despawns when the player leaves its room, would keep its claim,
    // and the priority tier outranks room music. So with no scripts alive,
    // release the claim here (as the boss-music system does). It is scoped to
    // this owner, so a conversation cue or the boss owner keep theirs.
    //
    // No shipped encounter authors `EncounterEffect::SetMusic` yet; this keeps
    // the capability correct.
    //
    // Limitation: this is correct only while at most one script is live.
    // `SCRIPT_MUSIC_OWNER` is one `&'static str` for every `EncounterScript`,
    // and `priority_owner` is `Option<&'static str>`, so two live scripts
    // overwrite each other's track (last writer wins), and a script that ends
    // while another lives leaves its claim in place. No content has two
    // concurrent scripts today. The fix is per-script ownership keyed by the
    // durable `encounter_id` (widening `priority_owner` for all owners) or one
    // deterministic aggregator. Do not key it by ECS `Entity`, which does not
    // survive a rewind. Add a two-script test with the fix.
    if scripts.is_empty() {
        music.release_priority(SCRIPT_MUSIC_OWNER);
    }

    for (participants, mut script, encounter_id, mut counter) in &mut scripts {
        let effects = script.advance(dt, participants, &fired);
        let member_entity = |i: usize| participants.members.get(i).and_then(|p| p.entity);
        for effect in &effects {
            match effect {
                EncounterEffect::ForceKill(i) => {
                    if let Some(m) = member_entity(*i) {
                        if let Ok((mut status, mut health)) = members.get_mut(m) {
                            health.health.current = 0;
                            if let Some(phase) = status.encounter.as_mut() {
                                let _ = phase.kill();
                            }
                        }
                    }
                }
                EncounterEffect::Banner { text, secs } => banner.show(text.clone(), *secs),
                EncounterEffect::SetMusic(track) => match track {
                    Some(track) => music.claim_priority(SCRIPT_MUSIC_OWNER, track.clone()),
                    None => music.release_priority(SCRIPT_MUSIC_OWNER),
                },
                EncounterEffect::CommandMoveTo {
                    member,
                    target,
                    speed,
                    arrive_tolerance,
                } => {
                    if let Some(m) = member_entity(*member) {
                        commands.entity(m).insert(CommandedMove {
                            target: *target,
                            speed: *speed,
                            arrive_tolerance: *arrive_tolerance,
                        });
                    }
                }
                EncounterEffect::DropHazard {
                    anchor,
                    size,
                    gravity,
                    terminal,
                    align_tolerance,
                    target_member,
                    impact_gate,
                } => {
                    if let Some(target) = member_entity(*target_member) {
                        // Refuse, and do not drop a hazard without an identity
                        // (ADR 0030). A hazard the sim cannot name cannot be
                        // rebuilt by a rewind, and this one decides when a
                        // boss takes an impact.
                        let (Some(encounter), Some(counter)) =
                            (encounter_id, counter.as_deref_mut())
                        else {
                            bevy::prelude::warn!(
                                "an encounter hazard drop was refused: the \
                                 encounter carries no SimId or no SimIdCounter, \
                                 so the hazard could not be named"
                            );
                            continue;
                        };
                        let sim_id = Some(
                            ambition_platformer2d_shared_tangle::sim_id::SimId::spawned(
                                encounter,
                                counter.next(),
                            ),
                        );
                        drop_hazard(
                            &mut commands,
                            SessionSpawnScope::new(
                                session_owners.get(target).ok().map(|owner| owner.0),
                            ),
                            *anchor,
                            FallingHazard {
                                size: *size,
                                gravity: *gravity,
                                terminal: *terminal,
                                align_tolerance: *align_tolerance,
                                target,
                                impact_gate: impact_gate.clone(),
                                vel_y: 0.0,
                                dropping: false,
                            },
                            sim_id,
                        );
                    }
                }
            }
        }
    }
}

/// Drop one hazard. This is the only way a falling hazard enters the world.
///
/// One seam, so a test can bring the real entity into a booted world. An
/// archetype that exists only after an encounter beat is not reachable by a
/// census of a booted room.
///
/// `vel_y`, `dropping` and the target entity are the fall, and the hazard
/// decides when a boss takes an impact, so they are rollback state. The
/// declaration (anchor and codec) is in this crate's
/// `register_rollback_state`.
///
/// `sim_id` is the hazard's identity: `SimId::spawned(encounter, counter)`
/// minted under the encounter that dropped it. `None` only for a fixture
/// without a spawner; production always identifies what it drops, because a
/// rollback-anchored entity with no identity rewinds anonymously.
pub fn drop_hazard(
    commands: &mut Commands,
    scope: SessionSpawnScope,
    anchor: ae::Vec2,
    hazard: FallingHazard,
    sim_id: Option<ambition_platformer2d_shared_tangle::sim_id::SimId>,
) -> Entity {
    let size = hazard.size;
    let mut spawned = commands.spawn_session_scoped(
        scope,
        (CenteredAabb::from_center_size(anchor, size), hazard),
    );
    if let Some(sim_id) = sim_id {
        spawned.insert(sim_id);
    }
    spawned.id()
}

/// Generic "lured movement" override: while present on a boss, its brain
/// control is replaced by steering toward `target.x` at `speed` (stopping
/// within `arrive_tolerance`). Attached by [`EncounterEffect::CommandMoveTo`];
/// the encounter removes it (for example when the member dies or the script
/// ends). Reusable by any "walk the boss to a spot" beat.
#[derive(Component, Clone, Copy, Debug)]
pub struct CommandedMove {
    pub target: ae::Vec2,
    pub speed: f32,
    pub arrive_tolerance: f32,
}

/// Steer every [`CommandedMove`] boss toward its target, overriding the
/// brain's `ActorControl` and clearing its attack intent, so no new move
/// starts and a windup in progress is interrupted; a committed strike runs
/// out. Runs in the boss steer slot (between brain tick and body integrate).
pub fn tick_commanded_moves(
    mut bosses: Query<(
        BossClusterRef,
        &ambition_characters::actor::BodyHealth,
        &mut ambition_characters::control::ActorControl,
        &mut ambition_characters::brain::BossAttackIntent,
        &CommandedMove,
    )>,
) {
    for (feature, health, mut control, mut attack_intent, cmd) in &mut bosses {
        let boss = feature.as_boss_ref();
        if !health.alive() {
            continue;
        }
        let dx = cmd.target.x - boss.kin.pos.x;
        attack_intent.clear();
        control.0.melee_pressed = false;
        control.0.special_pressed = false;
        // Turn toward the mark, and not again once there: an arrived body
        // that overshoots by a pixel does not spin round.
        control.0.facing = ambition_characters::brain::face_toward(boss.kin.facing, dx, cmd.arrive_tolerance);
        // `dx` is a world-x difference, so the command is world-space.
        control.0.velocity_target = if dx.abs() <= cmd.arrive_tolerance {
            ae::WorldVec2::ZERO
        } else {
            ae::WorldVec2::new(dx.signum() * cmd.speed, 0.0)
        };
    }
}

/// A generic hazard that hangs at its spawn point until its `target` is
/// aligned under it (within `align_tolerance` in x), then falls under
/// `gravity` (capped at `terminal`), fires `EncounterGate(impact_gate)` on
/// contact with the target, and despawns. The cut-rope anvil/piano is one.
#[derive(Component, Clone, Debug)]
pub struct FallingHazard {
    pub size: ae::Vec2,
    pub gravity: f32,
    pub terminal: f32,
    pub align_tolerance: f32,
    pub target: Entity,
    pub impact_gate: String,
    pub vel_y: f32,
    pub dropping: bool,
}

/// The `target` is an entity, so it must be remapped on restore. A rewind
/// rebuilds the world's entities; a raw id restored as-is points at whatever
/// is in that slot now.
impl bevy::ecs::entity::MapEntities for FallingHazard {
    fn map_entities<M: bevy::ecs::entity::EntityMapper>(&mut self, mapper: &mut M) {
        self.target = mapper.get_mapped(self.target);
    }
}

/// Integrate every [`FallingHazard`]: wait for the target to align, then fall,
/// clamp to the floor, and fire the impact gate on contact. Despawns the
/// hazard on impact (or if its target left the world).
pub fn tick_falling_hazards(
    mut commands: Commands,
    world_time: Res<ambition_time::WorldTime>,
    world: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
        ambition_platformer2d_core::RoomGeometry,
    >,
    mut gates: MessageWriter<EncounterGate>,
    mut hazards: Query<(Entity, &mut CenteredAabb, &mut FallingHazard)>,
    targets: Query<&CenteredAabb, Without<FallingHazard>>,
) {
    let dt = world_time.sim_dt().max(0.0);
    for (entity, mut aabb, mut hazard) in &mut hazards {
        let Ok(target) = targets.get(hazard.target) else {
            // Target gone (room change / despawn) — retire the hazard.
            commands.entity(entity).despawn();
            continue;
        };
        if !hazard.dropping {
            if (target.center.x - aabb.center.x).abs() <= hazard.align_tolerance {
                hazard.dropping = true;
            } else {
                continue;
            }
        }
        hazard.vel_y = (hazard.vel_y + hazard.gravity * dt).min(hazard.terminal);
        aabb.center.y += hazard.vel_y * dt;
        let floor_y = world.0.size.y - hazard.size.y * 0.5;
        if aabb.center.y > floor_y {
            aabb.center.y = floor_y;
            hazard.vel_y = 0.0;
        }
        if aabb.aabb().strict_intersects(target.aabb()) {
            gates.write(EncounterGate::new(hazard.impact_gate.clone()));
            commands.entity(entity).despawn();
        }
    }
}

#[cfg(test)]
mod tests;

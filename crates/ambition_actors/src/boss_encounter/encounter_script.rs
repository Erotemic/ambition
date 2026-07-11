//! Encounter-script EXECUTION + its actor-specific mechanics.
//!
//! The generic timeline vocabulary — [`EncounterGate`], [`EncounterTrigger`],
//! [`EncounterEffect`], [`EncounterBeat`], [`EncounterScript`] — and the generic
//! beat-advance (`EncounterScript::advance`) live in `ambition_encounter` (the
//! one timeline authority). This module owns only what TOUCHES actor bodies: it
//! reads each script's `advance`d effects and EXECUTES them (defeat a member,
//! command a member's brain, drop a hazard, banner, music), plus the two generic
//! mechanics an effect spawns — [`CommandedMove`] (a "walk the boss to a spot"
//! brain override) and [`FallingHazard`] (a "hang, wait for alignment, fall,
//! fire the impact gate" hazard). Member indices address the encounter's generic
//! [`EncounterParticipants`].
//!
//! The cut-rope fight is expressed entirely as a script: `Gate("rope_cut")` →
//! [`EncounterEffect::CommandMoveTo`] (lure the behemoth under the drop) +
//! [`EncounterEffect::DropHazard`] (a [`FallingHazard`]) → `ForceKill`. The
//! swallowed-NPC release falls out of the generic
//! [`ReleaseOnDeath`](super::encounter_entity::ReleaseOnDeath).

use bevy::prelude::*;

use crate::features::ecs::boss_clusters::{BossClusterRef, BossEncounter};
use crate::features::CenteredAabb;
use ambition_encounter::{EncounterEffect, EncounterGate, EncounterParticipants, EncounterScript};
use ambition_engine_core as ae;
use ambition_engine_core::AabbExt;

/// Advance every encounter script and EXECUTE the effects it yields this tick.
/// The trigger evaluation + cursor logic is generic (`EncounterScript::advance`,
/// reading fired gates + participant deadness); this system supplies the
/// actor-touching execution. Runs in the Progression set after
/// `update_encounter_progress` (which refreshes participant `alive`).
pub fn tick_encounter_scripts(
    mut commands: Commands,
    world_time: Res<ambition_time::WorldTime>,
    mut gates: MessageReader<EncounterGate>,
    mut scripts: Query<(&EncounterParticipants, &mut EncounterScript)>,
    mut members: Query<(
        &mut BossEncounter,
        &mut ambition_characters::actor::BodyHealth,
    )>,
    mut banner: ResMut<crate::features::GameplayBanner>,
    mut music: ResMut<crate::encounter::EncounterMusicRequest>,
) {
    let dt = world_time.sim_dt();
    let fired: Vec<String> = gates.read().map(|g| g.gate.clone()).collect();

    for (participants, mut script) in &mut scripts {
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
                EncounterEffect::SetMusic(track) => music.priority_track = track.clone(),
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
                        commands.spawn((
                            CenteredAabb::from_center_size(*anchor, *size),
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
                        ));
                    }
                }
            }
        }
    }
}

/// Generic "lured movement" override: while present on a boss, its brain control
/// is overridden to steer toward `target.x` at `speed` (stopping within
/// `arrive_tolerance`). Attached by [`EncounterEffect::CommandMoveTo`]; the
/// encounter removes it (e.g. the member dies / the script ends). Reusable by
/// any "walk the boss to a spot" beat.
#[derive(Component, Clone, Copy, Debug)]
pub struct CommandedMove {
    pub target: ae::Vec2,
    pub speed: f32,
    pub arrive_tolerance: f32,
}

/// Steer every [`CommandedMove`] boss toward its target, overriding the brain's
/// `ActorControl` (and clearing its attack intent). Runs in the boss steer slot
/// (between the brain tick and the body integrate).
pub fn tick_commanded_moves(
    mut bosses: Query<(
        BossClusterRef,
        &ambition_characters::actor::BodyHealth,
        &mut ambition_characters::brain::ActorControl,
        &mut ambition_characters::brain::BossAttackState,
        &CommandedMove,
    )>,
) {
    for (feature, health, mut control, mut attack_state, cmd) in &mut bosses {
        let boss = feature.as_boss_ref();
        if !health.alive() {
            continue;
        }
        let dx = cmd.target.x - boss.kin.pos.x;
        attack_state.clear();
        control.0.melee_pressed = false;
        control.0.special_pressed = false;
        control.0.facing = if dx.abs() > 2.0 {
            dx.signum()
        } else {
            boss.kin.facing
        };
        control.0.velocity_target = if dx.abs() <= cmd.arrive_tolerance {
            ae::Vec2::ZERO
        } else {
            ae::Vec2::new(dx.signum() * cmd.speed, 0.0)
        };
    }
}

/// A generic hazard that hangs at its spawn point until its `target` is aligned
/// under it (within `align_tolerance` in x), then falls under `gravity` (capped
/// at `terminal`) and fires `EncounterGate(impact_gate)` on contact with the
/// target — then despawns. The cut-rope anvil/piano is one of these.
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

/// Integrate every [`FallingHazard`]: wait for the target to align, then fall +
/// clamp to the floor + fire the impact gate on contact. Despawns the hazard on
/// impact (or if its target left the world).
pub fn tick_falling_hazards(
    mut commands: Commands,
    world_time: Res<ambition_time::WorldTime>,
    world: Res<ambition_engine_core::RoomGeometry>,
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

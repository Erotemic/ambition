//! Generic ENCOUNTER SCRIPT — ordered beats `{ when: Trigger, then: [Effect] }`
//! that advance as triggers fire.
//!
//! An encounter entity ([`EncounterDef`]) may carry an [`EncounterScript`] for
//! its bespoke scripted beats (a cut-rope puzzle, a timed add-wave, a gauntlet
//! cue). This is the encounter's OWN mechanism, parallel to — and sharing a
//! vocabulary with — the entity-local phase triggers ([`PhaseTrigger`]), but a
//! distinct concern: phase triggers are the boss's intrinsic self-progression;
//! the script is the orchestration the encounter layers on top.
//!
//! Triggers OBSERVE the world / members; effects COMMAND members / world. The
//! cut-rope fight is expressed ENTIRELY as a script: `Gate("rope_cut")` →
//! [`EncounterEffect::CommandMoveTo`] (lure the behemoth under the drop) +
//! [`EncounterEffect::DropHazard`] (a generic [`FallingHazard`] that hangs,
//! waits for alignment, falls, and fires its impact gate) → `ForceKill`. The
//! lure + falling-hazard are GENERIC mechanics ([`CommandedMove`] /
//! [`FallingHazard`]) any future "stand-under-the-thing" puzzle reuses; cut-rope
//! owns only its rope-cut detection + flavor visuals. The swallowed-NPC release
//! falls out of the generic [`ReleaseOnDeath`](super::encounter_entity::ReleaseOnDeath).
//!
//! See `docs/planning/boss-entity-local-refactor.md` (encounter-script shape).
//!
//! [`PhaseTrigger`]: crate::boss_encounter::PhaseTrigger
//! [`EncounterDef`]: super::encounter_entity::EncounterDef

use bevy::prelude::*;

use super::encounter_entity::EncounterDef;
use crate::features::ecs::boss_clusters::{BossClusterRef, BossEncounter};
use crate::features::CenteredAabb;
use ambition_engine_core as ae;
use ambition_engine_core::AabbExt;

/// A named gate fired by gameplay code (rope cut, hazard impact, cutscene cue,
/// "all adds dead") to advance an [`EncounterScript`] beat waiting on it. The
/// external / scripted hook — the same role `PhaseTrigger::External` plays for
/// intrinsic phases.
#[derive(Message, Clone, Debug, PartialEq, Eq)]
pub struct EncounterGate {
    pub gate: String,
}

impl EncounterGate {
    pub fn new(gate: impl Into<String>) -> Self {
        Self { gate: gate.into() }
    }
}

/// A condition that advances the current script beat. Observes world / members.
#[derive(Clone, Debug, PartialEq)]
pub enum EncounterTrigger {
    /// An external [`EncounterGate`] with this name fired this tick.
    Gate(String),
    /// The Nth member (by `EncounterDef.members` index) is dead (or gone).
    MemberDied(usize),
    /// Every member is dead (or gone).
    AllMembersDead,
    /// `secs` elapsed since this beat became current.
    Timer(f32),
}

/// A command the script issues when its beat's trigger fires. Commands members
/// / world.
#[derive(Clone, Debug, PartialEq)]
pub enum EncounterEffect {
    /// Force the Nth member straight to `Death` (an environmental kill that
    /// bypasses `environmental_kill_only`).
    ForceKill(usize),
    /// Show a gameplay banner for `secs` seconds.
    Banner { text: String, secs: f32 },
    /// Set (`Some`) or clear (`None`) the adaptive-music request.
    SetMusic(Option<String>),
    /// Lure the Nth member toward `target.x` at `speed` (stopping within
    /// `arrive_tolerance`) by overriding its brain control — attaches a generic
    /// [`CommandedMove`]. The cut-rope behemoth is lured under the anvil this way.
    CommandMoveTo {
        member: usize,
        target: ae::Vec2,
        speed: f32,
        arrive_tolerance: f32,
    },
    /// Spawn a generic [`FallingHazard`] hanging at `anchor`: it waits until the
    /// `target_member` is within `align_tolerance.x`, then falls under `gravity`
    /// (capped at `terminal`) and fires `EncounterGate(impact_gate)` on contact.
    DropHazard {
        anchor: ae::Vec2,
        size: ae::Vec2,
        gravity: f32,
        terminal: f32,
        align_tolerance: f32,
        target_member: usize,
        impact_gate: String,
    },
}

/// One scripted beat: when `when` fires, apply `then` and advance the cursor.
#[derive(Clone, Debug, PartialEq)]
pub struct EncounterBeat {
    pub when: EncounterTrigger,
    pub then: Vec<EncounterEffect>,
}

impl EncounterBeat {
    pub fn new(when: EncounterTrigger, then: Vec<EncounterEffect>) -> Self {
        Self { when, then }
    }
}

/// An ordered beat sequence attached to an encounter entity. Advances one beat
/// per fired trigger; inert once the cursor passes the last beat.
#[derive(Component, Clone, Debug, Default)]
pub struct EncounterScript {
    pub beats: Vec<EncounterBeat>,
    cursor: usize,
    /// Seconds in the current beat (for `Timer`).
    elapsed: f32,
}

impl EncounterScript {
    pub fn new(beats: Vec<EncounterBeat>) -> Self {
        Self {
            beats,
            cursor: 0,
            elapsed: 0.0,
        }
    }

    /// True once every beat has fired.
    pub fn done(&self) -> bool {
        self.cursor >= self.beats.len()
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }
}

/// Advance every encounter script: check the current beat's trigger against this
/// tick's fired gates + member state, and on a match apply the effects and step
/// the cursor. Runs in the Progression set after the encounter entity exists.
pub fn tick_encounter_scripts(
    mut commands: Commands,
    world_time: Res<ambition_time::WorldTime>,
    mut gates: MessageReader<EncounterGate>,
    mut scripts: Query<(&EncounterDef, &mut EncounterScript)>,
    mut members: Query<(
        &mut BossEncounter,
        &mut ambition_characters::actor::BodyHealth,
    )>,
    mut banner: ResMut<crate::features::GameplayBanner>,
    mut music: ResMut<crate::encounter::EncounterMusicRequest>,
) {
    let dt = world_time.sim_dt();
    let fired: Vec<String> = gates.read().map(|g| g.gate.clone()).collect();

    for (def, mut script) in &mut scripts {
        if script.done() {
            continue;
        }
        script.elapsed += dt;
        let beat = &script.beats[script.cursor];
        // A member is "dead" if its status is non-alive or it has left the world.
        let triggered = match &beat.when {
            EncounterTrigger::Gate(g) => fired.iter().any(|f| f == g),
            EncounterTrigger::MemberDied(i) => def
                .members
                .get(*i)
                .map_or(true, |&m| members.get(m).map_or(true, |(_, h)| !h.alive())),
            EncounterTrigger::AllMembersDead => {
                !def.members.is_empty()
                    && def
                        .members
                        .iter()
                        .all(|&m| members.get(m).map_or(true, |(_, h)| !h.alive()))
            }
            EncounterTrigger::Timer(secs) => script.elapsed >= *secs,
        };
        if !triggered {
            continue;
        }
        let effects = beat.then.clone();
        for effect in &effects {
            match effect {
                EncounterEffect::ForceKill(i) => {
                    if let Some(&m) = def.members.get(*i) {
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
                    if let Some(&m) = def.members.get(*member) {
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
                    if let Some(&target) = def.members.get(*target_member) {
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
        script.cursor += 1;
        script.elapsed = 0.0;
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

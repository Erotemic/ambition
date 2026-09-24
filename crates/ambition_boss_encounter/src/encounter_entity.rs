//! The encounter as a first-class, optional entity.
//!
//! An entity instance is one spawned creature (HP, phase, payload). An
//! encounter is the optional orchestration around one or more member
//! creatures: a progress model derived from member state, a HUD binding, lock
//! walls, win/lose conditions, music, and a scripted timeline.
//!
//! A boss spawned with no encounter is a tough enemy: no HUD, no lock walls,
//! no win/lose (fine for headless / RL). The encounter never gates the
//! creature's intrinsic phase-up (that is the entity-local [`ActorPhaseState`]);
//! it only frames and displays the fight and adds external/scripted triggers.
//!
//! See `docs/systems/boss-encounter-architecture.md`.

use std::collections::HashSet;

use bevy::prelude::*;

use crate::BossEncounterPhase;
use crate::{BossConfig, BossEncounter};
use ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity;
use ambition_encounter::{
    Encounter, EncounterCommand, EncounterCommandKind, EncounterLifecycle, EncounterObjective,
    EncounterParticipant, EncounterParticipants, EncounterRole, Objective,
};
use ambition_platformer2d_shared_tangle::lifecycle::{
    SessionScopedEntity, SessionSpawnScope, SpawnSessionScopedExt,
};

/// Definition of an encounter entity: its stable identity and how it frames
/// its members. Optional: a creature with no `EncounterDef` is simply not
/// orchestrated.
///
/// Membership is the generic [`EncounterParticipants`] component and the win
/// condition the generic [`EncounterObjective`] component (both on the same
/// entity), shared with wave arenas.
#[derive(Component, Clone, Debug)]
pub struct EncounterDef {
    /// Whether this encounter binds the HUD (a view of its progress). `false`
    /// / no encounter  no boss HUD.
    pub hud: bool,
}

/// Live, member-derived progress of an encounter, recomputed every frame by
/// [`update_encounter_progress`]. The HUD is a view of this; nothing in the
/// sim depends on it.
#[derive(Component, Clone, Debug, Default)]
pub struct EncounterProgress {
    /// One entry per resolvable member, in [`EncounterParticipants`] order.
    pub members: Vec<MemberProgress>,
    /// Mirror of the generic lifecycle's `Completed` phase. The reducer's
    /// objective evaluation is the one completion authority; this is its HUD
    /// projection, at most one frame behind. Display only: the boss death →
    /// save authority is the phase machine.
    pub complete: bool,
}

/// Snapshot of one member creature's fight-relevant state.
#[derive(Clone, Debug)]
pub struct MemberProgress {
    pub name: String,
    pub phase: BossEncounterPhase,
    pub hp: i32,
    pub max_hp: i32,
}

impl MemberProgress {
    pub fn hp_fraction(&self) -> f32 {
        if self.max_hp <= 0 {
            0.0
        } else {
            (self.hp.max(0) as f32 / self.max_hp as f32).clamp(0.0, 1.0)
        }
    }
}

/// Ensure every *active* boss in the room is wrapped by an encounter entity.
///
/// A boss that has woken (left `Dormant`) and is not in any encounter gets a
/// single-boss `EncounterDef` (HUD-bound). A boss spawned with `no_encounter`
/// opts out. Runs in the Progression set after `update_boss_encounters`, so it
/// sees this frame's woken phase.
pub fn sync_boss_encounter_entities(
    mut commands: Commands,
    mut lifecycle_commands: MessageWriter<EncounterCommand>,
    bosses: Query<
        (
            Entity,
            &BossConfig,
            &BossEncounter,
            Option<&crate::BossOverrides>,
            Option<&SessionScopedEntity>,
            Option<&ambition_characters::actor::BodyHealth>,
        ),
        With<FeatureSimEntity>,
    >,
    encounters: Query<(&Encounter, &EncounterParticipants, &EncounterLifecycle)>,
) {
    // Coverage by cached entity and by durable id: a snapshot restore clears
    // the entity caches (an Entity is never serialized), and re-wrapping an
    // already-wrapped boss after a restore would fork the timeline.
    let covered_entities: HashSet<Entity> = encounters
        .iter()
        .flat_map(|(_, p, _)| p.members.iter().filter_map(|m| m.entity))
        .collect();
    let covered_ids: HashSet<&str> = encounters
        .iter()
        .flat_map(|(_, p, _)| p.members.iter().map(|m| m.id.as_str()))
        .collect();
    for (entity, config, status, overrides, owner, health) in &bosses {
        // Only orchestrate a boss that has actually woken — a Dormant boss
        // (cleared / not yet entered) needs none.
        let active = status
            .encounter
            .as_ref()
            .map(|p| !matches!(p.phase, BossEncounterPhase::Dormant))
            .unwrap_or(false);
        if covered_entities.contains(&entity) || covered_ids.contains(config.id.as_str()) {
            // Already wrapped. The wrap persists for the session (a room exit
            // resets it; see `update_encounter_progress`), so a living boss
            // fighting under a wrap that is not in flight means a fresh
            // attempt: re-arm through the one ingress. `Death` and a dead body
            // are excluded: on the death frame the wrap completes before the
            // boss's phase machine reaches `Death`, and that won fight must
            // not reset.
            let fighting = status
                .encounter
                .as_ref()
                .map(|p| {
                    !matches!(
                        p.phase,
                        BossEncounterPhase::Dormant | BossEncounterPhase::Death
                    )
                })
                .unwrap_or(false)
                && health.is_some_and(|h| h.alive());
            if fighting {
                if let Some((enc, _, lifecycle)) = encounters
                    .iter()
                    .find(|(enc, _, _)| enc.id == config.id.as_str())
                {
                    match lifecycle.phase() {
                        // Room re-entry: the reset wrap waits Inactive.
                        ambition_encounter::EncounterPhase::Inactive => {
                            lifecycle_commands.write(EncounterCommand::new(
                                enc.id.clone(),
                                EncounterCommandKind::Start,
                            ));
                        }
                        // A new incarnation fighting under a terminal wrap (a
                        // re-armed boss): Reset re-arms and Start begins; the
                        // reducer applies both in order, in the same frame.
                        ambition_encounter::EncounterPhase::Completed
                        | ambition_encounter::EncounterPhase::Failed => {
                            lifecycle_commands.write(EncounterCommand::new(
                                enc.id.clone(),
                                EncounterCommandKind::Reset,
                            ));
                            lifecycle_commands.write(EncounterCommand::new(
                                enc.id.clone(),
                                EncounterCommandKind::Start,
                            ));
                        }
                        _ => {}
                    }
                }
            }
            continue;
        }
        // A boss spawned with `no_encounter` is a plain tough enemy — no
        // HUD / lock-walls / win-lose. Skip wrapping it.
        if overrides.is_some_and(|o| o.no_encounter) {
            continue;
        }
        if !active {
            continue;
        }
        // The boss is the encounter's single adopted `PrimaryTarget`; the win
        // is the generic "all PrimaryTargets defeated" objective, decided by
        // the generic lifecycle reducer. Started through the command ingress
        // because the fight is already underway when the wrap appears.
        commands.spawn_session_scoped(
            SessionSpawnScope::new(owner.map(|owner| owner.0)),
            (
                Encounter::new(config.id.clone()),
                // Stable simulation identity (E11): its own `encounter:`
                // namespace — the boss body owns `placement:{id}`.
                ambition_platformer2d_shared_tangle::sim_id::SimId::encounter(&config.id),
                EncounterLifecycle::default(),
                EncounterDef { hud: true },
                EncounterParticipants::new(vec![EncounterParticipant::adopted(
                    config.id.clone(),
                    entity,
                    EncounterRole::PrimaryTarget,
                )]),
                EncounterObjective::win(Objective::AllWithRoleDefeated(
                    EncounterRole::PrimaryTarget,
                )),
                EncounterProgress::default(),
            ),
        );
        lifecycle_commands.write(EncounterCommand::new(
            config.id.clone(),
            EncounterCommandKind::Start,
        ));
    }
}

/// Recompute each encounter's progress from its members' entity-local state
/// (HP from the body's `BodyHealth`, phase from the entity-local
/// `ActorPhaseState`). Runs after `sync_boss_encounter_entities` in the
/// Progression set.
///
/// The wrap persists for its session. An encounter whose members have all left
/// the world (room change) is reset through the command ingress, never
/// despawned: the authority keeps its durable member ids, the caches heal by
/// id on re-entry, and the sync system re-arms the fight with a new `Start`.
/// So the `encounter:` identity always exists at snapshot-restore time.
/// The HUD does not linger: an unresolved member contributes no
/// `MemberProgress` row, and an empty progress renders nothing.
pub fn update_encounter_progress(
    mut lifecycle_commands: MessageWriter<EncounterCommand>,
    mut encounters: Query<(
        &Encounter,
        &mut EncounterParticipants,
        Option<&EncounterLifecycle>,
        &mut EncounterProgress,
    )>,
    bosses: Query<(
        Entity,
        &BossConfig,
        &BossEncounter,
        &ambition_characters::actor::BodyHealth,
    )>,
) {
    for (encounter, mut participants, lifecycle, mut progress) in &mut encounters {
        progress.members.clear();
        let mut any_resolved = false;
        for member in &mut participants.members {
            // Live resolution is a cache over the durable id: prefer the cached
            // entity, but heal a cleared cache (a snapshot restore never
            // serializes Entity handles) by finding the boss whose placement
            // id is this member's id.
            let resolved = member.entity.and_then(|e| bosses.get(e).ok()).or_else(|| {
                bosses
                    .iter()
                    .find(|(_, config, _, _)| config.id == member.id)
            });
            let Some((boss_entity, config, status, health)) = resolved else {
                // The member left the world (room change or despawn): forget
                // the stale entity. Keep its last `alive` flag: "unresolved"
                // must not read as "defeated", or leaving an arena would
                // satisfy the defeat objective.
                member.entity = None;
                continue;
            };
            member.entity = Some(boss_entity);
            any_resolved = true;
            member.alive = health.alive();
            // Phase comes from the entity-local copy; fall back to the synced
            // `encounter_phase` mirror if the copy isn't populated yet.
            let phase = status
                .encounter
                .as_ref()
                .map(|p| p.phase)
                .unwrap_or(status.encounter_phase);
            progress.members.push(MemberProgress {
                name: config.name.clone(),
                phase,
                hp: health.current(),
                max_hp: health.max(),
            });
        }
        // Every member gone (boss despawned on a room change) means the fight
        // is over for this world. Reset the in-flight lifecycle through the
        // ingress; the wrap waits, Inactive, for the sync system to re-arm it.
        // A terminal wrap (Completed boss) is left alone; its outcome stands.
        if !any_resolved && !participants.members.is_empty() {
            if lifecycle.is_some_and(|lc| {
                matches!(
                    lc.phase(),
                    ambition_encounter::EncounterPhase::Starting { .. }
                        | ambition_encounter::EncounterPhase::Active
                )
            }) {
                lifecycle_commands.write(EncounterCommand::new(
                    encounter.id.clone(),
                    EncounterCommandKind::Reset,
                ));
            }
            continue;
        }
        // The generic projection the HUD reads: the lifecycle reducer's
        // completion decision (at most one frame behind).
        progress.complete = lifecycle
            .is_some_and(|lc| matches!(lc.phase(), ambition_encounter::EncounterPhase::Completed));
    }
}

/// Generic instance-payload capability: when the host entity dies, emit a
/// [`PayloadReleased`] so content can spawn what the host "contained" (e.g.
/// the Smirking Behemoth's swallowed victory NPC) at the host's death
/// position.
///
/// The release comes from death, not a script. This host frees its payload; a
/// different instance of the same archetype has none. The release event is
/// separate from the content-specific spawn, so this stays reusable in the
/// library while the payload stays content-owned.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct ReleaseOnDeath;

/// Emitted once per [`ReleaseOnDeath`] host when it dies.
#[derive(bevy::prelude::Message, Clone, Copy, Debug)]
pub struct PayloadReleased {
    pub host: Entity,
    pub pos: ambition_platformer2d_core::Vec2,
}

/// Emit [`PayloadReleased`] for each dead `ReleaseOnDeath` host (once — the
/// marker is removed after firing).
pub fn release_payloads_on_death(
    mut commands: Commands,
    mut released: bevy::prelude::MessageWriter<PayloadReleased>,
    hosts: Query<
        (
            Entity,
            &ambition_characters::actor::BodyHealth,
            &ambition_platformer2d_shared_tangle::body::BodyKinematics,
        ),
        With<ReleaseOnDeath>,
    >,
) {
    for (entity, health, kin) in &hosts {
        if !health.alive() {
            released.write(PayloadReleased {
                host: entity,
                pos: kin.pos,
            });
            commands.entity(entity).remove::<ReleaseOnDeath>();
        }
    }
}

#[cfg(test)]
mod tests;

//! The ENCOUNTER as a first-class, OPTIONAL entity.
//!
//! Jon's three-layer model: an *archetype* is reusable creature data; an
//! *entity instance* is one spawned creature (HP + phase + payload); and an
//! *encounter* is the optional orchestration wrapped around one or more member
//! creatures — a progress model derived from member state, a HUD binding, lock
//! walls, win/lose conditions, music, and a scripted timeline.
//!
//! A boss spawned with NO encounter is just a tough enemy: no HUD, no lock
//! walls, no win/lose — headless / RL fine. The encounter never *gates* the
//! creature's intrinsic phase-up (that is entity-local [`ActorPhaseState`]); it
//! only FRAMES / DISPLAYS the fight and adds external/scripted triggers.
//!
//! See `docs/planning/engine/encounter-orchestration.md`.

use std::collections::HashSet;

use bevy::prelude::*;

use crate::boss_encounter::BossEncounterPhase;
use crate::features::ecs::boss_clusters::{BossConfig, BossEncounter};
use crate::features::FeatureSimEntity;
use ambition_encounter::{
    Encounter, EncounterCommand, EncounterCommandKind, EncounterLifecycle, EncounterObjective,
    EncounterParticipant, EncounterParticipants, EncounterRole, Objective,
};
use ambition_platformer_primitives::lifecycle::{
    SessionScopedEntity, SessionSpawnScope, SpawnSessionScopedExt,
};

/// Definition of an encounter entity: its stable identity + how it FRAMES its
/// members. Optional by construction — a creature with no `EncounterDef` nearby
/// is simply un-orchestrated.
///
/// E2: membership moved to the generic [`EncounterParticipants`] component and
/// the win condition to the generic [`EncounterObjective`] component (both on
/// the same entity), so an encounter's members/objective are generic vocabulary
/// shared with wave arenas — not a boss-shaped `Vec<Entity>`.
#[derive(Component, Clone, Debug)]
pub struct EncounterDef {
    /// Stable placement id (the room / LDtk encounter key). R4 keys the
    /// "cleared" save record to THIS — the placement — not the archetype, so
    /// reusing a boss archetype elsewhere is not pre-marked cleared.
    pub placement_id: String,
    /// Whether this encounter binds the HUD (a view of its progress). `false`
    /// / no encounter ⇒ no boss HUD.
    pub hud: bool,
}

/// Live, member-derived progress of an encounter — recomputed every frame by
/// [`update_encounter_progress`]. The HUD is a view bound to this; nothing in
/// the sim depends on it (so a headless build that ignores it is fine).
#[derive(Component, Clone, Debug, Default)]
pub struct EncounterProgress {
    /// One entry per resolvable member, in [`EncounterParticipants`] order.
    pub members: Vec<MemberProgress>,
    /// Mirror of the generic lifecycle's `Completed` phase (E8 — the
    /// reducer's objective evaluation is the one completion authority; this
    /// is its HUD projection, one frame behind at most). Display/read-model
    /// only: the boss death → save authority is still the phase machine
    /// (converged at E4).
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
/// A boss that has woken (left `Dormant`) and is not yet a member of any
/// encounter gets a single-boss `EncounterDef` (HUD-bound). A boss spawned with
/// `no_encounter` opts out (a plain tough enemy, no HUD). Runs in the
/// Progression set after `update_boss_encounters` so it observes this frame's
/// woken phase.
pub fn sync_boss_encounter_entities(
    mut commands: Commands,
    mut lifecycle_commands: MessageWriter<EncounterCommand>,
    bosses: Query<
        (
            Entity,
            &BossConfig,
            &BossEncounter,
            Option<&crate::features::BossOverrides>,
            Option<&SessionScopedEntity>,
        ),
        With<FeatureSimEntity>,
    >,
    encounters: Query<&EncounterParticipants>,
) {
    // Coverage by cached entity AND by durable id: a snapshot restore nulls
    // the entity caches (an Entity is never serialized), and re-wrapping an
    // already-wrapped boss on the post-restore frame would fork the timeline.
    let covered_entities: HashSet<Entity> = encounters
        .iter()
        .flat_map(|p| p.members.iter().filter_map(|m| m.entity))
        .collect();
    let covered_ids: HashSet<&str> = encounters
        .iter()
        .flat_map(|p| p.members.iter().map(|m| m.id.as_str()))
        .collect();
    for (entity, config, status, overrides, owner) in &bosses {
        if covered_entities.contains(&entity) || covered_ids.contains(config.id.as_str()) {
            continue;
        }
        // A boss spawned with `no_encounter` is a plain tough enemy — no
        // HUD / lock-walls / win-lose. Skip wrapping it.
        if overrides.is_some_and(|o| o.no_encounter) {
            continue;
        }
        // Only wrap an encounter around a boss that has actually woken — a
        // Dormant boss (cleared / not yet entered) needs no orchestration.
        let active = status
            .encounter
            .as_ref()
            .map(|p| !matches!(p.phase, BossEncounterPhase::Dormant))
            .unwrap_or(false);
        if !active {
            continue;
        }
        // The boss is the encounter's single ADOPTED `PrimaryTarget`; the win is
        // the generic "all PrimaryTargets defeated" objective, decided by the
        // generic lifecycle reducer (E8) — started through the command ingress
        // because the fight is already underway when the wrap appears.
        commands.spawn_session_scoped(
            SessionSpawnScope::new(owner.map(|owner| owner.0)),
            (
                Encounter::new(config.id.clone()),
                // Stable simulation identity (E11): its own `encounter:`
                // namespace — the boss BODY owns `placement:{id}`.
                ambition_platformer_primitives::sim_id::SimId::encounter(&config.id),
                EncounterLifecycle::default(),
                EncounterDef {
                    placement_id: config.id.clone(),
                    hud: true,
                },
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
/// (HP from the body's `BodyHealth` (§A1), phase from the entity-local `ActorPhaseState`
/// copy). Despawns an encounter whose members have all left the world (room
/// change), so stale encounters don't linger on the HUD. Runs after
/// `sync_boss_encounter_entities` in the Progression set.
pub fn update_encounter_progress(
    mut commands: Commands,
    mut encounters: Query<(
        Entity,
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
    for (entity, mut participants, lifecycle, mut progress) in &mut encounters {
        progress.members.clear();
        let mut any_resolved = false;
        for member in &mut participants.members {
            // Live resolution is a CACHE over the durable id: prefer the
            // cached entity, but heal a nulled cache (a snapshot restore
            // never serializes Entity handles) by re-resolving the boss
            // whose placement id IS this member's id.
            let resolved = member.entity.and_then(|e| bosses.get(e).ok()).or_else(|| {
                bosses
                    .iter()
                    .find(|(_, config, _, _)| config.id == member.id)
            });
            let Some((boss_entity, config, status, health)) = resolved else {
                // The member left the world (room change / despawn): forget the
                // stale entity + read it as not alive.
                member.entity = None;
                member.alive = false;
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
        // Every member gone (boss despawned on a room change) ⇒ the encounter
        // is over its world; retire it.
        if !any_resolved {
            commands.entity(entity).despawn();
            continue;
        }
        // The generic projection the HUD read model observes: the lifecycle
        // reducer's completion decision (E8 — objective evaluation happens
        // there, once; this mirror is one frame behind at most).
        progress.complete = lifecycle
            .is_some_and(|lc| matches!(lc.phase, ambition_encounter::EncounterPhase::Completed));
    }
}

/// Generic instance-payload capability (R5): when the host entity dies, emit a
/// [`PayloadReleased`] so content can spawn whatever the host "contained" (e.g.
/// the Smirking Behemoth's swallowed victory NPC) at the host's death position.
///
/// The release falls out of DEATH — it is NOT scripted. THIS host frees ITS
/// payload; a different instance of the same archetype has none. Decoupling the
/// release event from the content-specific spawn keeps this reusable in the lib
/// while the payload (a content NPC) stays content-owned.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct ReleaseOnDeath;

/// Emitted once per [`ReleaseOnDeath`] host when it dies.
#[derive(bevy::prelude::Message, Clone, Copy, Debug)]
pub struct PayloadReleased {
    pub host: Entity,
    pub pos: ambition_engine_core::Vec2,
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
            &crate::features::BodyKinematics,
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

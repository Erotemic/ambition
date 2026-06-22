//! The ENCOUNTER as a first-class, OPTIONAL entity (refactor Stage R2).
//!
//! Jon's three-layer model: an *archetype* is reusable creature data; an
//! *entity instance* is one spawned creature (HP + phase + payload); and an
//! *encounter* is the optional orchestration wrapped around one or more member
//! creatures — a progress model derived from member state, a HUD binding, lock
//! walls, win/lose conditions, music, and (R5) a scripted timeline.
//!
//! A boss spawned with NO encounter is just a tough enemy: no HUD, no lock
//! walls, no win/lose — headless / RL fine. The encounter never *gates* the
//! creature's intrinsic phase-up (that is entity-local [`BossPhaseState`]); it
//! only FRAMES / DISPLAYS the fight and (R5) adds external/scripted triggers.
//!
//! R2 is reader-only: this introduces the entity + a member-derived progress
//! model and points the HUD at it, while the global `BossEncounterRegistry`
//! stays the authority. R3 flips writers (win conditions observe members; the
//! entity copy becomes the source of truth) and deletes the global live map.
//!
//! See `docs/planning/boss-entity-local-refactor.md` (DESIGN REFINEMENT + R2).

use std::collections::HashSet;

use bevy::prelude::*;

use crate::boss_encounter::BossEncounterPhase;
use crate::combat::boss_clusters::{BossConfig, BossStatus};
use crate::features::FeatureSimEntity;

/// Definition of an encounter entity: which members it orchestrates + how it
/// frames them. Optional by construction — a creature with no `EncounterDef`
/// nearby is simply un-orchestrated.
#[derive(Component, Clone, Debug)]
pub struct EncounterDef {
    /// Stable placement id (the room / LDtk encounter key). R4 keys the
    /// "cleared" save record to THIS — the placement — not the archetype, so
    /// reusing a boss archetype elsewhere is not pre-marked cleared.
    pub placement_id: String,
    /// The member creature entities this encounter orchestrates. A single-boss
    /// fight has one; a gauntlet / add-wave has many.
    pub members: Vec<Entity>,
    /// Whether this encounter binds the HUD (a view of its progress). `false`
    /// / no encounter ⇒ no boss HUD.
    pub hud: bool,
    /// Win condition. R3 observes members to resolve it + write the save.
    pub win: EncounterWin,
}

/// How an encounter is won. Extensible (add-wave "survive N waves", cut-rope
/// "boss crushed", …); R2 only needs the common case.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EncounterWin {
    /// Cleared once every member creature is dead (single-boss + gauntlet).
    AllMembersDead,
}

/// Live, member-derived progress of an encounter — recomputed every frame by
/// [`update_encounter_progress`]. The HUD is a view bound to this; nothing in
/// the sim depends on it (so a headless build that ignores it is fine).
#[derive(Component, Clone, Debug, Default)]
pub struct EncounterProgress {
    /// One entry per resolvable member, in `EncounterDef::members` order.
    pub members: Vec<MemberProgress>,
}

impl EncounterProgress {
    /// True once every tracked member is dead — the `AllMembersDead` shape,
    /// surfaced for the HUD / R3 win check.
    pub fn all_members_dead(&self) -> bool {
        !self.members.is_empty() && self.members.iter().all(|m| m.hp <= 0)
    }
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
/// R2 policy: a boss that has woken (left `Dormant`) and is not yet a member of
/// any encounter gets a single-boss `EncounterDef` (HUD-bound). This preserves
/// the live boss HUD when its reader migrates off the global registry. R6 adds
/// the spawn-seam opt-out so a boss can be spawned encounter-LESS (a plain
/// tough enemy, no HUD). Runs in the Progression set after
/// `update_boss_encounters` so it observes this frame's woken phase.
pub fn sync_boss_encounter_entities(
    mut commands: Commands,
    bosses: Query<(Entity, &BossConfig, &BossStatus), With<FeatureSimEntity>>,
    encounters: Query<&EncounterDef>,
) {
    let covered: HashSet<Entity> = encounters
        .iter()
        .flat_map(|d| d.members.iter().copied())
        .collect();
    for (entity, config, status) in &bosses {
        if covered.contains(&entity) {
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
        commands.spawn((
            EncounterDef {
                placement_id: config.id.clone(),
                members: vec![entity],
                hud: true,
                win: EncounterWin::AllMembersDead,
            },
            EncounterProgress::default(),
        ));
    }
}

/// Recompute each encounter's progress from its members' entity-local state
/// (HP from `BossStatus.health`, phase from the entity-local `BossPhaseState`
/// copy — NOT the global registry). Despawns an encounter whose members have
/// all left the world (room change), so stale encounters don't linger on the
/// HUD. Runs after `sync_boss_encounter_entities` in the Progression set.
pub fn update_encounter_progress(
    mut commands: Commands,
    mut encounters: Query<(Entity, &EncounterDef, &mut EncounterProgress)>,
    bosses: Query<(&BossConfig, &BossStatus)>,
) {
    for (entity, def, mut progress) in &mut encounters {
        progress.members.clear();
        for &member in &def.members {
            let Ok((config, status)) = bosses.get(member) else {
                continue;
            };
            // Phase comes from the entity-local copy (the reader migration);
            // fall back to the synced `encounter_phase` mirror if the copy
            // isn't populated yet.
            let phase = status
                .encounter
                .as_ref()
                .map(|p| p.phase)
                .unwrap_or(status.encounter_phase);
            progress.members.push(MemberProgress {
                name: config.name.clone(),
                phase,
                hp: status.health.current,
                max_hp: status.health.max,
            });
        }
        // Every member gone (boss despawned on a room change) ⇒ the encounter
        // is over its world; retire it.
        if progress.members.is_empty() {
            commands.entity(entity).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boss_encounter::{BossPhaseState, PhaseTrigger};
    use crate::combat::boss_clusters::{BossConfig, BossStatus};

    fn awake_boss(name: &str, hp: i32) -> (BossConfig, BossStatus, FeatureSimEntity) {
        let mut phase = BossPhaseState::new(vec![PhaseTrigger::hp_below(
            0.5,
            BossEncounterPhase::Phase1,
            BossEncounterPhase::Phase2,
            0.0,
        )]);
        phase.wake();
        let config = BossConfig {
            id: format!("{name}_runtime"),
            name: name.to_string(),
            spawn: crate::engine_core::Vec2::ZERO,
            brain: crate::actor::BossBrain::PhaseScript {
                script_id: name.to_string(),
            },
            behavior: crate::boss_encounter::behavior::BossBehaviorProfile::for_authored_boss(name),
        };
        let mut status = BossStatus {
            health: crate::actor::Health::new(hp),
            alive: true,
            hit_flash: 0.0,
            encounter_phase: BossEncounterPhase::Phase1,
            sprite_metrics: None,
            encounter: Some(phase),
        };
        status.health.current = hp;
        (config, status, FeatureSimEntity)
    }

    #[test]
    fn active_boss_gets_a_single_boss_encounter_entity() {
        let mut app = App::new();
        app.add_systems(Update, sync_boss_encounter_entities);
        let boss = app.world_mut().spawn(awake_boss("mockingbird", 30)).id();

        app.update();

        let mut q = app.world_mut().query::<&EncounterDef>();
        let defs: Vec<_> = q.iter(app.world()).collect();
        assert_eq!(defs.len(), 1, "one active boss ⇒ one encounter entity");
        assert_eq!(defs[0].members, vec![boss]);
        assert!(defs[0].hud);
        assert_eq!(defs[0].placement_id, "mockingbird_runtime");

        // Idempotent: a second pass does not spawn a duplicate.
        app.update();
        let mut q = app.world_mut().query::<&EncounterDef>();
        assert_eq!(q.iter(app.world()).count(), 1, "no duplicate encounter");
    }

    #[test]
    fn progress_reflects_member_hp_and_phase() {
        let mut app = App::new();
        app.add_systems(Update, (sync_boss_encounter_entities, update_encounter_progress).chain());
        app.world_mut().spawn(awake_boss("mockingbird", 40)).id();

        app.update();

        let mut q = app.world_mut().query::<&EncounterProgress>();
        let progress = q.iter(app.world()).next().expect("progress exists");
        assert_eq!(progress.members.len(), 1);
        let m = &progress.members[0];
        assert_eq!(m.name, "mockingbird");
        assert_eq!(m.hp, 40);
        assert_eq!(m.phase, BossEncounterPhase::Phase1);
        assert!(!progress.all_members_dead());
    }

    #[test]
    fn encounter_retires_when_its_member_despawns() {
        let mut app = App::new();
        app.add_systems(Update, (sync_boss_encounter_entities, update_encounter_progress).chain());
        let boss = app.world_mut().spawn(awake_boss("mockingbird", 40)).id();
        app.update();
        assert_eq!(
            app.world_mut().query::<&EncounterDef>().iter(app.world()).count(),
            1
        );

        // The boss leaves the world (room change).
        app.world_mut().entity_mut(boss).despawn();
        app.update();
        assert_eq!(
            app.world_mut().query::<&EncounterDef>().iter(app.world()).count(),
            0,
            "an encounter whose members all left the world retires"
        );
    }
}

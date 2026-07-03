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
//! creature's intrinsic phase-up (that is entity-local [`BossPhaseState`]); it
//! only FRAMES / DISPLAYS the fight and adds external/scripted triggers.
//!
//! See `docs/planning/boss-entity-local-refactor.md`.

use std::collections::HashSet;

use bevy::prelude::*;

use crate::boss_encounter::BossEncounterPhase;
use crate::combat::boss_clusters::{BossConfig, BossEncounter};
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
/// A boss that has woken (left `Dormant`) and is not yet a member of any
/// encounter gets a single-boss `EncounterDef` (HUD-bound). A boss spawned with
/// `no_encounter` opts out (a plain tough enemy, no HUD). Runs in the
/// Progression set after `update_boss_encounters` so it observes this frame's
/// woken phase.
pub fn sync_boss_encounter_entities(
    mut commands: Commands,
    bosses: Query<
        (
            Entity,
            &BossConfig,
            &BossEncounter,
            Option<&crate::features::BossOverrides>,
        ),
        With<FeatureSimEntity>,
    >,
    encounters: Query<&EncounterDef>,
) {
    let covered: HashSet<Entity> = encounters
        .iter()
        .flat_map(|d| d.members.iter().copied())
        .collect();
    for (entity, config, status, overrides) in &bosses {
        if covered.contains(&entity) {
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
/// (HP from the body's `BodyHealth` (§A1), phase from the entity-local `BossPhaseState`
/// copy). Despawns an encounter whose members have all left the world (room
/// change), so stale encounters don't linger on the HUD. Runs after
/// `sync_boss_encounter_entities` in the Progression set.
pub fn update_encounter_progress(
    mut commands: Commands,
    mut encounters: Query<(Entity, &EncounterDef, &mut EncounterProgress)>,
    bosses: Query<(&BossConfig, &BossEncounter, &ambition_characters::actor::BodyHealth)>,
) {
    for (entity, def, mut progress) in &mut encounters {
        progress.members.clear();
        for &member in &def.members {
            let Ok((config, status, health)) = bosses.get(member) else {
                continue;
            };
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
        if progress.members.is_empty() {
            commands.entity(entity).despawn();
        }
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
        (Entity, &ambition_characters::actor::BodyHealth, &crate::features::BodyKinematics),
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
mod tests {
    use super::*;
    use crate::boss_encounter::PhaseTrigger;
    use crate::combat::boss_clusters::test_support::{test_boss_config, test_boss_status_with};
    use crate::combat::boss_clusters::{BossConfig, BossEncounter};

    fn awake_boss(
        name: &str,
        hp: i32,
    ) -> (
        BossConfig,
        BossEncounter,
        ambition_characters::actor::BodyHealth,
        FeatureSimEntity,
    ) {
        // Placement id is the `<name>_runtime` LDtk-style key the tests assert on.
        let config = test_boss_config(format!("{name}_runtime"), name, name);
        // Awake in Phase1 with an hp<0.5 Phase1→Phase2 trigger — the half-health
        // phase-up the progress/encounter tests observe.
        let (status, health) = test_boss_status_with(
            hp,
            BossEncounterPhase::Phase1,
            vec![PhaseTrigger::hp_below(
                0.5,
                BossEncounterPhase::Phase1,
                BossEncounterPhase::Phase2,
                0.0,
            )],
        );
        (config, status, health, FeatureSimEntity)
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
        app.add_systems(
            Update,
            (sync_boss_encounter_entities, update_encounter_progress).chain(),
        );
        app.world_mut().spawn(awake_boss("mockingbird", 40));

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
        app.add_systems(
            Update,
            (sync_boss_encounter_entities, update_encounter_progress).chain(),
        );
        let boss = app.world_mut().spawn(awake_boss("mockingbird", 40)).id();
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&EncounterDef>()
                .iter(app.world())
                .count(),
            1
        );

        // The boss leaves the world (room change).
        app.world_mut().entity_mut(boss).despawn();
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&EncounterDef>()
                .iter(app.world())
                .count(),
            0,
            "an encounter whose members all left the world retires"
        );
    }

    #[test]
    fn release_on_death_emits_payload_once_at_host_position() {
        use crate::features::BodyKinematics;
        let mut app = App::new();
        app.add_message::<PayloadReleased>();
        app.add_systems(Update, release_payloads_on_death);

        let (config, status, mut health, sim) = awake_boss("behemoth", 9999);
        health.health.current = 0; // dead host
        let host = app
            .world_mut()
            .spawn((
                config,
                status,
                health,
                sim,
                BodyKinematics {
                    pos: ambition_engine_core::Vec2::new(120.0, 80.0),
                    vel: ambition_engine_core::Vec2::ZERO,
                    size: ambition_engine_core::Vec2::splat(32.0),
                    facing: 1.0,
                },
                ReleaseOnDeath,
            ))
            .id();

        app.update();

        let released: Vec<_> = app
            .world()
            .resource::<bevy::ecs::message::Messages<PayloadReleased>>()
            .iter_current_update_messages()
            .map(|m| (m.host, m.pos))
            .collect();
        assert_eq!(released.len(), 1, "exactly one release on death");
        assert_eq!(released[0].0, host);
        assert_eq!(released[0].1, ambition_engine_core::Vec2::new(120.0, 80.0));
        // Released once: the marker is gone, so a second tick emits nothing.
        assert!(app.world().entity(host).get::<ReleaseOnDeath>().is_none());
    }
}

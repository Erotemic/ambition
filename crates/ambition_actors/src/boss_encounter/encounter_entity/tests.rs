//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

use super::*;
use crate::boss_encounter::PhaseTrigger;
use crate::features::ecs::boss_clusters::test_support::{test_boss_config, test_boss_status_with};
use crate::features::ecs::boss_clusters::{BossConfig, BossEncounter};
use ambition_encounter::{EncounterParticipants, EncounterRole};

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
    app.add_message::<ambition_encounter::EncounterCommand>();
    app.add_systems(Update, sync_boss_encounter_entities);
    let boss = app.world_mut().spawn(awake_boss("mockingbird", 30)).id();

    app.update();

    let mut q = app
        .world_mut()
        .query::<(&EncounterDef, &EncounterParticipants, &EncounterLifecycle)>();
    let defs: Vec<_> = q.iter(app.world()).collect();
    assert_eq!(defs.len(), 1, "one active boss ⇒ one encounter entity");
    let (def, parts, _lifecycle) = defs[0];
    assert_eq!(parts.members.len(), 1);
    assert_eq!(parts.members[0].entity, Some(boss));
    assert_eq!(parts.members[0].role, EncounterRole::PrimaryTarget);
    assert!(def.hud);
    assert_eq!(def.placement_id, "mockingbird_runtime");
    // E8: the wrap starts its generic lifecycle through the command ingress.
    let started: Vec<_> = app
        .world()
        .resource::<bevy::ecs::message::Messages<ambition_encounter::EncounterCommand>>()
        .iter_current_update_messages()
        .filter(|c| matches!(c.kind, EncounterCommandKind::Start))
        .map(|c| c.encounter.clone())
        .collect();
    assert_eq!(started, vec!["mockingbird_runtime".to_string()]);

    // Idempotent: a second pass does not spawn a duplicate.
    app.update();
    let mut q = app.world_mut().query::<&EncounterDef>();
    assert_eq!(q.iter(app.world()).count(), 1, "no duplicate encounter");
}

#[test]
fn progress_reflects_member_hp_and_phase() {
    let mut app = App::new();
    app.add_message::<ambition_encounter::EncounterCommand>();
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
    assert!(!progress.complete, "a living boss ⇒ objective not met");
}

#[test]
fn encounter_retires_when_its_member_despawns() {
    let mut app = App::new();
    app.add_message::<ambition_encounter::EncounterCommand>();
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

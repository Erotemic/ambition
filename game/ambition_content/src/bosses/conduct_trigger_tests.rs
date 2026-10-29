//! What `ambition.boss.conduct` gives a conductor, read by a probe module
//! bound to a boss built as the game builds it.

use ambition_boss_encounter::{BossClusterScratch, BossEncounterPhase};
use ambition_boss_special_port::{BodyBox, BossConduct, BossConductPort};
use ambition_characters::brain::BossAttackState;
use ambition_combat::components::ActorTarget;
use ambition_extension_host::{ExtensionAppExt, ExtensionHostPlugin};
use ambition_extension_sdk::phases::BOSS_CONDUCT;
use ambition_extension_sdk::{
    CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation, Limits, ModuleDescriptor, ModuleKey, Port,
    TriggerBinding, API_VERSION,
};
use ambition_platformer2d_core::{self as ae, BodyKinematics};
use ambition_platformer2d_shared_tangle::lifecycle::{LiveRoomInstance, RoomInstanceRoot};
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
struct Sim;

const BEHAVIOUR: &str = "flying_spaghetti_monster";

thread_local! {
    // The schedule is single-threaded, so the probe runs on the test's thread.
    static SEEN: std::cell::RefCell<Vec<BossConduct>> = const { std::cell::RefCell::new(Vec::new()) };
}

fn record(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let conduct = inv.trigger::<BossConductPort>()?.clone();
    SEEN.with(|seen| seen.borrow_mut().push(conduct));
    Ok(())
}

fn probe() -> ModuleDescriptor {
    ModuleDescriptor {
        key: ModuleKey::new("test", "conduct_probe"),
        api: API_VERSION,
        code: CodeIdentity::StaticNative { crate_name: "test".into(), version: "0".into() },
        schemas: Vec::new(),
        entries: vec![EntryDescriptor {
            key: "conduct".into(),
            phase: BOSS_CONDUCT,
            trigger: TriggerBinding { port: BossConductPort::KEY, selector: BEHAVIOUR.into() },
            reads: Vec::new(),
            writes: Vec::new(),
            requests: Vec::new(),
            after: Vec::new(),
            limits: Limits { max_requests: 0 },
            on_idle: IdlePolicy::Invoke,
            run: EntryCode::Native(record),
        }],
    }
}

/// ⭐ The trigger carries the boss's health, the encounter's refusal of
/// damage, and the tracked body where it is NOW. The boss is in its intro:
/// it refuses damage, and it is not between two phases (the control that
/// `invulnerable` is not `between_phases` again). The target was seen at a
/// place the body has left.
#[test]
fn a_conductor_reads_the_boss_health_its_refusal_and_the_tracked_body_now() {
    let sprite_content = ambition_platformer2d::sprite_sheet::test_support::shared_sprites();
    let dt = 1.0 / 60.0;
    let mut app = App::new();
    app.insert_resource(sprite_content.clone());
    app.init_schedule(Sim);
    app.edit_schedule(Sim, |schedule| {
        schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
    });
    app.add_message::<ambition_characters::brain::ActorActionMessage>()
        .add_message::<ambition_projectiles::ProjectileSpawnRequest>()
        .add_message::<ambition_vfx::EffectRequest>()
        .add_message::<ambition_vfx::vfx::VfxInRoom>()
        .add_message::<ambition_sfx::OwnedSfxMessage>()
        .init_resource::<ambition_time::SimTick>()
        .insert_resource(ambition_time::WorldTime::new(dt, dt));
    app.add_plugins(ExtensionHostPlugin::new(Sim));
    ambition_platformer2d_runtime::extension_composition::install_ports(&mut app);
    app.add_extension_module(probe());
    app.finish();
    let hall = ae::World::new(
        "hall",
        ae::Vec2::new(1800.0, 1000.0),
        ae::Vec2::ZERO,
        vec![ae::Block::solid("floor", ae::Vec2::new(0.0, 900.0), ae::Vec2::new(1800.0, 100.0))],
    );
    app.world_mut().spawn((RoomInstanceRoot, LiveRoomInstance::ACTIVATION, ae::RoomGeometry(hall)));
    let player = app
        .world_mut()
        .spawn(BodyKinematics { pos: ae::Vec2::new(640.0, 870.0), vel: ae::Vec2::ZERO, size: ae::Vec2::new(20.0, 30.0), facing: 1.0 })
        .id();
    let catalog = crate::bosses::shipped_boss_catalog(&sprite_content);
    let mut boss = BossClusterScratch::new(&sprite_content,
        &catalog,
        "fsm_placement",
        "Flying Spaghetti Monster",
        ae::Aabb::new(ae::Vec2::new(900.0, 635.0), ae::Vec2::new(95.0, 44.0)),
        ambition_entity_catalog::placements::BossBrain::PhaseScript { script_id: BEHAVIOUR.to_string() },
    );
    assert_eq!(boss.config.behavior.id, BEHAVIOUR, "the premise: the probe is bound to this boss");
    let mut state = ambition_characters::boss_encounter::ActorPhaseState::new(Vec::new());
    state.phase = BossEncounterPhase::Intro;
    boss.status.encounter = Some(state);
    let full = boss.health.current();
    boss.health.damage(3);
    let health = boss.health.current();
    let kin = boss.kin;
    app.world_mut().spawn((
        boss.kin,
        ae::CenteredAabb::new(kin.pos, kin.size * 0.5),
        boss.config,
        boss.status,
        boss.health,
        BossAttackState::default(),
        ActorTarget { entity: Some(player), pos: ae::Vec2::new(500.0, 870.0) },
        ambition_characters::control::ActorControl::default(),
        ambition_characters::actor::ActorFaction::Boss,
        ambition_boss_encounter::conduct::ConductedFacing::of_facing(kin.facing),
    ));

    app.world_mut().run_schedule(Sim);
    let seen = SEEN.with(|seen| seen.borrow().clone());
    assert_eq!(seen.len(), 1, "the probe ran once: {seen:?}");
    let conduct = &seen[0];
    assert_eq!(conduct.health, health);
    assert_eq!(conduct.size, [kin.size.x, kin.size.y], "the boss's own size");
    assert_ne!(conduct.size, [20.0, 30.0], "the premise: the boss is not the size of its target");
    assert_eq!(health, full - 3, "the premise: the boss was hurt");
    assert!(conduct.invulnerable, "a boss in its intro refuses damage");
    assert!(!conduct.between_phases, "an intro is not between two phases");
    assert_eq!(conduct.target, Some([500.0, 870.0]), "where the boss last saw its target");
    assert_eq!(conduct.target_body, Some(BodyBox { position: [640.0, 870.0], size: [20.0, 30.0], swing: None }), "the tracked body now");
}

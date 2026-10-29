//! `ambition.boss.limb_opening` and `ambition.boss.limb_blow`, lowered on a
//! boss built as the game builds it, riding a mount with one fist.

use ambition_boss_encounter::conduct::{open_limbs, pass_limb_blows};
use ambition_boss_encounter::{BossClusterScratch, BossEncounter, BossEncounterPhase};
use ambition_boss_special_port::{LimbBlow, LimbOpening};
use ambition_characters::actor::{BodyCombat, BodyHealth, Health, Invulnerability, LimbRig, LimbSlot};
use ambition_extension_host::Submitted;
use ambition_platformer2d_core as ae;
use bevy::ecs::system::RunSystemOnce as _;
use bevy::prelude::*;

const BEHAVIOUR: &str = "flying_spaghetti_monster";

/// A boss in `phase`, riding a mount whose left hand is a fist of 10 health.
fn rider(sprite_content: &ambition_platformer2d::sprite_sheet::SpriteContent, phase: BossEncounterPhase) -> (World, Entity, Entity) {
    let mut world = World::new();
    let fist = world.spawn(BodyHealth::new(Health::new(10))).id();
    let mount = world.spawn(LimbRig::from_pairs([(LimbSlot::HAND_LEFT, fist)])).id();
    let catalog = crate::bosses::shipped_boss_catalog(&sprite_content);
    let mut boss = BossClusterScratch::new(&sprite_content,
        &catalog,
        "fsm_placement",
        "Flying Spaghetti Monster",
        ae::Aabb::new(ae::Vec2::new(900.0, 635.0), ae::Vec2::new(95.0, 44.0)),
        ambition_entity_catalog::placements::BossBrain::PhaseScript { script_id: BEHAVIOUR.to_string() },
    );
    let mut state = ambition_characters::boss_encounter::ActorPhaseState::new(Vec::new());
    state.phase = phase;
    boss.status.encounter = Some(state);
    let boss = world
        .spawn((boss.config, boss.status, boss.health, BodyCombat::default(), ambition_mount::RidingOn { mount }))
        .id();
    (world, boss, fist)
}

fn submitted<T>(boss: Entity, value: T) -> Vec<Submitted<T>> {
    vec![Submitted { scope: boss, invocation: Default::default(), occurrence: None, entry: "test::gnu/conduct".into(), value }]
}

fn closed(world: &World, fist: Entity) -> bool {
    world.get::<BodyHealth>(fist).unwrap().health.invulnerable.holds(Invulnerability::SCRIPTED)
}

/// ⭐ A closed fist refuses damage and an open one does not. The control: a
/// slot the rig does not have changes nothing.
#[test]
fn a_limb_opening_sets_the_fist_scripted_refusal() {
    let sprite_content = ambition_platformer2d::sprite_sheet::test_support::shared_sprites();
    let (mut world, boss, fist) = rider(&sprite_content, BossEncounterPhase::Phase1);
    let open = |slot: &str, open: bool| LimbOpening { slot: slot.into(), open };
    world.run_system_once_with(open_limbs, submitted(boss, open("hand_right", false))).unwrap();
    assert!(!closed(&world, fist), "another slot closed the fist");
    world.run_system_once_with(open_limbs, submitted(boss, open("hand_left", false))).unwrap();
    assert!(closed(&world, fist), "a closed fist refuses damage");
    world.run_system_once_with(open_limbs, submitted(boss, open("hand_left", true))).unwrap();
    assert!(!closed(&world, fist), "an open fist can be struck");
}

/// ⭐ A blow passed on heals the fist and hurts the boss, with a flash. A
/// blow not passed on only heals the fist. A boss that refuses damage (its
/// intro) is not hurt, and the fist still heals. A blow that kills the boss
/// ends its encounter.
#[test]
fn a_limb_blow_heals_the_fist_and_hurts_the_boss_once() {
    let sprite_content = ambition_platformer2d::sprite_sheet::test_support::shared_sprites();
    let blow = |pass: bool| LimbBlow { slot: "hand_left".into(), pass };
    let fist_health = |world: &World, fist: Entity| world.get::<BodyHealth>(fist).unwrap().current();
    let boss_health = |world: &World, boss: Entity| world.get::<BodyHealth>(boss).unwrap().current();

    let (mut world, boss, fist) = rider(&sprite_content, BossEncounterPhase::Phase1);
    let full = boss_health(&world, boss);
    world.get_mut::<BodyHealth>(fist).unwrap().damage(4);
    world.run_system_once_with(pass_limb_blows, submitted(boss, blow(true))).unwrap();
    assert_eq!(fist_health(&world, fist), 10, "the fist heals what it lost");
    assert_eq!(boss_health(&world, boss), full - 4, "the boss takes what the fist lost");
    assert_eq!(world.get::<BodyCombat>(boss).unwrap().hit_flash, 0.18);

    world.get_mut::<BodyHealth>(fist).unwrap().damage(4);
    world.run_system_once_with(pass_limb_blows, submitted(boss, blow(false))).unwrap();
    assert_eq!((fist_health(&world, fist), boss_health(&world, boss)), (10, full - 4), "a blow not passed on only heals");

    let (mut world, boss, fist) = rider(&sprite_content, BossEncounterPhase::Intro);
    world.get_mut::<BodyHealth>(fist).unwrap().damage(4);
    world.run_system_once_with(pass_limb_blows, submitted(boss, blow(true))).unwrap();
    assert_eq!((fist_health(&world, fist), boss_health(&world, boss)), (10, full), "a boss in its intro refuses the blow");
    assert_eq!(world.get::<BodyCombat>(boss).unwrap().hit_flash, 0.0);

    let (mut world, boss, fist) = rider(&sprite_content, BossEncounterPhase::Phase1);
    world.get_mut::<BodyHealth>(boss).unwrap().health.current = 3;
    world.get_mut::<BodyHealth>(fist).unwrap().damage(4);
    world.run_system_once_with(pass_limb_blows, submitted(boss, blow(true))).unwrap();
    let phase = world.get::<BossEncounter>(boss).unwrap().encounter.as_ref().unwrap().phase;
    assert_eq!(phase, BossEncounterPhase::Death, "a blow that kills the boss ends its encounter");
}

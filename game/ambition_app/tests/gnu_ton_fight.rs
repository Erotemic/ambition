//! GNU-ton's fight, played headlessly in the real arena: the authored room, the
//! authored pair, the scripted pattern, and the content conductor that performs
//! it (`ambition_content::bosses::gnu_ton`).

#![cfg(feature = "rl_sim")]

use ambition_app::AmbitionSim;
use ambition_app::{AgentAction, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode};
use ambition_content::bosses::gnu_ton::choreography::Move;
use ambition_content::bosses::gnu_ton::conductor::back_platform;
use ambition_content::bosses::gnu_ton::GnuTonConductor;
use ambition_platformer2d::boss_encounter::BossConfig;
use ambition_platformer2d::characters::actor::{BodyHealth, Invulnerability, Limb, LimbRig};
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
use bevy::prelude::*;

const ARENA: &str = "gnu_ton_arena";

fn arena() -> Platformer2dSimHarness {
    let opts = Platformer2dSimHarnessOptions::default()
        .with_timestep(TimestepMode::fixed_60hz())
        // ⚠ REQUIRED: the tolerant road falls back to the start room, which has
        // no GNU-ton, and every assertion below would be vacuous.
        .with_required_start_room(ARENA);
    let mut sim = Platformer2dSimHarness::new_with_options(opts).expect("the gnu-ton arena builds headlessly");
    for _ in 0..30 {
        sim.step(AgentAction::default());
    }
    sim
}

fn place_player(sim: &mut Platformer2dSimHarness, at: ae::Vec2) {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<(ae::BodyClusterQueryData, &mut ambition_platformer2d::actor::MotionModel), PrimaryPlayerOnly>();
    let (mut clusters, mut model) = q.iter_mut(world).next().expect("the player");
    let mut clusters = clusters.as_clusters_mut();
    ae::movement::transit_body(&mut model, &mut clusters, at, ae::movement::TransitVelocity::Zero);
}

fn player(sim: &mut Platformer2dSimHarness) -> (ae::BodyKinematics, i32) {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<(&ae::BodyKinematics, &BodyHealth), PrimaryPlayerOnly>();
    let (kin, health) = q.iter(world).next().expect("the player");
    (kin.clone(), health.current())
}

fn untouchable_player(sim: &mut Platformer2dSimHarness) {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<&mut BodyHealth, PrimaryPlayerOnly>();
    for mut health in q.iter_mut(world) {
        health.health.invulnerable.set(Invulnerability::SCRIPTED, true);
    }
}

#[derive(Debug)]
struct Scene {
    scholar: ae::BodyKinematics,
    scholar_hp: i32,
    performing: Option<(Move, bool)>,
    floor: f32,
    giant: ae::BodyKinematics,
    /// Left fist first: body, health, max health.
    fists: Vec<(ae::BodyKinematics, i32, i32)>,
}

fn scene(sim: &mut Platformer2dSimHarness) -> Scene {
    let world = sim.world_mut();
    let (scholar, scholar_hp, performing, floor) = world
        .query::<(&BossConfig, &ae::BodyKinematics, &BodyHealth, &GnuTonConductor)>()
        .iter(world)
        .find(|(config, ..)| config.behavior.id == "gnu_ton_rider")
        .map(|(_, kin, health, conductor)| {
            (
                kin.clone(),
                health.current(),
                conductor.performing(),
                conductor.hall().expect("the conductor measured the hall").floor,
            )
        })
        .expect("GNU-ton is in the arena, conducted");
    let (giant, rig) = world
        .query_filtered::<(&ae::BodyKinematics, &LimbRig), Without<Limb>>()
        .iter(world)
        .next()
        .map(|(kin, rig)| (kin.clone(), rig.clone()))
        .expect("the giant gnu is in the arena");
    let mut fists: Vec<_> = rig
        .limbs
        .values()
        .map(|&fist| {
            let kin = world.get::<ae::BodyKinematics>(fist).expect("a fist body").clone();
            let health = *world.get::<BodyHealth>(fist).expect("a fist's health");
            (kin, health.current(), health.max())
        })
        .collect();
    fists.sort_by(|a, b| a.0.pos.x.total_cmp(&b.0.pos.x));
    Scene { scholar, scholar_hp, performing, floor, giant, fists }
}

fn saddle(s: &Scene) -> ae::Vec2 {
    let side = if s.giant.facing < 0.0 { -1.0 } else { 1.0 };
    s.giant.pos + ae::Vec2::new(50.0 * side, -67.0)
}

fn on_floor(kin: &ae::BodyKinematics, floor: f32) -> bool {
    (kin.pos.y + kin.size.y * 0.5 - floor).abs() < 1.0
}

/// Step until `until` holds, or panic naming what it was waiting for.
fn step_until(
    sim: &mut Platformer2dSimHarness,
    frames: usize,
    what: &str,
    mut until: impl FnMut(&mut Platformer2dSimHarness) -> bool,
) {
    for _ in 0..frames {
        sim.step(AgentAction::default());
        if until(sim) {
            return;
        }
    }
    panic!("waited {frames} frames for {what}; last scene: {:?}", scene(sim));
}

/// The giant stands on the floor at its drawn size, and the scholar sits on its
/// shoulder at his: two of the complaints the rework started from.
#[test]
fn the_gnu_stands_on_the_floor_and_carries_a_person_sized_scholar() {
    let mut sim = arena();
    let s = scene(&mut sim);
    assert!(on_floor(&s.giant, s.floor), "the gnu's feet are on the floor {}: {:?}", s.floor, s.giant);
    assert!(
        s.giant.size.x > 400.0 && s.giant.size.y > 380.0,
        "the gnu is its drawn size, not a 220 box: {:?}",
        s.giant.size
    );
    assert!(
        (60.0..80.0).contains(&s.scholar.size.x) && (80.0..100.0).contains(&s.scholar.size.y),
        "the scholar is a person, not a speck: {:?}",
        s.scholar.size
    );
    assert!(
        (s.scholar.pos - saddle(&s)).length() < 1.0,
        "he sits in the saddle {:?}, not wherever his own brain steers: {:?}",
        saddle(&s),
        s.scholar.pos
    );
    assert_eq!(s.fists.len(), 2);
    for (fist, ..) in &s.fists {
        assert!(fist.size.x > 100.0, "a fist is a giant's fist: {:?}", fist.size);
    }
}

/// A demonstration hovers over the player, falls where they stood, hurts them,
/// and stays in the floor; a blow to the stuck fist lands on the scholar.
#[test]
fn a_slam_lands_where_you_stood_and_the_stuck_fist_carries_your_blow_to_him() {
    let mut sim = arena();
    place_player(&mut sim, ae::Vec2::new(400.0, 1200.0));
    let (_, hp_before) = player(&mut sim);
    step_until(&mut sim, 900, "a fist stuck in the floor after a demonstration", |sim| {
        let s = scene(sim);
        s.performing == Some((Move::Demonstrate, true)) && s.fists.iter().any(|(fist, ..)| on_floor(fist, s.floor))
    });
    let s = scene(&mut sim);
    let (stuck, ..) = s.fists.iter().find(|(fist, ..)| on_floor(fist, s.floor)).cloned().expect("a stuck fist");
    assert!((stuck.pos.x - 400.0).abs() < 40.0, "it fell where the player stood: {:?}", stuck.pos);
    for _ in 0..6 {
        sim.step(AgentAction::default());
    }
    let (_, hp_after) = player(&mut sim);
    assert!(hp_after < hp_before, "and it hurt them: {hp_before} -> {hp_after}");

    // Stand beside the stuck fist and hit it.
    untouchable_player(&mut sim);
    place_player(&mut sim, ae::Vec2::new(stuck.pos.x + stuck.size.x * 0.5 + 18.0, 1200.0));
    let scholar_before = scene(&mut sim).scholar_hp;
    for frame in 0..24 {
        sim.step(AgentAction {
            move_x: if frame < 2 { -1.0 } else { 0.0 },
            attack: frame == 4,
            attack_held: (4..8).contains(&frame),
            ..Default::default()
        });
    }
    let s = scene(&mut sim);
    assert!(
        s.scholar_hp < scholar_before,
        "a blow to his stuck fist lands on him: {scholar_before} -> {}",
        s.scholar_hp
    );
    for (_, hp, max) in &s.fists {
        assert_eq!(hp, max, "and the fist itself is whole again");
    }
}

/// His hands are his: nothing he throws lands on him.
#[test]
fn his_own_fists_never_hurt_him() {
    let mut sim = arena();
    untouchable_player(&mut sim);
    place_player(&mut sim, ae::Vec2::new(900.0, 1200.0));
    let before = scene(&mut sim).scholar_hp;
    for _ in 0..1500 {
        sim.step(AgentAction::default());
    }
    assert_eq!(scene(&mut sim).scholar_hp, before, "a fist that passed him took nothing");
}

/// The gnu's back is ground — the way to reach him in the saddle — and when he
/// bucks it throws whoever is standing there.
#[test]
fn you_can_stand_on_the_giants_shoulders_until_it_bucks() {
    let mut sim = arena();
    untouchable_player(&mut sim);
    let back = back_platform(&scene(&mut sim).giant);
    let spot = ae::Vec2::new((back.min.x + back.max.x) * 0.5 - 60.0, back.min.y - 40.0);
    place_player(&mut sim, spot);
    for _ in 0..20 {
        sim.step(AgentAction::default());
    }
    let (kin, _) = player(&mut sim);
    let feet = kin.pos.y + kin.size.y * 0.5;
    assert!((feet - back.min.y).abs() < 2.0, "standing on the gnu's back ({}): feet at {feet}", back.min.y);

    step_until(&mut sim, 1500, "the buck to throw the player off the gnu's back", |sim| {
        let (kin, _) = player(sim);
        kin.pos.y + kin.size.y * 0.5 < back.min.y - 120.0
    });
}

/// After the apple rain a golden apple knocks him off the gnu: he sits dazed on
/// the floor, then climbs back to the saddle.
#[test]
fn eureka_puts_him_on_the_floor_and_he_climbs_back() {
    let mut sim = arena();
    untouchable_player(&mut sim);
    place_player(&mut sim, ae::Vec2::new(300.0, 1200.0));
    step_until(&mut sim, 3000, "him dazed on the floor after the apple rain", |sim| {
        let s = scene(sim);
        s.performing == Some((Move::Eureka, true)) && on_floor(&s.scholar, s.floor)
    });
    step_until(&mut sim, 400, "him back in the saddle", |sim| {
        let s = scene(sim);
        s.performing.is_none() && (s.scholar.pos - saddle(&s)).length() < 1.0
    });
}

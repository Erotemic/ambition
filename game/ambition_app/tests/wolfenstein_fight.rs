//! Wolfenstein's fight, played headlessly in the real arena: the throne room,
//! the scripted pattern, and the conducted module that performs it
//! (`ambition_content_modules::wolfenstein`). The shape of the fight is in
//! `docs/planning/game/bosses.md`, "Wolfenstein — the leash".

#![cfg(feature = "rl_sim")]

use ambition_app::AmbitionSim;
use ambition_app::{AgentAction, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode};
use ambition_content::bosses::wolfenstein::conductor_module as module;
use ambition_content::bosses::wolfenstein::{conductor_of, Move, WolfensteinView, WOLFENSTEIN_ID};
use ambition_platformer2d::boss_encounter::BossConfig;
use ambition_platformer2d::characters::actor::{BodyHealth, Invulnerability};
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
use bevy::prelude::*;

const ARENA: &str = "castle_godwin_throne";

fn arena() -> Platformer2dSimHarness {
    let opts = Platformer2dSimHarnessOptions::default()
        .with_timestep(TimestepMode::fixed_60hz())
        // ⚠ REQUIRED: the tolerant road falls back to the start room, which has
        // no Wolfenstein, and every assertion below would be vacuous.
        .with_required_start_room(ARENA);
    let mut sim = Platformer2dSimHarness::new_with_options(opts).expect("the throne room builds headlessly");
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

fn untouchable_player(sim: &mut Platformer2dSimHarness, untouchable: bool) {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<&mut BodyHealth, PrimaryPlayerOnly>();
    for mut health in q.iter_mut(world) {
        health.health.invulnerable.set(Invulnerability::SCRIPTED, untouchable);
    }
}

#[derive(Debug)]
struct Wolf {
    kin: ae::BodyKinematics,
    side: f32,
    view: WolfensteinView,
}

fn wolf(sim: &mut Platformer2dSimHarness) -> Wolf {
    let world = sim.world_mut();
    let (entity, kin, side) = world
        .query::<(Entity, &BossConfig, &ae::BodyKinematics, &ambition_platformer2d::boss_encounter::conduct::ConductedFacing)>()
        .iter(world)
        .find(|(_, config, ..)| config.behavior.id == WOLFENSTEIN_ID)
        .map(|(entity, _, kin, side)| (entity, kin.clone(), side.0))
        .expect("Wolfenstein is in the throne room, conducted");
    let view = conductor_of(world, entity).expect("his conductor has measured the room");
    Wolf { kin, side, view }
}

/// Wound him to `fraction` of his health: past phase 1 under 0.6.
fn wound(sim: &mut Platformer2dSimHarness, fraction: f32) {
    let world = sim.world_mut();
    let mut q = world.query::<(&BossConfig, &mut BodyHealth)>();
    let (_, mut health) = q
        .iter_mut(world)
        .find(|(config, _)| config.behavior.id == WOLFENSTEIN_ID)
        .expect("Wolfenstein");
    health.health.current = (health.max() as f32 * fraction).ceil() as i32;
}

fn summoned(sim: &mut Platformer2dSimHarness) -> usize {
    let world = sim.world_mut();
    let mut q = world.query::<&ambition_platformer2d::combat::components::EncounterMob>();
    q.iter(world).filter(|mob| mob.encounter_id == WOLFENSTEIN_ID).count()
}

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
    panic!("waited {frames} frames for {what}; last: {:?}", wolf(sim));
}

/// Stand on the floor `ahead` in front of him, every tick, until `until`
/// says so.
fn stand_until(
    sim: &mut Platformer2dSimHarness,
    ahead: f32,
    frames: usize,
    what: &str,
    mut until: impl FnMut(&mut Platformer2dSimHarness) -> bool,
) {
    for _ in 0..frames {
        let w = wolf(sim);
        let hall = w.view.hall.expect("room");
        let (kin, _) = player(sim);
        let x = (w.kin.pos.x + w.side * ahead).clamp(hall.left + 30.0, hall.right - 30.0);
        place_player(sim, ae::Vec2::new(x, hall.floor - kin.size.y * 0.5 - 1.0));
        sim.step(AgentAction::default());
        if until(sim) {
            return;
        }
    }
    panic!("waited {frames} frames for {what}; last: {:?}", wolf(sim));
}

/// Stand on the floor at the far end of the room from him, every tick.
fn stand_far_until(
    sim: &mut Platformer2dSimHarness,
    frames: usize,
    what: &str,
    mut until: impl FnMut(&mut Platformer2dSimHarness) -> bool,
) {
    for _ in 0..frames {
        let w = wolf(sim);
        let hall = w.view.hall.expect("room");
        let x = if w.kin.pos.x > (hall.left + hall.right) * 0.5 { hall.left + 60.0 } else { hall.right - 60.0 };
        let (kin, _) = player(sim);
        place_player(sim, ae::Vec2::new(x, hall.floor - kin.size.y * 0.5 - 1.0));
        sim.step(AgentAction::default());
        if until(sim) {
            return;
        }
    }
    panic!("waited {frames} frames for {what}; last: {:?}", wolf(sim));
}

/// He stands on the floor: his drawn soles on it, every tick he is not in the
/// air.
#[test]
fn he_stands_on_the_floor() {
    let mut sim = arena();
    untouchable_player(&mut sim, true);
    let floor = wolf(&mut sim).view.hall.expect("he measured his room").floor;
    for _ in 0..60 * 6 {
        sim.step(AgentAction::default());
        let w = wolf(&mut sim);
        if matches!(w.view.performing, Some((Move::Bound, true))) {
            continue;
        }
        let feet = w.kin.pos.y + module::feet_below();
        assert!((feet - floor).abs() < 0.5, "his feet are {feet}, the floor {floor}: {w:?}");
    }
}

/// The rake lands on a player in front of him.
#[test]
fn the_rake_lands_in_front_of_him() {
    let mut sim = arena();
    untouchable_player(&mut sim, true);
    stand_until(&mut sim, 40.0, 60 * 30, "a rake's tell", |sim| {
        matches!(wolf(sim).view.performing, Some((Move::Rake, false)))
    });
    untouchable_player(&mut sim, false);
    let (_, before) = player(&mut sim);
    stand_until(&mut sim, 40.0, 90, "the rake to land", |sim| player(sim).1 < before);
}

/// Far from him, he lunges across the room at you, and skids on after it.
#[test]
fn far_away_he_lunges_across_the_room() {
    let mut sim = arena();
    untouchable_player(&mut sim, true);
    stand_far_until(&mut sim, 60 * 30, "a lunge", |sim| {
        matches!(wolf(sim).view.performing, Some((Move::Lunge, true)))
    });
    let from = wolf(&mut sim).kin.pos.x;
    stand_far_until(&mut sim, 60 * 2, "the lunge to end", |sim| !matches!(wolf(sim).view.performing, Some((Move::Lunge, true))));
    let w = wolf(&mut sim);
    assert!((w.kin.pos.x - from).abs() > 300.0, "the lunge carried him only {} across the room", (w.kin.pos.x - from).abs());
}

/// The hammer sends a shock along the floor, and his fists stay in it: the
/// window.
#[test]
fn the_hammer_sends_a_shock_along_the_floor_and_his_fists_stick() {
    let mut sim = arena();
    untouchable_player(&mut sim, true);
    stand_until(&mut sim, 40.0, 60 * 40, "the hammer", |sim| {
        matches!(wolf(sim).view.performing, Some((Move::Hammer, true)))
    });
    step_until(&mut sim, 10, "a shock to leave his fists", |sim| wolf(sim).view.shock_rolling);
    step_until(&mut sim, 60, "his fists to stick in the floor", |sim| wolf(sim).view.stuck);
}

/// Every cycle closes on the galvanize: lightning on the mark under where you
/// stood, then he is flat — the long window, and his body is safe to touch.
#[test]
fn the_galvanize_strikes_where_you_stood_and_leaves_him_flat() {
    let mut sim = arena();
    untouchable_player(&mut sim, true);
    step_until(&mut sim, 60 * 40, "the galvanize's tell", |sim| {
        matches!(wolf(sim).view.performing, Some((Move::Galvanize, false)))
    });
    // Stand still on the mark (where the player stood as it was set).
    untouchable_player(&mut sim, false);
    let (_, before) = player(&mut sim);
    step_until(&mut sim, 60 * 3, "the lightning to strike the player", |sim| player(sim).1 < before);
    untouchable_player(&mut sim, true);
    step_until(&mut sim, 60 * 2, "him to go flat", |sim| wolf(sim).view.flat);
    // Flat: a window, not a flinch, and touching him does not hurt.
    let w = wolf(&mut sim);
    let (trunk, _) = module::trunk();
    untouchable_player(&mut sim, false);
    let (_, before) = player(&mut sim);
    for _ in 0..60 {
        place_player(&mut sim, w.kin.pos + ae::Vec2::new(trunk.x * w.side, trunk.y + 20.0));
        sim.step(AgentAction::default());
    }
    assert!(wolf(&mut sim).view.flat, "the flatline lasted under a second");
    assert_eq!(player(&mut sim).1, before, "his body hurt a player who touched him while he was flat");
}

/// Wounded past the transition, he is unleashed: far away he bounds off the
/// wall, and a howl calls the pack.
#[test]
fn unleashed_he_bounds_off_the_wall_and_his_howl_calls_the_pack() {
    let mut sim = arena();
    untouchable_player(&mut sim, true);
    wound(&mut sim, 0.5);
    step_until(&mut sim, 60 * 6, "the leash to break", |sim| wolf(sim).view.unleashed);
    let mut highest = f32::MAX;
    stand_far_until(&mut sim, 60 * 60, "a bound", |sim| matches!(wolf(sim).view.performing, Some((Move::Bound, true))));
    let standing = wolf(&mut sim).kin.pos.y;
    stand_far_until(&mut sim, 60 * 3, "the bound to end", |sim| {
        let w = wolf(sim);
        highest = highest.min(w.kin.pos.y);
        !matches!(w.view.performing, Some((Move::Bound, true)))
    });
    assert!(standing - highest > 80.0, "the bound rose only {} off the floor", standing - highest);
    assert!(wolf(&mut sim).view.winded, "he did not land winded");
    // The howl answers a player who stands behind him; unleashed, it calls
    // the pack.
    for _ in 0..60 * 60 {
        let w = wolf(&mut sim);
        let hall = w.view.hall.expect("room");
        let (kin, _) = player(&mut sim);
        let x = (w.kin.pos.x - w.side * 40.0).clamp(hall.left + 30.0, hall.right - 30.0);
        place_player(&mut sim, ae::Vec2::new(x, hall.floor - kin.size.y * 0.5 - 1.0));
        sim.step(AgentAction::default());
        if summoned(&mut sim) > 0 {
            return;
        }
    }
    panic!("no howl called the pack; last: {:?}", wolf(&mut sim));
}

/// A player on a gallery over his head is answered by the arc lash: he
/// aims along the line to them, and the current goes down it.
#[test]
fn a_player_on_a_gallery_is_answered_by_the_arc_lash() {
    let mut sim = arena();
    untouchable_player(&mut sim, true);
    let hall = wolf(&mut sim).view.hall.expect("room");
    // `gallery_east` (`castle_godwin_throne_area.ron`): 1248-1504 wide, its top
    // 160 over the floor; the room's west wall is 16 thick.
    let room_x = hall.left - 16.0;
    let (kin, _) = player(&mut sim);
    let on_gallery = ae::Vec2::new(room_x + 1376.0, hall.floor - 160.0 - kin.size.y * 0.5 - 1.0);
    let mut lashed = false;
    for _ in 0..60 * 60 {
        place_player(&mut sim, on_gallery);
        sim.step(AgentAction::default());
        if matches!(wolf(&mut sim).view.performing, Some((Move::Lash, false))) {
            lashed = true;
            break;
        }
    }
    assert!(lashed, "he never answered a player above him with the lash; last: {:?}", wolf(&mut sim));
    untouchable_player(&mut sim, false);
    let (_, before) = player(&mut sim);
    let mut bolt_seen = false;
    for _ in 0..90 {
        place_player(&mut sim, on_gallery);
        sim.step(AgentAction::default());
        bolt_seen |= a_force_ray_strikes(&mut sim);
        if player(&mut sim).1 < before {
            assert!(bolt_seen, "the lash hurt, but not as a ray drawn as force lightning");
            return;
        }
    }
    panic!("the lash missed a player standing still on the gallery; last: {:?}", wolf(&mut sim));
}

/// One lash is one ray: it starts once, so the body it reaches is hit by it
/// once. A second start would be a new ray, with no memory of the first one's
/// hits.
#[test]
fn one_lash_is_one_ray() {
    let mut sim = arena();
    untouchable_player(&mut sim, true);
    let hall = wolf(&mut sim).view.hall.expect("room");
    let room_x = hall.left - 16.0;
    let (kin, _) = player(&mut sim);
    let on_gallery = ae::Vec2::new(room_x + 1376.0, hall.floor - 160.0 - kin.size.y * 0.5 - 1.0);
    let mut rays = std::collections::BTreeSet::new();
    let mut struck = false;
    for _ in 0..60 * 60 {
        place_player(&mut sim, on_gallery);
        sim.step(AgentAction::default());
        let performing = wolf(&mut sim).view.performing;
        if performing == Some((Move::Lash, true)) {
            struck = true;
            rays.extend(force_rays(&mut sim));
        } else if struck {
            break;
        }
    }
    assert!(struck, "he never struck with the lash; last: {:?}", wolf(&mut sim));
    assert_eq!(rays.len(), 1, "one lash cast {} rays: {rays:?}", rays.len());
}

/// The live ray strikes drawn as force lightning.
fn force_rays(sim: &mut Platformer2dSimHarness) -> Vec<Entity> {
    let world = sim.world_mut();
    let mut rays = world.query::<(Entity, &ambition_platformer2d::combat::strike::RayStrike)>();
    rays.iter(world).filter(|(_, ray)| &*ray.look == "force").map(|(entity, _)| entity).collect()
}

/// Whether a ray strike drawn as force lightning is live: the lash's (a
/// ray from his neck down the line, its look the presentation's).
fn a_force_ray_strikes(sim: &mut Platformer2dSimHarness) -> bool {
    let world = sim.world_mut();
    let mut rays = world.query::<&ambition_platformer2d::combat::strike::RayStrike>();
    let live = rays.iter(world).any(|ray| &*ray.look == "force" && ray.length > 100.0);
    live
}

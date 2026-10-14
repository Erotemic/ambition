//! The Hunny Horror's fight, played headlessly in the real arena: the authored
//! room, the scripted pattern, and the conducted module that performs it
//! (`ambition_content_modules::hunny`).

#![cfg(feature = "rl_sim")]

use ambition_app::AmbitionSim;
use ambition_app::{AgentAction, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode};
use ambition_content::bosses::hunny::conductor_module as module;
use ambition_content::bosses::hunny::{conductor_of, HunnyView, Move, HUNNY_ID};
use ambition_platformer2d::boss_encounter::BossConfig;
use ambition_platformer2d::characters::actor::{BodyHealth, Invulnerability};
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
use bevy::prelude::*;

const ARENA: &str = "hunny_horror_arena";

fn arena() -> Platformer2dSimHarness {
    let opts = Platformer2dSimHarnessOptions::default()
        .with_timestep(TimestepMode::fixed_60hz())
        // ⚠ REQUIRED: the tolerant road falls back to the start room, which has
        // no bear, and every assertion below would be vacuous.
        .with_required_start_room(ARENA);
    let mut sim = Platformer2dSimHarness::new_with_options(opts).expect("the Hunny Horror's arena builds headlessly");
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
struct Bear {
    kin: ae::BodyKinematics,
    side: f32,
    view: HunnyView,
}

fn bear(sim: &mut Platformer2dSimHarness) -> Bear {
    let world = sim.world_mut();
    let (entity, kin, side) = world
        .query::<(Entity, &BossConfig, &ae::BodyKinematics, &ambition_platformer2d::boss_encounter::conduct::ConductedFacing)>()
        .iter(world)
        .find(|(_, config, ..)| config.behavior.id == HUNNY_ID)
        .map(|(entity, _, kin, side)| (entity, kin.clone(), side.0))
        .expect("the bear is in the arena, conducted");
    let view = conductor_of(world, entity).expect("his conductor has measured the room");
    Bear { kin, side, view }
}

/// Wound him to `fraction` of his health: past phase 1 at a half, enraged
/// near the end.
fn wound(sim: &mut Platformer2dSimHarness, fraction: f32) {
    let world = sim.world_mut();
    let mut q = world.query::<(&BossConfig, &mut BodyHealth)>();
    let (_, mut health) = q.iter_mut(world).find(|(config, _)| config.behavior.id == HUNNY_ID).expect("the bear");
    health.health.current = (health.max() as f32 * fraction).ceil() as i32;
}

fn honey(sim: &mut Platformer2dSimHarness) -> Vec<ae::Vec2> {
    let world = sim.world_mut();
    let mut q = world.query::<(&ambition_platformer2d::projectiles::ProjectileVisualId, &ae::BodyKinematics)>();
    q.iter(world).filter(|(id, _)| id.0 == "honey_glob").map(|(_, kin)| kin.pos).collect()
}

fn held(sim: &mut Platformer2dSimHarness) -> bool {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<&ambition_platformer2d::combat::capture::CapturedBy, PrimaryPlayerOnly>();
    q.iter(world).next().is_some()
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
    panic!("waited {frames} frames for {what}; last: {:?}", bear(sim));
}

/// Stand on the floor in front of him, close, every tick, until `until` says
/// so: his swipe and his hug are near moves.
fn stand_in_front_until(
    sim: &mut Platformer2dSimHarness,
    frames: usize,
    what: &str,
    mut until: impl FnMut(&mut Platformer2dSimHarness) -> bool,
) {
    for _ in 0..frames {
        let b = bear(sim);
        let hall = b.view.hall.expect("room");
        let (kin, _) = player(sim);
        let (reach, _) = module::swipe_box();
        let x = b.kin.pos.x + b.side * reach.x;
        place_player(sim, ae::Vec2::new(x, hall.floor - kin.size.y * 0.5 - 1.0));
        sim.step(AgentAction::default());
        if until(sim) {
            return;
        }
    }
    panic!("waited {frames} frames for {what}; last: {:?}", bear(sim));
}

/// He stands on the floor: his drawn soles on it, every tick, walking or not.
#[test]
fn the_bear_stands_on_the_floor() {
    let mut sim = arena();
    untouchable_player(&mut sim, true);
    let floor = bear(&mut sim).view.hall.expect("he measured his room").floor;
    for _ in 0..120 {
        sim.step(AgentAction::default());
        let b = bear(&mut sim);
        let feet = b.kin.pos.y + module::feet_below();
        assert!((feet - floor).abs() < 0.5, "his feet are {feet}, the floor {floor}: {b:?}");
    }
}

/// Sweet, he is harmless: a player inside him while he walks or stands is not
/// hurt. (The rule of the fight: his face is the tell.)
#[test]
fn sweet_he_does_not_hurt_to_touch() {
    let mut sim = arena();
    let mut quiet = 0;
    for _ in 0..60 * 20 {
        let b = bear(&mut sim);
        if b.view.performing.is_none() && !b.view.dazed && !b.view.squeezing {
            let (trunk, _) = module::trunk();
            let (_, before) = player(&mut sim);
            place_player(&mut sim, b.kin.pos + ae::Vec2::new(trunk.x * b.side, trunk.y + 30.0));
            sim.step(AgentAction::default());
            let (_, after) = player(&mut sim);
            if bear(&mut sim).view.performing.is_none() {
                assert_eq!(after, before, "the sweet bear hurt a player who touched him: {:?}", bear(&mut sim));
                quiet += 1;
            }
        } else {
            sim.step(AgentAction::default());
        }
        if quiet >= 30 {
            return;
        }
    }
    panic!("he was sweet for only {quiet} ticks in twenty seconds");
}

/// When he turns into the monster, his body hurts to touch.
#[test]
fn the_monster_hurts_to_touch() {
    let mut sim = arena();
    untouchable_player(&mut sim, true);
    step_until(&mut sim, 60 * 30, "a tell", |sim| matches!(bear(sim).view.performing, Some((_, false))));
    let b = bear(&mut sim);
    let (trunk, _) = module::trunk();
    untouchable_player(&mut sim, false);
    let (_, before) = player(&mut sim);
    place_player(&mut sim, b.kin.pos + ae::Vec2::new(trunk.x * b.side, trunk.y + 30.0));
    step_until(&mut sim, 20, "his body to hurt the player", |sim| player(sim).1 < before);
}

/// The swipe lands on a player in front of him.
#[test]
fn the_swipe_lands_in_front_of_him() {
    let mut sim = arena();
    untouchable_player(&mut sim, true);
    stand_in_front_until(&mut sim, 60 * 30, "a swipe's tell", |sim| {
        matches!(bear(sim).view.performing, Some((Move::Swipe, false)))
    });
    untouchable_player(&mut sim, false);
    let (_, before) = player(&mut sim);
    stand_in_front_until(&mut sim, 120, "the swipe to land", |sim| player(sim).1 < before);
}

/// The hug: caught in front of him, you are squeezed, then thrown forward and
/// up across the room.
#[test]
fn the_hug_squeezes_you_and_throws_you() {
    let mut sim = arena();
    untouchable_player(&mut sim, true);
    stand_in_front_until(&mut sim, 60 * 60, "his arms to open for a hug", |sim| {
        matches!(bear(sim).view.performing, Some((Move::Hug, false)))
    });
    untouchable_player(&mut sim, false);
    let (_, before) = player(&mut sim);
    stand_in_front_until(&mut sim, 60 * 2, "his arms to close on the player", held);
    for _ in 0..3 {
        sim.step(AgentAction::default());
    }
    let b = bear(&mut sim);
    assert!(b.view.squeezing, "the player is held but he is not squeezing: {b:?}");
    let (kin, _) = player(&mut sim);
    let ahead = (kin.pos.x - b.kin.pos.x) * b.side;
    assert!(ahead > 10.0, "the held player is {ahead} ahead of him, not in his arms");
    let mut thrown = None;
    for _ in 0..60 * 3 {
        sim.step(AgentAction::default());
        if !held(&mut sim) {
            thrown = Some(player(&mut sim).0);
            break;
        }
    }
    let thrown = thrown.expect("he never let go");
    assert!(
        thrown.vel.x * b.side > 250.0 && thrown.vel.y < -250.0,
        "the player left his arms at {:?}, not thrown forward and up",
        thrown.vel
    );
    let (_, after) = player(&mut sim);
    assert!(after <= before - 3, "the hug took {} health, not its squeezes and its throw", before - after);
}

/// Mashing gets you out of the hug before the throw.
#[test]
fn mashing_breaks_the_hug() {
    let mut sim = arena();
    untouchable_player(&mut sim, true);
    stand_in_front_until(&mut sim, 60 * 60, "his arms to open for a hug", |sim| {
        matches!(bear(sim).view.performing, Some((Move::Hug, false)))
    });
    untouchable_player(&mut sim, false);
    stand_in_front_until(&mut sim, 60 * 2, "his arms to close on the player", held);
    let mut freed_after = None;
    for tick in 0..60 * 3 {
        let mash = tick % 2 == 0;
        sim.step(AgentAction { jump: mash, attack: mash, dash: mash, ..AgentAction::default() });
        if !held(&mut sim) {
            freed_after = Some(tick);
            break;
        }
    }
    let freed_after = freed_after.expect("mashing never freed the player");
    assert!(freed_after < (1.4 * 60.0) as usize, "mashing freed the player only after {freed_after} ticks");
    for _ in 0..3 {
        sim.step(AgentAction::default());
    }
    assert!(!bear(&mut sim).view.squeezing, "the player broke free and he squeezes on");
}

/// Far away, he lobs honey at you; each glob lands in a puddle that slows
/// whoever stands in it, and does not hurt.
#[test]
fn far_away_he_lobs_honey_that_lies_in_puddles_that_slow_you() {
    let mut sim = arena();
    untouchable_player(&mut sim, true);
    let b = bear(&mut sim);
    let hall = b.view.hall.expect("room");
    let far = if b.kin.pos.x > (hall.left + hall.right) * 0.5 { hall.left + 300.0 } else { hall.right - 300.0 };
    let (kin, _) = player(&mut sim);
    let stand = ae::Vec2::new(far, hall.floor - kin.size.y * 0.5 - 1.0);
    for _ in 0..60 * 40 {
        place_player(&mut sim, stand);
        sim.step(AgentAction::default());
        if matches!(bear(&mut sim).view.performing, Some((Move::Spit, true))) {
            break;
        }
    }
    assert!(matches!(bear(&mut sim).view.performing, Some((Move::Spit, true))), "he never spat at a far player");
    step_until(&mut sim, 10, "the globs to fly", |sim| !honey(sim).is_empty());
    step_until(&mut sim, 90, "the puddles to lie", |sim| bear(sim).view.puddles);
    untouchable_player(&mut sim, false);
    let (_, before) = player(&mut sim);
    // In the puddle: slowed.
    place_player(&mut sim, stand);
    for _ in 0..3 {
        sim.step(AgentAction::default());
    }
    assert!(slowed(&mut sim) < 0.99, "a player standing in a honey puddle is not slowed");
    // And it holds: running out of it is slower than running on clear floor.
    let run = |sim: &mut Platformer2dSimHarness| {
        let x0 = player(sim).0.pos.x;
        for _ in 0..20 {
            sim.step(AgentAction { move_x: 1.0, right_pressed: true, ..AgentAction::default() });
        }
        player(sim).0.pos.x - x0
    };
    place_player(&mut sim, stand);
    sim.step(AgentAction::default());
    let stuck = run(&mut sim);
    let (_, after) = player(&mut sim);
    assert_eq!(after, before, "the honey puddle hurt the player; it should only hold them");
    // Wait out the honey, then run the same way on clear floor, out of his
    // reach (he walks to you; a blow would carry the player).
    untouchable_player(&mut sim, true);
    let clear = |sim: &mut Platformer2dSimHarness| {
        let mid = (hall.left + hall.right) * 0.5;
        let x = if bear(sim).kin.pos.x > mid { hall.left + 150.0 } else { mid + 150.0 };
        ae::Vec2::new(x, stand.y)
    };
    for _ in 0..60 * 6 {
        let at = clear(&mut sim);
        place_player(&mut sim, at);
        sim.step(AgentAction::default());
        if !bear(&mut sim).view.puddles && slowed(&mut sim) >= 1.0 {
            break;
        }
    }
    let at = clear(&mut sim);
    place_player(&mut sim, at);
    sim.step(AgentAction::default());
    let free = run(&mut sim);
    assert!(stuck < free * 0.7, "in honey the player ran {stuck}, on clear floor {free}");
}

/// How much of its speed the player keeps now.
fn slowed(sim: &mut Platformer2dSimHarness) -> f32 {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<&ambition_platformer2d::characters::actor::BodyCombat, PrimaryPlayerOnly>();
    q.iter(world).next().expect("the player").move_speed_scale()
}

/// The red balloon: he rises on it over the room, lets go and pounces at
/// where you are, and sits dazed where he lands — the long punish window.
#[test]
fn he_rises_on_his_balloon_and_pounces_where_you_are() {
    let mut sim = arena();
    untouchable_player(&mut sim, true);
    step_until(&mut sim, 60 * 40, "him to rise on his balloon", |sim| bear(sim).view.rising);
    let floor = bear(&mut sim).view.hall.expect("room").floor;
    let mut highest = f32::MAX;
    let mut balloon_with_him = false;
    for _ in 0..90 {
        sim.step(AgentAction::default());
        let b = bear(&mut sim);
        if !b.view.rising {
            break;
        }
        highest = highest.min(b.kin.pos.y);
        let balloons = projectiles(&mut sim, "red_balloon");
        balloon_with_him |= balloons.iter().any(|at| at.y < b.kin.pos.y && (at.x - b.kin.pos.x).abs() < 80.0);
    }
    let standing = floor - module::feet_below();
    assert!(standing - highest > 150.0, "his balloon lifted him only {} off his standing height", standing - highest);
    assert!(balloon_with_him, "no red balloon rose over him");
    // Where the player is when he lets go is where he pounces.
    let (kin, _) = player(&mut sim);
    step_until(&mut sim, 60 * 2, "him to land", |sim| bear(sim).view.dazed);
    let b = bear(&mut sim);
    assert!((b.kin.pos.x - kin.pos.x).abs() < 120.0, "he landed at {}, the player was at {}", b.kin.pos.x, kin.pos.x);
    // He sits there: a window, not a flinch.
    for _ in 0..90 {
        sim.step(AgentAction::default());
    }
    assert!(bear(&mut sim).view.dazed, "the daze lasted under a second and a half");
}

/// The pounce lands on a player who stands still.
#[test]
fn the_pounce_lands_on_a_player_who_stands_still() {
    let mut sim = arena();
    untouchable_player(&mut sim, true);
    step_until(&mut sim, 60 * 40, "him to rise on his balloon", |sim| bear(sim).view.rising);
    untouchable_player(&mut sim, false);
    let (_, before) = player(&mut sim);
    step_until(&mut sim, 60 * 3, "the pounce to land on the player", |sim| player(sim).1 < before);
}

fn projectiles(sim: &mut Platformer2dSimHarness, visual: &str) -> Vec<ae::Vec2> {
    let world = sim.world_mut();
    let mut q = world.query::<(&ambition_platformer2d::projectiles::ProjectileVisualId, &ae::BodyKinematics)>();
    q.iter(world).filter(|(id, _)| id.0 == visual).map(|(_, kin)| kin.pos).collect()
}

//! The robot's fireball against the Mockingbird, played in its real sky with
//! rollback on: what each of the robot's ranged inputs and its blade take
//! from the Mockingbird while it hangs winded after its dive (its guard is
//! down only then).

#![cfg(feature = "rl_sim")]

use ambition_app::AmbitionSim;
use ambition_app::{AgentAction, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode};
use ambition_content::bosses::mockingbird::{conductor_of, MOCKINGBIRD_ID};
use ambition_platformer2d::boss_encounter::{BossConfig, BossEncounter};
use ambition_platformer2d::characters::actor::{BodyHealth, Invulnerability};
use ambition_platformer2d::combat::components::DamageableVolumes;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
use bevy::prelude::*;

const SKY: &str = "mockingbird_sky";

fn sky() -> Platformer2dSimHarness {
    let opts = Platformer2dSimHarnessOptions::default()
        .with_timestep(TimestepMode::fixed_60hz())
        // Rollback on, as the game plays it: GGRS drives the schedule, and a
        // check distance of 0 saves and resimulates nothing.
        .with_sync_test_rollback_settings(0, 12)
        // ⚠ REQUIRED: the tolerant road falls back to the start room, which
        // has no Mockingbird.
        .with_required_start_room(SKY);
    let mut sim = Platformer2dSimHarness::new_with_options(opts).expect("the sky builds headlessly");
    for _ in 0..30 {
        sim.step(AgentAction::default());
    }
    let world = sim.world_mut();
    let mut q = world.query_filtered::<&mut BodyHealth, PrimaryPlayerOnly>();
    for mut health in q.iter_mut(world) {
        health.health.invulnerable.set(Invulnerability::SCRIPTED, true);
    }
    sim
}

struct Bird {
    hp: i32,
    guarded: bool,
    stunned: bool,
    /// The box around its damageable parts.
    hurt: Option<ae::Aabb>,
}

fn bird(sim: &mut Platformer2dSimHarness) -> Bird {
    let world = sim.world_mut();
    let (entity, hp, guarded, hurt) = world
        .query::<(Entity, &BossConfig, &BodyHealth, &BossEncounter, Option<&DamageableVolumes>)>()
        .iter(world)
        .find(|(_, config, ..)| config.behavior.id == MOCKINGBIRD_ID)
        .map(|(entity, _, health, status, volumes)| {
            let hurt = volumes.and_then(|v| {
                v.volumes.iter().map(|part| part.bounds()).reduce(|a, b| ae::Aabb {
                    min: a.min.min(b.min),
                    max: a.max.max(b.max),
                })
            });
            (entity, health.current(), status.guarded, hurt)
        })
        .expect("the Mockingbird is in its sky");
    let stunned = conductor_of(world, entity).expect("its conductor").stunned;
    Bird { hp, guarded, stunned, hurt }
}

fn player_entity(sim: &mut Platformer2dSimHarness) -> Entity {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<Entity, PrimaryPlayerOnly>();
    q.iter(world).next().expect("the player")
}

/// Hold the player at `at`, still, facing `facing`.
fn hold_player(sim: &mut Platformer2dSimHarness, at: ae::Vec2, facing: f32) {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<(ae::BodyClusterQueryData, &mut ambition_platformer2d::actor::MotionModel), PrimaryPlayerOnly>();
    let (mut clusters, mut model) = q.iter_mut(world).next().expect("the player");
    let mut clusters = clusters.as_clusters_mut();
    ae::movement::transit_body(&mut model, &mut clusters, at, ae::movement::TransitVelocity::Zero);
    let mut kin = world.query_filtered::<&mut ae::BodyKinematics, PrimaryPlayerOnly>();
    kin.iter_mut(world).next().expect("the player").facing = facing;
}

/// Wait (holding the player far across the sky) until it hangs winded.
fn until_it_hangs_winded(sim: &mut Platformer2dSimHarness) {
    for _ in 0..60 * 25 {
        let b = bird(sim);
        if b.stunned && !b.guarded {
            return;
        }
        let entity = {
            let world = sim.world_mut();
            world
                .query::<(Entity, &BossConfig)>()
                .iter(world)
                .find(|(_, c)| c.behavior.id == MOCKINGBIRD_ID)
                .map(|(e, _)| e)
                .expect("the Mockingbird")
        };
        let room = conductor_of(sim.world_mut(), entity).and_then(|view| view.room).expect("its room");
        place_player(sim, ae::Vec2::new(room.x * 0.78, room.y * 0.45), -1.0);
        sim.step(AgentAction::default());
    }
    panic!("it never hung winded");
}

/// What one way of fighting took from it in its winded window.
#[derive(Debug)]
struct Window {
    seconds: f32,
    damage: i32,
    presses: u32,
    shots: u32,
}

/// Where to stand `standoff` world units off the side of its hurt box that has
/// the more sky beside it, at the height of its centre, and which way to face.
fn beside_it(sim: &mut Platformer2dSimHarness, hurt: ae::Aabb, standoff: f32) -> (ae::Vec2, f32) {
    let entity = {
        let world = sim.world_mut();
        world
            .query::<(Entity, &BossConfig)>()
            .iter(world)
            .find(|(_, c)| c.behavior.id == MOCKINGBIRD_ID)
            .map(|(e, _)| e)
            .expect("the Mockingbird")
    };
    let room = conductor_of(sim.world_mut(), entity).and_then(|view| view.room).expect("its room");
    let y = (hurt.min.y + hurt.max.y) * 0.5;
    if room.x - hurt.max.x > hurt.min.x {
        (ae::Vec2::new(hurt.max.x + standoff, y), -1.0)
    } else {
        (ae::Vec2::new(hurt.min.x - standoff, y), 1.0)
    }
}

/// Fight it from `standoff` world units off its hurt box (see [`beside_it`])
/// for as long as it hangs winded. `input` gives the action for each tick of
/// the window.
fn fight(standoff: f32, mut input: impl FnMut(u32) -> AgentAction) -> Window {
    let mut sim = sky();
    until_it_hangs_winded(&mut sim);
    let player = player_entity(&mut sim);
    let start = bird(&mut sim);
    let hurt = start.hurt.expect("winded, it has a damageable part");
    let (at, facing) = beside_it(&mut sim, hurt, standoff);
    eprintln!("its hurt box {hurt:?}; the player stands at {at:?} facing {facing}");
    let (mut presses, mut shots, mut tick) = (0u32, 0u32, 0u32);
    let mut last_cooldown = 0.0f32;
    loop {
        let b = bird(&mut sim);
        if !b.stunned || b.guarded || tick > 60 * 6 {
            break;
        }
        hold_player(&mut sim, at, facing);
        let action = input(tick);
        if action.attack || action.projectile {
            presses += 1;
        }
        sim.step(action);
        // A sync test replays frames, so messages repeat; the spawner's
        // cooldown rising is one shot fired, read from the settled state.
        let cooldown = sim
            .world()
            .get::<ambition_platformer2d::projectiles::PlayerProjectileState>(player)
            .map_or(0.0, |state| state.spawner.cooldown_remaining);
        if cooldown > last_cooldown {
            shots += 1;
        }
        last_cooldown = cooldown;
        tick += 1;
    }
    // Shots still in the air land after the window closes only if it is
    // still open, so the count stops here.
    Window {
        seconds: tick as f32 / 60.0,
        damage: start.hp - bird(&mut sim).hp,
        presses,
        shots,
    }
}

fn tap_every(period: u32) -> impl FnMut(u32) -> AgentAction {
    move |tick| match tick % period {
        0 => AgentAction { projectile: true, projectile_held: true, ..Default::default() },
        1 => AgentAction { projectile_released: true, ..Default::default() },
        _ => AgentAction::default(),
    }
}

fn charge(hold: u32) -> impl FnMut(u32) -> AgentAction {
    move |tick| match tick % (hold + 2) {
        0 => AgentAction { projectile: true, projectile_held: true, ..Default::default() },
        t if t < hold => AgentAction { projectile_held: true, ..Default::default() },
        t if t == hold => AgentAction { projectile_released: true, ..Default::default() },
        _ => AgentAction::default(),
    }
}

fn swing_every(period: u32) -> impl FnMut(u32) -> AgentAction {
    move |tick| AgentAction { attack: tick % period == 0, ..Default::default() }
}

/// A measurement: each way of fighting, its damage per press and per second
/// of the winded window.
#[test]
#[ignore = "a measurement; run with --ignored"]
fn measure_each_way_of_fighting_the_winded_mockingbird() {
    let report = |name: &str, w: Window| {
        eprintln!(
            "{name:>28}: {:>3} damage in {:.2} s = {:5.2}/s; {} presses, {} shots, {:.2} damage per press",
            w.damage,
            w.seconds,
            w.damage as f32 / w.seconds.max(1e-3),
            w.presses,
            w.shots,
            w.damage as f32 / (w.presses.max(1)) as f32,
        );
    };
    report("blade, a press each 0.25 s", fight(10.0, swing_every(15)));
    report("fireball tap each 0.10 s", fight(60.0, tap_every(6)));
    report("fireball tap each 0.20 s", fight(60.0, tap_every(12)));
    report("fireball tap each 0.33 s", fight(60.0, tap_every(20)));
    report("fireball tap each 0.33 s far", fight(200.0, tap_every(20)));
    report("fireball held 0.40 s", fight(60.0, charge(24)));
    report("fireball held 0.90 s", fight(60.0, charge(54)));
}

/// Stand the player at `at`, still, facing `facing`, and make that world the
/// rollback baseline.
///
/// ⛔ A write through `world_mut` under a sync test with a check distance of
/// 1 or more is erased by the next rewind: the resimulation does not repeat
/// it. Without the rebase the player is not where the fixture put it.
fn place_player(sim: &mut Platformer2dSimHarness, at: ae::Vec2, facing: f32) {
    hold_player(sim, at, facing);
    sim.rebase_rollback_history().expect("the placed player becomes the rollback baseline");
}

/// The Mockingbird's hit points through one winded window, tick by tick, for
/// one way of fighting, under a GGRS sync test with `check_distance`.
///
/// The player flies, so it hovers where it is put and needs no hold. It is
/// put beside the bird once, before the window. In the window, only the input
/// moves it, so a resimulated tick does what the first run of that tick did.
fn hp_through_the_window(check_distance: usize, standoff: f32, mut input: impl FnMut(u32) -> AgentAction) -> Vec<i32> {
    let opts = Platformer2dSimHarnessOptions::default()
        .with_timestep(TimestepMode::fixed_60hz())
        .with_sync_test_rollback_settings(check_distance, 12)
        .with_required_start_room(SKY);
    let mut sim = Platformer2dSimHarness::new_with_options(opts).expect("the sky builds headlessly");
    for _ in 0..30 {
        sim.step(AgentAction::default());
    }
    {
        let world = sim.world_mut();
        let mut q = world.query_filtered::<&mut BodyHealth, PrimaryPlayerOnly>();
        for mut health in q.iter_mut(world) {
            health.health.invulnerable.set(Invulnerability::SCRIPTED, true);
        }
    }
    sim.grant_flight();
    until_it_hangs_winded(&mut sim);
    let hurt = bird(&mut sim).hurt.expect("winded, it has a damageable part");
    let (at, facing) = beside_it(&mut sim, hurt, standoff);
    place_player(&mut sim, at, facing);
    let loads_before = sim.rollback_execution_stats().map_or(0, |stats| stats.lifetime_load_runs);
    let mut hp = Vec::new();
    for tick in 0..75u32 {
        let b = bird(&mut sim);
        assert!(b.stunned && !b.guarded, "premise: it hangs winded through the window (tick {tick})");
        hp.push(b.hp);
        sim.step(input(tick));
    }
    hp.push(bird(&mut sim).hp);
    sim.rollback_health().expect("the resimulated ticks agree with the first run");
    let loads = sim.rollback_execution_stats().map_or(0, |stats| stats.lifetime_load_runs) - loads_before;
    if check_distance > 0 {
        assert!(loads > 60, "premise: the window was resimulated ({loads} rewinds at distance {check_distance})");
    }
    hp
}

/// Under rollback, every hit in the winded window lands, not only the first.
///
/// A GGRS sync test rewinds `check_distance` ticks each tick and plays them
/// again. Correct rollback state makes the replay equal to the first run, so
/// the bird loses the same hit points at each tick with and without the
/// rewinds.
#[test]
fn every_hit_in_the_winded_window_lands_under_rollback() {
    for (name, standoff, input) in [
        ("blade, a press each 0.25 s", 10.0, Box::new(swing_every(15)) as Box<dyn FnMut(u32) -> AgentAction>),
        ("fireball tap each 0.33 s", 60.0, Box::new(tap_every(20))),
    ] {
        let mut input = input;
        let mut tape = Vec::new();
        let unrewound = hp_through_the_window(0, standoff, |tick| {
            let action = input(tick);
            tape.push(action);
            action
        });
        let landed = unrewound.windows(2).filter(|pair| pair[1] < pair[0]).count();
        assert!(landed >= 3, "{name}: premise: three or more hits land without rollback; hp {unrewound:?}");
        for distance in [1usize, 4] {
            let rewound = hp_through_the_window(distance, standoff, |tick| tape[tick as usize]);
            assert_eq!(
                rewound, unrewound,
                "{name}: at check distance {distance} the bird's hit points differ from the run without rollback"
            );
        }
    }
}

/// A fireball that reaches the winded Mockingbird takes its damage from it:
/// a tap the fireball's, a held shot its charge tier's. Its guard is down
/// while it hangs winded, so nothing turns the shot.
#[test]
fn a_fireball_takes_its_damage_from_the_winded_mockingbird() {
    use ambition_platformer2d::projectiles::ProjectileKind;
    for hold in [1u32, 24] {
        let mut sim = sky();
        until_it_hangs_winded(&mut sim);
        let start = bird(&mut sim);
        let (at, facing) = beside_it(&mut sim, start.hurt.expect("a damageable part"), 60.0);
        let player = player_entity(&mut sim);
        let tier = sim
            .world()
            .get::<ambition_platformer2d::projectiles::PlayerProjectileState>(player)
            .expect("the robot charges its fireball")
            .charge_tuning
            .tier_for_hold(hold as f32 / 60.0);
        let expected = ProjectileKind::Fireball
            .charged_spec(ProjectileKind::Fireball.spec(ae::Vec2::ZERO, ae::Vec2::X, 1.0), tier)
            .damage;
        let mut input = charge(hold);
        let mut taken = 0;
        for tick in 0..hold + 40 {
            let b = bird(&mut sim);
            assert!(b.stunned && !b.guarded, "it stopped hanging winded before the shot landed: tick {tick}");
            taken = start.hp - b.hp;
            if taken > 0 {
                break;
            }
            hold_player(&mut sim, at, facing);
            // One shot: the input stops after its release.
            let action = if tick <= hold { input(tick) } else { AgentAction::default() };
            sim.step(action);
        }
        assert_eq!(taken, expected, "a fireball held {hold} ticks (tier {tier}) took {taken}, not its {expected}");
    }
}

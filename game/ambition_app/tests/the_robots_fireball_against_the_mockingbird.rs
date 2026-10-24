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
        hold_player(sim, ae::Vec2::new(room.x * 0.78, room.y * 0.45), -1.0);
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

/// A probe: every hit sent at it in its winded window, and what it did.
/// `PROBE_FIREBALL=1` taps the fireball instead of swinging the blade, and
/// `PROBE_CHECK_DISTANCE=N` runs it under a GGRS sync test with that check
/// distance (none: no rollback). With a distance of 1 or more, only the first
/// strike of the window damaged it (2026-10-24).
#[test]
#[ignore = "a probe; run with --ignored"]
fn probe_the_hits_on_the_winded_mockingbird() {
    use ambition_platformer2d::combat::events::HitEvent;
    let mut opts = Platformer2dSimHarnessOptions::default()
        .with_timestep(TimestepMode::fixed_60hz())
        .with_required_start_room(SKY);
    if let Ok(distance) = std::env::var("PROBE_CHECK_DISTANCE") {
        opts = opts.with_sync_test_rollback_settings(distance.parse().unwrap(), 12);
    }
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
    until_it_hangs_winded(&mut sim);
    let start = bird(&mut sim);
    let hurt = start.hurt.expect("hurt");
    let fireball = std::env::var("PROBE_FIREBALL").is_ok();
    let (at, facing) = beside_it(&mut sim, hurt, if fireball { 60.0 } else { 10.0 });
    let mut cursor = bevy::ecs::message::MessageCursor::<HitEvent>::default();
    for tick in 0..130u32 {
        hold_player(&mut sim, at, facing);
        let action = if fireball {
            tap_every(20)(tick)
        } else {
            AgentAction { attack: tick % 15 == 0, ..Default::default() }
        };
        sim.step(action);
        if fireball && tick < 40 {
            let w = sim.world_mut();
            let shots: Vec<_> = w
                .query_filtered::<&ae::BodyKinematics, With<ambition_platformer2d::projectiles::ProjectileGameplay>>()
                .iter(w)
                .map(|k| (k.pos, k.vel, k.size))
                .collect();
            let me = w.query_filtered::<&ae::BodyKinematics, PrimaryPlayerOnly>().iter(w).next().map(|k| (k.pos, k.facing));
            eprintln!("tick {tick}: me {me:?} shots {shots:?}");
        }
        let b = bird(&mut sim);
        let parts: Vec<_> = {
            let w = sim.world_mut();
            w.query::<(&BossConfig, &DamageableVolumes)>()
                .iter(w)
                .filter(|(c, _)| c.behavior.id == MOCKINGBIRD_ID)
                .flat_map(|(_, v)| v.volumes.iter().map(|p| p.bounds()).collect::<Vec<_>>())
                .collect()
        };
        let messages = sim.world().resource::<Messages<HitEvent>>();
        for hit in cursor.read(messages) {
            eprintln!(
                "tick {tick}: hit {:?} dmg {} target {:?} vol {:?}",
                hit.source, hit.damage, hit.target, hit.volume.bounds()
            );
        }
        if tick < 40 {
            let w = sim.world_mut();
            let playing = w
                .query_filtered::<Option<&ambition_platformer2d::actor::MovePlayback>, PrimaryPlayerOnly>()
                .iter(w)
                .next()
                .flatten()
                .map(|p| p.spec.id.clone());
            let dt = w.resource::<ambition_platformer2d::time::WorldTime>().sim_dt();
            eprintln!("tick {tick}: playing {playing:?} sim_dt {dt}");
        }
        if tick % 10 == 0 || tick < 3 {
            eprintln!("tick {tick}: hp {} guarded {} stunned {} parts {parts:?}", b.hp, b.guarded, b.stunned);
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

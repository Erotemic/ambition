//! Real-ECS headless DUEL harness — start the game in the authored `duel_arena`
//! room and watch the sim play out, exactly as if the player had walked through
//! the basement door into it.
//!
//! This is the "start a room and see how the sim plays out" test for the advanced
//! fighter brain. It builds the full `SandboxSim` app with `start_room =
//! "duel_arena"`, so the room's normal load path (`spawn_room_feature_entities` →
//! `features::stage_room_duel`) auto-stages the fight — a Perfect Cell-ular
//! Automaton (`Enemy`) vs a robot copy of the player (`Boss`), on different
//! factions so the physical-damage rule lets them hurt each other, hostile to
//! each other but not to the observing player. No trigger, no manual staging: the
//! fight is already underway the instant the room exists.
//!
//! It then steps the real game loop for many timesteps and asserts the brain
//! plays a real **neutral / attack / defense** game in the actual engine — both
//! fighters roam and hop (neutral), trade melee (attack), and blink/shield away
//! perceived lunges (defense) — and that the duel resolves (real damage drains
//! HP). This pins the regression where the anti-clump crowding signal counted the
//! opponent and froze both fighters at a standoff.

#![cfg(feature = "rl_sim")]

use ambition_app::{AgentAction, SandboxSim, SandboxSimOptions, TimestepMode};
use ambition_characters::brain::ActorControl;
use ambition_engine_core as ae;
use ambition_gameplay_core::actor::{BodyHealth, BodyKinematics};
use ambition_gameplay_core::features::{FeatureId, DUEL_PCA_ID, DUEL_ROBOT_ID};
use bevy::prelude::World;

/// Per-fighter behavior tally accumulated over the bout, plus spatial extent and
/// HP bookkeeping — the evidence that the fighter played a real game rather than
/// freezing or camping at point-blank.
#[derive(Debug)]
struct FighterLog {
    walk: u32,
    jump: u32,
    melee: u32,
    defense: u32, // blink + shield frames (the reactive defensive verbs)
    min_x: f32,
    max_x: f32,
    max_rise: f32, // peak height gained above spawn (against gravity) — proves hops
    start_hp: i32,
    last_hp: i32,
    spawn_y: f32,
    present: bool,
}

impl Default for FighterLog {
    fn default() -> Self {
        Self {
            walk: 0,
            jump: 0,
            melee: 0,
            defense: 0,
            min_x: f32::MAX,
            max_x: f32::MIN,
            max_rise: 0.0,
            start_hp: 0,
            last_hp: 0,
            spawn_y: 0.0,
            present: false,
        }
    }
}

impl FighterLog {
    fn x_range(&self) -> f32 {
        (self.max_x - self.min_x).max(0.0)
    }
    fn hp_lost(&self) -> i32 {
        self.start_hp - self.last_hp
    }
}

fn observe(world: &mut World, id: &str, log: &mut FighterLog) {
    let mut q = world.query::<(&FeatureId, &BodyKinematics, &BodyHealth, &ActorControl)>();
    let Some((_, kin, hp, control)) = q.iter(world).find(|(f, _, _, _)| f.as_str() == id) else {
        return;
    };
    let f = &control.0;
    if !log.present {
        log.present = true;
        log.spawn_y = kin.pos.y;
        log.start_hp = hp.current();
    }
    if f.locomotion.x.abs() > 0.05 {
        log.walk += 1;
    }
    if f.jump_pressed {
        log.jump += 1;
    }
    if f.melee_pressed {
        log.melee += 1;
    }
    if f.blink_pressed || f.shield_held {
        log.defense += 1;
    }
    log.min_x = log.min_x.min(kin.pos.x);
    log.max_x = log.max_x.max(kin.pos.x);
    // Authored geometry is y-down, so a smaller y is higher: rise = spawn_y - y.
    log.max_rise = log.max_rise.max(log.spawn_y - kin.pos.y);
    log.last_hp = hp.current();
}

/// Walking into the authored duel arena yields a real brain-vs-brain platform
/// fight: both fighters roam and hop, trade melee, defend with blink/shield, and
/// the duel drains HP toward a resolution.
#[test]
fn duel_arena_room_is_a_real_neutral_attack_defense_fight() {
    let mut sim = SandboxSim::new_with_options(
        SandboxSimOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_start_room("duel_arena"),
    )
    .expect("sandbox sim builds in the duel arena");

    // A couple of frames for the room-load spawn requests to materialize.
    for _ in 0..3 {
        sim.step(AgentAction::default());
    }

    let mut pca = FighterLog::default();
    let mut robot = FighterLog::default();
    observe(sim.world_mut(), DUEL_PCA_ID, &mut pca);
    observe(sim.world_mut(), DUEL_ROBOT_ID, &mut robot);
    assert!(pca.present, "PCA auto-spawned on room load");
    assert!(robot.present, "robot auto-spawned on room load");

    // ~12s of sim time — long enough for a full neutral/attack/defense rhythm.
    for _ in 0..720 {
        sim.step(AgentAction::default());
        observe(sim.world_mut(), DUEL_PCA_ID, &mut pca);
        observe(sim.world_mut(), DUEL_ROBOT_ID, &mut robot);
    }

    for (who, log) in [("PCA", &pca), ("robot", &robot)] {
        // NEUTRAL: roams the stage (not frozen, not camped at one x) and hops.
        assert!(
            log.x_range() > 40.0,
            "{who} should roam the arena (x-range {:.0}px) — a frozen/camping fighter barely moves",
            log.x_range()
        );
        assert!(
            log.walk > 60,
            "{who} should spend many frames walking (got {} of ~720)",
            log.walk
        );
        // ATTACK: trades real melee, repeatedly.
        assert!(
            log.melee >= 3,
            "{who} should throw multiple melee swings (got {})",
            log.melee
        );
        // ATTACK lands: the fighter takes real damage over the bout.
        assert!(
            log.hp_lost() >= 3,
            "{who} should take real damage over the duel (lost {} hp)",
            log.hp_lost()
        );
    }

    // NEUTRAL (vertical): at least one fighter uses the air — neutral hops give
    // the brain a vertical mix-up instead of a flat ground shuffle.
    assert!(
        pca.jump + robot.jump >= 2 && pca.max_rise.max(robot.max_rise) > 20.0,
        "the duel should use neutral hops (jumps PCA={} robot={}, peak rise {:.0}px)",
        pca.jump,
        robot.jump,
        pca.max_rise.max(robot.max_rise)
    );

    // DEFENSE: the reactive defensive game (blink-evade / reactive shield) fires —
    // a real fighter doesn't just walk into every swing. Both fighters carry the
    // kit, so require defense from each.
    assert!(
        pca.defense >= 1,
        "PCA should defend (blink/shield) at least once (got {})",
        pca.defense
    );
    assert!(
        robot.defense >= 1,
        "robot should defend (blink/shield) at least once (got {})",
        robot.defense
    );

    // RESOLUTION: the duel is decisive, not an endless stalemate — substantial
    // total HP drained across both fighters.
    let total_hp_lost = pca.hp_lost() + robot.hp_lost();
    assert!(
        total_hp_lost >= 15,
        "the duel should make real progress toward a winner (total hp lost {total_hp_lost}); \
         PCA {} -> {}, robot {} -> {}",
        pca.start_hp,
        pca.last_hp,
        robot.start_hp,
        robot.last_hp
    );
}

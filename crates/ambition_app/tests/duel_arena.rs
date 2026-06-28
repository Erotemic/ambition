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
use ambition_gameplay_core::actor::{
    BodyAbilities, BodyBlinkState, BodyDashState, BodyFlightState, BodyHealth, BodyKinematics,
    BodyShieldState,
};
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
    blink: u32,   // blink-evade presses (the mobile defensive option)
    shield: u32,  // reactive-block frames (the stand-ground option)
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
            blink: 0,
            shield: 0,
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
    if f.blink_pressed {
        log.blink += 1;
    }
    if f.shield_held {
        log.shield += 1;
    }
    log.min_x = log.min_x.min(kin.pos.x);
    log.max_x = log.max_x.max(kin.pos.x);
    // Authored geometry is y-down, so a smaller y is higher: rise = spawn_y - y.
    log.max_rise = log.max_rise.max(log.spawn_y - kin.pos.y);
    log.last_hp = hp.current();
}

/// Body-side ability ENACTMENT tally — proves the brain's emitted intents
/// actually resolve on the body (caps reached it + the shared pipeline enacted
/// them), not just that the brain pressed the button. This is the "are they
/// hooked up" witness.
#[derive(Debug, Default)]
struct AbilityLog {
    caps_blink: bool,
    caps_shield: bool,
    caps_dash: bool,
    caps_fly: bool,
    shield_active_frames: u32,
    dash_window_frames: u32,
    fly_frames: u32,
    fly_toggles: u32,
    blink_events: u32,
    prev_blink_cd: f32,
    present: bool,
}

fn observe_abilities(world: &mut World, id: &str, log: &mut AbilityLog) {
    let mut q = world.query::<(
        &FeatureId,
        &BodyAbilities,
        &BodyShieldState,
        &BodyDashState,
        &BodyFlightState,
        &BodyBlinkState,
        &ActorControl,
    )>();
    let Some((_, abil, shield, dash, flight, blink, control)) =
        q.iter(world).find(|(f, ..)| f.as_str() == id)
    else {
        return;
    };
    if control.0.fly_toggle_pressed {
        log.fly_toggles += 1;
    }
    log.present = true;
    log.caps_blink = abil.abilities.blink;
    log.caps_shield = abil.abilities.shield;
    log.caps_dash = abil.abilities.dash;
    log.caps_fly = abil.abilities.fly;
    if shield.active {
        log.shield_active_frames += 1;
    }
    if dash.timer > 0.0 {
        log.dash_window_frames += 1;
    }
    if flight.fly_enabled {
        log.fly_frames += 1;
    }
    // A blink fires the cooldown from 0 → positive: count that rising edge.
    if blink.cooldown > log.prev_blink_cd + 0.01 {
        log.blink_events += 1;
    }
    log.prev_blink_cd = blink.cooldown;
}

/// The PCA's body/hitbox must come from its AUTHORED sprite metadata — the same
/// resolution the peaceful symmetry-room PCA uses — not the tiny LDtk spawn box
/// the duel hands in. This pins the "same character, consistent body" fix: the
/// duel staging passes a 14x23 half-box, so a body still that small would mean the
/// sprite metadata never applied.
#[test]
fn duel_pca_body_is_sprite_authored_not_the_tiny_ldtk_box() {
    let mut sim = SandboxSim::new_with_options(
        SandboxSimOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_start_room("duel_arena"),
    )
    .expect("sandbox sim builds in the duel arena");
    for _ in 0..5 {
        sim.step(AgentAction::default());
    }
    let world = sim.world_mut();
    let mut q = world.query::<(&FeatureId, &BodyKinematics)>();
    let pca = q
        .iter(world)
        .find(|(f, _)| f.as_str() == DUEL_PCA_ID)
        .map(|(_, kin)| kin.size)
        .expect("duel PCA present");
    // The authored duel box is 14x23 half → 28x46 full. The PCA sprite body is much
    // taller than that; require the body to have grown past the LDtk box, proving
    // the authored sprite collision resolved.
    println!("duel PCA body size = {pca:?}");
    assert!(
        pca.y > 60.0,
        "PCA body should be sprite-authored (tall), not the 46px LDtk box; got {pca:?}"
    );
}

/// The brain emits shield/blink/dash/fly — but does the BODY enact them in the
/// real sim? This pins that the archetype capabilities reach the body AND the
/// shared movement pipeline resolves each ability (no player-only gate). Without
/// this the abilities would be "pressed but inert" — exactly the failure mode the
/// user reported ("I don't see any shield, dash, or blink, fly").
#[test]
fn duel_fighters_actually_enact_their_abilities_on_the_body() {
    let mut sim = SandboxSim::new_with_options(
        SandboxSimOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_start_room("duel_arena"),
    )
    .expect("sandbox sim builds in the duel arena");
    for _ in 0..3 {
        sim.step(AgentAction::default());
    }

    let mut pca = AbilityLog::default();
    let mut robot = AbilityLog::default();
    // ~30s — long enough to observe the slower abilities (dash-to-close, a fly
    // foray) on top of the frequent block.
    for _ in 0..1800 {
        sim.step(AgentAction::default());
        observe_abilities(sim.world_mut(), DUEL_PCA_ID, &mut pca);
        observe_abilities(sim.world_mut(), DUEL_ROBOT_ID, &mut robot);
    }

    for (who, log) in [("PCA", &pca), ("robot", &robot)] {
        println!(
            "{who}: caps[blink={} shield={} dash={} fly={}]  shield_frames={}  dash_frames={}  fly_frames={}  fly_toggles={}  blinks={}",
            log.caps_blink, log.caps_shield, log.caps_dash, log.caps_fly,
            log.shield_active_frames, log.dash_window_frames, log.fly_frames, log.fly_toggles, log.blink_events,
        );
        assert!(log.present, "{who} present");
        // The archetype capabilities must reach the BODY (not just the brain cfg).
        assert!(
            log.caps_blink && log.caps_shield && log.caps_dash && log.caps_fly,
            "{who} body must carry all four abilities (blink={} shield={} dash={} fly={})",
            log.caps_blink, log.caps_shield, log.caps_dash, log.caps_fly,
        );
        // And the body must actually RESOLVE every ability in the real sim — the
        // defensive ones fire frequently, and the damage-triggered regroup makes the
        // fighter dash away and take to the air for high ground.
        assert!(
            log.shield_active_frames > 0,
            "{who}: shield must actually go up on the body (got {} frames)",
            log.shield_active_frames
        );
        assert!(
            log.dash_window_frames > 0,
            "{who}: dash must open a burst window on the body (regroup dash) (got {} frames)",
            log.dash_window_frames
        );
        assert!(
            log.fly_frames > 0,
            "{who}: flight must engage on the body (regroup high-ground) (got {} frames)",
            log.fly_frames
        );
    }
    // Blink resolves on a body (the quick-blink tap commits, not arm-then-cancel).
    // Situational (a committed lunge), so require it across the pair.
    assert!(
        pca.blink_events + robot.blink_events > 0,
        "a blink-evade should resolve on a body at least once (PCA {} + robot {})",
        pca.blink_events,
        robot.blink_events
    );
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

    // ~40s of sim time — the fighters now play a deep spatial game (regroup, fly to
    // high ground, big spacing), so they defend more and the bout breathes; a longer
    // observation lets the full neutral/attack/defense rhythm repeat many times and
    // accumulate a decisive amount of damage.
    for _ in 0..2400 {
        sim.step(AgentAction::default());
        observe(sim.world_mut(), DUEL_PCA_ID, &mut pca);
        observe(sim.world_mut(), DUEL_ROBOT_ID, &mut robot);
    }

    for (who, log) in [("PCA", &pca), ("robot", &robot)] {
        println!(
            "{who}: x-range {:.0}px  walk {}  jump {}  melee {}  blink {}  shield {}  rise {:.0}px  hp {}->{}",
            log.x_range(),
            log.walk,
            log.jump,
            log.melee,
            log.blink,
            log.shield,
            log.max_rise,
            log.start_hp,
            log.last_hp,
        );
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

    // DEFENSE: the LAYERED reactive game (stand-ground block + mobile blink-evade)
    // fires — a real fighter doesn't just walk into every swing. Both fighters carry
    // the full kit, so require the bread-and-butter BLOCK from each…
    assert!(
        pca.shield >= 5,
        "PCA should reactively block the opponent's pressure (got {} frames)",
        pca.shield
    );
    assert!(
        robot.shield >= 5,
        "robot should reactively block the opponent's pressure (got {} frames)",
        robot.shield
    );
    // …and at least one BLINK-evade across the duel (the mobile option, reserved for
    // a committed lunge — rarer than the block by design).
    assert!(
        pca.blink + robot.blink >= 2,
        "the duel should show blink-evades against committed lunges (PCA {} + robot {})",
        pca.blink,
        robot.blink
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

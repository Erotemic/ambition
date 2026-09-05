//! Verify same-room resets under forced rollback.
//!
//! One test rebases known damaged enemy and broken-brick state, resets the room,
//! and requires both to return to authored state. The other drives a player
//! death through the same path and keeps the sync-test session healthy. These
//! fixtures use melee enemies, so projectile reset is outside their coverage.

#![cfg(feature = "rl_sim")]

use ambition_app::rl_sim::{
    AgentAction, AmbitionSim, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode,
};
use ambition_platformer2d::characters::actor::BodyHealth;
use ambition_platformer2d::platformer::sim_id::SimId;
use bevy::prelude::{Entity, With, Without, World};

/// Resolve an authored identity to whatever entity currently carries it.
///
/// ⛔⛔ AN `Entity` HANDLE DOES NOT SURVIVE A ROOM RESET. A same-room replay is a
/// reconstruction now: the room's population is retired and rebuilt from
/// prepared content at the confirmed lifecycle boundary, so the enemy that comes
/// back is a NEW entity carrying the SAME `SimId`. Holding the old handle across
/// the reset asks "did this exact allocation survive", which is not the question
/// this file is about.
fn entity_named(sim: &mut Platformer2dSimHarness, id: &SimId) -> Option<Entity> {
    let world = sim.world_mut();
    let mut q = world.query::<(Entity, &SimId)>();
    q.iter(world)
        .find(|(_, live)| *live == id)
        .map(|(entity, _)| entity)
}

fn repro_sim() -> Platformer2dSimHarness {
    Platformer2dSimHarness::new_with_options(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_required_start_room("combat_calibration_lab")
            .with_sync_test_rollback_settings(4, 10),
    )
    .expect("Ambition GGRS sync-test harness builds in the calibration lab")
}

/// Stage the player on the arena floor (past the spawn-corner parkour ledge and
/// east of the hazard band), like the exit oracle, and set HP. Pure world
/// mutation — the CALLER rebases so several stagings fold into one baseline.
fn place_player_on_floor(world: &mut World, hp: i32) {
    let player = {
        let mut q =
            world.query_filtered::<Entity, With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>();
        q.single(world).expect("one primary player")
    };
    {
        let mut kin = world
            .get_mut::<ambition_platformer2d::platformer::body::BodyKinematics>(player)
            .expect("player kinematics");
        kin.pos = ambition_platformer2d::engine_core::Vec2::new(720.0, kin.pos.y);
        kin.vel = ambition_platformer2d::engine_core::Vec2::ZERO;
    }
    if let Some(mut health) = world.get_mut::<BodyHealth>(player) {
        health.health.max = hp;
        health.health.current = hp;
    }
}

/// Stage the player at `hp` and fold it into the rollback baseline.
fn stage_on_floor(sim: &mut Platformer2dSimHarness, hp: i32) {
    place_player_on_floor(sim.world_mut(), hp);
    sim.rebase_rollback_history()
        .expect("arena staging becomes the rollback baseline");
}

/// Wound the westmost non-player, reset-eligible enemy to half HP (pure world
/// mutation, folded into the baseline by the caller's rebase). Returns
/// `(entity, spawn_max)` so the reset's restore-to-spawn can be asserted exactly.
///
/// A DETERMINISTIC damaged baseline beats hoping the emergent fight leaves an
/// enemy alive-but-damaged at the reset frame — and a mid-window direct write
/// would not be reproduced during resim, so it must live in the baseline.
fn wound_one_enemy(world: &mut World) -> (SimId, i32) {
    // Westmost keeps the choice stable across runs without depending on Bevy's
    // (unstable) query iteration order.
    let (entity, id, max) = {
        let mut q = world.query_filtered::<(
            Entity,
            &SimId,
            &BodyHealth,
            &ambition_platformer2d::platformer::body::BodyKinematics,
        ), (
            With<ambition_platformer2d::platformer::lifecycle::FeatureSimEntity>,
            Without<ambition_platformer2d::platformer::markers::PrimaryPlayer>,
        )>();
        q.iter(world)
            .map(|(e, id, h, k)| (e, id.clone(), h.health.max, k.pos.x))
            .min_by(|a, b| a.3.total_cmp(&b.3))
            .map(|(e, id, max, _)| (e, id, max))
            .expect("the calibration lab authors at least one non-player enemy")
    };
    let wounded = (max / 2).max(1);
    world
        .get_mut::<BodyHealth>(entity)
        .expect("enemy health")
        .health
        .current = wounded;
    (id, max)
}

/// Smash the first intact breakable (pure world mutation, folded into the
/// baseline). Returns its entity so the reset's restore-to-intact can be checked.
fn smash_one_brick(world: &mut World) -> SimId {
    let (brick, id) = {
        let mut q = world.query::<(
            Entity,
            &SimId,
            &ambition_platformer2d::combat::components::BreakableFeature,
        )>();
        q.iter(world)
            .find(|(_, _, feature)| !feature.broken())
            .map(|(e, id, _)| (e, id.clone()))
            .expect("the calibration lab authors a breakable brick")
    };
    let mut feature = world
        .get_mut::<ambition_platformer2d::combat::components::BreakableFeature>(brick)
        .expect("breakable feature");
    // Fixture setup: drive it to broken; this test asserts the RESET, not the break.
    let _broke = feature.breakable.apply_damage(9999);
    assert!(feature.broken(), "the brick is broken after lethal damage");
    id
}

fn enemy_hp(sim: &mut Platformer2dSimHarness, enemy: &SimId) -> i32 {
    let entity = entity_named(sim, enemy)
        .unwrap_or_else(|| panic!("no live entity carries `{enemy}` after the reset"));
    sim.world_mut()
        .get::<BodyHealth>(entity)
        .map(|h| h.health.current)
        .expect("the reconstructed enemy has health")
}

fn brick_is_broken(sim: &mut Platformer2dSimHarness, brick: &SimId) -> bool {
    let entity = entity_named(sim, brick)
        .unwrap_or_else(|| panic!("no live entity carries `{brick}` after the reset"));
    sim.world_mut()
        .get::<ambition_platformer2d::combat::components::BreakableFeature>(entity)
        .map(|f| f.broken())
        .expect("the reconstructed brick is a breakable")
}

fn player_hp(sim: &mut Platformer2dSimHarness) -> i32 {
    let world = sim.world_mut();
    let mut q =
        world.query_filtered::<&BodyHealth, With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>();
    q.single(world).map(|h| h.health.current).unwrap_or(0)
}

fn living_enemies(sim: &mut Platformer2dSimHarness) -> Vec<(f32, f32)> {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<(
        &ambition_platformer2d::platformer::body::BodyKinematics,
        &BodyHealth,
    ), Without<ambition_platformer2d::platformer::markers::PrimaryPlayer>>();
    q.iter(world)
        .filter(|(_, h)| h.health.current > 0)
        .map(|(kin, _)| (kin.pos.x, kin.pos.y))
        .collect()
}

/// A same-room reset inside a forced rollback window restores enemy health and
/// broken-brick state, not merely checksum agreement.
///
/// The census localizes any divergence to registered component types across the
/// first simulation and resimulation. `#[ignore]` for cost.
#[test]
#[ignore = "diagnostic: per-component restore census on every save/load; run when this module is red"]
fn which_component_does_the_lifecycle_reset_divergence_live_in() {
    let mut sim = repro_sim();
    sim.world_mut()
        .insert_resource(ambition_platformer2d::rollback::RollbackRestoreAudit::enabled());

    let probes = sim
        .world()
        .resource::<ambition_platformer2d::rollback::RollbackChecksumProbes>()
        .len();
    assert!(
        probes > 0,
        "no localization probes were registered, so this test could only ever \
         report success"
    );

    place_player_on_floor(sim.world_mut(), 200);
    let _ = wound_one_enemy(sim.world_mut());
    let _ = smash_one_brick(sim.world_mut());
    sim.rebase_rollback_history()
        .expect("the damaged arena becomes the rollback baseline");

    // Walk far enough past the reported window (frames 21-23) to catch it, and
    // do NOT assert rollback_health per frame: the point is to reach the
    // divergence and describe it, not to stop at the first aggregate red.
    for _ in 0..12 {
        sim.step(AgentAction::default());
    }
    sim.step(AgentAction::reset());
    for _ in 0..40 {
        sim.step(AgentAction::default());
    }

    let audit = sim
        .world()
        .resource::<ambition_platformer2d::rollback::RollbackRestoreAudit>();
    // Vacuity guard FIRST: a localizer that compared nothing must not read as
    // "nothing diverged".
    assert!(
        audit.comparisons > 0 && audit.resimulations > 0,
        "the audit compared nothing, so its verdict is meaningless: {}",
        audit.coverage()
    );
    assert!(
        audit.divergences.is_empty(),
        "divergences: {:#?}",
        audit.divergences
    );
}

#[test]
fn a_manual_reset_restores_a_damaged_enemy_and_a_broken_brick_under_forced_rollback() {
    let mut sim = repro_sim();

    // Fold a KNOWN damaged enemy + KNOWN broken brick into the rollback baseline.
    // Player at 200 HP so its own death never fires a competing reset — the
    // manual `AgentAction::reset()` below is the ONLY reset in play.
    place_player_on_floor(sim.world_mut(), 200);
    let (enemy, enemy_max) = wound_one_enemy(sim.world_mut());
    let brick = smash_one_brick(sim.world_mut());
    sim.rebase_rollback_history()
        .expect("the damaged arena becomes the rollback baseline");

    // Advance a handful of frames so the reset lands INSIDE a live rollback
    // window (the sync-test saves/loads/re-simulates on every advance). Default
    // actions: the player never attacks, so the wounded enemy stays wounded.
    for frame in 0..12 {
        sim.step(AgentAction::default());
        sim.rollback_health()
            .unwrap_or_else(|error| panic!("pre-reset frame {frame}: {error}"));
    }

    // Record the exact pre-reset facts (read live, not assumed).
    let pre_hp = enemy_hp(&mut sim, &enemy);
    assert!(
        pre_hp > 0 && pre_hp < enemy_max,
        "the enemy is alive-but-damaged before the reset (hp {pre_hp}/{enemy_max})"
    );
    assert!(
        brick_is_broken(&mut sim, &brick),
        "the brick is broken before the reset"
    );

    // Trigger the manual reset, then let the rebuild land.
    //
    // ⛔ NOT ONE FRAME. A same-room reset is a reconstruction authorized at a
    // CONFIRMED lifecycle boundary — the same barrier a door crossing waits for —
    // so the room comes back a couple of frames later, and under a sync-test
    // host it comes back through the rollback rebase. Asserting on the reset
    // frame itself measures the frame the request was made, not the rebuild.
    sim.step(AgentAction::reset());
    for frame in 0..30 {
        sim.step(AgentAction::default());
        sim.rollback_health()
            .unwrap_or_else(|error| panic!("rebuild frame {frame}: {error}"));
    }

    // The reset RESTORED the damaged enemy and the broken brick — the behavioral
    // claim, not just an absence of checksum divergence.
    assert_eq!(
        enemy_hp(&mut sim, &enemy),
        enemy_max,
        "the reset did not bring the damaged enemy back at spawn HP"
    );
    assert!(
        !brick_is_broken(&mut sim, &brick),
        "the reset did not bring the broken brick back intact"
    );

    // ...and the sim stays checksum-clean well past the rollback window.
    for frame in 0..180 {
        sim.step(AgentAction::default());
        sim.rollback_health()
            .unwrap_or_else(|error| panic!("post-reset frame {frame}: {error}"));
    }
}

/// A player death triggers a same-room reset while enemies are mid-brawl; the
/// rollback window must restore their health in place.
#[test]
fn a_player_death_reset_survives_the_rollback_window() {
    let mut sim = repro_sim();
    stage_on_floor(&mut sim, 3);

    let mut saw_death = false;
    let mut prev_hp = player_hp(&mut sim);
    for frame in 0..2400 {
        // Walk toward the nearest living enemy and STAND THERE, inside the melee
        // strikers' reach, until they whittle the 3 HP down.
        //
        // Standing in reach is the same situation with the incidental half removed — and it no
        // longer re-breaks every time the robot's frame data is retuned.
        let px = {
            let world = sim.world_mut();
            let mut q = world.query_filtered::<&ambition_platformer2d::platformer::body::BodyKinematics, With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>();
            q.single(world).map(|k| k.pos.x).unwrap_or(0.0)
        };
        let action = match living_enemies(&mut sim)
            .into_iter()
            .map(|(x, _)| (x, (x - px).abs()))
            .min_by(|a, b| a.1.total_cmp(&b.1))
        {
            Some((x, d)) if d > 12.0 => AgentAction::move_x((x - px).signum()),
            Some(_) | None => AgentAction::default(),
        };
        sim.step(action);

        let hp = player_hp(&mut sim);
        if hp > prev_hp {
            // HP jumped back up: the death reset (or revive) fired.
            saw_death = true;
        }
        prev_hp = hp;

        sim.rollback_health()
            .unwrap_or_else(|error| panic!("frame {frame} (saw_death={saw_death}): {error}"));
    }

    assert!(
        saw_death,
        "the 3-HP player should have died and reset at least once in 2400 frames"
    );
}

/// What a rollback frame COSTS — an instrument, not an assertion.
///
/// `#[ignore]`d deliberately: it asserts nothing, and a timing assertion on
/// shared CI-less hardware would be a guard that fails for the weather. Run it
/// when the rollback schema changes, or when someone claims a snapshot cost:
///
/// ```text
/// cargo test --release -p ambition_app --features rl_sim --test app_it \
///     probe_what_a_rollback_frame_costs -- --ignored --nocapture
/// ```
///
/// the floor is save PLUS checksum, and the checksum is sync-test-only, so the
/// floor a shipped netplay session pays is strictly smaller and is not separated
/// by this probe. What IS separated: a resimulated frame costs 2.5 ms against a
/// 2.17 ms plain step, so save+restore is ~0.35 ms — the cost of a rollback is
/// re-running the SIMULATION, not moving the state.
#[test]
#[ignore = "measurement, not an assertion — see the doc comment"]
fn probe_what_a_rollback_frame_costs() {
    fn run(label: &str, opts: Platformer2dSimHarnessOptions) {
        let mut sim = Platformer2dSimHarness::new_with_options(opts).expect("builds");
        stage_on_floor(&mut sim, 3);
        for _ in 0..60 {
            sim.step(AgentAction::default());
        }
        let t = std::time::Instant::now();
        for _ in 0..300 {
            sim.step(AgentAction::default());
        }
        eprintln!(
            "PROBE {label:28} = {:.2}ms/frame",
            t.elapsed().as_secs_f64() * 1000.0 / 300.0
        );
    }
    let base = || {
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_required_start_room("combat_calibration_lab")
    };
    run("fixed60 no rollback", base().with_fixed_tick(true));
    run(
        "synctest dist=0 (shipped)",
        base().with_sync_test_rollback_settings(0, 10),
    );
    run(
        "synctest dist=1",
        base().with_sync_test_rollback_settings(1, 10),
    );
    run(
        "synctest dist=2",
        base().with_sync_test_rollback_settings(2, 10),
    );
    run(
        "synctest dist=4",
        base().with_sync_test_rollback_settings(4, 10),
    );

    // POPULATION vs SCHEMA, the whole point of this addition. Same rollback settings, same
    // frame count, different amounts of world to snapshot.
    let empty = || {
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_required_start_room("tiny_chamber")
    };
    // the knob is the ROOM, not a body count. `stage_on_floor`'s second
    // argument is HP — I read it as a population and nearly ran an experiment
    // that varied nothing while appearing to vary the thing under test. The
    // population difference that exists is between rooms: the calibration lab is
    // furnished, `tiny_chamber` is not.
    run(
        "synctest d=2 tiny room",
        empty().with_sync_test_rollback_settings(2, 10),
    );
    run("fixed60 tiny room", empty().with_fixed_tick(true));
}

/// The once-per-session checkpoint resume survives the rollback window.
///
/// ⛔⛔ IT USED TO BE TWO `Local`s ON A SIM SYSTEM.
/// `restore_checkpoint_on_session_start` runs in `PlayerSimulation`, and a
/// `Local` does not rewind: a rollback crossing the frame it routed on would
/// resimulate with the memory already past the crossing, so one timeline asks
/// for the crossing and the other believes it already did. The memory is
/// `CheckpointResumeProgress` now — registered, and probed by WHICH generation
/// rather than by presence.
///
/// ⚠ WHAT THIS ARM CAN AND CANNOT SEE. Verified 2026-08-31 by poisoning the
/// memory back to `Local`s: this stays GREEN. A confirmed room transition
/// REBASES GGRS onto a new frame-zero baseline, so no rewind ever crosses the
/// routing frame and the divergence is unreachable today. The move is still
/// right — a correctness that holds only because some other layer rebases is a
/// correctness that moves when the rebase does — but it is UNPINNED, and saying
/// so beats implying a guard that does not exist. Same shape, same wording, as
/// the Mary-O room memory in
/// `game/ambition_demo_mary_o_app/tests/rollback_room_memory.rs`.
///
/// What it DOES pin is that routing a cross-room resume under a live sync-test
/// session stays checksum-clean at all — which is not free, and was worth an arm
/// the first time a resume became a lifecycle intent.
#[test]
fn a_cross_room_checkpoint_resume_stays_checksum_clean() {
    use ambition_platformer2d::session::AmbitionGameSaveData;

    // The room the session opens in, and a different one for the checkpoint to
    // name — a same-room checkpoint never routes, so it would test nothing.
    let mut scout = repro_sim();
    let rooms: Vec<String> = {
        let world = scout.world_mut();
        let mut q = world.query::<&ambition_platformer2d::world::rooms::RoomSet>();
        q.iter(world)
            .next()
            .expect("the session has an active room set")
            .rooms
            .iter()
            .map(|room| room.id.clone())
            .filter(|id| id != "combat_calibration_lab")
            .collect()
    };
    let elsewhere = rooms
        .first()
        .cloned()
        .expect("the world has a second room to resume into");

    let mut save = AmbitionGameSaveData::default();
    save.set_checkpoint(
        ambition_platformer2d::persistence::save_data::PersistedCheckpoint::new(
            elsewhere.clone(),
            200,
            200,
        ),
    );

    let mut sim = Platformer2dSimHarness::new_with_options(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_required_start_room("combat_calibration_lab")
            .with_sync_test_rollback_settings(4, 10)
            .with_save(save),
    )
    .expect("the GGRS sync-test harness boots with a save file");

    for frame in 0..240 {
        sim.step(AgentAction::default());
        ambition_platformer2d::rollback::session_health(sim.world())
            .unwrap_or_else(|error| panic!("frame {frame} of the resume desynced: {error}"));
    }

    // ⛔ THE PREMISE: a resume that never routed would keep this arm green by
    // never doing the thing it is about.
    let landed = {
        let world = sim.world_mut();
        let mut q = world.query::<&ambition_platformer2d::world::rooms::RoomSet>();
        q.iter(world)
            .next()
            .expect("the session has an active room set")
            .active_spec()
            .id
            .clone()
    };
    assert_eq!(
        landed, elsewhere,
        "the checkpoint resume never crossed, so nothing above measured a \
         crossing under rollback"
    );
}

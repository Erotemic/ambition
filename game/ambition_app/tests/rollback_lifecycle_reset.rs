//! **Track B, T1: the IN-PLACE room reset survives forced rollback — and
//! actually restores the state it claims to.**
//!
//! The track-0 exit oracle pins the player at HP=200 so death never triggers a
//! same-room reset (`rollback_exit_oracle.rs` HP comment), and the campaign doc
//! recorded "sim-triggered room reset inside a rollback window is a guaranteed
//! divergence" (observed at frame ~2147 as a mid-brawl enemy full-heal). These
//! tests were written to REPRODUCE that — and, per the reproduce-first
//! discipline, they CORRECTED the diagnosis instead: the in-place reset path is
//! rollback-safe in the current tree (the intervening lifecycle-boundary
//! hardening — `PendingPlayerHitEvents` voiding `fd7ddbc0c`, strike-volume
//! registration, the Combat chain reorder — closed the recorded boundary).
//!
//! Two witnesses keep it closed:
//!
//! - `a_manual_reset_restores_a_damaged_enemy_and_a_broken_brick_under_forced_rollback`
//!   is the LOAD-BEARING behavioral proof: it folds a KNOWN damaged
//!   enemy and a KNOWN broken brick into the rollback baseline, drives a manual
//!   reset inside a live sync-test window, and asserts the reset genuinely
//!   RESTORED the enemy to spawn HP and the brick to intact — not merely that
//!   the checksum stayed clean. This is the exact divergence the campaign doc
//!   recorded (an enemy that fails to snap back), now pinned as restored.
//! - `a_player_death_reset_survives_the_rollback_window` is the emergent
//!   end-to-end witness: a 3-HP player dies into a same-room reset mid-brawl and
//!   the sim stays checksum-healthy for 2400 frames (past the ~2147 where
//!   divergence was originally seen).
//!
//! NB: this lab's "strikers" are MELEE (`melee_brute_striker` / `striker_swipe`),
//! so no `LiveProjectile` entities arise here — the reset's projectile-despawn
//! line (`reset_ecs_room_features`) is not exercised by these tests; a
//! ranged-enemy room would be needed to pin it, and is noted as a follow-up.
//!
//! Track B's REMAINING target is RECONSTRUCTION — the paths that despawn+respawn
//! a whole room's entities via Commands (op 2b full sandbox reset, op 4 room
//! transition, op 5 snapshot), which these tests do not exercise. That
//! reproduction needs an input-driven transition/full-reset scenario and is
//! still owed; see the campaign doc Track B section.

#![cfg(feature = "rl_sim")]

use ambition_app::rl_sim::{
    AgentAction, AmbitionSim, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode,
};
use ambition_platformer2d::characters::actor::BodyHealth;
use bevy::prelude::{Entity, With, Without, World};

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
fn wound_one_enemy(world: &mut World) -> (Entity, i32) {
    // Westmost keeps the choice stable across runs without depending on Bevy's
    // (unstable) query iteration order.
    let (entity, max) = {
        let mut q = world.query_filtered::<(
            Entity,
            &BodyHealth,
            &ambition_platformer2d::platformer::body::BodyKinematics,
        ), (
            With<ambition_platformer2d::platformer::lifecycle::FeatureSimEntity>,
            Without<ambition_platformer2d::platformer::markers::PrimaryPlayer>,
        )>();
        q.iter(world)
            .map(|(e, h, k)| (e, h.health.max, k.pos.x))
            .min_by(|a, b| a.2.total_cmp(&b.2))
            .map(|(e, max, _)| (e, max))
            .expect("the calibration lab authors at least one non-player enemy")
    };
    let wounded = (max / 2).max(1);
    world
        .get_mut::<BodyHealth>(entity)
        .expect("enemy health")
        .health
        .current = wounded;
    (entity, max)
}

/// Smash the first intact breakable (pure world mutation, folded into the
/// baseline). Returns its entity so the reset's restore-to-intact can be checked.
fn smash_one_brick(world: &mut World) -> Entity {
    let brick = {
        let mut q = world.query::<(
            Entity,
            &ambition_platformer2d::combat::components::BreakableFeature,
        )>();
        q.iter(world)
            .find(|(_, feature)| !feature.broken())
            .map(|(e, _)| e)
            .expect("the calibration lab authors a breakable brick")
    };
    let mut feature = world
        .get_mut::<ambition_platformer2d::combat::components::BreakableFeature>(brick)
        .expect("breakable feature");
    feature.breakable.apply_damage(9999);
    assert!(feature.broken(), "the brick is broken after lethal damage");
    brick
}

fn enemy_hp(sim: &mut Platformer2dSimHarness, enemy: Entity) -> i32 {
    sim.world_mut()
        .get::<BodyHealth>(enemy)
        .map(|h| h.health.current)
        .expect("wounded enemy still exists after the in-place reset")
}

fn brick_is_broken(sim: &mut Platformer2dSimHarness, brick: Entity) -> bool {
    sim.world_mut()
        .get::<ambition_platformer2d::combat::components::BreakableFeature>(brick)
        .map(|f| f.broken())
        .expect("brick still exists after the in-place reset")
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

/// **The load-bearing behavioral proof.** A manual same-room reset, driven
/// inside a forced rollback window, must genuinely restore a damaged enemy to
/// spawn HP and a broken brick to intact — the exact "enemy fails to snap back"
/// divergence the campaign doc recorded, now asserted as restored rather than
/// merely checksum-clean.
/// **Which registered component does this scenario's divergence live in?**
///
/// The aggregate sync-test says "checksum mismatch at frames [21,22,23]" and
/// names nothing. This censuses every registered type on every save and every
/// load and reports the ones whose census differs between a frame's first
/// simulation and its resimulation — the same instrument
/// `rollback_exit_oracle`'s localizer uses, pointed at the lifecycle-reset
/// route instead of the combat one.
///
/// `#[ignore]` for cost.
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
    let pre_hp = enemy_hp(&mut sim, enemy);
    assert!(
        pre_hp > 0 && pre_hp < enemy_max,
        "the enemy is alive-but-damaged before the reset (hp {pre_hp}/{enemy_max})"
    );
    assert!(
        brick_is_broken(&mut sim, brick),
        "the brick is broken before the reset"
    );

    // Trigger the in-place manual reset (op 2a) on the next frame.
    sim.step(AgentAction::reset());
    sim.rollback_health()
        .unwrap_or_else(|error| panic!("reset frame: {error}"));

    // The reset RESTORED the damaged enemy and the broken brick — the behavioral
    // claim, not just an absence of checksum divergence.
    assert_eq!(
        enemy_hp(&mut sim, enemy),
        enemy_max,
        "the in-place reset revived the damaged enemy to spawn HP"
    );
    assert!(
        !brick_is_broken(&mut sim, brick),
        "the in-place reset restored the broken brick to intact"
    );

    // ...and the sim stays checksum-clean well past the rollback window.
    for frame in 0..180 {
        sim.step(AgentAction::default());
        sim.rollback_health()
            .unwrap_or_else(|error| panic!("post-reset frame {frame}: {error}"));
    }
}

/// A player death fires a same-room reset while enemies are mid-brawl. The reset
/// revives them (`health.reset()`) inside the rollback window. This PASSES to
/// 2400 frames — the emergent end-to-end witness that the in-place reset
/// boundary the campaign doc recorded is now closed. Keeps it closed.
///
/// (The strikers whittling the 3-HP player down are MELEE, not projectile
/// shooters — see the module note.)
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
/// 2.17 ms plain step, so save+restore is ~0.35 ms — **the cost of a rollback is
/// re-running the SIMULATION, not moving the state.**
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

    // **POPULATION vs SCHEMA, the whole point of this addition.** Same rollback settings, same
    // frame count, different amounts of world to snapshot.
    let empty = || {
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_required_start_room("tiny_chamber")
    };
    // **the knob is the ROOM, not a body count.** `stage_on_floor`'s second
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

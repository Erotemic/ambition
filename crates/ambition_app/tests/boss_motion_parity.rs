//! Boss MOTION + FLOAT parity net for the §A1 slice-3 driver fold.
//!
//! The slice-3 fold dissolves the boss island: `update_ecs_bosses` (which
//! integrates the boss body through the floating integrator `step_floating_body`)
//! folds into the shared actor `integrate_sim_bodies`, with the boss carried as an
//! actor archetype whose flight limb replaces the bespoke float. That swap is the
//! single biggest regression risk of the whole fold: an actor body integrates
//! under gravity, so if the boss doesn't come across as a genuine free-flyer it
//! will either PLUMMET (gravity leaks in) or FREEZE (the pattern's `desired_vel`
//! stops reaching the integrator).
//!
//! These tests pin exactly those two invariants — a boss FLOATS (never falls) and,
//! once woken, MOVES — as ranges, not exact trajectories. Per the project's
//! "behavior is not sacred" stance the fold may perturb the precise flight path
//! (the flight limb is not bit-identical to `step_floating_body`); what it must
//! NOT do is turn the boss into a rock or a brick. If either of these fails after
//! the fold, the boss stopped being a real floating actor — a true regression, not
//! a cosmetic drift.
//!
//! Run with `cargo test -p ambition_app --test boss_motion_parity -- --nocapture`
//! to see the per-frame float/altitude trace.

#![cfg(feature = "rl_sim")]

use ambition_actors::actor::BodyKinematics;
use ambition_actors::features::ecs::boss_clusters::BossConfig;
use ambition_app::{AgentAction, SandboxSim, TimestepMode};
use ambition_engine_core as ae;
use ambition_entity_catalog::placements::BossBrain;
use bevy::prelude::World;

/// Read the live boss's body position (only the boss carries `BossConfig`, so this
/// never matches the player even though both share `BodyKinematics`). `None` once
/// the boss has despawned (a stray room-edge reset can remove a programmatically
/// spawned boss — orthogonal to the float/motion property under test).
fn read_boss_pos(world: &mut World) -> Option<ae::Vec2> {
    let mut q = world.query::<(&BodyKinematics, &BossConfig)>();
    q.iter(world).next().map(|(kin, _)| kin.pos)
}

fn read_player_pos(world: &mut World) -> ae::Vec2 {
    use ambition_actors::actor::PrimaryPlayerOnly;
    let mut q = world.query_filtered::<&BodyKinematics, PrimaryPlayerOnly>();
    q.single(world).expect("primary player exists").pos
}

/// A freshly-spawned boss FLOATS: under the sandbox's downward gravity it must not
/// fall. Sim space is +Y-down, so a fall is `y` increasing. A non-floating body
/// would drop ~0.5·g·t² — well over 100px in half a second (and stay dropping even
/// at terminal velocity). The live boss only drifts ~12px in this window (its
/// pattern nudges it, it does not free-fall), so a 60px ceiling sits cleanly
/// between "floats" and "falls" and survives pattern drift without admitting a
/// real gravity plunge.
#[test]
fn dormant_boss_floats_and_does_not_fall() {
    let mut sim =
        SandboxSim::new_with_timestep(TimestepMode::fixed_60hz()).expect("sandbox sim builds");

    let start = read_player_pos(sim.world_mut());
    sim.spawn_boss_at(
        "test_float_boss",
        "mockingbird",
        (start.x, start.y - 60.0),
        (30.0, 30.0),
        BossBrain::PhaseScript {
            script_id: "mockingbird".to_string(),
        },
    );
    let spawn = read_boss_pos(sim.world_mut()).expect("boss spawned");

    // 30 frames = 0.5s at 60Hz. A body that fell under gravity would already be
    // >100px below; the floating boss only drifts a handful of px.
    let mut max_drop = 0.0f32;
    for _ in 0..30 {
        sim.step(AgentAction::default());
        let Some(pos) = read_boss_pos(sim.world_mut()) else {
            break;
        };
        // Drop = downward (+Y) displacement from spawn.
        max_drop = max_drop.max(pos.y - spawn.y);
    }

    assert!(
        max_drop < 60.0,
        "a floating boss must not fall, but it dropped {max_drop:.1}px in 0.5s — the \
         boss is integrating under gravity instead of floating"
    );
}

/// Over a longer run a woken boss must (a) still be afloat — never plummeting far
/// below its spawn — and (b) actually MOVE (its pattern steers it around its
/// anchor). Guards the two opposite failure modes of the integration fold: gravity
/// leaking in (fall) and the pattern's `desired_vel` no longer reaching the body
/// (freeze).
#[test]
fn woken_boss_moves_and_stays_afloat() {
    const FRAMES: usize = 300;
    let mut sim =
        SandboxSim::new_with_timestep(TimestepMode::fixed_60hz()).expect("sandbox sim builds");

    let start = read_player_pos(sim.world_mut());
    sim.spawn_boss_at(
        "test_motion_boss",
        "mockingbird",
        (start.x, start.y - 60.0),
        (30.0, 30.0),
        BossBrain::PhaseScript {
            script_id: "mockingbird".to_string(),
        },
    );
    let spawn = read_boss_pos(sim.world_mut()).expect("boss spawned");

    let mut prev = spawn;
    let mut path_len = 0.0f32;
    let mut max_drop = 0.0f32;
    let mut ticked = 0usize;
    for _ in 0..FRAMES {
        sim.step(AgentAction::default());
        let Some(pos) = read_boss_pos(sim.world_mut()) else {
            break;
        };
        path_len += (pos - prev).length();
        max_drop = max_drop.max(pos.y - spawn.y);
        prev = pos;
        ticked += 1;
    }

    // The boss must survive most of the run for the measurement to mean anything.
    assert!(
        ticked >= FRAMES / 2,
        "boss despawned after only {ticked}/{FRAMES} frames — the run is too short to \
         measure motion; the boss should persist through the pattern"
    );

    // (a) Stayed afloat: even patterned up/down swoops never turn into a plunge.
    // A falling body accelerates to hundreds of px/s; 250px is a generous ceiling
    // that a genuine free-flyer around its anchor never reaches.
    assert!(
        max_drop < 250.0,
        "boss fell {max_drop:.1}px below spawn over the run — it stopped floating"
    );

    // (b) Moved: a frozen boss has ~0 path length. A patterned flyer covers real
    // distance. The threshold is deliberately loose (the fold may change the exact
    // path); it only has to be clearly non-zero.
    assert!(
        path_len > 40.0,
        "boss barely moved ({path_len:.1}px of path over {ticked} frames) — the \
         pattern's desired velocity is not reaching the integrator"
    );
}

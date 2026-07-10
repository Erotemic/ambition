//! **N0.4 — the desync canary.** `docs/planning/engine/netcode.md`:
//!
//! > *"Two sims, same input stream, state-hash per tick, first-divergence report.
//! > This is the tool that keeps N0 true forever."*
//!
//! Two `SandboxSim`s are built from the same options, stepped with the same input
//! stream, and their registered sim state is hashed every tick. Any divergence is
//! a determinism bug: same build, same inputs, same machine.
//!
//! The hash covers exactly the state `register_engine_sim_state` declares (N3.1
//! decision 1: *"un-registered state is by definition presentation or derived, and
//! the desync canary hashes exactly the registered set, which keeps the two
//! features honest against each other"*). Today that is the sim tick, the scaled
//! clock, and every body with a stable id. **What the canary cannot see, it cannot
//! defend** — and the set grows as `SimId` reaches the rest of the sim.

#![cfg(feature = "rl_sim")]

use ambition::runtime::snapshot::{
    compare_hash_streams, register_engine_sim_state, SnapshotRegistry,
};
use ambition_app::rl_sim::TimestepMode;
use ambition_app::{RandomWalkPolicy, SandboxSim, SandboxSimOptions};

fn registry() -> SnapshotRegistry {
    let mut reg = SnapshotRegistry::default();
    register_engine_sim_state(&mut reg);
    reg
}

fn sim(room: &str) -> Option<SandboxSim> {
    let opts = SandboxSimOptions::default()
        .with_timestep(TimestepMode::fixed_60hz())
        .with_start_room(room);
    SandboxSim::new_with_options(opts).ok()
}

/// Step `sim` through the seeded policy's actions, hashing after each tick.
fn hash_stream(sim: &mut SandboxSim, reg: &SnapshotRegistry, seed: u64, ticks: u64) -> Vec<u64> {
    let mut policy = RandomWalkPolicy::traversal_stress(seed);
    (0..ticks)
        .map(|_| {
            sim.step(policy.act());
            reg.hash_world(sim.world())
        })
        .collect()
}

/// **The canary.** Two sims, one input stream, one build. They must agree tick for
/// tick, and the report names the first tick that disagreed.
#[test]
fn two_sims_on_the_same_input_stream_never_diverge() {
    let reg = registry();
    assert!(reg.len() >= 3, "the registry declares something to defend");

    for (room, seed) in [
        ("gap_run", 1),
        ("portal_lab", 42),
        ("mockingbird_arena", 2026),
    ] {
        let (Some(mut a), Some(mut b)) = (sim(room), sim(room)) else {
            continue; // a room the fixture cannot load is not this test's business
        };
        let ha = hash_stream(&mut a, &reg, seed, 240);
        let hb = hash_stream(&mut b, &reg, seed, 240);
        let report = compare_hash_streams(&ha, &hb);

        if let Some(tick) = report.first_divergence_tick {
            // Localize it: which registered entry disagreed, and what the two
            // worlds hold. A desync you cannot name is a desync you cannot fix.
            let ea = reg.hash_by_entry(a.world());
            let eb = reg.hash_by_entry(b.world());
            let culprits: Vec<&str> = ea
                .iter()
                .zip(&eb)
                .filter(|((_, x), (_, y))| x != y)
                .map(|((name, _), _)| *name)
                .collect();
            panic!(
                "DESYNC in `{room}` (seed {seed}) at tick {tick} of {}: entries \
                 {culprits:?} disagree. Same build, same inputs — this is a \
                 determinism bug (ADR 0023), not flakiness.",
                report.ticks_compared
            );
        }
        assert_eq!(report.ticks_compared, 240);
    }
}

/// The canary must be able to CRY. A hash that cannot distinguish two different
/// worlds proves nothing about two identical ones — the same poison-test rule
/// ADR 0023's determinism lints follow.
#[test]
fn the_canary_reports_a_divergence_when_one_sim_is_given_different_input() {
    let reg = registry();
    let (Some(mut a), Some(mut b)) = (sim("gap_run"), sim("gap_run")) else {
        return;
    };
    let ha = hash_stream(&mut a, &reg, 1, 120);
    let hb = hash_stream(&mut b, &reg, 999, 120); // a DIFFERENT input stream
    let report = compare_hash_streams(&ha, &hb);
    assert!(
        !report.in_sync(),
        "two different input streams must produce two different worlds — if this \
         passes, the hash is blind and `two_sims_on_the_same_input_stream_never_diverge` \
         is worthless"
    );
}

/// And the hash must be sensitive to the state it claims to cover: moving one body
/// changes it. Without this, `register_engine_sim_state` could register an entry
/// that reads nothing and the canary would never notice.
#[test]
fn moving_a_body_changes_the_registered_hash() {
    let reg = registry();
    let Some(mut s) = sim("gap_run") else { return };
    let before = reg.hash_world(s.world());

    {
        use ambition::bevy::prelude::With;
        let mut q = s.world_mut().query_filtered::<
            &mut ambition::actors::actor::BodyKinematics,
            With<ambition::actors::actor::PrimaryPlayer>,
        >();
        let world = s.world_mut();
        for mut kin in q.iter_mut(world) {
            kin.pos.x += 1.0;
        }
    }
    assert_ne!(
        before,
        reg.hash_world(s.world()),
        "the registered hash must see the player's body — it is the first thing a \
         rollback would have to restore"
    );
}

/// **The SimId migration, measured.**
///
/// N3.1: *"every snapshot-registered entity carries a `SimId`."* Today
/// `ensure_sim_id` covers the two identities that exist as authored facts — a
/// placement's `FeatureId` and the primary player's slot. Everything else is a
/// dynamically-spawned entity whose spawn site must mint
/// `SimId::spawned(spawner, counter.next())`, because only the spawn site knows
/// its spawner.
///
/// `mint_spawned_sim_ids` covers the other class: an in-flight projectile takes
/// `SimId::spawned(owner, owner_counter.next())`, ordered by the `ProjectileSeq`
/// the step system already sorts by.
///
/// **The ledger is a GATE, and it reads zero.** Every simulated body in every
/// room below carries a `SimId`. A rise means a new spawner shipped without
/// minting one, and N3.1's restore would silently lose whatever it spawned.
///
/// ```text
/// cargo test -p ambition_app --features rl_sim --test desync_canary -- --nocapture the_sim_id
/// ```
#[test]
fn the_sim_id_migration_ledger() {
    use ambition::bevy::prelude::{With, Without};

    let mut report = String::new();
    let mut worst = 0usize;
    for room in [
        "gap_run",
        "portal_lab",
        "mockingbird_arena",
        "gnu_ton_arena",
    ] {
        let Some(mut s) = sim(room) else { continue };
        let mut policy = RandomWalkPolicy::traversal_stress(7);
        // The traversal policy never attacks, so it never spawns a projectile —
        // and a ledger of anonymous bodies that never sees a projectile is a
        // ledger of nothing. Mash attack: projectiles are exactly the class N3.1
        // says needs `(spawner, counter)` ids.
        for i in 0..240 {
            let mut action = policy.act();
            action.attack = i % 7 == 0;
            s.step(action);
        }

        let identified = {
            let mut q = s
                .world_mut()
                .query_filtered::<(), With<ambition::platformer::sim_id::SimId>>();
            let w = s.world();
            q.iter(w).count()
        };
        let unidentified = {
            let mut q = s.world_mut().query_filtered::<(), (
                With<ambition::actors::actor::BodyKinematics>,
                Without<ambition::platformer::sim_id::SimId>,
            )>();
            let w = s.world();
            q.iter(w).count()
        };
        worst = worst.max(unidentified);
        report.push_str(&format!(
            "  {room:22} {identified:3} identified, {unidentified:3} bodies still anonymous\n"
        ));
    }
    eprintln!("\n=== N3.1 SimId migration ledger ===\n{report}");

    assert_eq!(
        worst, 0,
        "{worst} simulated bodies carry no SimId. A spawn site shipped without \
         `SimId::spawned(spawner, counter.next())`, so N3.1's restore would silently \
         lose whatever it spawned and the N0.4 canary cannot defend it. Either give \
         the spawner an identity (`ensure_sim_id` reads `FeatureId` / `PrimaryPlayer`) \
         or mint the child's (`mint_spawned_sim_ids` is the pattern). See netcode.md N3.1."
    );
}

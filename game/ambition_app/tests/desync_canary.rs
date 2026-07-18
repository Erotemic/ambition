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

use ambition::runtime::snapshot::{compare_hash_streams, SnapshotRegistry};
use ambition_app::rl_sim::TimestepMode;
use ambition_app::AmbitionSim;
use ambition_app::{RandomWalkPolicy, SandboxSim, SandboxSimOptions};

/// Some probes below (the ghost-room boot) panic on purpose. Without this,
/// every run prints alarming backtraces for a test that passed.
fn quietly<T>(f: impl FnOnce() -> T) -> std::thread::Result<T> {
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
    std::panic::set_hook(prev);
    out
}

/// **The registry the app actually built**, taken out of the sim's world.
///
/// `SnapshotRegistryPlugin` installs it early and every plugin after may add the sim
/// state it owns — including `ambition_content`'s boss specials, whose types
/// `ambition_runtime` cannot name. Building a fresh engine-only registry here would
/// test a registry no binary uses.
///
/// It is REMOVED rather than borrowed: no system reads it, the tests want it owned
/// across `&mut world` calls, and taking it makes "who owns the definition of the sim"
/// answerable in one place.
fn registry_of(sim: &mut SandboxSim) -> SnapshotRegistry {
    sim.world_mut()
        .remove_resource::<SnapshotRegistry>()
        .expect("SnapshotRegistryPlugin installs it")
}

/// Build a sandbox sim for `room` **and prove it landed in that room**, or say why
/// it could not.
///
/// H1 (guardrail credibility): "required rooms can disappear from the gate." The
/// disappearance is subtler than a build error. An unknown room id does not fail
/// construction — `init_sandbox_resources` calls `set_start_by_id`, and on no match
/// it prints a warning and leaves the LDtk project's *authored* start room active
/// (see `resources.rs`). So `SandboxSim::new_with_options(..).is_ok()` is not enough:
/// a fixture-path regression that makes `room` unmatchable would silently run every
/// gate below against the WRONG world and pass. `try_sim` therefore also checks that
/// the active room is the one asked for — matching `room_index_by_id`'s own rule
/// (id **or** world name).
fn try_sim(room: &str) -> Result<SandboxSim, String> {
    let opts = SandboxSimOptions::default()
        .with_timestep(TimestepMode::fixed_60hz())
        .with_start_room(room);
    let sim = SandboxSim::new_with_options(opts)
        .map_err(|e| format!("room `{room}` failed to build: {e}"))?;
    let active = {
        let spec = ambition::platformer::lifecycle::session_world_component::<
            ambition::world::rooms::RoomSet,
        >(sim.world())
        .ok_or_else(|| format!("room `{room}`: no RoomSet after build"))?
        .active_spec();
        if spec.id == room || spec.world.name == room {
            None
        } else {
            Some(spec.id.clone())
        }
    };
    if let Some(fell_into) = active {
        return Err(format!(
            "room `{room}` did not become active — the sim fell back to `{fell_into}`. \
             A required room that silently becomes another room makes its gate pass \
             against the wrong world (H1)."
        ));
    }
    Ok(sim)
}

/// A **required** room. A load failure — or a silent fallback to another room — is a
/// HARD failure here, not a skip. Every gate in this file (canary, replay oracle,
/// SimId ledger, coverage ledger) builds through this, so a room that vanishes takes
/// its gate down with it instead of passing vacuously.
fn sim(room: &str) -> SandboxSim {
    try_sim(room).unwrap_or_else(|e| panic!("{e}"))
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
    for (room, seed) in [
        ("gap_run", 1),
        ("portal_lab", 42),
        ("mockingbird_arena", 2026),
    ] {
        let (mut a, mut b) = (sim(room), sim(room));
        // ONE registry for both sims: two sims hashed by two definitions of "the sim"
        // are not comparable.
        let reg = registry_of(&mut a);
        assert!(reg.len() >= 3, "the registry declares something to defend");
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
    let (mut a, mut b) = (sim("gap_run"), sim("gap_run"));
    // ONE registry, for both sims: two sims hashed by two definitions of "the sim"
    // are not comparable. `b` keeps its copy as an unread resource.
    let reg = registry_of(&mut a);
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
    let mut s = sim("gap_run");
    let reg = registry_of(&mut s);
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

/// **A required room that fails to load — or silently becomes another room — is a
/// HARD failure, not a skip.** (H1, guardrail credibility.)
///
/// This is the poison test for the hard-fail behavior in `sim`/`try_sim` above, and
/// it is in the same commit as that behavior (the poison-test atomicity rule:
/// static-audit-response-2026-07-10.md). Before Series 1, every gate in this file
/// opened with `let Some(s) = sim(room) else { continue }`, so a fixture-load failure
/// became a green skip. Worse, an unknown room id does not even fail to build:
/// `set_start_by_id` warns and leaves the authored start room active, so a required
/// room could vanish and be replaced by the WRONG world with nothing crying.
///
/// This proves the new `sim` cries in both directions: `try_sim` reports the
/// substitution, and `sim` turns that report into a panic.
#[test]
fn a_missing_required_room_is_a_hard_failure_not_a_skip() {
    const GHOST: &str = "no_such_room_exists_anywhere";

    // The failure is REPORTED, not swallowed. The build succeeds (an unknown room
    // falls back to the authored start room), so the tell is that the active room is
    // not the one asked for.
    let err = match try_sim(GHOST) {
        // `SandboxSim` is not `Debug`, so we cannot use `expect_err`.
        Ok(_) => panic!(
            "a room no world authors must not pass `try_sim` — it falls back to the \
             authored start room, which is exactly the silent substitution H1 is about"
        ),
        Err(e) => e,
    };
    assert!(
        err.contains(GHOST) && err.contains("fell back"),
        "the failure must name the missing room and the substitution, so a real \
         fixture-path regression is diagnosable: {err}"
    );

    // ...and the required-room helper turns that report into a panic, so no gate below
    // can run against a substituted room and pass.
    let crashed = quietly(|| sim(GHOST)).is_err();
    assert!(
        crashed,
        "`sim` must panic on a room it cannot make active — otherwise a fixture-path \
         regression makes a required room disappear from the canary, replay, and \
         ledgers, and every one of them passes vacuously (H1)."
    );

    // The control: a room that DOES load passes `try_sim`. Without this, a `try_sim`
    // that returned `Err` unconditionally would also satisfy the assertions above.
    assert!(
        try_sim("gap_run").is_ok(),
        "a real room must still build, or the hard-fail helper rejects everything"
    );
}

/// **The coverage ledger reacts to new sim debt — it is not a count-only false green.**
/// (Audit M10 / Series 1: the coverage-sensitivity poison test.)
///
/// `the_snapshot_coverage_ledger` pins the *number* of unregistered `ambition_`
/// resources. A count pin is only trustworthy if the count actually MOVES when real
/// debt is added. This inserts a deliberately-unregistered, `ambition_`-namespaced
/// resource into a live world and proves `unclaimed_resources` grows by exactly it — so
/// a pin trips on the addition rather than preserve a false green. The `ambition_`
/// namespace matters: a resource named otherwise is invisible to the filter, which is
/// why the poison fixture lives in `ambition_app` and not in this test crate.
///
/// It also pins the *other* half of why the resource ledger exists: a
/// `Messages<ambition_..>` buffer is named `bevy_ecs::message::Messages<..>`, and the
/// filter is `contains("ambition_")` — not `starts_with` — precisely so that class is
/// not hidden behind Bevy's own module path. At least one such buffer must be observed.
#[test]
fn the_coverage_ledger_reacts_to_a_new_unregistered_resource() {
    use ambition_app::rl_sim::CoveragePoisonResource;

    let mut s = sim("mockingbird_arena");
    let reg = registry_of(&mut s);
    for _ in 0..20 {
        s.step(RandomWalkPolicy::traversal_stress(7).act());
    }

    let before: Vec<String> = reg
        .unclaimed_resources(s.world())
        .into_iter()
        .map(|c| c.name)
        .collect();

    // The `Messages<T>` class the `contains` filter exists to catch is really present —
    // a regression to `starts_with("ambition_")` would hide every one of them.
    assert!(
        before
            .iter()
            .any(|n| n.contains("Messages<") && n.contains("ambition_")),
        "no `Messages<ambition_..>` in the coverage set — the `contains(\"ambition_\")` \
         filter that unhid an entire message class has regressed. Saw: {before:?}"
    );

    // Add real debt: a mutable sim resource nobody registered.
    s.world_mut().insert_resource(CoveragePoisonResource);
    let after: Vec<String> = reg
        .unclaimed_resources(s.world())
        .into_iter()
        .map(|c| c.name)
        .collect();

    assert_eq!(
        after.len(),
        before.len() + 1,
        "adding one unregistered `ambition_`-namespaced resource must grow the coverage \
         ledger by exactly one. If it does not, the ledger is blind to new sim debt and \
         its count pin is a false green (audit M10)."
    );
    assert!(
        after.iter().any(|n| n.contains("CoveragePoisonResource")),
        "the ledger grew, but not by the resource we added: {after:?}"
    );
}

/// **The sim-resource universe excludes presentation via a NAMED policy** (audit S2.8).
///
/// `lossless()` measures sim-state resource DEBT, not the raw unregistered total — most
/// of the ~181 unregistered resources are presentation, derived-per-frame, or authored
/// content that a rollback must not restore. This proves the `SIM_RESOURCE_EXCLUSIONS`
/// policy has teeth in BOTH directions: it drops known presentation/authored classes AND
/// keeps genuine sim state. A policy that excluded nothing would make `lossless()`
/// unachievable forever; one that excluded too much would make it vacuously true.
#[test]
fn the_sim_resource_universe_excludes_presentation_but_keeps_sim_state() {
    let mut s = sim("mockingbird_arena");
    let reg = registry_of(&mut s);
    for _ in 0..20 {
        s.step(RandomWalkPolicy::traversal_stress(7).act());
    }

    let total: Vec<String> = reg
        .unclaimed_resources(s.world())
        .into_iter()
        .map(|c| c.name)
        .collect();
    let sim_only: Vec<String> = reg
        .unclaimed_sim_resources(s.world())
        .into_iter()
        .map(|c| c.name)
        .collect();

    assert!(
        sim_only.len() < total.len(),
        "the exclusion policy dropped nothing ({} == {}) — it is not being applied",
        sim_only.len(),
        total.len()
    );

    // Dropped: clear presentation / authored content. (Each must be in the total first,
    // or the exclusion is untested.)
    for excluded in [
        "ambition_sim_view::",
        "ambition_ldtk_map::",
        "camera_ease::CameraShakeState",
    ] {
        assert!(
            total.iter().any(|n| n.contains(excluded)),
            "`{excluded}` is not in the unregistered total — its exclusion is untested"
        );
        assert!(
            !sim_only.iter().any(|n| n.contains(excluded)),
            "the sim-resource universe still contains presentation `{excluded}` (S2.8)"
        );
    }

    // Kept: genuine sim state must still count as debt.
    assert!(
        sim_only
            .iter()
            .any(|n| n.contains("ambition_time::ClockState")),
        "the policy dropped genuine sim state (`ClockState`) — it excludes too much: {sim_only:?}"
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
        let mut s = sim(room);
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

/// **N3.1's registration checklist, computed against real rooms.**
///
/// `restore` patches a surviving entity's registered components and leaves the rest
/// alone. Every component on a `SimId` entity that the registry neither registers
/// nor declares derived is therefore **stale** after a rewind: it still reads the
/// tick we rewound FROM. This test names them, counts them, and pins the count.
///
/// For an immutable authored fact — a moveset, a faction — stale and correct are
/// the same thing, so the number is not the debt itself; it is an upper bound on it,
/// and `a_restored_sim_replays_the_future_it_was_rewound_from` is what measures
/// whether the stale state actually leaks. netcode.md's N3.1 pin lists what each sim
/// crate still owes (move playbacks + cooldowns, brain memory, portal transit state,
/// falling-sand grids, every seeded RNG).
///
/// What this ledger buys is that the debt is a NUMBER, checked on every run, rather
/// than a paragraph someone reads once. It may fall. It may not rise.
///
/// ```text
/// cargo test -p ambition_app --features rl_sim --test desync_canary -- --nocapture the_snapshot
/// ```
#[test]
fn the_snapshot_coverage_ledger() {
    let mut report = String::new();
    let mut worst = 0usize;
    // The UNION of every component debt name sampled across all rooms, for the reviewed
    // inventory subset check (re-audit finding 6) — broader than `worst`, which is a single
    // room's peak COUNT.
    let mut component_debt: std::collections::BTreeSet<String> = Default::default();

    for room in [
        "gap_run",
        "portal_lab",
        "mockingbird_arena",
        "gnu_ton_arena",
    ] {
        let mut s = sim(room);
        // The registry the APP built: engine entries plus whatever `ambition_content`
        // registered for its own boss specials.
        let reg = registry_of(&mut s);
        let mut policy = RandomWalkPolicy::traversal_stress(7);

        // **Sample as we go, and keep the worst.**
        //
        // The first version of this test measured once, after 120 ticks. By then the
        // arena bosses were dead and despawned, and `gnu_ton_arena` reported the same
        // 35 types as `gap_run` — the count of a world containing only the player.
        // The debt was real the whole time; the instrument was looking at the wrong
        // tick. A ledger that under-reports is worse than no ledger.
        let mut peak: Vec<_> = Vec::new();
        for i in 0..120 {
            if i % 20 == 0 {
                let unclaimed = reg.unclaimed_components(s.world());
                for c in &unclaimed {
                    component_debt.insert(c.name.clone());
                }
                if unclaimed.len() > peak.len() {
                    peak = unclaimed;
                }
            }
            let mut action = policy.act();
            action.attack = i % 7 == 0;
            s.step(action);
        }
        worst = worst.max(peak.len());
        report.push_str(&format!(
            "  {room:22} {:3} component types a rewind leaves stale (peak over 120 ticks)\n",
            peak.len()
        ));
        for c in peak.iter().take(4) {
            report.push_str(&format!("      {c}\n"));
        }
    }
    eprintln!("\n=== N3.1 snapshot coverage ledger ===\n{report}");

    // **The other half of the ledger.** `unclaimed_components` walks entities; a
    // `Resource` sits on none, so it was invisible. netcode.md's N3.1 checklist names
    // resources explicitly — "`WorldTime` + every sim clock", "every seeded RNG
    // resource", "active room + spawn state", "falling-sand grids (ONE resource blob)".
    // None of that was being measured.
    let resources = {
        let mut s = sim("mockingbird_arena");
        let reg = registry_of(&mut s);
        for _ in 0..40 {
            s.step(RandomWalkPolicy::traversal_stress(7).act());
        }
        reg.unclaimed_resources(s.world())
    };
    eprintln!(
        "=== N3.1 unregistered sim RESOURCES ({}) ===\n{}\n",
        resources.len(),
        resources
            .iter()
            .map(|r| format!("  {r}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
    eprintln!(
        "  (component NAMES need bevy's `debug` feature; the counts key on TypeId \
         and are exact either way)\n"
    );

    // The resource half, pinned too. It is LARGE and most of it is presentation or
    // derived — `ActorRenderIndex`, `CameraShakeState`, `DeveloperTools`. But
    // `EncounterState` is in there, holding a live encounter phase and wave run, and
    // so is `EnemyProjectileState`.
    //
    // **45 of them are `Messages<T>` buffers**, and they were hidden twice over: a
    // `Resource` sits on no entity, and `Messages<ambition_..::HitEvent>` is NAMED
    // `bevy_ecs::message::Messages<..>`, so a `starts_with("ambition_")` filter missed
    // every one. A message written before a snapshot and read after a restore is an
    // event that happens twice — `ActorActionMessage` is how a boss's Special reaches
    // the executor on the NEXT tick, and it is very likely why `mockingbird_arena`
    // replays exactly for twenty ticks and breaks on the twenty-first.
    //
    // `declare_derived` is how the presentation half comes off; a codec is how the rest
    // does; the message buffers want neither, they want N3.2's resim discipline
    // ("side-effect suppression during resim").
    // **The debt is pinned by TYPE NAME, not just by count** (re-audit finding 6).
    //
    // A count-only gate (`len <= 181`) is *substitution-blind*: a newly-unregistered sim
    // type can replace a removed debt type and hold the count constant, so the gate stays
    // green while the debt has silently CHANGED. Requiring the current debt to be a SUBSET of
    // a reviewed inventory makes any NEW type a review event — register it, or add it to the
    // inventory (which is itself a reviewed diff, like a `SIM_RESOURCE_EXCLUSIONS` entry). The
    // count thresholds remain, but as SUMMARIES beneath the subset gate, not the enforcement.
    //
    // Type NAMES need `bevy_ecs/debug`, which `rl_sim` turns on — the resource census is
    // reliable in this build (asserted by `a_restore_of_a_real_room...`). The inventories live
    // in sibling text files so a debt change is a reviewable diff, not a buried constant.
    let resource_debt: std::collections::BTreeSet<String> =
        resources.iter().map(|c| c.name.clone()).collect();

    fn inventory(s: &str) -> std::collections::BTreeSet<&str> {
        s.lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .collect()
    }
    let known_resources = inventory(include_str!("known_resource_debt.txt"));
    let known_components = inventory(include_str!("known_component_debt.txt"));

    // Dump the full current inventories (sorted), so a debt change is auditable in the test
    // log and the reviewed files can be regenerated from `--nocapture` output.
    eprintln!(
        "=== reviewed resource-debt inventory ({} current / {} pinned) ===\n{}\n",
        resource_debt.len(),
        known_resources.len(),
        resource_debt
            .iter()
            .map(|n| n.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    );
    eprintln!(
        "=== reviewed component-debt inventory ({} current / {} pinned) ===\n{}\n",
        component_debt.len(),
        known_components.len(),
        component_debt
            .iter()
            .map(|n| n.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    );

    let new_resources: Vec<&str> = resource_debt
        .iter()
        .map(String::as_str)
        .filter(|n| !known_resources.contains(n))
        .collect();
    assert!(
        new_resources.is_empty(),
        "{} unregistered resource type(s) are NOT in the reviewed known-debt inventory \
         (game/ambition_app/tests/known_resource_debt.txt) — a substitution the old count-only \
         gate would have missed (re-audit finding 6). Register them, or record them in the \
         inventory as a reviewed diff:\n{}",
        new_resources.len(),
        new_resources.join("\n"),
    );
    let new_components: Vec<&str> = component_debt
        .iter()
        .map(String::as_str)
        .filter(|n| !known_components.contains(n))
        .collect();
    assert!(
        new_components.is_empty(),
        "{} unregistered component type(s) are NOT in the reviewed known-debt inventory \
         (game/ambition_app/tests/known_component_debt.txt) — a substitution the old count-only \
         gate would have missed (re-audit finding 6). Register them, declare them derived, or \
         record them in the inventory as a reviewed diff:\n{}",
        new_components.len(),
        new_components.join("\n"),
    );

    // The count thresholds, kept as SUMMARIES beneath the name-level subset gate above.
    // 2026-07-13: +5 reviewed authored-content resources from the App-local
    // character/hostile/boss authority completion (BossCatalog{,Registry},
    // CharacterRoster{,Registry}, AuthoredAttackVolumeResolver, actor placement
    // lowering registry) — reviewed in known_resource_debt.txt.
    // 2026-07-13 (shell-host campaign): +2 reviewed composition resources
    // (AmbitionPreparedWorld immutable boot data, ActiveAudioSelection
    // presentation authority) — reviewed in known_resource_debt.txt.
    // 2026-07-14 (exact audio contexts + provider-rendered SFX/adaptive music):
    // replace the cue-id-only index and raw SfxMessage queue with App-local
    // provider catalogs, provider-qualified bank/cache resources, exact
    // frontend/gameplay ownership, playback evidence, and context-change facts.
    // These are presentation/composition state, not rollback simulation state;
    // each reviewed type is listed in known_resource_debt.txt.
    const KNOWN_RESOURCE_DEBT: usize = 197;
    assert!(
        resources.len() <= KNOWN_RESOURCE_DEBT,
        "{} unregistered `ambition_*` resources, up from the pinned \
         {KNOWN_RESOURCE_DEBT}. A resource sits on no entity, so `unclaimed_components` \
         never saw one and `restore` never touched one. See netcode.md N3.1.",
        resources.len()
    );
    // 2026-07-16 (E11): +3 reviewed authored components on the newly
    // SimId-carrying encounter-authority entities (Encounter identity,
    // EncounterObjective, boss-wrap EncounterDef) — authored/immutable config;
    // the mutable state (lifecycle/participants/waves) IS registered. Reviewed
    // in known_component_debt.txt.
    // 2026-07-16 (snapshot closeout): ActorAggression, ActorDisposition,
    // BodyMelee, and RoomScopedEntity moved from reviewed debt into registered
    // state. Lower the guard with the name-level inventory rather than leaving
    // the old allowance available for unrelated substitutions.
    const KNOWN_DEBT: usize = 61;
    assert!(
        worst <= KNOWN_DEBT,
        "{worst} component types on SimId entities are neither registered as sim \
         state nor declared derived, up from the pinned {KNOWN_DEBT}. A rewind leaves \
         every one of them stale. Register it, declare it derived, or lower the pin — \
         but do not let the debt grow silently. See netcode.md N3.1."
    );
}

/// **The active-room ownership invariant** (audit item 2, N3.2).
///
/// At every tick, every live `placement:<iid>` entity is authored by the THEN-active
/// room. A `placement` the active room does not author would be a cross-room leak — a
/// `central_hub_main` NPC alive while `portal_lab` is the world.
///
/// This test is the audit's item-2 enforcement, and its result REFUTES the leak
/// hypothesis: it PASSES. The traversal policy bounces the player through a shared
/// loading zone between `portal_lab` and `central_hub_complex`; while
/// `central_hub_complex` is active its `NpcSpawn-0017` is legitimately alive, and it
/// is despawned the instant the player transitions away (confirmed by trace: it lives
/// only while its room is active). The rewind side of this story is closed: the atomic
/// room transaction (netcode.md N3.2b) stages the snapshot's room before reconciling,
/// so `portal_lab`'s cross-transition window now rewinds exactly — see
/// `restore_stages_the_snapshot_room_across_a_transition` and the CLEAN roster in
/// `a_restored_sim_replays_the_future_it_was_rewound_from`.
///
/// The invariant is precise: a live `placement:<iid>` that some room authors must be
/// authored by the ACTIVE room. A `placement:<iid>` that NO room authors is a
/// dynamically-spawned child (a boss's giant hands, spawned by the boss, not the
/// room) — a spawned child, not a cross-room leak, so it is exempt here. As of the
/// N3.2 boss-hand fix those hands mint `SimId::spawned(giant, ordinal)` =
/// `placement:<giant_iid>/<ordinal>` from the giant's AUTHORED id (no longer the
/// non-deterministic `giant_gnu_hand_left_7` derived from an entity index), so they
/// are deterministic spawned ids; they still surface below because a spawned child of
/// a `placement:` parent is itself `placement:`-prefixed and no room authors it.
/// `slot:` (the player) is carried/persistent and exempt.
#[test]
fn every_placement_entity_is_owned_by_the_active_room_every_tick() {
    use std::collections::{BTreeMap, BTreeSet};

    for room in [
        "gap_run",
        "portal_lab",
        "mockingbird_arena",
        "gnu_ton_arena",
    ] {
        let mut s = sim(room);

        // Which rooms author each authored id? Built once — it is the same RoomSet
        // every tick. An id maps to a room via ANY of the three authored lists
        // (`placements`, `enemy_spawns`, `boss_spawns`) — the same three arms
        // `RoomConstructionPlan::respawn_authoritative_entity` reconstructs from.
        let authored_by: BTreeMap<String, BTreeSet<String>> = {
            let rs = ambition::platformer::lifecycle::session_world_component::<
                ambition::world::rooms::RoomSet,
            >(s.world())
            .expect("a RoomSet");
            let mut map: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
            for spec in &rs.rooms {
                let ids = spec
                    .placements
                    .iter()
                    .map(|p| p.id.0.clone())
                    .chain(spec.enemy_spawns.iter().map(|e| e.id.clone()))
                    .chain(spec.boss_spawns.iter().map(|b| b.id.clone()));
                for iid in ids {
                    map.entry(iid).or_default().insert(spec.id.clone());
                }
            }
            map
        };

        // The no-room category, reported separately (reviewer): a `placement:<id>` no
        // room authors is a dynamically-spawned child. The giant hands now mint
        // `SimId::spawned(giant, ordinal)` from the giant's authored id, so this is a
        // DETERMINISTIC spawned identity — legible as `<giant_iid>/<ordinal>` — not the
        // old entity-index debt. It surfaces here only because a spawned child of a
        // `placement:` parent is itself `placement:`-prefixed. Collected so the set is
        // visible, not silently skipped.
        let mut dynamic_debt: BTreeSet<String> = BTreeSet::new();

        let mut policy = RandomWalkPolicy::traversal_stress(3);
        for tick in 0..120u32 {
            s.step(policy.act());

            let active_id = {
                let rs = ambition::platformer::lifecycle::session_world_component::<
                    ambition::world::rooms::RoomSet,
                >(s.world())
                .expect("a RoomSet");
                rs.active_spec().id.clone()
            };
            let live: Vec<String> = {
                let mut q = s
                    .world_mut()
                    .query::<&ambition::platformer::sim_id::SimId>();
                let w = s.world();
                q.iter(w).map(|id| id.as_str().to_string()).collect()
            };

            for id in live {
                let Some(iid) = id.strip_prefix("placement:") else {
                    continue; // slot: is carried
                };
                match authored_by.get(iid) {
                    // Authored by the active room: valid.
                    Some(rooms) if rooms.contains(&active_id) => {}
                    // Authored EXCLUSIVELY by another room: a real cross-room leak.
                    Some(rooms) => panic!(
                        "OWNERSHIP VIOLATION in `{room}` at tick {tick}: `placement:{iid}` \
                         is alive while `{active_id}` is active, but it is authored by \
                         {rooms:?}, not the active room — a cross-room leak (audit item 2)."
                    ),
                    // Authored by NO room: dynamic-spawn identity debt, not a leak.
                    None => {
                        dynamic_debt.insert(iid.to_string());
                    }
                }
            }
        }

        if !dynamic_debt.is_empty() {
            eprintln!(
                "  [{room}] spawned children (placement: ids no room authors — now \
                 deterministic SimId::spawned(giant, ordinal), netcode.md N3.2): {dynamic_debt:?}"
            );
        }
    }
}

/// The active room's `RoomSpec` id.
fn active_room(s: &SandboxSim) -> String {
    ambition::platformer::lifecycle::session_world_component::<ambition::world::rooms::RoomSet>(
        s.world(),
    )
    .map(|rs| rs.active_spec().id.clone())
    .unwrap_or_default()
}

/// **The atomic active-room transaction** (netcode.md N3.2b).
///
/// A rollback window MAY span a room transition: `restore` STAGES the snapshot's
/// room — through the same canonical construction a room transition runs (the
/// scoped-entity sweep, active-spec/geometry swap, moving-platform rebuild, and
/// the App-installed placement lowering) — and then reconciles the registered
/// blobs against the right `RoomSpec`. The restored world must be
/// registered-state-identical to the taken one: same hash, same re-taken
/// snapshot, active room included.
///
/// `portal_lab`'s traversal bounces the player through a shared loading zone with
/// `central_hub_complex`, which is exactly how a window comes to span a transition.
#[test]
fn restore_stages_the_snapshot_room_across_a_transition() {
    use ambition::runtime::snapshot::{restore, take};

    let mut s = sim("portal_lab");
    let reg = registry_of(&mut s);
    let mut policy = RandomWalkPolicy::traversal_stress(3);

    // Step until the player has bounced OUT of portal_lab, and snapshot there.
    let mut snap = None;
    for _ in 0..400 {
        s.step(policy.act());
        if active_room(&s) != "portal_lab" {
            snap = Some((take(s.world(), &reg), active_room(&s)));
            break;
        }
    }
    let (snap, snap_room) = snap.expect("the traversal never left portal_lab in 400 ticks");
    let at_snapshot = reg.hash_world(s.world());

    // Step until back in portal_lab, so the window now spans a transition.
    let mut returned = false;
    for _ in 0..400 {
        s.step(policy.act());
        if active_room(&s) == "portal_lab" {
            returned = true;
            break;
        }
    }
    assert!(returned, "the traversal never returned to portal_lab");

    let report = restore(s.world_mut(), &snap, &reg).expect(
        "a cross-room restore stages the snapshot's room through the canonical \
         construction (N3.2b) rather than refusing",
    );
    assert_eq!(
        report.staged_room.as_deref(),
        Some(snap_room.as_str()),
        "the transaction reports the room it staged"
    );
    assert_eq!(
        active_room(&s),
        snap_room,
        "the snapshot's room is the live one after restore — the active room is \
         restored sim state"
    );
    assert_eq!(
        reg.hash_world(s.world()),
        at_snapshot,
        "a staged cross-room restore reproduces the registered state bit for bit"
    );
    assert_eq!(
        take(s.world(), &reg),
        snap,
        "a snapshot of the staged-and-restored world is the snapshot it was \
         restored from"
    );
}

/// **A staged restore rebuilds CONTENT-STAGED occupants, complete** (N3.2b
/// closeout — GPT-5.6 review, 2026-07-16).
///
/// The duel arena's two fighters are not `RoomSpec` placements: content stages
/// them through the room-content staging seam (`SpawnActorRequest`s emitted when
/// the room's contents stage). A cross-room restore INTO `duel_arena` must
/// therefore rebuild them through that same seam — a restore that "succeeds" by
/// bare-spawning an identity-only entity and patching registered rows onto it
/// produces fighters with no `Brain`, no faction, no grudge: a hollow roster the
/// registered hash alone cannot see. Three teeth, in order of bluntness:
///
/// 1. one tick after the restore, each fighter's component SET equals the set
///    it carried one tick after the snapshot — the identical sim point, so
///    per-tick derived attachments (`SurfaceUpright`, portal transit tags)
///    compare like-for-like and authored config (`Brain`, faction, moveset)
///    cannot be silently absent;
/// 2. the registered hash and re-taken snapshot match exactly;
/// 3. the replayed suffix — including the same forced door transition at the
///    same tick — reproduces the abandoned future tick for tick.
#[test]
fn a_staged_restore_rebuilds_the_duel_roster_completely() {
    use ambition::runtime::snapshot::{restore, take};
    use ambition::world::rooms::{RoomSet, RoomTransitionRequested};
    use std::collections::BTreeSet;

    /// Every component TYPE the entity carrying `sim_id` wears, by name.
    fn roster_of(s: &mut SandboxSim, sim_id: &str) -> BTreeSet<String> {
        let entity = {
            let mut q = s.world_mut().query::<(
                bevy::ecs::entity::Entity,
                &ambition::platformer::sim_id::SimId,
            )>();
            let w = s.world();
            q.iter(w)
                .find(|(_, id)| id.as_str() == sim_id)
                .map(|(e, _)| e)
                .unwrap_or_else(|| panic!("`{sim_id}` is not alive"))
        };
        s.world()
            .inspect_entity(entity)
            .expect("the fighter entity just resolved")
            .map(|info| info.name().to_string())
            .collect()
    }

    let mut s = sim("duel_arena");
    let reg = registry_of(&mut s);

    // Let the staged duel develop: brains target, moves play, health moves.
    let mut warm = RandomWalkPolicy::traversal_stress(3);
    for _ in 0..60 {
        s.step(warm.act());
    }

    const PCA: &str = "placement:duel_pca";
    const ROBOT: &str = "placement:duel_robot";
    assert!(
        roster_of(&mut s, PCA)
            .iter()
            .any(|n| n.ends_with("brain::Brain")),
        "the warm-up staged a REAL fighter (a Brain)"
    );

    let snap = take(s.world(), &reg);
    let at_snapshot = reg.hash_world(s.world());

    // The duel arena's one exit is a Door: resolve its transition through the
    // canonical room graph, exactly as an interact press would.
    let door = {
        let rs = ambition::platformer::lifecycle::session_world_component::<RoomSet>(s.world())
            .expect("a RoomSet");
        let zone = rs
            .active_loading_zones()
            .iter()
            .find(|z| z.id == "duel_arena_entry")
            .expect("duel_arena authors its entry door")
            .clone();
        rs.transition_for_player(zone.aabb, ambition::engine_core::Vec2::ZERO, true)
            .expect("the duel arena's door resolves through the room graph")
    };

    // One 60-tick suffix, with the door forced at tick 10 — the SAME external
    // input both times, so the replay is of the same future.
    let inputs: Vec<_> = {
        let mut p = RandomWalkPolicy::traversal_stress(99);
        (0..60).map(|_| p.act()).collect()
    };
    // Runs the whole suffix and, after its FIRST tick — the identical sim
    // point in both runs, with every per-tick derived attachment freshly
    // published — samples each fighter's full component set.
    let mut run_suffix = |s: &mut SandboxSim| -> (Vec<u64>, Vec<BTreeSet<String>>) {
        let mut hashes = Vec::new();
        let mut rosters = Vec::new();
        for (i, a) in inputs.iter().enumerate() {
            if i == 10 {
                s.world_mut()
                    .resource_mut::<bevy::ecs::message::Messages<RoomTransitionRequested>>()
                    .write(RoomTransitionRequested::new(door.clone(), None));
            }
            s.step(a.clone());
            hashes.push(reg.hash_world(s.world()));
            if i == 0 {
                rosters = vec![roster_of(s, PCA), roster_of(s, ROBOT)];
            }
        }
        (hashes, rosters)
    };

    let (first, rosters_before) = run_suffix(&mut s);
    assert_eq!(
        active_room(&s),
        "central_hub_complex",
        "the forced door moved the window out of the duel arena"
    );

    let report = restore(s.world_mut(), &snap, &reg)
        .expect("a cross-room restore stages the duel arena rather than refusing");
    assert_eq!(report.staged_room.as_deref(), Some("duel_arena"));
    assert_eq!(
        reg.hash_world(s.world()),
        at_snapshot,
        "the staged restore reproduces the registered state bit for bit"
    );
    assert_eq!(
        take(s.world(), &reg),
        snap,
        "a snapshot of the restored world is the snapshot it was restored from"
    );

    // Tooth 3 (and tooth 1's sample): the same suffix replays into the same
    // future, from the staged-and-reconciled roster.
    let (second, rosters_after) = run_suffix(&mut s);

    // Tooth 1: at the identical sim point (one tick past the snapshot), the
    // rebuilt fighters wear exactly the component set the originals wore —
    // authored config included, not just registered blobs. A hollow rebuild
    // (no Brain, no faction, no grudge) cannot pass this.
    for (id, (before, after)) in [PCA, ROBOT]
        .iter()
        .zip(rosters_before.iter().zip(&rosters_after))
    {
        let missing: Vec<&String> = before.difference(after).collect();
        let extra: Vec<&String> = after.difference(before).collect();
        assert!(
            missing.is_empty() && extra.is_empty(),
            "`{id}`'s restored component set differs from the original at the same \
             sim point — missing {missing:?}, extra {extra:?}: the staging seam did \
             not rebuild the roster the snapshot was taken over"
        );
    }

    // Tooth 3: the staged roster's mutable behavior state — Smash-brain
    // history/clocks, aggression/disposition, and the body melee cursor — rewinds
    // with the rest of the registered sim. The identical suffix must therefore
    // reproduce the abandoned future tick for tick.
    let diff = compare_hash_streams(&first, &second);
    assert!(
        diff.in_sync(),
        "the cross-room duel replay diverged at tick {:?}",
        diff.first_divergence_tick
    );
}

/// Same-room rollback can span the death of one content-staged actor. The pure
/// room stager is replayed as a coordinated batch so authored cross-member
/// relationships (the duelists' mutual grudges) are reconstructed too.
#[test]
fn same_room_restore_rebuilds_a_missing_content_staged_batch() {
    use ambition::combat::components::ActorAggression;
    use ambition::platformer::sim_id::SimId;
    use ambition::runtime::snapshot::{restore, take};

    const PCA: &str = "placement:duel_pca";
    const ROBOT: &str = "placement:duel_robot";

    let mut s = sim("duel_arena");
    let reg = registry_of(&mut s);
    let snapshot = take(s.world(), &reg);

    let ids = {
        let mut query = s.world_mut().query::<(bevy::ecs::entity::Entity, &SimId)>();
        query
            .iter(s.world())
            .map(|(entity, id)| (id.as_str().to_string(), entity))
            .collect::<std::collections::BTreeMap<_, _>>()
    };
    s.world_mut().despawn(*ids.get(PCA).expect("PCA staged"));

    restore(s.world_mut(), &snapshot, &reg).expect("same-room staged batch rebuilds");
    assert_eq!(take(s.world(), &reg), snapshot);

    let ids = {
        let mut query = s.world_mut().query::<(bevy::ecs::entity::Entity, &SimId)>();
        query
            .iter(s.world())
            .map(|(entity, id)| (id.as_str().to_string(), entity))
            .collect::<std::collections::BTreeMap<_, _>>()
    };
    let pca = *ids.get(PCA).expect("PCA restored");
    let robot = *ids.get(ROBOT).expect("robot restored");
    assert_eq!(
        s.world()
            .entity(pca)
            .get::<ActorAggression>()
            .unwrap()
            .grudge,
        Some(robot)
    );
    assert_eq!(
        s.world()
            .entity(robot)
            .get::<ActorAggression>()
            .unwrap()
            .grudge,
        Some(pca)
    );
}

/// **A refusal leaves the live room untouched** (N3.2b preflight-before-mutation).
///
/// A snapshot naming a room this world cannot build (a different prepared world /
/// content identity) is refused by the mutation-free staging preflight
/// (`RoomNotStageable`), and the world after the refusal is REGISTERED-STATE
/// IDENTICAL to the world before it — nothing was swept, swapped, or lowered.
#[test]
fn an_unstageable_room_refuses_with_the_world_untouched() {
    use ambition::runtime::snapshot::{restore, take, RestoreError};

    let mut s = sim("gap_run");
    let reg = registry_of(&mut s);
    let mut policy = RandomWalkPolicy::traversal_stress(3);
    for _ in 0..40 {
        s.step(policy.act());
    }

    let mut snap = take(s.world(), &reg);
    snap.active_room = Some("a_room_this_world_never_authored".to_string());
    let before = reg.hash_world(s.world());

    match restore(s.world_mut(), &snap, &reg) {
        Err(RestoreError::RoomNotStageable { room, .. }) => {
            assert_eq!(room, "a_room_this_world_never_authored");
        }
        other => {
            panic!("restore accepted a snapshot whose room no RoomSet authors — got {other:?}")
        }
    }
    assert_eq!(
        reg.hash_world(s.world()),
        before,
        "the staging preflight refused AFTER mutating the world — the transaction \
         is not atomic"
    );
}

/// **The active room is IN the registered hash** (re-audit finding 2).
///
/// `take` captures the active room and `restore` refuses a window that crosses it, so
/// the active room is sim state. If `hash_world` omitted it, the snapshot would carry
/// state the canary hash could not see, and two worlds differing ONLY in which room is
/// active would hash equal — "the hash is the serialization" would be false. Change only
/// the active-room cursor (a clone of the current room under a fresh id; every entity and
/// resource left untouched) and the registered hash must move.
#[test]
fn changing_only_the_active_room_changes_the_registered_hash() {
    use ambition::world::rooms::RoomSet;

    let mut s = sim("gap_run");
    let reg = registry_of(&mut s);
    let before = reg.hash_world(s.world());

    let mut rooms = ambition::platformer::lifecycle::session_world_component::<RoomSet>(s.world())
        .expect("the sandbox sim has a RoomSet")
        .clone();
    // A second room, identical to the active one but for its id, made active. Nothing
    // else in the world changes, so any hash movement is the room cursor alone.
    let mut probe = rooms.active_spec().clone();
    probe.id = format!("{}\u{0}hash-probe", probe.id);
    rooms.rooms.push(probe);
    rooms.active = rooms.rooms.len() - 1;
    *ambition::platformer::lifecycle::session_world_component_mut::<RoomSet>(s.world_mut())
        .expect("the sandbox sim has a mutable RoomSet") = rooms;

    let after = reg.hash_world(s.world());
    assert_ne!(
        before, after,
        "changing only the active room did not change the registered hash — the room \
         cursor is sim state the hash omitted (re-audit finding 2)"
    );
}

/// **N3.1's exit oracle: rewind, replay, and land in the same place.**
///
/// Run the sim, take a snapshot, run K ticks recording each hash, restore, replay
/// the same K inputs. The two hash streams must be identical.
///
/// This is strictly stronger than `take`'s unit round-trip, which says only that a
/// restored world *looks* like the taken one for one tick. This says it *continues*
/// like it. Any sim state the registry misses, and that feeds back into registered
/// state, diverges here on the tick it first matters — and `body_kinematics` is in
/// the hash, so "feeds back into registered state" means "moves anything at all".
///
/// **Every proven room is CLEAN**: the plain platformer room, both boss arenas
/// (brains, move playbacks, pattern clocks), and — since the atomic room
/// transaction (N3.2b) — the portal lab, whose window spans a room transition.
/// The DIRTY half of this ledger emptied and was deleted; a room that fails this
/// oracle now is a regression to diagnose, not a row to record.
#[test]
fn a_restored_sim_replays_the_future_it_was_rewound_from() {
    /// Rooms where a rewind is exact. This list may grow. It may not shrink.
    ///
    /// `gnu_ton_arena` joined it the day `GameplayElapsed` — an accumulating sim clock
    /// a brain stamps its memories with — was registered. `mockingbird_arena` joined it
    /// the day `BossEncounter.encounter` did: rewinding only the exposed
    /// `encounter_phase` mirror is rewinding a thermometer. **Two boss fights rewind and
    /// replay bit for bit.**
    ///
    /// `portal_lab` joined it the day the active room became restored sim state
    /// (netcode.md N3.2b): its 60-tick window SPANS a room transition (the snapshot is
    /// taken while `central_hub_complex` is active; the replay ends in `portal_lab`),
    /// and the atomic room transaction now STAGES the snapshot's room through the
    /// canonical construction before reconciling — `NpcSpawn-0017` rebuilds against the
    /// RIGHT `RoomSpec`, and **a cross-room rewind replays bit for bit**. The former
    /// DIRTY ledger is empty and gone; a NEW room that fails this oracle gets a precise
    /// diagnosis, not a ledger row.
    const CLEAN: &[&str] = &[
        "gap_run",
        "gnu_ton_arena",
        "mockingbird_arena",
        "portal_lab",
    ];

    for room in CLEAN {
        replay_after_rewind(room);
    }
}

/// Take a snapshot, run K ticks hashing each, restore, replay the same K inputs,
/// and demand the two hash streams agree. Panics on divergence, naming the tick.
fn replay_after_rewind(room: &str) {
    use ambition::runtime::snapshot::{restore, take};

    let mut s = sim(room);
    let reg = registry_of(&mut s);

    // Warm up, so the snapshot is of a moving world rather than of a spawn pose.
    let mut warm = RandomWalkPolicy::traversal_stress(3);
    for _ in 0..40 {
        s.step(warm.act());
    }

    let snap = take(s.world(), &reg);
    let at_snapshot = reg.hash_world(s.world());
    let inputs: Vec<_> = {
        let mut p = RandomWalkPolicy::traversal_stress(99);
        (0..60).map(|_| p.act()).collect()
    };

    let first: Vec<u64> = inputs
        .iter()
        .map(|a| {
            s.step(a.clone());
            reg.hash_world(s.world())
        })
        .collect();

    let report = restore(s.world_mut(), &snap, &reg).expect("same-room restore");
    assert_eq!(
        reg.hash_world(s.world()),
        at_snapshot,
        "restore did not reproduce the taken state"
    );

    let second: Vec<u64> = inputs
        .iter()
        .map(|a| {
            s.step(a.clone());
            reg.hash_world(s.world())
        })
        .collect();

    let diff = compare_hash_streams(&first, &second);
    assert!(
        diff.in_sync(),
        "`{room}`: a rewound sim replayed into a different future at tick {:?}. \
         {} component types were left STALE by the restore and {} unidentified bodies \
         survived it — one of them is the state that leaked. See netcode.md N3.1.",
        diff.first_divergence_tick,
        report.stale_components.len(),
        report.unidentified_survivors,
    );
}

/// **`restore` reproduces exactly the state it registered, and admits the rest.**
///
/// On a real room `restore` puts the registered hash back bit for bit, leaves no
/// unidentified body standing, and names every component type it could not rewind.
/// What it cannot do, it says.
///
/// A `lossless()` report here would mean the coverage ledger had reached zero. The
/// `assert!(!lossless)` is what tells you that day has come.
#[test]
fn a_restore_of_a_real_room_is_exact_where_it_is_registered_and_honest_where_it_is_not() {
    use ambition::runtime::snapshot::{restore, take, RestoreReport};

    let mut s = sim("gap_run");
    let reg = registry_of(&mut s);
    let mut policy = RandomWalkPolicy::traversal_stress(3);
    for _ in 0..40 {
        s.step(policy.act());
    }

    let snap = take(s.world(), &reg);
    let at_snapshot = reg.hash_world(s.world());
    for _ in 0..30 {
        s.step(policy.act());
    }
    assert_ne!(
        reg.hash_world(s.world()),
        at_snapshot,
        "the sim must actually have moved, or this proves nothing"
    );

    let report = restore(s.world_mut(), &snap, &reg).expect("same-room restore");
    assert_eq!(
        reg.hash_world(s.world()),
        at_snapshot,
        "restore did not reproduce the registered state it had snapshotted"
    );
    assert_eq!(
        take(s.world(), &reg),
        snap,
        "a snapshot of a restored world is not the snapshot it was restored from"
    );
    assert_eq!(
        report.unidentified_survivors, 0,
        "an unidentified body walked out of the rollback"
    );
    assert!(
        !report.stale_components.is_empty() && !report.lossless(),
        "restore is lossless on a real room — the coverage ledger has reached zero. \
         Un-ignore `a_restored_sim_replays_the_future_it_was_rewound_from`, which is \
         N3.1's real exit oracle, and delete this assertion."
    );
    // H3 / finding 6: losslessness REQUIRES resource coverage, and the report MEASURES it
    // itself now (restore fills `unregistered_sim_resources` + `resource_census_reliable`),
    // so the caller can no longer claim `lossless(0)` against a world with debt. Under
    // rl_sim, bevy's `debug` names are on, so the census is reliable; gap_run leaves sim
    // resources unrestored, so restore is not lossless even where ENTITY state is exact.
    assert!(
        report.resource_census_reliable,
        "the resource census was unreliable under rl_sim, where bevy_ecs/debug is on — \
         `lossless()` would refuse blind and prove nothing"
    );
    assert!(
        report.unregistered_sim_resources > 0,
        "no unregistered sim resources on gap_run — the H3 resource term is untested here"
    );
    // The resource term alone denies losslessness: a report with perfect ENTITY state but
    // this room's measured resource debt is still not lossless (the false green H3 flagged).
    let entity_exact = RestoreReport {
        unregistered_sim_resources: report.unregistered_sim_resources,
        resource_census_reliable: true,
        ..RestoreReport::default()
    };
    assert!(
        !entity_exact.lossless(),
        "an all-zero entity report is STILL not lossless while sim resources go unrestored"
    );
}

/// **A move in flight rewinds to its clock, not to its hitboxes.**
///
/// `MovePlayback` embeds a whole authored `MoveSpec` and a private
/// `live_boxes: Vec<(usize, Entity)>`. The blob carries only the CHOICE — which move,
/// how far in, did it land — and `SnapshotResolve` rebuilds the spec out of the
/// owner's surviving `ActorMoveset`. The box cache comes back empty, because a blob
/// cannot carry an `Entity` (N3.1 decision 2), and it does not have to: a strike
/// volume's existence is DERIVED from `(t, window)`, and
/// `retire_orphaned_strike_volumes` maintains that derivation every frame.
///
/// This test lives in `ambition_app` rather than beside the codec because
/// `ambition_runtime` may not name `ambition_entity_catalog` — F1.9's headless-tier
/// boundary, which caught the dev-dependency that would have quietly widened it.
#[test]
fn a_move_in_flight_rewinds_to_its_clock_and_not_to_its_hitboxes() {
    use ambition::combat::moveset::{ActorMoveset, MovePlayback};
    use ambition::entity_catalog::{ClipBinding, MoveSpec, MovesetContract};
    use ambition::platformer::sim_id::SimId;
    use ambition::runtime::snapshot::{restore, take};

    let spec = MoveSpec {
        id: "smash".into(),
        clip: ClipBinding {
            clip: "attack".into(),
            fallbacks: Vec::new(),
        },
        duration_s: 1.0,
        windows: Vec::new(),
        events: Vec::new(),
        gates: Default::default(),
        start_impulse: None,
        smash_charge_mult: 1.0,
    };

    let mut s = sim("gap_run");
    let reg = registry_of(&mut s);
    for _ in 0..10 {
        s.step(RandomWalkPolicy::traversal_stress(3).act());
    }

    let player = {
        let mut q = s
            .world_mut()
            .query_filtered::<ambition::bevy::prelude::Entity, ambition::bevy::prelude::With<SimId>>();
        let w = s.world();
        q.iter(w).next().expect("a simulated body")
    };
    s.world_mut().entity_mut(player).insert((
        ActorMoveset(MovesetContract {
            verbs: Default::default(),
            moves: vec![spec.clone()],
        }),
        MovePlayback::resumed(spec, 1.0, 0.25, true),
    ));

    let snap = take(s.world(), &reg);
    let before = reg.hash_world(s.world());

    // The move ends.
    s.world_mut().entity_mut(player).remove::<MovePlayback>();
    assert_ne!(reg.hash_world(s.world()), before, "removal must be visible");

    restore(s.world_mut(), &snap, &reg).expect("same-room restore");
    let pb = s
        .world()
        .entity(player)
        .get::<MovePlayback>()
        .expect("the move came back");
    assert_eq!(pb.spec.id, "smash", "the spec resolved out of the moveset");
    assert_eq!(pb.t, 0.25, "the clock rewound");
    assert!(pb.landed_hit, "the combo-confirm fact rewound");
    assert_eq!(reg.hash_world(s.world()), before);
}

/// **`ambition_content` registers the sim state it owns, and this test is why.**
///
/// netcode.md N3.1: *"each sim crate registers its components' serialization."*
/// `SnapshotRegistry` is a resource precisely so a crate `ambition_runtime` cannot name
/// can add to it. For one commit this worked by accident and then silently stopped: the
/// content plugin builds BEFORE `SnapshotRegistryPlugin`, so its
/// `if let Some(registry) = get_resource_mut(..)` found nothing and registered nothing.
/// Every test stayed green. The ledger simply reported a debt it had stopped measuring.
///
/// Both plugins `init_resource` now, so registration is additive and order-independent.
/// This test is the thing that would have caught the silence.
#[test]
fn the_content_crate_registers_its_own_boss_special_state() {
    let mut s = sim("mockingbird_arena");
    let reg = registry_of(&mut s);
    let names: Vec<&str> = reg.names().collect();

    for owned_by_content in [
        "echo_fan_state",
        "seismic_stomp_state",
        "exploding_gradient_state",
        "overflow_state",
        "gradient_cascade_state",
        "minima_trap_state",
        "apple_rain_spawn_state",
        "mode_collapse_state",
        "eye_beam_state",
        "overfit_volley_state",
        "saddle_point_state",
    ] {
        assert!(
            names.contains(&owned_by_content),
            "`{owned_by_content}` is sim state owned by `ambition_content`, and the \
             registry has not got it. Its plugin's registration is silently not running \
             — see `BossSpecialContentPlugin::build`. Registered: {names:?}"
        );
    }

    // ...and the engine's own entries are still there: `init_resource` must not have
    // let one plugin clobber the other's registry.
    for owned_by_engine in ["sim_tick", "body_kinematics", "brain"] {
        assert!(
            names.contains(&owned_by_engine),
            "{owned_by_engine} vanished"
        );
    }
}

/// **A message written before a snapshot must not be read after a restore.**
///
/// `Messages<T>` is a `Resource`, so the coverage ledger never saw one, and it is named
/// `bevy_ecs::message::Messages<..>`, so the resource ledger's first filter missed one
/// too. It is nonetheless state: at a tick boundary `Messages<ActorActionMessage>` holds
/// the actions the brains just emitted.
///
/// `restore` clears every registered channel. This test proves the channels are
/// non-empty in the first place — a clearing test against buffers that were always empty
/// proves nothing at all.
#[test]
fn a_rewind_empties_the_message_channels_it_registered() {
    use ambition::runtime::snapshot::{restore, take};

    let mut s = sim("mockingbird_arena");
    let reg = registry_of(&mut s);
    for _ in 0..40 {
        s.step(RandomWalkPolicy::traversal_stress(7).act());
    }

    let pending = reg.pending_messages(s.world());
    assert!(
        !pending.is_empty(),
        "no registered channel holds a message at a tick boundary, so this test would \
         pass on a `restore` that did nothing. Either the sim now drains itself inside \
         the tick — in which case say so and delete this — or the channels are \
         registered wrong."
    );

    let snap = take(s.world(), &reg);
    let report = restore(s.world_mut(), &snap, &reg).expect("same-room restore");
    // Every registered channel: actor_action, hit_event, on_hit_effect,
    // move_event + the E8 encounter ingress pair (command, event) + the
    // room-construction staging fact (room_loaded, N3.2b) + spawn_actor_request +
    // room_transition_requested + the runtime brain-switch authority
    // (brain_command) + the "you are free" release (release_provocation).
    assert_eq!(report.messages_cleared, 11);
    assert!(
        reg.pending_messages(s.world()).is_empty(),
        "a message from the abandoned future survived the rewind: {:?}",
        reg.pending_messages(s.world())
    );
}

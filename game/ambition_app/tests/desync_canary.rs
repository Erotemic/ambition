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
use ambition_app::{RandomWalkPolicy, SandboxSim, SandboxSimOptions};

/// The DIRTY probes below panic on purpose. Without this, every run prints four
/// alarming backtraces for a test that passed.
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
        let spec = sim
            .world()
            .get_resource::<ambition::world::rooms::RoomSet>()
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
    const KNOWN_RESOURCE_DEBT: usize = 181;
    assert!(
        resources.len() <= KNOWN_RESOURCE_DEBT,
        "{} unregistered `ambition_*` resources, up from the pinned \
         {KNOWN_RESOURCE_DEBT}. A resource sits on no entity, so `unclaimed_components` \
         never saw one and `restore` never touched one. See netcode.md N3.1.",
        resources.len()
    );

    // Today's debt, pinned. Lower it by registering a component or by declaring it
    // structurally derived — both are claims, and `declare_derived` is the one that
    // promises a per-frame system rebuilds it.
    const KNOWN_DEBT: usize = 59;
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
/// hypothesis: it PASSES. `portal_lab`'s rewind dirtiness is **not** a leak. The
/// traversal policy bounces the player through a shared loading zone between
/// `portal_lab` and `central_hub_complex`; while `central_hub_complex` is active its
/// `NpcSpawn-0017` is legitimately alive, and it is despawned the instant the player
/// transitions away (confirmed by trace: it lives only while its room is active). What
/// makes the *rewind* dirty is separate: the 60-tick rollback window SPANS a room
/// transition, and `restore` does not yet restore the active room, so it reconstructs
/// against the wrong `RoomSpec`. That exact fix — the active room is sim state — is the
/// bounded-window/room-restore work (netcode.md N3.2). See there.
///
/// The invariant is precise: a live `placement:<iid>` that some room authors must be
/// authored by the ACTIVE room. A `placement:<iid>` that NO room authors is a
/// dynamically-spawned child (a boss's `giant_gnu_hand_left_7`, spawned by the boss,
/// not the room) — that is an identity-vocabulary concern (its id should be
/// `spawned(..)`; netcode.md N3.2 dynamic-spawn), not a cross-room leak, so it is
/// exempt here. `slot:` (the player) is carried/persistent and exempt.
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
        // `respawn_authored_entity` reconstructs from.
        let authored_by: BTreeMap<String, BTreeSet<String>> = {
            let rs = s
                .world()
                .get_resource::<ambition::world::rooms::RoomSet>()
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
        // room authors is a dynamically-spawned child promoted into the placement
        // namespace because `ensure_sim_id` gave it a `FeatureId`. Its name is derived
        // from an entity index, so it is not a deterministic authored identity — it is
        // dynamic-spawn identity debt (netcode.md N3.2 dynamic-spawn / S2.9), not a
        // room leak. Collected here so the debt is visible, not silently skipped.
        let mut dynamic_debt: BTreeSet<String> = BTreeSet::new();

        let mut policy = RandomWalkPolicy::traversal_stress(3);
        for tick in 0..120u32 {
            s.step(policy.act());

            let active_id = {
                let rs = s
                    .world()
                    .get_resource::<ambition::world::rooms::RoomSet>()
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
                "  [{room}] dynamic-spawn identity debt (placement: ids no room authors, \
                 should be SimId::spawned(..) — netcode.md N3.2): {dynamic_debt:?}"
            );
        }
    }
}

/// The active room's `RoomSpec` id.
fn active_room(s: &SandboxSim) -> String {
    s.world()
        .get_resource::<ambition::world::rooms::RoomSet>()
        .map(|rs| rs.active_spec().id.clone())
        .unwrap_or_default()
}

/// **A rollback window may not span a room transition** (audit item 2 / reviewer).
///
/// The active room is not restored sim state, so a snapshot taken while one room was
/// active cannot be reconciled against another — `respawn_from_the_room` would rebuild
/// the snapshot's entities against the wrong `RoomSpec`. `restore` captures the active
/// room in the snapshot and REFUSES the mismatch (`CrossRoomBoundary`) before touching a
/// single entity, rather than produce a world more inconsistent than a clean refusal (a
/// transition also rebuilds room-scoped entities, platforms, and clocks). This is the
/// honest rollback boundary; the full atomic room restore is the bounded-window work.
///
/// `portal_lab`'s traversal bounces the player through a shared loading zone with
/// `central_hub_complex`, which is exactly how a window comes to span a transition.
#[test]
fn restore_refuses_a_snapshot_that_spans_a_room_transition() {
    use ambition::runtime::snapshot::{restore, take, RestoreError};

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

    match restore(s.world_mut(), &snap, &reg) {
        Err(RestoreError::CrossRoomBoundary {
            snapshot_room,
            active_room,
        }) => {
            assert_eq!(snapshot_room, snap_room);
            assert_eq!(active_room, "portal_lab");
            assert_ne!(snapshot_room, active_room);
        }
        other => panic!(
            "restore did not reject a cross-room snapshot (taken in `{snap_room}`, world \
             now in `portal_lab`) with CrossRoomBoundary — got {other:?}. It must refuse \
             rather than rebuild entities against the wrong RoomSpec (audit item 2)."
        ),
    }
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
/// **`gap_run` is CLEAN**: a plain platformer room rewinds and replays bit for bit.
/// The other three do not, and the ledger says so rather than skipping them. Each
/// carries state the registry has not reached yet — portals carry transit latches,
/// the arenas carry brains and move playbacks (netcode.md N3.1's checklist).
///
/// The dirty list is asserted to be dirty. Fix a room and this test fails, telling
/// you to promote it. A ledger you can only ever satisfy by lowering it is not a
/// ledger.
#[test]
fn a_restored_sim_replays_the_future_it_was_rewound_from() {
    /// Rooms where a rewind is exact. This list may grow. It may not shrink.
    ///
    /// `gnu_ton_arena` joined it the day `GameplayElapsed` — an accumulating sim clock
    /// a brain stamps its memories with — was registered. `mockingbird_arena` joined it
    /// the day `BossEncounter.encounter` did: rewinding only the exposed
    /// `encounter_phase` mirror is rewinding a thermometer. **Two boss fights rewind and
    /// replay bit for bit.**
    const CLEAN: &[&str] = &["gap_run", "gnu_ton_arena", "mockingbird_arena"];
    /// Rooms whose rewind is not yet exact. `portal_lab` is DIRTY for a precise,
    /// confirmed reason (audit item 2, traced in
    /// `every_placement_entity_is_owned_by_the_active_room_every_tick`): its 60-tick
    /// window SPANS a room transition. The snapshot is taken while `central_hub_complex`
    /// is active — which authors `NpcSpawn-0017` — but the replay ends with `portal_lab`
    /// active, and `restore` does not yet restore the active room, so it reconstructs
    /// `NpcSpawn-0017` against the wrong `RoomSpec` (`respawned = 1`). It is NOT a
    /// cross-room leak. It joins CLEAN when the active room becomes restored sim state
    /// (netcode.md N3.2 room-restore / bounded window).
    const DIRTY: &[&str] = &["portal_lab"];

    for room in CLEAN {
        replay_after_rewind(room);
    }

    for room in DIRTY {
        let clean = quietly(|| replay_after_rewind(room)).is_ok();
        assert!(
            !clean,
            "`{room}` now rewinds exactly. Move it from DIRTY to CLEAN — and if that \
             empties DIRTY, N3.1 is done: delete the honesty assertion in \
             `a_restore_of_a_real_room_is_exact_where_it_is_registered_and_honest_where_it_is_not`."
        );
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
    use ambition::runtime::snapshot::{restore, take};

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
    assert_eq!(report.messages_cleared, 4);
    assert!(
        reg.pending_messages(s.world()).is_empty(),
        "a message from the abandoned future survived the rewind: {:?}",
        reg.pending_messages(s.world())
    );
}

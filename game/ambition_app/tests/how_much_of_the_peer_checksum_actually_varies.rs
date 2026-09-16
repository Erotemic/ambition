//! ID-PEER — of the entries that FEED the peer checksum, how many actually move?
//!
//! ⛔⛤ **THIS EXISTS BECAUSE ONE HASHED ENTRY WAS FOUND FROZEN AND NOBODY HAD
//! ASKED THE QUESTION OF THE OTHERS.** `AmbitionGameSave` is registered
//! `resource-clone-custom-checksum` — inside the session checksum, projected
//! through a function that serialises the whole save — and measured 2026-09-16 it
//! takes **exactly one value** across every frame GGRS saved twice, while the
//! live save reaches 247 mirrored items
//! (`which_hashed_entry_moves_when_the_bag_does.rs`). A hashed entry contributing
//! a constant is in the peer contract by registration and out of it in effect.
//!
//! ⇒ So the general question: **how much of what two peers would compare
//! actually varies?** The registry knows which entries feed the checksum
//! (`RollbackEntryKind::feeds_peer_checksum`) and `RollbackRestoreAudit` knows
//! which types' censuses moved at the frames it compared. The join is the answer.
//!
//! ⚠ **A CONSTANT ENTRY IS NOT A DEFECT, AND READING THIS AS A DEFECT LIST IS THE
//! TRAP.** Most rollback state is legitimately still in a 240-step run of one
//! room: authored configuration, a seat table nobody re-seats, a match receipt
//! minted once. The finding `AmbitionGameSave` represents is narrower — an entry
//! whose LIVE value moves while its hashed projection does not — and separating
//! those two requires reading the live value too, which this file does not do for
//! 300 types. What this measures is the SIZE of the varying part, which nobody
//! had.
//!
//! ⚠ **AND THE JOIN IS ON `std::any::type_name`, EXACTLY.** Both sides build it
//! the same way — `descriptor_owned` stores `std::any::type_name::<T>()` and
//! `ChecksumProbe` is constructed with it — so a zero overlap would be a broken
//! join, not a clean world, and the probe asserts against that.

#![cfg(feature = "rl_sim")]

use ambition_app::rl_sim::{
    AgentAction, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode,
};

type OwnedItems = ambition_platformer2d::item::OwnedItems;
type Item = ambition_platformer2d::item::Item;

const ROOM: &str = "combat_calibration_lab";

/// ⚠ FROM TICK 4. A per-tick grant starting at tick 1 kills the session at step
/// 6, and everything after that is a frozen world agreeing with itself — which
/// would make every entry here read as constant for a reason that has nothing to
/// do with the checksum. Measured by CalculexAmbition: gated to tick 4 the same
/// grant runs 240 steps clean.
fn grant_each_tick_from_four(
    tick: bevy::prelude::Res<ambition_platformer2d::time::SimTick>,
    mut owned: bevy::prelude::ResMut<OwnedItems>,
) {
    if tick.0 >= 4 {
        owned.grant(Item::HealthCell, 1);
    }
}

/// ⛔⛤ **THE FIRST VERSION OF THIS FILE STEPPED WITH `AgentAction::default()` AND
/// WOULD HAVE PUBLISHED 131 OF 144 AS CONSTANT.** No input at all: the player
/// stands still, nothing attacks, nothing takes damage. Most rollback state being
/// unchanged in that run is not a finding about the checksum, it is a finding
/// about an idle world — and the number looked alarming enough to write down.
///
/// ⇒ So the measurement is a PAIR, and the difference between them is the part
/// that means anything: entries that move only once somebody plays are working,
/// and entries constant under BOTH are where a frozen projection could hide.
fn idle() -> AgentAction {
    AgentAction::default()
}

/// Move, jump and press attack on an edge — the cadence measured to actually
/// arm state in this composition (`does_a_presence_probed_row_move_when_its_value_does.rs`
/// found that holding attack moves nothing and only a LANDING moves
/// `BodyAnimFacts`).
fn playing() -> AgentAction {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static N: AtomicUsize = AtomicUsize::new(0);
    let n = N.fetch_add(1, Ordering::SeqCst);
    AgentAction {
        move_x: if (n / 30) % 2 == 0 { 1.0 } else { -1.0 },
        right_pressed: (n / 30) % 2 == 0,
        left_pressed: (n / 30) % 2 == 1,
        jump: n % 8 == 0,
        jump_held: n % 8 < 3,
        attack: n % 12 == 0,
        ..AgentAction::default()
    }
}

fn run_with(action: fn() -> AgentAction) -> Platformer2dSimHarness {
    use ambition_platformer2d::sim::SimScheduleExt;
    let mut sim = Platformer2dSimHarness::build(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_required_start_room(ROOM)
            .with_sync_test_rollback_settings(4, 10),
        |app, options| {
            ambition_app::rl_sim::ambition_sim_composition(app, options)?;
            let label = app.sim_schedule();
            app.add_systems(label, grant_each_tick_from_four);
            Ok(())
        },
    )
    .expect("the sync-test harness builds with a gated grant inside the tick");
    sim.world_mut()
        .insert_resource(ambition_platformer2d::rollback::RollbackRestoreAudit::enabled());
    for _ in 0..240 {
        sim.step(action());
    }
    sim
}

/// `(hashed type names, hashed names that also carry a probe)`.
fn hashed_types(sim: &Platformer2dSimHarness) -> (Vec<String>, Vec<String>) {
    let probed: std::collections::BTreeSet<&str> = sim
        .world()
        .resource::<ambition_platformer2d::rollback::RollbackChecksumProbes>()
        .type_names()
        .into_iter()
        .collect();
    let registry = sim
        .world()
        .resource::<ambition_platformer2d::rollback::RollbackRegistry>();
    let mut hashed: Vec<String> = registry
        .descriptors()
        .filter(|d| d.kind.feeds_peer_checksum())
        .map(|d| d.type_name.clone())
        .collect();
    hashed.sort();
    hashed.dedup();
    let with_probe = hashed
        .iter()
        .filter(|name| probed.contains(name.as_str()))
        .cloned()
        .collect();
    (hashed, with_probe)
}

#[test]
#[ignore = "PROBE, print-only: how much of the peer checksum actually varies"]
fn probe_how_much_of_the_peer_checksum_actually_varies() {
    let mut constant_in: Vec<std::collections::BTreeSet<String>> = Vec::new();
    for (label, action) in [
        ("IDLE — no input at all", idle as fn() -> AgentAction),
        ("PLAYING — run, jump, attack on an edge", playing as fn() -> AgentAction),
    ] {
        let sim = run_with(action);
        let (hashed, with_probe) = hashed_types(&sim);
        let audit = sim
            .world()
            .resource::<ambition_platformer2d::rollback::RollbackRestoreAudit>();
        let moved: std::collections::BTreeMap<&str, usize> = audit
            .types_whose_census_moved_across_compared_frames()
            .into_iter()
            .collect();
        println!("\n── {label}");
        println!("   {}", audit.coverage());
        println!("   hashed entries (kind feeds the peer checksum): {}", hashed.len());
        println!("   of those, carrying a probe this audit censused: {}", with_probe.len());
        let (mut moving, mut still) = (Vec::new(), std::collections::BTreeSet::new());
        for name in &with_probe {
            match moved.get(name.as_str()) {
                Some(values) => moving.push((name.clone(), *values)),
                None => {
                    still.insert(name.clone());
                }
            }
        }
        moving.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        println!("   MOVED across the compared frames: {}", moving.len());
        for (name, values) in &moving {
            println!("       {values:>4} distinct  {name}");
        }
        println!("   CONSTANT across every compared frame: {}", still.len());
        constant_in.push(still);
    }

    // ⛔⛤ THE DIFFERENCE IS THE MEASUREMENT. An entry constant while idle and
    // moving under play is working exactly as it should; an entry constant under
    // BOTH is where a frozen projection can hide, and that is the population the
    // `AmbitionGameSave` finding came from.
    let woke = constant_in[0].difference(&constant_in[1]).count();
    let never: Vec<&String> = constant_in[0].intersection(&constant_in[1]).collect();
    println!("\n⇒ {woke} hashed entr(ies) were constant while IDLE and MOVED under PLAY.");
    println!("⇒ {} were constant under BOTH.", never.len());
    for name in &never {
        println!("     {name}");
    }
    println!(
        "\n⚠ CONSTANT UNDER BOTH IS STILL NOT A DEFECT LIST. A component nobody \
         spawns in this room, a resource only a boss encounter writes, and a \
         projection frozen like `AmbitionGameSave`'s all land here. Separating \
         them needs the LIVE value read beside the census, which this file does \
         not do for 144 types — it reports the size of each bucket, which nobody \
         had."
    );
}

// ---------------------------------------------------------------------------
// S8's four `Update`-WRITTEN HASHED ENTRIES, ASKED THE ONLY QUESTION THAT
// SEPARATES "QUIET" FROM "FROZEN".
//
// ⛔⛤ S8 measured `CustodyBaseline` and `OccurrenceBaseline` "clean" and STATED
// its own limit: the audit reports which entries DIVERGED, not whether the
// system that writes them ran at all. The paired census above closes half of
// that — all three of `CustodyBaseline`, `OccurrenceBaseline` and
// `NewGameResetRequested` are CONSTANT under both idle and play, so their clean
// verdict had nothing to disagree about.
//
// ⚠ AND CONSTANT IS NOT FROZEN. A baseline nobody re-checkpoints in this room is
// legitimately quiet, and that is indistinguishable from a snapshot that stopped
// tracking — which is exactly the trap `AmbitionGameSave` sprang. The
// discriminator is the LIVE value: read it at the start and at the end of the
// same run. Live MOVED + census constant is the save's defect; live constant is a
// quiet subject and says nothing.

type CustodyBaseline = ambition_platformer2d::platformer::lifecycle::CustodyBaseline;
type OccurrenceBaseline = ambition_platformer2d::platformer::lifecycle::OccurrenceBaseline;

#[test]
#[ignore = "PROBE, print-only: are S8's Update-written baselines quiet or frozen?"]
fn probe_whether_s8s_baselines_are_quiet_or_frozen() {
    let mut sim = {
        use ambition_platformer2d::sim::SimScheduleExt;
        let mut sim = Platformer2dSimHarness::build(
            Platformer2dSimHarnessOptions::default()
                .with_timestep(TimestepMode::fixed_60hz())
                .with_required_start_room(ROOM)
                .with_sync_test_rollback_settings(4, 10),
            |app, options| {
                ambition_app::rl_sim::ambition_sim_composition(app, options)?;
                let label = app.sim_schedule();
                app.add_systems(label, grant_each_tick_from_four);
                Ok(())
            },
        )
        .expect("the sync-test harness builds");
        sim.world_mut()
            .insert_resource(ambition_platformer2d::rollback::RollbackRestoreAudit::enabled());
        sim
    };
    // ⛔⛤ THE POPULATION, NOT ONLY THE DIGEST. Two structurally different types
    // whose checksums are EQUAL is the signature of both being empty — and "the
    // baseline never moved" over an empty baseline is S8's stated limit, not a
    // clean bill of health: the audit reports what DIVERGED, never whether the
    // capture ran at all. So the row count is printed beside the digest.
    let rows_before = (
        sim.world().resource::<CustodyBaseline>().rows().count(),
        sim.world()
            .resource::<OccurrenceBaseline>()
            .remembered()
            .rows()
            .count(),
    );
    let before = (
        CustodyBaseline::checksum(sim.world().resource::<CustodyBaseline>()),
        OccurrenceBaseline::checksum(sim.world().resource::<OccurrenceBaseline>()),
    );
    for _ in 0..240 {
        sim.step(playing());
    }
    let after = (
        CustodyBaseline::checksum(sim.world().resource::<CustodyBaseline>()),
        OccurrenceBaseline::checksum(sim.world().resource::<OccurrenceBaseline>()),
    );
    let rows_after = (
        sim.world().resource::<CustodyBaseline>().rows().count(),
        sim.world()
            .resource::<OccurrenceBaseline>()
            .remembered()
            .rows()
            .count(),
    );
    println!(
        "   CustodyBaseline rows {} -> {} ; OccurrenceBaseline rows {} -> {}",
        rows_before.0, rows_after.0, rows_before.1, rows_after.1
    );
    if rows_after.0 == 0 && rows_after.1 == 0 {
        println!(
            "   ⛔ BOTH BASELINES ARE EMPTY AT THE END OF THE RUN. Every verdict \
             below — S8's \"measured clean\" and this probe's \"quiet\" — is about a \
             subject that was never captured. That is S8's own stated limit, now \
             measured rather than suspected."
        );
    }
    let audit = sim
        .world()
        .resource::<ambition_platformer2d::rollback::RollbackRestoreAudit>();
    println!("{}", audit.coverage());
    for (name, live_before, live_after, censuses) in [
        (
            "CustodyBaseline",
            before.0,
            after.0,
            audit.distinct_censuses_across_compared_frames_of::<CustodyBaseline>(),
        ),
        (
            "OccurrenceBaseline",
            before.1,
            after.1,
            audit.distinct_censuses_across_compared_frames_of::<OccurrenceBaseline>(),
        ),
    ] {
        let verdict = match (live_before != live_after, censuses > 1) {
            (true, false) => "⛔ FROZEN — the live value moved and the census did not",
            (true, true) => "✔ tracking",
            (false, _) => "ⓘ QUIET — the live value never moved, so this says nothing",
        };
        println!(
            "   {name:<20} live {live_before:#018x} -> {live_after:#018x}  \
             censuses across compared frames: {censuses}   {verdict}"
        );
    }
}

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

//! ⛔⛔ **THIS FILE'S ARMS ASSUME THEY ARE THE ONLY SIM APP RUNNING IN THIS
//! PROCESS, AND THAT IS A PROPERTY OF THE TEST BINARY'S CONTENTS RATHER THAN OF
//! THE TREE. MEASURED 2026-09-16.**
//!
//! Adding a SECOND sim App to this file makes
//! `no_registered_type_is_written_outside_the_rewinding_schedule` report **99**
//! types written outside the rewinding schedule instead of none — under default
//! parallelism only, never alone and never under `--test-threads=1`. The 99 is a
//! corrupted comparison baseline, not a finding, and a reader who did not know
//! that would read it as a save bug.
//!
//! ⇒ **SO DO NOT ADD A SECOND SIM-APP FIXTURE HERE WITHOUT `#[ignore]`.**
//! `run_with_a_writer_outside_the_schedule` is the one that exists and both arms
//! using it are ignored for exactly this reason. It is not a rule about taste:
//! the arms in this file measure a per-frame comparison between the live world
//! and its own most recent snapshot, and that comparison is what goes wrong.
//!
//! ⚠ Not leaked state — `probe_whether_a_second_sim_app_leaves_state_behind`
//! runs A, B, A sequentially and the third reading is identical to the first, so
//! order is innocent and the failure needs SIMULTANEITY. Not the wall-clock
//! timestep either (`013b70c89`'s mechanism): this harness pins the clock
//! whenever rollback is enabled. See
//! `docs/planning/triage/a-composition-acceptance-that-only-fails-in-company.md`,
//! which this instance advanced from intermittent-and-unexplained to
//! deterministic-with-two-mechanisms-eliminated.

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

/// The same composition with the grant moved OUT of the rewinding schedule —
/// the synthetic subject the detector is asserted against.
///
/// ⭐⭐ **THE POISON IS THE FIXTURE, WHICH IS WHY IT CANNOT DIE OF SUCCESS.**
/// `no_registered_type_is_written_outside_the_rewinding_schedule` used
/// `AmbitionGameSave` as its positive control because production was writing it
/// from `Update`; ROLLBACK-BAG-DESYNC's P0 repair moved it into the schedule and
/// the control went with it. A control that is a live defect has a lifetime
/// bounded by the defect. This one reproduces the defect DELIBERATELY in a
/// fixture nothing else uses, so the detector keeps a subject no repair can take
/// away.
///
/// ⚠ THE SUBJECT IS THE SAME SYSTEM, ONE SCHEDULE OVER, and that is the point:
/// `run_with` adds `grant_each_tick_from_four` to `app.sim_schedule()` and this
/// adds it to `Update`. The only difference between the two runs is WHERE the
/// write happens, so a detector that reports the same answer for both is
/// reporting on something else.
fn run_with_a_writer_outside_the_schedule() -> Platformer2dSimHarness {
    let mut sim = Platformer2dSimHarness::build(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_required_start_room(ROOM)
            .with_sync_test_rollback_settings(4, 10),
        |app, options| {
            ambition_app::rl_sim::ambition_sim_composition(app, options)?;
            app.add_systems(bevy::prelude::Update, grant_each_tick_from_four);
            Ok(())
        },
    )
    .expect("the sync-test harness builds with the grant outside the tick");
    sim.world_mut()
        .insert_resource(ambition_platformer2d::rollback::RollbackRestoreAudit::enabled());
    for _ in 0..240 {
        sim.step(playing());
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

// ---------------------------------------------------------------------------
// S8's POPULATION, MEASURED RATHER THAN READ OFF EIGHT SYSTEMS' SCHEDULES.
//
// ⛔⛤ S8 found its four by reading `add_systems` calls and registrations —
// careful work that a forwarder, a set, or a `cfg` can hide from. The GGRS
// advance runs in `PreUpdate` (`run_ggrs_schedules`), so at the END of a frame
// the live world is the last saved frame PLUS whatever ran outside the rewinding
// schedule. `record_live_census` compares the two, and what it names is that
// population directly.
//
// ⚠ IT NAMES A POPULATION, NOT A DEFECT. Presentation state is legitimately
// written there. What makes an entry dangerous is that it ALSO feeds the peer
// checksum — the registry knows that and the audit does not, so the JOIN is the
// finding and it is done here.

#[test]
#[ignore = "PROBE, print-only: which HASHED entries are written outside the rewinding schedule"]
fn probe_which_hashed_entries_are_written_outside_the_rewinding_schedule() {
    let sim = run_with(playing);
    let (hashed, _) = hashed_types(&sim);
    let hashed: std::collections::BTreeSet<String> = hashed.into_iter().collect();
    let audit = sim
        .world()
        .resource::<ambition_platformer2d::rollback::RollbackRestoreAudit>();
    println!("{}", audit.coverage());
    // ⛔ THE FLOOR. An empty set with zero comparisons reads exactly like a world
    // where nothing is written outside the schedule.
    println!("   live comparisons: {}", audit.live_comparisons);
    let outside = audit.written_outside_the_rewinding_schedule();
    println!("   types written OUTSIDE the rewinding schedule: {}", outside.len());
    let (mut hashed_too, mut not_hashed) = (Vec::new(), Vec::new());
    for name in &outside {
        if hashed.contains(*name) {
            hashed_too.push(*name);
        } else {
            not_hashed.push(*name);
        }
    }
    println!("   ⛔ OF THOSE, FEEDING THE PEER CHECKSUM: {}", hashed_too.len());
    for name in &hashed_too {
        println!("       {name}");
    }
    println!("   ⓘ not hashed (presentation and local state live here legitimately): {}", not_hashed.len());
    for name in not_hashed.iter().take(20) {
        println!("       {name}");
    }
    println!(
        "\n⚠ THIS CANNOT SEE A WRITE THAT PUTS THE VALUE BACK inside one frame, \
         which is exactly why `NewGameResetRequested` satisfies S8's first two \
         conditions and does not desync. Same blind spot as the checksum's."
    );
}

/// ⛔⛤ **S8'S POPULATION, ASSERTED: EXACTLY ONE REGISTERED TYPE IS WRITTEN
/// OUTSIDE THE REWINDING SCHEDULE, AND IT IS THE SAVE.**
///
/// S8 found four `Update`-written hashed entries by reading `add_systems` calls.
/// Measured over 240 frames by comparing the world at the end of the GGRS advance
/// with the world at the end of the frame, the answer is **one**:
/// `AmbitionGameSave`. The other three are explained by this instrument's two
/// stated blind spots rather than by disagreement — `NewGameResetRequested` is
/// put back within the frame, and `CustodyBaseline` / `OccurrenceBaseline` are
/// measured EMPTY for the whole run (`probe_whether_s8s_baselines_are_quiet_or_frozen`).
///
/// ⚠ **THE POPULATION IS "TYPES WHOSE PROBE CAN SEE A VALUE CHANGE", NOT "ALL
/// STATE".** A presence probe counts carriers and is blind to a value, and a type
/// that is not rollback-registered at all cannot appear here however it is
/// written — `SeatControlFrameModes` and `PlayerDamagePolicy` are both written
/// from `Update`, read by sim systems, and invisible to this arm because neither
/// is registered. That is `SETTINGS-ROLLBACK`'s row, not a hole in this one.
///
/// ⛔⛤ **THE POSITIVE CONTROL DIED OF SUCCESS ON 2026-09-16 AND THIS ARM IS
/// WEAKER FOR IT — SAID OUT LOUD BECAUSE A GUARD CHANGED BY THE WORK IT WAS
/// WATCHING IS THE ONE TO DISTRUST.** The control was `AmbitionGameSave` MUST
/// appear, established independently by reading its registration and by the
/// pinned-snapshot measurement. It does not appear any more: the three
/// live→save mirrors moved into the sim schedule (ROLLBACK-BAG-DESYNC's P0
/// repair), so the set this arm measures is now genuinely EMPTY and the arm's
/// own control was the defect.
///
/// ⛔ **SO AN EMPTY SET NO LONGER DISTINGUISHES A CLEAN WORLD FROM A BLIND
/// INSTRUMENT, and the floors below do not fully close that.** They prove the
/// audit ran and that the world was moving; they do NOT prove the
/// outside-the-schedule DETECTOR still has power, because nothing in the tree
/// exercises it any more.
/// ⇒ **OWED: a SYNTHETIC positive control** — compose a system that writes a
/// rollback-registered hashed resource from `Update` and assert this arm names
/// it. That is a poison-as-fixture rather than a live defect, so it cannot die
/// of success the way this one did. Until then this arm can report clean over a
/// broken detector, and that is the failure mode to suspect first if it is ever
/// the only thing standing between a regression and a green lane.
/// (YardratAmbition's arm and their framing: a control pinned to a live defect
/// dies when the defect is fixed.)
#[test]
fn no_registered_type_is_written_outside_the_rewinding_schedule() {
    let sim = run_with(playing);
    let (hashed, _) = hashed_types(&sim);
    let hashed: std::collections::BTreeSet<String> = hashed.into_iter().collect();
    let audit = sim
        .world()
        .resource::<ambition_platformer2d::rollback::RollbackRestoreAudit>();
    assert!(
        audit.live_comparisons > 0,
        "the live census never ran or never had a post-advance baseline to \
         compare against, so the empty set below would be a reading about the \
         instrument ({})",
        audit.coverage()
    );
    // ⛔ THE WORLD MUST HAVE BEEN MOVING. This does not replace the positive
    // control the repair removed — see this arm's doc — but it does refuse the
    // cheapest way for the set below to be empty: a run in which nothing
    // happened at all.
    let moved = audit.types_whose_census_moved_across_compared_frames();
    assert!(
        moved.len() >= 5,
        "only {} type(s) moved across the compared frames, so this was a \
         near-static world and an empty set below says nothing about where \
         writes happen",
        moved.len()
    );
    let outside: Vec<&str> = audit.written_outside_the_rewinding_schedule();
    assert_eq!(
        outside,
        Vec::<&str>::new(),
        "a registered type other than the save is being written outside the \
         rewinding schedule. ⇒ THAT IS THE GOOD FAILURE IF IT IS NEW WORK and the \
         bad one if it is a regression: check whether the new entry also feeds \
         the peer checksum, because that is what turns this population into a \
         desync candidate. Of the {} entries that feed it, the save was the only \
         member of this set on 2026-09-16.",
        hashed.len()
    );
}


/// ⛔⛤ **THE SYNTHETIC POSITIVE CONTROL THIS FILE OWES CANNOT BE BUILT FROM A
/// CLONE-REGISTERED RESOURCE, AND THIS ARM IS THE MEASUREMENT THAT SAYS SO.**
///
/// `no_registered_type_is_written_outside_the_rewinding_schedule` asserts a set
/// is EMPTY, and an empty set is what a clean world reports AND what a blind
/// detector reports. Its separator used to be `AmbitionGameSave` — a LIVE
/// DEFECT, which ROLLBACK-BAG-DESYNC's P0 repair removed. Its doc names a
/// synthetic subject as the durable replacement: write a rollback-registered
/// resource from `Update` in a fixture and assert the detector names it.
///
/// ⛔ **IT DOES NOT.** This fixture adds the same grant system to `Update`
/// instead of `app.sim_schedule()`, so `OwnedItems` — registered, inside the
/// peer checksum — is written outside the rewinding schedule on every tick from
/// the fourth. MEASURED: 240 live comparisons, and
/// `written_outside_the_rewinding_schedule()` returns `[]`.
///
/// ⇒ **THE CAUSE IS THE PROBE'S STRENGTH, NOT THE DETECTOR'S PLACEMENT.**
/// `record_live_census` runs in `Last`, after `Update`, so the write is
/// certainly in the world when the comparison happens. But `OwnedItems` is
/// `rollback_resource_clone`, which gets a PRESENCE probe — and a presence
/// census of a resource is `(count: 1, xor: 0)` whatever the value is. The
/// comparison is between two identical censuses of a value that changed.
///
/// ⛔⛤ **SO THE ARM ABOVE HAS A NARROWER POPULATION THAN ITS OWN DOC CLAIMS.**
/// It says the population is *"types whose probe can see a value change"*, which
/// is right — but the consequence was not drawn: **every clone-registered
/// RESOURCE is outside it**, and `AmbitionGameSave` was only ever visible
/// because it is `rollback_resource_clone_checksum`, which gets a value probe.
/// An outside-the-schedule write to any of the clone-registered resources would
/// be reported as a clean world today.
///
/// ⚠ AND `strengthen_with` CANNOT CLOSE IT: its bound is `T: Component`, so a
/// resource's probe cannot be upgraded at runtime the way `GroundItem`'s is in
/// `does_a_presence_probed_row_move_when_its_value_does`. ⇒ The road is to give
/// `strengthen_with` a resource half, or to use a value-probed subject — and the
/// two value-probed resources in reach (`AmbitionGameSave`, `LastQuestRoom`) are
/// in `ambition_persistence`, which is not a dependency of this crate and must
/// not become one for a test fixture.
///
/// ⇒ **THIS ARM IS THE TRIPWIRE FOR THAT WORK.** It asserts the limitation, so
/// the day a resource probe can be strengthened it goes RED and whoever did it
/// is handed the control that has been owed since 2026-09-16. A recorded
/// limitation with a failing test attached is the difference between a known gap
/// and a forgotten one.
///
/// ⛔⛤ **`#[ignore]`, AND THE REASON IS A THIRD INSTANCE OF A CLASS THIS
/// REPOSITORY ALREADY HAS OPEN.** Building a SECOND sim App in this process
/// makes the arm above fail — `no_registered_type_is_written_outside_the_rewinding_schedule`
/// reports **99** types written outside the rewinding schedule instead of none,
/// which is the signature of a corrupted comparison baseline rather than a
/// finding. MEASURED, and it separates cleanly:
///
///     this arm alone                         1 passed
///     the arm above alone                    1 passed
///     both, `--test-threads=1`               2 passed
///     both, default parallelism              1 passed, 1 FAILED
///
/// ⇒ Two concurrently-built sim Apps share process state, so this is
/// `docs/planning/triage/a-composition-acceptance-that-only-fails-in-company.md`
/// — whose named next step is the `--test-threads=1` run above, and which
/// recorded the class as having at least two instances. This is the third, and
/// the first with a deterministic reproduction rather than an intermittent one.
///
/// ⚠ NOT the item catalog: `install_item_catalog` is a documented process-global
/// `OnceLock` that ALLOWS identical reinstallation, and both fixtures install the
/// same one. The shared state is elsewhere and finding it is that page's work,
/// not this arm's. ⇒ Run it with `--ignored`, or with `--test-threads=1`, and it
/// answers honestly either way.
#[test]
#[ignore = "IGNORED, not broken: it builds a second sim App, which makes the             arm above report 99 types instead of none under default             parallelism. Passes alone and under --test-threads=1; see this             arm's doc and triage/a-composition-acceptance-that-only-fails-in-company.md"]
fn the_outside_the_schedule_detector_cannot_see_a_presence_probed_resource() {
    let sim = run_with_a_writer_outside_the_schedule();
    let subject = std::any::type_name::<OwnedItems>();
    let audit = sim
        .world()
        .resource::<ambition_platformer2d::rollback::RollbackRestoreAudit>();
    // ⛔ THE PREMISE: the comparison actually happened. Without it the empty set
    // below is about the audit and the arm would pin the wrong limitation.
    assert!(
        audit.live_comparisons > 0,
        "the live census never ran, so this says nothing about probe strength \
         ({})",
        audit.coverage()
    );
    let outside: Vec<&str> = audit.written_outside_the_rewinding_schedule();
    let probes = sim
        .world()
        .resource::<ambition_platformer2d::rollback::RollbackChecksumProbes>();
    let presence_only: std::collections::BTreeSet<&str> = probes.presence_only_type_names();
    // ⛔ AND THE SECOND PREMISE, WHICH IS THE WHOLE EXPLANATION: the subject is
    // presence-probed. If it ever becomes a value probe this assertion fails
    // FIRST and names the reason, instead of the reader concluding the detector
    // is broken.
    assert!(
        presence_only.contains(&subject),
        "`{subject}` is no longer a presence-only probe, so the limitation this \
         arm records has changed shape. ⇒ Re-derive it: if its probe is now a \
         VALUE probe, the detector should see the `Update` write this fixture \
         makes, and this file finally owes the positive control \
         `no_registered_type_is_written_outside_the_rewinding_schedule` has been \
         missing. presence_only={} of {} probes",
        presence_only.len(),
        probes.type_names().len()
    );
    assert!(
        !outside.contains(&subject),
        "the detector DID name `{subject}`, which is the outcome this arm exists \
         to stop recording as impossible. ⇒ GOOD NEWS, AND ACT ON IT: delete this \
         arm and make the same fixture the positive control \
         `no_registered_type_is_written_outside_the_rewinding_schedule` owes. \
         Found: {outside:?} over {} live comparison(s).",
        audit.live_comparisons
    );
}


/// ⭐⭐ **LEAK OR CONCURRENCY? THE ONE EXPERIMENT THAT SEPARATES THEM, for the
/// deterministic in-company failure recorded in
/// `triage/a-composition-acceptance-that-only-fails-in-company.md`.**
///
/// Building a second sim App in this process makes
/// `no_registered_type_is_written_outside_the_rewinding_schedule` report 99
/// types instead of none — but only under default parallelism, never under
/// `--test-threads=1` and never alone. Two hypotheses fit that equally well:
///
///   LEAK        the second App leaves process-global state the first App's
///               measurement then reads, in which case ORDER is what matters
///   CONCURRENCY the two Apps interfere only while running AT THE SAME TIME,
///               in which case order is irrelevant and the shared thing is
///               something like Bevy's process-global task pools
///
/// ⇒ This runs A, then B, then A AGAIN, all sequentially in one thread, and
/// prints all three readings. `--test-threads=1` already shows A-then-B passing,
/// so the new information is **A after B**:
///
///   · A₂ disagrees with A₁  ⇒ LEAK, and the next step is bisecting which
///     plugin's global state B installs (compose B with fewer plugins).
///   · A₂ agrees with A₁     ⇒ NOT a leak. Order is innocent, the failure needs
///     simultaneity, and the next step is the shared runtime rather than the
///     shared data.
///
/// ⚠ **AND ONE MECHANISM IS ALREADY ELIMINATED, MEASURED rather than argued.**
/// CalculexAmbition's `013b70c89` found that `add_headless_foundation` leaves
/// `TimeUpdateStrategy::Automatic`, so fixed steps are drawn from WALL TIME and a
/// contended box runs a different number of them — offered there as a mechanism
/// to test for exactly this class. It does not apply here:
/// `Platformer2dSimHarness` pins the clock whenever rollback is enabled
/// (`set_timestep` calls `enable_manual_stepping`), and both fixtures build with
/// `with_sync_test_rollback_settings`. A load-dependent world would also be
/// intermittent, and this failure is deterministic in both directions.
///
/// ⭐⭐ **RUN 2026-09-16, AND THE ANSWER IS `NOT A LEAK`:**
///
///     A1  (production fixture, first)    live_comparisons=240 outside=0 moved=32
///     B   (writer outside the schedule)  live_comparisons=240 outside=0 moved=32
///     A2  (production fixture, AFTER B)  live_comparisons=240 outside=0 moved=32
///
/// A₂ is IDENTICAL to A₁ on every one of the three numbers. Building a second sim
/// App first changes nothing about what the production fixture measures
/// afterwards, so **order is innocent and no process-global data is carried
/// between the two Apps.** ⇒ The 99-type failure needs the two Apps running AT
/// THE SAME TIME, which moves the search from shared DATA to shared RUNTIME —
/// Bevy's process-global task pools being the first candidate, since
/// `TaskPoolPlugin` initialises them once per process and two Apps then schedule
/// their systems onto one set of worker threads.
///
/// ⚠ AND B'S `outside=0` IS THE SECOND CONFIRMATION of the tripwire above: the
/// fixture writes `OwnedItems` from `Update` on every tick from the fourth, and
/// the detector reports nothing, because a presence census of a resource is
/// `(count: 1, xor: 0)` whatever the value is. Two independent runs, same
/// reading.
///
/// Print-only: it asserts nothing, because its job is to tell the next person
/// WHICH of the two searches to run.
#[test]
#[ignore = "PROBE, print-only: runs three sim Apps sequentially to separate a \
            leak from a concurrency effect. Run with --ignored."]
fn probe_whether_a_second_sim_app_leaves_state_behind() {
    fn reading(label: &str, sim: &Platformer2dSimHarness) -> (usize, usize) {
        let audit = sim
            .world()
            .resource::<ambition_platformer2d::rollback::RollbackRestoreAudit>();
        let outside = audit.written_outside_the_rewinding_schedule();
        println!(
            "   {label}: live_comparisons={} outside={} moved={}",
            audit.live_comparisons,
            outside.len(),
            audit.types_whose_census_moved_across_compared_frames().len()
        );
        (audit.live_comparisons, outside.len())
    }

    println!("\n⭐ THREE SIM APPS, SEQUENTIALLY, IN ONE THREAD");
    let first = run_with(playing);
    let a1 = reading("A1  (production fixture, first)   ", &first);
    drop(first);

    let second = run_with_a_writer_outside_the_schedule();
    let b = reading("B   (writer outside the schedule) ", &second);
    drop(second);

    let third = run_with(playing);
    let a2 = reading("A2  (production fixture, AFTER B) ", &third);

    println!("\n⇒ VERDICT");
    if a1 == a2 {
        println!(
            "   A2 AGREES WITH A1 {a1:?}. Running B first changes nothing, so this \
             is NOT leaked state and order is innocent.\n   ⇒ The failure needs the \
             two Apps running AT THE SAME TIME. Look at what two concurrent Bevy \
             Apps share at RUNTIME — the process-global task pools are the first \
             candidate — not at what one leaves behind."
        );
    } else {
        println!(
            "   A2 {a2:?} DISAGREES WITH A1 {a1:?}. Building B changed what the \
             production fixture measures afterwards, in one thread.\n   ⇒ LEAKED \
             PROCESS STATE. Bisect it by composing B with successively fewer \
             plugins until A2 agrees with A1 again; the last plugin removed owns \
             the global. B read {b:?}."
        );
    }
}

//! ROLLBACK-BAG-DESYNC — the per-entry attribution the row says is owed.
//!
//! The row establishes that granting an item once per tick desyncs a GGRS
//! sync test within six ticks, by either road, while `OwnedItems` itself is
//! `resource-clone` and **not hashed** — so the bag cannot be the value the two
//! passes disagree about. Something hashed derives from it. Three candidates were
//! eliminated by measurement and the row's own next step is:
//!
//! > *"The registry already knows every entry that feeds the peer checksum, so
//! > the direct route is a per-entry checksum dump at the mismatching frames
//! > rather than another hypothesis — the three above were each cheap and each
//! > wrong, which is the argument for instrumenting instead of guessing a fourth
//! > time."*
//!
//! ⭐⭐ **THAT INSTRUMENT ALREADY EXISTS AND IS ALREADY RUNNING UNDER ANOTHER
//! NAME.** `RollbackRestoreAudit` + `record_saved_census` take a per-type census
//! at every save and COMPARE it when GGRS saves the same frame twice — which is
//! exactly what a resimulation is. Its own doc says so: *"the cheapest possible
//! place to answer 'the aggregate says frames 149-151 differ — differ in WHAT':
//! no second run, no bisection, just the census already being taken."* Five
//! existing arms turn it on; none of them turns it on over a world whose bag is
//! moving.
//!
//! ⇒ So this file adds no instrument. It points the existing one at the repro.

#![cfg(feature = "rl_sim")]

use ambition_app::rl_sim::{
    AgentAction, AmbitionSim, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode,
};

type OwnedItems = ambition_platformer2d::item::OwnedItems;
type Item = ambition_platformer2d::item::Item;

/// The room the other rollback arms use.
const ROOM: &str = "combat_calibration_lab";

/// ⚠ THE WINDOW IS FRAMES 0–6, NOT 240. The mismatch is reported at frames
/// `[2, 3, 4]` and the session dies at step 6; everything after that is a frozen
/// world agreeing with itself. A probe that ran 240 steps and printed the end
/// state would report on the freeze and not on the divergence.
const LIVE_STEPS: usize = 8;

fn grant_each_tick(mut owned: bevy::prelude::ResMut<OwnedItems>) {
    owned.grant(Item::HealthCell, 1);
}

/// ⛔ THE CONTROL, and it is the same one the row used to separate the VALUE from
/// the system: identical `ResMut<OwnedItems>`, identical schedule position,
/// identical change detection, and it grants ZERO. If the audit reports the same
/// divergences here, they are not about the bag.
fn touch_the_bag_each_tick(mut owned: bevy::prelude::ResMut<OwnedItems>) {
    owned.grant(Item::HealthCell, 0);
}

fn sim_composed_with<M>(
    system: impl bevy::prelude::IntoScheduleConfigs<bevy::ecs::system::ScheduleSystem, M>
        + Clone
        + Send
        + Sync
        + 'static,
) -> Platformer2dSimHarness {
    use ambition_platformer2d::sim::SimScheduleExt;
    Platformer2dSimHarness::build(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_required_start_room(ROOM)
            .with_sync_test_rollback_settings(4, 10),
        |app, options| {
            ambition_app::rl_sim::ambition_sim_composition(app, options)?;
            let label = app.sim_schedule();
            app.add_systems(label, system.clone());
            Ok(())
        },
    )
    .expect("the sync-test harness builds with a grant inside the tick")
}

fn sim_tick(sim: &Platformer2dSimHarness) -> u64 {
    sim.world()
        .get_resource::<ambition_platformer2d::time::SimTick>()
        .map_or(0, |tick| tick.0)
}

fn health(sim: &Platformer2dSimHarness) -> Result<(), String> {
    sim.rollback_health()
}

/// ⛔ THE WHOLE HASHED VALUE, NOT ONE FIELD OF IT. `AmbitionGameSave::checksum`
/// serialises the ENTIRE save to RON and hashes the bytes, so every field is in
/// the peer-visible value. Sampling one field and concluding "the save is not
/// changing" is a claim about that field.
fn save_checksum(sim: &Platformer2dSimHarness) -> u64 {
    sim.world()
        .resource::<ambition_platformer2d::persistence::save::AmbitionGameSave>()
        .checksum()
}

/// The field the row's elimination sampled: the mirrored item list, by ROW.
fn mirrored_rows(sim: &Platformer2dSimHarness) -> usize {
    sim.world()
        .resource::<ambition_platformer2d::persistence::save::AmbitionGameSave>()
        .data()
        .items()
        .len()
}

/// ⛔ THE SAME LIST BY QUANTITY. `PersistedItem` is `{ id, count }`, so granting
/// the same item repeatedly moves a `count` and leaves the ROW COUNT alone.
fn mirrored_quantity(sim: &Platformer2dSimHarness) -> u32 {
    sim.world()
        .resource::<ambition_platformer2d::persistence::save::AmbitionGameSave>()
        .data()
        .items()
        .iter()
        .map(|item| item.count)
        .sum()
}

/// ⚠ AND THE SUBSTRING THE ROW'S FILTER USED, kept as an arm of its own. `id` is
/// documented as *"the stable lowercase `dialog_id`"*, so `"HealthCell"` cannot
/// appear in any row's `Debug` in any world — a filter on it returns 0 whatever
/// the bag holds. "It found nothing" was a fact about the string.
fn rows_matching_the_camel_case_name(sim: &Platformer2dSimHarness) -> usize {
    sim.world()
        .resource::<ambition_platformer2d::persistence::save::AmbitionGameSave>()
        .data()
        .items()
        .iter()
        .filter(|item| format!("{item:?}").contains("HealthCell"))
        .count()
}

/// What the ids ACTUALLY are, printed once so the line above is checkable.
fn mirrored_ids(sim: &Platformer2dSimHarness) -> Vec<String> {
    sim.world()
        .resource::<ambition_platformer2d::persistence::save::AmbitionGameSave>()
        .data()
        .items()
        .iter()
        .map(|item| format!("{}={}", item.id, item.count))
        .collect()
}

/// Print the per-type census divergences the existing audit records while the
/// bag is moving, beside the control where it is not.
#[test]
#[ignore = "PROBE, print-only: which hashed entry diverges when the bag moves"]
fn probe_which_registered_type_diverges_when_the_bag_moves() {
    for (name, mut sim) in [
        ("grant 1 each tick", sim_composed_with(grant_each_tick)),
        (
            "CONTROL: touch the bag, grant ZERO",
            sim_composed_with(touch_the_bag_each_tick),
        ),
    ] {
        sim.world_mut()
            .insert_resource(ambition_platformer2d::rollback::RollbackRestoreAudit::enabled());
        println!("── {name}");
        for step in 0..LIVE_STEPS {
            sim.step(AgentAction::default());
            let audit = sim
                .world()
                .resource::<ambition_platformer2d::rollback::RollbackRestoreAudit>();
            println!(
                "   step={step:>2} tick={:>3} save_sum={:#018x} rows={:>2} qty={:>3} \
                 camel_case_matches={} divergences={:>3} health={:?}",
                sim_tick(&sim),
                save_checksum(&sim),
                mirrored_rows(&sim),
                mirrored_quantity(&sim),
                rows_matching_the_camel_case_name(&sim),
                audit.divergences.len(),
                health(&sim)
                    .err()
                    .map(|error| error.chars().take(44).collect::<String>()),
            );
        }
        let audit = sim
            .world()
            .resource::<ambition_platformer2d::rollback::RollbackRestoreAudit>();
        // ⛔ COVERAGE FIRST. An audit that compared nothing reports no divergence,
        // and that reads exactly like a clean one.
        println!("   ids: {:?}", mirrored_ids(&sim));
        println!("   coverage: {}", audit.coverage());
        println!("   report:\n{}", audit.report());
    }
}

/// ⛔⛤ **THE ATTRIBUTION, PINNED — exactly one hashed entry diverges, and it is
/// the save mirror.**
///
/// This is the answer ROLLBACK-BAG-DESYNC asked for, asserted rather than
/// printed, so that a fix, a second diverging entry, or a regression all report
/// instead of needing the probe re-read.
///
/// ⚠ **THE GRANT MUST BE PER-TICK AND SUSTAINED.** Measured by CalculexAmbition:
/// a `SimTick`-gated grant firing ONCE at tick 20 runs 240 steps clean, health
/// `Ok`, with the bag moving 3 → 4. Only the sustained per-tick change
/// reproduces. ⇒ An arm rewritten to grant once would report an empty divergence
/// set, which this arm's own assertion reads as "the repair landed" — so the
/// cadence is load-bearing and both audits are floored on `resimulations > 0`
/// before either result is read.
///
/// ⚠ **WHICH FAILURE DIRECTION IS THE GOOD ONE.** If the granting arm reports NO
/// divergence, `persist_inventory_to_save` has been moved inside the rewinding
/// schedule or the mirror has stopped deriving from the bag — delete this arm and
/// close the row. If it reports a SECOND type, that type is a new finding. If the
/// CONTROL starts diverging, the cause is no longer the bag and every elimination
/// in the row needs redoing.
#[test]
fn exactly_one_hashed_entry_diverges_when_the_bag_moves_and_it_is_the_save() {
    let mut granting = sim_composed_with(grant_each_tick);
    granting
        .world_mut()
        .insert_resource(ambition_platformer2d::rollback::RollbackRestoreAudit::enabled());
    let mut control = sim_composed_with(touch_the_bag_each_tick);
    control
        .world_mut()
        .insert_resource(ambition_platformer2d::rollback::RollbackRestoreAudit::enabled());
    for _ in 0..LIVE_STEPS {
        granting.step(AgentAction::default());
        control.step(AgentAction::default());
    }

    let granting_audit = granting
        .world()
        .resource::<ambition_platformer2d::rollback::RollbackRestoreAudit>();
    // ⛔ FLOOR BOTH SIDES, NOT JUST THE CONTROL. An audit that saw no
    // resimulation reports no divergence, and on THIS side that reads as "the
    // repair landed" rather than as "the instrument saw nothing".
    assert!(
        granting_audit.resimulations > 0,
        "the granting audit saw no resimulation ({}), so the divergence set below \
         is a reading about the audit and not about the world",
        granting_audit.coverage()
    );
    let diverging: std::collections::BTreeSet<&str> = granting_audit
        .divergences
        .iter()
        .map(|divergence| divergence.type_name)
        .collect();

    // ⛔ COVERAGE BEFORE THE CONTROL'S SILENCE IS READ. An audit that compared
    // nothing reports no divergence, which reads exactly like a clean world.
    let control_audit = control
        .world()
        .resource::<ambition_platformer2d::rollback::RollbackRestoreAudit>();
    assert!(
        control_audit.resimulations > 0,
        "the control audit saw no resimulation ({}), so its silence is a reading \
         about the audit and not about the world",
        control_audit.coverage()
    );
    let control_diverging: std::collections::BTreeSet<&str> = control_audit
        .divergences
        .iter()
        .map(|divergence| divergence.type_name)
        .collect();
    assert!(
        control_diverging.is_empty(),
        "the CONTROL diverges too ({control_diverging:?}), so the cause is not the \
         bag's value moving — it is the system's presence in the schedule, and \
         every elimination in ROLLBACK-BAG-DESYNC needs redoing"
    );

    assert_eq!(
        diverging.iter().copied().collect::<Vec<_>>(),
        vec!["ambition_persistence::save::AmbitionGameSave"],
        "the set of hashed entries that disagree with themselves across a \
         resimulation is not exactly the save mirror. See this arm's doc for \
         which direction is the good one."
    );
}

/// ⛔⛤ **IS THE SAVE'S SNAPSHOT PINNED AT TICK 1, OR STALE BY A FIXED AMOUNT?**
///
/// CalculexAmbition's probe and the audit above printed the SAME number from two
/// instruments and two sessions: in a desyncing run the save's per-tick census
/// reads `0xce4e4758…` at tick 1 and a new value every tick after, and
/// `0xce4e4758…` is exactly the constant replay value the audit reported for
/// frames 2, 3 and 4. So the replay of every compared frame saw the save as it
/// was at tick 1.
///
/// ⚠ **TWO EXPLANATIONS SURVIVE THAT AND THE WINDOW ABOVE CANNOT SEPARATE THEM.**
/// "The restore point is tick 1" and "the snapshot is stale by a fixed amount"
/// predict the same thing over frames 2–4, because a fixed lag of one or two
/// frames IS tick 1 when you are standing at frame 3. The window is eight steps
/// because the session dies at step 6, so it has never sampled a compared frame
/// far from the start.
///
/// ⇒ A grant gated to start at tick 4 runs CLEAN for 240 steps (measured by
/// CalculexAmbition), which means the audit gets ~120 compared frames spread
/// across the whole run. If the save's census takes MANY distinct values across
/// those frames, its snapshot tracks the frame it is taken at, and "stale by a
/// fixed amount" is dead everywhere except the first three ticks. If it takes
/// ONE, the snapshot is pinned and the desync window is where that happens to
/// matter.
///
/// ⛔ Print-only, because the number it produces is the input to somebody else's
/// open question (`Q129`) and not an assertion this file is entitled to make.
#[test]
#[ignore = "PROBE, print-only: does the save's snapshot track its frame once the first ticks are past"]
fn probe_whether_the_saves_snapshot_tracks_its_frame_after_the_window() {
    /// The same per-tick grant, gated to start clear of the first three ticks.
    fn grant_each_tick_from_four(
        tick: bevy::prelude::Res<ambition_platformer2d::time::SimTick>,
        mut owned: bevy::prelude::ResMut<OwnedItems>,
    ) {
        if tick.0 >= 4 {
            owned.grant(Item::HealthCell, 1);
        }
    }

    type Save = ambition_platformer2d::persistence::save::AmbitionGameSave;

    // ⚠ The `fn(_, _)` cast this was written with does NOT implement
    // `SystemParamFunction`, so the loop shape the sibling arms use cannot carry
    // a two-parameter system. One name, one call.
    for name in ["grant 1 each tick FROM TICK 4"] {
        let mut sim = sim_composed_with(grant_each_tick_from_four);
        sim.world_mut()
            .insert_resource(ambition_platformer2d::rollback::RollbackRestoreAudit::enabled());
        for _ in 0..240 {
            sim.step(AgentAction::default());
        }
        let audit = sim
            .world()
            .resource::<ambition_platformer2d::rollback::RollbackRestoreAudit>();
        // ⛔ COVERAGE AND HEALTH FIRST. A run that died at step 6 and froze would
        // print a small number here and it would be about the freeze.
        println!("── {name}");
        println!("   end tick: {}", sim_tick(&sim));
        println!("   health: {:?}", health(&sim).err());
        println!("   coverage: {}", audit.coverage());
        println!(
            "   distinct save censuses across COMPARED frames: {}",
            audit.distinct_censuses_across_compared_frames_of::<Save>()
        );
        println!(
            "   distinct bag censuses across COMPARED frames: {}",
            audit.distinct_censuses_across_compared_frames_of::<OwnedItems>()
        );
        println!("   divergences: {}", audit.divergences.len());
        // ⛔⛤ A COUNT OF ONE IS NOT AN ANSWER; *WHICH* ONE IS. A census stuck at a
        // single value across 236 compared frames, while the live value moved
        // hundreds of times, is a snapshot that is not tracking — and from the
        // count alone that is indistinguishable from a quiet subject.
        let censuses = audit.censuses_across_compared_frames_of::<Save>();
        if let (Some(first), Some(last)) = (censuses.first(), censuses.last()) {
            println!(
                "   save census at the FIRST compared frame ({}): count={} xor={:#018x}",
                first.0, first.1.count, first.1.xor
            );
            println!(
                "   save census at the LAST compared frame  ({}): count={} xor={:#018x}",
                last.0, last.1.count, last.1.xor
            );
        }
        println!("   save checksum LIVE now: {:#018x}", save_checksum(&sim));
        println!("   mirrored qty now: {}", mirrored_quantity(&sim));
        println!(
            "   ⇒ if the two censuses agree with each other and DISAGREE with the \
             live checksum, the checksummed snapshot is pinned rather than tracking."
        );
        // ⛔⛤ THE CONTROL, AND IT IS THE ABSENCE OF THE SUBJECT. "The save's census
        // never moved across 236 compared frames" means nothing unless OTHER
        // types' censuses did. An empty list here is a reading about the audit.
        let moved = audit.types_whose_census_moved_across_compared_frames();
        println!(
            "   CONTROL — types whose census DID move across the compared frames: {}",
            moved.len()
        );
        for (type_name, values) in moved.iter().take(12) {
            println!("       {values:>4} distinct  {type_name}");
        }
    }
}

//! **How much of `Update` is even addressable by set-level gating.**
//!
//! `dev/journals/code_smells.md` 2026-07-23 recorded the everything-schedule:
//! *"311 static `add_systems(Update, …)` call sites … the multithreaded
//! executor's per-system graph bookkeeping measured 10-18% of CPU self-time in
//! EVERY phase of the desktop-lifecycle-1 profile — title screen included, where
//! almost none of those systems have work."* Its suggested fix is *"run_if-gating
//! whole sets by session phase and merging trivial systems"*.
//!
//! ⚠ **311 counted CALL SITES, and one `add_systems` registers many systems.**
//! The runtime population is what the executor pays for, so that is what this
//! measures.
//!
//! ## Why "systems in no set" is the number worth having first
//!
//! The proposed fix is set-level: gate a SET by session phase. A system that
//! belongs to no set cannot be gated that way — it would need its own condition,
//! one at a time, which is a different (and much larger) piece of work. So before
//! anyone estimates "gate the sets", the honest question is *what fraction of the
//! schedule is inside a set at all*.
//!
//! ⛔ **this deliberately does NOT claim to measure what runs.** Bevy does not
//! expose per-frame skip counts, and run conditions are not reachable from the
//! public `ScheduleGraph` API in 0.18 — so "how many are gated" cannot be
//! measured here and is not asserted. Set MEMBERSHIP can be, and it bounds the
//! answer from above: an unsetted system is certainly not set-gated.
//!
//! ⚠ **this counts the DEV composition, and that is not a detail.** The shipped
//! desktop build sets `SimulationHost::Ggrs` inside
//! `#[cfg(feature = "dev_tools")]`, so its ~242 `CoreSimulation` systems live in
//! `GgrsSchedule` and are NOT in the number below. A build without `dev_tools` —
//! the browser entry sets no host at all — resolves to the render-frame default
//! and puts those systems back into `Update`. So this census is a lower bound
//! for that composition, and comparing it against one taken from a different
//! feature set compares two different schedules. (Measured 2026-08-03.)

use bevy::ecs::schedule::graph::Direction;
use bevy::ecs::schedule::{NodeId, ScheduleLabel, Schedules};
use bevy::prelude::*;

/// Systems in the schedule, split into (in at least one AUTHORED set, in none),
/// plus a per-crate tally of the UNSETTED ones.
fn set_membership(
    app: &mut App,
    label: impl ScheduleLabel,
) -> (usize, usize, Vec<(String, usize)>) {
    let label = label.intern();
    app.world_mut()
        .resource_scope(|world, mut schedules: Mut<Schedules>| {
            let schedule = schedules.get_mut(label).expect("the schedule exists");
            // The graph is built lazily on first run; a schedule that has never
            // run reports no structure at all.
            let _ = schedule.initialize(world);
            let graph = schedule.graph();
            let hierarchy = graph.hierarchy().graph();
            let mut in_a_set = 0usize;
            let mut orphan = 0usize;
            let mut by_crate: std::collections::BTreeMap<String, usize> =
                std::collections::BTreeMap::new();
            for (key, system) in schedule
                .systems()
                .expect("initialized above, so the systems are enumerable")
            {
                // ⛔ **EXCLUDE `SystemTypeSet`.** Bevy puts every system into an
                // automatic per-system-type set so that `.after(my_system)` can
                // resolve. Counting those made the first draft of this census
                // report 97% "in a set" — a measure of Bevy's own bookkeeping,
                // not of authored structure, and it would have said the gating
                // work was nearly free. Sampling the parent NAMES is what caught
                // it; the sample showed `SystemTypeSet(fn …)` and nothing else
                // for row after row.
                //
                // ⛔ and this is done STRUCTURALLY, without naming anything.
                // `get_node_name` on a set walks its members to render an
                // anonymous set's name, and PANICS if the hierarchy still
                // references a system the schedule no longer holds — which this
                // app's graph does. A `SystemTypeSet` is exactly the set whose
                // only member is its own system, so "a parent with more than one
                // member" identifies an authored grouping without asking any
                // node what it is called.
                //
                // ⚠ the known error: a genuinely authored set with exactly ONE
                // member is counted as unsetted. That biases the number DOWN, so
                // the conclusion it supports (how much is addressable) is
                // conservative rather than flattering.
                let authored_parents = hierarchy
                    .neighbors_directed(NodeId::System(key), Direction::Incoming)
                    .filter(|parent| matches!(parent, NodeId::Set(_)))
                    .filter(|parent| {
                        hierarchy
                            .neighbors_directed(*parent, Direction::Outgoing)
                            .count()
                            > 1
                    })
                    .count();
                if authored_parents > 0 {
                    in_a_set += 1;
                } else {
                    orphan += 1;
                    // ⚠ the SYSTEM's own name, not `get_node_name` — see above
                    // for why naming a node can panic on this graph.
                    let name = format!("{}", system.name());
                    let owner = name
                        .split("::")
                        .next()
                        .unwrap_or("<unknown>")
                        .rsplit(' ')
                        .next()
                        .unwrap_or("<unknown>")
                        .to_string();
                    *by_crate.entry(owner).or_default() += 1;
                }
            }
            let mut tally: Vec<(String, usize)> = by_crate.into_iter().collect();
            tally.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
            (in_a_set, orphan, tally)
        })
}

/// ⭐ **The census, on the shipped composition rather than a fixture.**
///
/// Printed rather than pinned to an exact number: this is a MEASUREMENT that
/// should move, and a test that fails whenever a system is added would be
/// deleted within a week. The assertion is only that the schedule is still large
/// enough for the question to matter, so the print is not silently measuring an
/// empty app.
#[test]
fn census_of_how_much_of_update_is_inside_a_set() {
    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    for _ in 0..4 {
        app.update();
    }

    let (update_in_set, update_orphan, update_tally) = set_membership(&mut app, Update);
    let (ggrs_in_set, ggrs_orphan, _) = set_membership(
        &mut app,
        ambition_platformer2d::runtime::rollback::GgrsSchedule,
    );

    let update_total = update_in_set + update_orphan;
    let ggrs_total = ggrs_in_set + ggrs_orphan;
    eprintln!(
        "[update-census] Update: {update_total} systems — {update_in_set} in a set, \
         {update_orphan} in NONE ({:.0}% unsetted)",
        100.0 * update_orphan as f32 / update_total.max(1) as f32
    );
    eprintln!(
        "[update-census] GgrsSchedule: {ggrs_total} systems — {ggrs_in_set} in a set, \
         {ggrs_orphan} in NONE ({:.0}% unsetted)",
        100.0 * ggrs_orphan as f32 / ggrs_total.max(1) as f32
    );

    for (owner, count) in update_tally.iter().take(15) {
        eprintln!("[update-census]   unsetted in `Update`: {count:>4}  {owner}");
    }

    assert!(
        update_total > 100,
        "the shipped app's `Update` should carry hundreds of systems; {update_total} \
         means this measured something other than the real composition"
    );
}

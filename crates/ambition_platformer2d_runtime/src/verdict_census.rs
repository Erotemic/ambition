//! `[census] verdicts` — WHAT AUTHORED CONTENT ASKED, AND WHAT IS STUCK.
//!
//! ⛔⛤ **THE RING COULD ONLY BE READ OUT OF A TEST.** `AuthoredVerdictLog`
//! answers *"why is this door shut"* — it holds the structured `no`, who asked
//! it and which frame it was produced on — and every consumer of it was an
//! `assert!`. A running game had the evidence in memory and no way to show it,
//! so diagnosing a stuck gate still meant attaching a debugger or writing a
//! test that reproduces the thing you are trying to understand.
//!
//! ⚠ **IT ADDS NO RING.** The last review's steering was *"move to
//! running-process access … rather than adding more independent diagnostic
//! rings"*. This is a derived, read-only projection of the one that exists,
//! like `[census] rooms` is of `RoomSet`.
//!
//! ⛔ **IT PRINTS WHO IS BLOCKED, NOT EVERY ENTRY.** A 256-entry ring rendered
//! every sample is a firehose nobody reads, and the question a periodic line
//! is actually answering is *"what is refusing right now, and who is asking"*.
//! So the row carries the counts, the sources that are refusing, and the most
//! recent negative in full. A reader who needs the rest has the ring.
//!
//! ⛔⛤ **`blocked` MEANT "HAS EVER BEEN REFUSED" AND WAS LABELLED "RIGHT NOW"
//! — REVIEW 2026-09-21.** It inserted a source on every negative entry and
//! nothing ever removed one, so an ordinary history — `flag_set("key") -> no`,
//! the player finds the key, `flag_set("key") -> yes` — went on printing
//! `blocked=1[lock_wall:gate]` until the `no` aged out of the ring. After a
//! room change it was worse: a wall that no longer exists stayed listed
//! because its old `no` was still in history.
//!
//! ⇒ **THE ROW CARRIES BOTH PRODUCTS AND NAMES EACH.** `stuck` folds the ring
//! down to the LATEST outcome per `(source, id, args)` and counts only the
//! ones whose latest answer is negative; `ever-no` is the historical set the
//! old number actually measured, kept because "this refused at some point"
//! is the forensic question and is worth one integer. A source that stops
//! asking cannot leave `stuck` — nothing contradicts its last `no` — and the
//! doc says so rather than the number implying otherwise; truly current state
//! needs source liveness, which is OW1's problem and not this row's.

use bevy::prelude::{App, Last, Plugin, Res};

use ambition_dev_tools::runtime_census::RuntimeCensus;
use ambition_platformer2d_shared_tangle::authored_logic::{
    AuthoredVerdict, AuthoredVerdictLog, CommandOutcome, ConditionOutcome,
};

/// Emit `[census] verdicts` on the shared census clock.
pub fn report_verdict_census(census: Res<RuntimeCensus>, log: Option<Res<AuthoredVerdictLog>>) {
    let Some(at) = census.due() else {
        return;
    };
    eprintln!(
        "{}",
        verdict_census_row(at, log.as_deref().map(|log| log.recent()).as_deref())
    );
}

/// The row itself, as a value, so something can assert on it.
///
/// `None` is **no ring installed**, which is the shipped default and a
/// different world from an installed ring holding nothing. A row that printed
/// the same text for both would tell a reader who just enabled the census that
/// their game asks no authored questions.
pub fn verdict_census_row(at: f64, entries: Option<&[AuthoredVerdict]>) -> String {
    let Some(entries) = entries else {
        return format!(
            "[census] verdicts t={at:.3} log=absent (insert `AuthoredVerdictLog` to record)"
        );
    };

    let mut asked = 0usize;
    let mut ran = 0usize;
    let mut unsatisfied = 0usize;
    let mut unanswerable = 0usize;
    let mut refused = 0usize;
    let mut speculative = 0usize;
    // BTree, not Hash: this line is read by comparing one sample against the
    // next, and a set that reorders between samples makes a stable world look
    // like a changing one.
    let mut ever_negative: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    // ⭐ **THE LATEST ANSWER PER INVOCATION, WHICH IS WHAT "STUCK" MEANS.** The
    // key is the whole identity of a repeated question — WHO asked, WHAT they
    // asked, and with which arguments — because a later `yes` only contradicts
    // an earlier `no` when all three match. `recent()` yields oldest-first, so
    // a plain insert leaves the newest answer in place.
    let mut latest: std::collections::BTreeMap<(String, String, String), bool> =
        std::collections::BTreeMap::new();
    let mut last_negative: Option<&AuthoredVerdict> = None;

    for entry in entries {
        if !entry.stamp().confirmed {
            speculative += 1;
        }
        let negative = match entry {
            AuthoredVerdict::Asked(verdict) => {
                asked += 1;
                match &verdict.outcome {
                    ConditionOutcome::Satisfied => false,
                    ConditionOutcome::NotSatisfied(_) => {
                        unsatisfied += 1;
                        true
                    }
                    ConditionOutcome::Unanswerable(_) => {
                        unanswerable += 1;
                        true
                    }
                }
            }
            AuthoredVerdict::Ran(verdict) => {
                ran += 1;
                match &verdict.outcome {
                    CommandOutcome::Done => false,
                    CommandOutcome::Refused(_) => {
                        refused += 1;
                        true
                    }
                }
            }
        };
        // ⚠ Formatted, not structural: `AuthoredArg` is not `Ord`, and a
        // rendered identity is exactly as discriminating as the values it
        // renders. It is a diagnostic key, never a simulation one.
        let source = entry.asked_by().to_string();
        let invocation = match entry {
            AuthoredVerdict::Asked(v) => format!("ask:{}", v.id),
            AuthoredVerdict::Ran(v) => format!("run:{:?}", v.id),
        };
        latest.insert(
            (source.clone(), invocation, format!("{:?}", entry.args())),
            negative,
        );
        if negative {
            ever_negative.insert(source);
            last_negative = Some(entry);
        }
    }

    // ⛔ THE SOURCES WHOSE LATEST ANSWER IS STILL NO. A source with one stuck
    // question and nine satisfied ones is stuck.
    let stuck: std::collections::BTreeSet<&str> = latest
        .iter()
        .filter(|(_, negative)| **negative)
        .map(|((source, _, _), _)| source.as_str())
        .collect();

    let mut row = format!(
        "[census] verdicts t={at:.3} ring={} asked={asked} ran={ran} no={unsatisfied} \
         unanswerable={unanswerable} refused={refused} speculative={speculative}",
        entries.len(),
    );
    // ⚠ The FULL set is counted and a bounded sample is printed, so a reader
    // never mistakes the cap for the population.
    row.push_str(&format!(" stuck={}", stuck.len()));
    if !stuck.is_empty() {
        const SHOWN: usize = 6;
        let sample: Vec<&str> = stuck.iter().copied().take(SHOWN).collect();
        row.push_str(&format!("[{}", sample.join(" ")));
        if stuck.len() > SHOWN {
            row.push_str(&format!(" +{}", stuck.len() - SHOWN));
        }
        row.push(']');
    }
    // ⚠ ONE INTEGER FOR THE FORENSIC HALF. `stuck <= ever-no` always; the two
    // being different is the normal shape of a game somebody is playing, and
    // the two being equal after a long session is worth a look.
    row.push_str(&format!(" ever-no={}", ever_negative.len()));
    if let Some(entry) = last_negative {
        row.push_str(&format!(" | last-no: {entry}"));
    }
    row
}

/// Registers [`report_verdict_census`] AND the ring it reports, and only when
/// the census is on.
///
/// ⛔ Registered only when asked, for the reason `RoomCensusPlugin` gives:
/// `due_at` is only set while the census is enabled, so this could never have
/// reported when off.
///
/// ⛔⛤ **IT INSTALLED THE READER AND NOT THE RECORDER, SO THE ANSWER A RUNNING
/// GAME GAVE WAS INSTRUCTIONS FOR A PROGRAMMER — REVIEW 2026-09-21.** Nothing
/// on any production road inserts [`AuthoredVerdictLog`], so
/// `AMBITION_PROFILE_CENSUS=1` printed `log=absent (insert AuthoredVerdictLog
/// to record)` forever. The slice's claim was *"a running game can now be
/// asked what is stuck"*, and what it had actually built was a formatter
/// reachable from a running game — getting output still required a debugger or
/// a test, which is the limitation it said it closed. An observability surface
/// whose normal answer is *"modify the process"* is not one.
///
/// ⚠ **THE RING STAYS ABSENT BY DEFAULT, AND THAT IS UNCHANGED.** The ring's
/// own module says absence is the off switch and there is no env var; this
/// does not add one. Opting into the CENSUS is the opt-in — the same gate that
/// already decides whether this plugin registers anything at all — so a game
/// nobody is profiling still pays a resource lookup per evaluation and
/// nothing else. A composition that wants the ring WITHOUT the census still
/// inserts it, exactly as before, and `init_resource` will not disturb one
/// that did.
pub struct VerdictCensusPlugin;

impl Plugin for VerdictCensusPlugin {
    fn build(&self, app: &mut App) {
        if RuntimeCensus::from_env().enabled() {
            app.init_resource::<AuthoredVerdictLog>()
                .add_systems(Last, report_verdict_census);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::App;

    /// TURNING THE CENSUS ON TURNS THE RECORDER ON.
    ///
    /// ⛔⛤ **REVIEW 2026-09-21: THE PLUGIN INSTALLED THE READER AND NOT THE
    /// RING**, so a running game with `AMBITION_PROFILE_CENSUS=1` printed
    /// `log=absent (insert AuthoredVerdictLog to record)` — an observability
    /// surface whose answer is instructions for a programmer.
    ///
    /// ⚠ **AND THE OFF ARM IS THE HALF THAT MATTERS.** The ring's own module
    /// says absence IS the off switch; a plugin that installed it
    /// unconditionally would make every composition record, which is the
    /// opposite defect and would not be visible in any output.
    #[test]
    fn the_census_installs_the_ring_it_reports_and_only_when_it_is_on() {
        let on = RuntimeCensus::from_env().enabled();
        let mut app = App::new();
        app.add_plugins(VerdictCensusPlugin);
        assert_eq!(
            app.world().get_resource::<AuthoredVerdictLog>().is_some(),
            on,
            "the census is {}, so the ring should be {} — with the census ON a running \
             game must record without being modified, and with it OFF nothing may start \
             recording behind a composition's back",
            if on { "enabled" } else { "disabled" },
            if on { "present" } else { "absent" },
        );
    }
}

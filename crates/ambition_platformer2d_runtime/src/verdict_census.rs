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
//! So the row carries the counts, the distinct SOURCES with at least one
//! unsatisfied or refused entry, and the most recent negative in full. A
//! reader who needs the rest has the ring.

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
    let mut blocked: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
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
        if negative {
            blocked.insert(entry.asked_by().to_string());
            last_negative = Some(entry);
        }
    }

    let mut row = format!(
        "[census] verdicts t={at:.3} ring={} asked={asked} ran={ran} no={unsatisfied} \
         unanswerable={unanswerable} refused={refused} speculative={speculative}",
        entries.len(),
    );
    // ⚠ The FULL set is counted and a bounded sample is printed, so a reader
    // never mistakes the cap for the population.
    row.push_str(&format!(" blocked={}", blocked.len()));
    if !blocked.is_empty() {
        const SHOWN: usize = 6;
        let sample: Vec<&str> = blocked.iter().take(SHOWN).map(String::as_str).collect();
        row.push_str(&format!("[{}", sample.join(" ")));
        if blocked.len() > SHOWN {
            row.push_str(&format!(" +{}", blocked.len() - SHOWN));
        }
        row.push(']');
    }
    if let Some(entry) = last_negative {
        row.push_str(&format!(" | last-no: {entry}"));
    }
    row
}

/// Registers [`report_verdict_census`], and only when the census is on.
///
/// ⛔ Registered only when asked, for the reason `RoomCensusPlugin` gives:
/// `due_at` is only set while the census is enabled, so this could never have
/// reported when off.
pub struct VerdictCensusPlugin;

impl Plugin for VerdictCensusPlugin {
    fn build(&self, app: &mut App) {
        if RuntimeCensus::from_env().enabled() {
            app.add_systems(Last, report_verdict_census);
        }
    }
}

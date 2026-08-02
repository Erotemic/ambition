//! **Who wrote this body's velocity.**
//!
//! ⛔ The causal log answered "what is the velocity" and never "who set it", and
//! that gap cost six rebuild-and-print cycles on a single twelve-tick window
//! (queue S51) — eight candidates eliminated one at a time, none of them the
//! cause, because a body moved and nothing that published a fact admitted to
//! moving it.
//!
//! A velocity write OUTSIDE the integrator is the interesting kind: knockback, a
//! move's lunge, ranged recoil, a launch, a pushout. Each one is a place where
//! something other than the body's own motion law decided where it goes, and
//! each is invisible unless it says so.
//!
//! ## Why this is a helper and not a pattern
//!
//! The first two writers were instrumented by hand and came out subtly
//! different — different field names, one carrying the move id, one not. That
//! is the same divergence the log exists to prevent: two writers describing the
//! same event in two vocabularies makes them uncomparable in the one place they
//! are meant to be compared. **One constructor, so every writer's story has the
//! same shape**, and instrumenting the next one costs a line instead of thirty.
//!
//! ⚠ the caller resolves its OWN subject. Subject choice is
//! crate-specific (seat first, actor id second — see each domain's
//! `subject_of`), and a helper that guessed would be a second authority on
//! identity.

use crate::{domains, CausalFact, FactDetail, SubjectKey};

/// One velocity write, named.
///
/// `writer` is a stable, greppable label for the SITE — not the mechanic. Two
/// sites that both apply "knockback" should carry two labels if a reader would
/// ever need to tell them apart; the moveset's plain trigger and its CANCEL path
/// are the worked example, because "a move moved this body" and "a cancel moved
/// this body" are different bugs.
pub fn velocity_authored(
    tick: u64,
    subject: SubjectKey,
    writer: &'static str,
    before_x: f32,
    after_x: f32,
) -> CausalFact {
    CausalFact::new(
        domains::MOVEMENT,
        tick,
        FactDetail::new(
            "velocity_authored",
            format!("{writer} moved this body {:+.0}/s", after_x - before_x),
        ),
    )
    .about(subject)
    .field("writer", writer)
    .field("kick_x", after_x - before_x)
    .field("vel_x_before", before_x)
    .field("vel_x_after", after_x)
}

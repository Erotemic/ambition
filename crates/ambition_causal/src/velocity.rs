//! Who wrote this body's velocity.
//!
//! The causal log needs to identify velocity writers, not only report the
//! resulting velocity.
//!
//! A velocity write OUTSIDE the integrator is the interesting kind: knockback, a
//! move's lunge, ranged recoil, a launch, a pushout. Each one is a place where
//! something other than the body's own motion law decided where it goes, and
//! each is invisible unless it says so.
//!
//! ## Why this is a helper and not a pattern
//!
//! Use one constructor so every velocity writer emits the same fact shape.
//!
//!  the caller resolves its OWN subject. Subject choice is
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

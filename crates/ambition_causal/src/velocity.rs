//! Who wrote this body's velocity.
//!
//! The causal log needs to identify velocity writers, not only report the
//! resulting velocity.
//!
//! A velocity write outside the integrator matters most: knockback, a move's
//! lunge, ranged recoil, a launch, a pushout. Each is invisible unless it
//! records itself.
//!
//! Use one constructor so every velocity writer emits the same fact shape.
//! The caller resolves its own subject (seat first, actor id second; see each
//! domain's `subject_of`), so the helper is not a second authority on identity.

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

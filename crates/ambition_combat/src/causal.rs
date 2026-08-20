//! This crate's causal facts: **why did a body lose a stock, get eliminated, or
//! end the match?**
//!
//! ## An observer over MESSAGES, not over the rules
//!
//! Every fact here is derived from a message the stocks ruleset already writes
//! (`BodyKnockedOut`, `FighterStockSpent`, `StocksMatchDecided`). That is worth
//! more than instrumenting the rules directly:
//!
//! * it is **additive** — no rule gained a parameter, a branch, or a reason to
//!   care that anybody is watching;
//! * it **cannot** affect the outcome, because a message reader holds no
//!   authority over the writer;
//! * and it stays correct when the rules change, because it observes what the
//!   rules DECIDED rather than re-deriving how.
//!
//! ⚠ the one thing it cannot do is explain a knockout that was never announced.
//! That is the right limitation: a consequence no message carries is a
//! consequence no ruleset could act on either.
//!
//! ## Subjects
//!
//! The messages carry `Entity`, which is not an identity — indices are recycled.
//! A seated body's `DrivingParticipant(slot)` is, and a stock investigation is
//! always about a seat ("seat 1 lost three stocks without being hit"), which
//! survives the respawns in the middle of the answer.

use ambition_causal::{CausalFact, CausalRecording, FactDetail, SubjectKey, domains};
use ambition_characters::brain::DrivingParticipant;
use bevy::prelude::*;

use crate::stocks::{BodyKnockedOut, FighterStockSpent, StocksMatchDecided};

/// The seat a body is driven from, when it has one.
fn seat_of(bodies: &Query<&DrivingParticipant>, body: Entity) -> Option<u8> {
    Some(bodies.get(body).ok()?.0 .0)
}

/// **The strongest stable subject this body has** — and never `None`.
///
/// ⛔ a body-specific fact with no subject is treated by `explain` as a WORLD
/// fact and returned for EVERY subject. So a CPU fighter losing a stock on tick
/// 400 appeared in seat 0's causal chain, and the inspector answered the wrong
/// question precisely where it is needed most: a Smash match or a boss fight,
/// where several bodies act on the same tick (GPT 5.6, 2026-08-01, finding 5).
///
/// The old code returned no subject for an unseated body and its test said the
/// fact "explains the WORLD on that tick". A knockout does not explain the
/// world; it explains a body.
///
/// Order is strongest-first:
/// 1. the SEAT, which survives death and respawn;
/// 2. the actor's stable id, for a CPU / boss / NPC;
/// 3. an explicitly UNSTABLE entity key, which the variant exists to mark as a
///    recorded API leak. ⚠ that is still better than global: a recycled index
///    can mislead one later query, while a world fact misleads every query
///    forever.
fn subject_of(
    bodies: &Query<&DrivingParticipant>,
    identities: &Query<&crate::components::ActorIdentity>,
    body: Entity,
) -> (SubjectKey, Option<u8>) {
    if let Some(seat) = seat_of(bodies, body) {
        return (SubjectKey::Seat(seat), Some(seat));
    }
    if let Ok(identity) = identities.get(body) {
        return (SubjectKey::Sim(identity.id.clone()), None);
    }
    (SubjectKey::Unstable(body.to_bits()), None)
}

/// Publish the stock lifecycle: knockouts, spends, eliminations, the decision.
///
/// Takes `DrivingParticipant` immutably and the messages by read — an observer by
/// signature,
/// which matters here because a rollback host resimulates a deciding frame and
/// an instrument that nudged the ruleset would change who won.
pub fn record_stock_lifecycle(
    log: Option<ResMut<CausalRecording>>,
    mut knockouts: MessageReader<BodyKnockedOut>,
    mut spends: MessageReader<FighterStockSpent>,
    mut decided: MessageReader<StocksMatchDecided>,
    bodies: Query<&DrivingParticipant>,
    identities: Query<&crate::components::ActorIdentity>,
) {
    let Some(mut log) = log else {
        // ⚠ still DRAIN the readers. A reader that only advances while
        // recording would hand a backlog of stale messages to the first frame
        // somebody turned the instrument on — an explanation of a knockout that
        // happened minutes ago, stamped with this tick.
        knockouts.clear();
        spends.clear();
        decided.clear();
        return;
    };
    if !log.is_recording() {
        knockouts.clear();
        spends.clear();
        decided.clear();
        return;
    }

    for knockout in knockouts.read() {
        let mut fact = CausalFact::new(
            domains::LIFECYCLE,
            0,
            FactDetail::new(
                "body_knocked_out",
                format!("knocked out by {:?}", knockout.cause),
            ),
        )
        .field("cause", format!("{:?}", knockout.cause));
        let (subject, seat) = subject_of(&bodies, &identities, knockout.body);
        fact = fact.about(subject);
        if let Some(seat) = seat {
            fact = fact.by_participant(seat);
        }
        log.record(fact);
    }

    for spend in spends.read() {
        let mut fact = CausalFact::new(
            domains::LIFECYCLE,
            0,
            FactDetail::new(
                "stock_spent",
                if spend.eliminated {
                    "spent its last stock — eliminated".to_string()
                } else {
                    format!("spent a stock, {} left", spend.remaining)
                },
            ),
        )
        .field("remaining", i64::from(spend.remaining))
        .field("eliminated", spend.eliminated);
        let (subject, seat) = subject_of(&bodies, &identities, spend.body);
        fact = fact.about(subject);
        if let Some(seat) = seat {
            fact = fact.by_participant(seat);
        }
        log.record(fact);
    }

    for decision in decided.read() {
        // No subject: the match ending is about the world, so it explains every
        // body on that tick — including the ones it ended for.
        log.record(
            CausalFact::new(
                domains::LIFECYCLE,
                0,
                FactDetail::new(
                    "match_decided",
                    match &decision.winner {
                        Some(winner) => format!("match decided: {winner}"),
                        None => "match decided: a draw".to_string(),
                    },
                ),
            )
            .field(
                "winner",
                decision.winner.clone().unwrap_or_else(|| "<draw>".into()),
            )
            .field("draw", decision.winner.is_none()),
        );
    }
}

#[cfg(test)]
mod tests;

//! Causal facts derived from stock-lifecycle messages.
//!
//! The recorder is read-only with respect to match rules. Body subjects use the
//! strongest stable identity available: seat, then `ActorIdentity`, then an
//! explicitly unstable entity key.

use ambition_causal::{domains, CausalFact, CausalRecording, FactDetail, SubjectKey};
use ambition_characters::control::DrivingParticipant;
use bevy::prelude::*;

use crate::stocks::{BodyKnockedOut, FighterStockSpent, MatchVerdict, StocksMatchDecided};

/// The seat a body is driven from, when it has one.
fn seat_of(bodies: &Query<&DrivingParticipant>, body: Entity) -> Option<u8> {
    Some(bodies.get(body).ok()?.0 .0)
}

/// Return the strongest available subject identity, never a world-global fact.
/// Priority is seat, stable actor id, then explicitly unstable entity id.
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

/// Record stock lifecycle messages without participating in match decisions.
pub fn record_stock_lifecycle(
    log: Option<ResMut<CausalRecording>>,
    mut knockouts: MessageReader<BodyKnockedOut>,
    mut spends: MessageReader<FighterStockSpent>,
    mut decided: MessageReader<StocksMatchDecided>,
    bodies: Query<&DrivingParticipant>,
    identities: Query<&crate::components::ActorIdentity>,
) {
    let Some(mut log) = log else {
        // still DRAIN the readers. A reader that only advances while
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
                    match &decision.outcome {
                        MatchVerdict::Winner(winner) => format!("match decided: {winner}"),
                        MatchVerdict::Draw => "match decided: a draw".to_string(),
                        MatchVerdict::NoContest => "match stopped: no contest".to_string(),
                    },
                ),
            )
            // ⛔⛔ THREE ENDINGS, NOT TWO, and this instrument was still asking
            // the old binary question — it read `winner: Option<String>` where
            // `None` meant DRAW, which is precisely the conflation
            // `MatchVerdict` was introduced to remove: an abandoned match had
            // nowhere to go but to impersonate a draw. The fields say which of
            // the three it was, and `draw` now means DRAW rather than
            // "no winner".
            .field(
                "verdict",
                match &decision.outcome {
                    MatchVerdict::Winner(_) => "winner",
                    MatchVerdict::Draw => "draw",
                    MatchVerdict::NoContest => "no_contest",
                },
            )
            .field(
                "winner",
                match &decision.outcome {
                    MatchVerdict::Winner(winner) => winner.clone(),
                    MatchVerdict::Draw => "<draw>".to_string(),
                    MatchVerdict::NoContest => "<no contest>".to_string(),
                },
            )
            .field("draw", matches!(decision.outcome, MatchVerdict::Draw)),
        );
    }
}

#[cfg(test)]
mod tests;

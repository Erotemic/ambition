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
//! A seated body's `Brain::Player(slot)` is, and a stock investigation is
//! always about a seat ("seat 1 lost three stocks without being hit"), which
//! survives the respawns in the middle of the answer.

use ambition_causal::{CausalFact, CausalRecording, FactDetail, SubjectKey, domains};
use ambition_characters::brain::Brain;
use bevy::prelude::*;

use crate::stocks::{BodyKnockedOut, FighterStockSpent, StocksMatchDecided};

/// The seat a body is driven from, when it has one.
fn seat_of(bodies: &Query<&Brain>, body: Entity) -> Option<u8> {
    bodies.get(body).ok()?.player_slot().map(|slot| slot.0)
}

/// Publish the stock lifecycle: knockouts, spends, eliminations, the decision.
///
/// Takes `Brain` immutably and the messages by read — an observer by signature,
/// which matters here because a rollback host resimulates a deciding frame and
/// an instrument that nudged the ruleset would change who won.
pub fn record_stock_lifecycle(
    log: Option<ResMut<CausalRecording>>,
    mut knockouts: MessageReader<BodyKnockedOut>,
    mut spends: MessageReader<FighterStockSpent>,
    mut decided: MessageReader<StocksMatchDecided>,
    bodies: Query<&Brain>,
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
        if let Some(seat) = seat_of(&bodies, knockout.body) {
            fact = fact.about(SubjectKey::Seat(seat)).by_participant(seat);
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
        if let Some(seat) = seat_of(&bodies, spend.body) {
            fact = fact.about(SubjectKey::Seat(seat)).by_participant(seat);
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
mod tests {
    use super::*;
    use ambition_causal::{FactValue, RecordingPolicy};
    use ambition_characters::brain::PlayerSlot;

    fn app() -> App {
        let mut app = App::new();
        app.init_resource::<CausalRecording>();
        app.add_message::<BodyKnockedOut>();
        app.add_message::<FighterStockSpent>();
        app.add_message::<StocksMatchDecided>();
        app.add_systems(Update, record_stock_lifecycle);
        app
    }

    fn seated(app: &mut App, slot: u8) -> Entity {
        app.world_mut().spawn(Brain::Player(PlayerSlot(slot))).id()
    }

    #[test]
    fn a_spent_stock_explains_the_seat_that_spent_it() {
        let mut app = app();
        app.world_mut()
            .resource_mut::<CausalRecording>()
            .set_policy(RecordingPolicy::All)
            .set_tick(90);
        let body = seated(&mut app, 1);
        app.world_mut().write_message(FighterStockSpent {
            body,
            remaining: 2,
            eliminated: false,
        });
        app.update();

        let why = app
            .world()
            .resource::<CausalRecording>()
            .explain(90, &SubjectKey::Seat(1));
        let spend = why.first("stock_spent").expect("the seat's own stock");
        assert_eq!(spend.get("remaining"), Some(&FactValue::Int(2)));
        assert_eq!(spend.get("eliminated"), Some(&FactValue::Bool(false)));
        assert_eq!(spend.participant, Some(1));
        assert!(
            app.world()
                .resource::<CausalRecording>()
                .explain(90, &SubjectKey::Seat(0))
                .first("stock_spent")
                .is_none(),
            "seat 0 lost nothing, and must not inherit seat 1's stock"
        );
    }

    #[test]
    fn the_match_decision_explains_every_seat_because_it_is_about_the_world() {
        let mut app = app();
        app.world_mut()
            .resource_mut::<CausalRecording>()
            .set_policy(RecordingPolicy::All)
            .set_tick(91);
        app.world_mut().write_message(StocksMatchDecided {
            winner: Some("seat_0".into()),
        });
        app.update();

        for seat in [0u8, 3] {
            let why = app
                .world()
                .resource::<CausalRecording>()
                .explain(91, &SubjectKey::Seat(seat));
            assert_eq!(
                why.first("match_decided").and_then(|f| f.get("winner")),
                Some(&FactValue::Text("seat_0".into())),
                "the match ending is the world's fact and explains seat {seat} too"
            );
        }
    }

    #[test]
    fn a_knockout_of_an_unseated_body_is_recorded_without_a_borrowed_identity() {
        // A boss, an NPC, a training dummy: no seat, so no `SubjectKey`. The
        // fact is still worth keeping — it explains the WORLD on that tick —
        // but publishing it under a recycled entity index would let a later
        // body inherit it.
        let mut app = app();
        app.world_mut()
            .resource_mut::<CausalRecording>()
            .set_policy(RecordingPolicy::All)
            .set_tick(92);
        let body = app.world_mut().spawn(Brain::stand_still()).id();
        app.world_mut().write_message(BodyKnockedOut {
            body,
            cause: crate::HitSource::PlayerProjectile,
        });
        app.update();

        let log = app.world().resource::<CausalRecording>();
        let fact = log.facts().next().expect("the knockout was recorded");
        assert_eq!(fact.kind(), "body_knocked_out");
        assert!(fact.subject.is_none());
        assert_eq!(
            fact.get("cause"),
            Some(&FactValue::Text("PlayerProjectile".into())),
            "a ring-out and a meter death are different findings, so the cause is a FIELD \
             rather than something a reader has to infer from the summary"
        );
    }

    #[test]
    fn messages_are_drained_even_while_recording_is_off() {
        // ⚠ otherwise the first frame after somebody turns the instrument on
        // reports a backlog of old knockouts stamped with the CURRENT tick —
        // an explanation of something that happened minutes ago.
        let mut app = app();
        let body = seated(&mut app, 0);
        app.world_mut().write_message(FighterStockSpent {
            body,
            remaining: 1,
            eliminated: false,
        });
        app.update();

        app.world_mut()
            .resource_mut::<CausalRecording>()
            .set_policy(RecordingPolicy::All)
            .set_tick(50);
        app.update();
        assert!(
            app.world().resource::<CausalRecording>().is_empty(),
            "a message from before the instrument was on must not surface as this tick's"
        );
    }
}

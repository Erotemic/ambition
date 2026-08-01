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
    bodies: &Query<&Brain>,
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
/// Takes `Brain` immutably and the messages by read — an observer by signature,
/// which matters here because a rollback host resimulates a deciding frame and
/// an instrument that nudged the ruleset would change who won.
pub fn record_stock_lifecycle(
    log: Option<ResMut<CausalRecording>>,
    mut knockouts: MessageReader<BodyKnockedOut>,
    mut spends: MessageReader<FighterStockSpent>,
    mut decided: MessageReader<StocksMatchDecided>,
    bodies: Query<&Brain>,
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

    /// ⛔ **an unseated body's knockout is NOT a world fact**, and this test used
    /// to assert that it was.
    ///
    /// The old reasoning, written here: *"no seat, so no `SubjectKey`. The fact
    /// is still worth keeping — it explains the WORLD on that tick."* A knockout
    /// does not explain the world; it explains a body. And a fact with no
    /// subject is returned by `explain` for EVERY subject, so a CPU fighter
    /// losing a stock landed in seat 0's causal chain — the inspector answering
    /// the wrong question exactly where it is needed most (GPT 5.6, 2026-08-01,
    /// finding 5).
    ///
    /// A body with no seat and no stable id now gets an explicitly UNSTABLE
    /// key. That variant exists to mark a recorded API leak, and it is still
    /// strictly better than global: a recycled index can mislead one later
    /// query, while a world fact misleads every query forever.
    #[test]
    fn a_knockout_of_an_unseated_body_is_about_that_body_not_about_the_world() {
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
        assert!(
            matches!(fact.subject, Some(SubjectKey::Unstable(_))),
            "an unseated, unidentified body gets an explicitly UNSTABLE subject \
             rather than none: {:?}",
            fact.subject
        );
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

    /// **A CPU's stock loss must not appear in a PARTICIPANT's explanation.**
    ///
    /// The concrete failure the review names, reproduced: a CPU fighter loses a
    /// stock on the same tick a seated player is doing something, and the
    /// inspector is asked why seat 0 is where it is. Before the fix the CPU's
    /// fact had no subject, so `explain` returned it for seat 0 — and for every
    /// other seat, and for the world.
    ///
    /// ⚠ the second half matters as much: a genuinely GLOBAL fact must still
    /// reach every participant. `match_decided` is one — the match ending really
    /// does explain every seat — and a fix that made everything body-specific
    /// would break it.
    #[test]
    fn a_cpus_stock_loss_stays_out_of_a_participants_explanation() {
        let mut app = app();
        app.world_mut()
            .resource_mut::<CausalRecording>()
            .set_policy(RecordingPolicy::All)
            .set_tick(400);

        // A seated player and a CPU, acting on the same tick.
        let seated = seated(&mut app, 0);
        let cpu = app
            .world_mut()
            .spawn((
                Brain::stand_still(),
                crate::components::ActorIdentity::new("cpu_duelist", "CPU"),
            ))
            .id();

        app.world_mut().write_message(FighterStockSpent {
            body: cpu,
            remaining: 1,
            eliminated: false,
        });
        app.world_mut().write_message(FighterStockSpent {
            body: seated,
            remaining: 2,
            eliminated: false,
        });
        app.world_mut().write_message(StocksMatchDecided {
            winner: Some("seat_0".to_string()),
        });
        app.update();

        let log = app.world().resource::<CausalRecording>();

        // Seat 0's explanation carries ITS OWN stock loss…
        let seat_view = log.explain(400, &SubjectKey::Seat(0));
        let spends: Vec<_> = seat_view.all("stock_spent").collect();
        assert_eq!(
            spends.len(),
            1,
            "seat 0 spent one stock; the other belongs to the CPU. Got {:?}",
            spends
                .iter()
                .map(|f| (f.subject.clone(), f.get("remaining")))
                .collect::<Vec<_>>()
        );
        assert_eq!(
            spends[0].get("remaining"),
            Some(&FactValue::Int(2)),
            "and it is seat 0's own count, not the CPU's"
        );

        // …and the CPU's is filed under the CPU.
        let cpu_view = log.explain(400, &SubjectKey::Sim("cpu_duelist".into()));
        assert_eq!(
            cpu_view
                .first("stock_spent")
                .and_then(|f| f.get("remaining")),
            Some(&FactValue::Int(1)),
            "the CPU's stock loss is filed under the CPU's stable id"
        );

        // ⚠ and the genuinely global fact still reaches the seat.
        assert!(
            seat_view.first("match_decided").is_some(),
            "the match ending explains every seat — a fix that filed EVERYTHING \
             under a body would have broken the case the old behaviour got right"
        );
    }
}

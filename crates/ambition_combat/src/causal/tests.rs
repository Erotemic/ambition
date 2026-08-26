//! Tests for the parent module's causal publishers.

use super::*;
use ambition_causal::{FactValue, RecordingPolicy};
use ambition_characters::brain::Brain;
use ambition_characters::control::PlayerSlot;

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
    app.world_mut()
        .spawn(ambition_characters::control::DrivingParticipant(
            PlayerSlot(slot),
        ))
        .id()
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
        outcome: crate::stocks::MatchVerdict::Winner("seat_0".into()),
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
        cause: crate::HitSource::Projectile,
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
        // ⛔ `Projectile`, not `PlayerProjectile`. `HitSource` names WHAT STRUCK,
        // not whose it was — the owner is a separate fact — and this expectation
        // was still spelling a variant that no longer exists. It went unnoticed
        // because this whole module is behind `--features causal`, which no
        // per-crate `cargo test` turns on.
        Some(&FactValue::Text("Projectile".into())),
        "a ring-out and a meter death are different findings, so the cause is a FIELD \
         rather than something a reader has to infer from the summary"
    );
}

#[test]
fn messages_are_drained_even_while_recording_is_off() {
    // otherwise the first frame after somebody turns the instrument on
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

/// A CPU's stock loss must not appear in a PARTICIPANT's explanation.
///
/// the second half matters as much: a genuinely GLOBAL fact must still
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
        outcome: crate::stocks::MatchVerdict::Winner("seat_0".to_string()),
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

    // and the genuinely global fact still reaches the seat.
    assert!(
        seat_view.first("match_decided").is_some(),
        "the match ending explains every seat — a fix that filed EVERYTHING \
         under a body would have broken the case the old behaviour got right"
    );
}

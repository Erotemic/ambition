//! Aggregated integration tests for `ambition_runtime`'s causal recording.
//!
//! ⚠ **these were inline `#[cfg(test)]` and were moved OUT**, because the
//! agent-KB instrument flagged the module at 261 test lines and the honest
//! answer to "does it need to be inline" was no: every item they touch is
//! `pub`. The repo's own rule puts assembled behaviour in `tests/` and reserves
//! inline modules for tests that reach private items — marking this
//! `behavioral-local` to keep it where it was would have been a false claim
//! about why it was there.
//!
//! ⚠ named `causal_it` so further causal tests land in THIS binary rather than
//! adding another link step per file. Rust links one integration-test
//! executable per top-level `tests/*.rs`, and four crates in this workspace
//! were paying that per file until earlier today.

use ambition_causal::{
    CausalFact, CausalRecording, Execution, FactDetail, RecordingPolicy, SubjectKey, domains,
};
use ambition_runtime::causal::{
    CausalPlugin, RecordingSet, assert_no_offthread_loss, record_domains,
    record_execution_identity, stamp_causal_frame,
};
use bevy::prelude::*;


fn app() -> App {
    let mut app = App::new();
    app.add_plugins(CausalPlugin);
    app.insert_resource(ambition_time::SimTick(0));
    // The head-of-schedule stamp, exactly where `player_schedule` installs
    // it in a real host: BEFORE anything publishes. Putting it after was the
    // bug the parallel proof caught.
    app.add_systems(
        Update,
        stamp_causal_frame.before(RecordingSet::Publish),
    );
    app.configure_sets(Update, RecordingSet::Publish);
    app
}

#[test]
fn an_original_tick_and_a_resimulated_one_are_different_facts() {
    use ambition_platformer_primitives::schedule::SimulationReplayState;

    let mut app = app();
    record_domains(&mut app, RecordingPolicy::All);
    app.insert_resource(SimulationReplayState {
        replaying_history: false,
    });
    app.add_systems(Update, record_execution_identity.in_set(RecordingSet::Publish));
    app.world_mut().resource_mut::<ambition_time::SimTick>().0 = 7;
    app.update();

    // Any subject: the fact is about the world, so it explains every body.
    let anybody = SubjectKey::Sim("fighter_1".into());
    let explanation = app.world().resource::<CausalRecording>().explain(7, &anybody);
    assert_eq!(
        explanation.execution(),
        Some(Execution::Original),
        "a tick nobody replayed is original"
    );

    app.world_mut()
        .resource_mut::<SimulationReplayState>()
        .replaying_history = true;
    app.world_mut().resource_mut::<ambition_time::SimTick>().0 = 8;
    app.update();
    let explanation = app.world().resource::<CausalRecording>().explain(8, &anybody);
    assert_eq!(
        explanation.execution(),
        Some(Execution::Resimulated),
        "a replayed tick says so — the thing the old text trace explicitly could not"
    );
    assert_no_offthread_loss();
}

/// **THE PARALLEL-SCHEDULE PROOF.** (GPT 5.6 review, requested explicitly.)
///
/// The concern is real and was worth proving rather than reasoning about:
/// `ambition_causal`'s thread-local sink cannot collect from Bevy's worker
/// threads, so if the ECS path shared that mechanism the whole inspector
/// integration would be silently lossy.
///
/// It does not share it. A system publishes through
/// `ResMut<CausalRecording>`, and that is sound for a reason stronger than
/// "it happens to work": Bevy will not run two systems with conflicting
/// resource access concurrently, so the publishes are SERIALISED by the
/// scheduler itself and ordered by it deterministically.
///
/// This drives several publishers with disjoint component access — the case
/// most likely to be parallelised — under a real `App::update()`, and
/// asserts every fact arrived AND that nothing went to the sink.
#[test]
fn facts_survive_a_parallel_schedule() {
    #[derive(bevy::prelude::Component)]
    struct Alpha;
    #[derive(bevy::prelude::Component)]
    struct Beta;
    #[derive(bevy::prelude::Component)]
    struct Gamma;

    macro_rules! publisher {
        ($name:ident, $marker:ty, $kind:literal, $seat:expr) => {
            fn $name(
                mut log: ResMut<CausalRecording>,
                // Disjoint component access, so the scheduler is free to
                // consider these for parallel execution.
                q: Query<&$marker>,
            ) {
                for _ in &q {
                    log.record(
                        CausalFact::new(
                            domains::MOVEMENT,
                            0,
                            FactDetail::new($kind, $kind),
                        )
                        .about(ambition_causal::SubjectKey::Seat($seat)),
                    );
                }
            }
        };
    }
    publisher!(publish_alpha, Alpha, "alpha", 0);
    publisher!(publish_beta, Beta, "beta", 0);
    publisher!(publish_gamma, Gamma, "gamma", 0);

    ambition_causal::reset_lost_offthread();
    let mut app = app();
    record_domains(&mut app, RecordingPolicy::All);
    for _ in 0..8 {
        app.world_mut().spawn(Alpha);
        app.world_mut().spawn(Beta);
        app.world_mut().spawn(Gamma);
    }
    app.add_systems(
        Update,
        (publish_alpha, publish_beta, publish_gamma).in_set(RecordingSet::Publish),
    );
    app.world_mut().resource_mut::<ambition_time::SimTick>().0 = 5;
    app.update();

    let log = app.world().resource::<CausalRecording>();
    let explanation = log.explain(5, &ambition_causal::SubjectKey::Seat(0));
    for kind in ["alpha", "beta", "gamma"] {
        assert_eq!(
            explanation.all(kind).count(),
            8,
            "every `{kind}` fact reached ONE coherent explanation — a thread-local \
             collector would have dropped whichever ran off the main thread"
        );
    }
    assert_no_offthread_loss();
}

/// **THE COMPOSITION CLAIM: several domains, ONE explanation.**
///
/// Every publisher has its own test proving it publishes. None of them
/// proved they COMPOSE — that a movement fact, a damage fact and a
/// lifecycle fact about the same body on the same tick arrive as one
/// coherent answer rather than three unrelated logs. That is the whole
/// premise of the inspector, and it is the one thing per-domain tests
/// cannot show.
///
/// It runs the real publishers, in the real order the sim schedule uses:
/// stamp first, then everything else.
#[test]
fn three_domains_answer_one_question_about_one_body_on_one_tick() {
    use ambition_actors::features::ecs::damage_apply::{
        BodyHitResolution, BodyHitResolved,
    };
    use ambition_characters::brain::{Brain, PlayerSlot};
    use ambition_combat::stocks::FighterStockSpent;

    let mut app = app();
    record_domains(&mut app, RecordingPolicy::All);
    app.add_message::<BodyHitResolved>();
    app.add_message::<ambition_combat::stocks::BodyKnockedOut>();
    app.add_message::<FighterStockSpent>();
    app.add_message::<ambition_combat::stocks::StocksMatchDecided>();
    app.add_systems(
        Update,
        (
            // The host's own fact, and the three domains', in the set the
            // sim schedule puts them in — after the stamp.
            record_execution_identity,
            ambition_actors::causal::record_hit_resolutions,
            ambition_combat::causal::record_stock_lifecycle,
            ambition_actors::causal::record_player_movement_intent,
        )
            .in_set(RecordingSet::Publish),
    );

    // ONE body, seated. Every domain will key on the same seat.
    let body = app
        .world_mut()
        .spawn((
            ambition_actors::avatar::movement_components::BodyKinematics::default(),
            ambition_actors::avatar::movement_components::BodyGroundState::default(),
            Brain::Player(PlayerSlot(1)),
            ambition_characters::brain::ActorControl::default(),
        ))
        .id();

    app.world_mut().resource_mut::<ambition_time::SimTick>().0 = 60;
    app.world_mut().write_message(BodyHitResolved {
        body,
        resolution: BodyHitResolution::Damaged {
            damage: 40,
            died: true,
        },
        source: ambition_combat::HitSource::LeftTheWorld,
        raw_damage: 40,
    });
    app.world_mut().write_message(FighterStockSpent {
        body,
        remaining: 0,
        eliminated: true,
    });
    app.update();

    let why = app
        .world()
        .resource::<CausalRecording>()
        .explain(60, &SubjectKey::Seat(1));

    // The three answers, in one explanation.
    assert!(
        why.first("movement_intent").is_some(),
        "movement: what the body was asking for"
    );
    assert!(
        why.first("hit_resolved").is_some(),
        "damage: what the hit did"
    );
    assert!(
        why.first("stock_spent").is_some(),
        "lifecycle: what it cost"
    );
    assert!(
        why.first("tick_execution").is_some(),
        "and the world's own answer about whether this tick was a replay"
    );

    // ⚠ ONE tick and ONE execution across all of them. Domains that stamped
    // their own clock would produce four explanations of four moments, which
    // is the failure that makes a multi-domain log unreadable.
    assert!(
        why.facts().iter().all(|fact| fact.tick == 60),
        "every domain used the tick the HOST stamped: {:?}",
        why.facts().iter().map(|f| (f.kind(), f.tick)).collect::<Vec<_>>()
    );
    assert_eq!(why.execution(), Some(Execution::Original));
    assert_no_offthread_loss();

    println!("{}", why.render());
}

#[test]
fn recording_off_is_the_default_and_costs_nothing() {
    let mut app = app();
    app.update();
    assert!(
        app.world().resource::<CausalRecording>().is_empty(),
        "installing the plugin makes recording POSSIBLE, never automatic"
    );
}

#[test]
fn the_tick_the_host_stamps_is_the_tick_the_facts_carry() {
    let mut app = app();
    record_domains(&mut app, RecordingPolicy::All);
    app.world_mut().resource_mut::<ambition_time::SimTick>().0 = 41;
    app.update();
    let log = app.world().resource::<CausalRecording>();
    assert!(
        log.facts().all(|fact| fact.tick == 41),
        "one clock — a domain that guessed its own would be unjoinable"
    );
}

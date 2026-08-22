use super::*;
use crate::fact::domains;

fn fact(kind: &'static str, tick: u64, domain: CausalDomain) -> CausalFact {
    CausalFact::new(domain, tick, FactDetail::new(kind, kind))
}

fn body() -> SubjectKey {
    SubjectKey::Sim("fighter_1".into())
}

fn other() -> SubjectKey {
    SubjectKey::Sim("fighter_2".into())
}

fn recording_log() -> CausalLog {
    let mut log = CausalLog::default();
    log.set_policy(RecordingPolicy::All);
    log
}

#[test]
fn a_log_that_is_not_recording_costs_nothing_and_keeps_nothing() {
    // The sink counters are PROCESS globals and tests run in
    // parallel; see `global_sink_test_lock`.
    let _serialised = crate::sink::global_sink_test_lock();
    // The shipped default. An instrument that is on by default is an
    // instrument somebody turns off, and then it is not there when needed.
    let mut log = CausalLog::default();
    assert!(!log.is_recording());
    assert_eq!(log.record(fact("moved", 1, domains::MOVEMENT)), None);
    assert!(log.is_empty());
}

#[test]
fn a_policy_admits_only_the_domains_under_investigation() {
    // The sink counters are PROCESS globals and tests run in
    // parallel; see `global_sink_test_lock`.
    let _serialised = crate::sink::global_sink_test_lock();
    let mut log = CausalLog::default();
    log.set_policy(RecordingPolicy::only([domains::MOVEMENT]));
    assert!(log.record(fact("moved", 1, domains::MOVEMENT)).is_some());
    assert!(
        log.record(fact("hit", 1, domains::DAMAGE)).is_none(),
        "the expensive domains are rarely the ones being investigated"
    );
    assert_eq!(log.len(), 1);
}

#[test]
fn explaining_a_tick_gathers_this_subject_and_the_world_but_not_another_body() {
    // The sink counters are PROCESS globals and tests run in
    // parallel; see `global_sink_test_lock`.
    let _serialised = crate::sink::global_sink_test_lock();
    let mut log = recording_log();
    log.record(fact("chose", 10, domains::BRAIN).about(body()));
    log.record(fact("moved", 10, domains::MOVEMENT).about(body()));
    log.record(fact("chose", 10, domains::BRAIN).about(other()));
    log.record(fact("moved", 11, domains::MOVEMENT).about(body()));
    // A fact with NO subject is about the world — a rebase, a rules change —
    // and it explains every body on that tick.
    log.record(fact("rebased", 10, domains::ROLLBACK));

    let explanation = log.explain(10, &body());
    let kinds: Vec<&str> = explanation.facts().iter().map(CausalFact::kind).collect();
    assert_eq!(kinds, vec!["chose", "moved", "rebased"]);
    assert!(
        !explanation
            .facts()
            .iter()
            .any(|f| f.subject == Some(other())),
        "another body's decision is not this body's explanation"
    );
}

#[test]
fn a_composition_with_no_combat_still_explains_its_movement() {
    // The sink counters are PROCESS globals and tests run in
    // parallel; see `global_sink_test_lock`.
    let _serialised = crate::sink::global_sink_test_lock();
    // The stated requirement: "the inspector should tolerate missing domains.
    // A movement-only consumer should not require combat traces."
    let mut log = CausalLog::default();
    log.set_policy(RecordingPolicy::only([domains::MOVEMENT]));
    log.record(
        fact("body_moved", 7, domains::MOVEMENT)
            .about(body())
            .field("dx", 4.5_f32),
    );

    let explanation = log.explain(7, &body());
    assert_eq!(explanation.facts().len(), 1);
    assert_eq!(explanation.domain(domains::DAMAGE).count(), 0);
    assert!(
        explanation.first("hit_accepted").is_none(),
        "asking a question no installed domain answers returns nothing, not an error"
    );
    assert_eq!(
        explanation.first("body_moved").and_then(|f| f.get("dx")),
        Some(&FactValue::Float(4.5))
    );
}

#[test]
fn an_empty_explanation_says_which_of_the_two_reasons_it_is() {
    let log = recording_log();
    let explanation = log.explain(3, &body());
    assert!(explanation.is_empty());
    assert!(
        explanation
            .render()
            .contains("no domain that would know is recording"),
        "«nothing happened» and «nobody was watching» are different answers and must not \
         render identically:\n{}",
        explanation.render()
    );
}

#[test]
fn the_chain_walks_causes_back_to_the_root() {
    // The sink counters are PROCESS globals and tests run in
    // parallel; see `global_sink_test_lock`.
    let _serialised = crate::sink::global_sink_test_lock();
    let mut log = recording_log();
    let input = log
        .record(fact("action_pressed", 5, domains::INPUT).about(body()))
        .expect("recorded");
    let decision = log
        .record(
            fact("move_scored", 5, domains::BRAIN)
                .about(body())
                .caused_by(input),
        )
        .expect("recorded");
    log.record(
        fact("playback_began", 5, domains::MOVESET)
            .about(body())
            .caused_by(decision)
            .from_content("ambition:move/heavy-smash"),
    );

    let explanation = log.explain(5, &body());
    let began = explanation.first("playback_began").expect("the last link");
    let chain: Vec<&str> = explanation
        .chain_to(began)
        .into_iter()
        .map(CausalFact::kind)
        .collect();
    assert_eq!(
        chain,
        vec!["action_pressed", "move_scored", "playback_began"],
        "the chain is oldest-first: a press, a decision, a move"
    );
    assert_eq!(
        began.content.as_deref(),
        Some("ambition:move/heavy-smash"),
        "the fact quotes the COMPILER's prepared identity rather than a runtime name"
    );
}

#[test]
fn a_cause_cycle_is_bounded_rather_than_hanging_the_debugger() {
    // The sink counters are PROCESS globals and tests run in
    // parallel; see `global_sink_test_lock`.
    let _serialised = crate::sink::global_sink_test_lock();
    // A malformed publisher is exactly when somebody is debugging, so the
    // debugger must survive it.
    let mut log = recording_log();
    let first = log
        .record(fact("a", 1, domains::BRAIN).about(body()))
        .unwrap();
    let second = log
        .record(fact("b", 1, domains::BRAIN).about(body()).caused_by(first))
        .unwrap();
    // Forge the cycle by hand — no honest publisher can make one, which is why
    // this has to be constructed.
    let mut cyclic = recording_log();
    let mut a = fact("a", 1, domains::BRAIN).about(body());
    a.cause = Some(FactId(1));
    let mut b = fact("b", 1, domains::BRAIN).about(body());
    b.cause = Some(FactId(0));
    cyclic.record(a);
    cyclic.record(b);
    let explanation = cyclic.explain(1, &body());
    let start = explanation.first("b").unwrap();
    let chain = explanation.chain_to(start);
    assert!(chain.len() <= explanation.facts().len() + 1);
    let _ = second;
}

#[test]
fn a_resimulated_tick_is_labelled_and_a_repeat_is_not_a_mystery() {
    // The sink counters are PROCESS globals and tests run in
    // parallel; see `global_sink_test_lock`.
    let _serialised = crate::sink::global_sink_test_lock();
    let mut log = recording_log();
    log.record(
        fact("chose", 40, domains::BRAIN)
            .about(body())
            .in_generation(2)
            .executed(Execution::Original),
    );
    let replayed = log.explain(40, &body());
    assert_eq!(replayed.execution(), Some(Execution::Original));
    assert_eq!(replayed.generation(), Some(2));

    let mut log = recording_log();
    log.record(
        fact("chose", 40, domains::BRAIN)
            .about(body())
            .in_generation(2)
            .executed(Execution::Resimulated),
    );
    assert_eq!(
        log.explain(40, &body()).execution(),
        Some(Execution::Resimulated)
    );
    assert!(log.dump().contains("resim"));
}

#[test]
fn the_ring_is_bounded_and_says_when_it_wrapped() {
    // The sink counters are PROCESS globals and tests run in
    // parallel; see `global_sink_test_lock`.
    let _serialised = crate::sink::global_sink_test_lock();
    let mut log = CausalLog::with_capacity(4);
    log.set_policy(RecordingPolicy::All);
    for tick in 0..10 {
        log.record(fact("moved", tick, domains::MOVEMENT).about(body()));
    }
    assert_eq!(log.len(), 4, "bounded retained history");
    assert_eq!(log.dropped(), 6);
    let explanation = log.explain(9, &body());
    assert!(
        explanation.truncated,
        "a gap the BUFFER caused must be distinguishable from a gap the simulation caused"
    );
    assert!(log.dump().contains("fell off the ring"));
}

#[test]
fn the_dump_is_deterministic_and_ordered_by_tick_then_record_order() {
    // The sink counters are PROCESS globals and tests run in
    // parallel; see `global_sink_test_lock`.
    let _serialised = crate::sink::global_sink_test_lock();
    let mut a = recording_log();
    let mut b = recording_log();
    for log in [&mut a, &mut b] {
        log.record(fact("second", 2, domains::MOVEMENT).about(body()));
        log.record(fact("first", 1, domains::BRAIN).about(body()));
        log.record(fact("third", 2, domains::DAMAGE).about(body()));
    }
    assert_eq!(a.dump(), b.dump(), "two runs, one dump");
    let dump = a.dump();
    let lines: Vec<&str> = dump.lines().collect();
    assert!(lines[0].contains("first") && lines[0].contains("t1"));
    assert!(
        lines[1].contains("second") && lines[2].contains("third"),
        "within a tick, the order things happened — not a sort by domain name:\n{}",
        dump
    );
}

#[test]
fn the_scoped_sink_collects_what_pure_code_publishes_and_is_inert_otherwise() {
    // The sink counters are PROCESS globals and tests run in
    // parallel; see `global_sink_test_lock`.
    let _serialised = crate::sink::global_sink_test_lock();
    // Publishing from code with no access to the log is the whole point: this
    // stands in for the fighter's decision, five hops below any ECS system.
    fn deep_pure_code(verb: &str) {
        record(
            CausalFact::new(
                domains::BRAIN,
                12,
                FactDetail::new("movement_verb_chosen", format!("chose {verb}")),
            )
            .about(SubjectKey::Sim("fighter_1".into()))
            .field("verb", verb),
        );
    }

    // Outside a scope it is a no-op — no panic, no allocation, nothing kept.
    assert!(!recording());
    deep_pure_code("Approach");

    let (log, ()) = with_sink(recording_log(), || deep_pure_code("Retreat"));
    let explanation = log.explain(12, &body());
    assert_eq!(
        explanation
            .first("movement_verb_chosen")
            .and_then(|f| f.get("verb")),
        Some(&FactValue::Text("Retreat".into())),
        "the verb is a FIELD, not a substring of a sentence a tool would have to parse"
    );

    // And the scope closed: the thread is inert again.
    assert!(!recording());
}

#[test]
fn a_nested_scope_does_not_leak_into_the_outer_dump() {
    // The sink counters are PROCESS globals and tests run in
    // parallel; see `global_sink_test_lock`.
    let _serialised = crate::sink::global_sink_test_lock();
    let (outer, inner) = with_sink(recording_log(), || {
        record(fact("outer", 1, domains::BRAIN).about(body()));
        let (inner, ()) = with_sink(recording_log(), || {
            record(fact("inner", 1, domains::BRAIN).about(body()));
        });
        inner
    });
    assert_eq!(outer.len(), 1, "the outer scope kept only its own fact");
    assert_eq!(inner.len(), 1);
    assert_eq!(outer.facts().next().unwrap().kind(), "outer");
    assert_eq!(inner.facts().next().unwrap().kind(), "inner");
}

/// Facts published off the sink's thread are counted as lost.
#[test]
fn a_fact_published_off_thread_is_counted_rather_than_vanishing() {
    // The sink diagnostics are process-global, so serialize this test.
    let _serialised = crate::sink::global_sink_test_lock();
    reset_lost_offthread();
    let before = facts_lost_offthread();

    let (log, ()) = with_sink(recording_log(), || {
        record(fact("here", 1, domains::BRAIN).about(body()));
        std::thread::scope(|scope| {
            scope.spawn(|| {
                // Worker thread has no sink of its own.
                record(fact("elsewhere", 1, domains::BRAIN).about(body()));
            });
        });
    });

    assert_eq!(log.len(), 1, "only the same-thread fact was collected");
    assert_eq!(log.facts().next().unwrap().kind(), "here");
    assert_eq!(
        facts_lost_offthread(),
        before + 1,
        "and the one that got away is a NUMBER, not a silence"
    );

    // Publishing with instrumentation disabled is not an off-thread loss.
    let after_scope = facts_lost_offthread();
    record(fact("nobody_listening", 1, domains::BRAIN));
    assert_eq!(facts_lost_offthread(), after_scope);
}

/// Equal tick numbers in different lifecycle generations produce separate explanations.
#[test]
fn one_tick_in_two_generations_is_two_explanations() {
    // The sink counters are PROCESS globals and tests run in
    // parallel; see `global_sink_test_lock`.
    let _serialised = crate::sink::global_sink_test_lock();
    let mut log = recording_log();
    log.record(
        fact("chose", 20, domains::BRAIN)
            .about(body())
            .in_generation(1)
            .field("verb", "Approach"),
    );
    log.record(
        fact("chose", 20, domains::BRAIN)
            .about(body())
            .in_generation(2)
            .field("verb", "Retreat"),
    );

    let all = log.explanations(20, &body());
    assert_eq!(all.len(), 2, "one per generation");
    assert_eq!(all[0].generation(), Some(1));
    assert_eq!(all[1].generation(), Some(2));
    assert_eq!(
        all[0].facts().len(),
        1,
        "and neither borrows the other's fact"
    );

    // The single-answer query selects the latest generation.
    let latest = log.explain(20, &body());
    assert_eq!(latest.generation(), Some(2));
    assert_eq!(
        latest.first("chose").and_then(|f| f.get("verb")),
        Some(&FactValue::Text("Retreat".into())),
        "a stale generation's decision must not be reported as this one's"
    );
    assert!(latest.render().contains("generation 2"));
}

/// Original and resimulated executions of one tick produce separate explanations.
#[test]
fn an_original_tick_and_its_resimulation_do_not_share_an_explanation() {
    // The sink counters are PROCESS globals and tests run in
    // parallel; see `global_sink_test_lock`.
    let _serialised = crate::sink::global_sink_test_lock();
    let mut log = recording_log();
    log.record(
        fact("moved", 20, domains::MOVEMENT)
            .about(body())
            .executed(Execution::Original)
            .field("dx", 4.0_f32),
    );
    log.record(
        fact("moved", 20, domains::MOVEMENT)
            .about(body())
            .executed(Execution::Resimulated)
            .field("dx", 9.0_f32),
    );

    let all = log.explanations(20, &body());
    assert_eq!(all.len(), 2);
    assert_eq!(all[0].execution(), Some(Execution::Original));
    assert_eq!(all[1].execution(), Some(Execution::Resimulated));
    assert_eq!(
        all[0].first("moved").and_then(|f| f.get("dx")),
        Some(&FactValue::Float(4.0)),
        "the original execution keeps its own numbers"
    );
    assert_eq!(
        all[1].first("moved").and_then(|f| f.get("dx")),
        Some(&FactValue::Float(9.0)),
        "and the replay keeps its own — a merge would have reported one of them for both"
    );

    // The single-answer query selects the latest execution.
    let latest = log.explain(20, &body());
    assert_eq!(latest.execution(), Some(Execution::Resimulated));
    assert_eq!(latest.facts().len(), 1);
}

/// Two resimulations of the same tick are two explanations, not one.
///
///  the key was `(generation, execution)`, so every replay of tick 120 inside
/// one session generation collapsed together. Rollback executes a tick more than
/// once routinely, those attempts can produce DIFFERENT facts, and that
/// disagreement is exactly when somebody opens an inspector — at which point it
/// could not say which attempt produced a result
///
///  an ORIGINAL execution is always attempt 0. Numbering it by how many
/// rollbacks happened to precede it would make one unchanged original tick
/// answer to a different key depending on unrelated history.
#[test]
fn two_resimulations_of_one_tick_are_two_explanations() {
    // The sink counters are PROCESS globals and tests run in
    // parallel; see `global_sink_test_lock`.
    let _serialised = crate::sink::global_sink_test_lock();
    let mut log = recording_log();
    log.set_tick(120);

    // The original pass.
    log.set_frame_attempt(Execution::Original, 1, 0);
    log.record(fact("chose", 120, domains::BRAIN).about(body()));

    // Two separate rollback batches replay the same tick, disagreeing.
    log.set_frame_attempt(Execution::Resimulated, 1, 1);
    log.record(fact("chose_again", 120, domains::BRAIN).about(body()));
    log.set_frame_attempt(Execution::Resimulated, 1, 2);
    log.record(fact("chose_differently", 120, domains::BRAIN).about(body()));

    let explanations = log.explanations(120, &body());
    assert_eq!(
        explanations.len(),
        3,
        "one original and TWO resimulations are three answers; keys: {:?}",
        explanations.iter().map(|e| e.key).collect::<Vec<_>>()
    );

    let attempts: Vec<u32> = explanations.iter().map(|e| e.key.attempt).collect();
    assert_eq!(
        attempts,
        vec![0, 1, 2],
        "each attempt is separately addressable, so a query can name the one it \
         means"
    );
    assert!(
        explanations
            .iter()
            .any(|e| e.key.execution == Execution::Original && e.key.attempt == 0),
        "the original execution is attempt 0: {:?}",
        explanations.iter().map(|e| e.key).collect::<Vec<_>>()
    );
    // And the facts did not merge: each attempt carries its own.
    for (explanation, kind) in
        explanations
            .iter()
            .zip(["chose", "chose_again", "chose_differently"])
    {
        assert!(
            explanation.first(kind).is_some(),
            "attempt {} should carry `{kind}`",
            explanation.key.attempt
        );
    }
}

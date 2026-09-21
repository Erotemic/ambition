//! these tests build their own `App`, which is exactly the shape that can
//! pass with the production wiring absent. They are about the CONTRACT — its
//! arity checking, its refusals, its ordering — and the contract is what they
//! supply. The claim that real domains publish real conditions is proved by the
//! integration fixture that drives the composed app, not here.

use super::*;
use bevy::prelude::*;

const OCCURRENCE: ParamSpec = ParamSpec {
    name: "occurrence",
    kind: ParamKind::Reference,
    summary: "the occurrence being asked about",
};

/// A marker a test evaluator can look for, standing in for whatever real state a
/// domain would read.
#[derive(Component)]
struct Carried;

fn is_carried(world: &World, args: &[AuthoredArg]) -> ConditionOutcome {
    let Some(wanted) = args[0].as_reference() else {
        return ConditionOutcome::unanswerable("argument was not a reference");
    };
    let Some(mut query) = world.try_query::<(&SimId, Option<&Carried>)>() else {
        return ConditionOutcome::unanswerable("nothing in this world carries anything");
    };
    let found = query
        .iter(world)
        .any(|(sim_id, carried)| sim_id == wanted && carried.is_some());
    ConditionOutcome::from_bool_unexplained(found)
}

fn descriptor(domain: &str, question: &str) -> ConditionDescriptor {
    ConditionDescriptor {
        id: ConditionId::new(domain, question),
        summary: "a test condition",
        params: &[OCCURRENCE],
    }
}

/// A CRATE THAT IS NOT THE ENGINE CAN PUBLISH A CONDITION.
///
/// this is the milestone's behavioral acceptance, and it is a real test
/// rather than a review opinion. This file lives in the library's own test
/// module, but it names no other domain, edits no enum, and touches no
/// registration table — it calls `publish_condition` and nothing else. If a
/// central list of condition kinds existed, this could not compile.
#[test]
fn a_provider_that_names_no_other_domain_can_publish_and_be_asked() {
    let mut app = App::new();
    app.publish_condition(descriptor("bystander", "is_carried"), is_carried);

    let held = app
        .world_mut()
        .spawn((SimId::placement("axe"), Carried))
        .id();
    let _ = held;
    app.world_mut().spawn(SimId::placement("rock"));

    let catalog = app.world().resource::<ConditionCatalog>().clone();
    let id = ConditionId::new("bystander", "is_carried");

    assert_eq!(
        catalog.evaluate(
            app.world(),
            &id,
            &[AuthoredArg::Reference(SimId::placement("axe"))],
            &crate::authored_logic::AuthoredAsk::new("probe", "a test")),
        ConditionOutcome::Satisfied
    );
    assert!(matches!(
        catalog.evaluate(
            app.world(),
            &id,
            &[AuthoredArg::Reference(SimId::placement("rock"))],
            &crate::authored_logic::AuthoredAsk::new("probe", "a test")),
        ConditionOutcome::NotSatisfied(_)
    ));
}

/// TWO domains coexist and each is discoverable on its own.
#[test]
fn the_catalog_composes_domains_without_either_naming_the_other() {
    let mut app = App::new();
    app.publish_condition(descriptor("custody", "is_carried"), is_carried);
    app.publish_condition(descriptor("weather", "is_carried"), is_carried);

    let catalog = app.world().resource::<ConditionCatalog>();
    assert_eq!(catalog.len(), 2);
    assert_eq!(
        catalog
            .describe_all()
            .map(|d| d.id.to_string())
            .collect::<Vec<_>>(),
        ["custody.is_carried", "weather.is_carried"],
        "the listing is id-ordered, so a diagnostic that prints it is stable"
    );
    assert_eq!(catalog.describe_domain("weather").count(), 1);
}

/// AN UNANSWERABLE CONDITION IS NOT A FALSE ONE.
///
/// the failure this pins is folding the third answer into `false`: a gate that
/// opens on the negation of *"is the key held?"* would swing open in a world that
/// never authored a key at all.
#[test]
fn asking_an_unpublished_condition_is_unanswerable_rather_than_false() {
    let app = App::new();
    let catalog = ConditionCatalog::default();
    let outcome = catalog.evaluate(app.world(), &ConditionId::new("nobody", "cares"), &[], &crate::authored_logic::AuthoredAsk::new("probe", "a test"));
    assert!(matches!(outcome, ConditionOutcome::Unanswerable(_)));
    assert!(!outcome.is_satisfied());
}

/// ARITY AND KIND ARE CHECKED ONCE, CENTRALLY, AND THE REASON NAMES THE
/// PARAMETER.
///
/// a diagnostic that said only "bad arguments" would make every authoring
/// mistake a debugging session; the schema is right there, so the message uses it.
#[test]
fn a_mistyped_argument_is_refused_with_a_reason_an_author_can_act_on() {
    let mut app = App::new();
    app.publish_condition(descriptor("custody", "is_carried"), is_carried);
    let catalog = app.world().resource::<ConditionCatalog>().clone();
    let id = ConditionId::new("custody", "is_carried");

    let ConditionOutcome::Unanswerable(too_few) = catalog.evaluate(app.world(), &id, &[], &crate::authored_logic::AuthoredAsk::new("probe", "a test")) else {
        panic!("no arguments must be refused");
    };
    assert!(too_few.contains("occurrence"), "{too_few}");

    let ConditionOutcome::Unanswerable(wrong_kind) =
        catalog.evaluate(app.world(), &id, &[AuthoredArg::Name("axe".to_string())], &crate::authored_logic::AuthoredAsk::new("probe", "a test"))
    else {
        panic!("a Name where a Reference belongs must be refused");
    };
    assert!(wrong_kind.contains("occurrence"), "{wrong_kind}");
}

/// TWO DOMAINS CANNOT OWN ONE ID, AND IT FAILS AT STARTUP.
///
/// the alternative is that the winner is whichever plugin built last — a bug
/// that appears only when a host changes its plugin order, which is the worst
/// time to discover it.
#[test]
#[should_panic(expected = "already published")]
fn publishing_one_id_twice_panics_rather_than_letting_the_last_plugin_win() {
    let mut app = App::new();
    app.publish_condition(descriptor("custody", "is_carried"), is_carried);
    app.publish_condition(descriptor("custody", "is_carried"), is_carried);
}

/// AN ID CANNOT BE SPELLED TWO WAYS.
#[test]
#[should_panic(expected = "may not appear inside one")]
fn a_dot_inside_a_segment_is_refused() {
    let _ = ConditionId::new("custody", "is.carried");
}

/// AUTHORED CONTENT NAMES A CONDITION BY STRING, AND A TYPO IS A DIAGNOSTIC
/// RATHER THAN A PANIC.
#[test]
fn an_id_read_back_from_authored_text_refuses_instead_of_panicking() {
    assert_eq!(
        ConditionId::parse("world.flag_set"),
        Some(ConditionId::new("world", "flag_set"))
    );
    // every one of these would have PANICKED through `new`.
    assert_eq!(ConditionId::parse("flag_set"), None, "no domain at all");
    assert_eq!(ConditionId::parse(".flag_set"), None, "empty domain");
    assert_eq!(ConditionId::parse("world."), None, "empty question");
    assert_eq!(ConditionId::parse("a.b.c"), None, "ambiguous segments");
    assert_eq!(ConditionId::parse(""), None);
    // and it never repairs. A leading space parses — the shape is legal — but
    // it parses to a DIFFERENT id, which is what makes the lookup miss and the
    // author see a diagnostic naming their own spelling.
    assert_ne!(
        ConditionId::parse(" world.flag_set"),
        Some(ConditionId::new("world", "flag_set")),
        "trimming would make the authored name and the published name two \
         spellings of one id, which is the collision `new` asserts against"
    );
}

/// ⛔ **THE RING'S BOUND IS THE PART A LEAK HIDES IN, AND NOTHING ELSE
/// EXERCISES IT.** A gated wall asks its condition every sync, so a log that
/// grows is a leak measured in ticks rather than in allocations, and it would
/// look exactly like a working log for the whole of a test run.
#[test]
fn the_verdict_ring_forgets_its_oldest_answer_rather_than_growing() {
    let log = AuthoredVerdictLog::with_capacity(3);
    let verdict = |n: u32| ConditionVerdict {
        id: ConditionId::new("test", "counted"),
        args: vec![AuthoredArg::Number(f64::from(n))],
        asked_by: crate::authored_logic::AuthoredAsk::new("probe", "a test"),
        outcome: ConditionOutcome::Satisfied,
        stamp: VerdictStamp::default(),
    };
    for n in 0..10 {
        log.record(AuthoredVerdict::Asked(verdict(n)));
    }
    assert_eq!(log.len(), 3);
    let kept: Vec<AuthoredArg> = log
        .recent()
        .into_iter()
        .map(|entry| match entry {
            AuthoredVerdict::Asked(v) => v.args[0].clone(),
            AuthoredVerdict::Ran(v) => v.args[0].clone(),
        })
        .collect();
    assert_eq!(
        kept,
        vec![
            AuthoredArg::Number(7.0),
            AuthoredArg::Number(8.0),
            AuthoredArg::Number(9.0)
        ],
        "the ring kept the wrong end: `recent` is oldest-first over what \
         SURVIVED, and dropping the newest would make a diagnostic that goes \
         blind exactly when something starts asking a lot of questions"
    );
    // ⚠ AND A CAPACITY OF ZERO IS A CAPACITY OF ONE, not a log that silently
    // records nothing and not one that grows without a bound.
    //
    // ⛔⛤ **THIS ARM RECORDED ONCE UNTIL A POISON PASSED THROUGH IT.**
    // Deleting the `max(1)` left the test green: with a capacity of zero the
    // first record finds `len() == capacity`, pops an EMPTY deque, and pushes,
    // so one entry is exactly what a broken bound produces too. The second
    // record is where the two stories part — `1 == 0` is false, nothing is
    // evicted, and the ring grows forever. A bound has to be probed past the
    // bound.
    let degenerate = AuthoredVerdictLog::with_capacity(0);
    for n in 0..5 {
        degenerate.record(AuthoredVerdict::Asked(verdict(n)));
    }
    assert_eq!(degenerate.len(), 1);
}

/// ⛔⛤ **`latest_for` IS PER-QUESTION, AND THE OBVIOUS WRONG IMPLEMENTATION —
/// "the last entry, if it happens to match" — PASSES EVERY SINGLE-CONDITION
/// TEST.** The world a diagnostic is read in has a gated wall asking its
/// question every sync, so the newest entry is almost never the one the reader
/// wants.
#[test]
fn the_latest_answer_to_one_question_is_found_behind_other_questions() {
    let log = AuthoredVerdictLog::default();
    let mine = ConditionId::new("test", "mine");
    let noisy = ConditionId::new("test", "noisy");
    log.record(AuthoredVerdict::Asked(ConditionVerdict {
        id: mine.clone(),
        args: vec![],
        asked_by: crate::authored_logic::AuthoredAsk::new("probe", "a test"),
        outcome: ConditionOutcome::NotSatisfied(WhyNot::new("test.mine", "subject", "observed")),
        stamp: VerdictStamp::default(),
    }));
    for _ in 0..20 {
        log.record(AuthoredVerdict::Asked(ConditionVerdict {
            id: noisy.clone(),
            args: vec![],
            asked_by: crate::authored_logic::AuthoredAsk::new("probe", "a test"),
            outcome: ConditionOutcome::Satisfied,
            stamp: VerdictStamp::default(),
        }));
    }
    assert_eq!(
        log.why_not_for(&mine, &[]),
        Some(WhyNot::new("test.mine", "subject", "observed")),
        "the reader's question was buried under a wall's and could not be found"
    );
    assert_eq!(log.why_not_for(&noisy, &[]), None, "a `yes` has no why-not");
    assert_eq!(
        log.why_not_for(&ConditionId::new("test", "never_asked"), &[]),
        None
    );
    log.clear();
    assert!(log.is_empty() && log.why_not_for(&mine, &[]).is_none());
}

/// ⛔⛤ **ONE CONDITION ID IS MANY QUESTIONS, AND THE LOOKUP USED TO ANSWER FOR
/// WHICHEVER WAS ASKED LAST — REVIEWED 2026-09-20.**
///
/// `world.flag_set` is one id with as many subjects as the game has flags;
/// `inventory.holds` one with as many as it has items. So a tick that asks
/// about two doors is ordinary, and an id-keyed `why_not_for` hands somebody
/// investigating the first one the reason the SECOND is shut — in the same
/// units, in the same words, with nothing marking it.
///
/// ⚠ THE ARM THAT EXISTED PROTECTED AGAINST A DIFFERENT ID BURYING THE ENTRY
/// (`test.mine` under twenty `test.noisy`). That is the rarer case. Two
/// subjects of ONE id is the common one, and it was unguarded.
#[test]
fn two_subjects_of_one_condition_are_independently_answerable() {
    let log = AuthoredVerdictLog::default();
    let flag_set = ConditionId::new("world", "flag_set");
    let asked = |door: &str, why: &str| {
        log.record(AuthoredVerdict::Asked(ConditionVerdict {
            id: flag_set.clone(),
            args: vec![AuthoredArg::Name(door.to_string())],
            asked_by: crate::authored_logic::AuthoredAsk::new("probe", "a test"),
            outcome: ConditionOutcome::NotSatisfied(WhyNot::new("world.flag_set", door, why)),
            stamp: VerdictStamp::default(),
        }));
    };
    asked("door_A", "unset");
    asked("door_B", "unset, and nobody has the key either");

    let door = |name: &str| {
        log.why_not_for(&flag_set, &[AuthoredArg::Name(name.to_string())])
            .map(|why| why.observed)
    };
    assert_eq!(door("door_A").as_deref(), Some("unset"));
    assert_eq!(
        door("door_B").as_deref(),
        Some("unset, and nobody has the key either"),
        "the second subject's own reason is not retrievable"
    );
    assert_eq!(
        door("door_C"),
        None,
        "a subject nobody asked about answered with somebody else's reason"
    );
    // ⚠ AND THE BROWSING HELPER STILL ANSWERS THE COARSE QUESTION, honestly
    // named. Deleting it would push a reader who genuinely wants "did anybody
    // ask this at all" into reconstructing it from `recent()`.
    assert_eq!(
        log.latest_for_id(&flag_set).map(|v| v.args),
        Some(vec![AuthoredArg::Name("door_B".to_string())]),
        "`latest_for_id` is the most recent ASKING, whatever its subject"
    );
}

/// ⛔⛤ **A ROLLBACK HOST RE-SIMULATES A FRAME IT GUESSED WRONG ABOUT, AND AN
/// APPEND-ONLY RING TURNS THAT INTO A CONTRADICTORY HISTORY — REVIEWED
/// 2026-09-20.**
///
/// Keeping the ring out of rollback state is right: rewinding the evidence
/// erases what an observer came to read. But without a replacement rule the
/// ring holds a speculative `no` and a corrected `yes` side by side, and
/// *"this rule oscillated"* reads exactly like *"the first prediction was
/// rolled back and never became history"*.
///
/// ⛔⛤ **AND THE FIRST RULE WAS THE WRONG KEY — REVIEWED THE SAME DAY.** It
/// matched `(id, args, frame)` and overwrote, borrowed from
/// `GameplayTraceBuffer`, which holds ONE observation per `(session, frame)`.
/// This stream holds arbitrarily many: the engine runs
/// `world.set_flag("x")` twice from one command buffer, and two authored
/// sources can ask one condition with one argument list in one frame. So the
/// unit of replacement is the FRAME'S WHOLE BATCH, cleared by `begin_pass`
/// before the corrected pass refills it.
#[test]
fn a_corrected_rollback_pass_clears_the_frame_it_is_about_to_redo() {
    let log = AuthoredVerdictLog::default();
    let gate = ConditionId::new("world", "flag_set");
    let record = |door: &str, outcome: ConditionOutcome, frame: i32| {
        log.record(AuthoredVerdict::Asked(ConditionVerdict {
            id: gate.clone(),
            args: vec![AuthoredArg::Name(door.to_string())],
            asked_by: crate::authored_logic::AuthoredAsk::new("probe", "a test"),
            outcome,
            stamp: VerdictStamp {
                simulation: Some((7, frame)),
                confirmed: false,
            },
        }));
    };
    let no = || ConditionOutcome::NotSatisfied(WhyNot::new("world.flag_set", "door_A", "unset"));

    // ⭐ THE SPECULATIVE PASS OVER FRAME 120, AND ONE QUESTION ASKED TWICE.
    // The old rule collapsed those two into one entry and called the second a
    // correction of the first.
    record("door_A", no(), 120);
    record("door_A", no(), 120);
    record("door_B", ConditionOutcome::Satisfied, 120);
    // A neighbouring frame, which the rollback below must NOT touch.
    record("door_A", ConditionOutcome::Satisfied, 119);
    assert_eq!(
        log.len(),
        4,
        "two askings of one question in one frame collapsed into one: {:?}",
        log.recent()
    );

    // The host rewinds and re-simulates 120. This time the gate is open and
    // nobody asks about `door_B` at all.
    assert_eq!(
        log.begin_pass(7, 120),
        3,
        "the new pass over frame 120 did not clear what the abandoned one recorded"
    );
    record("door_A", ConditionOutcome::Satisfied, 120);

    assert_eq!(
        log.len(),
        2,
        "the abandoned prediction is still in the ring beside its correction: {:?}",
        log.recent()
    );
    assert_eq!(
        log.latest_for(&gate, &[AuthoredArg::Name("door_A".to_string())])
            .map(|v| v.outcome),
        Some(ConditionOutcome::Satisfied)
    );
    // ⭐ AND `door_B` IS GONE RATHER THAN STALE. It was asked in a future that
    // was abandoned; a reader finding it there would be reading a question
    // the surviving history never asked.
    assert_eq!(
        log.latest_for(&gate, &[AuthoredArg::Name("door_B".to_string())]),
        None,
        "a question only the abandoned pass asked is still being reported"
    );
    // ⭐ THE CONTROL: the neighbouring frame is untouched.
    assert_eq!(
        log.recent()[0].stamp().simulation,
        Some((7, 119)),
        "clearing frame 120 took frame 119 with it"
    );

    // ⭐ A DIFFERENT SESSION AT THE SAME FRAME NUMBER IS A DIFFERENT PASS.
    assert_eq!(
        log.begin_pass(9, 120),
        0,
        "clearing session 9's frame 120 cleared session 7's"
    );

    // ⭐ AND AN UNSTAMPED VERDICT IS NEVER CLEARED. With no rollback host
    // nothing can be re-simulated, so two identical questions are two
    // askings — collapsing them would hide a rule being hammered every tick.
    let plain = AuthoredVerdictLog::default();
    for _ in 0..3 {
        plain.record(AuthoredVerdict::Asked(ConditionVerdict {
            id: gate.clone(),
            args: vec![],
            asked_by: crate::authored_logic::AuthoredAsk::new("probe", "a test"),
            outcome: ConditionOutcome::Satisfied,
            stamp: VerdictStamp::default(),
        }));
    }
    assert_eq!(plain.len(), 3);
}

/// ⚠ **CONFIRMATION ARRIVES AFTER THE ANSWER DOES**, so an entry recorded on a
/// speculative frame must be able to become settled without being asked again.
/// A reader that only ever saw `confirmed: false` would conclude the engine
/// never settles anything.
#[test]
fn a_frame_the_host_later_confirms_stops_reading_as_a_guess() {
    let log = AuthoredVerdictLog::default();
    let gate = ConditionId::new("world", "flag_set");
    let at = |frame: i32, session: u64| {
        log.record(AuthoredVerdict::Asked(ConditionVerdict {
            id: gate.clone(),
            args: vec![AuthoredArg::Number(f64::from(frame))],
            asked_by: crate::authored_logic::AuthoredAsk::new("probe", "a test"),
            outcome: ConditionOutcome::Satisfied,
            stamp: VerdictStamp {
                simulation: Some((session, frame)),
                confirmed: false,
            },
        }));
    };
    at(10, 1);
    at(11, 1);
    // ⚠ A DIFFERENT SESSION AT THE SAME FRAME NUMBER. Without the session in
    // the key, confirming session 1 would settle a row belonging to a
    // timeline that no longer exists — the exact reason
    // `ConfirmedFrameBoundary` carries a generation.
    at(10, 2);

    log.confirm_through(1, 10);
    let settled: Vec<bool> = log.recent().iter().map(|e| e.stamp().confirmed).collect();
    assert_eq!(
        settled,
        vec![true, false, false],
        "confirmation reached the wrong rows: {:?}",
        log.recent()
    );
}

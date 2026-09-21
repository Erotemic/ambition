//! What can authored content ask this engine, and does asking work?
//!
//! The condition contract's whole claim is that a domain publishes its own
//! questions and nothing central learns they exist. That claim is only worth
//! anything about the composed engine — a contract that works in a hand-built
//! `App` and is wired up nowhere is vocabulary nobody speaks.
//!
//! so this file drives the real host: the real plugin group, the real item
//! domain, the real save layer, a real authored occurrence, and a real pressed
//! pickup. it publishes no condition of its own. The unit tests beside the
//! contract prove a stranger can publish one; this proves the engine actually
//! did.
//!
//! and it is deliberately thin on assertions about WHICH conditions exist.
//! Pinning the full catalog would make every new provider a failing test, which
//! is the opposite of the property being built — new questions are supposed to be
//! cheap. What is pinned is that independent domains are present and that asking
//! them returns real answers about real state.

use ambition_app::{AgentAction, Platformer2dSimHarness};
use ambition_platformer2d::platformer::authored_logic::{
    AuthoredArg, ConditionCatalog, ConditionId, ConditionOutcome,
};
use ambition_platformer2d::platformer::sim_id::SimId;
use std::path::Path;

use ambition_platformer2d::platformer::lifecycle::SessionRoot;
use bevy::prelude::With;

use crate::common::{base, fixed_60hz_room_sim};

const ROOM: &str = "blink_run";

fn catalog(sim: &Platformer2dSimHarness) -> ConditionCatalog {
    sim.world()
        .get_resource::<ConditionCatalog>()
        .expect(
            "the composed engine publishes at least one condition, so the catalog resource exists",
        )
        .clone()
}

fn ask(sim: &Platformer2dSimHarness, id: &ConditionId, args: &[AuthoredArg]) -> ConditionOutcome {
    catalog(sim).evaluate(sim.world(), id, args, &ambition_platformer2d::platformer::authored_logic::AuthoredAsk::new("probe", "a test"))
}

/// TWO INDEPENDENT DOMAINS PUBLISHED QUESTIONS INTO ONE CATALOG.
///
/// neither names the other, and neither is listed anywhere central: the item
/// domain publishes from its own simulation plugin, the world-fact domain from a
/// plugin of its own. What composed them is composition.
#[test]
fn the_composed_engine_publishes_questions_from_more_than_one_domain() {
    let mut sim = fixed_60hz_room_sim(ROOM);
    sim.step_n(base(), 4);
    let catalog = catalog(&sim);

    let domains: Vec<&str> = catalog
        .describe_all()
        .map(|descriptor| descriptor.id.domain())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    assert!(
        domains.len() >= 2,
        "the contract's acceptance is that a SECOND domain costs nothing central; \
         found only {domains:?}"
    );
    assert!(domains.contains(&"custody"), "{domains:?}");
    assert!(domains.contains(&"world"), "{domains:?}");
    // ⭐ AND THE BODY DOMAIN, WHICH IS THE INSTALLABILITY HALF. `body.can` and
    // `body.fits` are published by `BodyCapabilityConditionsPlugin` and unit-
    // tested against hand-built worlds; neither fact says the plugin is
    // COMPOSED. A domain that is written, tested and never added is exactly the
    // shape a route author would find by writing a `gated_by` that never opens,
    // and the composed engine is the only place that can witness it.
    assert!(
        domains.contains(&"body"),
        "`BodyCapabilityConditionsPlugin` is not in the composed engine, so no \
         authored route can gate on what a body can do: {domains:?}"
    );
}

/// EVERY PUBLISHED QUESTION DESCRIBES ITSELF WELL ENOUGH TO BE USED.
///
/// this is the discovery half, and it is an acceptance criterion rather than
/// polish: an agent that can list the questions but cannot tell what they take
/// has to read the engine's source, which is the thing this program exists to
/// stop.
#[test]
fn every_published_question_carries_a_schema_an_agent_could_act_on() {
    let mut sim = fixed_60hz_room_sim(ROOM);
    sim.step_n(base(), 4);
    let catalog = catalog(&sim);

    assert!(!catalog.is_empty(), "nothing published");
    for descriptor in catalog.describe_all() {
        let id = &descriptor.id;
        assert!(!descriptor.summary.is_empty(), "`{id}` has no summary");
        assert!(
            id.as_str().contains('.'),
            "`{id}` is not namespaced by its owning domain"
        );
        for param in descriptor.params {
            assert!(!param.name.is_empty(), "`{id}` has an unnamed parameter");
            assert!(
                !param.summary.is_empty(),
                "`{id}` parameter `{}` has no summary, so an author cannot tell what to pass",
                param.name
            );
        }
    }
}

/// ASKING THE ITEM DOMAIN ABOUT A REAL OCCURRENCE TRACKS REAL STATE.
///
/// the interesting assertion is the THIRD one. Satisfied-then-not is a
/// property any boolean would have; the third answer — *unanswerable* about an
/// identity this world never authored — is what stops a gate that opens on the
/// negation from standing open in a level that has no key.
#[test]
fn the_item_domain_answers_about_custody_and_says_so_when_it_cannot() {
    let mut sim = fixed_60hz_room_sim(ROOM);
    sim.step_n(base(), 8);

    let is_held = ConditionId::new("custody", "is_held");

    // The one authored ground item, found through the same road the checkpoint
    // fixture uses: lying in the world, so nobody has it yet.
    let (authored, at) = {
        let mut query = sim.world_mut().query::<(
            &SimId,
            &ambition_platformer2d::held_items::GroundItem,
            &ambition_platformer2d::held_items::ItemCustody,
        )>();
        let found: Vec<(SimId, (f32, f32))> = query
            .iter(sim.world())
            .filter(|(_, _, custody)| custody.in_world())
            .map(|(id, ground, _)| (id.clone(), (ground.pos.x, ground.pos.y)))
            .collect();
        assert_eq!(found.len(), 1, "'{ROOM}' authors exactly one ground item");
        found[0].clone()
    };

    assert!(
        matches!(
            ask(&sim, &is_held, &[AuthoredArg::Reference(authored.clone())]),
            ConditionOutcome::NotSatisfied(_)
        ),
        "it is lying on the floor"
    );

    // Pick it up through the ordinary pressed pickup.
    sim.teleport_player(at);
    for _ in 0..40 {
        sim.step(AgentAction {
            attack: true,
            ..base()
        });
        sim.step(base());
        if ask(&sim, &is_held, &[AuthoredArg::Reference(authored.clone())])
            == ConditionOutcome::Satisfied
        {
            break;
        }
    }
    assert_eq!(
        ask(&sim, &is_held, &[AuthoredArg::Reference(authored.clone())]),
        ConditionOutcome::Satisfied,
        "the pressed pickup took custody, and the domain says so"
    );

    // the third answer.
    let never_authored = SimId::placement("a_key_this_world_does_not_have");
    let outcome = ask(&sim, &is_held, &[AuthoredArg::Reference(never_authored)]);
    assert!(
        matches!(outcome, ConditionOutcome::Unanswerable(_)),
        "an occurrence this world never authored is UNANSWERABLE, not false — a \
         gate opening on the negation would stand open forever. Got {outcome:?}"
    );
    assert!(
        !outcome.is_satisfied(),
        "and unanswerable must never read as satisfied"
    );
}

/// ⛔⛤ **AN AGENT CAN NOW READ WHY A RULE DID NOT FIRE, WITHOUT A DEBUGGER —
/// THE OPEN HALF OF M5.**
///
/// The `WhyNot` vocabulary landed 2026-09-02 and every production evaluator
/// states one. Exactly ONE consumer published them: `GatedLockWallVerdicts`,
/// keyed by wall id, for the walls of the active room. Every other `no` in the
/// engine — a quest gate, a dialogue branch, an item condition, a boss phase —
/// was built on the tick it was wanted, returned to its one caller, and
/// dropped. The structure existed and was unreadable, which is the worst of
/// both: the cost of building it with none of the benefit.
///
/// ⭐ **AND IT IS DRIVEN THROUGH THE COMPOSED HOST BECAUSE THE RECORDER IS AT
/// THE CATALOG.** A hand-built `App` would prove the ring works. What this
/// asks is whether the engine that actually ships routes its questions through
/// the one door the recorder sits in — including the refusals the catalog
/// issues itself, which no domain ever sees.
#[test]
fn an_unsatisfied_condition_leaves_its_reason_somewhere_an_agent_can_read_it() {
    use ambition_platformer2d::platformer::authored_logic::AuthoredVerdictLog;

    let mut sim = fixed_60hz_room_sim(ROOM);
    sim.step_n(base(), 4);

    // ⚠ THE PREMISE, AND IT IS THE INTERESTING HALF OF THE DESIGN: the log is
    // ABSENT until a composition asks for one, so a shipping build pays a
    // resource lookup and nothing else. Reading it before installing it must
    // find nothing, or "the log has my answer" would be true of a world that
    // never recorded anything.
    assert!(
        sim.world().get_resource::<AuthoredVerdictLog>().is_none(),
        "the composed host installs a diagnostic ring nobody asked for"
    );
    let flag_set = ConditionId::new("world", "flag_set");
    let flag = "a_fact_this_run_has_not_recorded";
    let unrecorded = ask(&sim, &flag_set, &[AuthoredArg::Name(flag.to_string())]);
    assert!(matches!(unrecorded, ConditionOutcome::NotSatisfied(_)));

    sim.world_mut().insert_resource(AuthoredVerdictLog::default());
    assert_eq!(
        ask(&sim, &flag_set, &[AuthoredArg::Name(flag.to_string())]),
        unrecorded,
        "installing the log changed the answer, which would make it an \
         instrument that perturbs its subject"
    );

    // THE M5 QUESTION, asked of the world rather than of a debugger.
    let why = sim
        .world()
        .resource::<AuthoredVerdictLog>()
        .why_not_for(&flag_set, &[AuthoredArg::Name(flag.to_string())])
        .expect("the engine answered no and kept no reason");
    assert_eq!(why.term, "world.flag_set");
    assert_eq!(why.subject, flag);
    assert!(
        !why.observed.is_empty() && why.observed != "<unstated>",
        "the world-fact domain returned the fixture's unexplained `no`: {why:?}"
    );

    // ⭐ AND THE CATALOG'S OWN REFUSALS ARE IN IT, which is the arm that says
    // the recorder is at the door and not inside the domains. A misspelled id
    // reaches no evaluator at all, so a per-domain recorder could not see it —
    // and a misspelled id is exactly what an agent debugging authored content
    // has just typed.
    let nonsense = ConditionId::new("world", "flag_set_maybe");
    assert!(matches!(
        ask(&sim, &nonsense, &[AuthoredArg::Name(flag.to_string())]),
        ConditionOutcome::Unanswerable(_)
    ));
    let refused = sim
        .world()
        .resource::<AuthoredVerdictLog>()
        .latest_for(&nonsense, &[AuthoredArg::Name(flag.to_string())])
        .expect("the catalog refused a question and kept no record of refusing");
    assert!(matches!(refused.outcome, ConditionOutcome::Unanswerable(_)));
    assert!(
        refused.to_string().contains("flag_set_maybe"),
        "a verdict renders without naming what was asked: {refused}"
    );

    // ⛔⛤ **AND THE ANSWER CARRIES THE FRAME IT WAS PRODUCED ON, READ OFF THE
    // HOST'S OWN BOUNDARY — REVIEWED 2026-09-20.** A diagnostic kept out of
    // rollback state still needs rollback IDENTITY: a re-simulated frame
    // answers the same question twice, and without a stamp the ring presents
    // an abandoned prediction and its correction as two equally authoritative
    // entries. The replacement rule itself is witnessed in the crate's own
    // tests; this is about the stamp being FILLED rather than being a field
    // nobody writes.
    //
    // ⛔ **A POISON PASSED THROUGH THE FIRST VERSION OF THIS ARM.** It read
    // whatever boundary the harness happened to have and asserted the stamp
    // matched — and this harness has NONE, so only the absent branch ever ran
    // and breaking the present one was invisible. Both branches have to be
    // taken, which means installing the resource rather than hoping for it.
    use ambition_platformer2d::engine_core::ConfirmedFrameBoundary;
    use ambition_platformer2d::platformer::authored_logic::VerdictStamp;
    let stamp_now = |sim: &Platformer2dSimHarness| {
        sim.world()
            .resource::<AuthoredVerdictLog>()
            .latest_for(&flag_set, &[AuthoredArg::Name(flag.to_string())])
            .expect("the question is in the log")
            .stamp
    };
    assert!(
        sim.world().get_resource::<ConfirmedFrameBoundary>().is_none(),
        "this harness grew a rollback host, so the premise below is stale"
    );
    // ⚠ THE ABSENT CASE IS NOT A HOLE. That resource's own module says an
    // absent boundary means there is no rollback host and frames are
    // confirmed, so an answer produced here happened once and can never be
    // replaced.
    assert_eq!(
        stamp_now(&sim),
        VerdictStamp {
            simulation: None,
            confirmed: true,
        },
        "no rollback host, so the answer should read as settled and \
         unrepeatable"
    );

    sim.world_mut().insert_resource(ConfirmedFrameBoundary {
        current: 314,
        confirmed: 313,
        session: 9,
    });
    let _ = ask(&sim, &flag_set, &[AuthoredArg::Name(flag.to_string())]);
    assert_eq!(
        stamp_now(&sim),
        VerdictStamp {
            simulation: Some((9, 314)),
            confirmed: false,
        },
        "the stamp does not name the frame the host says it is on, so a \
         corrected pass could not find its predecessor — and a guess would \
         read as history"
    );
    // ⭐ AND THE CONFIRMED HALF IS READ, not assumed: the same question on a
    // frame the host has already settled is not a guess.
    sim.world_mut().insert_resource(ConfirmedFrameBoundary {
        current: 314,
        confirmed: 314,
        session: 9,
    });
    let _ = ask(&sim, &flag_set, &[AuthoredArg::Name(flag.to_string())]);
    assert!(
        stamp_now(&sim).confirmed,
        "a settled frame's answer still reads as speculative"
    );
    sim.world_mut().remove_resource::<ConfirmedFrameBoundary>();

    // ⚠ AND `None` FROM `why_not_for` HAS THREE CAUSES. A satisfied condition
    // must not read as "no reason recorded" — without this the arm above
    // passes for a log that only ever remembers failures, and an agent
    // checking a rule that DID fire would be told nothing and conclude the
    // instrument was broken.
    sim.world_mut()
        .resource_mut::<ambition_platformer2d::persistence::save::AmbitionGameSave>()
        .data_mut()
        .set_flag(flag, true);
    assert_eq!(
        ask(&sim, &flag_set, &[AuthoredArg::Name(flag.to_string())]),
        ConditionOutcome::Satisfied
    );
    let log = sim.world().resource::<AuthoredVerdictLog>();
    assert_eq!(
        log.why_not_for(&flag_set, &[AuthoredArg::Name(flag.to_string())]),
        None
    );
    assert!(
        log.latest_for(&flag_set, &[AuthoredArg::Name(flag.to_string())])
            .is_some_and(|v| v.outcome == ConditionOutcome::Satisfied),
        "a satisfied answer left the log, so `why_not_for(..) == None` cannot \
         be told apart from never having been asked"
    );
}

/// ⛔⛤ **THE RING KNEW ABOUT ROLLBACK AND THE HOST NEVER TOLD IT ANYTHING —
/// REVIEWED 2026-09-20.**
///
/// `confirm_through` re-stamps a frame the host has since settled, and its
/// only caller was its own unit test. So in a real rollback host a verdict
/// recorded while speculative stayed speculative forever: the frame settled,
/// nothing told the ring, and the diagnostic reported a real historical event
/// as a guess. The mechanism was right and wired to nothing.
///
/// This drives the COMPOSED engine's simulation schedule rather than calling
/// the two log methods, because "wired to nothing" is exactly what a direct
/// call cannot catch.
///
/// ⚠ THE HARNESS HAS NO ROLLBACK HOST, so the boundary is installed here —
/// the same reason the stamp arm above installs one.
#[test]
fn a_settled_frame_stops_reading_as_a_guess_without_being_asked_again() {
    use ambition_platformer2d::engine_core::ConfirmedFrameBoundary;
    use ambition_platformer2d::platformer::authored_logic::AuthoredVerdictLog;

    let mut sim = fixed_60hz_room_sim(ROOM);
    sim.step_n(base(), 4);
    sim.world_mut().insert_resource(AuthoredVerdictLog::default());

    let flag_set = ConditionId::new("world", "flag_set");
    let flag = "a_flag_only_this_test_asks_about";
    let subject = [AuthoredArg::Name(flag.to_string())];
    let stamp_of = |sim: &Platformer2dSimHarness| {
        sim.world()
            .resource::<AuthoredVerdictLog>()
            .latest_for(&flag_set, &subject)
            .map(|verdict| verdict.stamp)
    };

    // Asked while frame 314 is still a guess.
    sim.world_mut().insert_resource(ConfirmedFrameBoundary {
        current: 314,
        confirmed: 313,
        session: 9,
    });
    let _ = ask(&sim, &flag_set, &subject);
    assert_eq!(
        stamp_of(&sim).map(|stamp| stamp.confirmed),
        Some(false),
        "the premise is that this answer starts speculative"
    );

    // The host moves on and settles 314. Nothing asks the question again.
    sim.world_mut().insert_resource(ConfirmedFrameBoundary {
        current: 315,
        confirmed: 314,
        session: 9,
    });
    sim.step(base());
    assert_eq!(
        stamp_of(&sim),
        Some(ambition_platformer2d::platformer::authored_logic::VerdictStamp {
            simulation: Some((9, 314)),
            confirmed: true,
        }),
        "the host settled frame 314 and the ring still calls that answer a \
         guess, so every real historical event reads as speculative forever"
    );
}

/// ⛔⛤ **AND A RE-SIMULATED FRAME CLEARS WHAT THE ABANDONED PASS RECORDED.**
///
/// The companion to the arm above, on the same wiring: the reconciliation
/// system calls `begin_pass` for the frame about to run. Driven through the
/// composed schedule for the same reason.
#[test]
fn a_frame_simulated_again_does_not_keep_the_pass_that_was_abandoned() {
    use ambition_platformer2d::engine_core::ConfirmedFrameBoundary;
    use ambition_platformer2d::platformer::authored_logic::AuthoredVerdictLog;

    let mut sim = fixed_60hz_room_sim(ROOM);
    sim.step_n(base(), 4);
    sim.world_mut().insert_resource(AuthoredVerdictLog::default());

    let flag_set = ConditionId::new("world", "flag_set");
    let subject = [AuthoredArg::Name(
        "a_flag_only_the_abandoned_pass_asks_about".to_string(),
    )];
    sim.world_mut().insert_resource(ConfirmedFrameBoundary {
        current: 420,
        confirmed: 419,
        session: 3,
    });
    let _ = ask(&sim, &flag_set, &subject);
    assert!(
        sim.world()
            .resource::<AuthoredVerdictLog>()
            .latest_for(&flag_set, &subject)
            .is_some(),
        "the premise is that the abandoned pass recorded something"
    );

    // The host rewinds and runs 420 again. This pass never asks.
    sim.step(base());
    assert_eq!(
        sim.world()
            .resource::<AuthoredVerdictLog>()
            .latest_for(&flag_set, &subject),
        None,
        "a question only the abandoned pass asked is still being reported as \
         part of the history that survived"
    );
}


/// ASKING THE WORLD-FACT DOMAIN READS THE REAL SAVE.
///
/// an unset flag is `NotSatisfied` here, unlike the custody case, and the
/// asymmetry is the point rather than an inconsistency: a flag namespace is open,
/// so *"has this happened yet"* is a meaningful question about a fact nobody has
/// recorded. Answering *unanswerable* would leave every flag-gated thing stuck
/// until something set its flag once.
#[test]
fn the_world_fact_domain_answers_from_the_save_layer() {
    let mut sim = fixed_60hz_room_sim(ROOM);
    sim.step_n(base(), 4);

    let flag_set = ConditionId::new("world", "flag_set");
    let flag = "a_fact_this_run_has_not_recorded";

    assert!(matches!(
        ask(&sim, &flag_set, &[AuthoredArg::Name(flag.to_string())]),
        ConditionOutcome::NotSatisfied(_)
    ));

    sim.world_mut()
        .resource_mut::<ambition_platformer2d::persistence::save::AmbitionGameSave>()
        .data_mut()
        .set_flag(flag, true);

    assert_eq!(
        ask(&sim, &flag_set, &[AuthoredArg::Name(flag.to_string())]),
        ConditionOutcome::Satisfied,
        "the domain reads the live save rather than a copy taken at startup"
    );
}

/// ASKING THE INVENTORY DOMAIN READS THE REAL BAG.
///
/// the third domain, and it cost one line of composition. It is here
/// because it is the provider that let an authored Yarn function be deleted:
/// `inventory_has(...)` was a closure over a mirrored copy of `OwnedItems` that
/// `ambition_content` refilled every frame. what is pinned is that the
/// composed engine answers about live inventory — not that this domain exists in
/// some list.
#[test]
fn the_inventory_domain_answers_about_the_live_bag() {
    let mut sim = fixed_60hz_room_sim(ROOM);
    sim.step_n(base(), 4);

    let holds = ConditionId::new("inventory", "holds");
    let carried = |sim: &Platformer2dSimHarness, item: &str| {
        ask(sim, &holds, &[AuthoredArg::Name(item.to_string())])
    };

    assert_eq!(
        carried(&sim, "HealthPotion"),
        ConditionOutcome::Satisfied,
        "the app's starter bag carries health cells, and loose authored spelling \
         resolves through the item catalog's single normaliser"
    );

    // Empty the slot through the domain's own API; the answer follows, with
    // nothing refreshed and no snapshot in between.
    sim.world_mut()
        .resource_mut::<ambition_platformer2d::items::OwnedItems>()
        .take(ambition_platformer2d::items::Item::HealthCell, u32::MAX);
    assert!(matches!(
        carried(&sim, "healthcell"),
        ConditionOutcome::NotSatisfied(_)
    ));

    // and a kind no catalog row spells is UNANSWERABLE rather than "no",
    // which is what turns an authored typo into a diagnostic.
    let outcome = carried(&sim, "a_thing_this_game_has_no_row_for");
    assert!(
        matches!(outcome, ConditionOutcome::Unanswerable(_)),
        "got {outcome:?}"
    );
}

/// ⭐⭐ EVERY AUTHORED `gated_by` IN EVERY SHIPPED WORLD PREPARES AGAINST THE
/// COMPOSED CATALOG — the guard that makes the authoring vocabulary safe to use.
///
/// Since `gated_by` became a condition LINE, a level author may write
/// `body.fits 32` or `encounter.cleared goblin_encounter` instead of a bare flag
/// name. ⛔ The failure mode that buys is silent and expensive: a misspelt
/// condition, a wrong argument count or a verb the catalog does not publish
/// leaves the wall STANDING, which is correct behaviour and indistinguishable in
/// play from a gate whose condition is simply not satisfied yet. The engine logs
/// it once at `error!` and the route is shut for the rest of the session.
///
/// ⇒ This asks the same question at test time, against the SAME catalog the game
/// composes, so an unpreparable line fails a build rather than a playthrough.
///
/// ⛔ THE POPULATION IS SMALL AND THAT IS THE POINT OF THE FIRST ASSERTION.
/// Exactly two entity instances in the shipped worlds carry a `gated_by` value
/// today (both `intro.ldtk`, both the flag `bob_field_survey_received`), so a
/// version of this test that merely iterated would pass on an empty list — and
/// would keep passing if the field were renamed, the parser stopped emitting it,
/// or the worlds stopped loading. The count is asserted so the corpus cannot
/// quietly become empty; ⚠ it is a FLOOR, not an inventory, because authoring a
/// new gated wall must not fail this test.
#[test]
fn every_authored_gate_condition_prepares_against_the_composed_catalog() {
    use ambition_platformer2d::actors::world::gated_lock_walls::prepare_authored_gate;

    let mut sim = fixed_60hz_room_sim(ROOM);
    sim.step_n(base(), 4);
    let catalog = catalog(&sim);

    let mut seen: Vec<(String, String)> = Vec::new();
    let mut unpreparable: Vec<String> = Vec::new();

    for source in ambition_content::worlds::world_manifest().worlds {
        // ⛔⛔ `embedded_text` IS `None` HERE, AND READING ONLY IT MADE THIS TEST
        // VACUOUS ON ITS FIRST RUN. `static_world_text!` compiles to `None`
        // without the `static_map` feature, which this target does not enable —
        // so every world was skipped and the loop found nothing. The floor
        // assertion below is what said so ("Found []") instead of a green tick.
        // ⇒ The loose path is the desktop road and the one this test must read;
        // the embedded text is the web/Android road and is kept as the fallback
        // for a profile that bakes it and ships no files.
        let text = match source.loose_path.as_deref().map(std::fs::read_to_string) {
            Some(Ok(text)) => text,
            _ => match source.embedded_text {
                Some(text) => text.to_string(),
                None => continue,
            },
        };
        let Ok(project) =
            serde_json::from_str::<ambition_platformer2d::ldtk_map::LdtkProject>(&text)
        else {
            continue;
        };
        for level in &project.levels {
            for layer in &level.layer_instances {
                for entity in &layer.entity_instances {
                    for field in &entity.field_instances {
                        if field.identifier != "gated_by" {
                            continue;
                        }
                        let Some(authored) = field.value.as_str() else {
                            continue;
                        };
                        if authored.is_empty() {
                            continue;
                        }
                        seen.push((source.id.as_str().to_string(), authored.to_string()));
                        // ⛔ THE PRODUCTION FUNCTION, not a copy of its rule.
                        // `prepare_authored_gate` is the one decision about what
                        // an authored gate value means, and the wall system calls
                        // the same one — so this cannot drift into validating a
                        // rule the game stopped applying.
                        if let Err(error) = prepare_authored_gate(&catalog, authored) {
                            unpreparable.push(format!(
                                "world `{}` wall gated by `{authored}`: {}",
                                source.id.as_str(),
                                error.reason()
                            ));
                        }
                    }
                }
            }
        }
    }

    assert!(
        seen.len() >= 2,
        "the corpus went empty — `gated_by` was set on 2 entity instances when \
         this was written, so finding fewer means the field was renamed, the \
         parser stopped emitting it, or the worlds stopped loading, and this \
         test would then pass against anything. Found {seen:?}"
    );
    assert!(
        unpreparable.is_empty(),
        "an authored gate names a question this engine cannot prepare, so its \
         wall stands forever and the route behind it is unreachable in play:\n  {}",
        unpreparable.join("\n  ")
    );
}

/// ⭐⭐ EVERY `condition(...)` AN AUTHORED `.yarn` FILE ASKS IS ONE THE COMPOSED
/// ENGINE PUBLISHES — the DIALOGUE half of the guard above, and it was missing.
///
/// `gated_by` got a build-time check when it became a condition line. The other
/// authored road never did: `ambition_conversation`'s Yarn verb
/// `condition(id, arg)` takes the id as a STRING an author types, and there are
/// ten times more of those in shipped `.yarn` than there are gated walls in
/// shipped worlds.
///
/// ⛔ THE FAILURE IS THE SAME SHAPE AND WORSE PLACED. `ask_condition` refuses an
/// unparseable or unpublished id with a `warn!` and returns `false`, so a
/// misspelt `condition("body.cann", …)` does not error — **the branch simply
/// never opens**, for the rest of the game, indistinguishable in play from a
/// condition that is honestly not satisfied. That is a playthrough-time failure
/// this test converts into a build-time one.
///
/// ⭐ ONLY THE GENERIC SPELLING IS GUARDED, and that is deliberate rather than
/// partial. The NAMED functions bound to a condition (`boss_cleared(id)`,
/// `quest_active(id)`) cannot carry a misspelt condition id — the id lives in
/// Rust, not in the `.yarn` — and misspelling the FUNCTION is already a Yarn
/// load error. ⇒ **The guard covers exactly the spelling an author can get wrong
/// silently.**
///
/// ⚠ ARITY IS CHECKED TOO. Yarn's verb passes exactly one argument, so a
/// condition the catalog declares with zero or two parameters is mis-called by
/// construction — a different mistake from a misspelt id and equally silent.
///
/// ⛔ AND THE FLOOR IS THE FIRST ASSERTION, for the reason the wall guard
/// learned by being vacuous on its first run: a version of this that merely
/// iterated would pass on an empty list, and would keep passing if `YARN_SOURCES`
/// stopped being the manifest or the call spelling changed. ⚠ A FLOOR, not an
/// inventory — authoring a new `condition(...)` line must not fail this test.
#[test]
fn every_condition_an_authored_yarn_file_asks_is_published_by_the_engine() {
    let mut sim = fixed_60hz_room_sim(ROOM);
    sim.step_n(base(), 4);
    let catalog = catalog(&sim);

    let mut asked: Vec<(String, String)> = Vec::new();
    let mut unpublished: Vec<String> = Vec::new();
    let mut wrong_arity: Vec<String> = Vec::new();

    // ⭐ THE SHIPPED MANIFEST, not a directory walk: `YARN_SOURCES` is what
    // `yarn_spinner_plugin` registers and what the compile test compiles, so a
    // file this test reads is a file the game loads. A `.yarn` on disk that is
    // not in the manifest is not content, and should not fail a content guard.
    for (name, text) in ambition_content::dialogue::yarn::YARN_SOURCES {
        // ⭐⭐ REGIONS FIRST, CALLS SECOND. `executable_regions` is the one
        // definition of what the interpreter evaluates; a `condition(` outside
        // one is a character SAYING the words, not the engine being asked.
        //
        // ⛔⛔ THIS LOOP USED TO SCAN THE WHOLE FILE and defend itself with a
        // private rule — *"the quote must be the very next non-space
        // character"* — grown after its first run reported a defect that was
        // not there: `kernel.yarn` says in prose *"condition() reads the
        // world-fact domain directly"*, and the scanner walked past the empty
        // parens into `<<command "world.set_flag" …>>` two lines down and
        // reported a COMMAND id as an unpublished CONDITION. That patch made
        // this one sentence safe and left the class alive; two Python
        // instruments then grew their own. ⇒ The heuristic is DELETED, not
        // tuned. Prose is excluded by structure now, so a `condition(` that
        // IS evaluated is checked however it is spaced.
        for (_line, region) in ambition_content::dialogue::yarn::executable_regions(text) {
            for rest in region.split("condition(").skip(1) {
                let after = rest.trim_start();
                let Some(inner) = after.strip_prefix('"') else {
                    continue;
                };
                let Some(close) = inner.find('"') else {
                    continue;
                };
                let raw = &inner[..close];
                asked.push(((*name).to_string(), raw.to_string()));

                let Some(id) = ConditionId::parse(raw) else {
                    unpublished.push(format!("{name}: {raw:?} is not a `domain.question` id"));
                    continue;
                };
                match catalog.describe(&id) {
                    None => unpublished.push(format!(
                        "{name}: {raw:?} — no domain in the composed engine publishes it"
                    )),
                    Some(descriptor) if descriptor.params.len() != 1 => wrong_arity.push(format!(
                        "{name}: {raw:?} declares {} parameter(s), but the Yarn verb passes exactly 1",
                        descriptor.params.len()
                    )),
                    Some(_) => {}
                }
            }
        }
    }

    assert!(
        asked.len() >= 8,
        "only {} authored `condition(...)` call(s) found across {} shipped .yarn \\
         file(s) — the corpus this test walks has gone empty or the call spelling \\
         changed, and an empty walk passes every assertion below. Found: {asked:#?}",
        asked.len(),
        ambition_content::dialogue::yarn::YARN_SOURCES.len(),
    );
    assert!(
        unpublished.is_empty() && wrong_arity.is_empty(),
        "authored dialogue asks for conditions the composed engine cannot answer, \\
         so those branches never open in play:\\n  unpublished: {unpublished:#?}\\n  \\
         wrong arity: {wrong_arity:#?}",
    );
}

/// ⭐⭐ NO PLANNING DOC NAMES A CONDITION ID THE ENGINE DOES NOT PUBLISH — the
/// third road, and the one with no compiler behind it at all.
///
/// The two guards above cover authored CONTENT, where a wrong id silently closes
/// a branch. This one covers authored PROSE, where a wrong id costs something
/// different and slower: the next reader believes it. A condition id in a
/// planning doc is a citation, and a fabricated one is invisible to every check
/// this repository has — it compiles, because no code names it; it passes,
/// because no test names it; and it reads as authoritative, because it is a
/// plausible domain for the crate around it.
///
/// ⛔ THIS IS NOT HYPOTHETICAL AND IT IS NOT ONE MISTAKE. `held.is_held` was
/// written by hand across the planning set for a day — the id is
/// `custody.is_held`, because `ambition_held_items` declares `DOMAIN =
/// "custody"`. It was corrected on 2026-09-04 in four files; a second sweep the
/// same day found NINE more sites in six files, including the roadmap, a queue
/// receipt, and the module doc of the gate code itself. ⚠ And a third spelling,
/// `item.is_held`, sat in `inspection-diagnostics-and-workbench.md` listing the
/// production evaluators — found by this rule, after two prose sweeps missed it.
/// ⇒ Nine of eleven wrong is what a prose sweep scores against a mechanical one.
///
/// ⭐ THE RULE IS THE QUESTION-HALF MATCH, and that is what keeps it quiet. Docs
/// are full of backticked `a.b` tokens (`mod.rs`, `Vec2.x`); flagging every one
/// would be unusable. A token is only suspicious when its QUESTION half is a
/// question the engine really publishes but the whole id is not — `held.is_held`
/// against `custody.is_held`. That is precisely the near-miss shape, and
/// measured over 100 planning docs it fires on the real defects and nothing else.
///
/// ⚠ A DOC MAY NAME A WRONG ID DELIBERATELY — several must, because the
/// correction is their subject, and a guard that forbade it would delete the
/// record of the bug it exists to prevent. The escape is a rule worth having on
/// its own: **name the correct id in the same paragraph.** A wrong spelling
/// alone is a defect; a wrong spelling next to its correction is documentation.
#[test]
fn no_planning_doc_names_a_condition_the_engine_does_not_publish() {
    let mut sim = fixed_60hz_room_sim(ROOM);
    sim.step_n(base(), 4);
    let catalog = catalog(&sim);

    // ⭐ THE COMPOSED CATALOG IS THE AUTHORITY, not a text scan for
    // `ConditionId::new`. A doc is checked against what the engine actually
    // answers, so publishing a condition cannot leave this guard behind.
    let published: Vec<String> = catalog.describe_all().map(|d| d.id.to_string()).collect();

    // ⛔⛔ **A SECOND PUBLISHED VOCABULARY SHAPED EXACTLY LIKE THE FIRST, and
    // without it this guard reports a correct document as a defect.** Rollback
    // schema rows are `namespace.snake_case` too, and they COLLIDE: the row
    // `feature.switch_on` shares its question half with the condition
    // `world.switch_on`, so the near-miss rule below reads a correct citation of
    // a schema row as a misspelled condition. MEASURED 2026-09-10 — it flagged
    // `simulation-authority-and-determinism.md`'s census table, whose subject IS
    // the schema, and the "fix" would have been to write a WRONG row id into a
    // right document to silence a guard asking a different question.
    //
    // ⭐ CROSS-EVIDENCE, not a bigger allow-list: the schema dump is a different
    // world from the condition catalog, and a token that appears verbatim in
    // either is a real id. Both come from the composed app, so neither can drift
    // into a hand-kept list.
    let schema_rows: Vec<String> = sim
        .world()
        .get_resource::<ambition_platformer2d::rollback::RollbackRegistry>()
        .map(|registry| {
            registry
                .schema_dump()
                .lines()
                .filter_map(|line| line.split('\t').next())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    assert!(
        published.len() >= 5,
        "the composed catalog published {} condition(s); with a near-empty \
         catalog this test cannot recognise a near-miss and passes vacuously",
        published.len(),
    );

    let docs = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/planning");
    let mut suspicious: Vec<String> = Vec::new();
    let mut correct_citations = 0usize;
    let mut scanned = 0usize;

    for path in markdown_under(&docs) {
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}\nThis is an IO failure, NOT a finding about the repository — do not read any verdict from this run.", path.display()));
        scanned += 1;
        let rel = path.strip_prefix(&docs).unwrap_or(&path).display().to_string();
        let lines: Vec<&str> = text.split('\n').collect();

        // paragraph = a run of non-blank lines; the escape is scoped to it
        let mut para_of: Vec<Option<usize>> = Vec::with_capacity(lines.len());
        let mut para = 0usize;
        for line in &lines {
            if line.trim().is_empty() {
                para += 1;
                para_of.push(None);
            } else {
                para_of.push(Some(para));
            }
        }

        for (n, line) in lines.iter().enumerate() {
            for token in backticked_dotted_words(line) {
                if published.iter().any(|p| p == &token) {
                    correct_citations += 1;
                    continue;
                }
                // A rollback SCHEMA row is a published id of a different kind.
                if schema_rows.iter().any(|r| r == &token) {
                    correct_citations += 1;
                    continue;
                }
                let Some((_, question)) = token.split_once('.') else {
                    continue;
                };
                // Only a token whose QUESTION half is really published is a
                // near-miss; everything else is ordinary dotted prose.
                let intended: Vec<&String> = published
                    .iter()
                    .filter(|p| p.split_once('.').map(|(_, q)| q) == Some(question))
                    .collect();
                if intended.is_empty() {
                    continue;
                }
                let paragraph: String = lines
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| para_of[*i].is_some() && para_of[*i] == para_of[n])
                    .map(|(_, l)| *l)
                    .collect::<Vec<_>>()
                    .join("\n");
                if intended.iter().any(|c| paragraph.contains(c.as_str())) {
                    continue; // names its own correction alongside it
                }
                suspicious.push(format!(
                    "{rel}:{}: `{token}` names no published condition; the engine publishes {intended:?}",
                    n + 1
                ));
            }
        }
    }

    assert!(
        scanned >= 50 && correct_citations >= 10,
        "scanned {scanned} planning doc(s) holding {correct_citations} correct condition \
         citation(s) — the corpus this test walks has moved or gone empty, and an empty \
         walk passes the assertion below"
    );
    assert!(
        suspicious.is_empty(),
        "planning docs cite condition ids the composed engine does not publish. A \
         fabricated id compiles, passes, and reads as authoritative, so the next reader \
         inherits it. Fix the spelling, or name the correct id in the same paragraph if \
         the wrong one is the subject:\n  {}",
        suspicious.join("\n  ")
    );
}

/// Every `.md` beneath `dir`, recursively.
fn markdown_under(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let entries = std::fs::read_dir(&d)
            .unwrap_or_else(|e| panic!("cannot list {}: {e}\nThis is an IO failure, NOT a finding about the repository.", d.display()));
        for entry in entries {
            let path = entry.expect("directory entry").path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "md") {
                out.push(path);
            }
        }
    }
    out
}

/// Backtick-delimited `lower_snake.lower_snake` tokens in one line.
fn backticked_dotted_words(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (i, chunk) in line.split('`').enumerate() {
        if i % 2 == 0 {
            continue; // outside the backticks
        }
        let ok = |c: char| c.is_ascii_lowercase() || c == '_';
        if let Some((domain, question)) = chunk.split_once('.') {
            if !domain.is_empty()
                && !question.is_empty()
                && domain.chars().all(ok)
                && question.chars().all(ok)
            {
                out.push(chunk.to_string());
            }
        }
    }
    out
}

/// ⭐⭐ EVERY AUTHORED `boss_cleared("…")` NAMES A BOSS PLACEMENT THAT EXISTS.
///
/// Jon's ruling on decision 57 (2026-09-05): *"Boss progress is keyed only by
/// stable authored encounter/placement IDs. `boss.cleared(id)` means 'has this
/// specific authored boss encounter been cleared?'"* — and he named this guard
/// as the fifth step, because the ruling is only worth anything if a typo cannot
/// quietly reintroduce the defect it fixes.
///
/// ⛔⛔ THE DEFECT THIS EXISTS FOR WAS LIVE FOR WEEKS AND SILENT. The durable
/// record is written under the PLACEMENT id and `boss.cleared` looks it up
/// exactly, but an author can only type a name they can know — so `cove.yarn`
/// and `kernel.yarn` asked `boss_cleared("mockingbird")`, the BEHAVIOUR id,
/// against a save keyed `BossSpawn-4308`. Three executable branches that could
/// never open, with no error anywhere: a missing key reads `Untouched`, so the
/// gate simply stayed shut and looked like content nobody had written.
///
/// ⇒ A wrong id must be a RED, not a closed door. That is the whole point.
///
/// ⭐ Resolution goes through `boss_placement_id`, the PRODUCTION function
/// `convert_boss_spawn` uses — not a copy of its rule — so this cannot certify a
/// spelling the converter would resolve differently.
#[test]
fn every_authored_boss_cleared_call_names_a_real_boss_placement() {
    let mut placements: Vec<(String, String)> = Vec::new();
    for source in ambition_content::worlds::world_manifest().worlds {
        // The loose path is the desktop road; the embedded text is the
        // web/Android one. Same fallback the gate-condition arm above documents.
        let text = match source.loose_path.as_deref().map(std::fs::read_to_string) {
            Some(Ok(text)) => text,
            _ => match source.embedded_text {
                Some(text) => text.to_string(),
                None => continue,
            },
        };
        let Ok(project) =
            serde_json::from_str::<ambition_platformer2d::ldtk_map::LdtkProject>(&text)
        else {
            continue;
        };
        for level in &project.levels {
            for layer in &level.layer_instances {
                for entity in &layer.entity_instances {
                    if entity.identifier != "BossSpawn" {
                        continue;
                    }
                    placements.push((
                        source.id.as_str().to_string(),
                        ambition_platformer2d::ldtk_map::boss_placement_id(entity),
                    ));
                }
            }
        }
    }

    // ⛔ ANTI-VACUITY, and the floor clears the LARGEST SINGLE WORLD: `sandbox`
    // authors 9 of the 11 shipped placements, so a floor of 10 fails if that one
    // world stops being read — the failure mode that would otherwise let this
    // test certify an empty set.
    assert!(
        placements.len() >= 10,
        "only {} boss placement(s) found across the shipped worlds; `sandbox` \
         alone authors 9, so this walk has gone empty and every assertion below \
         is vacuous. Found: {placements:#?}",
        placements.len(),
    );

    let known: std::collections::BTreeSet<&str> =
        placements.iter().map(|(_, id)| id.as_str()).collect();

    let mut unresolved: Vec<String> = Vec::new();
    let mut asked = 0usize;
    for (name, text) in ambition_content::dialogue::yarn::YARN_SOURCES {
        for (line, region) in ambition_content::dialogue::yarn::executable_regions(text) {
            for rest in region.split("boss_cleared(").skip(1) {
                let Some(inner) = rest.trim_start().strip_prefix('"') else {
                    continue;
                };
                let Some(close) = inner.find('"') else {
                    continue;
                };
                let id = &inner[..close];
                asked += 1;
                if !known.contains(id) {
                    unresolved.push(format!(
                        "{name}:{line} asks boss_cleared({id:?}), which names no \
                         authored boss placement in any shipped world"
                    ));
                }
            }
        }
    }

    // ⛔ The corpus floor is EXECUTABLE calls: `kernel.yarn` also SPEAKS the call
    // twice in Kernel Guide prose, and counting those was a real 25% over-count
    // until `executable_regions` landed. Three is what the interpreter evaluates.
    assert!(
        asked >= 3,
        "only {asked} executable `boss_cleared(...)` call(s) across the shipped \
         .yarn corpus — the walk has gone empty or the spelling changed, and an \
         empty walk passes the assertion below"
    );
    assert!(
        unresolved.is_empty(),
        "authored dialogue gates on boss placements that do not exist, so those \
         branches can never open and nothing errors — the save simply reads \
         `Untouched`:\n  {}\n\nAuthored placements are: {known:#?}\n\n\
         ⛔ BEFORE READING THIS AS A CONTENT DEFECT, CHECK YOUR SUBMODULE. Every \
         `.ldtk` in `game/ambition_content/assets/worlds/` is a SYMLINK into the \
         `game/ambition_map_assets` submodule, so this test reads whatever that \
         checkout holds — not what the pin names. A checkout BEHIND the pin is \
         missing the ids the pin authors, and this assertion is the symptom:\n\
           git submodule status game/ambition_map_assets\n\
         ⚠ and if it is behind, do NOT blindly `git submodule update`: that is a \
         detached checkout which DISCARDS uncommitted work in the submodule. \
         Commit anything dirty there first. (Measured 2026-09-05: a peer carried \
         these three reds as a maintainer question for a day; the cause was one \
         missing submodule commit.)",
        unresolved.join("\n  "),
    );
}

/// ⛔⛤ **TWENTY-FIVE CENSUS SURFACES AND NOT ONE OF THEM ANSWERED "WHERE AM
/// I".**
///
/// Every existing `[census]` row describes the machine — entities,
/// archetypes, schedules, draw calls, render passes, phase costs. None
/// describes the WORLD. A room transition that stalls, commits into the wrong
/// room, or opens a transaction nobody closes was diagnosable only with a
/// debugger or by reading four files across three crates.
///
/// ⭐ **AND IT IS THE FIRST FACT THE OPEN-WORLD WORK NEEDS.** OW1 on
/// `engine/open-world-runtime-and-residency.md` is *"two instances of one
/// room; audit selection/identity/query/teardown paths"*, and today *"which
/// room is live"* is answered by `RoomSet::active: usize` — an INDEX into a
/// list of definitions, which is the conflation OW1 exists to unpick. The row
/// prints the index beside the authored id so the day they stop corresponding
/// is visible rather than inferred.
///
/// ⚠ **THE ARM IS AGAINST THE SESSION'S OWN ANSWER, NOT A LITERAL.** Pinning
/// the room name would make this a test about the fixture. What is checked is
/// that the row agrees with `RoomSet` about which room is active — which is
/// the only thing a derived, read-only surface can get wrong.
#[test]
fn the_room_census_names_the_room_the_session_is_actually_in() {
    use ambition_platformer2d::world::rooms::{LiveRoomInstance, RoomSet};
    use ambition_platformer2d::runtime::runtime_census::room_census_row;

    let mut sim = fixed_60hz_room_sim(ROOM);
    sim.step_n(base(), 4);

    let (active_id, active_index, room_count) = {
        let mut query = sim
            .world_mut()
            .query_filtered::<&RoomSet, With<SessionRoot>>();
        let world = sim.world();
        let room_set = query
            .iter(world)
            .next()
            .expect("the composed session root carries a RoomSet");
        (
            room_set.active_spec().id.clone(),
            room_set.active(),
            room_set.rooms.len(),
        )
    };

    let row = {
        let mut query = sim
            .world_mut()
            .query::<(&RoomSet, &SessionRoot, Option<&LiveRoomInstance>)>();
        let world = sim.world();
        let rows: Vec<_> = query.iter(world).collect();
        room_census_row(1.5, rows.into_iter(), None)
    };

    assert!(
        row.contains(&format!("active={active_id}[{active_index}]")),
        "the row does not name the room the session is in \
         (`{active_id}` at index {active_index}): {row}"
    );
    assert!(
        row.contains(&format!("rooms={room_count}")),
        "the row does not count the rooms this session was built with \
         ({room_count}): {row}"
    );
    assert!(
        row.contains("sessions=1"),
        "one composed host, one session root: {row}"
    );
    // ⛔⛤ **THE SCOPE IS ASSERTED, AND IT WAS NOT — WHICH IS HOW THE ROW
    // PRINTED `scope=?` FOR EVERY REAL ROOT FOR ONE COMMIT.** The census asked
    // for `Option<&SessionScopeId>` as a SIBLING of `SessionRoot`, and the
    // scope lives inside it (`SessionRoot(pub SessionScopeId)`), so the
    // Option never matched. This arm repeated the same wrong query and never
    // looked at the value: an instrument's own test agreeing with its defect.
    let scope = {
        let mut query = sim.world_mut().query::<&SessionRoot>();
        let world = sim.world();
        query
            .iter(world)
            .next()
            .expect("the composed session root exists")
            .0
             .0
    };
    assert!(
        row.contains(&format!("scope={scope}")),
        "the row does not name the session's own scope ({scope}): {row}"
    );
    // ⛔⛤ **AND THE LIVE-ROOM IDENTITY IS READ FROM A SESSION THAT HAS MOVED,
    // because this fixture never crosses a room.** `scope=?` printed for one
    // commit because this arm repeated the census's own wrong query and never
    // looked at the value; `live=` is the same trap one field along, and worse,
    // because the honest value HERE is `#0`. Measured: a poison printing the
    // constant `LiveRoomInstance::ACTIVATION` instead of reading the component
    // PASSED against a session standing in its activation room. So the arm
    // moves the session's live room by hand and asserts the row follows it.
    let moved_to = {
        let mut query = sim.world_mut().query::<&mut LiveRoomInstance>();
        let world = sim.world_mut();
        let mut live = query
            .iter_mut(world)
            .next()
            .expect("the composed session root carries a live-room instance");
        live.advance();
        live.advance();
        *live
    };
    assert_ne!(
        moved_to,
        LiveRoomInstance::ACTIVATION,
        "the arm must read a value a constant cannot be"
    );
    let after_moving = {
        let mut query = sim
            .world_mut()
            .query::<(&RoomSet, &SessionRoot, Option<&LiveRoomInstance>)>();
        let world = sim.world();
        let rows: Vec<_> = query.iter(world).collect();
        room_census_row(1.5, rows.into_iter(), None)
    };
    assert!(
        after_moving.contains(&format!("live={moved_to}")),
        "the row does not name the live room the session is standing in ({moved_to}): {after_moving}"
    );
    assert!(
        after_moving.contains(&format!("active={active_id}[{active_index}]")),
        "moving the LIVE ROOM moved the active DEFINITION too, which would mean the row is reading one field for both questions: {after_moving}"
    );
    // ⚠ AND "NOTHING IS CROSSING" IS PRINTED RATHER THAN OMITTED. A row that
    // said nothing about the transaction would make a stalled crossing and a
    // quiet world produce the same text, which is the one comparison a reader
    // watching a sequence of samples is actually making.
    assert!(row.contains("crossing=none"), "{row}");

    // ⛔⛤ **AND `active` IS FOLLOWED, NOT `start` — A POISON PASSED THROUGH
    // THIS ARM UNTIL IT MOVED THE ROOM.** This fixture boots into the room it
    // starts in, so `active == start`, and printing the wrong one of the two
    // produced a byte-identical row. Every arm above still passed. The only
    // way to separate them is to make them differ.
    //
    // ⚠ THE ROOM IS MOVED BY HAND AND NOTHING IS STEPPED AFTERWARDS. A real
    // crossing is a whole transaction; what is under test is a derived row, so
    // what it needs is for its SOURCE to change. The world is left
    // inconsistent for the two lines it takes to read the row, and nothing
    // runs in between.
    let elsewhere = {
        let mut query = sim
            .world_mut()
            .query_filtered::<&mut RoomSet, With<SessionRoot>>();
        let world = sim.world_mut();
        let mut room_set = query
            .iter_mut(world)
            .next()
            .expect("the composed session root carries a RoomSet");
        let elsewhere = (0..room_set.rooms.len())
            .find(|index| *index != room_set.active())
            .expect("this session was built with one room, so `active` and \
                     `start` can never differ and this arm cannot run");
        room_set
            .set_active(elsewhere)
            .expect("`elsewhere` was chosen from this set's own index range")
            .id
            .clone()
    };
    let moved = {
        let mut query = sim
            .world_mut()
            .query::<(&RoomSet, &SessionRoot, Option<&LiveRoomInstance>)>();
        let world = sim.world();
        let rows: Vec<_> = query.iter(world).collect();
        room_census_row(2.5, rows.into_iter(), None)
    };
    assert!(
        moved.contains(&format!("active={elsewhere}")),
        "the row did not follow the active room to `{elsewhere}`: {moved}"
    );
    assert!(
        moved.contains(&format!("start={active_id}[{active_index}]")),
        "the row's `start` moved with the active room, so the two columns are \
         one fact printed twice: {moved}"
    );
}

/// ⭐⛤ **A RUNNING GAME CAN NOW BE ASKED WHAT IS STUCK — the last open item on
/// the verdict stream, 2026-09-21.**
///
/// `AuthoredVerdictLog` holds the structured `no`, who asked it and which
/// frame produced it, and every consumer of it was an `assert!`. Diagnosing a
/// stuck gate in a running game still meant attaching a debugger or writing a
/// test that reproduces the thing you are trying to understand.
///
/// ⚠ THE RING IS FILLED BY THE PRODUCTION RECORDER, not by hand:
/// `ConditionCatalog::evaluate` records, the real world-fact evaluator
/// answers, and the row is a projection of what that left behind. A
/// hand-built ring would be asserting that a formatter formats.
#[test]
fn the_verdict_census_says_what_is_blocked_and_who_asked() {
    use ambition_platformer2d::platformer::authored_logic::AuthoredVerdictLog;
    use ambition_platformer2d::runtime::verdict_census::verdict_census_row;

    let mut sim = fixed_60hz_room_sim(ROOM);
    sim.step_n(base(), 4);
    sim.world_mut().insert_resource(AuthoredVerdictLog::default());

    let flag_set = ConditionId::new("world", "flag_set");
    let shut = "a_door_nobody_has_opened";
    // The evaluator's own words, not this test's: the premise is that a REAL
    // refusal reached the ring, and pinning the wording here would make the
    // arm about the message rather than about the census.
    assert!(
        matches!(
            ask(&sim, &flag_set, &[AuthoredArg::Name(shut.to_string())]),
            ConditionOutcome::NotSatisfied(_)
        ),
        "the premise is a real refusal from the real evaluator"
    );

    let entries = sim.world().resource::<AuthoredVerdictLog>().recent();
    let row = verdict_census_row(2.5, Some(&entries));
    assert!(
        row.contains("blocked=1[probe:a test]"),
        "the row does not name who is blocked: {row}"
    );
    assert!(row.contains("no=1"), "the row does not count the `no`: {row}");
    assert!(
        row.contains(&format!("last-no: probe:a test world.flag_set(\"{shut}\")")),
        "the row does not carry the refusal itself: {row}"
    );

    // ⭐ THE CONTROL: setting the flag makes the same question answer yes, and
    // a row that said "blocked" whatever the world was doing would read the
    // same here.
    sim.world_mut()
        .resource_mut::<ambition_platformer2d::persistence::save::AmbitionGameSave>()
        .data_mut()
        .set_flag(shut, true);
    sim.world().resource::<AuthoredVerdictLog>().clear();
    assert_eq!(
        ask(&sim, &flag_set, &[AuthoredArg::Name(shut.to_string())]),
        ConditionOutcome::Satisfied
    );
    let entries = sim.world().resource::<AuthoredVerdictLog>().recent();
    let open = verdict_census_row(3.5, Some(&entries));
    assert!(
        open.contains("blocked=0") && !open.contains("last-no:"),
        "the door opened and the census still reports it stuck: {open}"
    );

    // ⛔ AND NO RING IS NOT AN EMPTY RING. A reader who has just enabled the
    // census and installed nothing must not be told their game asks no
    // authored questions.
    let absent = verdict_census_row(4.5, None);
    assert!(absent.contains("log=absent"), "{absent}");
    assert!(
        !absent.contains("blocked="),
        "an absent ring reported a blocked count: {absent}"
    );
}

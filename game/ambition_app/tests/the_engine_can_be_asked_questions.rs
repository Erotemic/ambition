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
    catalog(sim).evaluate(sim.world(), id, args)
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

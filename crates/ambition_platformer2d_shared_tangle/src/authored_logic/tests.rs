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
            &[AuthoredArg::Reference(SimId::placement("axe"))]
        ),
        ConditionOutcome::Satisfied
    );
    assert!(matches!(
        catalog.evaluate(
            app.world(),
            &id,
            &[AuthoredArg::Reference(SimId::placement("rock"))]
        ),
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
    let outcome = catalog.evaluate(app.world(), &ConditionId::new("nobody", "cares"), &[]);
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

    let ConditionOutcome::Unanswerable(too_few) = catalog.evaluate(app.world(), &id, &[]) else {
        panic!("no arguments must be refused");
    };
    assert!(too_few.contains("occurrence"), "{too_few}");

    let ConditionOutcome::Unanswerable(wrong_kind) =
        catalog.evaluate(app.world(), &id, &[AuthoredArg::Name("axe".to_string())])
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

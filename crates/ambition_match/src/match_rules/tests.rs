use ambition_content_pack::ModuleNamespace;

use super::*;

const PATH: &str = "data/match_rules.ron";

fn check(text: &str) -> FacetOutcome {
    let schema = SchemaId::new(MATCH_RULES_SCHEMA);
    let namespace = ModuleNamespace("test".to_string());
    let facet = FacetSource {
        declared_path: PATH,
        text,
        schema: &schema,
        namespace: &namespace,
    };
    let mut out = FacetOutcome::default();
    MatchRulesSchema.check(&facet, &mut out);
    out
}

fn lowered(out: &FacetOutcome) -> Option<AuthoredMatchRules> {
    out.lowered
        .as_ref()
        .and_then(|artifact| artifact.downcast_ref::<AuthoredMatchRules>())
        .copied()
}

/// Each field is the `MatchRules` field of its name, and `apply_to` writes it
/// there; a field left out is the engine's answer.
#[test]
fn each_authored_field_is_the_match_rule_it_names() {
    let out = check(
        "(stocks: Some(3), stock_respawn_seconds: 1.0, health_pool: Some(100), opens_suspended: true, \
         opening_countdown_ticks: 18, time_limit_ticks: 28800, fighter_contact: Some((resistance: 0.85)))",
    );
    assert!(out.diagnostics.is_empty(), "{:?}", out.diagnostics);
    let mut rules = MatchRules::default();
    lowered(&out).expect("a valid document lowers").apply_to(&mut rules);
    assert_eq!(rules.stocks, Some(3));
    assert_eq!(rules.stock_respawn_seconds, 1.0);
    assert_eq!(rules.health_pool, Some(100));
    assert!(rules.opens_suspended);
    assert_eq!((rules.opening_countdown_ticks, rules.time_limit_ticks), (18, 28800));
    assert_eq!(rules.fighter_contact.map(|contact| contact.resistance), Some(0.85));
    assert_eq!((rules.body, rules.abilities), (None, None), "a field left out is the engine's answer");
}

#[test]
fn a_value_the_runtime_cannot_use_is_refused_with_its_file_and_field() {
    for (text, field) in [
        ("(stocks: Some(0))", "stocks"),
        ("(health_pool: Some(0))", "health_pool"),
        ("(stock_respawn_seconds: -1.0)", "stock_respawn_seconds"),
        ("(fighter_contact: Some((resistance: -0.5)))", "fighter_contact.resistance"),
    ] {
        let out = check(text);
        assert!(lowered(&out).is_none(), "{text} lowered");
        let shown = format!("{:?}", out.diagnostics);
        assert!(shown.contains(field), "the refusal of {text} does not name `{field}`: {shown}");
        assert!(shown.contains(PATH), "the refusal of {text} does not name the file: {shown}");
    }
}

#[test]
fn a_misspelled_field_is_an_unknown_field() {
    let out = check("(stock: Some(3))");
    assert!(lowered(&out).is_none());
    assert_eq!(out.diagnostics.first().map(|d| d.code), Some(DiagnosticCode::UnknownField));
}

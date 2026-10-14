use ambition_content_pack::ModuleNamespace;

use super::*;
use crate::death_rules::LevelReset;

const PATH: &str = "data/room_rules.ron";

fn check(text: &str) -> FacetOutcome {
    let schema = SchemaId::new(ROOM_RULES_SCHEMA);
    let namespace = ModuleNamespace("test".to_string());
    let facet = FacetSource {
        declared_path: PATH,
        text,
        schema: &schema,
        namespace: &namespace,
    };
    let mut out = FacetOutcome::default();
    RoomRulesSchema.check(&facet, &mut out);
    out
}

fn lowered(out: &FacetOutcome) -> Option<RoomRules> {
    out.lowered
        .as_ref()
        .and_then(|artifact| artifact.downcast_ref::<RoomRules>())
        .copied()
}

#[test]
fn each_authored_field_is_the_rule_it_names() {
    let out = check(
        "(replay_level_after_death_s: Some(1.5), heavy_hit_damage: Some(3), landing_lag_share: Some(0.25))",
    );
    assert!(out.diagnostics.is_empty(), "{:?}", out.diagnostics);
    let rules = lowered(&out).expect("a valid document lowers");
    assert_eq!(
        rules.death,
        Some(DeathRules {
            interlude: 1.5,
            level_reset: LevelReset::WhenNoParticipantRemains
        })
    );
    assert_eq!(rules.strike_weight, Some(StrikeWeightRules::heavy_at(3)));
    assert_eq!(rules.landing_lag, Some(LandingLagRules::scaled_by(0.25)));
}

/// An absent field declares nothing, so the engine's undeclared answer
/// applies: it is not a zero.
#[test]
fn an_absent_field_declares_no_rule() {
    let out = check("(heavy_hit_damage: Some(12))");
    let rules = lowered(&out).expect("a partial document lowers");
    assert_eq!(rules.death, None);
    assert_eq!(rules.landing_lag, None);
    assert_eq!(rules.strike_weight, Some(StrikeWeightRules::heavy_at(12)));
}

/// The value parses and the runtime cannot use it. The refusal names the file
/// and the field, at admission, and nothing lowers.
#[test]
fn a_value_the_runtime_cannot_use_is_refused_with_its_file_and_field() {
    for (text, field) in [
        ("(landing_lag_share: Some(-0.5))", "landing_lag_share"),
        ("(heavy_hit_damage: Some(0))", "heavy_hit_damage"),
        ("(replay_level_after_death_s: Some(-1.0))", "replay_level_after_death_s"),
    ] {
        let out = check(text);
        assert!(lowered(&out).is_none(), "{text} lowered");
        let diagnostic = out.diagnostics.first().unwrap_or_else(|| panic!("{text} was not refused"));
        let shown = format!("{diagnostic:?}");
        assert!(shown.contains(field), "the refusal of {text} does not name `{field}`: {shown}");
        assert!(shown.contains(PATH), "the refusal of {text} does not name the file: {shown}");
    }
}

#[test]
fn a_misspelled_field_is_an_unknown_field() {
    let out = check("(landing_lag_shares: Some(0.25))");
    assert!(lowered(&out).is_none());
    assert_eq!(out.diagnostics.first().map(|d| d.code), Some(DiagnosticCode::UnknownField));
}

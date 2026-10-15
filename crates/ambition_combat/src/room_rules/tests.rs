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
        "(replay_level_after_death_s: Some(1.5), heavy_hit_damage: Some(3), landing_lag_share: Some(0.25), \
          hostile_wake_radius: Some(900.0))",
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
    assert_eq!(rules.dormancy, Some(crate::dormancy_rule::DormancyRule { hostile_wake_radius: 900.0 }));
}

/// An absent field declares nothing, so the engine's undeclared answer
/// applies: it is not a zero.
#[test]
fn an_absent_field_declares_no_rule() {
    let out = check("(heavy_hit_damage: Some(12))");
    let rules = lowered(&out).expect("a partial document lowers");
    assert_eq!(rules.death, None);
    assert_eq!(rules.landing_lag, None);
    assert_eq!(rules.dormancy, None, "an absent wake radius lets no brain sleep");
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
        ("(hostile_wake_radius: Some(0.0))", "hostile_wake_radius"),
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

/// A whole `combat` block: every field of `CombatRules` that is not an
/// `Option`, with `edit` replacing one `field: value,` line.
fn combat_block(edit: (&str, &str)) -> String {
    let fields = [
        ("di_max_angle", "0.31"),
        ("knockback_growth", "0.02"),
        ("downward_hit", "Spike"),
        ("meteor_lock_time", "0.3"),
        ("rage_per_damage", "0.004"),
        ("rage_max_scale", "1.4"),
        ("stale_step", "0.05"),
        ("stale_floor", "0.55"),
        ("crouch_cancel_scale", "0.85"),
        ("hit_repeat_window_scale", "0.0"),
        ("grab_hold_base_seconds", "1.5"),
        ("grab_hold_per_damage", "0.03"),
        ("grab_hold_max_seconds", "6.0"),
        ("grab_mash_seconds", "0.24"),
        ("friendly_fire", "false"),
        ("clank_damage_window", "0.0"),
        ("clank_rebound_speed", "190.0"),
        ("ledge_occupancy", "Some(Trump)"),
    ];
    let mut lines = String::new();
    let mut replaced = false;
    for (field, value) in fields {
        if field == edit.0 {
            replaced = true;
            if !edit.1.is_empty() {
                lines.push_str(&format!("{field}: {},", edit.1));
            }
        } else {
            lines.push_str(&format!("{field}: {value},"));
        }
    }
    if !replaced {
        lines.push_str(&format!("{}: {},", edit.0, edit.1));
    }
    format!("(heavy_hit_damage: Some(12), combat: Some(({lines})))")
}

/// The combat rules are pack data (Smash's `room_rules.ron`): the block is
/// the `CombatRules` it states, and an `Option` field it leaves out is `None`.
#[test]
fn a_combat_block_is_the_combat_rules_it_states() {
    let out = check(&combat_block(("victim_percent_knockback_scale", "Some(1.25)")));
    assert!(out.diagnostics.is_empty(), "{:?}", out.diagnostics);
    let combat = lowered(&out).and_then(|rules| rules.combat).expect("the block lowers");
    assert_eq!(combat.di_max_angle, 0.31);
    assert_eq!(combat.downward_hit, crate::rules::DownwardHitStyle::Spike);
    assert_eq!(combat.victim_percent_knockback_scale, Some(1.25));
    assert_eq!(combat.ledge_occupancy, Some(crate::rules::LedgeOccupancy::Trump));
    assert_eq!(combat.sudden_death_damage, None, "an `Option` left out is `None`");
}

/// A game states every value its game plays under: a required field left out
/// is a refusal that names it, not the engine's baseline.
#[test]
fn a_combat_block_that_leaves_out_a_required_field_is_refused() {
    let out = check(&combat_block(("stale_floor", "")));
    assert!(lowered(&out).is_none());
    let shown = format!("{:?}", out.diagnostics);
    assert!(shown.contains("stale_floor"), "the refusal does not name the field: {shown}");
}

#[test]
fn a_combat_value_the_runtime_cannot_use_is_refused_with_its_field() {
    for (edit, field) in [
        (("clank_rebound_speed", "-1.0"), "combat.clank_rebound_speed"),
        (("bark_chance", "Some(1.5)"), "combat.bark_chance"),
        (("stale_knockback_influence", "Some(-0.1)"), "combat.stale_knockback_influence"),
    ] {
        let out = check(&combat_block(edit));
        assert!(lowered(&out).is_none(), "{edit:?} lowered");
        let shown = format!("{:?}", out.diagnostics);
        assert!(shown.contains(field), "the refusal of {edit:?} does not name `{field}`: {shown}");
        assert!(shown.contains(PATH), "the refusal of {edit:?} does not name the file: {shown}");
    }
}

#[test]
fn a_misspelled_combat_field_is_an_unknown_field() {
    let out = check(&combat_block(("knockback_grwoth", "0.02")));
    assert!(lowered(&out).is_none());
    assert_eq!(out.diagnostics.first().map(|d| d.code), Some(DiagnosticCode::UnknownField));
}

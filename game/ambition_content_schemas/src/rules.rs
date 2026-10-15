//! The `ambition_rules` schema: Ambition's own tuning, as data in its pack
//! (`assets/data/ambition_rules.ron`).

use ambition_content_pack::{lowered_rules, rules_schema, PreparedContentPack, SchemaRegistration};
use serde::Deserialize;

/// The authored FILE kind.
pub const AMBITION_RULES_SCHEMA: &str = "ambition_rules";

/// Ambition's own tuning.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AmbitionRules {
    pub boss_idle_bark: IdleBark,
    pub gnu_ton_duel: GnuTonDuelRules,
    pub african_swallow: SwallowFetch,
    /// The thrown items that hurt the body they strike, by held-item id. An
    /// item with no row lands and hurts nobody.
    pub thrown_impacts: std::collections::BTreeMap<String, ThrownImpact>,
}

/// What one kind of thrown item does to a body it strikes.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThrownImpact {
    /// At or above this speed (px/s) the item hurts what it strikes.
    pub min_speed: f32,
    pub damage: i32,
}

/// What an African swallow does with a coconut (`primeval`): it fetches one
/// at rest, carries it over its target, and lets it go there.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SwallowFetch {
    /// It goes for a coconut at rest no farther from it than this (px).
    pub fetch_reach: f32,
    /// How fast it flies to a coconut (px/s).
    pub fetch_speed: f32,
    /// How fast it flies with one (px/s).
    pub laden_speed: f32,
    /// It carries the coconut this far over its target (px).
    pub carry_over: f32,
    /// It lets go within this of the place over its target, side to side (px).
    pub let_go_within: f32,
    /// It lets go no lower over its target than this (px).
    pub let_go_over: f32,
    /// It aims where its target will be after this long (s).
    pub lead_seconds: f32,
}

/// How often an active boss says an idle line.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdleBark {
    /// The mean time between two idle lines, in seconds.
    pub base_seconds: f32,
    /// Each interval is the mean plus or minus at most this, in seconds.
    pub jitter_seconds: f32,
}

/// How a move of the fighter `npc_gnu_ton` is made for the duel on the
/// giant's back (`bosses::gnu_ton::duel`).
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GnuTonDuelRules {
    /// The table's damage at and over which a move is a heavy one.
    pub heavy_damage: i32,
    /// What a move under `heavy_damage` does in the main game.
    pub light: DuelStrike,
    /// What a move at or over `heavy_damage` does in the main game.
    pub heavy: DuelStrike,
    /// A move's reach is larger by this on the giant.
    pub reach: f32,
    /// A volume with its middle at or over this height (up is negative) is
    /// over his head; the others are put down at the player's height.
    pub overhead: f32,
    /// The height a volume that is not over his head is put at.
    pub at_the_player: f32,
    /// A volume that is put down is at least this tall, as a half height.
    pub least_half_height: f32,
}

/// A duel move's effect in the main game.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DuelStrike {
    /// Hearts of damage.
    pub hearts: i32,
    /// The fixed launch (a boss's strike is 1.25).
    pub launch: f32,
}

impl AmbitionRules {
    /// Each value the game cannot use, named by its field.
    fn problems(&self) -> Vec<String> {
        let mut problems = Vec::new();
        for (id, impact) in &self.thrown_impacts {
            if !(impact.min_speed.is_finite() && impact.min_speed > 0.0) {
                problems.push(format!(
                    "`thrown_impacts.{id}.min_speed` is {}; it is a finite speed more than 0",
                    impact.min_speed
                ));
            }
            if impact.damage < 1 {
                problems.push(format!("`thrown_impacts.{id}.damage` is {}; a row that hurts deals at least 1", impact.damage));
            }
        }
        let bark = self.boss_idle_bark;
        if !(bark.base_seconds.is_finite() && bark.jitter_seconds.is_finite() && bark.jitter_seconds >= 0.0) {
            problems.push(format!(
                "`boss_idle_bark` is {bark:?}; both are finite seconds and the jitter is not negative"
            ));
        } else if bark.base_seconds - bark.jitter_seconds <= 0.0 {
            problems.push(format!(
                "`boss_idle_bark.base_seconds` {} less `jitter_seconds` {} is not more than 0: \
                 an interval could be zero or negative, so a boss would bark each frame",
                bark.base_seconds, bark.jitter_seconds
            ));
        }
        let duel = self.gnu_ton_duel;
        if duel.heavy_damage < 1 {
            problems.push(format!("`gnu_ton_duel.heavy_damage` is {}; it is at least 1", duel.heavy_damage));
        }
        for (field, strike) in [("light", duel.light), ("heavy", duel.heavy)] {
            if strike.hearts < 0 || !(strike.launch.is_finite() && strike.launch >= 0.0) {
                problems.push(format!(
                    "`gnu_ton_duel.{field}` is {strike:?}; hearts are not negative and the launch is finite and not negative"
                ));
            }
        }
        for (field, value) in [("reach", duel.reach), ("least_half_height", duel.least_half_height)] {
            if !(value.is_finite() && value > 0.0) {
                problems.push(format!("`gnu_ton_duel.{field}` is {value}; it is finite and more than 0"));
            }
        }
        let bird = self.african_swallow;
        for (field, value) in [
            ("fetch_reach", bird.fetch_reach),
            ("fetch_speed", bird.fetch_speed),
            ("laden_speed", bird.laden_speed),
            ("carry_over", bird.carry_over),
            ("let_go_within", bird.let_go_within),
            ("let_go_over", bird.let_go_over),
            ("lead_seconds", bird.lead_seconds),
        ] {
            if !(value.is_finite() && value >= 0.0) {
                problems.push(format!("`african_swallow.{field}` is {value}; it is finite and not negative"));
            }
        }
        if bird.let_go_over > bird.carry_over {
            problems.push(format!(
                "`african_swallow.let_go_over` {} is more than `carry_over` {}: the bird carries the coconut \
                 lower than it may let go, so it never lets go",
                bird.let_go_over, bird.carry_over
            ));
        }
        for (field, value) in [("overhead", duel.overhead), ("at_the_player", duel.at_the_player)] {
            if !value.is_finite() {
                problems.push(format!("`gnu_ton_duel.{field}` is {value}; it is a finite height"));
            }
        }
        problems
    }
}

/// The schema.
pub fn ambition_rules_schema() -> SchemaRegistration {
    rules_schema::<AmbitionRules>(
        AMBITION_RULES_SCHEMA,
        1,
        crate::AMBITION_CAPABILITY,
        "Ambition's own tuning: the idle bark cadence of a boss, how the GNU-ton \
         duel makes a fighter's move for the giant's back, and how an African \
         swallow fetches and drops a coconut.",
        AmbitionRules::problems,
    )
}

/// The tuning the pack states, or `None` when it has no `ambition_rules`
/// source.
pub fn lowered_ambition_rules(pack: &PreparedContentPack) -> Option<AmbitionRules> {
    lowered_rules::<AmbitionRules>(pack, AMBITION_RULES_SCHEMA).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules() -> AmbitionRules {
        AmbitionRules {
            boss_idle_bark: IdleBark { base_seconds: 7.0, jitter_seconds: 3.0 },
            gnu_ton_duel: GnuTonDuelRules {
                heavy_damage: 12,
                light: DuelStrike { hearts: 1, launch: 1.0 },
                heavy: DuelStrike { hearts: 2, launch: 1.7 },
                reach: 1.3,
                overhead: -30.0,
                at_the_player: 24.0,
                least_half_height: 30.0,
            },
            african_swallow: SwallowFetch {
                fetch_reach: 900.0,
                fetch_speed: 250.0,
                laden_speed: 185.0,
                carry_over: 150.0,
                let_go_within: 18.0,
                let_go_over: 90.0,
                lead_seconds: 0.35,
            },
        }
    }

    #[test]
    fn a_value_the_game_cannot_use_is_named_by_its_field() {
        assert!(rules().problems().is_empty(), "{:?}", rules().problems());
        let mut barks_each_frame = rules();
        barks_each_frame.boss_idle_bark.jitter_seconds = 7.0;
        let problems = barks_each_frame.problems();
        assert!(problems.iter().any(|problem| problem.contains("boss_idle_bark.base_seconds")), "{problems:?}");
        let mut no_reach = rules();
        no_reach.gnu_ton_duel.reach = 0.0;
        let problems = no_reach.problems();
        assert!(problems.iter().any(|problem| problem.contains("gnu_ton_duel.reach")), "{problems:?}");
    }
}

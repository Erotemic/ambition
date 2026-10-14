//! The `versus_rounds` schema: the rules of the versus stage's rounds, as data
//! in its pack (`assets/data/versus_rounds.ron`).
//!
//! Rounds and victory are this stage's policy, so the app owns the schema and
//! no engine capability reads it. The pack names it
//! (`content_pack! { schemas: [..] }`), and the pack's compile checks and
//! lowers the file as it does an engine schema's.

use ambition_platformer2d::content::{lowered_rules, rules_schema, SchemaRegistration};
use serde::Deserialize;

/// The authored FILE kind: the rules of the versus stage's rounds.
pub const VERSUS_ROUNDS_SCHEMA: &str = "versus_rounds";

/// The rules of the versus stage's rounds, as its pack states them.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VersusRounds {
    /// Round wins needed to take the match.
    pub rounds_to_win: u8,
    /// How long the KO is held before the next round starts, in seconds.
    pub ko_hold_seconds: f32,
    /// How long the match-over card stays up before the match resets, in
    /// seconds.
    pub match_hold_seconds: f32,
    /// How long a round-start countdown runs, in simulation ticks.
    pub countdown_ticks: u32,
    /// The last part of the countdown, in ticks, that reads FIGHT and not
    /// ROUND n.
    pub fight_call_ticks: u32,
}

impl VersusRounds {
    /// Each value the stage cannot use, named by its field.
    fn problems(&self) -> Vec<String> {
        let mut problems = Vec::new();
        if self.rounds_to_win == 0 {
            problems.push("`rounds_to_win` is 0; a match is taken by at least one round".to_string());
        }
        for (field, value) in [
            ("ko_hold_seconds", self.ko_hold_seconds),
            ("match_hold_seconds", self.match_hold_seconds),
        ] {
            if !(value.is_finite() && value >= 0.0) {
                problems.push(format!("`{field}` is {value}; it is finite and not negative"));
            }
        }
        if self.fight_call_ticks > self.countdown_ticks {
            problems.push(format!(
                "`fight_call_ticks` is {} and `countdown_ticks` is {}; the FIGHT call is the last \
                 part of the countdown, so it is not longer than the countdown",
                self.fight_call_ticks, self.countdown_ticks
            ));
        }
        problems
    }
}

/// The schema, for the versus pack's `content_pack! { schemas: [..] }`.
pub fn versus_rounds_schema() -> SchemaRegistration {
    rules_schema(
        VERSUS_ROUNDS_SCHEMA,
        1,
        super::super::versus::VERSUS_EXPERIENCE,
        "The rules of the versus stage's rounds: the wins that take the match, \
         the KO and match-over holds, the countdown and its FIGHT call.",
        VersusRounds::problems,
    )
}

/// The rules of the versus stage's rounds, as its pack states them
/// (`assets/data/versus_rounds.ron`).
pub fn versus_rounds() -> VersusRounds {
    *lowered_rules::<VersusRounds>(super::super::versus_fighters::PACK.prepared(), VERSUS_ROUNDS_SCHEMA)
        .expect("the versus pack states the rules of its rounds (`data/versus_rounds.ron`)")
}

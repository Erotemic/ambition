//! The `smash_rules` schema: the demo's own match flow and summon tuning, as
//! data in its pack (`assets/data/smash_rules.ron`).
//!
//! The match rules every fighting game shares (stocks, clocks, the fighter
//! body) are the engine's `match_rules`. These are what only this demo has:
//! the wait before the results card goes back to the select screen, and the
//! shark the pirate admiral summons. The demo owns the schema and no engine
//! capability reads it.

use ambition_platformer2d::content::{lowered_rules, rules_schema, SchemaRegistration};
use serde::Deserialize;

/// The authored FILE kind: the demo's match flow and summon tuning.
pub const SMASH_RULES_SCHEMA: &str = "smash_rules";

/// The demo's match flow and summon tuning, as its pack states them.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SmashRules {
    /// How long the results card stays before the select screen, in seconds.
    pub return_to_select_seconds: f32,
    /// The shark the admiral's up-B summons.
    pub shark_ride: SharkRide,
}

/// The shark the admiral's up-B summons, and how it leaves.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SharkRide {
    /// How near its summoner the shark must come to take it on, in px.
    pub board_radius: f32,
    /// How long the shark waits for its summoner, in seconds.
    pub board_deadline_seconds: f32,
    /// The punishment the shark takes before it dies.
    pub health: u32,
    /// How fast a dismissed shark leaves, in px per second.
    pub depart_speed: f32,
    /// How long a dismissed shark flies before it is removed, in seconds.
    pub depart_seconds: f32,
}

impl SmashRules {
    /// Each value the demo cannot use, named by its field.
    fn problems(&self) -> Vec<String> {
        let mut problems = Vec::new();
        let ride = &self.shark_ride;
        for (field, value) in [
            ("return_to_select_seconds", self.return_to_select_seconds),
            ("shark_ride.board_radius", ride.board_radius),
            ("shark_ride.board_deadline_seconds", ride.board_deadline_seconds),
            ("shark_ride.depart_speed", ride.depart_speed),
            ("shark_ride.depart_seconds", ride.depart_seconds),
        ] {
            if !(value.is_finite() && value >= 0.0) {
                problems.push(format!("`{field}` is {value}; it is finite and not negative"));
            }
        }
        if ride.health == 0 {
            problems.push("`shark_ride.health` is 0; the shark would die before it can carry anyone".to_string());
        }
        problems
    }
}

/// The schema, for the pack's `content_pack! { schemas: [..] }`.
pub fn smash_rules_schema() -> SchemaRegistration {
    rules_schema(
        SMASH_RULES_SCHEMA,
        1,
        "smash_demo",
        "Smash's own match flow and summon tuning: the results card's wait and \
         the admiral's shark.",
        SmashRules::problems,
    )
}

/// The rules, as the demo's pack states them (`assets/data/smash_rules.ron`).
pub fn smash_rules() -> SmashRules {
    *lowered_rules::<SmashRules>(crate::smash_pack::PACK.prepared(), SMASH_RULES_SCHEMA)
        .expect("the demo's pack states its rules (`data/smash_rules.ron`)")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The demo plays its pack's `smash_rules.ron`: an edit is the value the
    /// next start reads, and a value the demo cannot use is refused at
    /// admission with the file and the field named.
    #[test]
    fn smashs_rules_are_its_pack_file() {
        use ambition_platformer2d::content::EmbeddedPack;

        let authored = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/data/smash_rules.ron"))
            .expect("the demo's rules file is readable");
        let root = std::env::temp_dir().join(format!("smash_rules_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("data")).unwrap();
        std::fs::write(
            root.join("pack.ron"),
            format!(
                r#"(id: "smash_rules_probe", version: "0.1.0", namespace: "probe", requires: [],
                    sources: [(path: "data/smash_rules.ron", schema: "{SMASH_RULES_SCHEMA}", version: 1)])"#
            ),
        )
        .unwrap();
        let compile = |text: &str| {
            std::fs::write(root.join("data/smash_rules.ron"), text).unwrap();
            EmbeddedPack::rooted(Box::leak(root.display().to_string().into_boxed_str()), None)
                .with_schemas(|| vec![smash_rules_schema()])
                .compile()
        };
        let rules_of = |text: &str| {
            let prepared = compile(text).unwrap_or_else(|failure| panic!("{}", failure.render()));
            *lowered_rules::<SmashRules>(&prepared, SMASH_RULES_SCHEMA).expect("the probe pack lowers the rules")
        };

        let file = rules_of(&authored);
        assert_eq!(file, smash_rules(), "the demo plays the file");

        let health = "health: 40,";
        assert_eq!(authored.matches(health).count(), 1, "the probe edits the shark");
        assert_eq!(rules_of(&authored.replace(health, "health: 60,")).shark_ride.health, 60);

        let refused = match compile(&authored.replace(health, "health: 0,")) {
            Ok(_) => panic!("a shark with no health was admitted"),
            Err(failure) => failure.render(),
        };
        assert!(refused.contains("data/smash_rules.ron"), "the refusal names the file: {refused}");
        assert!(refused.contains("`shark_ride.health`"), "the refusal names the field: {refused}");
        let _ = std::fs::remove_dir_all(&root);
    }
}

//! The `smash_rules` schema: the demo's own match flow, as data in its pack
//! (`assets/data/smash_rules.ron`).
//!
//! The match rules every fighting game shares (stocks, clocks, the fighter
//! body) are the engine's `match_rules`. These are what only this demo has:
//! the select grid's roster and stand-ins, the wait before the results card
//! goes back to the select screen, and the swing of a fighter with no kit.
//! The shark the pirate admiral summons is his move's (`pirate_admiral.ron`).
//! The demo owns the schema and no engine capability reads it.

use ambition_platformer2d::content::{lowered_rules, rules_schema, SchemaRegistration};
use serde::Deserialize;

/// The authored FILE kind: the demo's match flow and summon tuning.
pub const SMASH_RULES_SCHEMA: &str = "smash_rules";

/// The demo's match flow and summon tuning, as its pack states them.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SmashRules {
    /// The fighter ids the select grid asks for, in grid order. The grid
    /// shows those the composition has (`SmashRoster::assemble`).
    pub roster: Vec<String>,
    /// The demo's copies of characters other packs own: each steps aside
    /// when the character it stands in for is in the composition.
    pub stand_ins: Vec<StandIn>,
    /// How long the results card stays before the select screen, in seconds.
    pub return_to_select_seconds: f32,
    /// Seconds one press burns off a sleep.
    pub sleep_mash_seconds: f32,
    /// The stock counts the select screen's stocks button offers, in the
    /// order it walks them.
    pub stock_choices: Vec<u32>,
    /// How long Back is held on the select screen before it leaves, in
    /// seconds. Holding Back is navigation; tapping Back is token
    /// manipulation.
    pub back_hold_to_leave_seconds: f32,
    /// How the select cursor feels.
    pub select_cursor: crate::select_screen::cursor::CursorFeel,
    /// The select screen's colours.
    pub select_look: crate::select_screen::SelectLook,
    /// The swing a fighter that authors no kit is seated with
    /// ([`crate::smash_seating_melee`]).
    pub seating_melee: ambition_platformer2d::character::MeleeActionSpec,
    /// The Limit every seat holds, and what fills it.
    pub limit: ambition_platformer2d::entity_catalog::smash_limit::LimitMeterFill,
}

/// A copy of a character another pack owns, on this demo's grid.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StandIn {
    /// The demo's copy (a roster id).
    pub copy: String,
    /// The character it stands in for.
    pub stands_in_for: String,
}

impl SmashRules {
    /// Each value the demo cannot use, named by its field.
    fn problems(&self) -> Vec<String> {
        let mut problems = Vec::new();
        for (field, value) in [
            ("return_to_select_seconds", self.return_to_select_seconds),
            ("sleep_mash_seconds", self.sleep_mash_seconds),
            ("back_hold_to_leave_seconds", self.back_hold_to_leave_seconds),
        ] {
            if !(value.is_finite() && value >= 0.0) {
                problems.push(format!("`{field}` is {value}; it is finite and not negative"));
            }
        }
        let mut seen = std::collections::BTreeSet::new();
        for (index, id) in self.roster.iter().enumerate() {
            if !seen.insert(id.as_str()) {
                problems.push(format!("`roster[{index}]` (`{id}`) is on the roster twice; the grid shows each fighter once"));
            }
        }
        // A composition with no other pack has only the demo's own fighters:
        // without them on the roster, its grid is empty.
        for own in crate::select::OWN_FIGHTERS {
            if !self.roster.iter().any(|id| id == own) {
                problems.push(format!("`roster` does not name `{own}`, the demo's own fighter"));
            }
        }
        for (index, stand_in) in self.stand_ins.iter().enumerate() {
            if !self.roster.contains(&stand_in.copy) {
                problems.push(format!("`stand_ins[{index}].copy` (`{}`) is not on the roster", stand_in.copy));
            }
        }
        if self.stock_choices.is_empty() {
            problems.push("`stock_choices` is empty; the stocks button offers at least one count".to_string());
        }
        for (index, count) in self.stock_choices.iter().enumerate() {
            if *count == 0 {
                problems.push(format!("`stock_choices[{index}]` is 0; a fighter with no stock is out before the match starts"));
            }
            if self.stock_choices[..index].contains(count) {
                problems.push(format!("`stock_choices[{index}]` repeats {count}; the stocks button would offer it twice"));
            }
        }
        let melee = self.seating_melee;
        if !(melee.total_duration_s().is_finite() && melee.total_duration_s() > 0.0) {
            problems.push(format!("`seating_melee` lasts {} s; a swing takes some time", melee.total_duration_s()));
        }
        if melee.damage() < 0 {
            problems.push(format!("`seating_melee` deals {} damage; it is not negative", melee.damage()));
        }
        if !(melee.reach_px().is_finite() && melee.reach_px() > 0.0) {
            problems.push(format!("`seating_melee` reaches {} px; a swing reaches past the body", melee.reach_px()));
        }
        problems.extend(self.limit.problems().into_iter().map(|problem| format!("`limit`: {problem}")));
        problems.extend(self.select_cursor.problems().into_iter().map(|problem| format!("`select_cursor`: {problem}")));
        problems.extend(self.select_look.problems().into_iter().map(|problem| format!("`select_look`: {problem}")));
        problems
    }
}

/// The schema, for the pack's `content_pack! { schemas: [..] }`.
pub fn smash_rules_schema() -> SchemaRegistration {
    rules_schema(
        SMASH_RULES_SCHEMA,
        1,
        "smash_demo",
        "Smash's own match flow: the results card's wait and the kitless \
         fighter's swing.",
        SmashRules::problems,
    )
}

/// The rules, as the demo's pack states them (`assets/data/smash_rules.ron`).
pub fn smash_rules() -> &'static SmashRules {
    lowered_rules::<SmashRules>(crate::smash_pack::PACK.prepared(), SMASH_RULES_SCHEMA)
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
            lowered_rules::<SmashRules>(&prepared, SMASH_RULES_SCHEMA).expect("the probe pack lowers the rules").clone()
        };

        let file = rules_of(&authored);
        assert_eq!(&file, smash_rules(), "the demo plays the file");
        // The Limit the seats hold and fill is the file's `limit`.
        assert_eq!(crate::limit::smash_limit(), file.limit, "the match plays a Limit that is not the file's");
        let cap = "cap: 60.0,";
        assert_eq!(authored.matches(cap).count(), 1, "the probe edits the Limit");
        assert_eq!(rules_of(&authored.replace(cap, "cap: 80.0,")).limit.cap, 80.0);
        let refused = match compile(&authored.replace("on_block: 1.0,", "on_block: -1.0,")) {
            Ok(_) => panic!("a Limit that a block drains was admitted"),
            Err(failure) => failure.render(),
        };
        assert!(refused.contains("`limit`: on_block is -1"), "the refusal names the field: {refused}");
        // The stocks button's counts are the file's, and a repeat is refused.
        let choices = "stock_choices: [1, 3, 5],";
        assert_eq!(authored.matches(choices).count(), 1, "the probe edits the stock choices");
        assert_eq!(rules_of(&authored.replace(choices, "stock_choices: [1, 3, 5, 7],")).stock_choices, vec![1, 3, 5, 7]);
        let refused = match compile(&authored.replace(choices, "stock_choices: [1, 3, 3],")) {
            Ok(_) => panic!("a stocks button that offers one count twice was admitted"),
            Err(failure) => failure.render(),
        };
        assert!(refused.contains("`stock_choices[2]` repeats 3"), "the refusal names the field: {refused}");

        // The select cursor's feel is the file's, and a build that takes no
        // time is refused.
        assert_eq!(crate::select_screen::cursor::cursor_feel(), file.select_cursor, "the cursor does not feel as the file states");
        let speed = "cells_per_second: 4.5,";
        assert_eq!(authored.matches(speed).count(), 1, "the probe edits the cursor speed");
        assert_eq!(rules_of(&authored.replace(speed, "cells_per_second: 6.0,")).select_cursor.cells_per_second, 6.0);
        let refused = match compile(&authored.replace("ramp_to_full_seconds: 0.6,", "ramp_to_full_seconds: 0.0,")) {
            Ok(_) => panic!("a cursor build that takes no time was admitted"),
            Err(failure) => failure.render(),
        };
        assert!(refused.contains("`select_cursor`: `ramp_to_full_seconds` is 0"), "the refusal names the field: {refused}");

        // The select screen's colours are the file's, and a colour that is
        // not one is refused.
        assert_eq!(crate::select_screen::select_look(), file.select_look, "the select screen does not wear the file's colours");
        let refused = match compile(&authored.replace("ink: (0.94, 0.96, 1.0),", "ink: (0.94, 0.96, 1.5),")) {
            Ok(_) => panic!("an ink brighter than white was admitted"),
            Err(failure) => failure.render(),
        };
        assert!(refused.contains("`select_look`: `ink` is"), "the refusal names the field: {refused}");
        let refused = match compile(&authored.replace("token_rim: (1.0, 1.0, 1.0, 0.85),", "token_rim: (1.0, 1.0, 1.0, 1.85),")) {
            Ok(_) => panic!("a token rim more than opaque was admitted"),
            Err(failure) => failure.render(),
        };
        assert!(refused.contains("`select_look`: `token_rim` is"), "the refusal names the field: {refused}");

        // The swing a kitless fighter is seated with is the file's, and one
        // that reaches nothing is refused.
        assert_eq!(crate::smash_seating_melee(), file.seating_melee, "a kitless seat swings a melee that is not the file's");
        let reach = "reach_px: 34.0))";
        assert_eq!(authored.matches(reach).count(), 1, "the probe edits the seating swing");
        assert_eq!(rules_of(&authored.replace(reach, "reach_px: 40.0))")).seating_melee.reach_px(), 40.0);
        let refused = match compile(&authored.replace(reach, "reach_px: 0.0))")) {
            Ok(_) => panic!("a seating swing that reaches nothing was admitted"),
            Err(failure) => failure.render(),
        };
        assert!(refused.contains("data/smash_rules.ron"), "the refusal names the file: {refused}");
        assert!(refused.contains("`seating_melee` reaches 0 px"), "the refusal names the field: {refused}");

        // The roster is the file's: an edit is the grid the next start asks
        // for, and a roster without the demo's own fighter is refused.
        let last = r#""medic","#;
        assert_eq!(authored.matches(last).count(), 1, "the probe edits one roster row");
        let edited = rules_of(&authored.replace(last, r#""medic", "npc_gnu_ton_twin","#));
        assert_eq!(edited.roster.iter().filter(|id| *id == "npc_gnu_ton_twin").count(), 1);
        let own = format!("\n        \"{}\",\n", crate::SMASH_OPPONENT_ID);
        assert_eq!(authored.matches(&own).count(), 1, "the probe removes one roster row");
        let refused = match compile(&authored.replace(&own, "\n")) {
            Ok(_) => panic!("a roster without the demo's own fighter was admitted"),
            Err(failure) => failure.render(),
        };
        assert!(
            refused.contains(&format!("`roster` does not name `{}`", crate::SMASH_OPPONENT_ID)),
            "the refusal names the fighter: {refused}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}

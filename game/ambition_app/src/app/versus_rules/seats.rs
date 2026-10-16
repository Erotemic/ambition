//! The `versus_seats` schema: who the versus stage seats, as data in its
//! pack (`assets/data/versus_seats.ron`).
//!
//! Seating is this stage's policy, so the app owns the schema and no engine
//! capability reads it.

use ambition_platformer2d::content::{lowered_rules, rules_schema, PreparedContentPack, SchemaRegistration};
use serde::Deserialize;

/// The authored FILE kind: who the versus stage seats.
pub const VERSUS_SEATS_SCHEMA: &str = "versus_seats";

/// Who the versus stage seats, as its pack states it.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VersusSeats {
    /// The characters of the pack's catalog that the seats play: seat `n`
    /// plays `fighters[n % len]`. The first is also the stage's catalog
    /// default, which is read when the stage is composed, so an edit to it
    /// is played after a restart.
    pub fighters: Vec<String>,
    /// The brain profile a CPU seat plays (a row of the pack's catalog).
    pub cpu_brain: String,
    /// The team names, in seat order: seat `n` is on `teams[n % len]`. Seats
    /// on one team cannot hurt each other.
    pub teams: Vec<String>,
}

impl VersusSeats {
    /// Each value the stage cannot seat from, named by its field.
    fn problems(&self) -> Vec<String> {
        let mut problems = Vec::new();
        for (field, list) in [("fighters", &self.fighters), ("teams", &self.teams)] {
            if list.is_empty() {
                problems.push(format!("`{field}` is empty; each seat takes one"));
            }
            for (index, value) in list.iter().enumerate() {
                if value.trim().is_empty() {
                    problems.push(format!("`{field}[{index}]` is empty"));
                }
            }
        }
        for (index, team) in self.teams.iter().enumerate() {
            if self.teams[..index].contains(team) {
                problems.push(format!("`teams[{index}]` repeats `{team}`; two sides would be one team"));
            }
        }
        if self.cpu_brain.trim().is_empty() {
            problems.push("`cpu_brain` is empty; a CPU seat plays a brain profile".to_string());
        }
        problems
    }

    /// The character seat `seat` plays.
    pub fn fighter(&self, seat: usize) -> &str {
        &self.fighters[seat % self.fighters.len()]
    }

    /// The team seat `seat` is on.
    pub fn team(&self, seat: usize) -> &str {
        &self.teams[seat % self.teams.len()]
    }
}

/// The schema, for the versus pack's `content_pack! { schemas: [..] }`.
pub fn versus_seats_schema() -> SchemaRegistration {
    rules_schema(
        VERSUS_SEATS_SCHEMA,
        1,
        super::super::versus::VERSUS_EXPERIENCE,
        "Who the versus stage seats: the fighters in seat order, the CPU \
         seats' brain profile and the teams.",
        VersusSeats::problems,
    )
}

/// Who the stage seats, as `pack` states it (`assets/data/versus_seats.ron`).
pub fn versus_seats_in(pack: &PreparedContentPack) -> Option<&VersusSeats> {
    lowered_rules::<VersusSeats>(pack, VERSUS_SEATS_SCHEMA)
}

/// Who the stage seats, as the versus pack states it.
pub fn versus_seats() -> &'static VersusSeats {
    versus_seats_in(super::super::versus_fighters::PACK.prepared())
        .expect("the versus pack states who it seats (`data/versus_seats.ron`)")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The stage seats what its pack's `versus_seats.ron` states: an edit is
    /// the roster the next match seats, and a list the stage cannot seat
    /// from is refused at admission with the file and the field named.
    #[test]
    fn the_versus_seats_are_its_pack_file() {
        use ambition_platformer2d::content::EmbeddedPack;
        use ambition_platformer2d::versus_match::{ControllerBinding, RosterSeating};

        let authored = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/data/versus_seats.ron"))
            .expect("the versus seats file is readable");
        let root = std::env::temp_dir().join(format!("versus_seats_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("data")).unwrap();
        std::fs::write(
            root.join("pack.ron"),
            format!(
                r#"(id: "versus_seats_probe", version: "0.1.0", namespace: "probe", requires: [],
                    sources: [(path: "data/versus_seats.ron", schema: "{VERSUS_SEATS_SCHEMA}", version: 1)])"#
            ),
        )
        .unwrap();
        let compile = |text: &str| {
            std::fs::write(root.join("data/versus_seats.ron"), text).unwrap();
            EmbeddedPack::rooted(Box::leak(root.display().to_string().into_boxed_str()), None)
                .with_schemas(|| vec![versus_seats_schema()])
                .compile()
        };
        let seats_of = |text: &str| {
            let prepared = compile(text).unwrap_or_else(|failure| panic!("{}", failure.render()));
            versus_seats_in(&prepared).expect("the probe pack lowers the seats").clone()
        };
        // One human and three CPUs: each seat names its fighter, its team and,
        // for a CPU, its brain.
        let roster_of = |seats: &VersusSeats| {
            crate::app::versus::versus_roster_seating(seats, 1, RosterSeating::Proposed)
                .participants
                .into_iter()
                .map(|seat| {
                    let brain = match seat.controller {
                        ControllerBinding::Cpu { brain_profile } => brain_profile.map(|profile| profile.to_string()),
                        _ => None,
                    };
                    (seat.character.to_string(), seat.team.map(|team| team.to_string()), brain)
                })
                .collect::<Vec<_>>()
        };

        assert_eq!(&seats_of(&authored), versus_seats(), "the stage seats the file");
        let fighters = r#"fighters: ["arena_duelist_long", "arena_duelist_close"],"#;
        let teams = r#"teams: ["blue", "red"],"#;
        let brain = r#"cpu_brain: "versus_duelist","#;
        for line in [fighters, teams, brain] {
            assert_eq!(authored.matches(line).count(), 1, "the probe edits `{line}`");
        }
        let edited = seats_of(
            &authored
                .replace(fighters, r#"fighters: ["arena_duelist_close"],"#)
                .replace(teams, r#"teams: ["gold", "green"],"#)
                .replace(brain, r#"cpu_brain: "probe_brain","#),
        );
        let roster = roster_of(&edited);
        assert_eq!(roster.len(), 2, "one human still meets one opponent");
        assert_eq!(
            roster,
            vec![
                ("arena_duelist_close".to_string(), Some("gold".to_string()), None),
                ("arena_duelist_close".to_string(), Some("green".to_string()), Some("probe_brain".to_string())),
            ],
            "the roster is not the seats row's"
        );

        let refused = match compile(&authored.replace(teams, r#"teams: ["blue", "blue"],"#)) {
            Ok(_) => panic!("two sides with one team name were admitted"),
            Err(failure) => failure.render(),
        };
        assert!(refused.contains("data/versus_seats.ron"), "the refusal names the file: {refused}");
        assert!(refused.contains("`teams[1]` repeats `blue`"), "the refusal names the field: {refused}");
        let _ = std::fs::remove_dir_all(&root);
    }
}

//! The `npc_lives` schema: authored NPC lives as data in Ambition's pack
//! (`assets/data/npc_lives.ron`).
//!
//! A life is a planning domain (`ambition_htn`) and the facts one character
//! starts with: two scientists share one domain and keep their own hours as
//! facts. `holds` says how long a verb keeps a body at its task, in hours of
//! the game's day. The executor that turns a plan's steps into goals on a
//! body is the engine's (`body::life`); which characters live, and how, is
//! this file. Design: `docs/planning/engine/radiant-npc-planning.md`.

use std::collections::BTreeMap;

use ambition_content_pack::{lowered_rules, rules_schema, PreparedContentPack, SchemaRegistration};
use ambition_htn::{Domain, Value};
use serde::Deserialize;

/// The authored FILE kind.
pub const NPC_LIVES_SCHEMA: &str = "npc_lives";

/// One character's life: the domain it plans with, its own facts, and the
/// facts it reads from the world when it plans.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LifeRow {
    pub domain: String,
    /// The facts the character starts with (what it has done).
    #[serde(default)]
    pub facts: BTreeMap<String, Value>,
    /// The facts the author owns (its hours): read from the book each time
    /// the character plans, never kept as its own.
    #[serde(default)]
    pub authored: BTreeMap<String, Value>,
    /// Facts the character keeps that every day starts from again (`ate`:
    /// false): each takes this value on the first plan of a new day.
    #[serde(default)]
    pub daily: BTreeMap<String, Value>,
    /// What the character says for each `say` topic of its domain.
    #[serde(default)]
    pub lines: BTreeMap<String, String>,
    #[serde(default)]
    pub observes: BTreeMap<String, Observation>,
}

/// A fact the world states, read when a body plans.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub enum Observation {
    /// No other body in the room holds a claim on this thing.
    Free(String),
    /// A body of this character is in the room.
    Present(String),
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NpcLives {
    /// Planning domains by name.
    pub domains: BTreeMap<String, Domain>,
    /// Lives by the character id of the body that lives them.
    pub lives: BTreeMap<String, LifeRow>,
    /// How long each verb keeps a body at its task (hours). A verb not named
    /// is done the tick it starts; `go_to` is done on arrival.
    #[serde(default)]
    pub holds: BTreeMap<String, f32>,
}

fn problems(lives: &NpcLives) -> Vec<String> {
    let mut problems = Vec::new();
    for (name, domain) in &lives.domains {
        if let Err(error) = domain.validate() {
            problems.push(format!("`domains.{name}`: {error}"));
        }
    }
    for (character, life) in &lives.lives {
        if !lives.domains.contains_key(&life.domain) {
            problems.push(format!("`lives.{character}` plans with `{}`, which no domain is", life.domain));
        }
        for (kind, names) in [
            ("observes", life.observes.keys().collect::<Vec<_>>()),
            ("authors", life.authored.keys().collect()),
            ("starts each day with", life.daily.keys().collect()),
        ] {
            if let Some(name) = names.iter().find(|name| life.facts.contains_key(**name)) {
                problems.push(format!("`lives.{character}` both keeps and {kind} `{name}`; the {kind} value replaces the kept one"));
            }
        }
        if let Some(name) = life.observes.keys().find(|name| life.authored.contains_key(*name)) {
            problems.push(format!("`lives.{character}` both authors and observes `{name}`"));
        }
        if let Some(name) = life.daily.keys().find(|name| life.authored.contains_key(*name) || life.observes.contains_key(*name)) {
            problems.push(format!("`lives.{character}` starts each day with `{name}`, which it also authors or observes"));
        }
    }
    // The verbs the engine's executor carries out itself (`body::life`);
    // any other verb a domain uses must be a declared hold, or a step with it
    // would stop every plan it is in.
    const ENGINE_VERBS: [&str; 4] = ["go_to", "claim", "release", "say"];
    for (name, domain) in &lives.domains {
        for (task, primitive) in &domain.primitive {
            let verb = primitive.action.verb.as_str();
            if !ENGINE_VERBS.contains(&verb) && !lives.holds.contains_key(verb) {
                problems.push(format!(
                    "`domains.{name}.primitive.{task}` uses the verb `{verb}`, which the executor does not carry out; \
                     declare it in `holds` (a timed task) or use one of {ENGINE_VERBS:?}"
                ));
            }
        }
    }
    for (character, life) in &lives.lives {
        let Some(domain) = lives.domains.get(&life.domain) else { continue };
        for (task, primitive) in &domain.primitive {
            if primitive.action.verb == "say" {
                let topic = primitive.action.target.clone().unwrap_or_default();
                if !life.lines.contains_key(&topic) {
                    problems.push(format!("`lives.{character}` has no line for `{topic}`, which `{task}` says"));
                }
            }
        }
    }
    for (verb, hours) in &lives.holds {
        if !(hours.is_finite() && *hours >= 0.0) {
            problems.push(format!("`holds.{verb}` is {hours}; a hold is hours, 0 or more"));
        }
    }
    problems
}

/// The schema, for Ambition's pack.
pub fn npc_lives_schema() -> SchemaRegistration {
    rules_schema::<NpcLives>(
        NPC_LIVES_SCHEMA,
        1,
        crate::AMBITION_CAPABILITY,
        "Ambition's NPC lives: planning domains, the characters that plan with \
         them and their own facts, and how long each verb holds a body.",
        problems,
    )
}

/// The lives `pack` states, if it states any.
pub fn lowered_npc_lives(pack: &PreparedContentPack) -> Option<&NpcLives> {
    lowered_rules::<NpcLives>(pack, NPC_LIVES_SCHEMA)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lives(verb: &str, lines: &str) -> NpcLives {
        ron::from_str(&format!(
            r#"(
                domains: {{ "d": (root: "day", compound: {{ "day": [(name: "talk", subtasks: ["talk"])] }},
                    primitive: {{ "talk": (action: (verb: "{verb}", target: Some("topic"))) }}) }},
                lives: {{ "someone": (domain: "d", lines: {{ {lines} }}) }},
                holds: {{ "use": 0.25 }},
            )"#
        ))
        .expect("the test lives parse")
    }

    /// A verb the executor does not carry out, and no hold declares, is
    /// refused: a step with it would never be done.
    #[test]
    fn a_verb_nothing_carries_out_is_refused() {
        assert!(problems(&lives("dance", "")).iter().any(|problem| problem.contains("the verb `dance`")));
        assert!(problems(&lives("use", "")).is_empty(), "a declared hold is carried out");
    }

    /// A `say` with no authored line for its topic is refused.
    #[test]
    fn a_line_nobody_wrote_is_refused() {
        assert!(problems(&lives("say", "")).iter().any(|problem| problem.contains("no line for `topic`")));
        assert!(problems(&lives("say", r#""topic": "hello""#)).is_empty());
    }
}

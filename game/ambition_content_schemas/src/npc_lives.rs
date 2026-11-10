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

/// One character's life: the domain it plans with and its own facts.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LifeRow {
    pub domain: String,
    #[serde(default)]
    pub facts: BTreeMap<String, Value>,
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

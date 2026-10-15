//! The `flag_chains` schema: save flags that set other save flags, as data in
//! Ambition's pack (`assets/data/flag_chains.ron`).
//!
//! A chain is a story consequence: when the save holds `when`, it also gets
//! `set`, through the ordinary flag path (so a quest waiting for `set` hears
//! it). The game reads the rows (`intro::route_state`); the engine does not.

use std::collections::BTreeSet;

use ambition_content_pack::{lowered_rules, rules_schema, PreparedContentPack, SchemaRegistration};
use serde::Deserialize;

/// The authored FILE kind.
pub const FLAG_CHAINS_SCHEMA: &str = "flag_chains";

/// One chain: the save flag `when` also sets the save flag `set`.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlagChain {
    pub when: String,
    pub set: String,
}

/// Each row the chains cannot run, named by its index. `&Vec` because
/// `rules_schema` passes `&T` and the file is a list.
#[allow(clippy::ptr_arg)]
fn problems(chains: &Vec<FlagChain>) -> Vec<String> {
    let mut problems = Vec::new();
    let mut triggers = BTreeSet::new();
    for (index, chain) in chains.iter().enumerate() {
        if chain.when.trim().is_empty() || chain.set.trim().is_empty() {
            problems.push(format!("`[{index}]` has an empty flag; no save holds it"));
        }
        if chain.when == chain.set {
            problems.push(format!("`[{index}].set` is its own `when` (`{}`); it sets nothing new", chain.when));
        }
        // Two rows with one trigger set two flags from one fact: write them as
        // a chain (`a` sets `b`, `b` sets `c`), so the order is stated.
        if !triggers.insert(chain.when.as_str()) {
            problems.push(format!("`[{index}].when` (`{}`) is the trigger of an earlier row", chain.when));
        }
    }
    problems
}

/// The schema, for Ambition's pack.
pub fn flag_chains_schema() -> SchemaRegistration {
    rules_schema::<Vec<FlagChain>>(
        FLAG_CHAINS_SCHEMA,
        1,
        crate::AMBITION_CAPABILITY,
        "Ambition's flag chains: a save flag that also sets another save flag, \
         through the ordinary flag path.",
        problems,
    )
}

/// The chains `pack` states, if it states any.
pub fn lowered_flag_chains(pack: &PreparedContentPack) -> Option<&Vec<FlagChain>> {
    lowered_rules::<Vec<FlagChain>>(pack, FLAG_CHAINS_SCHEMA)
}

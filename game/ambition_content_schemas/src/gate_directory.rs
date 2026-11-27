//! The `gate_directory` schema: what Ambition's gate terminals list, as data
//! in its pack (`assets/data/gate_directory.ron`).
//!
//! The engine's gate network (`gate_network.ron`) dials rings by address and
//! keeps no knowledge. Which addresses a traveller knows is Ambition's: the
//! addresses written on each terminal (`posted`), and whether a traveller
//! starts knowing every address (`start_known`). The terminal reads them
//! (`ambition_content::gate_terminal::known_addresses`); that each posted
//! address is one the network answers is checked against the network by
//! Ambition's tests, since a file's own check sees only the file.

use std::collections::BTreeMap;

use ambition_content_pack::{lowered_rules, rules_schema, PreparedContentPack, SchemaRegistration};
use serde::Deserialize;

/// The authored FILE kind.
pub const GATE_DIRECTORY_SCHEMA: &str = "gate_directory";

/// Which addresses a traveller knows before learning any.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
pub enum GateStartKnowledge {
    /// Only what the traveller learned: a dial that answered, a sign, a
    /// person, a document.
    #[default]
    Discovered,
    /// Every address of the network, hidden ones too: a development game that
    /// starts with everything unlocked.
    All,
}

/// What Ambition's terminals list.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GateDirectory {
    #[serde(default)]
    pub start_known: GateStartKnowledge,
    /// The addresses written on each ring's terminal (by the ring's zone),
    /// listed first and known without learning: every terminal away from home
    /// posts a way home.
    #[serde(default)]
    pub posted: BTreeMap<String, Vec<String>>,
}

impl GateDirectory {
    /// The addresses written on the terminal of the ring `zone`.
    pub fn posted(&self, zone: &str) -> &[String] {
        self.posted.get(zone).map(Vec::as_slice).unwrap_or_default()
    }
}

fn problems(directory: &GateDirectory) -> Vec<String> {
    let mut problems = Vec::new();
    for (zone, addresses) in &directory.posted {
        if zone.trim().is_empty() {
            problems.push("a `posted` row names no ring".to_string());
        }
        if addresses.is_empty() {
            problems.push(format!("`posted.{zone}` posts nothing; leave the ring out"));
        }
        for (index, address) in addresses.iter().enumerate() {
            if address.trim().is_empty() {
                problems.push(format!("`posted.{zone}[{index}]` is empty"));
            }
            if addresses[..index].contains(address) {
                problems.push(format!("`posted.{zone}` posts `{address}` twice"));
            }
        }
    }
    problems
}

/// The schema, for Ambition's pack.
pub fn gate_directory_schema() -> SchemaRegistration {
    rules_schema::<GateDirectory>(
        GATE_DIRECTORY_SCHEMA,
        1,
        crate::AMBITION_CAPABILITY,
        "Ambition's gate directory: the addresses written on each terminal, and \
         whether a traveller starts knowing every address.",
        problems,
    )
}

/// The directory `pack` states, if it states one.
pub fn lowered_gate_directory(pack: &PreparedContentPack) -> Option<&GateDirectory> {
    lowered_rules::<GateDirectory>(pack, GATE_DIRECTORY_SCHEMA)
}

//! The `gate_network` schema: Ambition's gate network as its pack writes it
//! (`assets/data/gate_network.ron`), and its lowering into the engine's
//! catalog.
//!
//! The engine dials numbered chevrons ([`GateNetworkCatalog`]). Ambition
//! writes a ring's chevrons as LETTERS: the ring's inscription is one letter
//! per chevron, and an important ring's address is a word, which spells into
//! chevrons by the inscription's own rule ([`GateWords::spell`]): when a
//! letter is on several chevrons, its k-th use in the word takes the k-th of
//! them. Any other ring's address is its chevrons, as authored (`glyphs`: a
//! random string, not a word). The words stay the game's ([`GateWords`]):
//! what a terminal shows, and what the save's known flags are keyed by.

use std::collections::BTreeMap;

use ambition_content_pack::{lowered_rules, rules_schema, PreparedContentPack, SchemaRegistration};
use ambition_platformer2d_world::rooms::{
    default_gate_idle_timeout_secs, gate_network_problems, GateFacility, GateNetworkCatalog, GateRing, GateStatusRule,
    GateSymbol, RingKind,
};

/// One chevron of a ring: Ambition's name for the engine's numbered symbol.
pub type Chevron = GateSymbol;
use serde::Deserialize;

/// The authored FILE kind.
pub const GATE_NETWORK_SCHEMA: &str = "gate_network";

/// Ambition's gate network, as written.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthoredGateNetwork {
    /// Idle simulation seconds before an unused connection closes.
    #[serde(default = "default_gate_idle_timeout_secs")]
    pub idle_timeout_secs: u32,
    /// The letters on a ring's chevrons, one per chevron, in order: the
    /// chevrons a terminal shows. A letter may stand on several chevrons;
    /// each chevron is its own symbol.
    pub inscription: String,
    /// Each logistical facility that holds rings.
    pub facilities: Vec<AuthoredGateFacility>,
    /// Each ring of the network.
    pub rings: Vec<AuthoredGateRing>,
}

/// One logistical facility: a gate station and the rings it holds. Two
/// rings in one facility never dial each other (that is a walk); two
/// facilities of one dimension can.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthoredGateFacility {
    pub id: String,
    /// What a terminal calls it.
    pub name: String,
    /// The dimension it is in: the engine's world, which a portal does not
    /// leave.
    pub dimension: String,
}

/// One ring, as written: the engine's ring with its address as a word or as
/// chevrons.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthoredGateRing {
    pub zone: String,
    pub name: String,
    pub facility: String,
    pub room: String,
    /// The word whose chevrons reach this ring: an important ring's address.
    #[serde(default)]
    pub address: Option<String>,
    /// The chevrons that reach this ring, when its address is no word. A ring
    /// has exactly one of `address` and `glyphs`.
    #[serde(default)]
    pub glyphs: Option<Vec<Chevron>>,
    #[serde(default)]
    pub hidden: bool,
    #[serde(default)]
    pub terminal: Option<String>,
    #[serde(default)]
    pub status: GateStatusRule,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub kind: RingKind,
    #[serde(default)]
    pub reach: Option<u32>,
}

/// The words of Ambition's network: the inscription a ring shows, and each
/// ring's address word (by zone) when its address is one.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GateWords {
    /// The letter on each chevron, in order.
    pub inscription: Vec<char>,
    /// Each word address, by ring zone.
    pub words: BTreeMap<String, String>,
}

impl GateWords {
    /// The chevrons a word spells: each letter in turn takes the next chevron
    /// that carries it, so the k-th use of a letter takes the k-th chevron of
    /// it, round again when the word uses it more often than the ring has it
    /// ("LEGALLY DISTINCT" spells every chevron in order). The word is
    /// normalized first. `Err`: a letter on no chevron.
    pub fn spell(&self, word: &str) -> Result<Vec<Chevron>, char> {
        spell(&self.inscription, word)
    }

    /// The address word of the ring `zone`, if its address is a word.
    pub fn word(&self, zone: &str) -> Option<&str> {
        self.words.get(zone).map(String::as_str)
    }

    /// The ring whose address is the word `word` (normalized first).
    pub fn zone_named(&self, word: &str) -> Option<&str> {
        let word = normalize_address(word);
        self.words.iter().find(|(_, named)| **named == word).map(|(zone, _)| zone.as_str())
    }

    /// What the save's known flag of `ring`'s address is keyed by: its word,
    /// or, for a ring whose address is glyphs, its zone.
    pub fn address_key<'a>(&'a self, ring: &'a GateRing) -> &'a str {
        self.word(&ring.zone).unwrap_or(&ring.zone)
    }

    /// The word of the address `address` in `catalog`, if its ring has one.
    pub fn word_of<'a>(&'a self, catalog: &GateNetworkCatalog, address: &[Chevron]) -> Option<&'a str> {
        catalog.ring_at(address).and_then(|ring| self.word(&ring.zone))
    }
}

/// An address word as Ambition compares it: no spaces, upper case.
pub fn normalize_address(address: &str) -> String {
    address.chars().filter(|c| !c.is_whitespace()).flat_map(char::to_uppercase).collect()
}

fn spell(inscription: &[char], word: &str) -> Result<Vec<Chevron>, char> {
    let mut used: BTreeMap<char, usize> = BTreeMap::new();
    normalize_address(word)
        .chars()
        .map(|letter| {
            let places: Vec<Chevron> =
                inscription.iter().enumerate().filter(|(_, on)| **on == letter).map(|(at, _)| at as Chevron).collect();
            if places.is_empty() {
                return Err(letter);
            }
            let k = used.entry(letter).or_default();
            let chevron = places[*k % places.len()];
            *k += 1;
            Ok(chevron)
        })
        .collect()
}

impl AuthoredGateNetwork {
    /// The engine's catalog and the game's words. A ring whose word does not
    /// spell lowers with no address (a problem the engine's check names).
    pub fn lower(&self) -> (GateNetworkCatalog, GateWords) {
        let inscription: Vec<char> = self.inscription.chars().collect();
        let mut words = BTreeMap::new();
        let rings = self
            .rings
            .iter()
            .map(|ring| {
                let address = match (&ring.address, &ring.glyphs) {
                    (Some(word), _) => {
                        words.insert(ring.zone.clone(), normalize_address(word));
                        spell(&inscription, word).unwrap_or_default()
                    }
                    (None, Some(glyphs)) => glyphs.clone(),
                    (None, None) => Vec::new(),
                };
                GateRing {
                    zone: ring.zone.clone(),
                    name: ring.name.clone(),
                    facility: ring.facility.clone(),
                    room: ring.room.clone(),
                    address,
                    hidden: ring.hidden,
                    terminal: ring.terminal.clone(),
                    status: ring.status.clone(),
                    note: ring.note.clone(),
                    kind: ring.kind,
                    reach: ring.reach,
                }
            })
            .collect();
        let catalog = GateNetworkCatalog {
            idle_timeout_secs: self.idle_timeout_secs,
            symbols: u16::try_from(inscription.len()).unwrap_or(u16::MAX),
            facilities: self
                .facilities
                .iter()
                .map(|facility| GateFacility { id: facility.id.clone(), name: facility.name.clone(), world: facility.dimension.clone() })
                .collect(),
            rings,
        };
        (catalog, GateWords { inscription, words })
    }
}

/// Each problem of the file: its spelling (the inscription and each word),
/// then the engine's checks of the lowered network.
fn problems(network: &AuthoredGateNetwork) -> Vec<String> {
    let mut problems = Vec::new();
    for letter in network.inscription.chars() {
        if !letter.is_ascii_uppercase() {
            problems.push(format!("the inscription letter '{letter}' is not an upper-case letter"));
        }
    }
    let mut words: BTreeMap<String, &str> = BTreeMap::new();
    let inscription: Vec<char> = network.inscription.chars().collect();
    for ring in &network.rings {
        let zone = ring.zone.as_str();
        if ring.address.is_some() && ring.glyphs.is_some() {
            problems.push(format!("`{zone}` has both an address word and glyphs"));
        }
        if let Some(address) = &ring.address {
            let normalized = normalize_address(address);
            if normalized != *address {
                problems.push(format!("`{zone}.address` `{address}` is not written as `{normalized}`"));
            }
            if let Err(letter) = spell(&inscription, address) {
                problems.push(format!("`{zone}.address` `{address}` has '{letter}', which is on no chevron"));
            }
            if let Some(other) = words.insert(normalized, zone) {
                problems.push(format!("the address `{address}` reaches both `{other}` and `{zone}`"));
            }
        }
    }
    problems.extend(gate_network_problems(&network.lower().0));
    problems
}

/// The schema, for Ambition's pack.
pub fn gate_network_schema() -> SchemaRegistration {
    rules_schema::<AuthoredGateNetwork>(
        GATE_NETWORK_SCHEMA,
        1,
        crate::AMBITION_CAPABILITY,
        "Ambition's gate network: the ring's inscription, the facilities, and \
         each ring with its room, address word or glyphs, terminal, reach and \
         status, lowered into the engine's numbered gate network.",
        problems,
    )
}

/// The network `pack` states, if it states one.
pub fn lowered_gate_network(pack: &PreparedContentPack) -> Option<&AuthoredGateNetwork> {
    lowered_rules::<AuthoredGateNetwork>(pack, GATE_NETWORK_SCHEMA)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn network(rings: &[(&str, Option<&str>, Option<Vec<Chevron>>)]) -> AuthoredGateNetwork {
        AuthoredGateNetwork {
            idle_timeout_secs: 30,
            inscription: "LEGALLYDISTINCT".into(),
            facilities: ["home", "math", "ninja"]
                .iter()
                .map(|id| AuthoredGateFacility { id: (*id).into(), name: (*id).into(), dimension: "home".into() })
                .collect(),
            rings: rings
                .iter()
                .zip(["home", "math", "ninja"])
                .map(|((zone, address, glyphs), facility)| AuthoredGateRing {
                    zone: (*zone).into(),
                    name: (*zone).into(),
                    facility: facility.into(),
                    room: format!("{zone}_room"),
                    address: address.map(Into::into),
                    glyphs: glyphs.clone(),
                    hidden: false,
                    terminal: Some(format!("{zone}_terminal")),
                    status: GateStatusRule::Online,
                    note: String::new(),
                    kind: RingKind::Gate,
                    reach: None,
                })
                .collect(),
        }
    }

    /// A word spells by the inscription's rule: the k-th use of a letter
    /// takes the k-th chevron of it, round again past the last. LEGALLY
    /// DISTINCT spells every chevron in order.
    #[test]
    fn a_word_spells_its_chevrons_by_the_inscriptions_rule() {
        let (_, words) = network(&[]).lower();
        assert_eq!(words.spell("LEGALLY DISTINCT"), Ok((0..15).collect::<Vec<Chevron>>()));
        // L E G A L: the second L is the second Λ (chevron 4).
        assert_eq!(words.spell("legal"), Ok(vec![0, 1, 2, 3, 4]));
        // T A N G E N T: one N, used twice; the second T is chevron 14.
        assert_eq!(words.spell("TANGENT"), Ok(vec![10, 3, 12, 2, 1, 12, 14]));
        // Four L's: the fourth goes round to the first Λ.
        assert_eq!(words.spell("LLLL"), Ok(vec![0, 4, 5, 0]));
        assert_eq!(words.spell("TANGENTO"), Err('O'));
    }

    /// The engine receives chevrons only; the words stay the game's, and a
    /// ring whose address is glyphs is keyed by its zone.
    #[test]
    fn the_lowered_network_dials_chevrons_and_keeps_its_words_apart() {
        let (catalog, words) =
            network(&[("hub", Some("LEGAL"), None), ("math", Some("TANGENT"), None), ("ring7", None, Some(vec![9, 13, 2]))])
                .lower();
        assert_eq!(catalog.symbols(), 15);
        assert_eq!(catalog.ring("math").map(|ring| ring.address.clone()), Some(vec![10, 3, 12, 2, 1, 12, 14]));
        assert_eq!(words.word_of(&catalog, &[0, 1, 2, 3, 4]), Some("LEGAL"));
        assert_eq!(words.word_of(&catalog, &[0, 1, 2, 3, 0]), None);
        assert_eq!(words.word_of(&catalog, &[9, 13, 2]), None);
        assert_eq!(words.zone_named("tangent"), Some("math"));
        assert_eq!(catalog.ring("ring7").map(|ring| words.address_key(ring)), Some("ring7"));
        assert_eq!(catalog.ring("hub").map(|ring| words.address_key(ring)), Some("LEGAL"));
    }

    #[test]
    fn a_spelling_problem_is_named_by_its_ring_and_field() {
        let problems = problems(&network(&[
            ("hub", Some("LEGAL"), None),
            ("math", Some("LEGAL"), None),
            ("ninja", Some("SILENTO"), Some(vec![1])),
        ]));
        for expected in [
            "the address `LEGAL` reaches both `hub` and `math`",
            "`ninja.address` `SILENTO` has 'O', which is on no chevron",
            "`ninja` has both an address word and glyphs",
            "`hub` and `math` answer the same symbols",
        ] {
            assert!(problems.iter().any(|p| p.contains(expected)), "missing {expected:?} in {problems:?}");
        }
    }
}

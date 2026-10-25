//! The gate network: rings that a terminal dials by address.
//!
//! A game states its network in one authored catalog ([`GATE_NETWORK_SCHEMA`]):
//! the ring's inscription, the facilities that hold rings, and each ring (its
//! loading zone, room, address, terminal and status). This module owns the
//! rules that read the catalog: addresses, the dial decision
//! ([`resolve_dial`]) and the live network state ([`GateNetworkState`]),
//! which is rollback state keyed by ring zone ids.
//!
//! An address is a sequence of CHEVRONS ([`Chevron`]: a place in the
//! inscription). The catalog names each address by a word, and a word spells
//! into chevrons by the inscription's own rule ([`GateNetworkCatalog::spell`]):
//! when a letter is on several chevrons, its k-th use in the word takes the
//! k-th of them. A dial must enter exactly those chevrons.
//!
//! No rule of a faction refuses a dial: a dial fails when no ring answers,
//! when a ring is down or busy, or when the far ring is in the same facility.
//!
//! A dial never moves a body. A dial that succeeds opens a LINK: the source
//! ring boots (its portal phase follows the link) and the far ring shows the
//! incoming wormhole. The traveller then goes through the source ring, and the
//! ordinary room-transition road takes it to the far ring's zone.
//!
//! Knowledge of an address is convenience only. [`resolve_dial`] never reads
//! what the player knows.

use std::collections::{BTreeMap, BTreeSet};

use ambition_platformer2d_core::snapshot::RollbackRegistrar;
use bevy_ecs::prelude::{Message, Resource};

/// The authored FILE kind of a game's gate network. The engine's schema
/// registry registers it (the world crate does not link the content pack).
pub const GATE_NETWORK_SCHEMA: &str = "gate_network";

/// How long a link stays open after a dial, in simulation seconds, when no
/// traveller goes through. A link closes by itself so that a gate is never
/// left open.
pub const GATE_LINK_OPEN_SECS: f32 = 30.0;

/// How long a link stays open after a traveller goes in: enough for the
/// passage into the ring to finish while the wormhole is still visible.
pub const GATE_LINK_LINGER_SECS: f32 = 1.5;

/// How many recent addresses the network remembers for a terminal's Recent
/// view.
pub const GATE_RECENT_LEN: usize = 6;

/// One chevron of a ring: its place in the inscription, from the first.
pub type Chevron = u8;

/// The save flag that records that the player knows `address` (its word). A dial that
/// succeeds sets it; a sign, a person or a document can set it too (Yarn
/// `world.set_flag`). Knowledge changes what a terminal lists, never what a
/// dial does.
pub fn gate_address_known_flag(address: &str) -> String {
    format!("gate.known.{address}")
}

/// A game's gate network, as authored.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GateNetworkCatalog {
    /// The letters on a ring's chevrons, one per chevron, in order: the
    /// chevrons a terminal shows. A letter may stand on several chevrons;
    /// each chevron is its own symbol.
    pub inscription: String,
    /// Each logistical facility that holds rings. Two rings in one facility
    /// never dial each other: that is a walk, not a gate trip.
    pub facilities: Vec<GateFacility>,
    /// Each ring of the network.
    pub rings: Vec<GateRing>,
    /// Which addresses a traveller knows before learning any. Knowledge only
    /// decides what a terminal lists; a dial never reads it.
    #[serde(default)]
    pub start_known: GateStartKnowledge,
}

/// Which addresses a traveller knows before learning any.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Deserialize)]
pub enum GateStartKnowledge {
    /// Only what the traveller learned: a dial that answered, a sign, a
    /// person, a document (each sets [`gate_address_known_flag`]).
    #[default]
    Discovered,
    /// Every address of the network, hidden ones too: a development game that
    /// starts with everything unlocked.
    All,
}

/// One logistical facility: a gate station and the rings it holds.
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GateFacility {
    pub id: String,
    /// What a terminal calls it.
    pub name: String,
    /// The dimension it is in. Two facilities in one dimension can dial each
    /// other: only one facility is too near.
    pub dimension: String,
}

/// One ring: a loading zone that a link can open, and the terminal that
/// dials from it.
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GateRing {
    /// The loading zone of the ring. A traveller goes in through it, and
    /// arrives in it from another ring. Its id is the ring's identity.
    pub zone: String,
    /// What a terminal calls the ring.
    pub name: String,
    /// The [`GateFacility::id`] that holds the ring.
    pub facility: String,
    /// The room that holds the zone.
    pub room: String,
    /// The word whose chevrons reach this ring. `None`: the ring sends but no
    /// address reaches it.
    #[serde(default)]
    pub address: Option<String>,
    /// A hidden address is not in any directory. A dial reaches it the same
    /// way it reaches any other.
    #[serde(default)]
    pub hidden: bool,
    /// The id of the terminal (`Switch` with action `GateTerminal`) that
    /// dials from this ring. `None`: the ring only receives.
    #[serde(default)]
    pub terminal: Option<String>,
    /// The ring's infrastructure state, from authored facts only.
    #[serde(default)]
    pub status: GateStatusRule,
    /// Addresses written on the ring's terminal. A terminal always lists them,
    /// so a traveller can go back without knowing an address.
    #[serde(default)]
    pub posted: Vec<String>,
    /// A line the terminal shows about the ring (its upkeep, its traffic).
    #[serde(default)]
    pub note: String,
}

/// A ring's infrastructure state. Each state reads authored facts (the save's
/// switches and flags), so two runs of one save give one answer.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Deserialize)]
pub enum GateStatusRule {
    /// The ring works.
    #[default]
    Online,
    /// The ring does not work, for the authored reason.
    Offline(String),
    /// The ring works only while the switch is on.
    PoweredBy { switch: String, reason: String },
    /// The ring is busy while the flag is set.
    BusyWhile { flag: String, reason: String },
}

/// Why a dial did not open a link. Each kind has its own message, so a
/// player can tell a wrong address from a ring that is down.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DialRefusal {
    /// No chevrons were entered.
    Empty,
    /// A chevron the ring does not have.
    NoSuchChevron(Chevron),
    /// No ring answers the address.
    Unknown,
    /// The address is a ring in the dialing ring's own facility.
    SameFacility,
    /// A ring is down (the dialing ring or the far one), for the reason.
    Offline(String),
    /// A ring is in use, for the reason.
    Busy(String),
}

impl DialRefusal {
    /// The kind, as one word, for a status line and for a test.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::NoSuchChevron(_) => "illegal",
            Self::Unknown => "unknown",
            Self::SameFacility => "same_facility",
            Self::Offline(_) => "offline",
            Self::Busy(_) => "busy",
        }
    }

    /// What a terminal shows.
    pub fn message(&self) -> String {
        match self {
            Self::Empty => "Enter an address first.".to_string(),
            Self::NoSuchChevron(chevron) => format!("The ring has no chevron {chevron}."),
            Self::Unknown => "No ring answers that address.".to_string(),
            Self::SameFacility => "That ring is in this facility. Walk there.".to_string(),
            Self::Offline(reason) => format!("OFFLINE: {reason}"),
            Self::Busy(reason) => format!("BUSY: {reason}"),
        }
    }
}

/// What the dial decision reads of the world, beyond the catalog.
pub struct GateFacts<'a> {
    /// Whether a save switch is on.
    pub switch_on: &'a dyn Fn(&str) -> bool,
    /// Whether a save flag is set.
    pub flag: &'a dyn Fn(&str) -> bool,
    /// Whether this session's world holds the zone in the room.
    pub zone_exists: &'a dyn Fn(&str, &str) -> bool,
}

impl GateNetworkCatalog {
    /// The ring whose loading zone is `zone`.
    pub fn ring(&self, zone: &str) -> Option<&GateRing> {
        self.rings.iter().find(|ring| ring.zone == zone)
    }

    /// The ring the terminal `terminal` dials from.
    pub fn ring_of_terminal(&self, terminal: &str) -> Option<&GateRing> {
        self.rings.iter().find(|ring| ring.terminal.as_deref() == Some(terminal))
    }

    /// The ring the chevrons `address` reach.
    pub fn ring_at(&self, address: &[Chevron]) -> Option<&GateRing> {
        self.rings
            .iter()
            .find(|ring| ring.address.as_deref().is_some_and(|word| self.spell(word).as_deref() == Ok(address)))
    }

    /// The ring whose address is the word `word` (normalized first).
    pub fn ring_named(&self, word: &str) -> Option<&GateRing> {
        let word = normalize_address(word);
        self.rings.iter().find(|ring| ring.address.as_deref() == Some(word.as_str()))
    }

    /// The word of the address the chevrons spell, if a ring has it.
    pub fn word_of(&self, address: &[Chevron]) -> Option<&str> {
        self.ring_at(address).and_then(|ring| ring.address.as_deref())
    }

    /// The chevrons a word spells: each letter in turn takes the next chevron
    /// that carries it, so the k-th use of a letter takes the k-th chevron of
    /// it, round again when the word uses it more often than the ring has it
    /// ("LEGALLY DISTINCT" spells every chevron in order). The word is
    /// normalized first. `Err`: a letter on no chevron.
    pub fn spell(&self, word: &str) -> Result<Vec<Chevron>, char> {
        let mut used: BTreeMap<char, usize> = BTreeMap::new();
        normalize_address(word)
            .chars()
            .map(|letter| {
                let places: Vec<Chevron> = self
                    .inscription
                    .chars()
                    .enumerate()
                    .filter(|(_, on)| *on == letter)
                    .map(|(at, _)| at as Chevron)
                    .collect();
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

    /// How many chevrons a ring has.
    pub fn chevrons(&self) -> usize {
        self.inscription.chars().count()
    }

    /// The facility `id`.
    pub fn facility(&self, id: &str) -> Option<&GateFacility> {
        self.facilities.iter().find(|facility| facility.id == id)
    }

    /// Whether a traveller knows `address`: under [`GateStartKnowledge::All`]
    /// every address, else the ones whose known flag `flag` reads as set.
    pub fn knows(&self, address: &str, flag: impl Fn(&str) -> bool) -> bool {
        matches!(self.start_known, GateStartKnowledge::All) || flag(&gate_address_known_flag(address))
    }
}

/// An address word as the catalog compares it: no spaces, upper case.
pub fn normalize_address(address: &str) -> String {
    address.chars().filter(|c| !c.is_whitespace()).flat_map(char::to_uppercase).collect()
}

/// The ring's own state, read from authored facts.
fn ring_status(ring: &GateRing, facts: &GateFacts<'_>) -> Result<(), DialRefusal> {
    match &ring.status {
        GateStatusRule::Online => Ok(()),
        GateStatusRule::Offline(reason) => Err(DialRefusal::Offline(reason.clone())),
        GateStatusRule::PoweredBy { switch, reason } => {
            if (facts.switch_on)(switch) {
                Ok(())
            } else {
                Err(DialRefusal::Offline(reason.clone()))
            }
        }
        GateStatusRule::BusyWhile { flag, reason } => {
            if (facts.flag)(flag) {
                Err(DialRefusal::Busy(reason.clone()))
            } else {
                Ok(())
            }
        }
    }
}

/// Decide a dial of the chevrons `address` from the ring `source`: the far
/// ring, or why not. The checks go in a fixed order, so one dial has one
/// answer:
///
/// 1. the address is not empty and each chevron is on the ring;
/// 2. the dialing ring works;
/// 3. a ring answers the address;
/// 4. the far ring is not in the dialing ring's facility;
/// 5. the far ring works and its room is in this world;
/// 6. the far ring is not in another link.
///
/// What the player knows is not read, and no faction refuses a dial.
pub fn resolve_dial<'a>(
    catalog: &'a GateNetworkCatalog,
    state: &GateNetworkState,
    source: &str,
    address: &[Chevron],
    facts: &GateFacts<'_>,
) -> Result<&'a GateRing, DialRefusal> {
    if address.is_empty() {
        return Err(DialRefusal::Empty);
    }
    if let Some(chevron) = address.iter().find(|chevron| usize::from(**chevron) >= catalog.chevrons()) {
        return Err(DialRefusal::NoSuchChevron(*chevron));
    }
    let Some(from) = catalog.ring(source) else {
        return Err(DialRefusal::Offline("this terminal has no ring".to_string()));
    };
    ring_status(from, facts)?;
    let Some(far) = catalog.ring_at(address) else {
        return Err(DialRefusal::Unknown);
    };
    if far.facility == from.facility {
        return Err(DialRefusal::SameFacility);
    }
    ring_status(far, facts)?;
    if !(facts.zone_exists)(&far.room, &far.zone) {
        return Err(DialRefusal::Offline("the far ring does not answer".to_string()));
    }
    if state.engaged_elsewhere(&far.zone, source) {
        return Err(DialRefusal::Busy("the far ring is in another link".to_string()));
    }
    Ok(far)
}

/// Each problem of a catalog, named by its ring and field. The pack refuses
/// a catalog with a problem. That the rooms and zones exist in the world
/// files is checked against the world, not here.
pub fn gate_network_problems(catalog: &GateNetworkCatalog) -> Vec<String> {
    let mut problems = Vec::new();
    for letter in catalog.inscription.chars() {
        if !letter.is_ascii_uppercase() {
            problems.push(format!("the inscription letter '{letter}' is not an upper-case letter"));
        }
    }
    if catalog.inscription.is_empty() {
        problems.push("the inscription is empty".to_string());
    }
    if catalog.chevrons() > usize::from(Chevron::MAX) + 1 {
        problems.push("the inscription has more chevrons than a dial can name".to_string());
    }
    let mut facilities = BTreeSet::new();
    for facility in &catalog.facilities {
        if facility.id.trim().is_empty() || facility.dimension.trim().is_empty() {
            problems.push(format!("the facility `{}` has an empty id or dimension", facility.id));
        }
        if !facilities.insert(facility.id.as_str()) {
            problems.push(format!("the facility `{}` is stated twice", facility.id));
        }
    }
    let mut zones = BTreeSet::new();
    let mut terminals = BTreeMap::new();
    let mut addresses: BTreeMap<String, &str> = BTreeMap::new();
    let address_ok = |address: &str, problems: &mut Vec<String>, at: String| {
        let normalized = normalize_address(address);
        if normalized.is_empty() {
            problems.push(format!("{at} is empty"));
        }
        if normalized != address {
            problems.push(format!("{at} `{address}` is not written as `{normalized}`"));
        }
        if let Err(letter) = catalog.spell(&normalized) {
            problems.push(format!("{at} `{address}` has '{letter}', which is on no chevron"));
        }
    };
    for ring in &catalog.rings {
        let zone = ring.zone.as_str();
        for (field, value) in [("zone", &ring.zone), ("name", &ring.name), ("room", &ring.room)] {
            if value.trim().is_empty() {
                problems.push(format!("a ring's `{field}` is empty (ring `{zone}`)"));
            }
        }
        if !zones.insert(zone) {
            problems.push(format!("the ring `{zone}` is stated twice"));
        }
        if !facilities.contains(ring.facility.as_str()) {
            problems.push(format!("`{zone}.facility` `{}` is not a facility", ring.facility));
        }
        if let Some(address) = &ring.address {
            address_ok(address, &mut problems, format!("`{zone}.address`"));
            if let Some(other) = addresses.insert(normalize_address(address), zone) {
                problems.push(format!("the address `{address}` reaches both `{other}` and `{zone}`"));
            }
        } else if ring.hidden {
            problems.push(format!("`{zone}` is hidden but has no address"));
        }
        if let Some(terminal) = &ring.terminal {
            if terminal.trim().is_empty() {
                problems.push(format!("`{zone}.terminal` is empty"));
            }
            if let Some(other) = terminals.insert(terminal.as_str(), zone) {
                problems.push(format!("the terminal `{terminal}` dials from both `{other}` and `{zone}`"));
            }
        }
        let status_text = match &ring.status {
            GateStatusRule::Online => None,
            GateStatusRule::Offline(reason) => Some(("", reason)),
            GateStatusRule::PoweredBy { switch, reason } => Some((switch.as_str(), reason)),
            GateStatusRule::BusyWhile { flag, reason } => Some((flag.as_str(), reason)),
        };
        if let Some((fact, reason)) = status_text {
            let names_a_fact = !matches!(ring.status, GateStatusRule::Offline(_));
            if reason.trim().is_empty() || (names_a_fact && fact.trim().is_empty()) {
                problems.push(format!("`{zone}.status` has an empty fact or reason"));
            }
        }
        for address in &ring.posted {
            address_ok(address, &mut problems, format!("`{zone}.posted` address"));
        }
    }
    // A posting may only name an address the network has.
    for ring in &catalog.rings {
        for address in &ring.posted {
            if !addresses.contains_key(&normalize_address(address)) {
                problems.push(format!("`{}` names the address `{address}`, which no ring has", ring.zone));
            }
        }
    }
    problems
}

/// One open link: the ring that dialed it sends, and the far ring receives.
#[derive(Clone, Debug, PartialEq)]
pub struct GateLink {
    /// The far ring's zone.
    pub far: String,
    /// The room that holds the far ring's zone.
    pub far_room: String,
    /// The chevrons that were dialed.
    pub address: Vec<Chevron>,
    /// Simulation seconds before the link closes.
    pub remaining: f32,
    /// A traveller has gone in.
    pub crossed: bool,
}

/// The answer to the latest dial of one terminal.
#[derive(Clone, Debug, PartialEq)]
pub struct DialReport {
    /// Counts each dial of the network, so a reader can tell a new report
    /// from the one it already showed.
    pub serial: u64,
    /// The chevrons as dialed.
    pub address: Vec<Chevron>,
    /// The far ring's zone, or why there is no link.
    pub outcome: Result<String, DialRefusal>,
}

/// A request from a terminal's user (a host intent: it enters the
/// simulation on a stamped tick).
#[derive(Message, Clone, Debug, PartialEq, Eq)]
pub struct GateTerminalIntent {
    /// The ring whose terminal is used.
    pub ring: String,
    pub action: GateTerminalAction,
}

/// What a terminal's user asks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GateTerminalAction {
    /// Dial the chevrons. A link this ring already has closes first.
    Dial(Vec<Chevron>),
    /// Shut this ring's link, if it has one. The terminal stays open.
    Hangup,
    /// Leave the terminal. An open link stays open, so the user can walk in.
    Close,
}

/// The live state of the network: which terminal is in use, each open link,
/// and the latest answers. Rollback state: the links decide when a ring is
/// open, and a ring that is open decides a room transition.
///
/// Keyed by ring zone ids in `BTreeMap`s, so each peer iterates in one order.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct GateNetworkState {
    /// The ring whose terminal is open, if any.
    pub terminal: Option<String>,
    /// Each open link, by the ring that dialed it.
    pub links: BTreeMap<String, GateLink>,
    /// The latest dial answer of each terminal, by its ring.
    pub reports: BTreeMap<String, DialReport>,
    /// The chevrons of recent links, newest first.
    pub recent: Vec<Vec<Chevron>>,
    /// The serial the next report takes.
    pub next_serial: u64,
}

impl GateNetworkState {
    /// The far ring's zone of the link `ring` sends, if any.
    pub fn route_from(&self, ring: &str) -> Option<&GateLink> {
        self.links.get(ring)
    }

    /// Whether a link holds `ring` open, as its sender or its receiver.
    pub fn holds_open(&self, ring: &str) -> bool {
        self.links.contains_key(ring) || self.links.values().any(|link| link.far == ring)
    }

    /// Whether `ring` is in a link that `source` did not open.
    fn engaged_elsewhere(&self, ring: &str, source: &str) -> bool {
        self.links.iter().any(|(from, link)| from != source && (from == ring || link.far == ring))
    }

    /// Record the answer of a dial from `ring`.
    pub fn report(&mut self, ring: &str, address: Vec<Chevron>, outcome: Result<String, DialRefusal>) {
        let serial = self.next_serial;
        self.next_serial += 1;
        self.reports.insert(ring.to_string(), DialReport { serial, address, outcome });
    }

    /// Open a link from `ring` to the zone `far` of the room `far_room`.
    pub fn open_link(&mut self, ring: &str, far_room: &str, far: &str, address: &[Chevron]) {
        self.links.insert(
            ring.to_string(),
            GateLink {
                far: far.to_string(),
                far_room: far_room.to_string(),
                address: address.to_vec(),
                remaining: GATE_LINK_OPEN_SECS,
                crossed: false,
            },
        );
        self.recent.retain(|recent| recent != address);
        self.recent.insert(0, address.to_vec());
        self.recent.truncate(GATE_RECENT_LEN);
    }

    /// A traveller went in through `ring`: its link closes soon.
    pub fn crossed(&mut self, ring: &str) {
        if let Some(link) = self.links.get_mut(ring) {
            link.crossed = true;
            link.remaining = link.remaining.min(GATE_LINK_LINGER_SECS);
        }
    }

    /// Advance each link's timer by `dt`, and close the links that ran out.
    pub fn tick(&mut self, dt: f32) {
        for link in self.links.values_mut() {
            link.remaining -= dt;
        }
        self.links.retain(|_, link| link.remaining > 0.0);
    }
}

/// The value projection of [`GateNetworkState`] for the rollback checksum:
/// every field that decides a ring's phase or a transition.
pub fn gate_network_state_checksum(state: &GateNetworkState) -> u64 {
    use ambition_platformer2d_core::snapshot::{checksum_bytes, put_bool, put_f32, put_opt_str, put_str, put_u64};
    let put_chevrons = |bytes: &mut Vec<u8>, address: &[Chevron]| {
        put_u64(bytes, address.len() as u64);
        bytes.extend_from_slice(address);
    };
    let mut bytes = Vec::new();
    put_opt_str(&mut bytes, state.terminal.as_deref());
    put_u64(&mut bytes, state.links.len() as u64);
    for (ring, link) in &state.links {
        put_str(&mut bytes, ring);
        put_str(&mut bytes, &link.far);
        put_str(&mut bytes, &link.far_room);
        put_chevrons(&mut bytes, &link.address);
        put_f32(&mut bytes, link.remaining);
        put_bool(&mut bytes, link.crossed);
    }
    put_u64(&mut bytes, state.reports.len() as u64);
    for (ring, report) in &state.reports {
        put_str(&mut bytes, ring);
        put_u64(&mut bytes, report.serial);
        put_chevrons(&mut bytes, &report.address);
        match &report.outcome {
            Ok(far) => put_str(&mut bytes, far),
            Err(refusal) => {
                put_str(&mut bytes, refusal.kind());
                put_str(&mut bytes, &refusal.message());
            }
        }
    }
    put_u64(&mut bytes, state.recent.len() as u64);
    for address in &state.recent {
        put_chevrons(&mut bytes, address);
    }
    put_u64(&mut bytes, state.next_serial);
    checksum_bytes(&bytes)
}

/// Register the gate network's rewound state and its terminal intent.
pub fn register_gate_network_rollback_state<R>(registrar: &mut R)
where
    R: RollbackRegistrar,
{
    registrar.rollback_resource_clone_checksum::<GateNetworkState>(
        super::GATE_PORTAL_ROLLBACK_OWNER,
        "resource.gate_network_state",
        "open terminal, links, reports and recent addresses",
        gate_network_state_checksum,
    );
    registrar.clear_message_on_rollback::<GateTerminalIntent>(
        super::GATE_PORTAL_ROLLBACK_OWNER,
        "message.gate_terminal_intent",
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ring(zone: &str, facility: &str, address: Option<&str>) -> GateRing {
        GateRing {
            zone: zone.into(),
            name: zone.into(),
            facility: facility.into(),
            room: format!("{zone}_room"),
            address: address.map(Into::into),
            hidden: false,
            terminal: Some(format!("{zone}_terminal")),
            status: GateStatusRule::Online,
            posted: Vec::new(),
            note: String::new(),
        }
    }

    fn catalog() -> GateNetworkCatalog {
        let facility = |id: &str, dimension: &str| GateFacility {
            id: id.into(),
            name: id.into(),
            dimension: dimension.into(),
        };
        let mut ninja = ring("ninja", "ninja_shrine", Some("SILENT"));
        ninja.status = GateStatusRule::Offline("the far ring is not yet built".into());
        let mut ring7 = ring("ring7", "home", None);
        ring7.status = GateStatusRule::PoweredBy { switch: "breaker".into(), reason: "no power".into() };
        let mut station = ring("station", "station", Some("GALACTIC"));
        station.status = GateStatusRule::BusyWhile { flag: "convoy".into(), reason: "a freight convoy".into() };
        GateNetworkCatalog {
            inscription: "LEGALLYDISTINCT".into(),
            facilities: vec![
                facility("home", "home"),
                facility("math", "math"),
                facility("ninja_shrine", "ninja"),
                facility("fort", "home"),
                facility("station", "science"),
                facility("trail", "science"),
            ],
            rings: vec![
                ring("hub", "home", Some("LEGAL")),
                ring7,
                ring("math", "math", Some("TANGENT")),
                ninja,
                ring("fort", "fort", Some("SENTINEL")),
                station,
                ring("trail", "trail", Some("ASCENT")),
            ],
            start_known: GateStartKnowledge::Discovered,
        }
    }

    fn spelled(catalog: &GateNetworkCatalog, word: &str) -> Vec<Chevron> {
        catalog.spell(word).expect("every letter is on a chevron")
    }

    fn dial<'a>(
        catalog: &'a GateNetworkCatalog,
        state: &GateNetworkState,
        from: &str,
        address: &[Chevron],
        on: &[&str],
    ) -> Result<&'a str, DialRefusal> {
        let switch_on = |id: &str| on.contains(&id);
        let flag = |id: &str| on.contains(&id);
        let zone_exists = |_: &str, _: &str| true;
        let facts = GateFacts { switch_on: &switch_on, flag: &flag, zone_exists: &zone_exists };
        resolve_dial(catalog, state, from, address, &facts).map(|ring| ring.zone.as_str())
    }

    /// A word spells by the inscription's rule: the k-th use of a letter
    /// takes the k-th chevron of it, round again past the last. LEGALLY
    /// DISTINCT spells every chevron in order.
    #[test]
    fn a_word_spells_its_chevrons_by_the_inscriptions_rule() {
        let catalog = catalog();
        assert_eq!(spelled(&catalog, "LEGALLY DISTINCT"), (0..15).collect::<Vec<Chevron>>());
        // L E G A L: the second L is the second Λ (chevron 4).
        assert_eq!(spelled(&catalog, "legal"), [0, 1, 2, 3, 4]);
        // T A N G E N T: one N, used twice; the second T is chevron 14.
        assert_eq!(spelled(&catalog, "TANGENT"), [10, 3, 12, 2, 1, 12, 14]);
        // Four L's: the fourth goes round to the first Λ.
        assert_eq!(spelled(&catalog, "LLLL"), [0, 4, 5, 0]);
        assert_eq!(catalog.spell("TANGENTO"), Err('O'));
    }

    /// A dial must enter the address's exact chevrons: LEGAL with the plain Λ
    /// twice is another address, which no ring answers.
    #[test]
    fn a_dial_answers_the_exact_chevrons_only() {
        let catalog = catalog();
        let state = GateNetworkState::default();
        assert_eq!(dial(&catalog, &state, "hub", &spelled(&catalog, "TANGENT"), &[]), Ok("math"));
        assert_eq!(dial(&catalog, &state, "math", &[0, 1, 2, 3, 4], &[]), Ok("hub"));
        assert_eq!(dial(&catalog, &state, "math", &[0, 1, 2, 3, 0], &[]), Err(DialRefusal::Unknown));
        assert_eq!(catalog.word_of(&[0, 1, 2, 3, 4]), Some("LEGAL"));
        assert_eq!(catalog.word_of(&[0, 1, 2, 3, 0]), None);
    }

    #[test]
    fn each_refusal_has_its_own_kind() {
        let catalog = catalog();
        let mut state = GateNetworkState::default();
        let kind = |result: Result<&str, DialRefusal>| result.unwrap_err().kind();
        assert_eq!(kind(dial(&catalog, &state, "hub", &[], &[])), "empty");
        assert_eq!(dial(&catalog, &state, "hub", &[10, 15], &[]), Err(DialRefusal::NoSuchChevron(15)));
        assert_eq!(kind(dial(&catalog, &state, "hub", &spelled(&catalog, "SAND"), &[])), "unknown");
        assert_eq!(kind(dial(&catalog, &state, "hub", &spelled(&catalog, "SILENT"), &[])), "offline");
        assert_eq!(kind(dial(&catalog, &state, "hub", &spelled(&catalog, "GALACTIC"), &["convoy"])), "busy");
        let tangent = spelled(&catalog, "TANGENT");
        state.open_link("trail", "math_room", "math", &tangent);
        assert_eq!(kind(dial(&catalog, &state, "hub", &tangent, &[])), "busy");
        assert_eq!(kind(dial(&catalog, &state, "hub", &spelled(&catalog, "ASCENT"), &[])), "busy", "a sending ring is engaged too");
    }

    /// No faction refuses a dial: every working ring answers every ring, the
    /// military one too.
    #[test]
    fn no_ring_refuses_a_dial_by_who_dials() {
        let catalog = catalog();
        let state = GateNetworkState::default();
        assert_eq!(dial(&catalog, &state, "hub", &spelled(&catalog, "SENTINEL"), &[]), Ok("fort"));
        assert_eq!(dial(&catalog, &state, "ring7", &spelled(&catalog, "ASCENT"), &["breaker"]), Ok("trail"));
    }

    #[test]
    fn a_ring_never_dials_its_own_facility_but_may_dial_its_own_dimension() {
        let catalog = catalog();
        let state = GateNetworkState::default();
        let legal = spelled(&catalog, "LEGAL");
        assert_eq!(dial(&catalog, &state, "hub", &legal, &[]), Err(DialRefusal::SameFacility));
        assert_eq!(dial(&catalog, &state, "ring7", &legal, &["breaker"]), Err(DialRefusal::SameFacility));
        assert_eq!(dial(&catalog, &state, "station", &spelled(&catalog, "ASCENT"), &[]), Ok("trail"));
        assert_eq!(dial(&catalog, &state, "trail", &spelled(&catalog, "GALACTIC"), &[]), Ok("station"));
    }

    #[test]
    fn a_powered_source_answers_by_its_breaker() {
        let catalog = catalog();
        let state = GateNetworkState::default();
        let tangent = spelled(&catalog, "TANGENT");
        assert_eq!(dial(&catalog, &state, "ring7", &tangent, &[]).unwrap_err().kind(), "offline");
        assert_eq!(dial(&catalog, &state, "ring7", &tangent, &["breaker"]), Ok("math"));
    }

    #[test]
    fn a_link_opens_holds_both_rings_and_closes_after_its_traveller() {
        let mut state = GateNetworkState::default();
        let tangent = spelled(&catalog(), "TANGENT");
        state.open_link("hub", "math_room", "math", &tangent);
        assert!(state.holds_open("hub") && state.holds_open("math") && !state.holds_open("trail"));
        assert_eq!(state.route_from("hub").map(|link| link.far.as_str()), Some("math"));
        assert!(state.route_from("math").is_none(), "a receiving ring sends nobody back");
        state.tick(1.0);
        assert!(state.holds_open("hub"));
        state.crossed("hub");
        state.tick(GATE_LINK_LINGER_SECS + 0.01);
        assert!(!state.holds_open("hub") && !state.holds_open("math"));
        state.open_link("hub", "math_room", "math", &tangent);
        state.tick(GATE_LINK_OPEN_SECS + 0.01);
        assert!(state.links.is_empty(), "an unused link closes by itself");
        assert_eq!(state.recent, [tangent]);
    }

    #[test]
    fn the_checksum_sees_a_timer_and_a_report() {
        let mut a = GateNetworkState::default();
        a.open_link("hub", "math_room", "math", &[10, 3, 12]);
        let mut b = a.clone();
        assert_eq!(gate_network_state_checksum(&a), gate_network_state_checksum(&b));
        b.tick(1.0 / 60.0);
        assert_ne!(gate_network_state_checksum(&a), gate_network_state_checksum(&b));
        let mut c = a.clone();
        c.report("hub", vec![9, 3], Err(DialRefusal::Unknown));
        assert_ne!(gate_network_state_checksum(&a), gate_network_state_checksum(&c));
        let mut d = a.clone();
        d.report("hub", vec![9, 4], Err(DialRefusal::Unknown));
        assert_ne!(gate_network_state_checksum(&c), gate_network_state_checksum(&d), "the chevrons themselves count");
    }

    #[test]
    fn a_catalog_problem_is_named_by_its_ring_and_field() {
        assert!(gate_network_problems(&catalog()).is_empty(), "{:?}", gate_network_problems(&catalog()));
        let mut bad = catalog();
        bad.rings[2].address = Some("LEGAL".into());
        bad.rings[3].address = Some("SILENTO".into());
        bad.rings[4].facility = "nowhere".into();
        bad.rings[1].posted = vec!["SAND".into()];
        bad.rings.push(ring("hub", "home", None));
        let problems = gate_network_problems(&bad);
        for expected in [
            "the address `LEGAL` reaches both `hub` and `math`",
            "`ninja.address` `SILENTO` has 'O', which is on no chevron",
            "`fort.facility` `nowhere` is not a facility",
            "`ring7` names the address `SAND`, which no ring has",
            "the ring `hub` is stated twice",
        ] {
            assert!(problems.iter().any(|p| p.contains(expected)), "missing {expected:?} in {problems:?}");
        }
    }
}

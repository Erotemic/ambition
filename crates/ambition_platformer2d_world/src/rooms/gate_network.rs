//! The gate network: rings that a terminal dials by address.
//!
//! A game gives its network as one catalog resource ([`GateNetworkCatalog`]):
//! how many chevrons a ring has, the facilities that hold rings, and each ring
//! (its loading zone, room, address, terminal and status). This module owns
//! the rules that read the catalog: addresses, the dial decision
//! ([`resolve_dial`]) and the live network state ([`GateNetworkState`]),
//! which is rollback state keyed by ring zone ids.
//!
//! An address is a sequence of CHEVRONS ([`Chevron`]: a numbered place on the
//! ring, from 0). A dial must enter exactly a ring's chevrons. What a chevron
//! shows, and how a game writes an address (Ambition spells words into
//! chevrons), is the game's: it lowers its own file into this catalog.
//!
//! Every ring has an address and a terminal (Jon, 2026-10-24): any ring can
//! always be dialed, and dialed from.
//!
//! No rule of a faction refuses a dial: a dial fails when no ring answers,
//! when a ring is down or busy, or when the far ring is in the same facility.
//!
//! A successful dial opens one connection between two rings. Either ring
//! reaches the other through the shared room transition. Use restarts its
//! idle timer. An active passage or pending crossing keeps both ends open.
//! Either terminal can close the connection. A dial never moves a body.
//!
//! Knowledge of an address is convenience only. [`resolve_dial`] never reads
//! what the player knows.
//!
//! **Two kinds of ring** (Jon, 2026-10-31: "portals should generally be within
//! dimension, whereas dimension gates are a different thing. Portals maybe
//! even have some lore-like range."). A GATE ([`RingKind::Gate`]) is a door
//! between worlds and has no reach. A PORTAL ([`RingKind::Portal`], drawn as
//! an arch) folds one world onto itself: it reaches only portals of its own
//! dimension, and only as far as its reach on the flat map
//! ([`GateRing::reach`], [`DEFAULT_PORTAL_REACH`]). A gate dials gates and a
//! portal reaches portals. Where each ring stands on the flat map is the
//! world files' ([`GateRingPositions`], derived host-side when they load).

use std::collections::{BTreeMap, BTreeSet};

use ambition_platformer2d_core::snapshot::RollbackRegistrar;
use bevy_ecs::prelude::{Message, Resource};

/// The idle timeout a game's file states when it states none.
pub fn default_gate_idle_timeout_secs() -> u32 {
    30
}

/// How many recent addresses the network remembers for a terminal's Recent
/// view.
pub const GATE_RECENT_LEN: usize = 6;

/// How far a portal reaches on the flat map when its ring states no reach, in
/// world units (pixels). Lore: a fold is only so deep. About a region and its
/// neighbours: a dimension's far side takes a chain of arches, a walk, or the
/// long way (no surface arch reaches orbit).
pub const DEFAULT_PORTAL_REACH: u32 = 8000;

/// What kind of ring a ring is: a gate between worlds, or a portal within
/// one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Deserialize)]
pub enum RingKind {
    /// A dimension gate: dials gates, of any dimension, at any distance.
    #[default]
    Gate,
    /// A portal arch: reaches portals of its own dimension within its reach.
    Portal,
}

/// Where each ring stands on its world's flat map (its zone's middle, in
/// world units), by ring zone id, and the world file it is in. Derived from
/// the world files when they load (a host fact, like the room set): the
/// catalog states no place, so a room moved in the editor moves its ring.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct GateRingPositions(pub BTreeMap<String, RingPosition>);

/// One ring's place on the flat map.
#[derive(Clone, Debug, PartialEq)]
pub struct RingPosition {
    /// The world file (`WorldSource` id) whose coordinates `at` is in.
    pub world: String,
    /// The middle of the ring's zone, in world units.
    pub at: (f32, f32),
}

impl GateRingPositions {
    /// The distance between two rings on one flat map; `None` when either is
    /// not placed or they are in different world files.
    pub fn distance(&self, a: &str, b: &str) -> Option<f32> {
        let (a, b) = (self.0.get(a)?, self.0.get(b)?);
        (a.world == b.world).then(|| ((a.at.0 - b.at.0).powi(2) + (a.at.1 - b.at.1).powi(2)).sqrt())
    }
}

/// One chevron of a ring: its place on the ring, from the first.
pub type Chevron = u8;

/// Which addresses a game's rings can be dialed by, beyond having chevrons
/// the ring has: the game's spelling (how long an address may be, how often a
/// chevron may repeat) is its own. The dial asks it before it looks the
/// address up ([`GateFacts::fits`]); a session reads it from this resource,
/// which a game inserts. Absent, every address can be dialed.
#[derive(Resource, Clone, Copy, Debug)]
pub struct GateAddressRule(pub fn(&[Chevron]) -> bool);

impl Default for GateAddressRule {
    fn default() -> Self {
        Self(|_| true)
    }
}

/// A ring's address became known: the ring answered a dial, or a portal's
/// terminal was opened at it (standing at a portal attunes it). The engine
/// keeps no knowledge; a game records what it means (Ambition: a save flag
/// its terminals list by), read in the same tick after the dial.
#[derive(Message, Clone, Debug, PartialEq, Eq)]
pub struct GateAddressLearned {
    /// The zone of the ring whose address became known.
    pub ring: String,
}

/// A game's gate network, as its game lowered it.
#[derive(Resource, Clone, Debug, PartialEq, Eq)]
pub struct GateNetworkCatalog {
    /// Idle simulation seconds before an unused connection closes.
    pub idle_timeout_secs: u32,
    /// How many chevrons a ring has: an address names chevrons `0..chevrons`.
    pub chevrons: u16,
    /// Each logistical facility that holds rings. Two rings in one facility
    /// never dial each other: that is a walk, not a gate trip.
    pub facilities: Vec<GateFacility>,
    /// Each ring of the network.
    pub rings: Vec<GateRing>,
}

impl Default for GateNetworkCatalog {
    fn default() -> Self {
        Self { idle_timeout_secs: default_gate_idle_timeout_secs(), chevrons: 0,
            facilities: Vec::new(), rings: Vec::new() }
    }
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
#[derive(Clone, Debug, PartialEq, Eq)]
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
    /// The chevrons that reach this ring. Every ring has an address.
    pub address: Vec<Chevron>,
    /// A hidden address is not in any directory. A dial reaches it the same
    /// way it reaches any other.
    pub hidden: bool,
    /// The id of the terminal (`Switch` with action `GateTerminal`) that
    /// dials from this ring. Every ring has one; `None` is a catalog problem.
    pub terminal: Option<String>,
    /// The ring's infrastructure state, from authored facts only.
    pub status: GateStatusRule,
    /// A line the terminal shows about the ring (its upkeep, its traffic).
    pub note: String,
    /// A gate (the default) or a portal arch.
    pub kind: RingKind,
    /// How far a portal reaches on the flat map, in world units; `None` is
    /// [`DEFAULT_PORTAL_REACH`]. A gate states none: a gate has no reach.
    pub reach: Option<u32>,
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

/// Why a dial did not open a link. Each kind is its own, so a game can word
/// a wrong address apart from a ring that is down (the words are the game's).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DialRefusal {
    /// No chevrons were entered.
    Empty,
    /// A chevron the ring does not have.
    NoSuchChevron(Chevron),
    /// The game's spelling refuses it ([`GateAddressRule`]).
    Malformed,
    /// No ring answers the address.
    Unknown,
    /// The address is a ring in the dialing ring's own facility.
    SameFacility,
    /// A ring is down (the dialing ring or the far one), for the reason.
    Offline(String),
    /// A ring is in use, for the reason.
    Busy(String),
    /// A gate dialed a portal or a portal a gate: a gate dials gates, and a
    /// portal reaches portals.
    WrongKind,
    /// A portal reached for a portal of another dimension: a portal folds its
    /// own world, and another world takes a gate.
    OtherDimension,
    /// The far portal is farther than this one reaches.
    OutOfReach { distance: u32, reach: u32 },
}

impl DialRefusal {
    /// The kind, as one word, for a status line and for a test.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::NoSuchChevron(_) | Self::Malformed => "illegal",
            Self::Unknown => "unknown",
            Self::SameFacility => "same_facility",
            Self::Offline(_) => "offline",
            Self::Busy(_) => "busy",
            Self::WrongKind => "wrong_kind",
            Self::OtherDimension => "other_dimension",
            Self::OutOfReach { .. } => "out_of_reach",
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
    /// The flat-map distance between two rings (by zone), when both are
    /// placed in one world file ([`GateRingPositions::distance`]).
    pub distance: &'a dyn Fn(&str, &str) -> Option<f32>,
    /// Whether the game's spelling admits an address ([`GateAddressRule`]).
    pub fits: &'a dyn Fn(&[Chevron]) -> bool,
}

impl GateNetworkCatalog {
    /// The fewest room crossings through authored doors and available rings.
    /// Each ring edge must pass the same checks as a real dial. Addresses
    /// come from the catalog; no route keeps a separate address table.
    pub fn route(
        &self,
        rooms: &super::RoomSet,
        state: &GateNetworkState,
        from: usize,
        to: usize,
        facts: &GateFacts<'_>,
    ) -> Option<Vec<super::RoomHop>> {
        let mut links = Vec::new();
        for source in &self.rings {
            let Some(from) = rooms.room_index_by_id(&source.room) else { continue };
            if source.terminal.is_none() || !rooms.has_zone(&source.room, &source.zone) { continue; }
            for far in &self.rings {
                if resolve_dial(self, state, &source.zone, &far.address, facts).is_err() { continue; }
                let Some(to) = rooms.room_index_by_id(&far.room) else { continue };
                links.push(super::RoomHop { from, to, zone: source.zone.clone(), dial: Some(far.address.clone()) });
            }
        }
        rooms.route_with_links(from, to, &links)
    }

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
        self.rings.iter().find(|ring| ring.address == address)
    }

    /// How many chevrons a ring has.
    pub fn chevrons(&self) -> usize {
        usize::from(self.chevrons)
    }

    /// The facility `id`.
    pub fn facility(&self, id: &str) -> Option<&GateFacility> {
        self.facilities.iter().find(|facility| facility.id == id)
    }
}

impl GateRing {
    /// How far this ring reaches on the flat map, in world units: a portal's
    /// reach (its own, else [`DEFAULT_PORTAL_REACH`]); a gate reaches
    /// anywhere.
    pub fn reach_px(&self) -> u32 {
        match self.kind {
            RingKind::Gate => u32::MAX,
            RingKind::Portal => self.reach.unwrap_or(DEFAULT_PORTAL_REACH),
        }
    }

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
/// Between 3 and 4, the kinds: a gate dials only gates and a portal reaches
/// only portals; a portal reaches only its own dimension, and only within its
/// reach (an unplaced ring is out of reach: nothing measures it).
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
    if !(facts.fits)(address) {
        return Err(DialRefusal::Malformed);
    }
    let Some(from) = catalog.ring(source) else {
        return Err(DialRefusal::Offline("this terminal has no ring".to_string()));
    };
    ring_status(from, facts)?;
    let Some(far) = catalog.ring_at(address) else {
        return Err(DialRefusal::Unknown);
    };
    if far.kind != from.kind {
        return Err(DialRefusal::WrongKind);
    }
    if from.kind == RingKind::Portal {
        let dimension = |ring: &GateRing| catalog.facility(&ring.facility).map(|facility| facility.dimension.as_str());
        if dimension(far) != dimension(from) {
            return Err(DialRefusal::OtherDimension);
        }
        let reach = from.reach_px();
        match (facts.distance)(&from.zone, &far.zone) {
            Some(distance) if distance <= reach as f32 => {}
            Some(distance) => return Err(DialRefusal::OutOfReach { distance: distance.round() as u32, reach }),
            None => return Err(DialRefusal::OutOfReach { distance: u32::MAX, reach }),
        }
    }
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
    if catalog.idle_timeout_secs == 0 {
        problems.push("the gate idle timeout must be greater than zero".into());
    }
    if catalog.chevrons == 0 {
        problems.push("a ring has no chevrons".to_string());
    }
    if catalog.chevrons() > usize::from(Chevron::MAX) + 1 {
        problems.push("a ring has more chevrons than a dial can name".to_string());
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
    let mut addresses: BTreeMap<&[Chevron], &str> = BTreeMap::new();
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
        if ring.kind == RingKind::Gate && ring.reach.is_some() {
            problems.push(format!("the gate `{zone}` states a reach: a gate has none (only a portal reaches so far)"));
        }
        if ring.reach == Some(0) {
            problems.push(format!("the portal `{zone}` reaches 0: it reaches nothing"));
        }
        if !facilities.contains(ring.facility.as_str()) {
            problems.push(format!("`{zone}.facility` `{}` is not a facility", ring.facility));
        }
        if ring.address.is_empty() {
            problems.push(format!("`{zone}` has no address: every ring can be dialed"));
        } else if ring.address.iter().any(|chevron| usize::from(*chevron) >= catalog.chevrons()) {
            problems.push(format!("`{zone}.address` names a chevron the ring does not have"));
        } else if let Some(other) = addresses.insert(&ring.address, zone) {
            problems.push(format!("`{other}` and `{zone}` answer the same chevrons"));
        }
        if ring.terminal.is_none() {
            problems.push(format!("`{zone}` has no terminal: every ring can be dialed from"));
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
    }
    problems
}

/// One end of an open connection, with its address fixed at dial time.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct GateEndpoint {
    pub zone: String,
    pub room: String,
    pub address: Vec<Chevron>,
}

/// One connection. Its dialing endpoint is the map key; its other end is here.
#[derive(Clone, Debug, PartialEq)]
pub struct GateLink {
    pub far: GateEndpoint,
    pub remaining: f32,
    pub idle_timeout_secs: u32,
    /// A redial's link: it waits for its rings to have closed before it holds
    /// them open, so the old wormhole closes and the new one opens, rather
    /// than one wormhole taking another's address. It routes nothing while it
    /// waits, and it reserves its rings.
    pub opens_after_close: bool,
}

/// The route from either end of a connection.
#[derive(Clone, Copy, Debug)]
pub struct GateRoute<'a> {
    pub far: &'a str,
    pub far_room: &'a str,
    pub address: &'a [Chevron],
}

/// The answer to the latest dial of one terminal.
#[derive(Clone, Debug, PartialEq)]
pub struct DialReport {
    /// The ring whose dial produced this answer.
    pub ring: String,
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
    /// The actor using this terminal.
    pub subject: ambition_platformer2d_shared_tangle::lifecycle::LiveBodyId,
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

/// The live state of the network: each actor's terminal use, each open link,
/// and the latest answers. Rollback state: the links decide when a ring is
/// open, and a ring that is open decides a room transition.
///
/// Actor identities and ring zone ids keep each map in a stable order.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct GateNetworkState {
    /// The ring each actor is using.
    pub terminals: BTreeMap<ambition_platformer2d_shared_tangle::lifecycle::LiveBodyId, String>,
    /// Each connection, keyed by its dialing endpoint.
    pub links: BTreeMap<GateEndpoint, GateLink>,
    /// The latest dial answer for each actor.
    pub reports: BTreeMap<ambition_platformer2d_shared_tangle::lifecycle::LiveBodyId, DialReport>,
    /// The chevrons of recent links, newest first.
    pub recent: Vec<Vec<Chevron>>,
    /// The serial the next report takes.
    pub next_serial: u64,
}

impl GateNetworkState {
    /// The destination and address reached through `ring` in either direction.
    pub fn route_from(&self, ring: &str) -> Option<GateRoute<'_>> {
        self.links.iter().filter(|(_, link)| !link.opens_after_close).find_map(|(source, link)| {
            let far = if source.zone == ring { &link.far }
                else if link.far.zone == ring { source } else { return None };
            Some(GateRoute { far: &far.zone, far_room: &far.room, address: &far.address })
        })
    }

    pub fn holds_open(&self, ring: &str) -> bool {
        self.route_from(ring).is_some()
    }

    pub fn connected_rooms(&self) -> impl Iterator<Item = &str> {
        self.links.iter().flat_map(|(source, link)| [source.room.as_str(), link.far.room.as_str()])
    }

    fn engaged_elsewhere(&self, ring: &str, source: &str) -> bool {
        self.links.iter().any(|(from, link)| from.zone != source && link.far.zone != source
            && (from.zone == ring || link.far.zone == ring))
    }

    /// Closing either endpoint closes the one connection.
    pub fn close_link(&mut self, ring: &str) {
        self.links.retain(|source, link| source.zone != ring && link.far.zone != ring);
    }

    /// The latest answer to this actor at this ring.
    pub fn report_for(&self, subject: &ambition_platformer2d_shared_tangle::lifecycle::LiveBodyId, ring: &str) -> Option<&DialReport> {
        self.reports.get(subject).filter(|report| report.ring == ring)
    }

    /// Record the answer of a dial from `ring`.
    pub fn report(&mut self, subject: &ambition_platformer2d_shared_tangle::lifecycle::LiveBodyId, ring: &str, address: Vec<Chevron>, outcome: Result<String, DialRefusal>) {
        let serial = self.next_serial;
        self.next_serial += 1;
        self.reports.insert(subject.clone(), DialReport { ring: ring.to_string(), serial, address, outcome });
    }

    /// Connect two rings after the dial rules accept their addresses.
    pub fn open_link(&mut self, catalog: &GateNetworkCatalog, ring: &str, far: &str) -> Result<(), DialRefusal> {
        let endpoint = |zone: &str| {
            let ring = catalog.ring(zone)?;
            Some(GateEndpoint { zone: ring.zone.clone(), room: ring.room.clone(), address: ring.address.clone() })
        };
        let source = endpoint(ring).ok_or(DialRefusal::Unknown)?;
        let far = endpoint(far).ok_or(DialRefusal::Unknown)?;
        if source.zone == far.zone { return Err(DialRefusal::SameFacility); }
        if self.engaged_elsewhere(&far.zone, &source.zone) {
            return Err(DialRefusal::Busy("the ring already has a connection".into()));
        }
        self.close_link(ring);
        self.recent.retain(|recent| *recent != far.address);
        self.recent.insert(0, far.address.clone());
        self.recent.truncate(GATE_RECENT_LEN);
        self.links.insert(source, GateLink {
            far, remaining: catalog.idle_timeout_secs as f32, idle_timeout_secs: catalog.idle_timeout_secs,
            opens_after_close: false,
        });
        Ok(())
    }

    /// Make `ring`'s new link wait for its rings to close (a redial).
    pub fn open_after_close(&mut self, ring: &str) {
        for (source, link) in &mut self.links {
            if source.zone == ring {
                link.opens_after_close = true;
            }
        }
    }

    /// Release each waiting link whose rings are both closed now.
    pub fn release_closed(&mut self, closed: impl Fn(&str) -> bool) {
        for (source, link) in &mut self.links {
            if link.opens_after_close && closed(&source.zone) && closed(&link.far.zone) {
                link.opens_after_close = false;
            }
        }
    }

    /// An admitted traversal restarts the shared idle timer.
    pub fn used(&mut self, ring: &str) {
        for (source, link) in &mut self.links {
            if source.zone == ring || link.far.zone == ring {
                link.remaining = link.idle_timeout_secs as f32;
            }
        }
    }

    /// Active passages and pending crossings keep both ends open.
    pub fn tick(&mut self, dt: f32, in_use: impl Fn(&str) -> bool) {
        for (source, link) in &mut self.links {
            if in_use(&source.zone) || in_use(&link.far.zone) {
                link.remaining = link.idle_timeout_secs as f32;
            } else {
                link.remaining -= dt.max(0.0);
            }
        }
        self.links.retain(|_, link| link.remaining > 0.0);
    }

}

/// The value projection of [`GateNetworkState`] for the rollback checksum:
/// every field that decides a ring's phase or a transition.
pub fn gate_network_state_checksum(state: &GateNetworkState) -> u64 {
    use ambition_platformer2d_core::snapshot::{checksum_bytes, put_f32, put_str, put_u64};
    let put_chevrons = |bytes: &mut Vec<u8>, address: &[Chevron]| {
        put_u64(bytes, address.len() as u64);
        bytes.extend_from_slice(address);
    };
    let mut bytes = Vec::new();
    put_u64(&mut bytes, state.terminals.len() as u64);
    for (subject, ring) in &state.terminals {
        subject.write_peer_stable_into(&mut bytes);
        put_str(&mut bytes, ring);
    }
    put_u64(&mut bytes, state.links.len() as u64);
    for (source, link) in &state.links {
        for endpoint in [source, &link.far] {
            put_str(&mut bytes, &endpoint.zone);
            put_str(&mut bytes, &endpoint.room);
            put_chevrons(&mut bytes, &endpoint.address);
        }
        put_f32(&mut bytes, link.remaining);
        put_u64(&mut bytes, u64::from(link.idle_timeout_secs));
        put_u64(&mut bytes, u64::from(link.opens_after_close));
    }
    put_u64(&mut bytes, state.reports.len() as u64);
    for (subject, report) in &state.reports {
        subject.write_peer_stable_into(&mut bytes);
        put_str(&mut bytes, &report.ring);
        put_u64(&mut bytes, report.serial);
        put_chevrons(&mut bytes, &report.address);
        match &report.outcome {
            Ok(far) => put_str(&mut bytes, far),
            Err(refusal) => {
                put_str(&mut bytes, refusal.kind());
                match refusal {
                    DialRefusal::NoSuchChevron(chevron) => put_u64(&mut bytes, u64::from(*chevron)),
                    DialRefusal::Offline(reason) | DialRefusal::Busy(reason) => put_str(&mut bytes, reason),
                    DialRefusal::OutOfReach { distance, reach } => {
                        put_u64(&mut bytes, u64::from(*distance));
                        put_u64(&mut bytes, u64::from(*reach));
                    }
                    DialRefusal::Empty | DialRefusal::Malformed | DialRefusal::Unknown | DialRefusal::SameFacility
                    | DialRefusal::WrongKind | DialRefusal::OtherDimension => {}
                }
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
        "actor terminal uses, links, reports and recent addresses",
        gate_network_state_checksum,
    );
    registrar.clear_message_on_rollback::<GateTerminalIntent>(
        super::GATE_PORTAL_ROLLBACK_OWNER,
        "message.gate_terminal_intent",
    );
    registrar.clear_message_on_rollback::<GateAddressLearned>(
        super::GATE_PORTAL_ROLLBACK_OWNER,
        "message.gate_address_learned",
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ring(zone: &str, facility: &str, address: &[Chevron]) -> GateRing {
        GateRing {
            zone: zone.into(),
            name: zone.into(),
            facility: facility.into(),
            room: format!("{zone}_room"),
            address: address.to_vec(),
            hidden: false,
            terminal: Some(format!("{zone}_terminal")),
            status: GateStatusRule::Online,
            note: String::new(),
            kind: RingKind::Gate,
            reach: None,
        }
    }

    fn catalog() -> GateNetworkCatalog {
        let facility = |id: &str, dimension: &str| GateFacility {
            id: id.into(),
            name: id.into(),
            dimension: dimension.into(),
        };
        let mut ninja = ring("ninja", "ninja_shrine", &[6, 7, 8, 10]);
        ninja.status = GateStatusRule::Offline("the far ring is not yet built".into());
        let mut ring7 = ring("ring7", "home", &[9, 13, 2, 6, 11, 0]);
        ring7.status = GateStatusRule::PoweredBy { switch: "breaker".into(), reason: "no power".into() };
        let mut station = ring("station", "station", &[2, 3, 0, 3, 11, 10, 7, 11]);
        station.status = GateStatusRule::BusyWhile { flag: "convoy".into(), reason: "a freight convoy".into() };
        GateNetworkCatalog {
            idle_timeout_secs: 30,
            chevrons: 15,
            facilities: vec![
                facility("home", "home"),
                facility("math", "math"),
                facility("ninja_shrine", "ninja"),
                facility("fort", "home"),
                facility("station", "science"),
                facility("trail", "science"),
            ],
            rings: vec![
                ring("hub", "home", &[0, 1, 2, 3, 4]),
                ring7,
                ring("math", "math", &[10, 3, 12, 2, 1, 12, 14]),
                ninja,
                ring("fort", "fort", &[8, 1, 12, 10, 7, 12, 1, 0]),
                station,
                ring("trail", "trail", &[3, 8, 11, 1, 12, 10]),
            ],
        }
    }

    /// The chevrons that reach the ring `zone`.
    fn at(catalog: &GateNetworkCatalog, zone: &str) -> Vec<Chevron> {
        catalog.ring(zone).expect("the fixture has the ring").address.clone()
    }

    #[test]
    fn a_route_combines_doors_and_available_gates_without_a_door_between_worlds() {
        use super::super::{LoadingZone, LoadingZoneActivation, RoomLink, RoomSet, RoomSpec};
        use ambition_platformer2d_core::{Aabb, Vec2, World};

        let mut catalog = catalog();
        catalog.rings.retain(|ring| ["hub", "math"].contains(&ring.zone.as_str()));
        let room = |id: &str, zone: &str| {
            let mut room = RoomSpec::new(id, World::new(id, Vec2::splat(500.0), Vec2::splat(100.0), vec![]));
            room.loading_zones.push(LoadingZone { presentation: Default::default(),
                id: zone.into(), name: zone.into(), activation: LoadingZoneActivation::Door,
                aabb: Aabb::new(Vec2::splat(100.0), Vec2::splat(30.0)),
            });
            room
        };
        let rooms = RoomSet::from_parts_or_panic("approach", vec![
            room("approach", "door"), room("hub_room", "hub"),
            room("math_room", "math"), room("destination", "return"),
        ], vec![
            RoomLink { from_room: "approach".into(), from_zone: "door".into(), to_room: "hub_room".into(), to_zone: "hub".into(), bidirectional: true },
            RoomLink { from_room: "math_room".into(), from_zone: "math".into(), to_room: "destination".into(), to_zone: "return".into(), bidirectional: true },
        ]);
        let state = GateNetworkState::default();
        let facts = GateFacts {
            switch_on: &|_| false, flag: &|_| false,
            zone_exists: &|room, zone| rooms.has_zone(room, zone), distance: &|_, _| None, fits: &|_| true,
        };
        assert!(rooms.route(0, 3).is_none());
        let path = catalog.route(&rooms, &state, 0, 3, &facts).expect("door, gate, door");
        assert_eq!(path.iter().map(|hop| (hop.from, hop.to)).collect::<Vec<_>>(), [(0, 1), (1, 2), (2, 3)]);
        assert!(path[0].dial.is_none() && path[2].dial.is_none());
        assert_eq!(path[1].dial, Some(at(&catalog, "math")));
        let reverse = catalog.route(&rooms, &state, 3, 0, &facts).expect("the reverse trip dials home");
        assert_eq!(reverse[1].dial, Some(at(&catalog, "hub")));
        catalog.rings[1].status = GateStatusRule::PoweredBy { switch: "power".into(), reason: "no power".into() };
        assert!(catalog.route(&rooms, &state, 0, 3, &facts).is_none(), "a route must not promise a refused dial");
        let powered = GateFacts { switch_on: &|id| id == "power", ..facts };
        assert!(catalog.route(&rooms, &state, 0, 3, &powered).is_some());
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
        let distance = |_: &str, _: &str| Some(0.0);
        let facts = GateFacts { switch_on: &switch_on, flag: &flag, zone_exists: &zone_exists, distance: &distance, fits: &|address: &[Chevron]| address.len() <= 16 };
        resolve_dial(catalog, state, from, address, &facts).map(|ring| ring.zone.as_str())
    }

    /// A portal arch (Jon, 2026-10-31: "portals should generally be within
    /// dimension, whereas dimension gates are a different thing. Portals
    /// maybe even have some lore-like range"): it reaches a portal of its own
    /// dimension within its reach; not a farther one, not one in another
    /// world, and not a gate. A gate dials no portal.
    #[test]
    fn a_portal_reaches_its_own_world_within_its_reach_and_a_gate_reaches_no_portal() {
        let mut catalog = catalog();
        catalog.facilities.push(GateFacility { id: "grove".into(), name: "grove".into(), dimension: "science".into() });
        catalog.facilities.push(GateFacility { id: "deep".into(), name: "deep".into(), dimension: "science".into() });
        catalog.facilities.push(GateFacility { id: "glade".into(), name: "glade".into(), dimension: "math".into() });
        let portal = |zone: &str, facility: &str, address: &[Chevron]| GateRing {
            kind: RingKind::Portal,
            reach: Some(5000),
            ..ring(zone, facility, address)
        };
        catalog.rings.push(portal("grove_arch", "grove", &[2, 0, 3, 6, 1]));
        catalog.rings.push(portal("deep_arch", "deep", &[6, 1, 12, 10]));
        catalog.rings.push(portal("glade_arch", "glade", &[2, 0, 1, 12]));
        let placed = |zone: &str| match zone {
            "grove_arch" => Some((0.0, 0.0)),
            "deep_arch" => Some((3000.0, 4000.0)),
            "glade_arch" => Some((100.0, 0.0)),
            _ => None,
        };
        let state = GateNetworkState::default();
        let attempt = |from: &str, to: &str, far_at: (f32, f32)| {
            let switch_on = |_: &str| true;
            let flag = |_: &str| false;
            let zone_exists = |_: &str, _: &str| true;
            let distance = |a: &str, b: &str| {
                let (a, b) = (placed(a)?, if b == "deep_arch" { Some(far_at) } else { placed(b) }?);
                Some(((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt())
            };
            let facts = GateFacts { switch_on: &switch_on, flag: &flag, zone_exists: &zone_exists, distance: &distance, fits: &|address: &[Chevron]| address.len() <= 16 };
            resolve_dial(&catalog, &state, from, &at(&catalog, to), &facts).map(|ring| ring.zone.clone())
        };
        // 5000 away, and it reaches 5000: in reach.
        assert_eq!(attempt("grove_arch", "deep_arch", (3000.0, 4000.0)).as_deref(), Ok("deep_arch"));
        // 6000 away: too far.
        assert_eq!(attempt("grove_arch", "deep_arch", (6000.0, 0.0)).unwrap_err().kind(), "out_of_reach");
        // 100 away, but in another world: a portal does not fold another one.
        assert_eq!(attempt("grove_arch", "glade_arch", (0.0, 0.0)).unwrap_err().kind(), "other_dimension");
        // A portal reaches no gate, and a gate dials no portal.
        assert_eq!(attempt("grove_arch", "trail", (0.0, 0.0)).unwrap_err().kind(), "wrong_kind");
        assert_eq!(attempt("trail", "deep_arch", (0.0, 0.0)).unwrap_err().kind(), "wrong_kind");
    }

    /// A gate states no reach: the catalog refuses one that does.
    #[test]
    fn a_gate_with_a_reach_is_a_catalog_problem() {
        let mut catalog = catalog();
        catalog.rings[0].reach = Some(100);
        let problems = gate_network_problems(&catalog);
        assert!(problems.iter().any(|problem| problem.contains("states a reach")), "{problems:?}");
    }

    /// A dial must enter the address's exact chevrons: one chevron off is
    /// another address, which no ring answers.
    #[test]
    fn a_dial_answers_the_exact_chevrons_only() {
        let catalog = catalog();
        let state = GateNetworkState::default();
        assert_eq!(dial(&catalog, &state, "hub", &at(&catalog, "math"), &[]), Ok("math"));
        assert_eq!(dial(&catalog, &state, "math", &[0, 1, 2, 3, 4], &[]), Ok("hub"));
        assert_eq!(dial(&catalog, &state, "math", &[0, 1, 2, 3, 0], &[]), Err(DialRefusal::Unknown));
    }

    #[test]
    fn each_refusal_has_its_own_kind() {
        let catalog = catalog();
        let mut state = GateNetworkState::default();
        let kind = |result: Result<&str, DialRefusal>| result.unwrap_err().kind();
        assert_eq!(kind(dial(&catalog, &state, "hub", &[], &[])), "empty");
        assert_eq!(dial(&catalog, &state, "hub", &[10, 15], &[]), Err(DialRefusal::NoSuchChevron(15)));
        assert_eq!(kind(dial(&catalog, &state, "hub", &[7, 7, 7], &[])), "unknown");
        assert_eq!(kind(dial(&catalog, &state, "hub", &at(&catalog, "ninja"), &[])), "offline");
        assert_eq!(kind(dial(&catalog, &state, "hub", &at(&catalog, "station"), &["convoy"])), "busy");
        let tangent = at(&catalog, "math");
        state.open_link(&catalog, "trail", "math").unwrap();
        assert_eq!(kind(dial(&catalog, &state, "hub", &tangent, &[])), "busy");
        assert_eq!(kind(dial(&catalog, &state, "hub", &at(&catalog, "trail"), &[])), "busy", "a sending ring is engaged too");
    }

    /// No faction refuses a dial: every working ring answers every ring, the
    /// military one too.
    #[test]
    fn no_ring_refuses_a_dial_by_who_dials() {
        let catalog = catalog();
        let state = GateNetworkState::default();
        assert_eq!(dial(&catalog, &state, "hub", &at(&catalog, "fort"), &[]), Ok("fort"));
        assert_eq!(dial(&catalog, &state, "ring7", &at(&catalog, "trail"), &["breaker"]), Ok("trail"));
    }

    #[test]
    fn a_ring_never_dials_its_own_facility_but_may_dial_its_own_dimension() {
        let catalog = catalog();
        let state = GateNetworkState::default();
        let legal = at(&catalog, "hub");
        assert_eq!(dial(&catalog, &state, "hub", &legal, &[]), Err(DialRefusal::SameFacility));
        assert_eq!(dial(&catalog, &state, "ring7", &legal, &["breaker"]), Err(DialRefusal::SameFacility));
        assert_eq!(dial(&catalog, &state, "station", &at(&catalog, "trail"), &[]), Ok("trail"));
        assert_eq!(dial(&catalog, &state, "trail", &at(&catalog, "station"), &[]), Ok("station"));
    }

    /// The game's spelling refuses an address before it is looked up (the
    /// test's rule: at most 16 chevrons).
    #[test]
    fn the_games_rule_refuses_an_address_it_can_not_spell() {
        let catalog = catalog();
        let state = GateNetworkState::default();
        assert_eq!(dial(&catalog, &state, "hub", &[3; 17], &[]), Err(DialRefusal::Malformed));
        assert_ne!(dial(&catalog, &state, "hub", &[3; 16], &[]), Err(DialRefusal::Malformed));
    }

    #[test]
    fn a_powered_source_answers_by_its_breaker() {
        let catalog = catalog();
        let state = GateNetworkState::default();
        let tangent = at(&catalog, "math");
        assert_eq!(dial(&catalog, &state, "ring7", &tangent, &[]).unwrap_err().kind(), "offline");
        assert_eq!(dial(&catalog, &state, "ring7", &tangent, &["breaker"]), Ok("math"));
    }

    #[test]
    fn a_connection_routes_both_ways_and_counts_idle_time_after_use() {
        let mut catalog = catalog();
        catalog.idle_timeout_secs = 5;
        let mut state = GateNetworkState::default();
        state.open_link(&catalog, "hub", "math").unwrap();
        assert_eq!(state.links.len(), 1);
        assert_eq!(state.route_from("hub").unwrap().far, "math");
        let back = state.route_from("math").unwrap();
        assert_eq!(back.far, "hub");
        assert_eq!(back.far_room, catalog.ring("hub").unwrap().room);
        assert_eq!(back.address, at(&catalog, "hub"));
        state.tick(4.0, |_| false);
        state.used("hub");
        state.tick(4.0, |_| false);
        assert!(state.holds_open("hub") && state.holds_open("math"));
        state.used("math");
        state.tick(4.0, |_| false);
        assert!(state.holds_open("hub"));
        state.tick(1.1, |_| false);
        assert!(!state.holds_open("hub") && !state.holds_open("math"));
    }

    #[test]
    fn an_active_crossing_holds_the_connection_and_either_terminal_can_close_it() {
        let catalog = catalog();
        let mut state = GateNetworkState::default();
        for active in ["hub", "math"] {
            state.open_link(&catalog, "hub", "math").unwrap();
            state.tick(100.0, |ring| ring == active);
            assert!(state.holds_open("hub") && state.holds_open("math"));
            state.close_link(active);
            assert!(state.links.is_empty());
        }
        state.open_link(&catalog, "hub", "math").unwrap();
        state.open_link(&catalog, "math", "trail").unwrap();
        assert!(!state.holds_open("hub"));
        assert_eq!(state.route_from("trail").unwrap().far, "math");
        assert_eq!(state.links.len(), 1);
    }

    #[test]
    fn terminal_uses_and_replies_keep_the_actor_and_its_live_room() {
        use ambition_platformer2d_shared_tangle::lifecycle::{LiveBodyId, LiveRoomInstance};
        use ambition_platformer2d_shared_tangle::sim_id::SimId;
        let first = LiveBodyId::new(SimId::placement("traveller"), Some(LiveRoomInstance::from_ordinal(1)));
        let second = LiveBodyId::new(SimId::placement("traveller"), Some(LiveRoomInstance::from_ordinal(2)));
        let mut state = GateNetworkState::default();
        state.terminals.insert(first.clone(), "hub".into());
        state.terminals.insert(second.clone(), "hub".into());
        state.report(&first, "hub", vec![1], Ok("math".into()));
        state.report(&second, "hub", vec![2], Err(DialRefusal::Unknown));
        assert_eq!(state.report_for(&first, "hub").unwrap().outcome, Ok("math".into()));
        assert_eq!(state.report_for(&second, "hub").unwrap().outcome, Err(DialRefusal::Unknown));
        assert!(state.report_for(&first, "other").is_none());
        let mut other = state.clone();
        other.terminals.remove(&first);
        assert_ne!(gate_network_state_checksum(&state), gate_network_state_checksum(&other));
        other = state.clone();
        other.reports.remove(&second);
        assert_ne!(gate_network_state_checksum(&state), gate_network_state_checksum(&other));
    }

    #[test]
    fn the_checksum_sees_a_timer_and_a_report() {
        let mut a = GateNetworkState::default();
        a.open_link(&catalog(), "hub", "math").unwrap();
        let mut b = a.clone();
        assert_eq!(gate_network_state_checksum(&a), gate_network_state_checksum(&b));
        b.tick(1.0 / 60.0, |_| false);
        assert_ne!(gate_network_state_checksum(&a), gate_network_state_checksum(&b));
        let mut c = a.clone();
        c.report(&ambition_platformer2d_shared_tangle::lifecycle::LiveBodyId::new(ambition_platformer2d_shared_tangle::sim_id::SimId::from_snapshot("user".into()), None), "hub", vec![9, 3], Err(DialRefusal::Unknown));
        assert_ne!(gate_network_state_checksum(&a), gate_network_state_checksum(&c));
        let mut d = a.clone();
        d.report(&ambition_platformer2d_shared_tangle::lifecycle::LiveBodyId::new(ambition_platformer2d_shared_tangle::sim_id::SimId::from_snapshot("user".into()), None), "hub", vec![9, 4], Err(DialRefusal::Unknown));
        assert_ne!(gate_network_state_checksum(&c), gate_network_state_checksum(&d), "the chevrons themselves count");
    }

    #[test]
    fn a_catalog_problem_is_named_by_its_ring_and_field() {
        assert!(gate_network_problems(&catalog()).is_empty(), "{:?}", gate_network_problems(&catalog()));
        let mut bad = catalog();
        bad.idle_timeout_secs = 0;
        bad.rings[2].address = vec![0, 1, 2, 3, 4];
        bad.rings[3].address = vec![6, 15];
        bad.rings[4].facility = "nowhere".into();
        bad.rings[5].terminal = None;
        bad.rings.push(ring("hub", "home", &[]));
        bad.rings.push(ring("twin", "trail", &[9, 13, 2, 6, 11, 0]));
        let problems = gate_network_problems(&bad);
        for expected in [
            "the gate idle timeout must be greater than zero",
            "`hub` and `math` answer the same chevrons",
            "`ninja.address` names a chevron the ring does not have",
            "`fort.facility` `nowhere` is not a facility",
            "the ring `hub` is stated twice",
            "`hub` has no address: every ring can be dialed",
            "`station` has no terminal: every ring can be dialed from",
            "`ring7` and `twin` answer the same chevrons",
        ] {
            assert!(problems.iter().any(|p| p.contains(expected)), "missing {expected:?} in {problems:?}");
        }
    }
}

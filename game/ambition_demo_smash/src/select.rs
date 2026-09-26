//! Pure character-select state for up to four match seats.
//!
//! Each slot independently records its occupant (absent/controller/CPU) and selected
//! character. A match may start only when at least two slots participate and every
//! participating slot has selected a character. Input sources may claim a slot by
//! selecting a fighter or by explicitly changing a slot role; rendering/input handling
//! lives in `select_screen`.

#[cfg(test)]
use crate::STARTING_STOCKS;
use crate::{MatchParticipant, MatchParticipantRoster};

/// One screen, four slots: the same ceiling as the versus stage and
/// `SlotControls`.
pub const MAX_SMASH_SEATS: usize = 4;

/// Ordered fighter IDs requested by the select grid.
///
/// Layout derives from this list's length. [`SmashRoster::assemble`] drops IDs
/// unavailable in the current composition while preserving order. Cross-game
/// fighters are shared by ID; do not declare duplicate character copies here.
pub const SMASH_ROSTER: &[&str] = &[
    // The content catalog's robot v3; the demo does not declare its own copy.
    "player_robot_v3",
    // This demo's own, on a sheet nobody else claims.
    crate::SMASH_GEORGE_BOOUL,
    // Mary-O's tall form. Both Mary-O ids have identical kits; only the sheet
    // differs, so no test notices a swap.
    "mary_o_tall",
    "sanic",
    // Ambition's own cast.
    "npc_pirate_admiral",
    "npc_ninja_shadow_oni_leader",
    "npc_alice",
    "npc_bob",
    "npc_oiler",
    "perfect_cellular_automaton",
    "goblin",
    "npc_emmy_noether",
    // Both author their own repertoire (their catalog rows), like every id on
    // this roster. No
    // seated fighter falls back to a generic repertoire. `SMASH_FIGHTER_KIT`
    // is still live as an ability grant (`lib.rs`).
    "npc_carl_stargan",
    "special_patent_clerk",
    // The deliberately simple SVG-rigged humanoid reference fighter. Unlike the
    // stand-ins below, this is a real character owned by Ambition content.
    "pointed_polygon",
    // The ranged one: a beast biped whose neutral special fires from a
    // head-mounted cannon. The grid's only body-authored projectile kit, so
    // it tests that a ranged kit survives the same seating, scoring and rules.
    "projectile_polygon",
    "pugnacious_polygon",
    // The four easter eggs, after the archetypes they borrow and before the
    // stand-ins. Each is a polygon archetype wearing a different person: same
    // skeleton, clips and frame data under its own move names. The two faceted
    // ones come first; the two hand-drawn ones follow their archetype (the
    // Performer after the Director, the Medic after the Officer). Neither
    // hand-drawn one has gameplay rules for her own specials yet.
    "director",
    "performer",
    "officer",
    "medic",
    // The stand-ins, last; see [`STAND_INS`].
    crate::SMASH_CHARACTER_ID,
    crate::SMASH_OPPONENT_ID,
];

/// Stand-ins: `(the copy, the character it stands in for)`.
///
/// This demo declares two rows on the robot lineage's sheets, copies of
/// characters Ambition's catalog already has. They keep the standalone app
/// from being a one-portrait grid, and [`SmashRoster::assemble`] drops each
/// once the real one resolves. This is the only sanctioned duplication;
/// everything else names the shared id (see [`SMASH_ROSTER`]).
///
/// Public so `ambition_app`'s guard ("did every buildable fighter reach the
/// grid") can tell a fighter dropped by a bug from one that stood down.
pub const STAND_INS: &[(&str, &str)] = &[
    (crate::SMASH_CHARACTER_ID, "player_robot_v3"),
    (crate::SMASH_OPPONENT_ID, "player_robot_v2"),
];

/// The characters a slot can choose between, in this composition.
///
/// [`SMASH_ROSTER`] filtered to the ids the assembled catalog carries, in
/// roster order. Resolved once at `Startup`: the cast is a composition fact.
///
/// The default is this demo's own fighters, not an empty list, so screen tests
/// without a catalog do not pass over an empty grid.
#[derive(bevy::prelude::Resource, Clone, Debug, PartialEq, Eq)]
pub struct SmashRoster(pub Vec<String>);

/// The ids this demo declares itself: the cast of a composition with no other
/// providers. Both are stand-ins; see [`STAND_INS`].
pub const OWN_FIGHTERS: &[&str] = &[crate::SMASH_CHARACTER_ID, crate::SMASH_OPPONENT_ID];

impl Default for SmashRoster {
    fn default() -> Self {
        Self(OWN_FIGHTERS.iter().map(|id| id.to_string()).collect())
    }
}

impl SmashRoster {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn get(&self, index: usize) -> Option<&str> {
        self.0.get(index).map(String::as_str)
    }

    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.0.iter().map(String::as_str)
    }

    /// [`SMASH_ROSTER`] ∩ what this composition can seat, in roster order.
    ///
    /// An id this host cannot seat is dropped, not kept as a hole: a pickable
    /// portrait the match then refuses is worse.
    pub fn assemble(
        registry: &ambition_platformer2d::characters::prepared::PreparedCharacterRegistry,
    ) -> Self {
        let present = |id: &str| registry.get(id).is_some();
        Self(
            SMASH_ROSTER
                .iter()
                .filter(|id| present(id))
                .filter(|id| {
                    // A stand-in steps aside once the character it stands in
                    // for is in the composition.
                    !STAND_INS
                        .iter()
                        .any(|(copy, real)| copy == *id && present(real))
                })
                .map(|id| id.to_string())
                .collect(),
        )
    }
}

/// Who is at one slot.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SlotOccupant {
    /// Not participating. It is not a fighter and is not waited for.
    #[default]
    Absent,
    /// A person, driving through one named local input source.
    ///
    /// `device` indexes the local source order: 0 is the primary source (the
    /// keyboard on a desk, pad one on a couch). No two slots may hold the same
    /// index.
    Controller { device: usize },
    /// The machine. Needs no device.
    Cpu,
}

impl SlotOccupant {
    pub fn participates(self) -> bool {
        !matches!(self, SlotOccupant::Absent)
    }

    pub fn is_cpu(self) -> bool {
        matches!(self, SlotOccupant::Cpu)
    }

    pub fn device(self) -> Option<usize> {
        match self {
            SlotOccupant::Controller { device } => Some(device),
            _ => None,
        }
    }
}

/// What a slot has chosen.
///
/// Not a `usize`, because one choice is not a character. `Fighter(i)` indexes
/// [`SmashRoster`]; `Random` indexes nothing and is resolved when the match
/// starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlotPick {
    /// This exact fighter, by roster index.
    Fighter(usize),
    /// Surprise me. Resolved when the match starts, not when the square is
    /// clicked.
    Random,
}

impl SlotPick {
    /// The roster index, if this is a committed fighter. `None` for random.
    pub fn fighter(self) -> Option<usize> {
        match self {
            Self::Fighter(index) => Some(index),
            Self::Random => None,
        }
    }

    pub fn is_random(self) -> bool {
        matches!(self, Self::Random)
    }
}

impl From<usize> for SlotPick {
    fn from(index: usize) -> Self {
        Self::Fighter(index)
    }
}

/// One deterministic stream for a match's random squares.
///
/// ADR 0023 forbids ambient RNG, so the caller seeds it. The mixer is the same
/// 64-bit LCG the boss patterns use, kept local.
struct RandomPick(u64);

impl RandomPick {
    fn seeded(seed: u64) -> Self {
        // A zero seed is a real input, and a zero state would make the LCG
        // constant, so mix it once.
        Self(seed ^ 0x9E37_79B9_7F4A_7C15)
    }

    /// A uniform index into `0..len`, or `None` for an empty grid.
    fn draw(&mut self, len: usize) -> Option<usize> {
        if len == 0 {
            return None;
        }
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        // The high bits: an LCG's low bits have short periods.
        Some(((self.0 >> 33) % len as u64) as usize)
    }
}

/// What one slot card says.
///
/// Human <-> CPU preserves the current pick because the chair is still active;
/// becoming [`SlotOccupant::Absent`] clears it. Reactivating an undecided card
/// starts on [`SlotPick::Random`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SlotCard {
    pub occupant: SlotOccupant,
    /// What this slot chose. `None` is the state the match waits on.
    pub pick: Option<SlotPick>,
}

/// The grid's cells: the fighters plus the random square.
///
/// The random square is last, so every fighter keeps its cell index.
impl SmashRoster {
    /// How many cells the grid draws — one per fighter, plus random.
    pub fn cell_count(&self) -> usize {
        self.len() + 1
    }

    /// What clicking cell `index` chooses.
    pub fn cell(&self, index: usize) -> Option<SlotPick> {
        match index {
            _ if index < self.len() => Some(SlotPick::Fighter(index)),
            _ if index == self.len() => Some(SlotPick::Random),
            _ => None,
        }
    }

    /// Which cell the random square is drawn in.
    pub fn random_cell(&self) -> usize {
        self.len()
    }
}

impl SlotCard {
    /// The character this slot has committed to. `None` while the slot is
    /// empty or has not picked. An absent slot answers `None`, so
    /// [`SmashSelect::ready`] can be a count.
    pub fn locked_pick(self) -> Option<SlotPick> {
        self.occupant.participates().then_some(self.pick).flatten()
    }
}

/// The whole screen's decision.
#[derive(bevy::prelude::Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SmashSelect {
    slots: [SlotCard; MAX_SMASH_SEATS],
}

impl SmashSelect {
    pub fn slot(&self, slot: usize) -> SlotCard {
        self.slots.get(slot).copied().unwrap_or_default()
    }

    pub fn slots(&self) -> impl Iterator<Item = (usize, SlotCard)> + '_ {
        self.slots.iter().copied().enumerate()
    }

    /// Assign one connected input source to one absent match slot.
    ///
    /// This is the single ownership primitive for human seats. It refuses to
    /// steal an occupied card or duplicate a source that already owns another
    /// card; callers choose *which* slot/source pair they are asking for.
    pub fn assign_controller(&mut self, slot: usize, source: usize) -> bool {
        if slot >= MAX_SMASH_SEATS
            || self.slots[slot].occupant != SlotOccupant::Absent
            || self.slot_driven_by(source).is_some()
        {
            return false;
        }
        self.set_occupant(slot, SlotOccupant::Controller { device: source });
        true
    }

    /// Return this source's slot, claiming the first absent card if needed.
    ///
    /// Character selection uses this path: an unseated connected participant may
    /// move a cursor immediately, and the first A press on a fighter both joins
    /// the lobby and makes that fighter the new slot's choice.
    pub fn slot_for_or_claim(&mut self, source: usize) -> Option<usize> {
        if let Some(slot) = self.slot_driven_by(source) {
            return Some(slot);
        }
        let slot = self
            .slots
            .iter()
            .position(|card| card.occupant == SlotOccupant::Absent)?;
        self.assign_controller(slot, source).then_some(slot)
    }

    /// Cycle a role button through Human / CPU / Absent.
    ///
    /// On an absent card, prefer the source that pressed the button when that
    /// source is unseated. Otherwise seat the first connected, unseated source;
    /// this is what lets player one enable a card for player two after a second
    /// controller connects. If every connected source is already seated, the
    /// card becomes CPU.
    ///
    /// `Absent` is a lifecycle boundary. Leaving the lobby removes the active
    /// fighter choice; re-entering starts on Random. Controller <-> CPU keeps the
    /// current pick because only the controller policy changed, not the chair.
    pub fn cycle_role(
        &mut self,
        slot: usize,
        requesting_source: usize,
        connected_sources: &[usize],
    ) {
        if slot >= MAX_SMASH_SEATS {
            return;
        }
        match self.slots[slot].occupant {
            SlotOccupant::Absent => {
                let next_human = connected_sources
                    .iter()
                    .copied()
                    .filter(|source| self.slot_driven_by(*source).is_none())
                    .min_by_key(|source| (*source != requesting_source, *source));
                if let Some(source) = next_human {
                    let _ = self.assign_controller(slot, source);
                } else {
                    self.set_occupant(slot, SlotOccupant::Cpu);
                }
            }
            SlotOccupant::Controller { .. } => self.set_occupant(slot, SlotOccupant::Cpu),
            SlotOccupant::Cpu => self.set_occupant(slot, SlotOccupant::Absent),
        }
    }

    /// The roster slot this local input source drives, if any.
    ///
    /// A match slot is not an input seat. The select screen keys cursors by
    /// input seat; using that index as the card fails when a CPU sits between
    /// two people:
    ///
    /// ```text
    /// card 0   Controller { device: 0 }
    /// card 1   Cpu
    /// card 2   Controller { device: 1 }   ← the second person
    /// ```
    ///
    /// Pad one reports on seat 1 and would drive card one, the CPU's. Ask this
    /// instead of indexing.
    ///
    /// `None` for an unseated source: a new participant may move a cursor
    /// before taking a card.
    pub fn slot_driven_by(&self, device: usize) -> Option<usize> {
        self.slots
            .iter()
            .position(|card| card.occupant.device() == Some(device))
    }

    /// Put a slot directly into a state, for a screen that has a reason to
    /// (the walkthrough, a test, a future "everyone in" button).
    ///
    /// `Absent` owns the reset rule too: there is no hidden remembered fighter
    /// behind an inactive card. Any active occupant entering an undecided card
    /// starts on Random.
    pub fn set_occupant(&mut self, slot: usize, occupant: SlotOccupant) {
        if slot >= MAX_SMASH_SEATS {
            return;
        }
        self.slots[slot].occupant = occupant;
        if occupant.participates() {
            if self.slots[slot].pick.is_none() {
                self.slots[slot].pick = Some(SlotPick::Random);
            }
        } else {
            self.slots[slot].pick = None;
        }
    }

    /// Set the fighter choice owned by one match slot.
    ///
    /// The index is not bounds-checked: only portraits the layout drew produce
    /// one. [`Self::roster`] drops a pick with no id, so a stale index costs a
    /// seat, not a fighter nobody chose.
    pub fn set_pick(&mut self, slot: usize, pick: impl Into<SlotPick>) {
        if slot < MAX_SMASH_SEATS {
            self.slots[slot].pick = Some(pick.into());
        }
    }

    /// The character a slot starts on when nothing has been dropped on it yet.
    ///
    /// Slot-indexed, so a solo player who adds one CPU gets Duelist A against
    /// Duelist B with no dragging.
    pub fn seed_pick(&mut self, slot: usize, fighters: &SmashRoster) {
        if slot < MAX_SMASH_SEATS && self.slots[slot].pick.is_none() && !fighters.is_empty() {
            self.slots[slot].pick = Some(SlotPick::Random);
        }
    }

    /// How many slots participate at all.
    pub fn participating(&self) -> usize {
        self.slots
            .iter()
            .filter(|card| card.occupant.participates())
            .count()
    }

    /// How many have a character.
    pub fn decided(&self) -> usize {
        self.slots
            .iter()
            .filter(|card| card.locked_pick().is_some())
            .count()
    }

    /// How many CPUs.
    pub fn cpus(&self) -> usize {
        self.slots
            .iter()
            .filter(|card| card.occupant.is_cpu())
            .count()
    }

    /// Slots a person has decided, as opposed to ones somebody added.
    pub fn humans_decided(&self) -> usize {
        self.slots
            .iter()
            .filter(|card| card.occupant.device().is_some() && card.pick.is_some())
            .count()
    }

    /// Can the battle start?
    ///
    /// Every participating slot has picked, and at least two participate.
    pub fn ready(&self) -> bool {
        self.decided() >= 2 && self.participating() == self.decided()
    }

    /// Why the match cannot start, in the words the screen puts under the
    /// cards.
    pub fn blocker(&self) -> Option<&'static str> {
        if self.participating() < 2 {
            Some("Two fighters needed — click a slot's button to add a controller or a CPU")
        } else if self.participating() != self.decided() {
            Some("Drag each slot's token onto a portrait")
        } else {
            None
        }
    }

    /// The match this screen decided.
    ///
    /// `None` until [`Self::ready`].
    pub fn roster(
        &self,
        fighters: &SmashRoster,
        policy: ambition_platformer2d::input::sources::InputAssignmentPolicy,
    ) -> Option<MatchParticipantRoster> {
        // Declares no floor and knows no repertoires: a kit-less character
        // seated through this wrapper reaches the stage unarmed. Production
        // uses `roster_seeded` with [`crate::smash_seating_melee`].
        self.roster_seeded(
            fighters,
            0,
            policy,
            &Default::default(),
            None,
            crate::STARTING_STOCKS,
        )
    }

    /// The match this screen decided, with the random squares resolved.
    ///
    /// `seed` is required (ADR 0023: no ambient RNG); this rolls a
    /// deterministic stream from it, so a test can ask for a specific draw.
    ///
    /// The policy is a parameter, as for [`source_name_under`]: what occupant
    /// index zero means (keyboard or first pad) is the policy's answer.
    pub fn roster_seeded(
        &self,
        fighters: &SmashRoster,
        seed: u64,
        policy: ambition_platformer2d::input::sources::InputAssignmentPolicy,
        // Ids whose character authors its own moves; those keep them, and
        // only the rest take the stage's generic kit. A set of ids, not the
        // registry, because the registry needs an `App` to populate and these
        // regressions should not. Empty means nobody authors anything.
        repertoires: &std::collections::BTreeSet<String>,
        // The kit this experience gives a seat whose character states none
        // ([`crate::smash_seating_melee`]). `None` means the engine default.
        // A value, not a rules resource: the seat's kit is settled here so
        // the body has one move authority.
        seating_melee: Option<ambition_platformer2d::character::MeleeActionSpec>,
        // Stated by the caller, not read here: both roads (`smash_roster` and
        // this one) must name the count. See `apply_smash_match_rules`.
        stocks: u32,
    ) -> Option<MatchParticipantRoster> {
        if !self.ready() {
            return None;
        }
        // One stream for the match, advanced once per random seat in slot
        // order. Two random seats can draw the same fighter; a mirror match
        // is a legal outcome.
        let mut rng = RandomPick::seeded(seed);
        let mut roster = MatchParticipantRoster::of(Vec::<String>::new());
        roster.participants = self
            .slots
            .iter()
            .enumerate()
            .filter_map(|(slot, card)| {
                // A pick with no id is dropped, not clamped or panicked: the
                // roster shrank under a decided screen. One fewer seat is
                // better than a fighter nobody chose.
                let character = match card.locked_pick()? {
                    SlotPick::Fighter(index) => fighters.get(index)?,
                    SlotPick::Random => fighters.get(rng.draw(fighters.len())?)?,
                };
                let authors_its_own = repertoires.contains(character);
                let seat = MatchParticipant::new(character)
                    // Driven by whoever the screen says is at the slot. An
                    // absent slot stays out; a CPU slot was asked for.
                    .driven_by(match card.occupant {
                        SlotOccupant::Controller { device } => crate::ControllerBinding::Human {
                            source: local_source_under(device, policy),
                        },
                        _ => crate::ControllerBinding::Cpu {
                            brain_profile: Some(crate::SMASH_DUELIST_BRAIN.to_string()),
                        },
                    })
                    // The kit this match gives seats with none, below.
                    .on_team(format!("seat {}", slot + 1));
                Some(match (authors_its_own, seating_melee.clone()) {
                    // Its own repertoire outranks any floor.
                    (true, _) => seat,
                    // The experience grants one: hand it over.
                    (false, Some(melee)) => {
                        let mut kit = ambition_platformer2d::character::ActionSet::default();
                        kit.melee = Some(melee);
                        seat.with_action_set(kit)
                    }
                    // The experience grants nothing and the character says
                    // nothing: leave the seat bare. Only fixtures reach this.
                    (false, None) => seat,
                })
            })
            .collect();
        crate::apply_smash_match_rules(&mut roster, stocks);
        // Publish under this experience, so another stage's teardown of "the
        // roster" does not remove this one.
        Some(roster.published_by(crate::SMASH_EXPERIENCE))
    }
}

/// How many local input sources this screen can offer, from the devices
/// plugged in.
///
/// Reads the live device order: players plug controllers in on this screen.
/// A rollback session freezes its seating so the match cannot change; the
/// seam is the moment the roster is published.
pub fn seats_offered(devices: &ambition_platformer2d::input::LocalDeviceOrder) -> usize {
    seats_offered_under(
        devices,
        ambition_platformer2d::input::sources::InputAssignmentPolicy::UnifiedPrimary,
    )
}

/// How many sources present can claim a slot, under a stated policy.
///
/// Under [`InputAssignmentPolicy::JoinToClaim`] the keyboard is a source like
/// any other and brings its own slot: keyboard + one pad is two players.
/// (`LocalDeviceOrder` holds only gamepads, so the keyboard is added here.)
/// [`InputAssignmentPolicy::UnifiedPrimary`] offers one source per pad.
pub fn seats_offered_under(
    devices: &ambition_platformer2d::input::LocalDeviceOrder,
    policy: ambition_platformer2d::input::sources::InputAssignmentPolicy,
) -> usize {
    let pads = devices.devices().len();
    let seats = match policy {
        ambition_platformer2d::input::sources::InputAssignmentPolicy::UnifiedPrimary => pads,
        // The keyboard is player one and each pad brings its own slot.
        _ => pads + 1,
    };
    seats.clamp(1, MAX_SMASH_SEATS)
}

/// Which input device a slot's person holds, as text (for debugging a couch
/// match).
///
/// Derived from the same authorities that chose the index
/// ([`seats_offered_under`] and the policy), not a second table. The keyboard
/// is device zero only under the multi-source policies; `UnifiedPrimary` has
/// no keyboard seat. That is why the policy is a parameter.
pub fn source_name_under(
    device: usize,
    devices: &ambition_platformer2d::input::LocalDeviceOrder,
    policy: ambition_platformer2d::input::sources::InputAssignmentPolicy,
) -> String {
    let pads = devices.devices().len();
    match local_source_under(device, policy) {
        ambition_platformer2d::actor::LocalInputSource::Keyboard => "KEYBOARD".to_string(),
        // A slot for an unplugged pad still names that pad; that is more
        // useful for debugging than hiding the gap.
        ambition_platformer2d::actor::LocalInputSource::Pad(pad) if (pad as usize) < pads => {
            format!("PAD {}", pad + 1)
        }
        ambition_platformer2d::actor::LocalInputSource::Pad(pad) => {
            format!("PAD {} (not connected)", pad + 1)
        }
    }
}

/// Which source a slot's occupant number names, under a stated policy. The
/// label and the roster must use this same mapping.
///
/// The keyboard is device zero only under the multi-source policies;
/// `UnifiedPrimary` has no keyboard seat.
pub fn local_source_under(
    device: usize,
    policy: ambition_platformer2d::input::sources::InputAssignmentPolicy,
) -> ambition_platformer2d::actor::LocalInputSource {
    use ambition_platformer2d::actor::LocalInputSource;
    match policy {
        // One seat per pad, no keyboard seat: the index is the pad.
        ambition_platformer2d::input::sources::InputAssignmentPolicy::UnifiedPrimary => {
            LocalInputSource::Pad(device as u8)
        }
        // The keyboard is player one and each pad brings its own slot.
        _ if device == 0 => LocalInputSource::Keyboard,
        _ => LocalInputSource::Pad((device - 1) as u8),
    }
}

#[cfg(test)]
mod tests;

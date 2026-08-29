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

/// One screen, four slots — the same ceiling the versus stage carries and the
/// same one `SlotControls` holds.
pub const MAX_SMASH_SEATS: usize = 4;

/// Ordered fighter IDs requested by the select grid.
///
/// Layout derives from this list's length. [`SmashRoster::assemble`] drops IDs
/// unavailable in the current composition while preserving order. Cross-game
/// fighters are shared by ID; do not declare duplicate character copies here.
pub const SMASH_ROSTER: &[&str] = &[
    // Just robot v3 is fine."* Chasing that found the reason it had a second name at all: the demo
    // declared its OWN row on `player_robot_v3_spritesheet.png` while the content catalog already
    // declared one, so "Duelist A" was a copy of a character that exists — the same mistake as
    // `smash_mary_o`, and it had survived only because the display names happened to differ.
    "player_robot_v3",
    // This demo's own, on a sheet nobody else claims.
    crate::SMASH_GEORGE_BOOUL,
    // Nothing goes red either way: both ids are real characters with identical kits, so every test
    // passes and only the sheet changes.  a stale working copy is a REVERT WITH NO DIFF TO REVIEW.
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
    // neither authors a repertoire yet, so both take the generic fighter floor from
    // `smash_fighter_kit()` — which is scaffolding whose adopter count is supposed to be FALLING
    // (redirect P6/§8). Seating them raises it from three to five.
    "npc_carl_stargan",
    "special_patent_clerk",
    // The deliberately simple SVG-rigged humanoid reference fighter. Unlike the
    // stand-ins below, this is a real character owned by Ambition content.
    "pointed_polygon",
    // THE RANGED ONE, and the trio's reason for existing: a non-humanoid beast
    // biped whose neutral special fires from a head-mounted cannon. It is the
    // only fighter on this grid whose combat distinction is a body-authored
    // PROJECTILE, so it is also the grid's only test that a ranged kit survives
    // the same seating, scoring and match rules a melee one does.
    "projectile_polygon",
    "pugnacious_polygon",
    // THE FOUR EASTER EGGS, and they sit HERE — after the archetypes they
    // borrow, before the stand-ins. Each is a polygon archetype wearing a
    // different person: same skeleton, same clips, same frame data under its
    // own move names. On the grid because a hidden fighter is FOUND, and
    // nothing in the game depends on any of them being picked.
    //
    // ⚠ The two faceted ones came first; the two HAND-DRAWN ones follow them,
    // paired with the archetype each borrows — the Performer after the Author on the
    // sword side, the Medic after the Officer on the brawler side. Neither of
    // the two has gameplay rules for her own specials yet: those exist as clips
    // and hit volumes in the sprite repository and as nothing here.
    "author",
    "performer",
    "officer",
    "medic",
    // THE STAND-INS, and they are LAST for a reason. See [`STAND_INS`].
    crate::SMASH_CHARACTER_ID,
    crate::SMASH_OPPONENT_ID,
];

/// Stand-ins: `(the copy, the character it stands in for)`.
///
/// This demo declares two rows on the robot lineage's sheets — copies of
/// characters Ambition's catalog already has. They stay selectable so the
/// STANDALONE app is not a one-portrait grid, and [`SmashRoster::assemble`]
/// drops each the moment the real one resolves, so a host never shows two
/// robots side by side with one of them wearing a made-up name.
///
/// this is the ONLY sanctioned duplication, and it exists because a demo that
/// composes nothing else still has to have a cast — not because copies are
/// acceptable. Everything else names the shared id; see [`SMASH_ROSTER`].
const STAND_INS: &[(&str, &str)] = &[
    (crate::SMASH_CHARACTER_ID, "player_robot_v3"),
    (crate::SMASH_OPPONENT_ID, "player_robot_v2"),
];

/// The characters a slot can choose between, in this composition.
///
/// [`SMASH_ROSTER`] filtered to the ids the assembled catalog actually carries,
/// in the order it names them. Resolved once at `Startup`, because which cast is
/// present is a fact about the COMPOSITION and a multi-game host is what
/// assembles one.
///
/// the default is this demo's own fighters, not an empty list. A fixture
/// with no catalog is testing the SCREEN, and a roster that collapsed to nothing
/// there would make every one of those tests pass over an empty grid.
#[derive(bevy::prelude::Resource, Clone, Debug, PartialEq, Eq)]
pub struct SmashRoster(pub Vec<String>);

/// The ids this demo declares itself, which is what a composition with no other
/// providers can offer.
///
/// both are STAND-INS — see [`STAND_INS`]. The standalone demo needs a cast;
/// a host carrying the real robot lineage gets that instead and never sees both.
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

    /// [`SMASH_ROSTER`] ∩ what this composition can SEAT, in roster order.
    ///
    /// an id this host cannot seat is DROPPED rather than kept as a hole:
    /// a grid cell for a character that cannot be built is a portrait a player
    /// can pick and a seat the match then refuses.
    ///
    /// Nobody knew, because the one configuration anybody tested was the one the permissive
    /// path served.
    ///
    /// Both halves are needed and neither implies the other.
    pub fn assemble(
        registry: &ambition_platformer2d::characters::prepared::PreparedCharacterRegistry,
    ) -> Self {
        let present = |id: &str| registry.get(id).is_some();
        Self(
            SMASH_ROSTER
                .iter()
                .filter(|id| present(id))
                .filter(|id| {
                    // A stand-in steps aside as soon as the character it stands
                    // in for is in the composition.
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
    /// Not participating. A slot that never fills is not a fighter and is not
    /// waited for.
    #[default]
    Absent,
    /// A person, driving through one named local input source.
    ///
    /// `device` indexes the local source order — 0 is the primary source (the
    /// keyboard on a desk, pad one on a couch). No two slots may hold the same
    /// index; see the module doc.
    Controller { device: usize },
    /// The machine. Needs no device, which is the entire point of it.
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
/// not a `usize`, because one of the choices is not a character. The grid
/// offers a RANDOM cell, and spelling that as a reserved index
/// into the fighter list would put arithmetic between "what somebody clicked"
/// and "who they are playing" — the shape this file already refuses for the
/// occupant. `Fighter(i)` indexes [`SmashRoster`]; `Random` indexes nothing and
/// is not resolved until the match starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlotPick {
    /// This exact fighter, by roster index.
    Fighter(usize),
    /// Surprise me. The character is chosen when the match starts, not when
    /// the square is clicked — so a person who takes random and then waits is
    /// not sitting there already knowing.
    Random,
}

impl SlotPick {
    /// The roster index, if this is a committed fighter. `None` for random —
    /// which is the whole point: there is no index yet.
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
/// ADR 0023 forbids ambient RNG, so this is a value seeded by its caller rather
/// than anything reaching for the clock. The mixer is the same 64-bit LCG the
/// boss patterns roll on, kept local because a screen drawing one number per
/// seat has no business owning a shared stream.
struct RandomPick(u64);

impl RandomPick {
    fn seeded(seed: u64) -> Self {
        // A zero seed is a real input (a test asking for the same draw twice),
        // and a zero state would make the LCG constant. Mix it once so every
        // seed — including zero — starts somewhere.
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
        // The HIGH bits, because an LCG's low bits have short periods — the
        // classic way to make "random" alternate between two fighters.
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

/// The grid's cells, which are the fighters PLUS the random square.
///
/// The random square is LAST, deliberately: every fighter keeps the cell index
/// it already had, so a screen, a walkthrough or a test that names a portrait by
/// position is not silently re-pointed at its neighbour by this feature.
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
    /// The character this slot has COMMITTED to. `None` while the slot is empty
    /// or has not picked — the two states the match waits on.
    ///
    /// An absent slot answers `None`, which is what makes
    /// [`SmashSelect::ready`] safe to write as a count.
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

    /// The roster slot this local input SOURCE drives, if any.
    ///
    /// a match slot is not an input seat, and this is the translation
    /// nobody wrote. The select screen keys its cursors by input seat —
    /// correctly, a hand belongs to a person — and then used that same index as
    /// the card to write a pick into. That holds only while the roster is dense
    /// and in source order; a CPU hole deliberately breaks both:
    ///
    /// ```text
    /// card 0   Controller { device: 0 }
    /// card 1   Cpu
    /// card 2   Controller { device: 1 }   ← the second person
    /// ```
    ///
    /// Pad one reports on seat 1 and would drive CARD ONE, which is the CPU's,
    /// and card two — its own — would be unreachable. Ask this instead of
    /// indexing, and the arithmetic that cannot survive a hole in the roster
    /// stops existing.
    ///
    /// `None` for a source nobody has seated. That is a real state, not a
    /// fault: a newly connected participant may move a cursor before selecting
    /// a fighter or explicitly taking a card, and it must not get somebody else's.
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
    /// the index is not bounds-checked here, and the reason is that the only
    /// thing that produces one is a portrait the LAYOUT drew — which the layout
    /// only draws for a fighter in the roster. [`Self::roster`] drops a pick
    /// with no id, so an index that somehow outlived its roster costs a seat
    /// rather than a fighter nobody chose.
    pub fn set_pick(&mut self, slot: usize, pick: impl Into<SlotPick>) {
        if slot < MAX_SMASH_SEATS {
            self.slots[slot].pick = Some(pick.into());
        }
    }

    /// The character a slot starts on when nothing has been dropped on it yet.
    ///
    /// Slot-indexed rather than constant, so a solo player who adds one CPU gets
    /// Duelist A against Duelist B — the fight this demo is about — with no
    /// dragging at all.
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

    /// Slots a PERSON has decided, as opposed to ones somebody added.
    pub fn humans_decided(&self) -> usize {
        self.slots
            .iter()
            .filter(|card| card.occupant.device().is_some() && card.pick.is_some())
            .count()
    }

    /// Can the battle start?
    ///
    /// Every participating slot has picked, and at least two participate.
    ///
    /// Its stated reason ("the second CPU somebody adds starts a match they are not in") had
    /// already expired: the screen waits for START to be clicked, so nothing launches on its
    /// own.
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
        // DECLARES NO FLOOR and knows no repertoires — this is the
        // convenience wrapper, so a kit-less character seated through it reaches
        // the stage unarmed. That is the honest answer for a caller that has not
        // said what its experience grants; production goes through
        // `roster_seeded` with [`crate::smash_seating_melee`].
        self.roster_seeded(fighters, 0, policy, &Default::default(), None)
    }

    /// The match this screen decided, with the random squares resolved.
    ///
    /// `seed` is required, not ambient (ADR 0023: no ambient RNG). The
    /// caller supplies something that varies per match; this rolls a
    /// deterministic stream off it, the same shape the boss patterns use. A
    /// seeded stream is also what lets a test ask for a specific draw instead of
    /// asserting "some fighter".
    ///
    /// the POLICY is a parameter for the same reason
    /// [`source_name_under`] takes one: a slot's occupant number is an index
    /// into the sources this screen offered, and what index zero MEANS —
    /// the keyboard, or the first pad — is the policy's answer. Turning that
    /// index into a roster binding without it would encode one policy's
    /// arithmetic into the match.
    pub fn roster_seeded(
        &self,
        fighters: &SmashRoster,
        seed: u64,
        policy: ambition_platformer2d::input::sources::InputAssignmentPolicy,
        // WHO ALREADY HAS A REPERTOIRE, by id. See the kit block below: a
        // seat whose character authors its own moves keeps them, and only the
        // ones that authored nothing take this stage's generic kit.
        //
        // a set of ids rather than the registry, deliberately. The
        // registry can only be populated through the preparation barrier, which
        // needs an `App` — so taking it here would mean this screen's regressions
        // could not state the case they are about without standing a whole app
        // up. The caller answers the question; this decides what to do about it.
        //
        // Empty = nobody authors anything, which is what every seat got before.
        repertoires: &std::collections::BTreeSet<String>,
        // ⭐ THE ADAPTATION THIS EXPERIENCE APPLIES to a seat whose character
        // states no kit — [`crate::smash_seating_melee`]. `None` means the
        // experience grants nothing, and a kit-less seat gets whatever the
        // engine's own default is wherever it is built.
        //
        // ⛔ Passed as a VALUE rather than read off a rules resource: this is a
        // roster-preparation policy, and the seat's kit is settled here so the
        // body reaches simulation with ONE move authority.
        seating_melee: Option<ambition_platformer2d::character::MeleeActionSpec>,
    ) -> Option<MatchParticipantRoster> {
        if !self.ready() {
            return None;
        }
        // One stream for the whole match, advanced once per random seat in slot
        // order — so two random seats draw independently, and CAN draw the same
        // fighter. A mirror match is a legal outcome of two people both asking
        // to be surprised; de-duplicating would be this screen quietly deciding
        // that it is not.
        let mut rng = RandomPick::seeded(seed);
        let mut roster = MatchParticipantRoster::of(Vec::<String>::new());
        roster.participants = self
            .slots
            .iter()
            .enumerate()
            .filter_map(|(slot, card)| {
                // a pick with no id is DROPPED rather than clamped or
                // panicked. It means the roster shrank under a decided screen —
                // impossible today and exactly the kind of thing a hosted
                // composition could arrange — and seating a fighter nobody
                // chose is worse than seating one fewer.
                let character = match card.locked_pick()? {
                    SlotPick::Fighter(index) => fighters.get(index)?,
                    SlotPick::Random => fighters.get(rng.draw(fighters.len())?)?,
                };
                let authors_its_own = repertoires.contains(character);
                let seat = MatchParticipant::new(character)
                    // A slot is driven by whoever the SCREEN says is at it.
                    // Nothing is filled in on anybody's behalf: an absent
                    // slot stays out of the match, and a CPU slot is one
                    // somebody asked for.
                    .driven_by(match card.occupant {
                        SlotOccupant::Controller { device } => crate::ControllerBinding::Human {
                            source: local_source_under(device, policy),
                        },
                        _ => crate::ControllerBinding::Cpu {
                            brain_profile: Some(crate::SMASH_DUELIST_BRAIN.to_string()),
                        },
                    })
                    // THE KIT THIS MATCH GIVES THE ONES WITH NONE.
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
                    // the experience grants nothing AND this character says
                    // nothing. Leaving the seat bare is the honest outcome, and
                    // it is reachable only from a fixture: the shipped smash
                    // experience always grants one.
                    (false, None) => seat,
                })
            })
            .collect();
        crate::apply_smash_match_rules(&mut roster);
        // WHOSE match this is. A host with a second stage in it removes "the
        // roster" on leaving its own route, and without an owner that teardown
        // reaches this one — which is how the stage stopped opening the day this
        // demo was listed on the title screen.
        Some(roster.published_by(crate::SMASH_EXPERIENCE))
    }
}

/// How many local input SOURCES this screen can hand out, from the devices
/// that are actually plugged in.
///
/// because a keyboard is player one on every other route in this game and a
/// select screen that offered zero sources when nobody had a gamepad would be a
/// demo you cannot start.
///
/// this reads the live device order rather than a frozen topology, and that is
/// correct HERE and would be wrong one route later. A select screen is exactly
/// where somebody plugs a controller in — that is what the screen is for — so it
/// must follow discovery. A rollback session freezes its seating precisely so the
/// MATCH cannot; the two answers are different on purpose, and the seam between
/// them is the moment the roster is published.
pub fn seats_offered(devices: &ambition_platformer2d::input::LocalDeviceOrder) -> usize {
    seats_offered_under(
        devices,
        ambition_platformer2d::input::sources::InputAssignmentPolicy::UnifiedPrimary,
    )
}

/// How many sources present can claim a slot, under a stated policy.
///
/// A gamepad joins a second participant."* With one keyboard and one pad it offers ONE source, so
/// both drive player one and the pad player has nowhere to sit. The keyboard was never a row in
/// `LocalDeviceOrder` — it holds gamepad entities — so it could not be counted, only assumed.
///
/// Under [`InputAssignmentPolicy::JoinToClaim`] the keyboard is a SOURCE like any
/// other and brings its own slot: keyboard + one pad is two players, which is
/// the whole couch flow.
///
/// [`InputAssignmentPolicy::UnifiedPrimary`] keeps the old arithmetic exactly.
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

/// WHICH INPUT DEVICE a slot's person is holding, in words.
///
/// which input device, so idk if that is the problem or not"* — asked while
/// debugging a couch match, and answered with text rather than a glyph because
/// *"text saying which input device it is is fine for the prototype. gives more
/// info for debugging."*
///
/// derived from the SAME two authorities that decided the index, not from
/// a second table: [`seats_offered_under`] turns `LocalDeviceOrder` + the policy
/// into how many sources exist, and this turns one of those indices back into
/// the source it names. A separate mapping would be a second answer to "what is
/// device 1" and would drift the first time the policy changed — which is
/// exactly the shape the roster/topology pair has already been bitten by.
///
/// the keyboard is device ZERO only under the multi-source policies;
/// `UnifiedPrimary` offers one seat per pad and no keyboard seat, so the same
/// index means a different thing. That is why the policy is a parameter.
pub fn source_name_under(
    device: usize,
    devices: &ambition_platformer2d::input::LocalDeviceOrder,
    policy: ambition_platformer2d::input::sources::InputAssignmentPolicy,
) -> String {
    let pads = devices.devices().len();
    match local_source_under(device, policy) {
        ambition_platformer2d::actor::LocalInputSource::Keyboard => "KEYBOARD".to_string(),
        // A slot offered for a pad that has since been unplugged still names the
        // pad it is waiting for. Saying "PAD 2" for a seat with nothing in it is
        // more useful while debugging than hiding the gap.
        ambition_platformer2d::actor::LocalInputSource::Pad(pad) if (pad as usize) < pads => {
            format!("PAD {}", pad + 1)
        }
        ambition_platformer2d::actor::LocalInputSource::Pad(pad) => {
            format!("PAD {} (not connected)", pad + 1)
        }
    }
}

/// WHICH SOURCE a slot's occupant number names, under a stated policy.
///
/// That is exactly the pair that drifts: the label said `KEYBOARD` while the roster said pad
/// zero, and the match then bound the keyboard player to a controller.
///
/// the keyboard is device ZERO only under the multi-source policies;
/// `UnifiedPrimary` offers one seat per pad and no keyboard seat, so the same
/// index means a different thing.
pub fn local_source_under(
    device: usize,
    policy: ambition_platformer2d::input::sources::InputAssignmentPolicy,
) -> ambition_platformer2d::actor::LocalInputSource {
    use ambition_platformer2d::actor::LocalInputSource;
    match policy {
        // One seat per pad, no keyboard seat: the index IS the pad.
        ambition_platformer2d::input::sources::InputAssignmentPolicy::UnifiedPrimary => {
            LocalInputSource::Pad(device as u8)
        }
        // The keyboard is player one and each pad brings its own slot.
        _ if device == 0 => LocalInputSource::Keyboard,
        _ => LocalInputSource::Pad((device - 1) as u8),
    }
}

/// What every fighter on this stage swings.
///
/// The demo's `duelist` preset, in Rust rather than by catalog reference,
/// because the roster hands the kit to characters whose OWN rows this demo does
/// not own and must not edit. Numbers match `SMASH_CATALOG_RON`'s `duelist`
/// action set — a real swipe, because the whole point of the stage is that a hit
/// LAUNCHES and a fighter with no melee cannot knock anybody off anything.
// This screen reads the declaration instead of carrying one.

#[cfg(test)]
mod tests;

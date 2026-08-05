//! **Character select: where a match is DECIDED before it is seated.**
//!
//! Jon, 2026-07-31: *"the smash demo must start with the character select screen
//! and then start the battle when up to 4 players are locked in."*
//!
//! Jon, 2026-08-05, redrawing it: *"a grid of portraits for each of the
//! selectable characters on the top 65% of the screen. The bottom 35% of the
//! screen should be 4 participant slot cards… Each participant slot will have a
//! button to toggle it between a controller player (which must have a
//! corresponding attached controller), a CPU player, or not participating."*
//!
//! ## Why this is a value and not a pile of widgets
//!
//! A menu picks one thing for one person. This decides, per slot, two
//! independent facts — **who is there** and **what they chose** — and the match
//! cannot begin until every slot that is there has answered the second. That is
//! a small state machine per slot and a quorum rule over the set, which is
//! exactly the thing that goes wrong when it is written as UI callbacks:
//! "everyone is ready" gets computed from whichever widget last fired.
//!
//! So the whole decision is a pure value with no Bevy in it beyond `Resource`.
//! [`crate::select_screen`] draws it and drives it; this decides.
//!
//! ## The rule, stated once
//!
//! **The battle starts when every PARTICIPATING slot has picked a character, at
//! least two slots participate, and at least one of them is a person.** Each
//! part is a different bug, and all three were found the hard way:
//!
//! * without "every participating slot has picked", a player who joined and is
//!   still browsing gets dropped into a fight as whoever the cursor was over;
//! * without "at least two", the first slot to decide starts a match against
//!   nobody — and a stocks match with one side never ends, because
//!   `last_side_standing` correctly refuses to call a sole survivor a winner;
//! * without "at least one person", the second CPU somebody adds starts a match
//!   they are not in, before they have chosen a character.
//!
//! ## Who is at a slot
//!
//! A slot is [`SlotOccupant::Absent`], a [`SlotOccupant::Controller`], or a
//! [`SlotOccupant::Cpu`] — and the third exists because the screen used to offer
//! one seat per PAD, so a player alone at a keyboard could never reach the two
//! decided slots a match needs. (Jon, 2026-07-31: *"there seems to be no way to
//! have player 2 be a CPU player."*)
//!
//! ⚠ **`Controller` carries WHICH device, and that is the whole couch bug in one
//! field.** Two slots both meaning "a person" is ambiguous the moment there are
//! two sources in the room; a slot that names its device cannot silently share
//! one with its neighbour. [`SmashSelect::cycle_occupant`] refuses to make a slot
//! a controller when no unclaimed source is left, which is Jon's *"which must
//! have a corresponding attached controller"* enforced in the value rather than
//! checked in the widget.

use crate::{MatchParticipant, MatchParticipantRoster, STARTING_STOCKS};

/// One screen, four slots — the same ceiling the versus stage carries and the
/// same one `SlotControls` holds.
pub const MAX_SMASH_SEATS: usize = 4;

/// **The fighters this demo DECLARES**, by catalog id.
///
/// Four rows in `SMASH_CATALOG_RON`, and four is the floor rather than the
/// roster: [`SmashRoster`] adds every character the composition around this demo
/// has tagged for the crossover.
pub const OWN_FIGHTERS: &[&str] = &[
    crate::SMASH_CHARACTER_ID,
    crate::SMASH_OPPONENT_ID,
    crate::SMASH_MARY_O_FIRE,
    crate::SMASH_GEORGE_BOOUL,
];

/// **The tag a character wears to say it can fight here.**
///
/// ⭐ opt-IN, from the character's own catalog row. A crossover stage cannot
/// have a hard-coded roster: the cast it can offer is a fact about the
/// COMPOSITION, and the composition is what a multi-game host assembles.
pub const SMASH_TAG: &str = "smash";

/// **The characters a slot can choose between, in this composition.**
///
/// ⛔ **this used to be a `const` list and the engine refused it, correctly.**
/// The first draft declared its own `smash_mary_o`, `smash_sanic` and
/// `smash_solid_snake` rows on the sheets those characters already use, and the
/// assembled catalog rejected all four:
///
/// ```text
/// characters 'mary_o' and 'smash_mary_o' share display_name 'Mary-O'
/// characters 'sanic' and 'smash_sanic' share display_name 'Sanic'
/// characters 'smash_solid_snake' and 'solid_snake' share display_name …
/// characters 'smash_super_sanic' and 'super_sanic' share display_name …
/// ```
///
/// Which is the right answer to the wrong question. **A crossover stage does not
/// need copies of the cast; it needs the cast.** Mary-O, Sanic and Solid Snake
/// already exist — declared by the demos they belong to, with their own sheets,
/// their own names and their own dialogue — and the display-name rule exists
/// precisely to stop a second declaration of a character that is already there.
/// (Jon's rule, recorded: characters are shared BY ID; display-name uniqueness
/// is how that is enforced.)
///
/// So the roster is resolved once, at startup, from the assembled catalog: the
/// demo's own fighters, then every character whose row carries [`SMASH_TAG`].
///
/// ⚠ **the default is the demo's own four**, not an empty list. A fixture with
/// no catalog is testing the SCREEN, and a roster that collapsed to nothing
/// there would make every one of those tests pass over an empty grid.
#[derive(bevy::prelude::Resource, Clone, Debug, PartialEq, Eq)]
pub struct SmashRoster(pub Vec<String>);

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

    /// **Everything this composition can offer**, own fighters first.
    ///
    /// ⚠ own-first and de-duplicated: a demo that tagged one of its OWN rows
    /// would otherwise appear twice, and two grid cells for one fighter are two
    /// cells a token can be dropped on with one of them wrong.
    pub fn assemble(catalog: &ambition_platformer2d::character::CharacterCatalog) -> Self {
        let mut ids: Vec<String> = OWN_FIGHTERS.iter().map(|id| id.to_string()).collect();
        // `CharacterCatalog::iter` is a `BTreeMap` walk, so the crossover half of
        // the grid is in id order on every machine and every run. An unordered
        // roster would put a different fighter under the cursor's start position
        // depending on hash seed.
        for (id, entry) in catalog.iter() {
            if entry.tags.iter().any(|tag| tag == SMASH_TAG) && !ids.iter().any(|kept| kept == id) {
                ids.push(id.clone());
            }
        }
        Self(ids)
    }
}

/// **Who is at one slot.**
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

/// **What one slot card says.**
///
/// The pick SURVIVES the occupant changing, on purpose: toggling a slot from
/// controller to CPU and back is how a player hands their character to the
/// machine, and clearing the portrait on the way through would make that a
/// re-pick every time.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SlotCard {
    pub occupant: SlotOccupant,
    /// Indexes [`SELECTABLE`]. `None` is the state the match waits on.
    pub pick: Option<usize>,
}

impl SlotCard {
    /// The character this slot has COMMITTED to. `None` while the slot is empty
    /// or has not picked — the two states the match waits on.
    ///
    /// ⚠ an absent slot with a remembered pick answers `None`, which is what
    /// makes [`SmashSelect::ready`] safe to write as a count.
    pub fn locked_character(self) -> Option<usize> {
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

    /// **Toggle one slot between the three things it can be.**
    ///
    /// `Absent → Controller → Cpu → Absent`, which is Jon's button. The order is
    /// deliberate: the first press of an empty card seats the PERSON pressing
    /// it, because somebody reaching for an empty chair almost always means
    /// themselves, and a second press hands that chair to the machine.
    ///
    /// `sources` is how many local input sources exist — see
    /// [`seats_offered_under`]. When none is free the controller rung is SKIPPED
    /// rather than refused with a beep: a fourth card on a two-pad couch goes
    /// straight from empty to CPU, which is the only thing it could honestly
    /// become. That is Jon's *"which must have a corresponding attached
    /// controller"*, and it lives here because a widget that checked it would be
    /// checking a rule it does not own.
    pub fn cycle_occupant(&mut self, slot: usize, sources: usize) {
        if slot >= MAX_SMASH_SEATS {
            return;
        }
        let next = match self.slots[slot].occupant {
            SlotOccupant::Absent => match self.first_free_device(slot, sources) {
                Some(device) => SlotOccupant::Controller { device },
                None => SlotOccupant::Cpu,
            },
            SlotOccupant::Controller { .. } => SlotOccupant::Cpu,
            SlotOccupant::Cpu => SlotOccupant::Absent,
        };
        self.slots[slot].occupant = next;
    }

    /// Put a slot directly into a state, for a screen that has a reason to
    /// (the walkthrough, a test, a future "everyone in" button).
    pub fn set_occupant(&mut self, slot: usize, occupant: SlotOccupant) {
        if slot < MAX_SMASH_SEATS {
            self.slots[slot].occupant = occupant;
        }
    }

    /// **The lowest input source no other slot is holding.**
    ///
    /// ⛔ This is the couch-input trap in its smallest form. Two slots that both
    /// say "a person" and never say WHICH person is how one pad ends up driving
    /// two fighters — the defect this repo has now found five separate times,
    /// and every one of them was invisible with a single pad plugged in.
    pub fn first_free_device(&self, slot: usize, sources: usize) -> Option<usize> {
        (0..sources).find(|device| {
            !self
                .slots
                .iter()
                .enumerate()
                .any(|(other, card)| other != slot && card.occupant.device() == Some(*device))
        })
    }

    /// **Somebody dropped a token on a portrait.**
    ///
    /// ⚠ the index is not bounds-checked here, and the reason is that the only
    /// thing that produces one is a portrait the LAYOUT drew — which the layout
    /// only draws for a fighter in the roster. [`Self::roster`] drops a pick
    /// with no id, so an index that somehow outlived its roster costs a seat
    /// rather than a fighter nobody chose.
    pub fn set_pick(&mut self, slot: usize, character: usize) {
        if slot < MAX_SMASH_SEATS {
            self.slots[slot].pick = Some(character);
        }
    }

    /// The character a slot starts on when nothing has been dropped on it yet.
    ///
    /// Slot-indexed rather than constant, so a solo player who adds one CPU gets
    /// Duelist A against Duelist B — the fight this demo is about — with no
    /// dragging at all.
    pub fn seed_pick(&mut self, slot: usize, fighters: &SmashRoster) {
        if slot < MAX_SMASH_SEATS && self.slots[slot].pick.is_none() && !fighters.is_empty() {
            self.slots[slot].pick = Some(slot % fighters.len());
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
            .filter(|card| card.locked_character().is_some())
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

    /// **Can the battle start?**
    ///
    /// Every participating slot has picked, at least two participate, and at
    /// least one of them is a person. See the module doc for why each clause is
    /// load-bearing.
    pub fn ready(&self) -> bool {
        self.decided() >= 2 && self.participating() == self.decided() && self.humans_decided() >= 1
    }

    /// **Why the match cannot start**, in the words the screen puts under the
    /// cards.
    ///
    /// ⚠ it used to read "Two players needed" and stop there, which was true and
    /// useless: there was no press that produced a second player and the screen
    /// did not name one. A prompt that states a requirement without naming the
    /// action that satisfies it is a dead end with punctuation.
    pub fn blocker(&self) -> Option<&'static str> {
        if self.participating() < 2 {
            Some("Two fighters needed — click a slot's button to add a controller or a CPU")
        } else if self.participating() != self.decided() {
            Some("Drag each slot's token onto a portrait")
        } else if self.humans_decided() == 0 {
            Some("At least one slot must be a controller player")
        } else {
            None
        }
    }

    /// The match this screen decided.
    ///
    /// `None` until [`Self::ready`]. Building a roster from a half-decided
    /// screen is the failure this returns an `Option` to make impossible: a
    /// hovered portrait reads exactly like a choice.
    pub fn roster(&self, fighters: &SmashRoster) -> Option<MatchParticipantRoster> {
        if !self.ready() {
            return None;
        }
        let mut roster = MatchParticipantRoster::of(Vec::<String>::new());
        roster.participants = self
            .slots
            .iter()
            .enumerate()
            .filter_map(|(slot, card)| {
                // ⚠ a pick with no id is DROPPED rather than clamped or
                // panicked. It means the roster shrank under a decided screen —
                // impossible today and exactly the kind of thing a hosted
                // composition could arrange — and seating a fighter nobody
                // chose is worse than seating one fewer.
                let character = fighters.get(card.locked_character()?)?;
                Some(
                    MatchParticipant::new(character)
                        // A slot is driven by whoever the SCREEN says is at it.
                        // Nothing is filled in on anybody's behalf: an absent
                        // slot stays out of the match, and a CPU slot is one
                        // somebody asked for.
                        .driven_by(match card.occupant {
                            SlotOccupant::Controller { device } => {
                                crate::ControllerBinding::Human {
                                    device_slot: device as u8,
                                }
                            }
                            _ => crate::ControllerBinding::Cpu {
                                brain_profile: Some(crate::SMASH_DUELIST_BRAIN.to_string()),
                            },
                        })
                        .on_team(format!("seat {}", slot + 1)),
                )
            })
            .collect();
        roster.opens_suspended = true;
        roster.fighter_stocks = Some(STARTING_STOCKS);
        // **EVERY FIGHTER IN THIS MATCH HAS THE SAME VERBS.**
        //
        // ⛔ Measured 2026-08-01, both seats wearing the right duelist:
        //
        //     seat 0 (ADOPTED)     every ability true - fly, blink,
        //                          blink_through_hard_walls, glide, swim, shield
        //     seat 1 (SPAWNED)     move, jump, variable_jump, double_jump, attack
        //
        // Player one fought as the exploration protagonist and player two as a
        // duelist, on the same stage. The touch bezel advertised it (Blink / Fly
        // Toggle / Ranged / Bubble Shield) and was the only honest thing in the
        // picture - it reports what the CONTROLLED SUBJECT can do, and it was right.
        //
        // Seating already levels this and says so in its own comment, found the same
        // way on the VERSUS stage in July: "a SPAWNED seat's abilities come from
        // `AncillaryMovementBundle`; the ADOPTED primary player brought whatever the
        // session granted it". It is gated on the roster DECLARING a set, because
        // "what a fighter may do is a rule of the match" - and this demo declared
        // nothing, so the levelling never ran.
        //
        // ⚠ SPELLED OUT rather than a named set, and both named candidates were
        // tried and measured first. `basic()` has no double jump and no attack, so
        // it would REMOVE verbs both duelists already had. `sane_subset()` reads
        // like a fighter's kit in its first ten lines and is not one - measured, it
        // also grants fly, blink, precision_blink, wall climb and pogo, so declaring
        // it made the two seats agree that they could both FLY. This is the actual
        // floor of a platform fighter: run, jump, double jump, fast fall, dash,
        // attack. WHICH verbs is a product call and this is the one place to change
        // it; that the two seats agree is not.
        //
        // ⭐ it is also what makes a WIDE roster honest. Eight fighters that share
        // one kit are eight looks and one game; nobody is stronger because their
        // sheet came from a different demo. Per-character kits are the next
        // question, not this one.
        roster.fighter_abilities = Some(ambition_platformer2d::engine_core::AbilitySet {
            move_horizontal: true,
            jump: true,
            variable_jump: true,
            double_jump: true,
            fast_fall: true,
            dash: true,
            attack: true,
            ..ambition_platformer2d::engine_core::AbilitySet::NONE
        });
        // WHOSE match this is. A host with a second stage in it removes "the
        // roster" on leaving its own route, and without an owner that teardown
        // reaches this one — which is how the stage stopped opening the day this
        // demo was listed on the title screen.
        Some(roster.published_by(crate::SMASH_EXPERIENCE))
    }
}

/// **How many local input SOURCES this screen can hand out**, from the devices
/// that are actually plugged in.
///
/// Jon's *"up to 4 players"* is a CEILING, not a count. The floor is one,
/// because a keyboard is player one on every other route in this game and a
/// select screen that offered zero sources when nobody had a gamepad would be a
/// demo you cannot start.
///
/// ⚠ this reads the live device order rather than a frozen topology, and that is
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

/// **How many sources present can claim a slot, under a stated policy.**
///
/// ⛔ The pad-only count above is Jon's couch milestone 2 failing:
/// *"Keyboard joins one participant. A gamepad joins a second participant."*
/// With one keyboard and one pad it offers ONE source, so both drive player one
/// and the pad player has nowhere to sit. The keyboard was never a row in
/// `LocalDeviceOrder` — it holds gamepad entities — so it could not be counted,
/// only assumed.
///
/// Under [`InputAssignmentPolicy::JoinToClaim`] the keyboard is a SOURCE like any
/// other and brings its own slot: keyboard + one pad is two players, which is
/// the whole couch flow.
///
/// ⚠ [`InputAssignmentPolicy::UnifiedPrimary`] keeps the old arithmetic exactly.
/// Solo play drives one character with either hand and must not discover that
/// plugging a controller in created a second empty chair — Jon's milestone 8,
/// and the reason this takes a policy rather than just adding one.
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

#[cfg(test)]
mod tests;

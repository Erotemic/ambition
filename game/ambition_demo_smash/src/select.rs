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
//! **The battle starts when every PARTICIPATING slot has picked a character and
//! at least two slots participate.** Both were found the hard way:
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

#[cfg(test)]
use crate::STARTING_STOCKS;
use crate::{MatchParticipant, MatchParticipantRoster};

/// One screen, four slots — the same ceiling the versus stage carries and the
/// same one `SlotControls` holds.
pub const MAX_SMASH_SEATS: usize = 4;

/// **THE GRID. Edit this list.**
///
/// Jon, 2026-08-05: *"we will probably tweak which characters will be in the
/// game in the future, so make it easy to have the exact number of characters
/// configurable. We may go more than 8."* This is that list, and it is the only
/// place a fighter is named — the grid reads its column count from the length,
/// the layout balances the rows around it, and nothing else has an opinion about
/// how many there are.
///
/// ⚠ **it is a WISH LIST, not a guarantee.** Ids the composition around this
/// demo does not carry are dropped by [`SmashRoster::assemble`] — Mary-O, Sanic
/// and Solid Snake are declared by the demos they belong to, so the standalone
/// smash app offers only the fighters it declares itself, and the multi-game
/// host offers the whole crossover cast. Order is preserved.
///
/// ⛔ **do not add a fighter by declaring a COPY of it here.** The first draft
/// did exactly that — its own `smash_mary_o`, `smash_sanic`, `smash_solid_snake`
/// and `smash_super_sanic` on the sheets those characters already use — and the
/// assembled catalog refused all four:
///
/// ```text
/// characters 'mary_o' and 'smash_mary_o' share display_name 'Mary-O'
/// characters 'sanic' and 'smash_sanic' share display_name 'Sanic'
/// characters 'smash_solid_snake' and 'solid_snake' share display_name …
/// characters 'smash_super_sanic' and 'super_sanic' share display_name …
/// ```
///
/// Which is the right answer to the wrong question: a crossover stage does not
/// need copies of the cast, it needs the cast. Characters are shared BY ID, and
/// display-name uniqueness is how that rule is enforced.
///
/// ⚠ **no two entries are FORMS of one character** (Jon, 2026-08-05: *"I don't
/// want copies of different character forms"*). Fire Mary-O and Super Sanic were
/// on the first grid and are gone; a transformation is something that happens
/// during a match, not a second slot on the select screen.
pub const SMASH_ROSTER: &[&str] = &[
    // ⚠ **`player_robot_v3`, NOT this demo's `smash_duelist_a`.** Jon, 2026-08-05:
    // *"The robot v3 should not be named dualist A. Just robot v3 is fine."*
    // Chasing that found the reason it had a second name at all: the demo
    // declared its OWN row on `player_robot_v3_spritesheet.png` while the
    // content catalog already declared one, so "Duelist A" was a copy of a
    // character that exists — the same mistake as `smash_mary_o`, and it had
    // survived only because the display names happened to differ.
    "player_robot_v3",
    // This demo's own, on a sheet nobody else claims.
    crate::SMASH_GEORGE_BOOUL,
    // The other demos' protagonists — present only when a host composes them.
    "mary_o",
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
    // ⭐ **JON, 2026-08-11: add Stargan, the Patent Clerk and the PCA.** The PCA
    // was already here; these two are the addition. Both carry the standardized
    // full-fighter sprite vocabulary (123 and 133 rows), which is what made them
    // worth seating — but ROSTER MEMBERSHIP IS A CONTENT DECISION and Jon made
    // it. The art existing is not the argument; he is.
    //
    // ⚠ neither authors a repertoire yet, so both take the generic fighter floor
    // from `smash_fighter_kit()` — which is scaffolding whose adopter count is
    // supposed to be FALLING (redirect P6/§8). Seating them raises it from three
    // to five. That is a real cost and it is Jon's call to pay it; the fix is to
    // author them repertoires, never to broaden the floor.
    "npc_carl_stargan",
    "special_patent_clerk",
    // ⚠ **THE STAND-INS, and they are LAST for a reason.** See [`STAND_INS`].
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
/// ⛔ this is the ONLY sanctioned duplication, and it exists because a demo that
/// composes nothing else still has to have a cast — not because copies are
/// acceptable. Everything else names the shared id; see [`SMASH_ROSTER`].
const STAND_INS: &[(&str, &str)] = &[
    (crate::SMASH_CHARACTER_ID, "player_robot_v3"),
    (crate::SMASH_OPPONENT_ID, "player_robot_v2"),
];

/// **The characters a slot can choose between, in this composition.**
///
/// [`SMASH_ROSTER`] filtered to the ids the assembled catalog actually carries,
/// in the order it names them. Resolved once at `Startup`, because which cast is
/// present is a fact about the COMPOSITION and a multi-game host is what
/// assembles one.
///
/// ⚠ **the default is this demo's own fighters**, not an empty list. A fixture
/// with no catalog is testing the SCREEN, and a roster that collapsed to nothing
/// there would make every one of those tests pass over an empty grid.
#[derive(bevy::prelude::Resource, Clone, Debug, PartialEq, Eq)]
pub struct SmashRoster(pub Vec<String>);

/// The ids this demo declares itself, which is what a composition with no other
/// providers can offer.
///
/// ⚠ both are STAND-INS — see [`STAND_INS`]. The standalone demo needs a cast;
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

    /// **[`SMASH_ROSTER`] ∩ what this composition can SEAT**, in roster order.
    ///
    /// ⚠ an id this host cannot seat is DROPPED rather than kept as a hole:
    /// a grid cell for a character that cannot be built is a portrait a player
    /// can pick and a seat the match then refuses.
    ///
    /// ⛔ **the REGISTRY, not the catalog, and the difference is a whole class
    /// of bug.** A catalog row says what a character IS; `register_character` is
    /// what makes one BUILDABLE, and only the second is what a seat needs. This
    /// filtered on the catalog until 2026-08-07, and eight of the twelve
    /// portraits on the grid were rows nothing had registered — seatable as
    /// player one, where the adopted home body consulted the registry
    /// OPTIONALLY, and unbuildable in every other seat. Nobody knew, because the
    /// one configuration anybody tested was the one the permissive path served.
    ///
    /// ⭐ registering the missing cast fixed today's twelve; filtering here is
    /// what stops the NEXT catalog-only addition from putting an unpickable
    /// portrait on the screen. Both halves are needed and neither implies the
    /// other.
    pub fn assemble(
        registry: &ambition_platformer2d::actors::character_runtime::PreparedCharacterRegistry,
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
/// **What a slot has chosen.**
///
/// ⛔ **not a `usize`, because one of the choices is not a character.** The grid
/// offers a RANDOM cell (Jon, 2026-08-07), and spelling that as a reserved index
/// into the fighter list would put arithmetic between "what somebody clicked"
/// and "who they are playing" — the shape this file already refuses for the
/// occupant. `Fighter(i)` indexes [`SmashRoster`]; `Random` indexes nothing and
/// is not resolved until the match starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlotPick {
    /// This exact fighter, by roster index.
    Fighter(usize),
    /// **Surprise me.** The character is chosen when the match starts, not when
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

/// **One deterministic stream for a match's random squares.**
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SlotCard {
    pub occupant: SlotOccupant,
    /// What this slot chose. `None` is the state the match waits on.
    pub pick: Option<SlotPick>,
}

/// **The grid's cells, which are the fighters PLUS the random square.**
///
/// The random square is LAST, deliberately: every fighter keeps the cell index
/// it already had, so a screen, a walkthrough or a test that names a portrait by
/// position is not silently re-pointed at its neighbour by this feature.
impl SmashRoster {
    /// How many cells the grid draws — one per fighter, plus random.
    pub fn cell_count(&self) -> usize {
        self.len() + 1
    }

    /// What clicking cell `index` chooses. `None` past the end of the grid,
    /// matching `SelectLayout::portrait`: an index nobody drew is a bug to see,
    /// not a choice to invent.
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
    /// ⚠ an absent slot with a remembered pick answers `None`, which is what
    /// makes [`SmashSelect::ready`] safe to write as a count.
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
        // **A SLOT THAT JUST JOINED IS ON RANDOM.** (Jon, 2026-08-07: *"going
        // from 'Not Playing' to a player does not auto assign to random, and I
        // would like that to be the case."*)
        //
        // ⚠ **only when it has no pick**, which is what keeps the promise the
        // card already makes: a pick SURVIVES the occupant changing, so cycling
        // controller → CPU → absent → controller hands your fighter to the
        // machine and back rather than re-rolling it. Random is the state of a
        // slot nobody has chosen for, not a thing the button does to you.
        if next.participates() && self.slots[slot].pick.is_none() {
            self.slots[slot].pick = Some(SlotPick::Random);
        }
    }

    /// Put a slot directly into a state, for a screen that has a reason to
    /// (the walkthrough, a test, a future "everyone in" button).
    pub fn set_occupant(&mut self, slot: usize, occupant: SlotOccupant) {
        if slot < MAX_SMASH_SEATS {
            self.slots[slot].occupant = occupant;
            // The same rule the button follows, so a screen that seats somebody
            // directly does not produce a state the button cannot reach.
            if occupant.participates() && self.slots[slot].pick.is_none() {
                self.slots[slot].pick = Some(SlotPick::Random);
            }
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

    /// **Can the battle start?**
    ///
    /// Every participating slot has picked, and at least two participate.
    ///
    /// ⛔ **there used to be a third clause — `humans_decided() >= 1` — and it
    /// was an ENGINE LIMITATION wearing a product rationale.** Jon, 2026-08-06:
    /// *"it does not let me make a CPU vs CPU match, and it is very important
    /// that that is expressible and easy to do."*
    ///
    /// Its stated reason ("the second CPU somebody adds starts a match they are
    /// not in") had already expired: the screen waits for START to be clicked,
    /// so nothing launches on its own. What the clause was really holding up was
    /// that a match with nobody local had no answer for the session's home body —
    /// nothing adopted it, so it stood on the stage unclaimed. That is fixed
    /// where it belongs, in how a match builds its cast, and this is a product
    /// rule again: two fighters, everyone has chosen.
    pub fn ready(&self) -> bool {
        self.decided() >= 2 && self.participating() == self.decided()
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
        } else {
            None
        }
    }

    /// The match this screen decided.
    ///
    /// `None` until [`Self::ready`]. Building a roster from a half-decided
    /// screen is the failure this returns an `Option` to make impossible: a
    /// hovered portrait reads exactly like a choice.
    pub fn roster(
        &self,
        fighters: &SmashRoster,
        policy: ambition_platformer2d::input::sources::InputAssignmentPolicy,
    ) -> Option<MatchParticipantRoster> {
        // ⚠ **DECLARES NO FLOOR and knows no repertoires** — this is the
        // convenience wrapper, so a kit-less character seated through it reaches
        // the stage unarmed. That is the honest answer for a caller that has not
        // said what its experience grants; production goes through
        // `roster_seeded` with the stage's `DeclaredCombatRules::unarmed_melee`.
        self.roster_seeded(fighters, 0, policy, &Default::default(), None)
    }

    /// **The match this screen decided, with the random squares resolved.**
    ///
    /// ⭐ **this is where "when the match starts" happens** (Jon, 2026-08-07:
    /// *"The exact character is chosen when the match starts."*). Resolving here
    /// rather than at the click is what makes random actually random to the
    /// person who chose it — and resolving here rather than LATER is what keeps
    /// every stage downstream ordinary: preparation, activation and the rollback
    /// window all see a roster of concrete character ids, with no notion that
    /// one of them was a surprise.
    ///
    /// ⚠ **`seed` is required, not ambient** (ADR 0023: no ambient RNG). The
    /// caller supplies something that varies per match; this rolls a
    /// deterministic stream off it, the same shape the boss patterns use. A
    /// seeded stream is also what lets a test ask for a specific draw instead of
    /// asserting "some fighter".
    ///
    /// ⚠ **the POLICY is a parameter for the same reason
    /// [`source_name_under`] takes one**: a slot's occupant number is an index
    /// into the sources this screen offered, and what index zero MEANS —
    /// the keyboard, or the first pad — is the policy's answer. Turning that
    /// index into a roster binding without it would encode one policy's
    /// arithmetic into the match.
    pub fn roster_seeded(
        &self,
        fighters: &SmashRoster,
        seed: u64,
        policy: ambition_platformer2d::input::sources::InputAssignmentPolicy,
        // **WHO ALREADY HAS A REPERTOIRE**, by id. See the kit block below: a
        // seat whose character authors its own moves keeps them, and only the
        // ones that authored nothing take this stage's generic kit.
        //
        // ⚠ **a set of ids rather than the registry**, deliberately. The
        // registry can only be populated through the preparation barrier, which
        // needs an `App` — so taking it here would mean this screen's regressions
        // could not state the case they are about without standing a whole app
        // up. The caller answers the question; this decides what to do about it.
        //
        // Empty = nobody authors anything, which is what every seat got before.
        repertoires: &std::collections::BTreeSet<String>,
        // **The floor this EXPERIENCE declares** for a seat whose character
        // states no kit — `DeclaredCombatRules::unarmed_melee`. `None` means the
        // stage says nothing, and a kit-less seat gets whatever the engine's own
        // default is wherever it is built.
        unarmed_melee: Option<ambition_platformer2d::character::MeleeActionSpec>,
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
                // ⚠ a pick with no id is DROPPED rather than clamped or
                // panicked. It means the roster shrank under a decided screen —
                // impossible today and exactly the kind of thing a hosted
                // composition could arrange — and seating a fighter nobody
                // chose is worse than seating one fewer.
                let character = match card.locked_pick()? {
                    SlotPick::Fighter(index) => fighters.get(index)?,
                    SlotPick::Random => fighters.get(rng.draw(fighters.len())?)?,
                };
                // ⭐⭐ **DOES THIS CHARACTER ALREADY HAVE MOVES?** (Jon's redirect
                // §17.) The normal Smash path is *seat character X → use X's
                // real repertoire*, and the generic kit is scaffolding for the
                // seats that have none yet. It used to be applied to EVERY seat
                // unconditionally, which meant seating the real robot got you
                // the robot wearing somebody's generic swipe.
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
                    // **THE KIT THIS MATCH GIVES THE ONES WITH NONE.**
                    //
                    // ⛔ measured 2026-08-05: SEVEN of the twelve grid
                    // fighters had no melee at all. They are Ambition's Hall
                    // cast, and a Hall NPC's row says `peaceful` because
                    // standing in a room and talking is what they were
                    // authored for — which is CORRECT where they live. A
                    // crossover stage is the one place allowed to say
                    // otherwise, and `MatchParticipant::action_set` is how it
                    // says it without editing a row that belongs to another
                    // game.
                    //
                    // ⚠ **one kit for everybody is a FLOOR, not the design**
                    // — and as of 2026-08-11 it is no longer for everybody.
                    // `player_robot_v3` authors eleven real timelines, so it
                    // keeps them; the Hall cast still takes the floor until
                    // somebody writes them one. The adopter count is the
                    // thing to watch: this kit's goal is DELETION, and every
                    // character that gains a repertoire removes an adopter.
                    .on_team(format!("seat {}", slot + 1));
                Some(match (authors_its_own, unarmed_melee.clone()) {
                    // Its own repertoire outranks any floor.
                    (true, _) => seat,
                    // The stage declared one: hand it over.
                    (false, Some(melee)) => {
                        let mut kit = ambition_platformer2d::character::ActionSet::default();
                        kit.melee = Some(melee);
                        seat.with_action_set(kit)
                    }
                    // ⚠ the stage said nothing AND this character says nothing.
                    // Leaving the seat bare is the honest outcome — it is what a
                    // composition with no declared floor asked for — and it is
                    // reachable only from a fixture, because the shipped smash
                    // experience always declares one.
                    (false, None) => seat,
                })
            })
            .collect();
        // **THE RULESET, from the one place that states it.** This block used
        // to be a copy of `smash_roster`'s, comments and all.
        crate::apply_smash_match_rules(&mut roster);
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

/// **WHICH INPUT DEVICE a slot's person is holding, in words.**
///
/// Jon, 2026-08-07: *"the UI has no way to indicate which player is connected to
/// which input device, so idk if that is the problem or not"* — asked while
/// debugging a couch match, and answered with text rather than a glyph because
/// *"text saying which input device it is is fine for the prototype. gives more
/// info for debugging."*
///
/// ⭐ **derived from the SAME two authorities that decided the index**, not from
/// a second table: [`seats_offered_under`] turns `LocalDeviceOrder` + the policy
/// into how many sources exist, and this turns one of those indices back into
/// the source it names. A separate mapping would be a second answer to "what is
/// device 1" and would drift the first time the policy changed — which is
/// exactly the shape the roster/topology pair has already been bitten by.
///
/// ⚠ the keyboard is device ZERO only under the multi-source policies;
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

/// **WHICH SOURCE a slot's occupant number names**, under a stated policy.
///
/// ⭐ **the one place this screen's index arithmetic lives.** It was written out
/// twice — once to label a slot, once to build the roster binding — and the two
/// were the same three lines with different return types. That is exactly the
/// pair that drifts: the label said `KEYBOARD` while the roster said pad zero,
/// and the match then bound the keyboard player to a controller.
///
/// ⚠ the keyboard is device ZERO only under the multi-source policies;
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

/// **What every fighter on this stage swings.**
///
/// The demo's `duelist` preset, in Rust rather than by catalog reference,
/// because the roster hands the kit to characters whose OWN rows this demo does
/// not own and must not edit. Numbers match `SMASH_CATALOG_RON`'s `duelist`
/// action set — a real swipe, because the whole point of the stage is that a hit
/// LAUNCHES and a fighter with no melee cannot knock anybody off anything.
// ⛔ **`smash_fighter_kit()` IS DELETED** (2026-08-12). It was this crate's answer
// to "what does a body that authored no kit swing?" — one swipe, granted to every
// seat whose character says nothing — and EXPLORATION answered the same question
// with different numbers in a different crate. Two spellings, neither owned.
//
// ⇒ the numbers moved to `DeclaredCombatRules::unarmed_melee`, verbatim, where a
// ruleset fact belongs: a STAGE states what an unarmed fighter swings for. This
// screen reads the declaration instead of carrying one.

#[cfg(test)]
mod tests;

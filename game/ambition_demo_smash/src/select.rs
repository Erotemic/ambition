//! **Character select: where a match is DECIDED before it is seated.**
//!
//! Jon, 2026-07-31: *"the smash demo must start with the character select screen
//! and then start the battle when up to 4 players are locked in."*
//!
//! ## Why this is a state and not a menu
//!
//! A menu picks one thing for one person. This decides, per seat, three
//! independent facts — whether somebody is there, who they chose, and whether
//! they are committed — and the match cannot begin until every seat that is
//! there has answered the third. That is a small state machine per seat and a
//! quorum rule over the set, which is exactly the thing that goes wrong when it
//! is written as UI callbacks: "everyone is ready" gets computed from whichever
//! widget last fired.
//!
//! So the whole decision is a pure value with no Bevy in it. The route drives it
//! and draws it; the value decides.
//!
//! ## The rule, stated once
//!
//! **The battle starts when every JOINED seat is locked in, at least two are,
//! and at least one of them is a person.** Each part is a different bug:
//!
//! * without "every joined seat", a player who has joined and is still browsing
//!   gets dropped into a fight as whoever the cursor happened to be on;
//! * without "at least two", the first player to lock in starts a match against
//!   nobody — and a stocks match with one side never ends, because
//!   `last_side_standing` correctly refuses to call a sole survivor a winner;
//! * without "at least one person", the second CPU somebody adds starts a match
//!   they are not in, before they have chosen a character.
//!
//! ## Who is at a seat
//!
//! A seat is empty, a person, or a CPU — and the third exists because the
//! screen offered one seat per PAD, so a player alone at a keyboard could never
//! reach the two decided seats a match needs. Down adds a CPU to the lowest
//! empty seat; Up takes the last one back off.

use crate::{MatchParticipant, MatchParticipantRoster, STARTING_STOCKS};

/// One screen, four seats — the same ceiling the versus stage carries and the
/// same one `SlotControls` holds.
pub const MAX_SMASH_SEATS: usize = 4;

/// The characters a seat can choose between.
///
/// This demo's own two duelists. A wider roster is a content question, not a
/// select question, and the select code below does not know how many there are.
pub const SELECTABLE: &[&str] = &[crate::SMASH_CHARACTER_ID, crate::SMASH_OPPONENT_ID];

/// What one seat has decided so far.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SeatSelection {
    /// Nobody is at this seat. A seat that never joins is not a fighter, and is
    /// not waited for.
    #[default]
    Empty,
    /// Somebody joined and is still choosing. The match waits for this.
    Browsing { cursor: usize },
    /// Committed. `character` indexes [`SELECTABLE`].
    LockedIn { character: usize },
    /// **A seat the players ADDED rather than sat at.** `character` indexes
    /// [`SELECTABLE`].
    ///
    /// ⚠ this is the difference between a demo and a demo two people have to be
    /// in the room for. The screen offered one seat per PAD (floor one), the
    /// match needed two decided seats, and every decided seat was a human — so
    /// on a keyboard, alone, there was no sequence of presses that started a
    /// match at all. Jon, 2026-07-31: *"there seems to be no way to have player
    /// 2 be a CPU player."*
    ///
    /// A CPU seat is decided the moment it exists: nobody is browsing on its
    /// behalf, and a seat waiting for input from something that has none is a
    /// screen that never becomes ready.
    Cpu { character: usize },
}

impl SeatSelection {
    pub fn is_present(self) -> bool {
        !matches!(self, SeatSelection::Empty)
    }

    /// The character this seat has COMMITTED to, human or otherwise. `None`
    /// while a seat is empty or still browsing — the two states the match waits
    /// on.
    pub fn locked_character(self) -> Option<usize> {
        match self {
            SeatSelection::LockedIn { character } | SeatSelection::Cpu { character } => {
                Some(character)
            }
            _ => None,
        }
    }

    pub fn is_cpu(self) -> bool {
        matches!(self, SeatSelection::Cpu { .. })
    }
}

/// The whole screen's decision.
#[derive(bevy::prelude::Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SmashSelect {
    seats: [SeatSelection; MAX_SMASH_SEATS],
}

impl SmashSelect {
    pub fn seat(&self, seat: usize) -> SeatSelection {
        self.seats.get(seat).copied().unwrap_or_default()
    }

    /// **Somebody pressed confirm at an empty seat.** That is how you join —
    /// there is no separate "press start", because pressing anything at a seat
    /// nobody is using is unambiguous.
    ///
    /// Joining puts the cursor at the first character rather than locking in
    /// immediately: a join that also committed would make the fastest hand
    /// choose for the slowest.
    pub fn join(&mut self, seat: usize) {
        if seat >= MAX_SMASH_SEATS {
            return;
        }
        // A CPU's chair is takeable: sitting down at one replaces it, which is
        // what pressing confirm at a seat a machine is holding obviously means.
        if !matches!(self.seats[seat], SeatSelection::LockedIn { .. })
            && !matches!(self.seats[seat], SeatSelection::Browsing { .. })
        {
            self.seats[seat] = SeatSelection::Browsing { cursor: 0 };
        }
    }

    /// Move a browsing seat's cursor. Wraps, because a cursor that stops at the
    /// end makes the last character harder to pick than the first.
    pub fn browse(&mut self, seat: usize, delta: i32) {
        if SELECTABLE.is_empty() || seat >= MAX_SMASH_SEATS {
            return;
        }
        if let SeatSelection::Browsing { cursor } = self.seats[seat] {
            let count = SELECTABLE.len() as i32;
            let next = (cursor as i32 + delta).rem_euclid(count);
            self.seats[seat] = SeatSelection::Browsing {
                cursor: next as usize,
            };
        }
    }

    /// Commit a browsing seat.
    pub fn lock_in(&mut self, seat: usize) {
        if seat >= MAX_SMASH_SEATS {
            return;
        }
        if let SeatSelection::Browsing { cursor } = self.seats[seat] {
            self.seats[seat] = SeatSelection::LockedIn { character: cursor };
        }
    }

    /// **Add a CPU to the lowest empty seat.**
    ///
    /// ⚠ **bound to DOWN, and the binding is the feature.** The first cut used
    /// `start`, which reads well in a prompt and does not exist: on a keyboard
    /// `SandboxAction::Start` is Escape, which opens the pause menu, so the one
    /// press that made the demo playable alone was the one press a keyboard
    /// could not make. Jon, 2026-07-31: *"Start does not add a CPU. And there is
    /// no start on a keyboard."* Down/Up are the two directions this screen does
    /// nothing else with — left/right browse characters — and they are the same
    /// two keys/d-pad rows on every device.
    ///
    /// A CPU never lands on a seat somebody is at, and never on the seat of the
    /// player who ASKED for it — `pressed_by` is still empty when a lone player
    /// adds an opponent before sitting down, and taking that chair would answer
    /// "give me somebody to fight" by handing their own seat to a machine.
    ///
    /// Its character is the seat's own default, so pressing this once from seat
    /// 0 produces the fight the demo is about: Duelist A against Duelist B.
    pub fn add_cpu(&mut self, pressed_by: usize) {
        let empty = self
            .seats
            .iter()
            .enumerate()
            .position(|(seat, selection)| seat != pressed_by && *selection == SeatSelection::Empty);
        if let Some(seat) = empty {
            self.seats[seat] = SeatSelection::Cpu {
                character: default_character_for(seat),
            };
        }
    }

    /// Take the last added CPU back off, highest seat first — the opposite of
    /// [`Self::add_cpu`], so a press too many costs a press.
    pub fn remove_cpu(&mut self) {
        if let Some(seat) = self.seats.iter().rposition(|seat| seat.is_cpu()) {
            self.seats[seat] = SeatSelection::Empty;
        }
    }

    /// How many seats are CPUs.
    pub fn cpus(&self) -> usize {
        self.seats.iter().filter(|seat| seat.is_cpu()).count()
    }

    /// Back out: a locked seat returns to browsing, a browsing seat leaves.
    ///
    /// ⚠ **the ladder matters.** A single "cancel" that emptied the seat would
    /// make an accidental lock-in cost you your place in the match, which is the
    /// one thing a select screen must never do to somebody holding a controller.
    pub fn cancel(&mut self, seat: usize) {
        if seat >= MAX_SMASH_SEATS {
            return;
        }
        self.seats[seat] = match self.seats[seat] {
            SeatSelection::LockedIn { character } => SeatSelection::Browsing { cursor: character },
            SeatSelection::Browsing { .. } => SeatSelection::Empty,
            // Backing out AT a CPU's seat removes that CPU. Reachable when a pad
            // is plugged in after the CPU was added, which is exactly when
            // somebody wants the chair back.
            SeatSelection::Cpu { .. } => SeatSelection::Empty,
            SeatSelection::Empty => SeatSelection::Empty,
        };
    }

    /// How many seats are occupied at all.
    pub fn joined(&self) -> usize {
        self.seats.iter().filter(|seat| seat.is_present()).count()
    }

    /// How many have committed.
    pub fn locked_in(&self) -> usize {
        self.seats
            .iter()
            .filter(|seat| seat.locked_character().is_some())
            .count()
    }

    /// Seats a PERSON has committed to, as opposed to ones somebody added.
    pub fn humans_locked_in(&self) -> usize {
        self.seats
            .iter()
            .filter(|seat| matches!(seat, SeatSelection::LockedIn { .. }))
            .count()
    }

    /// **Can the battle start?**
    ///
    /// Every joined seat is locked in, at least two are, and at least one of
    /// them is a person. See the module doc for why the first two are
    /// load-bearing; the third arrived with CPU seats and is the same argument
    /// one step on — two CPUs decide the moment they exist, so without it the
    /// SECOND press of "add a CPU" started a match nobody was in, before the
    /// player who pressed it had chosen a character.
    pub fn ready(&self) -> bool {
        self.locked_in() >= 2 && self.joined() == self.locked_in() && self.humans_locked_in() >= 1
    }

    /// The match this screen decided.
    ///
    /// `None` until [`Self::ready`]. Building a roster from a half-decided
    /// screen is the failure this returns an `Option` to make impossible: a
    /// browsing seat has a cursor, and a cursor reads exactly like a choice.
    pub fn roster(&self) -> Option<MatchParticipantRoster> {
        if !self.ready() {
            return None;
        }
        let mut roster = MatchParticipantRoster::of(Vec::<String>::new());
        roster.participants = self
            .seats
            .iter()
            .enumerate()
            .filter_map(|(seat, selection)| {
                let character = SELECTABLE[selection.locked_character()?];
                Some(
                    MatchParticipant::new(character)
                        // A seat is driven by whoever the SCREEN says is at it.
                        // Nothing is filled in on anybody's behalf: an empty
                        // seat stays out of the match, and a CPU seat is one
                        // somebody asked for.
                        .driven_by(if selection.is_cpu() {
                            crate::ControllerBinding::Cpu {
                                brain_profile: Some(crate::SMASH_DUELIST_BRAIN.to_string()),
                            }
                        } else {
                            crate::ControllerBinding::Human {
                                device_slot: seat as u8,
                            }
                        })
                        .on_team(format!("seat {}", seat + 1)),
                )
            })
            .collect();
        roster.opens_suspended = true;
        roster.fighter_stocks = Some(STARTING_STOCKS);
        // WHOSE match this is. A host with a second stage in it removes "the
        // roster" on leaving its own route, and without an owner that teardown
        // reaches this one — which is how the stage stopped opening the day this
        // demo was listed on the title screen.
        Some(roster.published_by(crate::SMASH_EXPERIENCE))
    }
}

/// The character a seat starts on when nobody chose for it.
///
/// Seat-indexed rather than constant, so the first CPU somebody adds is the
/// OTHER duelist: a solo player pressing one button gets Duelist A against
/// Duelist B, which is the fight this demo is about.
fn default_character_for(seat: usize) -> usize {
    if SELECTABLE.is_empty() {
        0
    } else {
        seat % SELECTABLE.len()
    }
}

/// **How many seats this screen offers, from the pads that are actually
/// plugged in.**
///
/// Jon's *"up to 4 players"* is a CEILING, not a count. The floor is one,
/// because a keyboard is player one on every other route in this game and a
/// select screen that showed zero seats when nobody had a gamepad would be a
/// demo you cannot start.
///
/// ⚠ this reads the live device order rather than a frozen topology, and that is
/// correct HERE and would be wrong one route later. A select screen is exactly
/// where somebody plugs a controller in — that is what the screen is for — so it
/// must follow discovery. A rollback session freezes its seating precisely so the
/// MATCH cannot; the two answers are different on purpose, and the seam between
/// them is the moment the roster is published.
pub fn seats_offered(devices: &ambition::input::LocalDeviceOrder) -> usize {
    devices.devices().len().clamp(1, MAX_SMASH_SEATS)
}

/// Which seats a player may join, given the pads present.
pub fn joinable_seats(devices: &ambition::input::LocalDeviceOrder) -> std::ops::Range<usize> {
    0..seats_offered(devices)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn two_locked() -> SmashSelect {
        let mut select = SmashSelect::default();
        select.join(0);
        select.lock_in(0);
        select.join(1);
        select.lock_in(1);
        select
    }

    /// Joining puts you at the first character, browsing — not locked. A join
    /// that also committed would let the fastest hand choose for the slowest.
    #[test]
    fn joining_starts_you_browsing_rather_than_committed() {
        let mut select = SmashSelect::default();
        select.join(2);
        assert_eq!(select.seat(2), SeatSelection::Browsing { cursor: 0 });
        assert_eq!(select.joined(), 1);
        assert_eq!(select.locked_in(), 0);
    }

    /// The cursor wraps: a list that stops at the end makes the last character
    /// harder to pick than the first.
    #[test]
    fn the_cursor_wraps_in_both_directions() {
        let mut select = SmashSelect::default();
        select.join(0);
        select.browse(0, -1);
        assert_eq!(
            select.seat(0),
            SeatSelection::Browsing {
                cursor: SELECTABLE.len() - 1
            }
        );
        select.browse(0, 1);
        assert_eq!(select.seat(0), SeatSelection::Browsing { cursor: 0 });
    }

    /// **A joined-but-browsing seat holds the match**, or a player who is still
    /// deciding gets dropped into a fight as whoever the cursor was on.
    #[test]
    fn a_seat_still_browsing_holds_the_battle() {
        let mut select = two_locked();
        assert!(select.ready());
        select.join(2);
        assert!(
            !select.ready(),
            "a third player joined and is still choosing, and the match started \\
             without waiting for them"
        );
        select.lock_in(2);
        assert!(select.ready());
    }

    /// **One locked seat is not a match.** A stocks match with one side never
    /// ends — `last_side_standing` correctly refuses to call a sole survivor a
    /// winner — so starting one is a game that cannot finish.
    #[test]
    fn a_single_locked_seat_never_starts_a_battle() {
        let mut select = SmashSelect::default();
        select.join(0);
        select.lock_in(0);
        assert!(!select.ready());
        assert!(select.roster().is_none());
    }

    /// Cancel is a LADDER: locked goes back to browsing, browsing leaves. A
    /// single cancel that emptied the seat would make a misclick cost you your
    /// place in the match.
    #[test]
    fn cancel_steps_back_one_rung_at_a_time() {
        let mut select = SmashSelect::default();
        select.join(0);
        select.browse(0, 1);
        select.lock_in(0);
        assert_eq!(select.seat(0), SeatSelection::LockedIn { character: 1 });

        select.cancel(0);
        assert_eq!(
            select.seat(0),
            SeatSelection::Browsing { cursor: 1 },
            "cancelling a lock-in lost the choice as well as the commitment"
        );
        select.cancel(0);
        assert_eq!(select.seat(0), SeatSelection::Empty);
        select.cancel(0);
        assert_eq!(select.seat(0), SeatSelection::Empty, "cancel underflowed");
    }

    /// The roster is the screen's decision, and only exists once it IS one.
    #[test]
    fn the_roster_carries_every_locked_seat_as_a_human_on_its_own_side() {
        let mut select = two_locked();
        select.join(3);
        select.browse(3, 1);
        select.lock_in(3);

        let roster = select.roster().expect("three locked seats are a match");
        assert_eq!(roster.participants.len(), 3);
        assert_eq!(roster.fighter_stocks, Some(STARTING_STOCKS));
        assert!(roster.opens_suspended);

        // Seat 3's device slot is 3, not 2 — the roster is indexed by the SEAT
        // somebody sat at, not by how many people showed up. A compacted list
        // would hand seat 3's controller to the wrong body.
        let slots: Vec<u8> = roster
            .participants
            .iter()
            .filter_map(|participant| match participant.controller {
                crate::ControllerBinding::Human { device_slot } => Some(device_slot),
                _ => None,
            })
            .collect();
        assert_eq!(
            slots,
            vec![0, 1, 3],
            "the roster renumbered the seats, so a player's controller drives \\
             somebody else's fighter"
        );
    }

    /// **A match nobody is in is not one anybody asked for.**
    ///
    /// A CPU seat is decided the moment it exists, so two of them satisfied
    /// "every joined seat is locked in, and at least two are" on the SECOND
    /// press of the add-a-CPU button — the screen started a fight between two
    /// machines while the player who pressed it had not chosen a character.
    #[test]
    fn two_cpus_and_nobody_playing_is_not_a_match() {
        let mut select = SmashSelect::default();
        select.add_cpu(0);
        select.add_cpu(0);
        assert_eq!(select.cpus(), 2);
        assert!(!select.ready(), "a match with no people in it started");
        assert!(select.roster().is_none());

        // The person joins and commits: now it is a match.
        select.join(0);
        select.lock_in(0);
        assert!(select.ready());
        let roster = select.roster().expect("one player and two CPUs is a match");
        assert_eq!(roster.participants.len(), 3);
    }

    /// The presser's own chair is never the one that gets filled.
    #[test]
    fn a_cpu_never_takes_the_seat_of_the_player_who_asked_for_it() {
        let mut select = SmashSelect::default();
        select.add_cpu(0);
        assert_eq!(select.seat(0), SeatSelection::Empty);
        assert_eq!(select.seat(1), SeatSelection::Cpu { character: 1 });
    }

    /// A screen nobody joined decides nothing.
    #[test]
    fn an_untouched_screen_is_not_a_match() {
        let select = SmashSelect::default();
        assert!(!select.ready());
        assert!(select.roster().is_none());
        assert_eq!(select.joined(), 0);
    }

    /// **The seat count comes from the pads, and the floor is one.**
    ///
    /// A screen that showed zero seats when nobody had a gamepad would be a demo
    /// you cannot start — the keyboard is player one on every other route here.
    #[test]
    fn the_screen_offers_a_seat_per_pad_with_a_keyboard_floor() {
        use ambition::input::LocalDeviceOrder;
        use bevy::prelude::Entity;

        let pads = |count: u32| {
            LocalDeviceOrder::from_devices(
                (0..count)
                    .filter_map(Entity::from_raw_u32)
                    .collect::<Vec<_>>(),
            )
        };
        assert_eq!(
            seats_offered(&pads(0)),
            1,
            "no gamepads offered no seats, so the demo cannot be started from a \
             keyboard"
        );
        assert_eq!(seats_offered(&pads(2)), 2);
        assert_eq!(
            seats_offered(&pads(9)),
            MAX_SMASH_SEATS,
            "nine pads offered nine seats; four is the ceiling the screen, the \
             stage and `SlotControls` all share"
        );
        assert_eq!(joinable_seats(&pads(3)).len(), 3);
    }

    /// Four is the ceiling, and a fifth seat is not a panic.
    #[test]
    fn a_seat_past_the_ceiling_is_ignored_rather_than_a_crash() {
        let mut select = SmashSelect::default();
        select.join(MAX_SMASH_SEATS);
        select.lock_in(MAX_SMASH_SEATS);
        select.cancel(MAX_SMASH_SEATS);
        assert_eq!(select.joined(), 0);
    }
}

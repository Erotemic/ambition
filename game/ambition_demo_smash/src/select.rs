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
//! **The battle starts when every JOINED seat is locked in, and at least two
//! are.** Both halves matter and each is a different bug:
//!
//! * without "every joined seat", a player who has joined and is still browsing
//!   gets dropped into a fight as whoever the cursor happened to be on;
//! * without "at least two", the first player to lock in starts a match against
//!   nobody — and a stocks match with one side never ends, because
//!   `last_side_standing` correctly refuses to call a sole survivor a winner.

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
}

impl SeatSelection {
    pub fn is_present(self) -> bool {
        !matches!(self, SeatSelection::Empty)
    }

    pub fn locked_character(self) -> Option<usize> {
        match self {
            SeatSelection::LockedIn { character } => Some(character),
            _ => None,
        }
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
        if self.seats[seat] == SeatSelection::Empty {
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

    /// **Can the battle start?**
    ///
    /// Every joined seat is locked in, AND at least two are. See the module doc
    /// for why each half is load-bearing.
    pub fn ready(&self) -> bool {
        self.locked_in() >= 2 && self.joined() == self.locked_in()
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
                        // EVERY locked seat is a human. This is a couch game and
                        // the whole point of the screen is that the people at it
                        // chose; filling an empty seat with a CPU would make the
                        // screen a suggestion.
                        .driven_by(crate::ControllerBinding::Human {
                            device_slot: seat as u8,
                        })
                        .on_team(format!("seat {}", seat + 1)),
                )
            })
            .collect();
        roster.opens_suspended = true;
        roster.fighter_stocks = Some(STARTING_STOCKS);
        Some(roster)
    }
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

    /// A screen nobody joined decides nothing.
    #[test]
    fn an_untouched_screen_is_not_a_match() {
        let select = SmashSelect::default();
        assert!(!select.ready());
        assert!(select.roster().is_none());
        assert_eq!(select.joined(), 0);
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

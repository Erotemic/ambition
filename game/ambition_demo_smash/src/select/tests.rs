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
    use ambition_platformer2d::input::LocalDeviceOrder;
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

/// **Jon's couch milestones 1 and 2: a keyboard player and a pad player.**
///
/// ⛔ Pad-only counting made this impossible to express. One keyboard and one pad
/// offered ONE seat, so both sources drove player one and the pad player had
/// nowhere to sit. The keyboard is not a row in `LocalDeviceOrder` — that holds
/// gamepad entities — so it could never be counted, only assumed.
#[test]
fn a_keyboard_and_one_pad_offer_two_seats_under_the_couch_policy() {
    use ambition_platformer2d::input::sources::InputAssignmentPolicy;

    let mut world = bevy::prelude::World::new();
    let pad = world.spawn_empty().id();
    let one_pad = ambition_platformer2d::input::LocalDeviceOrder::from_devices(vec![pad]);

    assert_eq!(
        super::seats_offered_under(&one_pad, InputAssignmentPolicy::JoinToClaim),
        2,
        "the keyboard is player one and the pad brings its own seat"
    );
}

/// ⚠ **Milestone 8: solo play must not change.** A single player with a spare
/// controller must not discover that plugging it in created an empty chair.
#[test]
fn the_unified_policy_keeps_the_pad_only_count() {
    use ambition_platformer2d::input::sources::InputAssignmentPolicy;

    let mut world = bevy::prelude::World::new();
    let a = world.spawn_empty().id();
    let b = world.spawn_empty().id();
    let none = ambition_platformer2d::input::LocalDeviceOrder::from_devices(vec![]);
    let one = ambition_platformer2d::input::LocalDeviceOrder::from_devices(vec![a]);
    let two = ambition_platformer2d::input::LocalDeviceOrder::from_devices(vec![a, b]);

    for (devices, expected) in [(&none, 1), (&one, 1), (&two, 2)] {
        assert_eq!(
            super::seats_offered_under(devices, InputAssignmentPolicy::UnifiedPrimary),
            expected
        );
        // And the un-suffixed helper is the unified one, byte for byte.
        assert_eq!(super::seats_offered(devices), expected);
    }
}

/// The ceiling still holds: four sources is four seats, five is still four.
#[test]
fn the_couch_policy_still_respects_the_seat_ceiling() {
    use ambition_platformer2d::input::sources::InputAssignmentPolicy;

    let mut world = bevy::prelude::World::new();
    let pads: Vec<_> = (0..5).map(|_| world.spawn_empty().id()).collect();
    let many = ambition_platformer2d::input::LocalDeviceOrder::from_devices(pads);
    assert_eq!(
        super::seats_offered_under(&many, InputAssignmentPolicy::JoinToClaim),
        super::MAX_SMASH_SEATS
    );
}

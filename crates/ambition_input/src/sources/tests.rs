//! Keyboard ownership tests for couch multiplayer and solo play.

use super::*;

#[test]
fn solo_play_never_gives_the_keyboard_an_exclusive_owner() {
    // Milestone 8: "Single-participant Ambition still supports unified keyboard
    // and gamepad control." One seat means there is nobody to exclude, and
    // saying `Some(PRIMARY)` would rewrite an `InputMap` to no effect.
    for policy in [
        InputAssignmentPolicy::UnifiedPrimary,
        InputAssignmentPolicy::JoinToClaim,
        InputAssignmentPolicy::ExplicitAssignment,
    ] {
        assert_eq!(keyboard_owner_for(policy, KeyboardOwner(None), 1), None);
        assert_eq!(
            keyboard_owner_for(policy, KeyboardOwner(Some(ParticipantId::SECONDARY)), 1),
            None,
            "{policy:?} must not partition a keyboard nobody is competing for"
        );
    }
}

#[test]
fn unified_primary_shares_the_keyboard_even_with_two_seats() {
    // The default policy is today's behaviour byte for byte. Installing this
    // module must not change what any existing game does.
    assert_eq!(
        keyboard_owner_for(InputAssignmentPolicy::UnifiedPrimary, KeyboardOwner(None), 2),
        None
    );
    assert_eq!(
        keyboard_owner_for(
            InputAssignmentPolicy::UnifiedPrimary,
            KeyboardOwner(Some(ParticipantId::SECONDARY)),
            2
        ),
        None,
        "a recorded owner must not leak into a policy that does not partition"
    );
}

#[test]
fn an_unclaimed_keyboard_stays_with_player_one_when_a_pad_joins() {
    // Milestones 1 and 2 together: the keyboard is one participant and the pad
    // is another. The pad player joining must not take the keyboard away from
    // the person already using it.
    assert_eq!(
        keyboard_owner_for(InputAssignmentPolicy::JoinToClaim, KeyboardOwner(None), 2),
        Some(ParticipantId::PRIMARY)
    );
}

#[test]
fn a_claimed_keyboard_belongs_to_the_claimant_not_to_seat_zero() {
    // Milestone 6, in the shape it takes here: ownership moves only by an
    // explicit act. If player two claimed the keyboard, seat 0 does not get it
    // back by being seat 0.
    assert_eq!(
        keyboard_owner_for(
            InputAssignmentPolicy::JoinToClaim,
            KeyboardOwner(Some(ParticipantId::SECONDARY)),
            2
        ),
        Some(ParticipantId::SECONDARY)
    );
}

#[test]
fn explicit_assignment_says_nothing_of_its_own() {
    // The host is the authority under this policy, so an absent mapping means
    // "not assigned", NOT "fall back to player one". A default that guesses is
    // how a replay silently drives the wrong seat.
    assert_eq!(
        keyboard_owner_for(
            InputAssignmentPolicy::ExplicitAssignment,
            KeyboardOwner(None),
            2
        ),
        None
    );
    assert_eq!(
        keyboard_owner_for(
            InputAssignmentPolicy::ExplicitAssignment,
            KeyboardOwner(Some(ParticipantId::SECONDARY)),
            4
        ),
        Some(ParticipantId::SECONDARY)
    );
}

#[test]
fn the_keyboard_and_mouse_are_one_source() {
    let mut world = World::new();
    let pad = world.spawn_empty().id();
    assert!(InputSourceId::Keyboard.is_keyboard());
    assert!(!InputSourceId::Gamepad(pad).is_keyboard());
}

#[test]
fn two_gamepads_are_distinct_sources() {
    let mut world = World::new();
    let first = world.spawn_empty().id();
    let second = world.spawn_empty().id();
    assert_ne!(
        InputSourceId::Gamepad(first),
        InputSourceId::Gamepad(second)
    );
}

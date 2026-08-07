use super::*;

/// **A roster with room in it**, so a test about the DECISION is not also a
/// test of how many fighters ship today.
///
/// ⚠ six invented ids rather than `SmashRoster::default()`: the real default is
/// whatever this demo declares itself, and a decision test that picked index 3
/// would then start failing the day somebody edited [`SMASH_ROSTER`] — which is
/// exactly the list Jon asked to be easy to edit.
fn fighters() -> SmashRoster {
    SmashRoster(
        (0..6)
            .map(|index| format!("fighter_{index}"))
            .collect::<Vec<_>>(),
    )
}

/// Two people, two characters — the smallest thing that is a match.
fn two_decided() -> SmashSelect {
    let mut select = SmashSelect::default();
    select.set_occupant(0, SlotOccupant::Controller { device: 0 });
    select.set_pick(0, 0);
    select.set_occupant(1, SlotOccupant::Controller { device: 1 });
    select.set_pick(1, 1);
    select
}

/// **The button's three rungs, in Jon's order.** Empty seats the person who
/// pressed it; a second press hands that chair to the machine; a third empties
/// it again.
#[test]
fn the_slot_button_cycles_absent_controller_cpu() {
    let mut select = SmashSelect::default();
    select.cycle_occupant(0, 4);
    assert_eq!(
        select.slot(0).occupant,
        SlotOccupant::Controller { device: 0 }
    );
    select.cycle_occupant(0, 4);
    assert_eq!(select.slot(0).occupant, SlotOccupant::Cpu);
    select.cycle_occupant(0, 4);
    assert_eq!(select.slot(0).occupant, SlotOccupant::Absent);
}

/// **No two slots hold the same input source.**
///
/// ⛔ This is the couch defect in its smallest form. Two slots that both say "a
/// person" without saying WHICH person is how one pad drives two fighters —
/// found five separate times in this repo, and invisible every time with a
/// single pad plugged in.
#[test]
fn two_controller_slots_never_share_one_device() {
    let mut select = SmashSelect::default();
    for slot in 0..MAX_SMASH_SEATS {
        select.cycle_occupant(slot, MAX_SMASH_SEATS);
    }
    let devices: Vec<usize> = (0..MAX_SMASH_SEATS)
        .filter_map(|slot| select.slot(slot).occupant.device())
        .collect();
    assert_eq!(
        devices,
        vec![0, 1, 2, 3],
        "four slots claimed four sources and two of them agreed on one"
    );
}

/// **Jon: a controller player "must have a corresponding attached
/// controller".** With one source in the room, the second card cannot be a
/// controller — so its button skips that rung and offers the only honest thing
/// left.
#[test]
fn a_slot_with_no_free_source_skips_straight_to_cpu() {
    let mut select = SmashSelect::default();
    select.cycle_occupant(0, 1);
    assert_eq!(
        select.slot(0).occupant,
        SlotOccupant::Controller { device: 0 }
    );

    select.cycle_occupant(1, 1);
    assert_eq!(
        select.slot(1).occupant,
        SlotOccupant::Cpu,
        "the only source was taken and the second card became a controller anyway"
    );
}

/// Freeing a source hands it back. A slot that becomes absent releases its
/// device, or unplugging-by-toggling would leak chairs.
#[test]
fn emptying_a_slot_returns_its_source_to_the_pool() {
    let mut select = SmashSelect::default();
    select.cycle_occupant(0, 1); // controller on the one source
    select.cycle_occupant(0, 1); // cpu
    select.cycle_occupant(0, 1); // absent — source released
    select.cycle_occupant(1, 1);
    assert_eq!(
        select.slot(1).occupant,
        SlotOccupant::Controller { device: 0 }
    );
}

/// **A participating slot with no character holds the match**, or a player who
/// is still deciding gets dropped into a fight as whoever the cursor was over.
#[test]
fn a_slot_that_has_not_picked_holds_the_battle() {
    let mut select = two_decided();
    assert!(select.ready());

    select.set_occupant(2, SlotOccupant::Controller { device: 2 });
    assert!(
        !select.ready(),
        "a third player joined and had chosen nobody, and the match started anyway"
    );
    assert_eq!(
        select.blocker(),
        Some("Drag each slot's token onto a portrait")
    );

    select.set_pick(2, 3);
    assert!(select.ready());
    assert_eq!(select.blocker(), None);
}

/// **One decided slot is not a match.** A stocks match with one side never
/// ends — `last_side_standing` correctly refuses to call a sole survivor a
/// winner — so starting one is a game that cannot finish.
#[test]
fn a_single_decided_slot_never_starts_a_battle() {
    let mut select = SmashSelect::default();
    select.set_occupant(0, SlotOccupant::Controller { device: 0 });
    select.set_pick(0, 0);
    assert!(!select.ready());
    assert!(select.roster(&fighters()).is_none());
}

/// **A MATCH BETWEEN TWO MACHINES IS A MATCH.**
///
/// ⛔ **this test used to assert the opposite**, and it was wrong — not subtly,
/// but against a requirement Jon stated outright on 2026-08-06: *"it does not
/// let me make a CPU vs CPU match, and it is very important that that is
/// expressible and easy to do."* The rule it pinned was
/// `humans_decided() >= 1` in `ready()`, whose reasoning read *"the screen would
/// start a fight between two machines while the player who set them up had not
/// chosen anybody"* — which describes exactly what somebody watching their own
/// AI fight itself is asking for. Watching is participating.
///
/// ⚠ the shape is worth keeping in view: the rule and this test agreed with each
/// other, so the suite was green over a feature the product did not have. A test
/// that encodes a policy cannot also be the evidence the policy is right.
///
/// What remains is product policy that IS true: every participating slot has
/// picked, and at least two participate.
#[test]
fn two_cpus_are_a_match_somebody_can_watch() {
    let mut select = SmashSelect::default();
    for slot in [0, 1] {
        select.set_occupant(slot, SlotOccupant::Cpu);
        select.seed_pick(slot, &fighters());
    }
    assert_eq!(select.cpus(), 2);
    assert!(
        select.ready(),
        "two decided CPU slots are two fighters and a stage; refusing them is \
         the defect Jon reported, not a safeguard"
    );
    let roster = select
        .roster(&fighters())
        .expect("a decided two-CPU lobby produces a roster");
    assert_eq!(roster.participants.len(), 2);
    assert!(
        select.blocker().is_none(),
        "a ready lobby must not still be telling somebody what to do: {:?}",
        select.blocker()
    );
}

// ⛔ **THE GRID-FILTER TEST MOVED, and where it went is the point.** It built a
// synthetic CATALOG string and asserted `assemble` kept the id that catalog had
// and dropped the one it did not. On 2026-08-07 the filter moved from the
// catalog to the PREPARED REGISTRY — a row says what a character IS,
// `register_character` is what makes one BUILDABLE, and eight of the twelve
// shipped portraits were rows nothing had registered — and "can be seated" is
// not a fact a parsed catalog string has. Filling a registry needs the
// preparation barrier, which needs a composition, which this crate is not.
//
// The claim now lives in `ambition_app`'s `smash_roster_movesets`, against the
// REAL shipped registry: every assembled id is one the roster names and one the
// host can seat. A synthetic re-implementation here would have been a test of a
// fixture rather than of the screen.

/// **The roster list is a list of DISTINCT characters.**
///
/// ⚠ a duplicate id is two cells for one fighter, and a token dropped on the
/// second one picks a character whose cell is not the one that lit up.
#[test]
fn the_roster_names_no_character_twice() {
    let mut seen: Vec<&str> = SMASH_ROSTER.to_vec();
    seen.sort_unstable();
    let before = seen.len();
    seen.dedup();
    assert_eq!(before, seen.len(), "the roster names a character twice");
    assert!(
        SMASH_ROSTER.len() >= 4,
        "the grid is the feature; a roster this short is not one"
    );
}

/// A screen nobody touched decides nothing.
#[test]
fn an_untouched_screen_is_not_a_match() {
    let select = SmashSelect::default();
    assert!(!select.ready());
    assert!(select.roster(&fighters()).is_none());
    assert_eq!(select.participating(), 0);
    assert!(select.blocker().is_some());
}

/// Four is the ceiling, and a fifth slot is not a panic.
#[test]
fn a_slot_past_the_ceiling_is_ignored_rather_than_a_crash() {
    let mut select = SmashSelect::default();
    select.cycle_occupant(MAX_SMASH_SEATS, 4);
    select.set_pick(MAX_SMASH_SEATS, 0);
    select.seed_pick(MAX_SMASH_SEATS, &fighters());
    assert_eq!(select.participating(), 0);
}

/// **The source count comes from the pads, and the floor is one.**
///
/// A screen that offered zero sources when nobody had a gamepad would be a demo
/// you cannot start — the keyboard is player one on every other route here.
#[test]
fn the_screen_offers_a_source_per_pad_with_a_keyboard_floor() {
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
        "no gamepads offered no sources, so the demo cannot be started from a \
         keyboard"
    );
    assert_eq!(seats_offered(&pads(2)), 2);
    assert_eq!(
        seats_offered(&pads(9)),
        MAX_SMASH_SEATS,
        "nine pads offered nine sources; four is the ceiling the screen, the \
         stage and `SlotControls` all share"
    );
}

/// **Jon's couch milestones 1 and 2: a keyboard player and a pad player.**
///
/// ⛔ Pad-only counting made this impossible to express. One keyboard and one pad
/// offered ONE source, so both drove player one and the pad player had nowhere
/// to sit. The keyboard is not a row in `LocalDeviceOrder` — that holds gamepad
/// entities — so it could never be counted, only assumed.
#[test]
fn a_keyboard_and_one_pad_offer_two_sources_under_the_couch_policy() {
    use ambition_platformer2d::input::sources::InputAssignmentPolicy;

    let mut world = bevy::prelude::World::new();
    let pad = world.spawn_empty().id();
    let one_pad = ambition_platformer2d::input::LocalDeviceOrder::from_devices(vec![pad]);

    assert_eq!(
        super::seats_offered_under(&one_pad, InputAssignmentPolicy::JoinToClaim),
        2,
        "the keyboard is player one and the pad brings its own source"
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

/// The ceiling still holds: four sources is four slots, five is still four.
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

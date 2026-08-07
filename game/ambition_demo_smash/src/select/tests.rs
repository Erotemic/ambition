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

/// **Two CPUs ARE a match, and a person can join them.**
///
/// Jon, 2026-08-06: *"it does not let me make a CPU vs CPU match, and it is very
/// important that that is expressible and easy to do."*
///
/// ⛔ **this test asserted the opposite until 2026-08-07, and stayed red for a
/// day.** `ready()` had a third clause — `humans_decided() >= 1` — removed on
/// that instruction; the test that pinned the clause was left behind, still
/// demanding a blocker string (`"At least one slot must be a controller
/// player"`) that no longer exists anywhere in the source. ⚠ a red test whose
/// expected value is absent from the tree is the tell: it is describing a
/// version of the product that was decided against, not a fix that is owed.
///
/// It is kept rather than deleted because its second half was always right, and
/// because Jon's rule deserves a pin at THIS level — until now the only thing
/// asserting CPU-vs-CPU was the host-level `two_cpus_can_fight_each_other`,
/// which cannot fail for a reason as small as `ready()` growing a clause back.
#[test]
fn two_cpus_are_a_match_and_a_person_can_join_them() {
    let mut select = SmashSelect::default();
    for slot in [0, 1] {
        select.set_occupant(slot, SlotOccupant::Cpu);
        select.seed_pick(slot, &fighters());
    }
    assert_eq!(select.cpus(), 2);
    assert!(
        select.ready(),
        "a CPU-vs-CPU match could not start, which Jon asked for by name"
    );
    assert_eq!(
        select.blocker(),
        None,
        "the screen named an obstacle to a match that is allowed to start"
    );
    assert_eq!(
        select
            .roster(&fighters())
            .expect("two decided CPUs are a match")
            .participants
            .len(),
        2
    );

    // ...and a person joining does not displace them.
    select.set_occupant(2, SlotOccupant::Controller { device: 0 });
    select.set_pick(2, 2);
    assert!(select.ready());
    assert_eq!(
        select
            .roster(&fighters())
            .expect("one player and two CPUs is a match")
            .participants
            .len(),
        3
    );
}

/// **A pick outlives its occupant.** Toggling a slot from controller to CPU and
/// back is how a player hands their fighter to the machine; clearing the
/// portrait on the way through would make that a re-pick every time.
#[test]
fn the_chosen_character_survives_the_button() {
    let mut select = SmashSelect::default();
    select.cycle_occupant(0, 2);
    select.set_pick(0, 5);
    select.cycle_occupant(0, 2); // → CPU
    assert_eq!(select.slot(0).pick, Some(5));
    select.cycle_occupant(0, 2); // → absent
    assert_eq!(select.slot(0).pick, Some(5));
    assert_eq!(
        select.slot(0).locked_character(),
        None,
        "an absent slot's remembered pick counted toward the match"
    );
}

/// **A pick with no fighter behind it costs a SEAT, not a wrong fighter.**
///
/// The roster is a composition fact now, so it can in principle be smaller than
/// an index a decided screen is holding. Dropping that seat is the only safe
/// answer: clamping would seat somebody nobody chose, and a panic would take
/// the whole match down over one card.
#[test]
fn a_pick_past_the_end_of_the_roster_loses_its_seat_rather_than_inventing_one() {
    let mut select = SmashSelect::default();
    select.set_occupant(0, SlotOccupant::Controller { device: 0 });
    select.set_pick(0, 0);
    select.set_occupant(1, SlotOccupant::Controller { device: 1 });
    select.set_pick(1, 1);
    select.set_occupant(2, SlotOccupant::Cpu);
    select.set_pick(2, 999);

    let roster = select
        .roster(&fighters())
        .expect("two seats with real fighters are still a match");
    assert_eq!(
        roster.participants.len(),
        2,
        "a pick nothing in the roster answers put a fighter on the stage anyway"
    );
}

/// The roster is the screen's decision, and only exists once it IS one.
#[test]
fn the_roster_carries_every_decided_slot_on_its_own_side() {
    let mut select = two_decided();
    select.set_occupant(3, SlotOccupant::Controller { device: 3 });
    select.set_pick(3, 2);

    let roster = select
        .roster(&fighters())
        .expect("three decided slots are a match");
    assert_eq!(roster.participants.len(), 3);
    assert_eq!(roster.fighter_stocks, Some(STARTING_STOCKS));
    assert!(roster.opens_suspended);

    // Slot 3's device is 3, not 2 — the roster is indexed by the SOURCE
    // somebody holds, not by how many people showed up. A compacted list would
    // hand slot 3's controller to the wrong body.
    let devices: Vec<u8> = roster
        .participants
        .iter()
        .filter_map(|participant| match participant.controller {
            crate::ControllerBinding::Human { device_slot } => Some(device_slot),
            _ => None,
        })
        .collect();
    assert_eq!(
        devices,
        vec![0, 1, 3],
        "the roster renumbered the slots, so a player's controller drives \
         somebody else's fighter"
    );
}

/// **Every id this demo DECLARES is one its own catalog carries.** ⛔ a roster
/// naming a character the catalog does not have is a seat the match REFUSES,
/// and the refusal arrives at spawn time on a screen that already said "go".
#[test]
fn every_own_fighter_is_declared_by_this_demo() {
    for id in OWN_FIGHTERS {
        assert!(
            crate::SMASH_CATALOG_RON.contains(&format!("\"{id}\":")),
            "'{id}' is one of this demo's own fighters and no catalog row declares it"
        );
        assert!(
            SMASH_ROSTER.contains(id),
            "'{id}' is declared and then left off the grid"
        );
    }
}

/// **The grid is the ROSTER LIST, filtered to what this composition carries.**
///
/// ⛔ the first draft declared its own copies of Mary-O, Sanic and Solid Snake
/// and the assembled catalog rejected every one on display-name uniqueness. The
/// cast is shared by ID; this is the rule that replaced the copies, and the
/// filter is what lets one list serve a standalone demo and a multi-game host.
#[test]
fn the_grid_is_the_roster_list_filtered_to_what_the_composition_carries() {
    use ambition_platformer2d::character::{parse_catalog, CharacterCatalog};

    // A composition carrying exactly ONE of the roster's ids, plus a character
    // the roster does not name.
    let present = SMASH_ROSTER
        .iter()
        .find(|id| !OWN_FIGHTERS.contains(id))
        .expect("the roster names fighters beyond this demo's own");
    let catalog = CharacterCatalog::from_data(parse_catalog(&format!(
        r#"(
            brain_presets: {{ "stand_still": StandStill }},
            action_set_presets: {{
                "peaceful": (move_style: Walk, melee: None, ranged: None, special: None),
            }},
            characters: {{
                "{present}": (
                    display_name: "A Guest",
                    spritesheet: "sprites/guest_spritesheet.png",
                    manifest: "sprites/guest_spritesheet.ron",
                    tier: MainHall, body_kind: Standard, composition: None,
                    default_brain: "stand_still", default_action_set: "peaceful",
                ),
                "stranger_the_roster_does_not_name": (
                    display_name: "A Stranger",
                    spritesheet: "sprites/stranger_spritesheet.png",
                    manifest: "sprites/stranger_spritesheet.ron",
                    tier: MainHall, body_kind: Standard, composition: None,
                    default_brain: "stand_still", default_action_set: "peaceful",
                ),
            }},
        )"#
    )));

    let assembled = SmashRoster::assemble(&catalog);
    assert_eq!(
        assembled.ids().collect::<Vec<_>>(),
        vec![*present],
        "the grid dropped a fighter the composition HAS, kept one it does not, \
         or let in a character the roster never named"
    );
}

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

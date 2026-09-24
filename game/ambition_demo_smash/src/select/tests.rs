use super::*;

/// The policy every roster test in this file builds under.
///
/// Occupant numbers index the offered sources, and the policy says what index
/// zero means (the first pad here, the keyboard under `JoinToClaim`). Naming
/// it once lets these tests read "slot 3 holds pad 3".
const UNIFIED: ambition_platformer2d::input::sources::InputAssignmentPolicy =
    ambition_platformer2d::input::sources::InputAssignmentPolicy::UnifiedPrimary;

/// A seat with its own moves keeps them; a seat with none takes the floor.
///
/// Both terms matter: a character that authors a repertoire must not get the
/// stage kit, and one that authors nothing (a Hall NPC's `peaceful` row) must,
/// or it would be unarmed on the stage.
#[test]
fn a_fighter_that_authors_its_own_moves_is_not_handed_the_stage_kit() {
    const ARMED: &str = "has_its_own";
    const UNARMED: &str = "authors_nothing";
    let repertoires: std::collections::BTreeSet<String> = [ARMED.to_string()].into_iter().collect();

    let fighters = SmashRoster(vec![ARMED.to_string(), UNARMED.to_string()]);
    let mut select = SmashSelect::default();
    for (slot, pick) in [(0usize, 0usize), (1, 1)] {
        select.set_occupant(slot, SlotOccupant::Cpu);
        select.set_pick(slot, SlotPick::Fighter(pick));
    }
    // The fixture grants a floor, because the shipped experience does. An
    // experience that grants none seats an unarmed character unarmed.
    let roster = select
        .roster_seeded(
            &fighters,
            7,
            UNIFIED,
            &repertoires,
            Some(ambition_platformer2d::character::MeleeActionSpec::Swipe(
                ambition_platformer2d::character::SwipeSpec {
                    windup_s: 0.22,
                    active_s: 0.08,
                    damage: 4,
                    reach_px: 34.0,
                    recover_s: 0.26,
                },
            )),
            crate::STARTING_STOCKS,
        )
        .expect("two decided seats are a match");

    let seat_of = |id: &str| {
        roster
            .participants
            .iter()
            .find(|p| p.character.as_str() == id)
            .unwrap_or_else(|| panic!("`{id}` was not seated"))
    };
    assert!(
        seat_of(ARMED).action_set.is_none(),
        "a character with eleven authored timelines was handed the stage's \
         generic swipe, which is the whole defect §17 names"
    );
    assert!(
        seat_of(UNARMED).action_set.is_some(),
        "a character that authored no moves was seated unarmed, so the grid is \
         unplayable rather than principled"
    );
}

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

#[test]
fn the_slot_button_cycles_absent_controller_cpu() {
    let mut select = SmashSelect::default();
    select.cycle_role(0, 0, &[0]);
    assert_eq!(
        select.slot(0).occupant,
        SlotOccupant::Controller { device: 0 }
    );
    select.cycle_role(0, 0, &[0]);
    assert_eq!(select.slot(0).occupant, SlotOccupant::Cpu);
    select.cycle_role(0, 0, &[0]);
    assert_eq!(select.slot(0).occupant, SlotOccupant::Absent);
}

/// No two slots hold the same input source.
///
/// Two slots that both say "a person" without saying which one let one pad
/// drive two fighters, which a single-pad setup cannot reveal.
#[test]
fn two_controller_slots_never_share_one_device() {
    let mut select = SmashSelect::default();
    let connected = [0, 1, 2, 3];
    for slot in 0..MAX_SMASH_SEATS {
        select.cycle_role(slot, slot, &connected);
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

/// A seated player may enable an empty card for another connected person.
///
/// The role button edits the roster, not ownership-by-click: P1 can turn
/// slot 1 into connected-but-unseated P2's card without P2 pressing it.
#[test]
fn a_seated_participant_can_enable_a_card_for_another_connected_source() {
    let mut select = SmashSelect::default();
    select.cycle_role(0, 0, &[0, 1]);
    assert_eq!(
        select.slot(0).occupant,
        SlotOccupant::Controller { device: 0 }
    );

    select.cycle_role(1, 0, &[0, 1]);
    assert_eq!(
        select.slot(1).occupant,
        SlotOccupant::Controller { device: 1 },
        "P1 enabled a second human card but the connected P2 source was not seated"
    );
}

/// With no unseated connected participant left, an empty card advances to CPU.
#[test]
fn an_empty_card_becomes_cpu_when_every_connected_source_is_already_seated() {
    let mut select = SmashSelect::default();
    select.cycle_role(0, 0, &[0]);
    select.cycle_role(1, 0, &[0]);
    assert_eq!(select.slot(1).occupant, SlotOccupant::Cpu);
}

/// Once a participant's old slot becomes absent, that source is available for
/// another card again.
#[test]
fn an_absent_slot_releases_its_participant_for_another_card() {
    let mut select = SmashSelect::default();
    select.cycle_role(0, 0, &[0]); // controller
    select.cycle_role(0, 0, &[0]); // cpu
    select.cycle_role(0, 0, &[0]); // absent
    select.cycle_role(1, 0, &[0]);
    assert_eq!(
        select.slot(1).occupant,
        SlotOccupant::Controller { device: 0 }
    );
}

/// Selecting a fighter is itself a valid join action for an unseated source.
#[test]
fn an_unseated_source_claims_the_first_absent_card_on_selection() {
    let mut select = SmashSelect::default();
    select.set_occupant(0, SlotOccupant::Cpu);

    let slot = select
        .slot_for_or_claim(1)
        .expect("an absent card remained for the connected source");

    assert_eq!(slot, 1);
    assert_eq!(
        select.slot(1).occupant,
        SlotOccupant::Controller { device: 1 }
    );
    assert_eq!(select.slot(1).pick, Some(SlotPick::Random));
}

/// Claiming is idempotent: a source that already owns a card keeps that card.
#[test]
fn selecting_again_does_not_move_an_existing_human_between_cards() {
    let mut select = SmashSelect::default();
    assert_eq!(select.slot_for_or_claim(1), Some(0));
    assert_eq!(select.slot_for_or_claim(1), Some(0));
    assert_eq!(select.slot(1).occupant, SlotOccupant::Absent);
}

/// A third player joins on random and the match stays ready. `ready()` still
/// needs every participating slot to hold a pick; joining supplies one.
#[test]
fn a_third_player_joins_on_random_and_the_match_is_still_ready() {
    let mut select = two_decided();
    assert!(select.ready());

    select.set_occupant(2, SlotOccupant::Controller { device: 2 });
    assert_eq!(
        select.slot(2).pick,
        Some(SlotPick::Random),
        "a slot that just joined was left with nothing chosen"
    );
    assert!(
        select.ready(),
        "a player who joined and took random cannot start the match, so random \
         is not a decision the screen believes in"
    );
    assert_eq!(select.blocker(), None);

    // Naming a fighter afterwards is an ordinary re-pick.
    select.set_pick(2, 3);
    assert_eq!(select.slot(2).pick, Some(SlotPick::Fighter(3)));
    assert!(select.ready());
}

/// One decided slot is not a match: `last_side_standing` refuses to call a
/// sole survivor a winner, so the match could never finish.
#[test]
fn a_single_decided_slot_never_starts_a_battle() {
    let mut select = SmashSelect::default();
    select.set_occupant(0, SlotOccupant::Controller { device: 0 });
    select.set_pick(0, 0);
    assert!(!select.ready());
    assert!(select.roster(&fighters(), UNIFIED).is_none());
}

/// Two CPUs are a match, and a person can join them.
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
            .roster(&fighters(), UNIFIED)
            .expect("two decided CPUs are a match")
            .participants
            .len(),
        2
    );

    // A person joining does not displace them.
    select.set_occupant(2, SlotOccupant::Controller { device: 0 });
    select.set_pick(2, 2);
    assert!(select.ready());
    assert_eq!(
        select
            .roster(&fighters(), UNIFIED)
            .expect("one player and two CPUs is a match")
            .participants
            .len(),
        3
    );
}

/// Human↔CPU keeps the current character, but `Absent` is a lifecycle
/// boundary. Rejoining starts from Random rather than reviving stale selection.
#[test]
fn absent_clears_the_pick_and_rejoin_starts_on_random() {
    let mut select = SmashSelect::default();
    select.cycle_role(0, 0, &[0]);
    select.set_pick(0, 5);
    select.cycle_role(0, 0, &[0]); // → CPU
    assert_eq!(select.slot(0).pick, Some(SlotPick::Fighter(5)));
    select.cycle_role(0, 0, &[0]); // → absent
    assert_eq!(select.slot(0).pick, None);

    select.cycle_role(0, 0, &[0]); // → controller again
    assert_eq!(
        select.slot(0).pick,
        Some(SlotPick::Random),
        "rejoining an absent slot revived an old character instead of Random"
    );
}

/// A pick with no fighter behind it costs a seat, not a wrong fighter.
///
/// The roster is a composition fact, so it can be smaller than an index a
/// decided screen holds. Clamping would seat somebody nobody chose; a panic
/// would end the match over one card.
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
        .roster(&fighters(), UNIFIED)
        .expect("two seats with real fighters are still a match");
    assert_eq!(
        roster.participants.len(),
        2,
        "a pick nothing in the roster answers put a fighter on the stage anyway"
    );
}

/// The roster is the screen's decision, and exists only once it is one.
#[test]
fn the_roster_carries_every_decided_slot_on_its_own_side() {
    let mut select = two_decided();
    select.set_occupant(3, SlotOccupant::Controller { device: 3 });
    select.set_pick(3, 2);

    let roster = select
        .roster(&fighters(), UNIFIED)
        .expect("three decided slots are a match");
    assert_eq!(roster.participants.len(), 3);
    assert_eq!(roster.rules.stocks, Some(STARTING_STOCKS));
    assert!(roster.rules.opens_suspended);

    // Slot 3's device is 3, not 2: the roster is indexed by the source held,
    // not by how many people showed up.
    let devices: Vec<u8> = roster
        .participants
        .iter()
        .filter_map(|participant| match participant.controller {
            crate::ControllerBinding::Human {
                source: ambition_platformer2d::actor::LocalInputSource::Pad(pad),
            } => Some(pad),
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

/// Every id this demo declares is one its own catalog carries. Otherwise the
/// match refuses the seat at spawn, after the screen said "go".
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

// The grid-filter test lives in `ambition_app` as
// `smash_in_the_host::the_grid_offers_only_named_and_seatable_fighters`: the
// filter needs a real `PreparedCharacterRegistry` (a composition), which this
// crate cannot fill. It asserts both directions.

/// The roster list names distinct characters: a duplicate id is two cells for
/// one fighter.
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
    assert!(select.roster(&fighters(), UNIFIED).is_none());
    assert_eq!(select.participating(), 0);
    assert!(select.blocker().is_some());
}

/// Four is the ceiling, and a fifth slot is not a panic.
#[test]
fn a_slot_past_the_ceiling_is_ignored_rather_than_a_crash() {
    let mut select = SmashSelect::default();
    select.cycle_role(MAX_SMASH_SEATS, 0, &[0]);
    select.set_pick(MAX_SMASH_SEATS, 0);
    select.seed_pick(MAX_SMASH_SEATS, &fighters());
    assert_eq!(select.participating(), 0);
}

/// The source count comes from the pads, and the floor is one.
///
/// A screen with zero sources and no gamepad could not start; the keyboard is
/// player one on every other route.
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

/// One keyboard and one pad offer two sources under the couch policy.
/// `LocalDeviceOrder` holds only gamepads, so the keyboard is counted
/// separately.
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

/// Solo play must not change: plugging in a spare controller must not create
/// an empty chair.
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
        // The unsuffixed helper is the unified one.
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

/// The random square is a cell, and it is the last one.
///
/// Both directions, because each alone passes a broken mapping. Fighters
/// keeping their index is the load-bearing half: random first would re-point
/// every portrait-by-position in the screen, walkthrough and host tests.
#[test]
fn the_grid_is_the_fighters_plus_a_random_square_at_the_end() {
    let fighters = fighters();
    assert_eq!(fighters.cell_count(), fighters.len() + 1);
    for index in 0..fighters.len() {
        assert_eq!(
            fighters.cell(index),
            Some(SlotPick::Fighter(index)),
            "cell {index} stopped naming the fighter it used to"
        );
    }
    assert_eq!(
        fighters.cell(fighters.random_cell()),
        Some(SlotPick::Random)
    );
    assert_eq!(
        fighters.cell(fighters.cell_count()),
        None,
        "a click past the end of the grid chose something"
    );
}

#[test]
fn a_new_participant_starts_on_the_random_square() {
    let fighters = fighters();
    let mut select = SmashSelect::default();
    select.set_occupant(0, SlotOccupant::Cpu);
    select.seed_pick(0, &fighters);
    assert_eq!(select.slot(0).pick, Some(SlotPick::Random));
    assert!(
        select
            .slot(0)
            .locked_pick()
            .is_some_and(SlotPick::is_random),
        "a slot on random does not read as decided, so the match waits forever"
    );
}

/// RANDOM RESOLVES TO A REAL FIGHTER, AND ONLY WHEN THE MATCH STARTS.
///
/// The roster is where the draw happens, so everything downstream (the plan,
/// activation, the rollback window) sees ordinary character ids.
#[test]
fn a_random_seat_draws_a_real_fighter_at_the_start_and_not_before() {
    let fighters = fighters();
    let mut select = SmashSelect::default();
    for slot in [0, 1] {
        select.set_occupant(slot, SlotOccupant::Cpu);
        select.set_pick(slot, SlotPick::Random);
    }

    // Before the start there is no fighter to name.
    assert!(select.slot(0).pick.is_some_and(SlotPick::is_random));

    let roster = select
        .roster_seeded(&fighters, 12_345, UNIFIED, &Default::default(), None, crate::STARTING_STOCKS)
        .expect("two decided seats are a match");
    assert_eq!(roster.participants.len(), 2);
    for participant in &roster.participants {
        assert!(
            fighters
                .0
                .iter()
                .any(|id| id.as_str() == participant.character.as_str()),
            "a random seat drew `{}`, which is not on the grid",
            participant.character
        );
    }

    // Seeded, not ambient (ADR 0023): the same seed draws the same match.
    let again = select
        .roster_seeded(&fighters, 12_345, UNIFIED, &Default::default(), None, crate::STARTING_STOCKS)
        .expect("the same screen is still a match");
    assert_eq!(
        again.participants[0].character, roster.participants[0].character,
        "the same seed drew a different fighter"
    );

    // A different seed may differ. Requiring it to differ would assert that
    // no hash collision happens on a small grid.
    let other = select
        .roster_seeded(&fighters, 99, UNIFIED, &Default::default(), None, crate::STARTING_STOCKS)
        .expect("the same screen is still a match");
    assert_eq!(other.participants.len(), 2);
}

use super::*;

/// The policy every roster test in this file builds under.
///
/// the screen's occupant numbers are indices into the sources it offered,
/// and what index zero MEANS is the policy's answer — the first pad here, the
/// keyboard under `JoinToClaim`. Naming it once keeps these tests reading as
/// "slot 3 holds pad 3" rather than as arithmetic.
const UNIFIED: ambition_platformer2d::input::sources::InputAssignmentPolicy =
    ambition_platformer2d::input::sources::InputAssignmentPolicy::UnifiedPrimary;

/// A roster with room in it, so a test about the DECISION is not also a
/// test of how many fighters ship today.
///
/// A SEAT WITH ITS OWN MOVES KEEPS THEM; a seat with none takes the floor.
///
/// Two terms, and both matter: the character that authors a repertoire must NOT
/// be handed the stage kit, and the one that authors nothing must still be — a
/// Hall NPC's row says `peaceful` because standing in a room and talking is what
/// it was authored for, and a crossover stage that seated it unarmed would be
/// unplayable rather than principled.
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
    // the fixture GRANTS a floor, because the shipped experience does. It
    // passed `None` for one commit while the floor was being moved, and the test
    // went red exactly as it should have: an experience that grants no floor
    // seats an unarmed character unarmed. The invariant did not change — the
    // fixture had stopped modelling how a match is prepared.
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
/// Two slots that both say "a person" without saying WHICH person is how one pad drives two
/// fighters — found five separate times in this repo, and invisible every time with a single
/// pad plugged in.
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
/// The role button is roster editing, not ownership-by-click: when P1 already
/// owns slot 0 and P2 is connected but unseated, P1 can turn slot 1 into P2's
/// human card without P2 having to press that card first.
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

/// A THIRD PLAYER JOINS ON RANDOM AND THE MATCH STAYS READY.
///
/// the invariant underneath did not move — `ready()` still requires every
/// participating slot to hold a pick. What moved is that joining supplies one,
/// which makes "participating and undecided" unreachable through the button.
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

    // ...and naming a fighter afterwards is an ordinary re-pick, not a
    // different kind of state.
    select.set_pick(2, 3);
    assert_eq!(select.slot(2).pick, Some(SlotPick::Fighter(3)));
    assert!(select.ready());
}

/// One decided slot is not a match. A stocks match with one side never
/// ends — `last_side_standing` correctly refuses to call a sole survivor a
/// winner — so starting one is a game that cannot finish.
#[test]
fn a_single_decided_slot_never_starts_a_battle() {
    let mut select = SmashSelect::default();
    select.set_occupant(0, SlotOccupant::Controller { device: 0 });
    select.set_pick(0, 0);
    assert!(!select.ready());
    assert!(select.roster(&fighters(), UNIFIED).is_none());
}

/// Two CPUs ARE a match, and a person can join them.
///
/// important that that is expressible and easy to do."*
///
/// a red test whose expected value is absent from the tree is the tell: it is describing a
/// version of the product that was decided against, not a fix that is owed.
///
/// and the rule and this test AGREED with each other, so the suite was
/// green over a feature the product did not have. A test that encodes a policy
/// cannot also be the evidence the policy is right.
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

    // ...and a person joining does not displace them.
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

/// A pick with no fighter behind it costs a SEAT, not a wrong fighter.
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
        .roster(&fighters(), UNIFIED)
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
        .roster(&fighters(), UNIFIED)
        .expect("three decided slots are a match");
    assert_eq!(roster.participants.len(), 3);
    assert_eq!(roster.rules.stocks, Some(STARTING_STOCKS));
    assert!(roster.rules.opens_suspended);

    // Slot 3's device is 3, not 2 — the roster is indexed by the SOURCE
    // somebody holds, not by how many people showed up. A compacted list would
    // hand slot 3's controller to the wrong body.
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

/// Every id this demo DECLARES is one its own catalog carries. a roster
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

// `the_grid_is_the_roster_list_filtered_to_what_the_composition_carries`
// lives in `ambition_app` now, as
// `smash_in_the_host::the_grid_offers_only_named_and_seatable_fighters`.
//
// it filtered a synthetic `CharacterCatalog`, and the FILTER MOVED: a row
// says what a character IS, and `register_character` is what makes one
// BUILDABLE. Eight of the twelve shipped portraits were rows nothing had
// registered — seatable as player one, where the adopted home body consulted
// the registry optionally, and unbuildable in every other seat. This crate
// cannot fill a `PreparedCharacterRegistry` (that needs the preparation
// barrier, which needs a composition), so the claim had to move to a test with
// a real one. The host version asserts BOTH directions, because each alone is
// satisfiable by a broken filter.

/// The roster list is a list of DISTINCT characters.
///
/// a duplicate id is two cells for one fighter, and a token dropped on the
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

/// Pad-only counting made this impossible to express. One keyboard and one pad
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

/// Milestone 8: solo play must not change. A single player with a spare
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

/// THE RANDOM SQUARE IS A CELL, AND IT IS THE LAST ONE.
///
/// Both directions, because each alone is satisfiable by a broken mapping: a
/// grid that returns `Fighter` for every cell passes "the last cell is random"
/// if it never reaches it, and one that returns `Random` for everything passes
/// "fighters keep their index" if nothing checks the fighters.
///
/// fighters keeping their index is the load-bearing half. Putting random
/// FIRST would have been just as reasonable a design and would silently
/// re-point every portrait-by-position in the screen, the walkthrough and the
/// host tests at its neighbour.
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
/// the ROSTER is where the draw happens, so everything downstream — the
/// prepared plan, activation, the rollback window — sees ordinary character ids
/// and never learns that one of them was a surprise.
#[test]
fn a_random_seat_draws_a_real_fighter_at_the_start_and_not_before() {
    let fighters = fighters();
    let mut select = SmashSelect::default();
    for slot in [0, 1] {
        select.set_occupant(slot, SlotOccupant::Cpu);
        select.set_pick(slot, SlotPick::Random);
    }

    // Before the start there is no fighter to name — that is the whole point.
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

    // SEEDED, not ambient (ADR 0023). The same seed draws the same match,
    // which is what makes a desync explicable and a test able to name a draw.
    let again = select
        .roster_seeded(&fighters, 12_345, UNIFIED, &Default::default(), None, crate::STARTING_STOCKS)
        .expect("the same screen is still a match");
    assert_eq!(
        again.participants[0].character, roster.participants[0].character,
        "the same seed drew a different fighter"
    );

    // ...and a different seed is allowed to differ. Asserting it MUST differ
    // would be asserting a hash collision never happens on a grid this small.
    let other = select
        .roster_seeded(&fighters, 99, UNIFIED, &Default::default(), None, crate::STARTING_STOCKS)
        .expect("the same screen is still a match");
    assert_eq!(other.participants.len(), 2);
}


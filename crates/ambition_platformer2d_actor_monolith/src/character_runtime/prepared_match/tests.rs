//! C4 slice 1: a roster of CPU participants becomes bodies that can fight.

// Stepping a fixture is `finalize_and_update`, not `update`. Bevy's RUNNERS
// close the plugin-composition barrier; `App::update` does not, and character
// preparation publishes its registry there — so a fixture that only updated
// would register a cast and never publish one. Idempotent, so a helper called
// per step costs a set lookup after the first.
use ambition_platformer2d_shared_tangle::app_finalization::finalize_and_update;

use bevy::prelude::*;

use ambition_platformer2d_core::Vec2;

use super::*;
use crate::character_runtime::{
    ActiveMatch, CharacterDefinition, CharacterDefinitionAppExt, ControllerBinding,
    MatchParticipant, MatchParticipantRoster, MatchSeat, PreparedCharacterRegistry,
};

/// A CPU seat asking for a brain this fixture's roster ACTUALLY HAS.
///
/// ⚠ it said `medium_striker` until 2026-07-31, which the content-free default
/// roster does not contain — so every one of these tests was seating the
/// `combatant` fallback (a `StandStill` body) while naming a striker, and
/// passing, because nothing they assert depends on the brain. That silence is
/// what seating now refuses: a match seat whose brain profile is unknown is
/// unsatisfiable rather than handed a generic enemy.
///
/// `combatant` is the one archetype `CONTENT_FREE_ROSTER_RON` defines, so the
/// name now says what the fixture is really seating.
fn cpu(character: &str) -> MatchParticipant {
    MatchParticipant::new(character).driven_by(ControllerBinding::Cpu {
        brain_profile: Some("combatant".into()),
    })
}

fn seating_app() -> App {
    let mut app = App::new();
    app.init_resource::<PreparedCharacterRegistry>();
    app.insert_resource(ambition_characters::actor::character_catalog::CharacterCatalog::empty());
    app.init_resource::<ambition_sprite_sheet::character::sheets::AuthoredSheets>();
    // Seating sizes each body from its sheet (U1 stage B), so the authored
    // registry is authority the system requires. A fixture authors none.
    app.init_resource::<ambition_sprite_sheet::character::sheets::AuthoredSheets>();
    app.init_resource::<crate::features::CharacterRoster>();
    // A room whose authored spawn is the stage centre.
    let world = ambition_platformer2d_core::World::new(
        "Arena",
        Vec2::new(960.0, 540.0),
        Vec2::new(480.0, 400.0),
        vec![ambition_platformer2d_core::Block::solid(
            "floor",
            Vec2::new(0.0, 440.0),
            Vec2::new(960.0, 100.0),
        )],
    );
    ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
        app.world_mut(),
        ambition_platformer2d_core::RoomGeometry(world),
    );
    app.add_systems(
        Update,
        (
            // ⭐ **PREPARE then ACTIVATE**, the pair that replaced
            // `seat_match_participants`. Both, chained, because a fixture that
            // ran only one would be testing half a transaction — and the half
            // that can fail is the half that no longer builds anything.
            prepare_the_match,
            activate_the_prepared_match,
            // The ceremony's other end, chained exactly as production chains it.
            // ⛔ omitting it made the countdown test measure a fixture that had
            // no release system rather than a release that never fired — the
            // failure looked identical.
            release_the_opening_hold,
            // **The persona derive, which is now the writer for a seated body's
            // kit too** (Phase B, 2026-07-29). It used to be absent here and the
            // tests still passed, because the projection wrote seated kits — two
            // writers for one question, which is what let the worn and seated
            // paths disagree about the same character (H1).
            //
            // A fixture that omits the single writer proves whatever the OTHER
            // writer happened to do, which is precisely how this was missed. It
            // is chained after seating so the body exists before the derive looks
            // for it.
            crate::avatar::apply_worn_character_gameplay,
        )
            .chain(),
    );
    app
}

/// **The verb C4 was missing.** A roster could say who was in a match and could
/// demand their art; nothing turned a participant into a body, so the closest
/// thing to a versus mode was a test that hand-assembled two fighters.
#[test]
fn a_roster_of_two_cpu_participants_becomes_two_bodies_wearing_their_characters() {
    let mut app = seating_app();
    app.register_character(CharacterDefinition::new("mary_o", "Mary-O", "mary_o_demo"));
    app.register_character(CharacterDefinition::new("sanic", "Sanic", "sanic_demo"));
    app.insert_resource(MatchParticipantRoster {
        participants: vec![cpu("mary_o"), cpu("sanic")],
        ..Default::default()
    });

    finalize_and_update(&mut app);

    let world = app.world_mut();
    let mut q = world.query::<(
        &ambition_characters::actor::WornCharacter,
        &ambition_platformer2d_shared_tangle::body::BodyKinematics,
        &crate::combat::components::ActorFaction,
        Option<&crate::combat::targeting::MatchTeam>,
    )>();
    let mut seated: Vec<(
        String,
        f32,
        f32,
        crate::combat::components::ActorFaction,
        Option<crate::combat::targeting::MatchTeam>,
    )> = q
        .iter(world)
        .map(|(worn, kin, faction, team)| {
            (
                worn.id().to_string(),
                kin.pos.x,
                kin.facing,
                *faction,
                team.cloned(),
            )
        })
        .collect();
    seated.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    assert_eq!(
        seated.len(),
        2,
        "both participants must be seated: {seated:?}"
    );
    assert_eq!(seated[0].0, "mary_o");
    assert_eq!(seated[1].0, "sanic");

    // They face each other. A fighter looking the wrong way swings into empty
    // space, because a move's authored offsets are mirrored through facing —
    // this is the assertion that catches a seating change that "works" and
    // produces a match where nobody can land a hit.
    assert!(
        seated[0].1 < seated[1].1,
        "seats must be spread, not stacked: {seated:?}"
    );
    assert_eq!(seated[0].2, 1.0, "the left seat looks right");
    assert_eq!(seated[1].2, -1.0, "the right seat looks left");

    // **Opposing SIDES, or no strike between them resolves and the two bodies
    // stand and stare.** ⛔ this used to assert opposing FACTIONS, which was true
    // only because seating handed alternate seats alternate ones — the hack this
    // campaign deleted. The condition was never the faction; it is that the
    // relationship policy calls them foes, and in a match that is the team.
    let (left_team, right_team) = (
        seated[0]
            .4
            .as_ref()
            .expect("a seated fighter is in a match"),
        seated[1]
            .4
            .as_ref()
            .expect("a seated fighter is in a match"),
    );
    assert_ne!(
        left_team.as_str(),
        right_team.as_str(),
        "a free-for-all seat opposes every other seat"
    );
    // ⭐ and assert the OUTPUT, not the input: these two can actually hit each
    // other. A seating change that gives both the same team passes the line
    // above only if it also renames one of them, and fails here either way.
    assert!(
        crate::combat::targeting::damage_lands_between(
            seated[0].3,
            seated[1].3,
            Some(left_team),
            Some(right_team),
            crate::combat::targeting::FriendlyFire::default(),
            None,
            bevy::prelude::Entity::from_raw_u32(1).expect("nonzero raw index"),
        ),
        "seated fighters must be able to damage each other"
    );
}

/// Seating is a ONE-SHOT. Without the latch this re-seats every tick the roster
/// exists — a fresh pair of fighters per frame, which reads as a spawn bug three
/// systems away from the cause.
#[test]
fn seating_runs_once_however_many_ticks_pass() {
    let mut app = seating_app();
    app.register_character(CharacterDefinition::new("mary_o", "Mary-O", "mary_o_demo"));
    app.insert_resource(MatchParticipantRoster {
        participants: vec![cpu("mary_o")],
        ..Default::default()
    });

    for _ in 0..10 {
        finalize_and_update(&mut app);
    }

    let world = app.world_mut();
    let mut q = world.query::<&ambition_characters::actor::WornCharacter>();
    assert_eq!(
        q.iter(world).count(),
        1,
        "ten ticks produced more than one body: seating is not latched"
    );
}

/// **An unbuildable character is REFUSED BY NAME, not waited on.**
///
/// ⛔ this test used to say *"quiet by design"* and assert only that no body
/// appeared. That silence was the bug. Preparation's predecessor resolved this
/// seat with `registry.get(id)` and returned from the whole system on `None` —
/// no log, no record, and because the pass was all-or-nothing, **every other
/// seat went unbuilt too**. A player picking one of eight catalog-only portraits
/// in the smash grid got a stage with nobody on it and nothing anywhere saying
/// why (Jon, 2026-08-06).
///
/// ⭐ **and this is the DURABLE guard, which is why it names a character nothing
/// will ever register.** The host-level reproduction uses `npc_noether`, a real
/// grid fighter that is unbuildable *today* — and registering the Hall cast is a
/// planned step that will quietly turn that test into a check that a working
/// thing works. A guard that content can repair is not defending the gap. This
/// one names `nobody_registered_this`, so no amount of content can make it
/// vacuous.
#[test]
fn an_unbuildable_character_is_refused_by_name() {
    let mut app = seating_app();
    app.insert_resource(MatchParticipantRoster {
        participants: vec![cpu("nobody_registered_this")],
        ..Default::default()
    });
    finalize_and_update(&mut app);

    {
        let world = app.world_mut();
        let mut q = world.query::<&ambition_characters::actor::WornCharacter>();
        assert_eq!(
            q.iter(world).count(),
            0,
            "a body wearing nothing describable"
        );
    }
    assert!(
        app.world().get_resource::<ActiveMatch>().is_none(),
        "a match that built nobody must not ACTIVATE"
    );

    // **THE POINT.** A permanent failure must not present as a wait.
    let problems = app
        .world()
        .get_resource::<crate::character_runtime::MatchPreparationProblems>()
        .expect(
            "preparation refused this roster and recorded NOTHING, so a stage \
             would sit on it forever and no surface could say why — which is \
             exactly the failure this whole seam was built to remove",
        )
        .clone();
    assert_eq!(problems.problems.len(), 1, "one bad seat, one problem");
    assert_eq!(problems.problems[0].seat, 0);
    assert!(
        problems.problems[0]
            .detail
            .contains("nobody_registered_this"),
        "the refusal does not NAME the character that caused it, so a player \
         reading it learns nothing: {}",
        problems.problems[0].detail
    );
}

/// **A match builds its OWN cast and touches nothing else.**
///
/// ⛔ this replaces two tests that pinned the opposite rule — *"a human seat
/// ADOPTS the player body"* and *"a human seat does not re-dress a player
/// wearing someone else"* — and both were correct about a design that has been
/// deleted. Adoption existed to stop the arena holding two Mary-Os when the
/// session had already spawned one; the cost was that a fighter's construction
/// depended on who drove it, and every symptom of Jon's 2026-08-06 report came
/// out of that fork.
///
/// The duplicate is prevented at the other end now: a MATCH experience declares
/// no session body at all, so there is nothing to adopt, nothing to re-dress,
/// and no handshake to deadlock on. What is worth pinning is the invariant that
/// replaced them — **the match's cast is exactly its seats**, and a body that
/// was already standing there is none of the match's business.
#[test]
fn a_match_builds_its_own_cast_and_leaves_other_bodies_alone() {
    let mut app = seating_app();
    app.register_character(CharacterDefinition::new("mary_o", "Mary-O", "mary_o_demo"));
    app.register_character(CharacterDefinition::new("sanic", "Sanic", "sanic_demo"));
    // A body that is NOT in the match, wearing somebody the roster does not
    // name. Under the old design this was the thing seat 0 reached out and took.
    let bystander = app
        .world_mut()
        .spawn((
            crate::avatar::PlayerSimulationBundle::from_scratch(
                crate::avatar::primary_player_scratch(
                    Vec2::new(7.0, 0.0),
                    ambition_platformer2d_core::AbilitySet::default(),
                ),
                ambition_characters::actor::Health::new(5),
            ),
            ambition_characters::actor::WornCharacter::new("sanic"),
        ))
        .id();
    app.insert_resource(MatchParticipantRoster {
        participants: vec![
            MatchParticipant::new("mary_o").driven_by(ControllerBinding::Human {
                source: ambition_input::LocalInputSource::Pad(0),
            }),
            cpu("mary_o"),
        ],
        ..Default::default()
    });

    finalize_and_update(&mut app);

    // TWO seats, two bodies — built the same way whichever drives them.
    let seats = {
        let world = app.world_mut();
        let mut q = world.query::<&MatchSeat>();
        let mut seen: Vec<usize> = q.iter(world).map(|seat| seat.0).collect();
        seen.sort_unstable();
        seen
    };
    assert_eq!(
        seats,
        vec![0, 1],
        "a human seat and a CPU seat must both produce a body; they used to \
         travel different construction paths and only one of them spawned"
    );

    // The bystander is untouched: same costume, same place.
    assert_eq!(
        app.world()
            .get::<ambition_characters::actor::WornCharacter>(bystander)
            .map(|worn| worn.id().to_owned()),
        Some("sanic".to_owned()),
        "the match re-dressed a body outside its own cast"
    );
    let kin = app
        .world()
        .get::<ambition_platformer2d_shared_tangle::body::BodyKinematics>(bystander)
        .expect("the bystander survives");
    assert_eq!(
        kin.pos.x, 7.0,
        "the match MOVED a body outside its own cast to a seat mark"
    );
    assert!(
        app.world().get::<MatchSeat>(bystander).is_none(),
        "a body nobody put in the roster was enrolled in the match"
    );
}

/// **Couch versus: two human seats, two bodies, two slots.**
///
/// The engine has been ready for this for a while and nobody had asked it:
/// `SlotControls` holds four slots and `tick_player_brains` drives any body whose
/// `Brain::Player(slot)` names one. What was missing was a seat that produces the
/// second body — and, still, a device writer for the second slot, which is the
/// part `populate_slot_controls` names in its own docs as co-op's job.
///
/// So this asserts the half that exists: seat 1 gets its own body carrying
/// `Brain::Player(1)`, `LocalPlayer` and a `PlayerInputFrame`. A body that
/// carries the player brain for a slot nothing writes simply stands still, which
/// is the correct behaviour for an unplugged controller.
///
/// ⚠ **no pre-spawned player body here any more, and that IS the change.** This
/// fixture used to stand one up because seat 0 adopted it; now every seat is
/// built the same way and seat 0 has no privilege left to test. A match
/// experience declares no session body at all.
#[test]
fn a_second_human_seat_gets_its_own_body_on_its_own_slot() {
    use ambition_characters::brain::{Brain, PlayerSlot};

    let mut app = seating_app();
    app.register_character(CharacterDefinition::new("mary_o", "Mary-O", "mary_o_demo"));
    app.register_character(CharacterDefinition::new("sanic", "Sanic", "sanic_demo"));
    app.insert_resource(MatchParticipantRoster {
        participants: vec![
            MatchParticipant::new("mary_o").driven_by(ControllerBinding::Human {
                source: ambition_input::LocalInputSource::Pad(0),
            }),
            MatchParticipant::new("sanic").driven_by(ControllerBinding::Human {
                source: ambition_input::LocalInputSource::Pad(1),
            }),
        ],
        ..Default::default()
    });

    finalize_and_update(&mut app);

    let world = app.world_mut();
    let mut bodies = world.query::<(&ambition_characters::actor::WornCharacter, Option<&Brain>)>();
    let mut seats: Vec<(String, Option<u8>)> = bodies
        .iter(world)
        .map(|(worn, brain)| {
            (
                worn.id().to_string(),
                brain.and_then(Brain::player_slot).map(|slot| slot.0),
            )
        })
        .collect();
    seats.sort();

    assert_eq!(
        seats.len(),
        2,
        "two human seats are two bodies, not one adopted body: {seats:?}"
    );
    assert_eq!(
        seats,
        vec![
            ("mary_o".to_string(), Some(PlayerSlot::PRIMARY.0)),
            ("sanic".to_string(), Some(1)),
        ],
        "each seat's body must carry ITS OWN slot. Two bodies on one slot is one \
         player driving both, which looks like a control bug and is a seating one"
    );
}

/// A second human body is a LOCAL player, or the slot→body bridge skips it.
///
/// `sync_local_player_input_frame` only mirrors slots onto bodies carrying
/// `LocalPlayer`. Without the marker the body has a player brain, receives
/// nothing, and stands still — indistinguishable from an unplugged controller,
/// which is exactly the kind of silence that takes an afternoon to diagnose.
#[test]
fn a_second_human_body_is_marked_local_so_the_slot_bridge_reaches_it() {
    let mut app = seating_app();
    app.register_character(CharacterDefinition::new("sanic", "Sanic", "sanic_demo"));
    app.insert_resource(MatchParticipantRoster {
        participants: vec![
            MatchParticipant::new("sanic").driven_by(ControllerBinding::Human {
                source: ambition_input::LocalInputSource::Pad(1),
            }),
        ],
        ..Default::default()
    });

    finalize_and_update(&mut app);

    let world = app.world_mut();
    let mut locals = world.query::<(
        &crate::control::components::LocalPlayer,
        &crate::control::components::PlayerInputFrame,
    )>();
    assert_eq!(
        locals.iter(world).count(),
        1,
        "the second human's body is not a `LocalPlayer` with a `PlayerInputFrame`, \
         so `sync_local_player_input_frame` will never hand it its slot's input"
    );
}

/// **A SPARSE source does not make a sparse channel.** (GPT 5.6, 2026-08-07)
///
/// ⛔ **the smallest reachable form of the defect**: a lobby of two seats where
/// the FIRST is a CPU. One person is playing, so the session opens one GGRS
/// handle — and the human is holding the second controller, so the roster says
/// source `Pad(1)`. That number used to become the channel, and the fighter
/// spawned reading `PlayerSlot(1)` in a session whose only handle writes
/// `PlayerSlot(0)`. Nothing errored; the human simply could not move.
///
/// ⭐ so the assertion is a PAIR, and both halves matter: the fighter reads the
/// dense channel, and the plan still remembers which controller feeds it.
#[test]
fn a_human_behind_a_cpu_seat_still_lands_on_channel_zero() {
    use ambition_characters::brain::{Brain, PlayerSlot};

    let mut app = seating_app();
    app.register_character(CharacterDefinition::new("mary_o", "Mary-O", "mary_o_demo"));
    app.register_character(CharacterDefinition::new("sanic", "Sanic", "sanic_demo"));
    app.insert_resource(MatchParticipantRoster {
        participants: vec![
            cpu("mary_o"),
            MatchParticipant::new("sanic").driven_by(ControllerBinding::Human {
                source: ambition_input::LocalInputSource::Pad(1),
            }),
        ],
        ..Default::default()
    });

    finalize_and_update(&mut app);

    let plan = app.world().resource::<PreparedMatch>().channel_plan();
    assert_eq!(
        plan.channels(),
        1,
        "one person is playing, so the session opens exactly one handle"
    );
    assert_eq!(
        plan.source_for(ambition_input::ParticipantId::PRIMARY),
        Some(ambition_input::LocalInputSource::Pad(1)),
        "the dense channel must still remember WHICH controller feeds it — \
         compacting the source is how a player ends up driving somebody else"
    );

    let world = app.world_mut();
    let mut bodies = world.query::<(&ambition_characters::actor::WornCharacter, &Brain)>();
    let seated: Vec<(String, Option<PlayerSlot>)> = bodies
        .iter(world)
        .map(|(worn, brain)| (worn.id().to_string(), brain.player_slot()))
        .collect();
    assert!(
        seated.contains(&("sanic".to_string(), Some(PlayerSlot::PRIMARY))),
        "the only human's fighter must read the only channel the session opens; \
         a fighter on `PlayerSlot(1)` in a one-handle session receives nothing \
         for the whole match: {seated:?}"
    );
}

/// **Holes in the sources close in the channels, in seat order.**
///
/// The select screen's own case — three people holding pads 0, 1 and 3, which
/// its test pins because renumbering them hands somebody the wrong controller.
/// Three channels, no holes; three sources, one hole.
#[test]
fn three_people_on_pads_zero_one_and_three_get_channels_zero_one_and_two() {
    use ambition_characters::brain::{Brain, PlayerSlot};

    let mut app = seating_app();
    for id in ["mary_o", "sanic", "duelist"] {
        app.register_character(CharacterDefinition::new(id, id, format!("{id}_demo")));
    }
    app.insert_resource(MatchParticipantRoster {
        participants: ["mary_o", "sanic", "duelist"]
            .into_iter()
            .zip([0u8, 1, 3])
            .map(|(character, pad)| {
                MatchParticipant::new(character).driven_by(ControllerBinding::Human {
                    source: ambition_input::LocalInputSource::Pad(pad),
                })
            })
            .collect(),
        ..Default::default()
    });

    finalize_and_update(&mut app);

    assert_eq!(
        app.world()
            .resource::<PreparedMatch>()
            .channel_plan()
            .sources(),
        [0, 1, 3].map(ambition_input::LocalInputSource::Pad),
        "the plan must keep the sources people are holding"
    );

    let world = app.world_mut();
    let mut bodies = world.query::<&Brain>();
    let mut slots: Vec<u8> = bodies
        .iter(world)
        .filter_map(Brain::player_slot)
        .map(|slot| slot.0)
        .collect();
    slots.sort();
    assert_eq!(
        slots,
        vec![PlayerSlot::PRIMARY.0, 1, 2],
        "three people are three DENSE channels — `PlayerSlot(3)` names a handle \
         a three-handle session never opens"
    );
}

/// **One controller cannot drive two fighters, and preparation says so.**
///
/// ⚠ the ordinary way to arrive here is two seats built with
/// `MatchParticipant::new`, whose default is the first pad. Refused rather than
/// deduplicated: the second claimant would get a real channel, a real handle,
/// and no input at all — a fighter that stands still all match with no error
/// anywhere.
#[test]
fn two_seats_on_one_controller_are_refused_by_name() {
    let mut app = seating_app();
    app.register_character(CharacterDefinition::new("mary_o", "Mary-O", "mary_o_demo"));
    app.register_character(CharacterDefinition::new("sanic", "Sanic", "sanic_demo"));
    app.insert_resource(MatchParticipantRoster {
        participants: vec![
            MatchParticipant::new("mary_o"),
            MatchParticipant::new("sanic"),
        ],
        ..Default::default()
    });

    finalize_and_update(&mut app);

    let problems = app
        .world()
        .get_resource::<MatchPreparationProblems>()
        .expect("two seats claiming one pad must be refused, not seated")
        .to_string();
    assert!(
        problems.contains("Pad(0)"),
        "the refusal has to name the controller both seats claim: {problems}"
    );
    assert!(
        app.world().get_resource::<PreparedMatch>().is_none(),
        "a refused roster must build no plan"
    );
}

/// **A 2v2: teammates cannot hit each other, opponents can.** (queue L17)
///
/// `MatchTeam` was landed with a unit test over the pure relation and nothing
/// else, because every stage that seats a roster is 1v1 — so the property teams
/// exist FOR was proven only in a function. This seats four fighters on two
/// teams and asks the real damage relation about each pair.
///
/// It is also the first thing to seat more than two, which is what the seating
/// spread and the per-seat retry were written to handle and had never been
/// asked to do.
#[test]
fn four_fighters_on_two_teams_can_hit_their_opponents_and_not_their_partners() {
    use crate::combat::targeting::{damage_lands_between, FriendlyFire, MatchTeam};

    let mut app = seating_app();
    for id in ["alpha", "beta", "gamma", "delta"] {
        app.register_character(CharacterDefinition::new(id, id, "arena"));
    }
    // Blue, BLUE, red, red — teams that DISAGREE with the factions.
    //
    // `faction_for` alternates Player/Enemy by seat index, so a blue-red-blue-red
    // roster would have teams and factions saying the same thing and the test
    // would pass without the team rule existing. (The meaningfulness assertion at
    // the bottom caught exactly that on the first run.) Pairing adjacent seats
    // instead makes every interesting case appear at once: teammates with
    // DIFFERENT factions who must not hit each other, and opponents with the SAME
    // faction who must.
    app.insert_resource(MatchParticipantRoster {
        participants: vec![
            cpu("alpha").on_team("blue"),
            cpu("beta").on_team("blue"),
            cpu("gamma").on_team("red"),
            cpu("delta").on_team("red"),
        ],
        ..Default::default()
    });
    finalize_and_update(&mut app);

    let world = app.world_mut();
    let mut q = world.query::<(
        Entity,
        &MatchSeat,
        &MatchTeam,
        &crate::combat::components::ActorFaction,
    )>();
    let mut fighters: Vec<(
        Entity,
        usize,
        String,
        crate::combat::components::ActorFaction,
    )> = q
        .iter(world)
        .map(|(entity, seat, team, faction)| (entity, seat.0, team.0.clone(), *faction))
        .collect();
    fighters.sort_by_key(|(_, seat, ..)| *seat);
    assert_eq!(
        fighters.len(),
        4,
        "a four-participant roster seated {} bodies — seating has never been \
         asked for more than two",
        fighters.len()
    );
    assert_eq!(
        fighters
            .iter()
            .map(|(_, _, team, _)| team.as_str())
            .collect::<Vec<_>>(),
        vec!["blue", "blue", "red", "red"],
        "the declared teams did not reach the bodies"
    );

    let no_ff = FriendlyFire { enabled: false };
    for (entity, seat, team, faction) in &fighters {
        for (other_entity, other_seat, other_team, other_faction) in &fighters {
            if entity == other_entity {
                continue;
            }
            let lands = damage_lands_between(
                *faction,
                *other_faction,
                Some(&MatchTeam::new(team.clone())),
                Some(&MatchTeam::new(other_team.clone())),
                no_ff,
                None,
                *other_entity,
            );
            assert_eq!(
                lands,
                team != other_team,
                "seat {seat} ({team}) vs seat {other_seat} ({other_team}): \
                 expected the hit to land only across teams"
            );
        }
    }

    // ⭐⭐ **the vacuity guard, and the fold made it STRONGER.** It used to demand
    // that teammates hold DIFFERENT factions and opponents the SAME, so that
    // neither half of the loop above could be the faction rule wearing a team's
    // clothes. Arranging that took `faction_for(index)` — alternating
    // `Player, Enemy, Player, Enemy` by seat — which is the hack this campaign
    // deleted.
    //
    // ⇒ every seat is one faction now, so the faction rule has exactly one
    // answer for every pair in this match: ALLY. Anything the loop found that
    // differs from "nobody can hit anybody" is therefore the team rule, in both
    // directions at once. That is the same guard, and it no longer needs a
    // fixture that lies about who these characters are.
    assert!(
        fighters.iter().all(|f| f.3 == fighters[0].3),
        "every seat fights as itself, so the faction rule can only say ALLY — \
         which is what makes the cross-team hits above attributable to the team"
    );
}

/// **A seated fighter must receive its character's ACTION SET.** (campaign X9)
///
/// Seating writes `ActionSet::default()` and leaves a comment saying the derive
/// overwrites it "on the tick the worn character lands". The derive it means is
/// `apply_worn_character_gameplay`, whose query requires `IdentityKit` and
/// `BodyAbilities` — and `EnemyActorBundle` carries neither, so a seated body
/// does not match it at all. What DOES serve a seated body is
/// `project_prepared_character_definitions`, and that projects the moveset and
/// the hurtbox doc and stops.
///
/// So the action set was the one part of an authored identity that reached a
/// worn player and never reached a seated fighter. The visible consequence is a
/// CPU that will not use a kit its own definition gave it: the brain reads
/// `ActionSet` to decide whether it may press ranged at all, so an authored
/// ranged fighter stands there holding a gun it does not believe in.
#[test]
fn a_seated_fighter_receives_its_definitions_action_set() {
    use ambition_characters::brain::action_set::{RangedActionSpec, RangedStyle};
    use ambition_characters::brain::ActionSet;

    let mut app = seating_app();
    // The projection is the system that actually serves a seated body.
    app.add_systems(
        Update,
        crate::character_runtime::project_prepared_character_definitions,
    );

    let gunner = ActionSet {
        ranged: Some(RangedActionSpec {
            style: RangedStyle::default(),
            speed: 320.0,
            damage: 2,
            flight: None,
            visual: None,
        }),
        ..ActionSet::default()
    };
    app.register_character(
        CharacterDefinition::new("gunner", "Gunner", "demo").with_action_set(gunner.clone()),
    );
    app.insert_resource(MatchParticipantRoster {
        participants: vec![cpu("gunner")],
        ..Default::default()
    });
    finalize_and_update(&mut app);
    finalize_and_update(&mut app);

    let world = app.world_mut();
    let mut seated = world.query::<(&MatchSeat, &ActionSet)>();
    let found: Vec<&ActionSet> = seated.iter(world).map(|(_, set)| set).collect();
    assert_eq!(found.len(), 1, "the roster seated no fighter");
    assert_eq!(
        found[0], &gunner,
        "the seated fighter is wearing the placeholder action set seating wrote, \
         not the one its definition authored — its brain will never press ranged"
    );
}

/// **FORCING A CRAWLER INTO A FIGHTER SEAT GIVES YOU A CRAWLER.**
///
/// Jon's compositional acceptance test, at the seam where it is decided: *"Force
/// a Puppy Slug into Smash … movement input → uses Puppy Slug's actual authored
/// locomotion. Jump → no jump if its body cannot jump. Smash must not silently
/// give it a generic swipe, a generic humanoid jump, a generic dash."*
///
/// ⛔ what made it impossible until now: a seat was built from an enemy
/// ARCHETYPE, so its top speed, its gait and its contact damage were whichever
/// creature the CPU's brain key named. A crawler seated in a match ran at a
/// duelist's speed because it was, physically, a duelist.
///
/// ⚠ the fighter DEFAULT is the other half and it has to stay: a character that
/// has never said how fast it is must still be given something by the stage, or
/// every unmigrated fighter is a statue. This asserts both.
#[test]
fn a_crawler_seated_as_a_fighter_keeps_its_own_locomotion() {
    use ambition_characters::actor::{CharacterLocomotion, ContactDamage};
    use ambition_characters::brain::MoveStyleSpec;

    let mut app = seating_app();
    app.register_character(
        CharacterDefinition::new("crawler", "Puppy Slug", "demo")
            .with_locomotion(CharacterLocomotion {
                run_speed: 36.0,
                move_style: MoveStyleSpec::Slither,
                surface_walker: true,
                cling_breaks_on_hit: false,
                flies: false,
            })
            .with_contact_damage(ContactDamage {
                strength: 0.4,
                amount: 2,
            }),
    );
    app.register_character(CharacterDefinition::new("duelist", "Duelist", "demo"));
    app.insert_resource(MatchParticipantRoster {
        participants: vec![cpu("crawler"), cpu("duelist")],
        ..Default::default()
    });

    finalize_and_update(&mut app);

    let seats: Vec<(usize, f32, i32, bool)> = {
        let world = app.world_mut();
        let mut q = world.query::<(&MatchSeat, &crate::features::ActorConfig)>();
        let mut rows: Vec<(usize, f32, i32, bool)> = q
            .iter(world)
            .map(|(seat, config)| {
                (
                    seat.0,
                    config.tuning.max_run_speed,
                    config.tuning.damage_amount,
                    config.tuning.surface_walker,
                )
            })
            .collect();
        rows.sort_by_key(|(seat, _, _, _)| *seat);
        rows
    };
    assert_eq!(seats.len(), 2, "{seats:?}");
    let (_, crawler_speed, crawler_damage, crawler_clings) = seats[0];
    let (_, duelist_speed, duelist_damage, duelist_clings) = seats[1];

    assert_eq!(
        crawler_speed, 36.0,
        "the crawler is seated at somebody else's top speed: {seats:?}"
    );
    assert!(
        crawler_clings,
        "the crawler lost its surface cling by being seated: {seats:?}"
    );
    assert_eq!(
        crawler_damage, 2,
        "the crawler's contact damage did not survive the seat: {seats:?}"
    );

    assert!(
        duelist_speed > crawler_speed,
        "the character that authored NO locomotion did not receive the stage's \
         fighter default, so an unmigrated fighter is a statue: {seats:?}"
    );
    assert_eq!(
        duelist_damage, 0,
        "a fighter that authored no contact damage hurts on touch, which is the \
         engine inventing a capability: {seats:?}"
    );
    assert!(!duelist_clings);
}

/// **A MATCH GRANT COVERS THE ACTION SET, NOT THE MOVES.**
///
/// ⛔ the defect this pins, found the hour a real repertoire was first authored:
/// a crossover stage grants borrowed fighters an action set
/// (`MatchParticipant::action_set`) so a peaceful Hall NPC can attack at all —
/// and that grant was ALSO regenerating the moveset, from the granted set. So a
/// character that authored eleven move timelines was seated with one derived
/// swipe, and the timelines had no reader on the only path that seats a fighter.
///
/// The rule, in the words of the granting field's own doc: an ability is *may
/// this body attack* and levelling it is fairness; a moveset is *what the attack
/// IS* and levelling it erases the character.
///
/// ⚠ the second half is what keeps the grant working: a character that authored
/// NO moves still takes the stage's derived kit, because it has nothing to
/// protect.
#[test]
fn a_match_grant_does_not_overwrite_a_characters_authored_moves() {
    use ambition_characters::brain::{ActionSet, MeleeActionSpec, SwipeSpec};
    use ambition_entity_catalog::{ClipBinding, MoveGates, MoveSpec, MovesetContract};

    let signature = MovesetContract {
        verbs: [("attack".to_string(), "signature_smash".to_string())]
            .into_iter()
            .collect(),
        moves: vec![MoveSpec {
            id: "signature_smash".to_string(),
            clip: ClipBinding {
                clip: "attack".to_string(),
                fallbacks: Vec::new(),
            },
            duration_s: 0.5,
            windows: Vec::new(),
            events: Vec::new(),
            gates: MoveGates::default(),
            start_impulse: None,
            smash_charge_mult: 1.0,
            landing_lag_s: None,
            autocancel_after_s: None,
        }],
    };
    // The stage's borrowed kit, deliberately DIFFERENT from anything the
    // characters author, so "the grant won" and "the character won" cannot be
    // told apart by accident.
    let granted = ActionSet {
        melee: Some(MeleeActionSpec::Swipe(SwipeSpec {
            windup_s: 0.2,
            active_s: 0.1,
            recover_s: 0.2,
            damage: 4,
            reach_px: 34.0,
        })),
        ..ActionSet::default()
    };

    let mut app = seating_app();
    app.add_systems(
        Update,
        crate::character_runtime::project_prepared_character_definitions,
    );
    app.register_character(
        CharacterDefinition::new("fighter", "Fighter", "demo").with_moveset(signature.clone()),
    );
    app.register_character(CharacterDefinition::new("borrowed", "Borrowed", "demo"));
    app.insert_resource(MatchParticipantRoster {
        participants: vec![
            cpu("fighter").with_action_set(granted.clone()),
            cpu("borrowed").with_action_set(granted.clone()),
        ],
        ..Default::default()
    });
    finalize_and_update(&mut app);
    finalize_and_update(&mut app);

    let moves: Vec<(usize, Vec<String>)> = {
        let world = app.world_mut();
        let mut seated = world.query::<(&MatchSeat, &crate::combat::moveset::ActorMoveset)>();
        let mut rows: Vec<(usize, Vec<String>)> = seated
            .iter(world)
            .map(|(seat, moveset)| {
                (
                    seat.0,
                    moveset.0.moves.iter().map(|mv| mv.id.clone()).collect(),
                )
            })
            .collect();
        rows.sort_by_key(|(seat, _)| *seat);
        rows
    };
    assert_eq!(moves.len(), 2, "the roster seated fewer than two fighters");
    assert!(
        moves[0].1.iter().any(|id| id == "signature_smash"),
        "the fighter that authored its own moves is swinging the stage's borrowed \
         swipe instead: {:?}",
        moves[0].1
    );
    assert!(
        !moves[1].1.is_empty() && !moves[1].1.iter().any(|id| id == "signature_smash"),
        "the borrowed character either received nothing or received somebody \
         else's signature move: {:?}",
        moves[1].1
    );
}

/// **A seated fighter moves the way its DEFINITION says.** (campaign R-a)
///
/// The third leg of the kit, and wired into BOTH paths in one commit — because
/// the action set was wired into the worn path alone and a seated fighter went
/// without it until somebody pulled the thread (X9). A character whose
/// definition says "momentum" and whose catalog row says nothing should not move
/// like a swept-axis walker because it happens to be seated rather than worn.
///
/// Applied through `switch_motion_model`, so this also pins the ADR 0024 rule:
/// the component is transitioned, never replaced.
#[test]
fn a_seated_fighter_moves_by_its_definitions_motion_model() {
    use ambition_platformer2d_core::MotionModelSpec;

    let mut app = seating_app();
    app.add_systems(
        Update,
        crate::character_runtime::project_prepared_character_definitions,
    );

    let momentum = MotionModelSpec::SurfaceMomentum(ambition_platformer2d_core::MomentumParams {
        ground_accel: 900.0,
        top_speed: 1200.0,
        jump_speed: 700.0,
        ..Default::default()
    });
    app.register_character(
        CharacterDefinition::new("roller", "Roller", "demo").with_motion_model(momentum),
    );
    app.insert_resource(MatchParticipantRoster {
        participants: vec![cpu("roller")],
        ..Default::default()
    });
    finalize_and_update(&mut app);
    finalize_and_update(&mut app);

    let world = app.world_mut();
    let mut seated = world.query::<(&MatchSeat, &crate::features::MotionModel)>();
    let models: Vec<_> = seated.iter(world).map(|(_, model)| model.clone()).collect();
    assert_eq!(models.len(), 1, "the roster seated no fighter");
    assert!(
        matches!(models[0], crate::features::MotionModel::SurfaceMomentum(_)),
        "the seated fighter kept the catalog's swept-axis model instead of the \
         momentum one its definition authored: {:?}",
        models[0]
    );
}

/// **Movement FEEL reaches a seated fighter, and an unauthored one is left alone.**
///
/// The last kit-adjacent field, and the one whose `None` is an ANSWER rather
/// than a default: `AuthoredMovementTuning`'s presence means "this body's tuning
/// is authored, not the shared dev tuning", so a character that authored none
/// must end with NO marker.
///
/// Both halves are asserted because the first version of the projection read the
/// prepared value directly and made the two paths disagree — for a character
/// with catalog tuning and no authored tuning, the worn path inserted the marker
/// and the projection removed it on the same tick. Both go through one resolver
/// now, which is the discipline this whole campaign is about.
#[test]
fn a_seated_fighter_gets_authored_movement_feel_and_only_when_authored() {
    let mut app = seating_app();
    app.add_systems(
        Update,
        crate::character_runtime::project_prepared_character_definitions,
    );

    let springy = ambition_platformer2d_core::MovementTuning {
        jump_speed: 999.0,
        ..Default::default()
    };
    app.register_character(
        CharacterDefinition::new("springy", "Springy", "demo").with_movement_tuning(springy),
    );
    app.register_character(CharacterDefinition::new("plain", "Plain", "demo"));
    app.insert_resource(MatchParticipantRoster {
        participants: vec![cpu("springy"), cpu("plain")],
        ..Default::default()
    });
    finalize_and_update(&mut app);
    finalize_and_update(&mut app);

    let world = app.world_mut();
    let mut bodies = world.query::<(
        &ambition_characters::actor::WornCharacter,
        Option<&ambition_platformer2d_core::AuthoredMovementTuning>,
    )>();
    let found: Vec<(String, Option<f32>)> = bodies
        .iter(world)
        .map(|(worn, tuning)| {
            (
                worn.id().to_string(),
                tuning.map(|tuning| tuning.0.jump_speed),
            )
        })
        .collect();
    assert_eq!(found.len(), 2, "the roster seated {} fighters", found.len());

    for (id, jump) in found {
        match id.as_str() {
            "springy" => assert_eq!(
                jump,
                Some(999.0),
                "the authored feel never reached the seated fighter"
            ),
            "plain" => assert_eq!(
                jump, None,
                "a character that authored no feel was given the marker anyway, \
                 so it can never be returned to the live inspector sliders"
            ),
            other => panic!("unexpected seated character {other}"),
        }
    }
}

/// **A fighter that opens suspended is suspended on the tick it appears.**
/// (queue Y′8 / H5's second half, 2026-07-29)
///
/// The versus countdown gate landed and this half did not: the ruleset suspends
/// control when the countdown begins, and a fighter seating on that same tick
/// could take one simulation step first — a CPU decision, or a held direction
/// carried in from the menu — before the insert landed.
///
/// The distinction the assertion is making is ONE update, not "eventually". A
/// suspension applied by a later system would pass an "is it suspended by the
/// time the fight starts" test while still leaving the tick that was the bug.
#[test]
fn a_roster_that_opens_suspended_seats_fighters_that_cannot_act_yet() {
    let mut app = seating_app();
    app.register_character(CharacterDefinition::new("mary_o", "Mary-O", "mary_o_demo"));
    app.register_character(CharacterDefinition::new("sanic", "Sanic", "sanic_demo"));
    app.insert_resource(MatchParticipantRoster {
        participants: vec![cpu("mary_o"), cpu("sanic")],
        opens_suspended: true,
        ..Default::default()
    });

    finalize_and_update(&mut app);

    let world = app.world_mut();
    let mut bodies = world.query::<(
        &ambition_characters::actor::WornCharacter,
        Option<&ambition_characters::brain::ScriptedControl>,
    )>();
    let seated: Vec<_> = bodies
        .iter(world)
        .map(|(worn, scripted)| (worn.id().to_string(), scripted.is_some()))
        .collect();
    assert_eq!(seated.len(), 2, "both fighters must seat at all");
    for (id, suspended) in seated {
        assert!(
            suspended,
            "`{id}` seated able to act on the very tick it appeared, which is the \
             one tick the countdown cannot cover"
        );
    }
}

/// **3 — 2 — 1 — GO: the cast is held for the whole ceremony and freed on ONE
/// tick.**
///
/// Jon asked for exactly this test — *"add a deterministic test proving bodies
/// remain held before GO and release at the transition"* — and the property it
/// pins is the one a per-fighter timer would lose: not "everybody is eventually
/// free" but "there is no tick on which one fighter can act and another
/// cannot".
///
/// ⭐ **the ceremony is DERIVED, so this test is a clock and two reads.** There
/// is no timer to advance and nothing to tick: the phase is
/// `now - activated_on` against the ruleset's declared length, which is what
/// makes it survive a rollback — a rewound frame recomputes the same beat
/// instead of resuming a countdown from wherever it landed.
#[test]
fn a_declared_countdown_holds_every_seat_until_it_ends() {
    /// Short, and not a round number: a countdown that happened to match some
    /// other constant would pass for the wrong reason.
    const TICKS: u32 = 7;

    let mut app = seating_app();
    app.insert_resource(ambition_time::SimTick(0));
    app.register_character(CharacterDefinition::new("duelist", "Duelist", "demo"));
    app.insert_resource(MatchParticipantRoster {
        participants: vec![cpu("duelist"), cpu("duelist")],
        opens_suspended: true,
        opening_countdown_ticks: TICKS,
        ..Default::default()
    });

    fn held(app: &mut App) -> (usize, usize) {
        let world = app.world_mut();
        let mut q = world
            .query_filtered::<Option<&ambition_characters::brain::ScriptedControl>, With<MatchSeat>>(
            );
        let seats: Vec<bool> = q.iter(world).map(|scripted| scripted.is_some()).collect();
        (seats.iter().filter(|held| **held).count(), seats.len())
    }

    // The plan is stamped for the NEXT tick, so the cast appears on tick 1 and
    // the ceremony is measured from there.
    finalize_and_update(&mut app);
    // ⛔ **both terms must be OBSERVED.** The first version of this loop ran to
    // `TICKS` and every iteration took the "still held" branch — the release
    // assertion was never evaluated once, and it passed.
    let (mut saw_held, mut saw_released) = (false, false);
    for now in 1..=u64::from(TICKS) + 1 {
        app.insert_resource(ambition_time::SimTick(now));
        finalize_and_update(&mut app);
        let (held_now, seated) = held(&mut app);
        assert_eq!(seated, 2, "both fighters must seat at all (tick {now})");
        let elapsed = now - 1;
        if elapsed < u64::from(TICKS) {
            saw_held = true;
            assert_eq!(
                held_now, 2,
                "a fighter was free {elapsed} tick(s) into a {TICKS}-tick ceremony, \
                 so the round starts before the count does"
            );
        } else {
            saw_released = true;
            assert_eq!(
                held_now, 0,
                "the ceremony ended and somebody is still held: the release is not \
                 atomic, so one fighter acts on a tick another cannot"
            );
        }
    }
    assert!(
        saw_held && saw_released,
        "the loop never reached one of the two states, so this test is a \
         statement about its own range rather than about the ceremony"
    );
}

/// **And a LOCAL-INPUT seat is suspended on that tick too.**
///
/// A CPU seat opening suspended is nearly free — a body that does not exist
/// until the command queue flushes cannot act before its suspension lands. The
/// seat worth asking about is the one a person is holding a direction on when
/// the round opens.
///
/// ⚠ **this used to be about ADOPTION**, and it was the sharper question then:
/// seat 0 took over a body that already existed, was already in the brain's
/// query, and had been accepting input on the previous tick. That body is gone —
/// a match builds its own cast — so the question is now the ordinary one, and
/// the answer holds by construction rather than by a window being closed. Kept
/// because "every seat, whoever drives it" is still the claim.
#[test]
fn a_local_input_seat_is_also_suspended_on_the_tick_it_joins() {
    let mut app = seating_app();
    app.register_character(CharacterDefinition::new("duelist", "Duelist", "demo"));

    app.insert_resource(MatchParticipantRoster {
        participants: vec![
            MatchParticipant::new("duelist").driven_by(ControllerBinding::Human {
                source: ambition_input::LocalInputSource::Pad(0),
            }),
        ],
        opens_suspended: true,
        ..Default::default()
    });

    finalize_and_update(&mut app);

    let world = app.world_mut();
    let mut seated = world
        .query_filtered::<Option<&ambition_characters::brain::ScriptedControl>, With<MatchSeat>>();
    let suspended: Vec<bool> = seated.iter(world).map(|s| s.is_some()).collect();
    assert_eq!(
        suspended,
        vec![true],
        "the local-input seat joined the match still answering input, so whatever \
         the player was holding when the round opened moves the fighter before \
         the countdown says go"
    );
}

/// **A seated fighter's DURABLE capability baseline matches its identity.**
///
/// `ActionSet` is the hot per-frame resolver; `CombatKit` is the durable source
/// of the same capability — its own doc calls it "what the actor can do
/// innately". Several subsystems rebuild an `ActionSet` from it rather than
/// reading the live one (a brain command's `apply_catalog_mode`, the mount pair,
/// autonomous reconciliation).
///
/// Seating seeds it from `ActionSet::default()` — empty, matching what an enemy
/// spawn does before its archetype fills one in — and the persona writer then
/// installed the real action set, moveset and identity kit and left the durable
/// one at the placeholder. A seated fighter could act through its live
/// `ActionSet` and then lose its innate attacks the moment anything rebuilt them
/// from the stale baseline (GPT 5.6, 2026-07-29).
#[test]
fn a_seated_fighter_keeps_one_capability_baseline_not_two() {
    let mut app = seating_app();
    app.register_character(
        CharacterDefinition::new("brawler", "Brawler", "demo").with_action_set(
            ambition_characters::brain::ActionSet {
                melee: Some(ambition_characters::brain::MeleeActionSpec::Swipe(
                    ambition_characters::brain::SwipeSpec {
                        windup_s: 0.2,
                        active_s: 0.1,
                        recover_s: 0.2,
                        damage: 4,
                        reach_px: 44.0,
                    },
                )),
                ..ambition_characters::brain::ActionSet::default()
            },
        ),
    );
    app.insert_resource(MatchParticipantRoster {
        participants: vec![cpu("brawler")],
        ..Default::default()
    });

    finalize_and_update(&mut app);

    let world = app.world_mut();
    let mut bodies = world.query::<(
        &ambition_characters::brain::ActionSet,
        &crate::combat::components::CombatKit,
        &ambition_characters::actor::WornCharacter,
    )>();
    let (live, durable, _) = bodies
        .iter(world)
        .next()
        .expect("the fighter must seat at all");
    assert!(
        live.melee.is_some(),
        "fixture: the identity must GRANT a melee, or the mismatch under test \
         cannot exist"
    );
    assert_eq!(
        durable.innate_melee, live.melee,
        "the durable capability baseline disagrees with the identity the body is \
         actually wearing, so anything that rebuilds this fighter's kit from it — \
         a brain command, a mount, autonomous reconciliation — disarms it"
    );
}

/// **The same character is the same BODY in either seat.** (mirror match)
///
/// A spawned seat sized itself from `BodySource::Explicit` and took
/// `prepared.vitals.mass`; the adopted primary player kept whatever box and
/// weight its session gave it. So the same character could stand on the stage
/// twice with two different body shapes and two different masses, and the seat
/// that was wrong was always player one (GPT 5.6, 2026-07-29).
///
/// ⚠ **it used to drive ADOPTION specifically**, because that was the branch
/// that got these wrong: seat 0 took over a body its session had already built
/// and kept whatever box and mass that body came with. There is one construction
/// path now, so the divergence is unrepresentable — but the CHARACTER's authored
/// facts reaching a seated body is still a real claim, and a mirror match is
/// still the place a regression would show. Both seats are asserted, so "the
/// same character is the same fighter in either seat" stays checked.
#[test]
fn every_seat_gets_the_body_facts_its_character_authors() {
    let mut app = seating_app();
    app.register_character(
        // Public fields: the definition is authoring data, not a builder-only
        // type, and these two are exactly the facts under test.
        {
            let mut definition = CharacterDefinition::new("heavy", "Heavy", "demo");
            definition.body = Some(crate::character_runtime::BodySource::Explicit {
                half_extents: (19.0, 31.0),
            });
            definition.vitals.mass = Some(4.5);
            definition
        },
    );
    // A MIRROR match, driven two different ways: one seat by a local input
    // channel and one by a brain. If construction ever forks on who drives a
    // fighter again, these two stop agreeing.
    app.insert_resource(MatchParticipantRoster {
        participants: vec![
            MatchParticipant::new("heavy").driven_by(ControllerBinding::Human {
                source: ambition_input::LocalInputSource::Pad(0),
            }),
            cpu("heavy"),
        ],
        ..Default::default()
    });

    finalize_and_update(&mut app);

    let facts: Vec<((f32, f32), f32)> = {
        let world = app.world_mut();
        let mut q = world.query_filtered::<(
            &ambition_platformer2d_core::BodyKinematics,
            &crate::features::Mass,
        ), With<MatchSeat>>();
        q.iter(world)
            .map(|(kin, mass)| ((kin.size.x, kin.size.y), mass.0))
            .collect()
    };
    assert_eq!(facts.len(), 2, "a mirror match is two bodies");
    for (size, mass) in &facts {
        assert_eq!(
            *size,
            (38.0, 62.0),
            "a seat took a body box its character did not author, so a mirror \
             match puts two different shapes on the stage: {facts:?}"
        );
        assert!(
            (mass - 4.5).abs() < 1e-4,
            "a seat weighs {mass} instead of the authored 4.5, so the mount \
             pair's centre of gravity depends on which seat a character took"
        );
    }
}

/// And a roster that says nothing does not get a suspension it never asked for.
#[test]
fn an_ordinary_roster_seats_fighters_that_can_act() {
    let mut app = seating_app();
    app.register_character(CharacterDefinition::new("mary_o", "Mary-O", "mary_o_demo"));
    app.insert_resource(MatchParticipantRoster {
        participants: vec![cpu("mary_o")],
        ..Default::default()
    });

    finalize_and_update(&mut app);

    let world = app.world_mut();
    let mut bodies = world.query::<(
        &ambition_characters::actor::WornCharacter,
        Option<&ambition_characters::brain::ScriptedControl>,
    )>();
    let suspended: Vec<_> = bodies.iter(world).map(|(_, s)| s.is_some()).collect();
    assert_eq!(
        suspended,
        vec![false],
        "seating must not decide on its own that a fighter cannot act — a stage \
         with no countdown opens live"
    );
}

/// **A seated fighter INHERITS its catalog row's kit — the H1 case itself.**
/// (Phase B, 2026-07-29)
///
/// Everything before this proved that a seated fighter receives what its
/// DEFINITION authored. H1 was the other case, and it is the common one: a
/// character that authored no action set at all, whose kit comes from the
/// catalog row. That fighter worked as the worn player and stood empty-handed as
/// player two, for a day, with every test green — because the two paths had two
/// writers and only one of them consulted the catalog.
///
/// This is the test that could not have been written before Phase A: the
/// inheritance now happens at the preparation barrier, so a seated body reads a
/// resolved value rather than re-deriving one it has no catalog to derive from.
#[test]
fn a_seated_fighter_inherits_the_kit_its_catalog_row_authors() {
    use ambition_characters::actor::character_catalog::{parse_catalog, CharacterCatalog};

    const CATALOG: &str = r#"(
        brain_presets: { "stand_still": StandStill },
        action_set_presets: {
            "brawler": (
                move_style: Walk,
                melee: Some(Swipe(
                    windup_s: 0.1, active_s: 0.1, recover_s: 0.1,
                    damage: 7, reach_px: 40.0,
                )),
            ),
        },
        characters: {
            "inheritor": (
                display_name: "Inheritor", spritesheet: "a.png", manifest: "a.ron",
                tier: Basement, body_kind: Standard, composition: None,
                default_brain: "stand_still", default_action_set: "brawler", tags: [],
            ),
        },
    )"#;

    let mut app = seating_app();
    app.insert_resource(CharacterCatalog::from_data(parse_catalog(CATALOG)));
    // Registered, and authoring NO action set — so `None` means "the catalog row
    // stands", which is the registry's documented contract and the exact thing
    // the seated path could not honour.
    app.register_character(CharacterDefinition::new("inheritor", "Inheritor", "demo"));
    app.insert_resource(MatchParticipantRoster {
        participants: vec![cpu("inheritor")],
        ..Default::default()
    });

    finalize_and_update(&mut app);
    finalize_and_update(&mut app);

    let world = app.world_mut();
    let mut bodies = world.query::<(
        &ambition_characters::actor::WornCharacter,
        &ambition_characters::brain::ActionSet,
    )>();
    let (_, set) = bodies
        .iter(world)
        .next()
        .expect("the fighter must be seated at all");
    match &set.melee {
        Some(ambition_characters::brain::action_set::MeleeActionSpec::Swipe(swipe)) => {
            assert_eq!(
                swipe.damage, 7,
                "the seated fighter got A melee action, but not its row's"
            );
        }
        other => panic!(
            "the seated fighter inherited nothing from its catalog row — this is H1 \
             exactly: it would fight as the worn player and stand empty-handed as \
             player two. Got {other:?}"
        ),
    }
}

/// **A new cast generation must reach the PERSONA writer, not just the stamp.**
/// (H6 reopened, GPT 5.6 2026-07-29)
///
/// The first H6 probe replaced a cast under a hand-spawned body. That body did
/// not carry the persona writer's full column set, so it took the projection's
/// FALLBACK branch — and the probe proved generation replacement for the
/// population whose kit the projection still owns, while being cited as evidence
/// for the worn and seated population whose kit it deliberately does not.
///
/// This one uses a real seated fighter: every column, the derive installed, the
/// projection installed. The failure it catches is worse than a missed update —
/// the projection stamps `ProjectedCharacterKit` with the new generation whether
/// or not anything refreshed the kit, so the body ends up recorded as CURRENT
/// while wearing the retired cast's moves, and no later pass will revisit it.
#[test]
fn a_new_cast_generation_refreshes_a_seated_fighters_kit() {
    use ambition_characters::brain::action_set::{MeleeActionSpec, SwipeSpec};
    use ambition_characters::brain::ActionSet;

    fn swiping(damage: i32) -> ActionSet {
        ActionSet {
            melee: Some(MeleeActionSpec::Swipe(SwipeSpec {
                windup_s: 0.1,
                active_s: 0.1,
                recover_s: 0.1,
                damage,
                reach_px: 40.0,
            })),
            ..ActionSet::default()
        }
    }

    let mut app = seating_app();
    app.add_systems(
        Update,
        crate::character_runtime::project_prepared_character_definitions,
    );
    app.register_character(
        CharacterDefinition::new("veteran", "Veteran", "demo").with_action_set(swiping(3)),
    );
    app.insert_resource(MatchParticipantRoster {
        participants: vec![cpu("veteran")],
        ..Default::default()
    });
    finalize_and_update(&mut app);
    finalize_and_update(&mut app);

    fn melee_damage(app: &mut App) -> Option<i32> {
        let world = app.world_mut();
        let mut bodies = world.query::<(
            &ambition_characters::actor::WornCharacter,
            &ambition_characters::brain::ActionSet,
        )>();
        match bodies.iter(world).next().map(|(_, set)| set.melee.clone()) {
            Some(Some(MeleeActionSpec::Swipe(swipe))) => Some(swipe.damage),
            _ => None,
        }
    }

    assert_eq!(
        melee_damage(&mut app),
        Some(3),
        "the fighter must reach its first cast at all, or the replacement below \
         proves nothing"
    );

    // THE CAST IS REPLACED. Same id, same body, rebalanced numbers — a hot
    // reload, or a second composition landing on a running session.
    let rebalanced = crate::character_runtime::prepare_and_finalize_for_test(
        CharacterDefinition::new("veteran", "Veteran", "demo").with_action_set(swiping(9)),
        &crate::character_runtime::CharacterBindings::default(),
    )
    .prepared;
    app.world_mut()
        .resource_mut::<PreparedCharacterRegistry>()
        .insert_prepared(rebalanced);
    finalize_and_update(&mut app);
    finalize_and_update(&mut app);

    assert_eq!(
        melee_damage(&mut app),
        Some(9),
        "the seated fighter kept the retired cast's kit. `apply_worn_character_gameplay` \
         is the only writer for it and runs on `Changed<WornCharacter>` / \
         `Changed<BodyAbilities>` — a cast replacement changes neither, so nothing \
         refreshed it. Worse, the projection stamped the body with the NEW generation \
         anyway, so it now reads as current and no later pass will revisit it"
    );
}

/// **The authored maximum health applies to the ADOPTED seat too.**
/// (GPT 5.6, 2026-07-29)
///
/// A spawned seat took `prepared.vitals.max_health` from its seed. The adopted
/// primary player did not — it kept whatever maximum its session established from
/// the legacy catalog or the default player health. So the same character could
/// bring its authored 60 HP as player two and something else entirely as player
/// one, and the versus duelists' deliberate 60-vs-52 trade (one fighter paying for
/// a faster smash) simply did not apply to seat 0.
#[test]
fn an_adopted_seat_takes_its_characters_authored_maximum_health() {
    let mut app = seating_app();
    let mut tank = CharacterDefinition::new("tank", "Tank", "demo");
    tank.vitals = crate::character_runtime::Vitals {
        max_health: Some(60),
        mass: Some(1.0),
        knockback_weight: None,
    };
    app.register_character(tank);

    // The REAL player bundle: a hand-rolled body without the movement clusters
    // does not match seating's query, so adoption silently skips and the test
    // would pass for the wrong reason. Its health starts at a maximum the
    // character never authored, which is exactly what adoption finds.
    let player = app
        .world_mut()
        .spawn((
            crate::avatar::PlayerSimulationBundle::from_scratch(
                crate::avatar::primary_player_scratch(
                    Vec2::new(0.0, 0.0),
                    ambition_platformer2d_core::AbilitySet::default(),
                ),
                ambition_characters::actor::Health::new(999),
            ),
            ambition_characters::actor::WornCharacter::new("tank"),
        ))
        .id();
    app.insert_resource(MatchParticipantRoster {
        participants: vec![
            MatchParticipant::new("tank").driven_by(ControllerBinding::Human {
                source: ambition_input::LocalInputSource::Pad(0),
            }),
        ],
        ..Default::default()
    });
    finalize_and_update(&mut app);

    let health = app
        .world()
        .get::<ambition_characters::actor::BodyHealth>(player)
        .expect("the adopted body keeps its health component");
    assert_eq!(
        health.health.max, 60,
        "the adopted seat kept a maximum its character never authored, so the same \
         fighter is tougher as player one than as player two"
    );
    assert_eq!(
        health.health.current, 60,
        "and a seat starts full, adopted or spawned"
    );
}

/// **Both fighters get the same movement capabilities.** (2026-07-29)
///
/// A SPAWNED seat's abilities come from `AncillaryMovementBundle` — the basic
/// run-and-jump floor. The ADOPTED primary player brought whatever the session
/// granted it, which in the shipped host is the sandbox dev kit: blink, fly,
/// shield. So player one could teleport and FLY in a versus match while the
/// opponent could not, and the on-screen control legend advertised it.
///
/// Found by capturing the stage and looking at it. No test asserted anything
/// about it because nothing had thought to ask, which is the argument for looking
/// at the screen and not only at the suite.
///
/// ⚠ **the CARRIER of that unfairness is gone**: nothing adopts a body that has
/// been living in a session, so no seat can arrive holding a kit the match did
/// not grant. What still needs asserting is the other half — that the roster's
/// declared set actually REACHES every seat, and that equalising does not
/// quietly disarm the floor a fighter needs.
#[test]
fn the_matchs_declared_abilities_reach_every_seat() {
    let mut app = seating_app();
    app.register_character(CharacterDefinition::new("duelist", "Duelist", "demo"));

    app.insert_resource(MatchParticipantRoster {
        participants: vec![
            MatchParticipant::new("duelist").driven_by(ControllerBinding::Human {
                source: ambition_input::LocalInputSource::Pad(0),
            }),
            cpu("duelist"),
        ],
        fighter_abilities: Some(ambition_platformer2d_core::AbilitySet::basic()),
        ..Default::default()
    });

    finalize_and_update(&mut app);

    let kits: Vec<(bool, bool, bool, bool)> = {
        let world = app.world_mut();
        let mut q = world.query_filtered::<(
            &ambition_platformer2d_core::BodyAbilities,
            &ambition_platformer2d_core::AbilityBase,
        ), With<MatchSeat>>();
        q.iter(world)
            .map(|(abilities, base)| {
                (
                    abilities.abilities.fly,
                    abilities.abilities.jump,
                    // The BASE too, not only the effective set: the dev-tools
                    // sync recomputes `effective = base ∩ editable_mask` every
                    // frame for a player-driven body, so writing only the
                    // effective set would be undone next tick by a system doing
                    // its job correctly.
                    base.abilities.fly,
                    base.abilities.blink,
                )
            })
            .collect()
    };
    assert_eq!(kits.len(), 2, "two seats, two kits");
    for (fly, jump, base_fly, base_blink) in &kits {
        assert!(
            !fly && !base_fly && !base_blink,
            "a fighter can fly or teleport in a match that granted neither, so \
             the two seats are not playing the same game: {kits:?}"
        );
        assert!(
            jump,
            "equalising disarmed the floor every fighter needs: {kits:?}"
        );
    }
}

/// **A SEATED FIGHTER IS COMPLETE ON ITS CONSTRUCTION FRAME.**
///
/// ⛔⛔ **the last of Jon's P0: ordinary construction must not carry
/// `RecharacterizeBody`** (ledger D85). The seat did, because five things the
/// persona derive supplied could only be derived where the catalog was in scope
/// — the match kit, the death traits, the identity kit, the moveset and the
/// motion model. All five moved to preparation and seating; the applied-template
/// stamp went on LAST, which is the order that stops a stamp certifying work
/// nobody does.
///
/// ⚠ **the stamp is the assertion, not the absence of the request.** A seat with
/// neither would look identical to one that simply never got either — so this
/// checks the body carries a CURRENT stamp, which only construction can have
/// written, and that it displaced nothing, which is what distinguishes building
/// a body as a character from replacing one.
///
/// ⛔⛔ **AND IT IS STILL NOT SUFFICIENT — MEASURED.** With the seat's
/// `grant_prepared_character_body` call disabled, this test STAYS GREEN: the
/// persona derive runs on the unstamped body and writes a baseline that looks
/// exactly like construction's. Only
/// `a_seated_fighter_is_complete_and_the_next_pass_changes_nothing` goes red.
/// Keep both — this one names the request, that one names the invariant.
#[test]
fn a_seated_fighter_carries_its_applied_template_and_asks_for_nothing() {
    let mut app = seating_app();
    app.register_character(CharacterDefinition::new("duelist", "Duelist", "demo"));
    app.insert_resource(MatchParticipantRoster {
        participants: vec![cpu("duelist")],
        ..Default::default()
    });
    finalize_and_update(&mut app);

    let generation = app
        .world()
        .resource::<crate::character_runtime::PreparedCharacterRegistry>()
        .generation();
    let world = app.world_mut();
    let mut q = world.query_filtered::<(
        Option<&crate::avatar::PersonaBaseline>,
        bevy::prelude::Has<ambition_characters::actor::RecharacterizeBody>,
    ), With<MatchSeat>>();
    let seats: Vec<_> = q.iter(world).map(|(b, r)| (b.cloned(), r)).collect();
    assert_eq!(seats.len(), 1, "one seat, one body");
    let (baseline, asked) = &seats[0];
    let baseline = baseline.as_ref().expect(
        "a seated fighter carries no applied-template stamp, so the \
                 persona derive will apply its character a SECOND time",
    );
    assert_eq!(baseline.id, "duelist");
    assert_eq!(
        baseline.generation, generation,
        "the stamp names a cast other than the one that seated this fighter"
    );
    assert_eq!(
        baseline.displaced,
        Default::default(),
        "seating recorded a DISPLACEMENT, which claims a replacement happened"
    );
    assert!(
        !asked,
        "the seat still asks to be finished, so something it needs is still \
         arriving a tick late"
    );
}

/// **AND NEITHER TEMPLATE OBSERVER HAS WORK TO DO ON THE NEXT PASS.**
///
/// ⛔⛔ **the test above is not sufficient, and believing it was is the mistake
/// this one exists for** (GPT 5.6 §1, 2026-08-11). Removing `RecharacterizeBody`
/// silences the PERSONA derive. `project_prepared_character_definitions` is a
/// SECOND observer, it fires on `Changed<WornCharacter>`, and a seated body with
/// no `ProjectedCharacterKit` was still being finished by it a tick after
/// construction — hurtboxes, posed body, movement tuning, motion model.
///
/// ⭐ **so this asserts the OTHER record too, and then asserts nothing moves.**
/// Both stamps current on the construction frame is the claim; a second update
/// with no hot reload and no re-template request changing nothing is the proof.
/// D73's invariant in one test: *an ordinary match seat is a complete instance of
/// its CharacterDefinition on its construction frame.*
#[test]
fn a_seated_fighter_is_complete_and_the_next_pass_changes_nothing() {
    let mut app = seating_app();
    app.register_character(CharacterDefinition::new("duelist", "Duelist", "demo"));
    app.insert_resource(MatchParticipantRoster {
        participants: vec![cpu("duelist")],
        ..Default::default()
    });
    finalize_and_update(&mut app);

    let generation = app
        .world()
        .resource::<crate::character_runtime::PreparedCharacterRegistry>()
        .generation();

    // The two applied-template records, read together: a body carrying one and
    // not the other is a body one observer still owns.
    let read = |app: &mut App| {
        let world = app.world_mut();
        let mut q = world.query_filtered::<(
            Option<&crate::avatar::PersonaBaseline>,
            Option<&crate::character_runtime::presentation::ProjectedCharacterKit>,
            Option<&crate::features::MotionModel>,
        ), With<MatchSeat>>();
        let rows: Vec<_> = q
            .iter(world)
            .map(|(persona, projected, model)| {
                (
                    persona.cloned(),
                    projected.cloned(),
                    model.map(|m| m.kind()),
                )
            })
            .collect();
        assert_eq!(rows.len(), 1, "one seat, one body");
        rows.into_iter().next().unwrap()
    };

    let (persona, projected, model) = read(&mut app);
    let persona =
        persona.expect("no gameplay baseline, so the persona derive still owns this body");
    let projected = projected.expect(
        "no PROJECTION stamp, so `project_prepared_character_definitions` still \
         owns this body — the seat stopped asking one observer and kept the other",
    );
    assert_eq!(persona.generation, generation);
    assert_eq!(projected.generation, generation);
    assert_eq!(projected.id, "duelist");

    // ⭐ **the proof.** No hot reload, no `RecharacterizeBody`: a template system
    // that still had work would do it here.
    app.update();
    let (persona_after, projected_after, model_after) = read(&mut app);
    assert_eq!(
        persona_after.as_ref(),
        Some(&persona),
        "a template observer rewrote the gameplay baseline on an untouched body"
    );
    assert_eq!(
        projected_after.as_ref(),
        Some(&projected),
        "a template observer rewrote the projection stamp on an untouched body"
    );
    assert_eq!(
        model_after, model,
        "the motion model changed on the pass AFTER construction, which is the \
         two-phase body this whole row exists to remove"
    );
}

/// **A CPU SEAT RESOLVES A PUBLISHED POLICY BEFORE AN ARCHETYPE KEY.**
///
/// ⭐ **the direction Jon's second redirect (P4) asks for.** A match's public API
/// is *character + controller + team*, and the controller half was resolved
/// through `CharacterRoster` — an enemy ARCHETYPE table — so a seat asking for a
/// policy received one by way of a body definition, and Smash was not yet
/// proving the controller architecture it advertises.
///
/// Two terms: a published policy WINS over an archetype key of the same name,
/// and an archetype-only key still resolves. The first alone would pass if the
/// registry were simply consulted; the second is what says the legacy road is
/// still open while presets are migrated.
///
/// ⛔⛔ **AND A THIRD, BECAUSE THE FIRST TWO PASSED WHILE THE LOOKUP WAS DEAD**
/// (ledger D87). This built its registry with `from_catalog_for_test`, which
/// copied the catalog map VERBATIM and so keyed policies by BARE name;
/// production assembly keys them `provider::name`. The seat asked for a bare
/// key, matched the fixture, and matched NOTHING in any real game — every CPU
/// seat fell through to the archetype table, including Smash's, whose `duelist`
/// profile was published for exactly this and never once read.
///
/// The fixture is repaired rather than the assertion weakened: it namespaces
/// like assembly does, and the seat resolves the reference in the CHARACTER's
/// provider. The third term is the poison — a bare-keyed registry must NOT
/// answer, because that is the shape that lied.
#[test]
fn a_cpu_seat_prefers_a_published_policy_over_an_archetype_of_the_same_name() {
    use ambition_characters::actor::character_catalog::{BrainProfileRegistry, CharacterCatalog};

    const CATALOG: &str = r#"(
        autonomous_profiles: {
            "medium_striker": (
                template: StandStill,
                aggro_radius: 1.0,
                attack_range: 2.0,
            ),
        },
        brain_presets: {},
        action_set_presets: {},
        characters: {},
    )"#;
    const PROVIDER: &str = "fixture_game";
    let profiles = BrainProfileRegistry::from_catalog_for_test(
        PROVIDER,
        &CharacterCatalog::from_data(
            ambition_characters::actor::character_catalog::parse_catalog(CATALOG),
        ),
    );
    let archetypes = crate::features::enemies::fixture_roster_with_mount();

    let published = super::seat_brain_profile(
        "medium_striker",
        None,
        PROVIDER,
        Some(&profiles),
        &archetypes,
    )
    .expect(
        "a published policy of that name resolves — a BARE key reached a registry \
             that holds provider::name, which is the production shape",
    );
    assert_eq!(
        published.aggro_radius, 1.0,
        "the ARCHETYPE table answered a question a published controller policy \
         had already answered: {published:?}"
    );

    // ⚠ and the legacy road is still open, which is what makes the preference
    // above a preference rather than a replacement.
    let archetype_only =
        super::seat_brain_profile("combatant", None, PROVIDER, Some(&profiles), &archetypes)
            .expect("an archetype-only key still resolves while presets are migrating");
    assert_ne!(archetype_only.aggro_radius, 1.0);

    // ⛔⛔ **THE POISON.** A policy published by a DIFFERENT provider must not
    // answer this seat: that is the bare-key match that made the whole arm
    // vacuous, and it would also let one game's `duelist` silently drive
    // another's fighter.
    let foreign = super::seat_brain_profile(
        "medium_striker",
        None,
        "some_other_game",
        Some(&profiles),
        &archetypes,
    )
    .expect("the archetype table still answers, as it did before any policy was published");
    assert_ne!(
        foreign.aggro_radius, 1.0,
        "another provider's policy answered this seat, so the reference is not \
         being resolved in a provider at all"
    );
}

/// **A MATCH CANNOT HAND A BODY A VERB IT DOES NOT HAVE.**
///
/// ⭐ Jon's compositional acceptance test, in miniature: *"Forcing Puppy Slug
/// into Smash gives you Puppy Slug, even if Puppy Slug is a terrible fighter …
/// Jump → no jump if its body cannot jump."* The character here authors a
/// crawler's kit — it moves and it attacks — and the match declares the
/// platform-fighter floor including a jump and a double jump.
///
/// The seat gets the INTERSECTION. A ruleset may forbid; only a character may
/// grant.
///
/// ⛔ the poison is in the fixture, not in an extra assertion: the declared set
/// contains `jump`, so a regression to the old "stamp the match's set onto every
/// body" behaviour turns the first assertion green-side-up immediately. And the
/// second assertion is what stops it passing vacuously — `attack` is in BOTH,
/// so if the seat simply received nothing at all this would fail too.
#[test]
fn a_match_cannot_grant_a_verb_the_character_does_not_have() {
    let mut app = seating_app();
    app.register_character(
        CharacterDefinition::new("crawler", "Puppy Slug", "demo").with_abilities(
            ambition_platformer2d_core::AbilitySet {
                move_horizontal: true,
                attack: true,
                ..ambition_platformer2d_core::AbilitySet::NONE
            },
        ),
    );

    app.insert_resource(MatchParticipantRoster {
        participants: vec![cpu("crawler")],
        fighter_abilities: Some(ambition_platformer2d_core::AbilitySet {
            move_horizontal: true,
            jump: true,
            double_jump: true,
            dash: true,
            attack: true,
            ..ambition_platformer2d_core::AbilitySet::NONE
        }),
        ..Default::default()
    });

    finalize_and_update(&mut app);

    let kits: Vec<(bool, bool, bool, bool)> = {
        let world = app.world_mut();
        let mut q = world.query_filtered::<(
            &ambition_platformer2d_core::BodyAbilities,
            &ambition_platformer2d_core::AbilityBase,
        ), With<MatchSeat>>();
        q.iter(world)
            .map(|(abilities, base)| {
                (
                    abilities.abilities.jump,
                    abilities.abilities.dash,
                    abilities.abilities.attack,
                    base.abilities.jump,
                )
            })
            .collect()
    };
    assert_eq!(kits.len(), 1, "one seat, one kit");
    let (jump, dash, attack, base_jump) = kits[0];
    assert!(
        !jump && !base_jump && !dash,
        "the match handed a crawler a jump and a dash it never authored, which \
         is the engine manufacturing a capability: {kits:?}"
    );
    assert!(
        attack,
        "the seat received nothing at all, so the assertion above proves nothing \
         — `attack` is authored by the character AND declared by the match"
    );
}

/// **An authored EXPLICIT body box is the seated fighter's box.** (Y″5)
///
/// `BodySource::Explicit` had no consumer anywhere: a provider could author
/// half-extents and receive `SEAT_BODY_PX`, the placeholder constant, instead.
///
/// Consumed at SEATING rather than in the per-tick projection, deliberately — the
/// box is a construction fact, and a projection resizing a live body would be a
/// second geometry authority beside the transit seam (ADR 0024).
#[test]
fn a_seated_fighter_gets_the_body_box_its_definition_authors() {
    let mut app = seating_app();
    let mut chunky = CharacterDefinition::new("chunky", "Chunky", "demo");
    chunky.body = Some(crate::character_runtime::BodySource::Explicit {
        half_extents: (40.0, 60.0),
    });
    app.register_character(chunky);
    app.insert_resource(MatchParticipantRoster {
        participants: vec![cpu("chunky")],
        ..Default::default()
    });

    finalize_and_update(&mut app);

    let world = app.world_mut();
    let mut bodies = world.query::<(
        &ambition_characters::actor::WornCharacter,
        &ambition_platformer2d_shared_tangle::body::BodyKinematics,
    )>();
    let (_, kin) = bodies
        .iter(world)
        .next()
        .expect("the fighter must be seated at all");
    assert_eq!(
        (kin.size.x, kin.size.y),
        (80.0, 120.0),
        "the seated fighter got the placeholder box instead of the one its \
         definition authored — a provider can author an explicit body and receive \
         some other size"
    );
}

/// **A participant that CANNOT seat holds the latch open.** (H5's missing half)
///
/// `MatchSeated` is what the versus countdown waits on, and the predicate test
/// beside that countdown records why the real scenario had never been driven:
/// *"reaching the real case needs a participant that CANNOT seat, which the
/// fixture cannot yet produce"*. Poking the resource does not work, because
/// seating re-asserts it every tick once the roster is complete.
///
/// It can be produced, and cheaply: name a character nothing registered.
/// `seat_character` returns `None` for an unregistered id, so the seat count
/// never reaches the participant count and the latch stays open — which is the
/// state the countdown must refuse to advance through, and the state a fighter
/// arriving late would otherwise join a live round from.
///
/// ⚠ **the second assertion was REVERSED when activation became a transaction**,
/// and the reversal is the finding, not a weakened test. It used to read *"the
/// seatable participant should still have got its body — an incomplete roster
/// must not also mean nobody is staged"*. That is exactly what an incomplete
/// roster must now mean. A body staged for a match that never activates is an
/// ORPHAN: the latch never closes, so no ruleset ever owns it, and it stands in
/// the arena at full health for the rest of the session. Worse, the seat-0
/// ADOPTION path reached that state by rewriting the primary player's health,
/// body size and pose THROUGH THE QUERY — so a roster that could never complete
/// left the player permanently re-pooled and resized for a match that never
/// began. Resolve-then-commit means an unsatisfiable roster leaves the world
/// byte-for-byte as it found it.
#[test]
fn a_roster_with_an_unseatable_participant_never_latches() {
    let mut app = seating_app();
    app.register_character(CharacterDefinition::new("present", "Present", "demo"));
    app.insert_resource(MatchParticipantRoster {
        // One that can seat, one that cannot. The mix is the point: a latch that
        // closed on "any seat succeeded" would report a one-fighter match as
        // ready, which is the defect the atomic condition was written for.
        participants: vec![cpu("present"), cpu("never_registered")],
        ..Default::default()
    });

    // Several ticks, because seating RETRIES: the latch must stay open for as
    // long as the roster is incomplete, not merely on the first tick.
    for _ in 0..5 {
        finalize_and_update(&mut app);
    }

    assert!(
        app.world().get_resource::<ActiveMatch>().is_none(),
        "the match ACTIVATED with a seat still empty, so the countdown is free to \
         run and the round goes live one fighter short"
    );
    let world = app.world_mut();
    let mut worn = world.query::<&ambition_characters::actor::WornCharacter>();
    assert_eq!(
        worn.iter(world).count(),
        0,
        "a seat was constructed for a roster that can never complete, so the \
         arena now holds an orphan fighter no ruleset owns — activation resolves \
         every seat before it builds any"
    );
}

/// **Activation names WHICH bodies are in the match, in seat order.** (Z′7)
///
/// The thing a `MatchSeated(bool)` could never say. A bool reported that seating
/// had FINISHED; nothing could then ask whether the live fighters are still the
/// set the match was built from, which is why a roster that disagrees with its
/// session after seating could only be REPORTED and not repaired (queue Y′9).
///
/// It also has to be published in ONE insert, on the tick the last seat fills.
/// **A PROPOSED roster does not seat, and an activated one does.**
///
/// The activation authority `status.md` calls for: *"validate every participant,
/// activate the roster atomically, publish it, start the countdown from that.
/// Until it exists, a roster that has ALREADY seated and then disagrees with the
/// session is reported rather than repaired."*
///
/// A route builds its roster from live device discovery on entry; the rollback
/// session freezes its topology afterwards. Seating used to run from whatever was
/// published, so the bodies existed before anything could ask whether the session
/// agreed — and it could not be fixed by reordering, because seating runs on the
/// SIM schedule and a route's reconciliation runs in `Update`
/// (`PreUpdate` → Fixed → `Update`). Refusing is the only way to be first.
///
/// ⚠ **the two halves are ONE test on purpose.** A refusal test alone passes
/// against a seating pass that is simply broken; an activation test alone passes
/// against one that ignores the lifecycle. What has to be true is that the same
/// app, the same roster and the same participants seat when and only when the
/// roster has been agreed to.
#[test]
fn a_proposed_roster_waits_and_the_same_roster_activated_seats() {
    let mut app = seating_app();
    app.register_character(CharacterDefinition::new("alpha", "Alpha", "demo"));
    app.register_character(CharacterDefinition::new("beta", "Beta", "demo"));
    app.insert_resource(MatchParticipantRoster {
        participants: vec![cpu("alpha"), cpu("beta")],
        seating: crate::character_runtime::RosterSeating::Proposed,
        ..Default::default()
    });

    finalize_and_update(&mut app);

    assert_eq!(
        app.world_mut()
            .query::<&MatchSeat>()
            .iter(app.world())
            .count(),
        0,
        "a PROPOSED roster seated bodies. Nothing has agreed to this match yet, \
         and once the bodies exist the disagreement can only be reported"
    );
    assert!(
        app.world().get_resource::<ActiveMatch>().is_none(),
        "a match activated from a roster nobody agreed to seat"
    );
    // ⭐ and NOT because the composition cannot seat it — that would be a
    // different refusal with a different meaning, and a consumer shows it to a
    // human. A proposal is a wait, not a problem.
    assert!(
        app.world()
            .get_resource::<crate::character_runtime::MatchPreparationProblems>()
            .is_none(),
        "waiting for activation was reported as an unsatisfiable roster, which \
         would put a refusal on screen on every ordinary route entry"
    );

    // Same app, same participants — agreed to now.
    app.world_mut()
        .resource_mut::<MatchParticipantRoster>()
        .activate(Some(7));
    finalize_and_update(&mut app);

    assert_eq!(
        app.world_mut()
            .query::<&MatchSeat>()
            .iter(app.world())
            .count(),
        2,
        "an ACTIVATED roster did not seat, so the refusal above proved nothing"
    );
    let active = app
        .world()
        .get_resource::<ActiveMatch>()
        .expect("an activated roster that seats fully must activate the match");
    assert_eq!(active.seat_topology(), Some(7));
}

/// A roster that seats over several ticks — the ordinary case, since seating
/// retries — must not activate partially: the countdown gates on this, and a
/// half-built match releasing it is the defect the gate exists for.
#[test]
fn activation_publishes_every_seated_body_in_seat_order() {
    let mut app = seating_app();
    app.register_character(CharacterDefinition::new("alpha", "Alpha", "demo"));
    app.register_character(CharacterDefinition::new("beta", "Beta", "demo"));
    app.insert_resource(MatchParticipantRoster {
        participants: vec![cpu("alpha"), cpu("beta")],
        seating: crate::character_runtime::RosterSeating::activated_at(7),
        ..Default::default()
    });

    finalize_and_update(&mut app);

    let active = app
        .world()
        .get_resource::<ActiveMatch>()
        .expect("a fully seated roster must activate")
        .clone();
    assert_eq!(
        active.seats(),
        2,
        "activation must count every seat, not merely report that seating ended"
    );
    assert_eq!(
        active.seat_topology(),
        Some(7),
        "the activation records WHICH frozen topology decided its seating, or a \
         later disagreement has nothing to compare against"
    );

    // Seat order, checked through the bodies themselves — which is now the ONLY
    // place it lives. `match_participants` derives the cast from `MatchSeat`
    // rather than from a remembered list, so this reads what production reads.
    let world = app.world_mut();
    let worn: Vec<String> = world
        .run_system_cached(
            |seated: Query<(Entity, &MatchSeat)>,
             worn: Query<&ambition_characters::actor::WornCharacter>| {
                crate::character_runtime::match_participants(&seated)
                    .into_iter()
                    .map(|body| {
                        worn.get(body)
                            .expect("every seated participant wears its character")
                            .id()
                            .to_string()
                    })
                    .collect::<Vec<_>>()
            },
        )
        .expect("the participant query runs");
    assert_eq!(
        worn,
        vec!["alpha".to_string(), "beta".to_string()],
        "participants are in SEAT order; a set that arrives in spawn order makes \
         `participants[i]` mean nothing"
    );
}

/// **A seated fighter carries the mass its character authored.** (Y″4)
///
/// `Vitals.mass` was reported as authored-and-unconsumed, and the field had no
/// readers — but the CONCEPT did. `features::Mass` is rollback-registered and
/// drives the mount pair's mass-weighted centre of gravity (ADR 0020): a heavy
/// mount keeps the COG near itself, so the lighter rider orbits it on a gravity
/// flip. It was fed from the ROSTER archetype and never from the character
/// definition, which made `Vitals.mass` a second declaration of a fact only the
/// roster could state.
///
/// So this is not "a dead field given a purpose"; it is a disconnected authority
/// connected to the one it was always describing.
#[test]
fn a_seated_fighter_carries_its_authored_mass() {
    let mut app = seating_app();
    let mut heavy = CharacterDefinition::new("anvil", "Anvil", "demo");
    heavy.vitals = crate::character_runtime::Vitals {
        max_health: Some(40),
        mass: Some(6.5),
        knockback_weight: None,
    };
    app.register_character(heavy);
    app.insert_resource(MatchParticipantRoster {
        participants: vec![cpu("anvil")],
        ..Default::default()
    });

    finalize_and_update(&mut app);

    let world = app.world_mut();
    let mut bodies = world.query::<(
        &ambition_characters::actor::WornCharacter,
        &crate::features::Mass,
    )>();
    let (_, mass) = bodies
        .iter(world)
        .next()
        .expect("the fighter must be seated at all");
    assert_eq!(
        mass.0, 6.5,
        "the seated fighter got the default mass instead of the one its \
         definition authored, so a mounted pair computes its centre of gravity \
         from a number nobody wrote"
    );
}

/// **A seated fighter whose character takes the HOST kit still gets one.**
/// (Phase B remainder)
///
/// `PreparedKit::HostCode` is the one case a per-character value cannot hold: the
/// host's code-side kit is built from the BODY's own `AbilitySet`. While the
/// projection was the writer for seated bodies it could not build that — it has
/// no body abilities — so a seated fighter resolving to `HostCode` got nothing.
///
/// Phase B routed seated bodies through `apply_worn_character_gameplay`, which
/// DOES have the abilities. This asserts that actually closed the hole rather than
/// merely making it plausible: a registered character with no authored action set
/// and no catalog row resolves to `HostCode`, and the seated body must still come
/// out able to act.
#[test]
fn a_seated_fighter_on_the_host_kit_is_not_left_empty_handed() {
    let mut app = seating_app();
    // No authored action set, and the empty catalog has no row for it — the two
    // conditions that make `finalize_character` choose `HostCode`.
    app.register_character(CharacterDefinition::new("drifter", "Drifter", "demo"));
    app.insert_resource(MatchParticipantRoster {
        participants: vec![cpu("drifter")],
        ..Default::default()
    });

    finalize_and_update(&mut app);
    finalize_and_update(&mut app);

    let world = app.world_mut();
    let mut bodies = world.query::<(
        &ambition_characters::actor::WornCharacter,
        &ambition_characters::brain::ActionSet,
        &crate::combat::moveset::ActorMoveset,
    )>();
    let (_, action_set, moveset) = bodies
        .iter(world)
        .next()
        .expect("the fighter must be seated at all");
    assert!(
        action_set.melee.is_some(),
        "a seated fighter on the host kit has no melee, so its brain will never \
         press attack — the kit is built from the BODY's abilities and only the \
         persona derive can see those"
    );
    assert!(
        !moveset.0.moves.is_empty(),
        "and it has no moves to play even if it did press: an action set without \
         a timeline is a capability the body cannot perform"
    );
}

/// **A seated body carries every column the persona writer requires.**
///
/// This is the guard that would have caught H1, and it is deliberately a
/// STRUCTURAL assertion rather than a behavioural one: it does not ask whether
/// the fighter got the right kit, it asks whether it is even VISIBLE to the
/// single writer that hands out kits.
///
/// `apply_worn_character_gameplay` takes `Name`, `ActionSet`, `ActorMoveset`,
/// `IdentityKit`, `BodyAbilities` and `MotionModel` as REQUIRED, non-`Option`
/// query columns. A body missing any one of them silently stops matching — it
/// wears a character and derives nothing from it, with no error anywhere. That is
/// how a seated fighter came to need a second kit writer, and how a plan step came
/// to say "delete the seating placeholders" (they are these columns).
///
/// ⚠ this list is coupled to that system's query. If a column is added there and
/// not here, this goes red instead of a fighter silently losing its persona.
#[test]
fn a_seated_body_matches_every_column_the_persona_writer_requires() {
    let mut app = seating_app();
    app.register_character(CharacterDefinition::new("recruit", "Recruit", "demo"));
    app.insert_resource(MatchParticipantRoster {
        participants: vec![cpu("recruit")],
        ..Default::default()
    });

    finalize_and_update(&mut app);

    let world = app.world_mut();
    let mut visible_to_the_derive = world.query_filtered::<Entity, (
        With<ambition_characters::actor::WornCharacter>,
        With<Name>,
        With<ambition_characters::brain::ActionSet>,
        With<crate::combat::moveset::ActorMoveset>,
        With<ambition_characters::brain::action_set::IdentityKit>,
        With<crate::actor::BodyAbilities>,
        With<crate::features::MotionModel>,
    )>();
    assert_eq!(
        visible_to_the_derive.iter(world).count(),
        1,
        "the seated fighter is missing a column `apply_worn_character_gameplay` \
         requires, so it does not match the ONE writer that turns a worn character \
         into a persona — it will wear a character and derive nothing from it, \
         silently"
    );
}

/// **A CPU seat naming a brain the roster does not have is REFUSED.**
///
/// `spec_for_brain` falls back to the `combatant` row for an unknown key, and
/// its own doc says a provider that misspells an archetype "gets a generic enemy
/// instead of an error". For a placement that is defensible. For a MATCH SEAT it
/// is not: the fighter the roster asked for is not the fighter that arrives, the
/// match activates anyway, and the symptom is a duelist standing still — which is
/// indistinguishable from a brain that was never installed. The smash demo spent
/// an hour on exactly that, and needed a diagram to see it.
///
/// `resolve_initial_brain` already holds this line for placement overrides
/// ("never a silent fall back to the default"); this is the same class of mistake
/// on the other path.
///
/// ⚠ built as a RELEASE-mode assertion, because the engine's refusal is the
/// `return` — the `debug_assert` beside it is the diagnostic, and a test that
/// only ran in debug would prove the message rather than the behaviour.
#[cfg(not(debug_assertions))]
#[test]
fn a_seat_naming_an_unknown_brain_profile_is_not_seated() {
    let mut app = seating_app();
    app.register_character(CharacterDefinition::new("mary_o", "Mary-O", "mary_o_demo"));
    app.insert_resource(MatchParticipantRoster {
        participants: vec![
            MatchParticipant::new("mary_o").driven_by(ControllerBinding::Cpu {
                brain_profile: Some("no_such_archetype".into()),
            }),
            cpu("mary_o"),
        ],
        ..Default::default()
    });

    for _ in 0..5 {
        finalize_and_update(&mut app);
    }

    assert!(
        app.world().get_resource::<ActiveMatch>().is_none(),
        "a match activated with a fighter the roster could not describe"
    );
    let world = app.world_mut();
    let mut worn = world.query::<&ambition_characters::actor::WornCharacter>();
    assert_eq!(
        worn.iter(world).count(),
        0,
        "a body was built for a seat whose brain profile does not exist, so the \
         match holds a generic enemy wearing a fighter's name"
    );
}

/// **Activation is a TRANSACTION: resolve every seat, then build every seat.**
/// (S2 / AA2's lifecycle half)
///
/// Seating used to resolve and construct one seat at a time. The defect that
/// makes this a transaction rather than a tidier loop is not "the latch is
/// atomic and the bodies are not" — it is that the ADOPTION path writes the
/// primary player's health, body size and pose THROUGH THE QUERY, immediately,
/// not through deferred `Commands`. So seat 0 could be re-pooled, resized and
/// teleported to its mark on a tick where seat 1 then failed to resolve, and the
/// player wore that half-applied match state for as many ticks as the roster took
/// to complete — forever, for a roster that can never complete.
mod activation_transaction {
    use super::*;

    /// The player body these tests watch, with values chosen so that adoption
    /// CANNOT be mistaken for "nothing happened": a pool the character's
    /// baseline would replace, and a position seating would move.
    const PLAYER_POOL: i32 = 5;

    fn spawn_player(app: &mut App) -> Entity {
        app.world_mut()
            .spawn((
                crate::avatar::PlayerSimulationBundle::from_scratch(
                    crate::avatar::primary_player_scratch(
                        Vec2::new(0.0, 0.0),
                        ambition_platformer2d_core::AbilitySet::default(),
                    ),
                    ambition_characters::actor::Health::new(PLAYER_POOL),
                ),
                ambition_characters::actor::WornCharacter::new("mary_o"),
            ))
            .id()
    }

    fn body_state(
        app: &App,
        body: Entity,
    ) -> (i32, i32, f32, f32, ambition_platformer2d_core::Vec2) {
        let health = app
            .world()
            .get::<ambition_characters::actor::BodyHealth>(body)
            .expect("the player body has health");
        let kin = app
            .world()
            .get::<ambition_platformer2d_shared_tangle::body::BodyKinematics>(body)
            .expect("the player body has kinematics");
        (
            health.current(),
            health.max(),
            kin.pos.x,
            kin.facing,
            kin.size,
        )
    }

    /// **A roster that cannot complete leaves the world exactly as it found it.**
    ///
    /// The seatable HUMAN seat is the one that matters: it is the one the old
    /// code mutated in place. Seat 1 names a character nothing registered, which
    /// is `seat_character`'s only failure mode, so the roster can never complete
    /// and the retry runs forever.
    #[test]
    fn an_unsatisfiable_seat_leaves_the_adopted_player_untouched() {
        let mut app = seating_app();
        app.register_character(CharacterDefinition::new("mary_o", "Mary-O", "mary_o_demo"));
        let player = spawn_player(&mut app);
        finalize_and_update(&mut app);
        let before = body_state(&app, player);

        app.insert_resource(MatchParticipantRoster {
            participants: vec![
                MatchParticipant::new("mary_o").driven_by(ControllerBinding::Human {
                    source: ambition_input::LocalInputSource::Pad(0),
                }),
                cpu("never_registered"),
            ],
            ..Default::default()
        });

        // Several ticks, because seating RETRIES. One tick would not distinguish
        // "did not mutate" from "has not run yet".
        for _ in 0..5 {
            finalize_and_update(&mut app);
        }

        assert_eq!(
            body_state(&app, player),
            before,
            "the adopted player was re-pooled, resized or teleported to its seat \
             for a match that never activated — the half-applied state the \
             resolve/commit split exists to prevent"
        );
        assert!(
            app.world().get::<MatchSeat>(player).is_none(),
            "the player wears a seat in a match that does not exist"
        );
        assert!(
            app.world().get_resource::<ActiveMatch>().is_none(),
            "the latch closed on an incomplete roster"
        );
        let world = app.world_mut();
        let mut worn = world.query::<&ambition_characters::actor::WornCharacter>();
        assert_eq!(
            worn.iter(world).count(),
            1,
            "a body was constructed for a roster that can never complete, so the \
             stage now holds an orphan fighter no ruleset owns"
        );
    }

    /// **When every seat CAN be satisfied, they all arrive on one tick.**
    ///
    /// The counterpart assertion, and the one that would catch a "fix" that
    /// simply refused to build anything.
    ///
    /// ⚠ **the mix is still the point, for a different reason.** It used to be
    /// that a spawned seat and an ADOPTED seat took different construction
    /// paths and the seam between them could tear. There is one path now, so
    /// what this pins is that a local-input seat and a brain seat still land in
    /// the SAME command flush — a regression that staggered them would put a
    /// fighter on the stage a tick before its opponent, which is a head start.
    #[test]
    fn every_seat_is_constructed_on_the_same_tick() {
        let mut app = seating_app();
        app.register_character(CharacterDefinition::new("mary_o", "Mary-O", "mary_o_demo"));
        app.register_character(CharacterDefinition::new("sanic", "Sanic", "sanic_demo"));
        finalize_and_update(&mut app);

        app.insert_resource(MatchParticipantRoster {
            participants: vec![
                MatchParticipant::new("mary_o").driven_by(ControllerBinding::Human {
                    source: ambition_input::LocalInputSource::Pad(0),
                }),
                cpu("sanic"),
            ],
            ..Default::default()
        });

        finalize_and_update(&mut app);

        let world = app.world_mut();
        let mut seats = world.query::<&MatchSeat>();
        let mut indices: Vec<usize> = seats.iter(world).map(|seat| seat.0).collect();
        indices.sort();
        assert_eq!(
            indices,
            vec![0, 1],
            "the seats did not all appear on the tick the match activated"
        );
        let active = app
            .world()
            .get_resource::<ActiveMatch>()
            .expect("the latch closed with the cast");
        assert_eq!(
            active.seats(),
            2,
            "the activation counted fewer seats than it built — the adopted seat \
             was not recorded in the pass that built it"
        );
    }
}

/// **The refusal is a fact in the world, not a debug-only panic.** (API (g))
///
/// Seating declined an unresolvable brain profile through `debug_assert!`, which
/// does nothing in a release build — so the release behaviour was "return
/// quietly and let the match activate around the hole", which is the exact bug
/// the guard was written for. A guard whose build configuration decides whether
/// it guards is not one.
///
/// This asserts the published `MatchSeatingRefused`, which is present in every
/// build, and asserts it NAMES the profile so a player-facing message can.
#[test]
fn an_unseatable_brain_profile_publishes_a_refusal_that_names_it() {
    let mut app = seating_app();
    app.register_character(CharacterDefinition::new("fighter", "Fighter", "demo"));
    app.insert_resource(MatchParticipantRoster {
        // Both CPU: `MatchParticipant::new` defaults to a HUMAN seat, whose arm
        // returns when there is no primary player to adopt — a different refusal
        // than the one under test, reached first.
        participants: vec![
            cpu("fighter"),
            MatchParticipant::new("fighter").driven_by(ControllerBinding::Cpu {
                brain_profile: Some("no_such_archetype".into()),
            }),
        ],
        ..Default::default()
    });
    // `finalize_and_update`, not `update`: a registered character is not a
    // PREPARED one, and an unprepared seat refuses first — for a different
    // reason than the one under test.
    finalize_and_update(&mut app);

    let refusal = app
        .world()
        .get_resource::<crate::character_runtime::MatchPreparationProblems>()
        .expect("seating refused this roster and has to say so in every build");
    assert_eq!(refusal.problems.len(), 1, "{:?}", refusal.problems);
    assert_eq!(refusal.problems[0].seat, 1);
    assert!(
        refusal.problems[0].detail.contains("no_such_archetype"),
        "the refusal has to name what was asked for: {}",
        refusal.problems[0].detail
    );
    assert!(
        app.world()
            .get_resource::<crate::character_runtime::ActiveMatch>()
            .is_none(),
        "the match latched anyway, which is the partial activation the resolve \
         pass exists to prevent"
    );
}

/// **ONE DEFINITION, TWO INDEPENDENT INSTANCES** — the invariant D73 rests on.
///
/// Jon, 2026-08-10: *"A character is a reusable authored template, not a
/// singleton person … `spawn Fretjaw` twice → two independent Fretjaw actors."*
/// A mirror match is that sentence expressed in the one construction path a
/// match already uses, so it is where the invariant is cheapest to pin.
///
/// ⚠ **this passes today and is still worth writing.** The campaign moves NPC,
/// enemy, encounter and summon construction onto this path, and the failure it
/// guards against is not a uniqueness check somebody adds deliberately — it is a
/// shared or memoized per-character value leaking between instances, which reads
/// as "both Fretjaws lost health" and looks like a damage bug.
///
/// ⭐ the codebase already knew about mirror matches from the other end:
/// [`MatchSeat`]'s doc says *"the worn character id collides in a mirror
/// match"*, which is why seats are the identity rules use. This asserts the
/// complementary half — that the collision is legal.
#[test]
fn one_character_definition_seats_two_independent_fighters() {
    let mut app = seating_app();
    let mut fretjaw = CharacterDefinition::new("fretjaw", "Fretjaw", "demo");
    fretjaw.vitals = crate::character_runtime::Vitals {
        max_health: Some(40),
        mass: Some(1.0),
        knockback_weight: None,
    };
    app.register_character(fretjaw);
    app.insert_resource(MatchParticipantRoster {
        participants: vec![cpu("fretjaw"), cpu("fretjaw")],
        ..Default::default()
    });

    finalize_and_update(&mut app);

    let world = app.world_mut();
    let mut q = world.query::<(
        Entity,
        &ambition_characters::actor::WornCharacter,
        &MatchSeat,
        &ambition_platformer2d_shared_tangle::body::BodyKinematics,
    )>();
    let mut seated: Vec<(Entity, String, usize, f32)> = q
        .iter(world)
        .map(|(entity, worn, seat, kin)| (entity, worn.id().to_string(), seat.0, kin.pos.x))
        .collect();
    seated.sort_by_key(|(_, _, seat, _)| *seat);

    assert_eq!(
        seated.len(),
        2,
        "a mirror match is two bodies of one character: {seated:?}"
    );
    // SAME character identity …
    assert_eq!(seated[0].1, "fretjaw");
    assert_eq!(seated[1].1, "fretjaw");
    // … and DIFFERENT runtime identity, in every sense a rules layer uses.
    assert_ne!(
        seated[0].0, seated[1].0,
        "two instances of one character are two entities"
    );
    assert_ne!(seated[0].2, seated[1].2, "each instance holds its own seat");
    assert_ne!(
        seated[0].3, seated[1].3,
        "two Fretjaws stand in two places, not one"
    );

    // ⭐ **the assertion that catches a SHARED per-character value**, which is the
    // real risk when construction is unified: hurt one and the other must not
    // feel it. A memoized definition-keyed health would pass every line above.
    let (first, second) = (seated[0].0, seated[1].0);
    let before = app
        .world()
        .get::<ambition_characters::actor::BodyHealth>(second)
        .expect("a seated fighter has health")
        .current();
    app.world_mut()
        .get_mut::<ambition_characters::actor::BodyHealth>(first)
        .expect("a seated fighter has health")
        .damage(7);
    let hurt = app
        .world()
        .get::<ambition_characters::actor::BodyHealth>(first)
        .expect("a seated fighter has health")
        .current();
    let untouched = app
        .world()
        .get::<ambition_characters::actor::BodyHealth>(second)
        .expect("a seated fighter has health")
        .current();
    assert!(hurt < before, "the struck Fretjaw lost health: {hurt}");
    assert_eq!(
        untouched, before,
        "the OTHER Fretjaw shares a definition, not a health pool"
    );
}

/// **A character can author what happens when it dies** — D73 phase 1.
///
/// Until this landed, `CombatCapabilities` had exactly ONE producer in the
/// workspace (`ArchetypeSpecExt::combat_capabilities`), so death traits were a
/// thing only an archetype could say. A registered character could not declare
/// that it splits, explodes, or refuses to die, and a seated fighter therefore
/// had no death traits at all whatever it was.
///
/// ⚠ **and absence RETRACTS**, which is the half worth testing: a persona that
/// authors nothing must leave a body with no traits rather than inheriting the
/// previous character's. That rule already governs health, mass and the feel
/// marker; death traits now ride it too.
#[test]
fn a_character_authors_its_own_death_traits_and_absence_retracts_them() {
    let mut app = seating_app();
    let mut sandbag = CharacterDefinition::new("sandbag", "Sandbag", "demo");
    sandbag.death_traits = Some(ambition_characters::actor::CharacterDeathTraits {
        never_dies: true,
        ..Default::default()
    });
    app.register_character(sandbag);
    app.register_character(CharacterDefinition::new("duelist", "Duelist", "demo"));
    app.insert_resource(MatchParticipantRoster {
        participants: vec![cpu("sandbag"), cpu("duelist")],
        ..Default::default()
    });

    finalize_and_update(&mut app);

    let world = app.world_mut();
    let mut q = world.query::<(
        Entity,
        &ambition_characters::actor::WornCharacter,
        Option<&crate::combat::CombatCapabilities>,
    )>();
    let mut seen: Vec<(Entity, String, bool)> = q
        .iter(world)
        .map(|(entity, worn, caps)| {
            (
                entity,
                worn.id().to_string(),
                caps.is_some_and(|caps| caps.never_dies),
            )
        })
        .collect();
    seen.sort_by(|a, b| a.1.cmp(&b.1));
    assert_eq!(seen.len(), 2, "{seen:?}");
    assert!(
        seen[1].2,
        "the sandbag authored `never_dies` and must carry it: {seen:?}"
    );
    assert!(
        !seen[0].2,
        "the duelist authored no death traits and must have none: {seen:?}"
    );

    // ⭐ **THE RETRACTION.** Re-wear the sandbag's body as the duelist: the
    // trait must LEAVE. A derive that only ever inserts passes every assertion
    // above and makes a character swap permanently immortalising.
    let sandbag_body = seen[1].0;
    *app.world_mut()
        .get_mut::<ambition_characters::actor::WornCharacter>(sandbag_body)
        .expect("the seated body wears its character") =
        ambition_characters::actor::WornCharacter::new("duelist");
    // ⭐ **AND ASK FOR IT.** Writing the identity stopped rebuilding the body
    // (Jon's second redirect, P0): a re-wear is an explicit request now, the way
    // Mary-O's powerup already made it.
    app.world_mut()
        .entity_mut(sandbag_body)
        .insert(ambition_characters::actor::RecharacterizeBody);
    finalize_and_update(&mut app);
    // ⛔ **PRESENT AND DEFAULT, not absent** — and the difference cost sixteen
    // integration tests. `CombatCapabilities` is a required member of
    // `ActorClusterQueryData`, so a body without it drops out of the actor
    // cluster query and stops being simulated as an actor: versus reported
    // fighters swinging twelve times into an opponent stuck on full health.
    // Retraction means "claims nothing", which is the default value, never the
    // missing component.
    let after = app
        .world()
        .get::<crate::combat::CombatCapabilities>(sandbag_body)
        .cloned();
    assert!(
        after.is_some(),
        "the component must SURVIVE a retraction — removing it takes the body \
         out of the actor cluster query entirely"
    );
    assert!(
        !after.expect("checked above").never_dies,
        "wearing a character that authors no death traits must retract the \
         previous character's, or a swap through the sandbag is a free immortality"
    );
}

/// **A character authors how hard it is to LAUNCH** — D73 phase 1, and the
/// second half of the knockback loop.
///
/// `CombatTuning::weight` divides the growth term (`scaled_knockback`), so it is
/// what makes a heavy fighter resist a launch a light one cannot. It could be
/// stated only on a roster ARCHETYPE until now, which meant two characters
/// seated from one archetype weighed the same and could not differ — in a
/// platform fighter, weight is per-character or the roster has no heavies.
#[test]
fn a_seated_fighter_carries_its_authored_knockback_weight() {
    let mut app = seating_app();
    let mut heavy = CharacterDefinition::new("heavy", "Heavy", "demo");
    heavy.vitals.knockback_weight = Some(1.8);
    app.register_character(heavy);
    // ⚠ the control: a character that authors NO weight is the REFERENCE BODY.
    app.register_character(CharacterDefinition::new("light", "Light", "demo"));
    // ⛔⛔ **the archetype's 1.4 is the POISON now, and it used to be the
    // expectation.** This test read `1.4` for the unauthored fighter, because a
    // seat was built out of an archetype and inherited its weight — so a
    // character that had never said anything about how hard it is to launch
    // weighed whatever creature the CPU's brain key happened to name. A seat is
    // built from its CHARACTER now (campaign P1.11), so the archetype below is
    // present precisely so that reading `1.4` again would mean the archetype had
    // crept back in.
    app.insert_resource(crate::features::CharacterRoster::from_ron(
        r#"{ "combatant": (
                max_health: 1, run_speed: 0.0, patrol_effort: 0.0, chase_effort: 0.0,
                aggro_radius: 0.0, attack_range: 0.0, contact_strength: 0.0,
                damage_amount: 0, brain_template: StandStill, move_style: Walk,
                attacks_player: false, body_contact_damage: false,
                weight: 1.4,
            ) }"#,
    ));
    app.insert_resource(MatchParticipantRoster {
        participants: vec![cpu("heavy"), cpu("light")],
        ..Default::default()
    });

    finalize_and_update(&mut app);

    let world = app.world_mut();
    let mut q = world.query::<(
        &ambition_characters::actor::WornCharacter,
        &crate::combat::CombatTuning,
    )>();
    let mut seen: Vec<(String, f32)> = q
        .iter(world)
        .map(|(worn, tuning)| (worn.id().to_string(), tuning.weight))
        .collect();
    seen.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(seen.len(), 2, "{seen:?}");
    assert_eq!(
        seen[0].1, 1.8,
        "the heavy authored 1.8 and the seed must carry it: {seen:?}"
    );
    assert_eq!(
        seen[1].1, 1.0,
        "the light authored no weight and must be the reference body. 1.4 here \
         is the roster archetype's number, which means a fighter's body is being \
         built out of whichever creature its brain key names again: {seen:?}"
    );
}

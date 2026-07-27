//! C4 slice 1: a roster of CPU participants becomes bodies that can fight.

use super::*;
use crate::character_runtime::{
    CharacterDefinition, CharacterDefinitionAppExt, ControllerBinding, MatchParticipant,
};

fn cpu(character: &str) -> MatchParticipant {
    MatchParticipant::new(character).driven_by(ControllerBinding::Cpu {
        brain_profile: Some("medium_striker".into()),
    })
}

fn seating_app() -> App {
    let mut app = App::new();
    app.init_resource::<MatchSeated>();
    app.init_resource::<PreparedCharacterRegistry>();
    app.insert_resource(ambition_characters::actor::character_catalog::CharacterCatalog::empty());
    app.init_resource::<crate::features::CharacterRoster>();
    // A room whose authored spawn is the stage centre.
    let world = ambition_engine_core::World::new(
        "Arena",
        Vec2::new(960.0, 540.0),
        Vec2::new(480.0, 400.0),
        vec![ambition_engine_core::Block::solid(
            "floor",
            Vec2::new(0.0, 440.0),
            Vec2::new(960.0, 100.0),
        )],
    );
    ambition_platformer_primitives::lifecycle::insert_session_world_component(
        app.world_mut(),
        ambition_engine_core::RoomGeometry(world),
    );
    app.add_systems(Update, seat_match_participants);
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
    });

    app.update();

    let world = app.world_mut();
    let mut q = world.query::<(
        &ambition_characters::actor::WornCharacter,
        &ambition_platformer_primitives::body::BodyKinematics,
        &crate::combat::components::ActorFaction,
    )>();
    let mut seated: Vec<(String, f32, f32, crate::combat::components::ActorFaction)> = q
        .iter(world)
        .map(|(worn, kin, faction)| (worn.id().to_string(), kin.pos.x, kin.facing, *faction))
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

    // Opposing factions, or `effective_faction` refuses every strike between them
    // and the two bodies stand and stare.
    assert_ne!(
        seated[0].3, seated[1].3,
        "seated fighters must be on opposing sides or no strike between them resolves"
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
    });

    for _ in 0..10 {
        app.update();
    }

    let world = app.world_mut();
    let mut q = world.query::<&ambition_characters::actor::WornCharacter>();
    assert_eq!(
        q.iter(world).count(),
        1,
        "ten ticks produced more than one body: seating is not latched"
    );
}

/// An unregistered character seats nothing, and does not latch the match.
///
/// Quiet by design: the load ledger already reports unknown tokens, and a second
/// reporter of one fact is how a log becomes unreadable. What matters is that it
/// does not produce a body wearing a character nothing can describe.
#[test]
fn an_unregistered_participant_is_not_seated() {
    let mut app = seating_app();
    app.insert_resource(MatchParticipantRoster {
        participants: vec![cpu("nobody_registered_this")],
    });
    app.update();

    {
        let world = app.world_mut();
        let mut q = world.query::<&ambition_characters::actor::WornCharacter>();
        assert_eq!(q.iter(world).count(), 0);
    }
    assert!(
        !app.world().resource::<MatchSeated>().0,
        "a match that seated nobody must not be marked seated — the roster may \
         become seatable once its characters register"
    );
}

/// **A human seat ADOPTS the player body; it does not spawn a second one.**
///
/// This is the bug the versus stage shipped with for an hour: the session spawns
/// a primary player wearing the starting character, seating spawned a fighter
/// wearing the same character, and the arena held two of them. The old test here
/// asserted a human seat produced NO body, which was true and useless — the
/// defect was the body it did not account for.
#[test]
fn a_human_seat_adopts_the_existing_player_body_instead_of_duplicating_it() {
    let mut app = seating_app();
    app.register_character(CharacterDefinition::new("mary_o", "Mary-O", "mary_o_demo"));
    // The REAL player bundle. A hand-rolled body without the movement clusters
    // does not match seating's query, so adoption silently skips and the body
    // simply never moves — which the first version of this fixture could not tell
    // apart from working, because the seat count was right for the wrong reason.
    let player = app
        .world_mut()
        .spawn((
            crate::avatar::PlayerSimulationBundle::from_scratch(
                crate::avatar::primary_player_scratch(
                    Vec2::new(0.0, 0.0),
                    ambition_engine_core::AbilitySet::default(),
                ),
                ambition_characters::actor::Health::new(5),
            ),
            ambition_characters::actor::WornCharacter::new("mary_o"),
        ))
        .id();
    app.insert_resource(MatchParticipantRoster {
        participants: vec![
            MatchParticipant::new("mary_o").driven_by(ControllerBinding::Human { device_slot: 0 }),
            cpu("mary_o"),
        ],
    });

    app.update();

    let world = app.world_mut();
    let mut worn = world.query::<&ambition_characters::actor::WornCharacter>();
    assert_eq!(
        worn.iter(world).count(),
        2,
        "one body per seat. A human seat that spawns instead of adopting leaves \
         the player's own body beside a copy of itself — which is what the versus \
         arena looked like before this existed"
    );

    // The adopted body MOVED to its seat. A human seat left at the session spawn
    // is standing wherever the room put it rather than where the match wants it,
    // which is only invisible while the two happen to coincide.
    let kin = app
        .world()
        .get::<ambition_platformer_primitives::body::BodyKinematics>(player)
        .expect("the player body survives adoption");
    assert_ne!(
        kin.pos.x, 0.0,
        "the human seat was not moved to its side of the stage"
    );
    assert_eq!(kin.facing, 1.0, "the left seat looks right");
}

/// A human seat whose character disagrees with the body's is NOT re-dressed.
///
/// Seating places fighters; `WornCharacter` decides who they are. A stage that
/// wants a different fighter says so in its `StartingCharacter`, and silently
/// re-dressing the player body here would make two authorities for one fact.
#[test]
fn a_human_seat_does_not_redress_a_player_wearing_someone_else() {
    let mut app = seating_app();
    app.register_character(CharacterDefinition::new("mary_o", "Mary-O", "mary_o_demo"));
    app.register_character(CharacterDefinition::new("sanic", "Sanic", "sanic_demo"));
    app.world_mut().spawn((
        crate::avatar::PlayerSimulationBundle::from_scratch(
            crate::avatar::primary_player_scratch(
                Vec2::new(7.0, 0.0),
                ambition_engine_core::AbilitySet::default(),
            ),
            ambition_characters::actor::Health::new(5),
        ),
        ambition_characters::actor::WornCharacter::new("sanic"),
    ));
    app.insert_resource(MatchParticipantRoster {
        participants: vec![
            MatchParticipant::new("mary_o").driven_by(ControllerBinding::Human { device_slot: 0 })
        ],
    });

    app.update();

    let world = app.world_mut();
    let mut worn = world.query::<&ambition_characters::actor::WornCharacter>();
    let ids: Vec<String> = worn.iter(world).map(|w| w.id().to_string()).collect();
    assert_eq!(
        ids,
        vec!["sanic".to_string()],
        "seating re-dressed the player body, or spawned beside it"
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
#[test]
fn a_second_human_seat_gets_its_own_body_on_its_own_slot() {
    use ambition_characters::brain::{Brain, PlayerSlot};

    let mut app = seating_app();
    app.register_character(CharacterDefinition::new("mary_o", "Mary-O", "mary_o_demo"));
    app.register_character(CharacterDefinition::new("sanic", "Sanic", "sanic_demo"));
    app.world_mut().spawn((
        crate::avatar::PlayerSimulationBundle::from_scratch(
            crate::avatar::primary_player_scratch(
                Vec2::new(0.0, 0.0),
                ambition_engine_core::AbilitySet::default(),
            ),
            ambition_characters::actor::Health::new(5),
        ),
        ambition_characters::actor::WornCharacter::new("mary_o"),
    ));
    app.insert_resource(MatchParticipantRoster {
        participants: vec![
            MatchParticipant::new("mary_o").driven_by(ControllerBinding::Human { device_slot: 0 }),
            MatchParticipant::new("sanic").driven_by(ControllerBinding::Human { device_slot: 1 }),
        ],
    });

    app.update();

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
            MatchParticipant::new("sanic").driven_by(ControllerBinding::Human { device_slot: 1 })
        ],
    });

    app.update();

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
    });
    app.update();

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

    // Both override directions really are present, or the loop above proves
    // nothing the faction rule would not have decided by itself.
    assert_ne!(
        fighters[0].3, fighters[1].3,
        "teammates must have DIFFERENT factions here, or 'they cannot hit each \
         other' is just the faction rule"
    );
    assert_eq!(
        fighters[0].3, fighters[2].3,
        "opponents must share a faction here, or 'they can hit each other' is \
         just the faction rule"
    );
}

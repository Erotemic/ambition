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

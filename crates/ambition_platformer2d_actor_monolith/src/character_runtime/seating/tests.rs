//! C4 slice 1: a roster of CPU participants becomes bodies that can fight.

// Stepping a fixture is `finalize_and_update`, not `update`. Bevy's RUNNERS
// close the plugin-composition barrier; `App::update` does not, and character
// preparation publishes its registry there — so a fixture that only updated
// would register a cast and never publish one. Idempotent, so a helper called
// per step costs a set lookup after the first.
use ambition_platformer2d_shared_tangle::app_finalization::finalize_and_update;

use super::*;
use crate::character_runtime::{
    CharacterDefinition, CharacterDefinitionAppExt, ControllerBinding, MatchParticipant,
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
    app.init_resource::<crate::character_sprites::AuthoredSheets>();
    // Seating sizes each body from its sheet (U1 stage B), so the authored
    // registry is authority the system requires. A fixture authors none.
    app.init_resource::<crate::character_sprites::AuthoredSheets>();
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
            seat_match_participants,
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
        ..Default::default()
    });
    finalize_and_update(&mut app);

    {
        let world = app.world_mut();
        let mut q = world.query::<&ambition_characters::actor::WornCharacter>();
        assert_eq!(q.iter(world).count(), 0);
    }
    assert!(
        app.world().get_resource::<ActiveMatch>().is_none(),
        "a match that seated nobody must not ACTIVATE — the roster may become \
         seatable once its characters register"
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
                    ambition_platformer2d_core::AbilitySet::default(),
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
        ..Default::default()
    });

    finalize_and_update(&mut app);

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
        .get::<ambition_platformer2d_shared_tangle::body::BodyKinematics>(player)
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
                ambition_platformer2d_core::AbilitySet::default(),
            ),
            ambition_characters::actor::Health::new(5),
        ),
        ambition_characters::actor::WornCharacter::new("sanic"),
    ));
    app.insert_resource(MatchParticipantRoster {
        participants: vec![
            MatchParticipant::new("mary_o").driven_by(ControllerBinding::Human { device_slot: 0 })
        ],
        ..Default::default()
    });

    finalize_and_update(&mut app);

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
                ambition_platformer2d_core::AbilitySet::default(),
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
            MatchParticipant::new("sanic").driven_by(ControllerBinding::Human { device_slot: 1 })
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

/// **And the ADOPTED seat is suspended on that tick too.**
///
/// The test above seats two fighters by SPAWNING them, and a body that does not
/// exist until the command queue flushes cannot act before its suspension lands
/// — the property holds there almost by construction. Seat 0 is different: it
/// ADOPTS the primary player, a body that already exists, is already in the
/// brain's query, and was accepting input on the tick before the match began.
///
/// That is the body the "one simulation step first — a held direction carried in
/// from the menu" sentence was actually about, and it was the one the test did
/// not cover. Nothing had thought to ask, which is the same gap the abilities
/// test below was written for on the same body.
#[test]
fn an_adopted_seat_is_also_suspended_on_the_tick_it_joins() {
    let mut app = seating_app();
    app.register_character(CharacterDefinition::new("duelist", "Duelist", "demo"));

    let player = app
        .world_mut()
        .spawn((
            crate::avatar::PlayerSimulationBundle::from_scratch(
                crate::avatar::primary_player_scratch(
                    Vec2::new(0.0, 0.0),
                    ambition_platformer2d_core::AbilitySet::basic(),
                ),
                ambition_characters::actor::Health::new(10),
            ),
            ambition_characters::actor::WornCharacter::new("duelist"),
        ))
        .id();
    app.insert_resource(MatchParticipantRoster {
        participants: vec![
            MatchParticipant::new("duelist").driven_by(ControllerBinding::Human { device_slot: 0 })
        ],
        opens_suspended: true,
        ..Default::default()
    });

    finalize_and_update(&mut app);

    assert!(
        app.world()
            .get::<ambition_characters::brain::ScriptedControl>(player)
            .is_some(),
        "the adopted body joined the match still answering input, so whatever the \
         player was holding when the round opened moves the fighter before the \
         countdown says go"
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
/// Drives adoption specifically — seat 0 is a HUMAN seat, which is the branch
/// that adopts rather than spawns.
#[test]
fn an_adopted_seat_gets_the_same_body_facts_as_a_spawned_one() {
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
    let player = app
        .world_mut()
        .spawn((
            crate::avatar::PlayerSimulationBundle::from_scratch(
                crate::avatar::primary_player_scratch(
                    Vec2::new(0.0, 0.0),
                    ambition_platformer2d_core::AbilitySet::basic(),
                ),
                ambition_characters::actor::Health::new(10),
            ),
            ambition_characters::actor::WornCharacter::new("heavy"),
        ))
        .id();
    app.insert_resource(MatchParticipantRoster {
        participants: vec![
            MatchParticipant::new("heavy").driven_by(ControllerBinding::Human { device_slot: 0 })
        ],
        ..Default::default()
    });

    finalize_and_update(&mut app);

    let size = app
        .world()
        .get::<ambition_platformer2d_core::BodyKinematics>(player)
        .expect("the adopted body keeps its kinematics")
        .size;
    assert_eq!(
        (size.x, size.y),
        (38.0, 62.0),
        "the adopted seat kept its session's body box instead of the one its \
         character authored, so a mirror match puts two different shapes on the \
         stage"
    );
    let mass = app
        .world()
        .get::<crate::features::Mass>(player)
        .expect("the adopted body must carry the authored mass");
    assert!(
        (mass.0 - 4.5).abs() < 1e-4,
        "the adopted seat weighs {} instead of the authored 4.5, so the mount \
         pair's centre of gravity depends on which seat a character took",
        mass.0
    );
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
            MatchParticipant::new("tank").driven_by(ControllerBinding::Human { device_slot: 0 })
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
#[test]
fn an_adopted_seat_fights_with_the_same_abilities_as_a_spawned_one() {
    let mut app = seating_app();
    app.register_character(CharacterDefinition::new("duelist", "Duelist", "demo"));

    let mut sandbox_kit = ambition_platformer2d_core::AbilitySet::basic();
    sandbox_kit.fly = true;
    sandbox_kit.blink = true;
    let player = app
        .world_mut()
        .spawn((
            crate::avatar::PlayerSimulationBundle::from_scratch(
                crate::avatar::primary_player_scratch(Vec2::new(0.0, 0.0), sandbox_kit),
                ambition_characters::actor::Health::new(10),
            ),
            ambition_characters::actor::WornCharacter::new("duelist"),
        ))
        .id();
    app.insert_resource(MatchParticipantRoster {
        participants: vec![
            MatchParticipant::new("duelist").driven_by(ControllerBinding::Human { device_slot: 0 })
        ],
        fighter_abilities: Some(ambition_platformer2d_core::AbilitySet::basic()),
        ..Default::default()
    });

    finalize_and_update(&mut app);

    let abilities = app
        .world()
        .get::<ambition_platformer2d_core::BodyAbilities>(player)
        .expect("the adopted body keeps its ability component");
    assert!(
        !abilities.abilities.fly && !abilities.abilities.blink,
        "the adopted fighter carried the session's sandbox kit into the match: it \
         can fly and teleport and its opponent cannot"
    );
    assert!(
        abilities.abilities.jump,
        "and it must still be able to JUMP — equalising must not disarm the floor \
         every fighter needs"
    );
    // The BASE too, not only the effective set: the dev-tools sync recomputes
    // `effective = base ∩ editable_mask` every frame for the primary player, so a
    // fix that wrote only the effective set would be undone by a system doing its
    // job correctly.
    let base = app
        .world()
        .get::<ambition_platformer2d_core::AbilityBase>(player)
        .expect("the adopted body keeps its ability base");
    assert!(
        !base.abilities.fly && !base.abilities.blink,
        "the intrinsic set still authorises flight, so the next dev-sync tick puts \
         it back and the match is unfair again one frame later"
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
            .get_resource::<crate::character_runtime::MatchSeatingRefused>()
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
                MatchParticipant::new("mary_o")
                    .driven_by(ControllerBinding::Human { device_slot: 0 }),
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
    /// simply refused to seat anything. A spawned seat and an ADOPTED seat land
    /// in the same command flush — the mix is the point, because they take
    /// different paths and it was the seam between them that could tear.
    #[test]
    fn every_seat_is_constructed_on_the_same_tick() {
        let mut app = seating_app();
        app.register_character(CharacterDefinition::new("mary_o", "Mary-O", "mary_o_demo"));
        app.register_character(CharacterDefinition::new("sanic", "Sanic", "sanic_demo"));
        let player = spawn_player(&mut app);
        finalize_and_update(&mut app);

        app.insert_resource(MatchParticipantRoster {
            participants: vec![
                MatchParticipant::new("mary_o")
                    .driven_by(ControllerBinding::Human { device_slot: 0 }),
                cpu("sanic"),
            ],
            ..Default::default()
        });

        finalize_and_update(&mut app);

        assert!(
            app.world().get::<MatchSeat>(player).is_some(),
            "the adopted seat did not land on the activating tick"
        );
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
        .get_resource::<crate::character_runtime::MatchSeatingRefused>()
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

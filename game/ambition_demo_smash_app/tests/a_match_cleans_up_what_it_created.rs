//! Nothing a match put in the world outlives the match.
//!
//! Jon, 2026-09-05, playing it: *"I also notice that a mine laid in a match still
//! persists into the next match, that sounds like an issue with architecture
//! expression. Ending a match should be cleaning everything up, don't hack in a
//! solution to this, we need to find the right solution."*
//!
//! ✔ **FIXED 2026-09-05, AND THIS IS THE GUARD.** It was written as a red
//! acceptance criterion for a fix that did not exist, and the fix landed the same
//! day: `MatchScoped`, stamped where an object is created and swept by
//! `sweep_objects_from_ended_matches` in `CombatSet::Trigger`.
//!
//! ⛔ THE DEFECT IT PINS. Measured 2026-09-05: the smash ruleset spawns into the
//! world at five sites — `bomb.rs`, `bolt.rs`, `mine.rs`, `portal.rs` and
//! `spring.rs` — and no system despawned any of them at a match boundary. Each
//! object ended only by its own rule: a fuse, a trigger, a lifetime, a caster's
//! next cast. A match ending is not one of those rules.
//!
//! ⭐ IT NAMES THE MOVE, NOT THE MARKER, which is what keeps it honest about a
//! future redesign: it dispatches the authored `smash.place_mine` technique and
//! asks only whether that object survived the boundary. Any mechanism that ends a
//! match's objects satisfies it.
//!
//! ⛔⛔ AND IT FAILED ONCE FOR THE WRONG REASON, which is the lesson. The first
//! version SPAWNED a bare `PlacedMine` by hand — "fix-agnostic" — and kept
//! failing after the fix landed, correctly: an entity the ruleset never created
//! carries none of what the ruleset stamps on its own objects. ⇒ A test that
//! plants its subject outside the mechanism is asking about a different object.

use ambition_demo_smash_app::build_demo_app;
use ambition_platformer2d::actor::MatchSeat;
use bevy::prelude::*;

/// Seat a one-stock match on the gameplay route.
fn start_a_match(app: &mut App) {
    let mut roster = ambition_demo_smash::smash_roster_at_levels(
        [
            ambition_demo_smash::SMASH_CHARACTER_ID,
            ambition_demo_smash::SMASH_OPPONENT_ID,
        ],
        &[5, 5],
    );
    // One stock, so the match decides inside a budget a test can afford. The
    // pace is tuning; what is under test is the BOUNDARY.
    roster.rules.stocks = Some(1);
    app.world_mut().insert_resource(roster);
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellCommand::GoTo(
            ambition_platformer2d::game_shell::ShellRouteId::new(
                ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
            ),
        ));
}

fn seats(app: &mut App) -> usize {
    let world = app.world_mut();
    let mut query = world.query::<&MatchSeat>();
    query.iter(world).count()
}

#[test]
fn nothing_a_match_created_survives_into_the_next_one() {
    let mut app = build_demo_app();
    for _ in 0..30 {
        app.update();
    }
    start_a_match(&mut app);
    for _ in 0..120 {
        app.update();
    }
    assert!(
        seats(&mut app) >= 2,
        "the first match never seated its fighters, so this test never reached \
         the boundary it is about"
    );

    // ⛔⛔ PLACED THROUGH THE GAME, NOT SPAWNED BY HAND. The first version of
    // this test spawned a bare `PlacedMine` and it kept failing after the fix
    // landed — correctly, because an entity the ruleset never created carries
    // none of what the ruleset stamps on its own objects. ⇒ A test that plants
    // its subject outside the mechanism is asking about a different object.
    //
    // So it dispatches the authored technique for a seated fighter and lets the
    // real spawn path run, which is also what keeps this fix-agnostic: it names
    // the MOVE, not the marker.
    let placer = {
        let world = app.world_mut();
        let mut q = world.query::<(Entity, &MatchSeat)>();
        let mut seats: Vec<(Entity, usize)> =
            q.iter(world).map(|(e, s)| (e, s.0)).collect();
        seats.sort_by_key(|(_, seat)| *seat);
        seats.first().map(|(e, _)| *e).expect("a seated fighter")
    };
    app.world_mut().write_message(
        ambition_platformer2d::characters::brain::ActorActionMessage {
            actor: placer,
            request:
                ambition_platformer2d::characters::brain::action_set::ActionRequest::Special {
                    spec:
                        ambition_platformer2d::characters::brain::action_set::SpecialActionSpec::Special(
                            ambition_platformer2d::characters::smash_mine::PLACE_MINE.to_string(),
                        ),
                    params: ambition_platformer2d::entity_catalog::ParamValue::from_typed(
                        &ambition_platformer2d::characters::smash_mine::PlaceMineParams {
                            arm_s: 0.05,
                            damage: 1,
                            blast_radius: 8.0,
                            item_id: "polygon_mine".to_string(),
                            offset: (0.0, 0.0),
                            half_extents: (6.0, 6.0),
                        },
                    )
                    .expect("mine params serialize"),
                },
        },
    );
    app.update();

    let planted = {
        let world = app.world_mut();
        let mut q = world.query_filtered::<Entity, With<ambition_demo_smash::mine::PlacedMine>>();
        let found: Vec<Entity> = q.iter(world).collect();
        assert_eq!(
            found.len(),
            1,
            "the authored mine technique placed {} mines; this test needs exactly \
             one so it can ask whether THAT object survived the boundary",
            found.len()
        );
        found[0]
    };

    // Run the match out. A one-stock match decides well inside this.
    for _ in 0..5_400 {
        app.update();
        if seats(&mut app) < 2 {
            break;
        }
    }

    start_a_match(&mut app);
    for _ in 0..240 {
        app.update();
    }

    // ⛔ THE SECOND MATCH MUST ACTUALLY EXIST, or "survived into the next match"
    // is a claim about a world with no next match in it — and the assertion
    // below would then be failing for a reason that has nothing to do with
    // cleanup.
    assert!(
        seats(&mut app) >= 2,
        "the second match never seated its fighters, so the assertion below \
         would be measuring a boundary that was never crossed"
    );

    assert!(
        app.world().get_entity(planted).is_err(),
        "a mine placed during the first match is still in the world during the \
         second. Ending a match has to end what the match created: the ruleset \
         spawns at five sites (bomb, bolt, mine, portal, spring) and none of \
         them is despawned at a boundary, so every one of them outlives its \
         match. ⇒ The fix wants ONE owner for that lifetime, not five systems \
         each remembering."
    );
}

//! D255/R10: two people on the couch, two bombs, and neither steals the other's.

use ambition_platformer2d::game_shell::{ShellCommand, ShellRouteId};

/// ⭐⭐ EACH LOCAL SEAT PICKS UP ITS OWN ITEM.
///
/// ⛔⛔ ONLY ONE COULD. `pickup_held_item_system`, `throw_held_item_system` and
/// `fire_held_ranged_system` each read `ControlledSubject` — ONE entity. That is
/// the right answer for the adventure game, where you drive one body, and the
/// wrong one for a Smash stage with two people on it: seat 1 pressed attack
/// standing on a bomb and nothing happened, which from the sofa is a bomb that
/// ignores you.
///
/// ⛔ THE FIX IS A POPULATION, NOT A SECOND SINGLETON. `DrivenBodies` is the
/// union — the possessed subject plus every seat — asked once and shared by all
/// three verbs, so the next item verb cannot get a fourth answer.
///
/// ⛔ AND BOTH HALVES ARE ASSERTED. "Seat 1 can pick up" is satisfied by a fix
/// that moved the singleton; "and seat 0 still holds its own" is what says they
/// are independent.
#[test]
fn two_local_seats_each_pick_up_their_own_bomb() {
    use ambition_platformer2d::actor::MatchSeat;
    use ambition_platformer2d::characters::control::PlayerSlot;
    use ambition_platformer2d::item::{GroundItem, ItemCustody};
    use bevy::prelude::*;

    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    for _ in 0..30 {
        app.update();
    }
    // TWO HUMAN SEATS. `smash_roster` seats slot 0 human and the rest CPU, and a
    // CPU seat is not what this is about — a brain that never presses attack
    // would pass the seat-1 arm by never contesting anything.
    let mut roster =
        ambition_demo_smash::smash_roster(["npc_pirate_admiral", "npc_pirate_admiral"]);
    roster.participants[1] = roster.participants[1].clone().driven_by(
        ambition_platformer2d::actor::ControllerBinding::Human {
            source: ambition_platformer2d::actor::LocalInputSource::Pad(1),
        },
    );
    app.world_mut().insert_resource(roster);
    app.world_mut()
        .write_message(ShellCommand::GoTo(ShellRouteId::new(
            ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
        )));
    for _ in 0..900 {
        app.update();
        let (seated, held) = {
            let world = app.world_mut();
            let mut all = world.query::<&MatchSeat>();
            let seated = all.iter(world).count();
            let mut q = world.query_filtered::<
                &MatchSeat,
                With<ambition_platformer2d::characters::control::ScriptedControl>,
            >();
            (seated, q.iter(world).count())
        };
        if seated > 1 && held == 0 {
            break;
        }
    }
    let seat = |app: &mut App, want: usize| -> Entity {
        let world = app.world_mut();
        let mut q = world.query::<(Entity, &MatchSeat)>();
        q.iter(world)
            .find(|(_, s)| s.0 == want)
            .map(|(entity, _)| entity)
            .expect("the match seats this fighter")
    };
    let bodies = [seat(&mut app, 0), seat(&mut app, 1)];

    // A bomb under each fighter, so neither has to walk to one.
    let spec = ambition_platformer2d::character::held_item_by_id("polygon_bomb")
        .expect("polygon_bomb is a registered held item");
    let mut bombs = Vec::new();
    for body in bodies {
        let at = app
            .world()
            .get::<ambition_platformer2d::engine_core::BodyKinematics>(body)
            .expect("a seated fighter has kinematics")
            .pos;
        bombs.push(
            app.world_mut()
                .spawn((
                    GroundItem {
                        spec: spec.clone(),
                        pos: at,
                        vel: ambition_platformer2d::engine_core::Vec2::ZERO,
                        half_extent: ambition_platformer2d::engine_core::Vec2::splat(12.0),
                    },
                    ItemCustody::InWorld,
                ))
                .id(),
        );
    }

    // ⛔⛔ ONE PRESS EACH, THEN NOTHING. `polygon_bomb` grants no verb of its
    // own, so `HeldUseBehavior::Auto` makes a plain Attack THROW it — holding
    // the button picks the bomb up and throws it on the next edge, and the first
    // version of this test held it for twenty ticks and measured two bombs
    // sailing away from two empty hands.
    let grab = ambition_platformer2d::engine_core::ControlFrame {
        attack_pressed: true,
        ..Default::default()
    };
    for tick in 0..12 {
        for slot in 0..2u8 {
            // ⛔⛔ EXACTLY ONE FRAME OF PRESS. Measured: both bombs are in both
            // hands on the very first tick, and the SECOND edge of a continued
            // hold throws them straight back out — at four frames of hold, seat
            // 0's bomb is airborne by tick 1 and seat 1's by tick 2. This arm is
            // about who may pick up, so it presses once and then watches the
            // hands stay full.
            let frame = if tick == 0 {
                grab
            } else {
                ambition_platformer2d::engine_core::ControlFrame::default()
            };
            ambition_platformer2d::sim::drive_slot_frame(app.world_mut(), PlayerSlot(slot), frame);
        }
        app.update();
    }

    let holder_of = |app: &App, bomb: Entity| {
        app.world()
            .get::<ItemCustody>(bomb)
            .and_then(|custody| match custody {
                ItemCustody::Held { holder } => Some(*holder),
                ItemCustody::InWorld => None,
            })
    };
    let held = [holder_of(&app, bombs[0]), holder_of(&app, bombs[1])];
    assert_eq!(
        held,
        [Some(bodies[0]), Some(bodies[1])],
        "seat 0 holds {:?} and seat 1 holds {:?}, against {:?} and {:?}. A \
         press-gated item action reads ONE `ControlledSubject`, so the second \
         seat's press reached nobody",
        held[0],
        held[1],
        Some(bodies[0]),
        Some(bodies[1])
    );
}

//! The road off the smash stage after an ordinary knockout, in the SHIPPED
//! composition.
//!
//! ⛔⛔ NOT THE DEMO SHELL, and that is the point. Jon plays the shipped app, and
//! the two compositions differ in the one place this road can hang: the shipped
//! one runs a GGRS host, so `ConfirmedFrameBoundary` EXISTS and the return
//! countdown's `fully_confirmed()` gate is a real question rather than the
//! "no host, confirm everything" default.
//!
//! ⭐ AND THAT IS EXACTLY WHERE IT BROKE. The boundary was published from
//! `ConfirmedFrameCount`, which bevy_ggrs computes before bumping the frame, so
//! it reported `confirmed == current - 1` forever — even with rollback dormant.
//! The gate never opened, and the stage never went home.

use ambition_platformer2d::game_shell::{ShellCommand, ShellRouteId};

/// ⭐⭐ A KNOCKOUT ENDS THE MATCH AND TAKES YOU BACK TO THE LOBBY.
///
/// Jon, 2026-08-28: *"after a character loses all the stocks a smash match can
/// just softlock forcing you to use the menu to exit to the shell. The match
/// should end and you should go back to the character select screen."*
///
/// ⛔ THE TWO ROADS OFF THE STAGE ARE NOT ONE ROAD, and only the other one had
/// a test. `ShellAbandonRequested` reaches the `abandoned` arm of
/// `return_to_the_select_screen_when_the_match_ends` and leaves IMMEDIATELY —
/// no card, no countdown, no confirmed-frame gate. A knockout takes the arm
/// that waits for `fully_confirmed()`, arms `RETURN_TO_SELECT_AFTER`, and spends
/// it against `Res<Time>` while the SIM clock is held at zero. Each of those is
/// a way never to leave, and none of them was covered: the demo's
/// `a_second_match_on_the_same_stage_counts_in_and_ends` runs updates after the
/// first match and then issues its OWN `GoTo(SMASH_GAMEPLAY_ROUTE)`, so it
/// passes whether or not the stage ever went home.
#[test]
fn an_ordinary_knockout_returns_to_the_select_screen_on_its_own() {
    use ambition_platformer2d::actor::{BodyKinematics, MatchSeat, StocksMatchDecided};
    use bevy::prelude::*;

    #[derive(Resource, Default)]
    struct Decided(usize);

    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    app.init_resource::<Decided>();
    app.add_systems(
        Update,
        |mut decided: MessageReader<StocksMatchDecided>, mut seen: ResMut<Decided>| {
            seen.0 += decided.read().count();
        },
    );
    for _ in 0..30 {
        app.update();
    }
    let mut roster = ambition_demo_smash::smash_roster(["actor", "actor"]);
    // One stock, so a single ring-out is the whole match.
    roster.rules.stocks = Some(1);
    app.world_mut().insert_resource(roster);
    app.world_mut()
        .write_message(ShellCommand::GoTo(ShellRouteId::new(
            ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
        )));

    // The round going LIVE is observable; waiting a frame count would encode the
    // opening ceremony's length instead.
    let mut live = false;
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
        if seated > 0 && held == 0 {
            live = true;
            break;
        }
    }
    assert!(live, "the opening ceremony never released the cast");

    // Throw seat one off the side. A body at 2400px/s crosses the blast margin
    // in a handful of ticks, and a claim about what happens AFTER a match must
    // not depend on combat tuning to reach it.
    for _ in 0..600 {
        {
            let world = app.world_mut();
            let mut query = world.query::<(&MatchSeat, &mut BodyKinematics)>();
            for (seat, mut kin) in query.iter_mut(world) {
                if seat.0 == 1 {
                    kin.vel = ambition_platformer2d::engine_core::Vec2::new(2_400.0, -200.0);
                }
            }
        }
        app.update();
        if app.world().resource::<Decided>().0 > 0 {
            break;
        }
    }
    // ⛔ THE PREMISE. An assertion about returning would pass on a stage that
    // never ended.
    assert!(
        app.world().resource::<Decided>().0 > 0,
        "the fixture never knocked anybody out, so this run says nothing about \
         what happens when a match ends"
    );

    let route_now = |app: &App| -> Option<String> {
        app.world()
            .resource::<ambition_platformer2d::game_shell::ShellRouter>()
            .active
            .as_ref()
            .map(|active| active.route_id.as_str().to_string())
    };
    assert_eq!(
        route_now(&app).as_deref(),
        Some(ambition_demo_smash::SMASH_GAMEPLAY_ROUTE),
        "the winner card should still be on the stage on the tick the match was \
         decided"
    );

    // The card is worth `RETURN_TO_SELECT_AFTER` of REAL time; a headless loop
    // spends that quickly, so this is a ceiling rather than a schedule.
    for _ in 0..4_000 {
        app.update();
        if route_now(&app).as_deref() == Some(ambition_demo_smash::SMASH_SELECT_ROUTE) {
            break;
        }
    }
    assert_eq!(
        route_now(&app).as_deref(),
        Some(ambition_demo_smash::SMASH_SELECT_ROUTE),
        "the match ended and the stage never went home. This is the softlock: \
         the winner card sits over a frozen fight and the pause menu is the only \
         way out"
    );
}

//! The Actor's down-B, in the SHIPPED SMASH COMPOSITION and under smash rules.
//!
//! ⛔⛔ ON THE SMASH STAGE, NOT AN AMBITION ROOM. Jon, 2026-08-28: *"when we are
//! doing smash moves we probably should be using the smash stage and not any
//! ambition stages, to make sure that we're actually getting smash rules and not
//! ambition which might be different."* The exploration road seats a body under
//! a different ruleset — different stocks, different hit rules, no match — so a
//! move verified there is verified against the wrong game.

use ambition_platformer2d::characters::control::DrivingParticipant;
use ambition_platformer2d::game_shell::{ShellCommand, ShellRouteId};

/// ⭐⭐ SHE STAYS UNDER FOR AS LONG AS SPECIAL IS HELD, AND COMES UP WHEN IT
/// LETS GO. Jon, 2026-08-28: *"she should be able to pop up at any time from
/// it."*
///
/// The beat under the stage is the shipped CHARGE mechanic — `MoveCharge`
/// freezes the timeline at `HOLD_UNDER_AT_S` while the Special button is down
/// and resumes on release or at `MAX_UNDER_S` — so this is the wiring test for
/// that reuse. ⛔ AND IT IS THE ONE THAT MATTERS: the authoring test in
/// `actor_moveset` proves the policy is on the spec, and the spec being right
/// is worth nothing if `special_held` never reaches the charge on the human
/// road. Retiming the move on an authoring test alone would have shortened the
/// submerged beat from a fixed second to two ticks.
///
/// ⛔ THE ARMS STRADDLE THE BUTTON. A held press and a released one, measured
/// the same way, because "she was submerged for a while" is true of the old
/// fixed-duration move as well — only the CONTRAST says the hold is what is
/// doing it.
#[test]
fn the_trap_keeps_her_under_the_stage_while_special_is_held() {
    use ambition_platformer2d::actor::MatchSeat;
    use bevy::prelude::*;

    fn submerged_ticks(hold: bool) -> usize {
        let mut app = ambition_app::app::build_visible_app(
            ambition_app::app::VisibleRenderMode::NoWindow,
            true,
        );
        for _ in 0..30 {
            app.update();
        }
        // ⛔ `smash_roster`, NOT `smash_roster_at_levels`: the levelled helper
        // makes EVERY participant a CPU, and `drive_control_frame` would then be
        // talking to a slot nobody owns. `smash_ride.rs` records the day that
        // cost.
        app.world_mut()
            .insert_resource(ambition_demo_smash::smash_roster(["actor", "actor"]));
        app.world_mut()
            .write_message(ShellCommand::GoTo(ShellRouteId::new(
                ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
            )));
        // The round going LIVE is observable — a cast exists and nothing in it is
        // still held by the opening ceremony — so wait for that rather than for a
        // frame count that encodes the ceremony's length.
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

        let seat0 = {
            let world = app.world_mut();
            let mut q = world.query::<(Entity, &MatchSeat)>();
            q.iter(world)
                .find(|(_, seat)| seat.0 == 0)
                .map(|(entity, _)| entity)
                .expect("the match seats a first fighter")
        };
        assert!(
            app.world().get::<DrivingParticipant>(seat0).is_some(),
            "seat 0 is not driven by a participant, so the press below reaches \
             nobody and this measures an idle fighter"
        );

        // ⛔ SHE MUST BE ON THE GROUND. The stage drops its cast in, and a
        // down-Special in the air is `special_air_down`, a different move on a
        // body with no surface under it.
        for _ in 0..120 {
            ambition_platformer2d::sim::drive_control_frame(
                app.world_mut(),
                ambition_platformer2d::engine_core::ControlFrame::default(),
            );
            app.update();
        }

        let press = ambition_platformer2d::engine_core::ControlFrame {
            // +y is DOWN.
            axis_y: 1.0,
            special_pressed: true,
            special_held: true,
            ..Default::default()
        };
        ambition_platformer2d::sim::drive_control_frame(app.world_mut(), press);
        app.update();

        let mut under = 0usize;
        for _ in 0..90 {
            ambition_platformer2d::sim::drive_control_frame(
                app.world_mut(),
                ambition_platformer2d::engine_core::ControlFrame {
                    special_pressed: false,
                    special_held: hold,
                    ..press
                },
            );
            app.update();
            if app
                .world()
                .get::<ambition_platformer2d::engine_core::BodyModeState>(seat0)
                .is_some_and(|mode| {
                    mode.body_mode == ambition_platformer2d::engine_core::BodyMode::Submerged
                })
            {
                under += 1;
            }
        }
        under
    }

    let held = submerged_ticks(true);
    let released = submerged_ticks(false);
    assert!(
        held >= 30,
        "holding Special kept her under the stage for only {held} ticks of 90 — \
         the hold is not reaching the charge, and the move is now a two-tick \
         blink instead of the second Jon authored"
    );
    assert!(
        released < held,
        "letting Special go kept her under for {released} ticks against {held} \
         held — releasing does not bring her up, so `pop up at any time` is not \
         wired to the button"
    );
}

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

/// ⭐⭐ THE SUBTERRANEAN BEAT IS A DURATION, AND AN ACTION PRESS TAKES IT BACK.
///
/// Jon's own lifecycle for the move, 2026-08-28: *"In this subterranean state
/// they can move for up to the timelimit of the move (3 seconds)… When the move
/// ends or the character ends the move by pressing a non-move action, the final
/// stage of the move happens."*
///
/// ⛔⛔ THE FIRST VERSION OF THIS SHIPPED THE OTHER READING AND JON CAUGHT IT IN
/// A DAY: *"The latest main the actor doesn't spend any time under the stage…
/// It looks like the pop up happens immediately."* It was authored as a HELD
/// charge — freeze while the button is down — so a player steering with the
/// stick and no finger on B got three ticks under the boards. The authoring test
/// in `actor_moveset` was green throughout, because the policy WAS on the spec;
/// what was wrong was what the policy meant.
///
/// ⛔ SO THE ARMS STRADDLE THE PRESS, and neither one holds anything. Idle
/// hands keep her under; an action press brings her up. A test that only checked
/// the first would pass on a move that could never be cut short, and one that
/// only checked the second would pass on the bug Jon reported.
#[test]
fn the_trap_keeps_her_under_the_stage_until_an_action_press_ends_it() {
    use ambition_platformer2d::actor::MatchSeat;
    use bevy::prelude::*;

    /// Ticks spent submerged over a 90-tick window. `interrupt_at` presses
    /// ATTACK on that tick — a button the move was not started with, because
    /// *"a non-move action"* is the condition and not "the same button again".
    fn submerged_ticks(interrupt_at: Option<usize>) -> usize {
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
        // down-Special in the air is `special_air_down` on a body with no
        // surface under it.
        for _ in 0..120 {
            ambition_platformer2d::sim::drive_control_frame(
                app.world_mut(),
                ambition_platformer2d::engine_core::ControlFrame::default(),
            );
            app.update();
        }

        // Down + Special, then the button comes STRAIGHT BACK UP — which is how
        // a person presses it and is the case the bug was in.
        ambition_platformer2d::sim::drive_control_frame(
            app.world_mut(),
            ambition_platformer2d::engine_core::ControlFrame {
                // +y is DOWN.
                axis_y: 1.0,
                special_pressed: true,
                special_held: true,
                ..Default::default()
            },
        );
        app.update();

        let mut under = 0usize;
        for tick in 0..90 {
            let interrupt = interrupt_at == Some(tick);
            ambition_platformer2d::sim::drive_control_frame(
                app.world_mut(),
                ambition_platformer2d::engine_core::ControlFrame {
                    // Steering under the stage is what the beat is FOR, and it
                    // must not end it.
                    axis_x: 1.0,
                    attack_pressed: interrupt,
                    attack_held: interrupt,
                    ..Default::default()
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

    let idle = submerged_ticks(None);
    let interrupted = submerged_ticks(Some(20));
    assert!(
        idle >= 60,
        "with nobody holding anything she was under the stage for {idle} ticks \
         of 90. The subterranean beat is a DURATION — three seconds — and this \
         is the shape Jon reported as `doesn't spend any time under the stage`"
    );
    assert!(
        interrupted + 20 < idle,
        "an attack press on tick 20 left her under for {interrupted} ticks \
         against {idle} idle, so pressing an action does not bring her up and \
         the beat cannot be cut short"
    );
}

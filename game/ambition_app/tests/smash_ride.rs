//! D207 end-to-end: the admiral calls a burning flying shark and rides it.
//!
//! ⛔ IN THE SHIPPED COMPOSITION, not the demo shell. Same reason the goblin/PCA
//! census gives: the demo shell's catalog carries George and two stand-ins, so
//! `npc_pirate_admiral` cannot be seated there and a rig that tried would
//! measure an empty stage.

use ambition_platformer2d::characters::control::DrivingParticipant;
use ambition_platformer2d::game_shell::{ShellCommand, ShellRouteId};

/// ⭐⭐ THE ADMIRAL CALLS A SHARK, RIDES IT, AND COMES OFF WHEN HE JUMPS.
///
/// D207, end to end through the real composition: an authored effect key on a
/// move's timeline becomes a summoned mount, the summoner is welded into its
/// saddle, the mount carries him, and a jump press puts him down.
///
/// ⛔⛔ EVERY ARM HERE HAS A PREMISE, because almost every way this could
/// measure nothing looks exactly like it working. "No shark appeared" and "the
/// shark appeared and the pirate did not board" and "he boarded and the mount
/// never moved" all end with a pirate standing on a stage.
#[test]
fn the_admirals_up_b_summons_a_shark_he_rides_until_he_jumps_off() {
    use ambition_platformer2d::actor::{BodyKinematics, MatchSeat};
    use ambition_platformer2d::mount::{Mountable, RideLease, RidingOn};
    use bevy::prelude::*;

    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    for _ in 0..30 {
        app.update();
    }
    // BOTH seats the admiral: seat 0 is the human this test drives, and seat 1
    // is a CPU that simply has to exist for the match to stage.
    // ⛔⛔ `smash_roster`, NOT `smash_roster_at_levels`. The levelled helper
    // overwrites EVERY participant as a CPU, so the first version of this test
    // drove `drive_control_frame` at a slot nobody owned and measured a fighter
    // that never received the press. It looked exactly like a broken up-B.
    // `smash_roster` is the one that seats slot 0 as a human.
    app.world_mut()
        .insert_resource(ambition_demo_smash::smash_roster([
            "npc_pirate_admiral",
            "npc_pirate_admiral",
        ]));
    app.world_mut()
        .write_message(ShellCommand::GoTo(ShellRouteId::new(
            ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
        )));
    for _ in 0..240 {
        app.update();
    }

    let seat0 = {
        let world = app.world_mut();
        let mut q = world.query::<(Entity, &MatchSeat)>();
        q.iter(world)
            .find(|(_, seat)| seat.0 == 0)
            .map(|(entity, _)| entity)
            .expect("the match seats a first fighter")
    };
    // ⛔⛔ THE PREMISE THAT COST A DAY. The first version used
    // `smash_roster_at_levels`, which overwrites EVERY participant as a CPU —
    // including seat 0 — so `drive_control_frame` drove `PlayerSlot::PRIMARY`
    // at a slot nobody owned. Every downstream assertion then reported a
    // perfectly good up-B as broken. A rig that drives a seat must first say
    // the seat is drivable.
    assert!(
        app.world().get::<DrivingParticipant>(seat0).is_some(),
        "seat 0 is not driven by a participant, so `drive_control_frame` is \
         talking to nobody and nothing below measures the up-B"
    );

    let sharks = |app: &mut App| -> usize {
        let world = app.world_mut();
        let mut q = world.query_filtered::<Entity, With<Mountable>>();
        q.iter(world).count()
    };
    // ⛔ THE PREMISE: no shark before the press. Without this the count below
    // could be counting an authored stage prop.
    assert_eq!(sharks(&mut app), 0, "the stage already carries a mountable");

    // ── PRESS UP + SPECIAL. Held, because a one-tick press assumes the body
    //    steps after the frame is committed inside one update. ──
    let press = |app: &mut App, frames: usize, frame: ambition_platformer2d::engine_core::ControlFrame| {
        for _ in 0..frames {
            ambition_platformer2d::sim::drive_control_frame(app.world_mut(), frame);
            app.update();
        }
    };
    // ⛔ ONE press FRAME, then HELD. `special_pressed` is a rising EDGE: holding
    // it true for ten frames is ten presses, which repopulates the special
    // buffer every tick. The direction comes from `axis_y` — the fighter brain
    // folds it into `attack_axis` and the special resolver reads that — so
    // `up_pressed` is irrelevant to picking an up-special.
    let up_special = ambition_platformer2d::engine_core::ControlFrame {
        axis_y: -1.0,
        special_pressed: true,
        special_held: true,
        ..Default::default()
    };
    press(&mut app, 1, up_special);
    press(
        &mut app,
        9,
        ambition_platformer2d::engine_core::ControlFrame {
            special_pressed: false,
            ..up_special
        },
    );
    // Let the summon commit and the saddle weld.
    for _ in 0..20 {
        app.update();
    }

    assert_eq!(
        sharks(&mut app),
        1,
        "the up-B summoned no shark, so nothing below is about riding one"
    );
    assert!(
        app.world().get::<RidingOn>(seat0).is_some(),
        "a shark exists and the admiral is not on it — the board was refused, \
         which is a `CanPilot` or a mount-class problem rather than a summon one"
    );
    assert!(
        app.world().get::<RideLease>(seat0).is_some(),
        "the ride has no clock on it, so it would never end"
    );

    // ── IT CARRIES HIM. Hold the stick and the pair travels. ──
    let x_of = |app: &mut App, e: Entity| -> f32 {
        app.world()
            .get::<BodyKinematics>(e)
            .map(|kin| kin.pos.x)
            .expect("the rider has kinematics")
    };
    let before = x_of(&mut app, seat0);
    press(
        &mut app,
        60,
        ambition_platformer2d::engine_core::ControlFrame {
            axis_x: 1.0,
            ..Default::default()
        },
    );
    let travelled = x_of(&mut app, seat0) - before;
    assert!(
        travelled > 40.0,
        "the admiral moved {travelled:.1}px while holding right in the saddle — \
         the mount is not answering the rider's stick, which is the whole of \
         'effectively fly around using the control stick'"
    );
    assert!(
        app.world().get::<RidingOn>(seat0).is_some(),
        "he came off during ordinary steering"
    );

    // ── A JUMP PUTS HIM DOWN, and the shark leaves. ──
    press(
        &mut app,
        4,
        ambition_platformer2d::engine_core::ControlFrame {
            jump_pressed: true,
            jump_held: true,
            ..Default::default()
        },
    );
    assert!(
        app.world().get::<RidingOn>(seat0).is_none(),
        "the admiral jumped and stayed in the saddle"
    );
    // The departure is a flight, not a despawn, so give it its clock.
    for _ in 0..180 {
        app.update();
    }
    assert_eq!(
        sharks(&mut app),
        0,
        "the shark stayed on the stage after losing its rider — an unridden \
         mount running its own brain around a platform fighter is exactly what \
         `DepartsWhenRiderless` exists to prevent"
    );
}

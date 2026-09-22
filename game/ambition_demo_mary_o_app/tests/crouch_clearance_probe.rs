//! **BEING BLOCKED FROM STANDING KEEPS HER CROUCHED.**
//!
//! Jon, 2026-09-21: *"when I do crouch and then walk under a block and let her
//! stand, she starts to spaz out — if you are blocked from standing the game
//! should just not let you un-crouch until you have clearance."*
//!
//! `try_change_body_mode_clusters` clearance-tests every expansion, so the
//! refusal exists. It is only as good as the box the decision is made against:
//! if a sheet-authored body is not yet wearing its sheet geometry when
//! `update_body_mode` runs, the proposed box equals the current one, the
//! spatial-subset fast path skips the overlap test, and the stand is permitted.
//! This arm drives the real demo into a real low ceiling and asserts both that
//! she stays crouched and that she stops moving.

use ambition_demo_mary_o_app::build_demo_app;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::input::ControlFrame;
use ambition_platformer2d::platformer::markers::PrimaryPlayer;
use bevy::prelude::*;

fn boot() -> App {
    let mut app = build_demo_app();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 60.0),
    ));
    for _ in 0..600 {
        app.update();
        let mut q = app
            .world_mut()
            .query_filtered::<&ae::BodyKinematics, With<PrimaryPlayer>>();
        if q.iter(app.world()).next().is_some() {
            return app;
        }
    }
    panic!("the demo never activated a playable body");
}

fn step(app: &mut App, frame: ControlFrame) {
    app.world_mut()
        .resource_mut::<ambition_platformer2d::scripted_input::ScriptedControls>()
        .0 = frame;
    app.update();
}

fn body(app: &mut App) -> ae::BodyKinematics {
    let world = app.world_mut();
    let mut q = world.query_filtered::<&ae::BodyKinematics, With<PrimaryPlayer>>();
    *q.iter(world).next().expect("a player body")
}

#[test]
fn a_ceiling_she_cannot_stand_under_keeps_her_crouched() {
    let mut app = boot();
    ambition_platformer2d::scripted_input::drive_the_local_participant(&mut app);

    // Tall, so crouching is a real height change and a ceiling can tell the two
    // apart. A small body would make every arm below vacuous.
    {
        let world = app.world_mut();
        let mut q = world.query_filtered::<Entity, With<PrimaryPlayer>>();
        let player = q.iter(world).next().expect("a player");
        world.entity_mut(player).insert(
            ambition_platformer2d::characters::equipment::WornEquipment::new(vec![
                ambition_demo_mary_o::powerups::star_wand(),
            ]),
        );
    }

    let mut down = ControlFrame::default();
    down.axis_y = 1.0;
    for _ in 0..30 {
        step(&mut app, down.clone());
    }

    let crouched = body(&mut app);
    let standing_height = crouched.size.y * 2.0;
    assert!(
        standing_height > crouched.size.y + 1.0,
        "fixture: crouching must change her height, or a ceiling cannot \
         distinguish standing from crouching"
    );

    // A ceiling that clears her crouch and refuses her stand.
    let gap = crouched.size.y + 8.0;
    let feet = crouched.pos.y + crouched.size.y * 0.5;
    {
        let world = app.world_mut();
        let mut q = world.query::<&mut ae::RoomGeometry>();
        let mut geom = q.iter_mut(world).next().expect("the room's geometry");
        geom.0.blocks.push(ae::Block::solid(
            "test_ceiling",
            ae::Vec2::new(crouched.pos.x - 64.0, feet - gap - 16.0),
            ae::Vec2::new(128.0, 16.0),
        ));
    }
    assert!(
        gap < standing_height,
        "fixture: the gap ({gap:.1}) must be too short for her standing height \
         ({standing_height:.1}), or nothing is being refused"
    );

    // Release DOWN under the ceiling. She asks to stand and must be refused.
    let neutral = ControlFrame::default();
    let mut seen: Vec<f32> = Vec::new();
    for tick in 0..20 {
        step(&mut app, neutral.clone());
        let now = body(&mut app);
        seen.push(now.pos.y);
        assert!(
            now.size.y <= crouched.size.y + 0.01,
            "tick {tick}: she stood up to {:.1} under a {gap:.1}-unit ceiling — a \
             blocked stand must be refused, not performed and then collided out of",
            now.size.y
        );
    }

    // AND SHE HOLDS STILL. A body that is refused the stand does not move; the
    // reported symptom was the oscillation, so a test that only checked the
    // height could pass while she vibrated.
    let lowest = seen.iter().cloned().fold(f32::INFINITY, f32::min);
    let highest = seen.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    assert!(
        highest - lowest < 1.0,
        "she moved {:.1} units up and down while blocked from standing \
         ({lowest:.1}..{highest:.1}) — that is the spaz",
        highest - lowest
    );
}

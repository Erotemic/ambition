#![cfg(all(feature = "input", feature = "visible"))]

//! test that answers this in `ambition_demo_mary_o_app` is GREEN: warped to each
//! authored pole it goes 1-1 → 1-2 → 1-3 → 1-1, right level every leg. So does
//! every other Mary-O test. And he sees 1-1 → 1-1.
//!
//! because none of them build the composition that ships. He reaches Mary-O through
//! `run_game.sh` → the launcher → the shell host, and `build_visible_app` sets
//! `SimulationHost::Rollback` for every route. The standalone demo binary is
//! `PlatformerEnginePlugins::fixed_tick()`.
//!
//! a demo binary is not a coverage argument for the game. That is the
//! lesson this file exists to hold, and it is worth more than the assertion
//! below: whenever a road is "tested" only by `build_demo_app()`, the shipped
//! path through the launcher is untested no matter how green the suite is.

use bevy::prelude::*;

use ambition_app::app::shell_host;
use ambition_demo_mary_o::level_1_2::LEVEL_1_2_ROOM_ID;
use ambition_demo_mary_o::LEVEL_1_1_ROOM_ID;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::game_shell::{ShellLaunchCatalog, ShellLauncherCommand};
use ambition_platformer2d::platformer::markers::PrimaryPlayer;
use ambition_platformer2d::world::rooms::RoomSet;

/// Frames a leg may take before we call the transition wedged.
const COMMIT_CAP: usize = 900;

fn host_app() -> App {
    use bevy::asset::AssetPlugin;
    use bevy::image::ImagePlugin;
    use bevy::state::app::StatesPlugin;
    use bevy::transform::TransformPlugin;
    use bevy::MinimalPlugins;

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(AssetPlugin::default());
    app.add_plugins(ImagePlugin::default());
    app.add_plugins(TransformPlugin);
    app.add_plugins(StatesPlugin);
    app.init_state::<ambition_platformer2d::platformer::schedule::GameMode>();
    app.insert_resource(shell_host::AmbitionShellHosted);
    // the whole point of this file. Without it the test falls to the
    // render-frame default and stops being the shipped composition.
    use ambition_platformer2d::runtime::SimulationHostAppExt as _;
    app.set_simulation_host(ambition_platformer2d::runtime::SimulationHost::Rollback);
    ambition_app::app::init_sandbox_resources(&mut app);
    ambition_app::app::add_simulation_plugins(&mut app);
    app.add_plugins(ambition_platformer2d::host::PlatformerHostPlugins);
    shell_host::compose_ambition_shell_host(&mut app);
    // and the clock has to move. Under GGRS the simulation advances on the
    // fixed timestep; a test that calls `update()` inside a millisecond of real
    // time runs the GGRS schedule zero times and every reading is the boot state.
    let timestep = app.world().resource::<Time<Fixed>>().timestep();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(timestep));
    app
}

fn room_id(app: &mut App) -> Option<String> {
    let mut q = app.world_mut().query::<&RoomSet>();
    q.iter(app.world())
        .next()
        .map(|set| set.rooms[set.active].id.clone())
}

/// Launch the row with this label, the way the launcher does.
fn launch(app: &mut App, label: &str) {
    let (index, total) = {
        let catalog = app.world().resource::<ShellLaunchCatalog>();
        let index = catalog
            .entries
            .iter()
            .position(|entry| entry.label == label)
            .unwrap_or_else(|| {
                let offered: Vec<&str> = catalog.entries.iter().map(|e| e.label.as_str()).collect();
                panic!("the launcher offers no `{label}` row; it offers {offered:?}")
            });
        let exit = app
            .world()
            .resource::<ambition_platformer2d::game_shell::ShellLauncherPresentation>()
            .exit_label
            .is_some() as usize;
        (index, catalog.entries.len() + exit)
    };
    let current = app
        .world()
        .resource::<ambition_platformer2d::game_shell::ShellLauncherState>()
        .selected;
    for _ in 0..((index + total - current % total) % total) {
        app.world_mut().write_message(ShellLauncherCommand::Next);
        app.update();
    }
    app.world_mut()
        .write_message(ShellLauncherCommand::LaunchSelected);
    for _ in 0..240 {
        app.update();
        if room_id(app).is_some() {
            return;
        }
    }
    panic!("launching `{label}` from the host launcher never opened a room");
}

/// Finish `from` by touching its OWN authored pole; report where she landed.
fn finish_at_the_pole(app: &mut App, from: &str) -> String {
    let pole = ambition_demo_mary_o::pole_for_room(from);
    {
        let mut q = app
            .world_mut()
            .query_filtered::<&mut ae::BodyKinematics, With<PrimaryPlayer>>();
        let world = app.world_mut();
        let mut body = q
            .iter_mut(world)
            .next()
            .unwrap_or_else(|| panic!("no controlled body in '{from}' to carry to its pole"));
        body.pos = ae::Vec2::new(pole.x, pole.base_y - 8.0);
        body.vel = ae::Vec2::ZERO;
    }
    for _ in 0..COMMIT_CAP {
        app.update();
        if let Some(id) = room_id(app) {
            if id != from {
                for _ in 0..30 {
                    app.update();
                }
                return id;
            }
        }
    }
    panic!(
        "in the HOST, touching the authored pole of '{from}' never changed the \
         room within {COMMIT_CAP} frames. ⚠ this is the composition Jon plays; \
         the same walk in `ambition_demo_mary_o_app` is green."
    );
}

/// Finishing 1-1 in the host lands her in 1-2, not back in 1-1.
#[test]
fn finishing_the_first_level_in_the_host_lands_in_the_second() {
    let mut app = host_app();
    for _ in 0..8 {
        app.update();
    }
    launch(&mut app, "Mary-O");
    let opened = room_id(&mut app).expect("Mary-O opened a room");
    assert_eq!(
        opened, LEVEL_1_1_ROOM_ID,
        "the host launched Mary-O into '{opened}' rather than 1-1",
    );

    let landed = finish_at_the_pole(&mut app, LEVEL_1_1_ROOM_ID);
    assert_eq!(
        landed, LEVEL_1_2_ROOM_ID,
        "finishing 1-1 in the HOST put her in '{landed}'. ⛔ if that is 1-1 \
         itself, `exit_for_room` answered `Replay` on the shipped path while \
         answering correctly in the demo binary — which is the shape of every \
         host-only bug in this tree.",
    );
    let mut q = app
        .world_mut()
        .query_filtered::<&ae::BodyKinematics, With<PrimaryPlayer>>();
    let world = app.world_mut();
    assert!(
        q.iter(world).next().is_some(),
        "she arrived in 1-2 and there is no controlled body in it",
    );
}

/// ```text
///   write ControlFrame between update()s   ran, ignored — device systems rewrite it
///   a system .before(PrimarySlotInputCommit)  ran 5062x, slot 0 still read 0.0
///   into the latch on the feel clock       ran 5400x, slot 0 still read 0.0
///   drive_control_frame(world, frame)      slot 0 reads 1.0, and she MOVES
/// ```
///
/// `PendingLocalInput` is the input side there, and `drive_control_frame` picks whichever the
/// host wants. The latch is consulted only `if latch.is_device_authority()`, which is false
/// with no device wired — so a headless press into it is dropped.
///
/// this does NOT walk her to the pole, and an earlier draft that tried was
/// dishonest: holding right with a hop every 24 frames is not platforming, and
/// she got 750 of 3144 units before the cap. Completing 1-1 blind is not
/// something a test can claim. What IS worth guarding is the road — that a press
/// reaches the body at all on the shipped host — because that road being
/// silently dead is exactly the class of thing that reads as "the game ignores
/// me" from the couch.
#[test]
fn a_press_on_seat_zero_moves_her_in_the_shell_host() {
    let mut app = host_app();
    for _ in 0..8 {
        app.update();
    }
    launch(&mut app, "Mary-O");
    let opened = room_id(&mut app).expect("Mary-O opened a room");
    assert_eq!(opened, LEVEL_1_1_ROOM_ID);

    app.insert_resource(HeldPress::default());
    {
        use ambition_platformer2d::platformer::schedule::SimScheduleExt as _;
        let schedule = app.sim_schedule();
        app.add_systems(
            schedule,
            observe_from_inside_the_sim
                .after(ambition_platformer2d::actors::control::PrimarySlotInputCommit),
        );
    }

    let start_x = body_x(&mut app).expect("a controlled body in 1-1");
    for frame in 0..600 {
        let mut f = ambition_platformer2d::input::ControlFrame::default();
        f.axis_x = 1.0;
        f.right_pressed = true;
        f.jump_pressed = frame % 24 == 0;
        f.jump_held = frame % 24 < 8;
        ambition_platformer2d::sim::drive_control_frame(app.world_mut(), f);
        app.update();
    }
    let end_x = body_x(&mut app).expect("she still has a body");
    let seen = *app.world().resource::<HeldPress>();

    // THE PRECONDITIONS, because each one silently empties the assertion.
    assert_eq!(
        seen.driving_bodies, 1,
        "no body is driven by a participant, so nothing could have read the press"
    );
    assert!(
        seen.seen_slot_x > 0.5,
        "the press never reached slot 0 inside the sim (SlotControls[PRIMARY].axis_x \
         = {:.2}, ControlFrame.axis_x = {:.2}). ⇒ the INPUT ROAD is broken on this \
         host, which is a bigger fact than whatever this test was measuring.",
        seen.seen_slot_x,
        seen.seen_frame_x,
    );
    assert!(
        end_x - start_x > 100.0,
        "seat 0 held right for 600 frames on the shipped host and she moved \
         {:.0} units (x {start_x:.0} -> {end_x:.0}). The press DID reach slot 0 \
         (axis_x = {:.2}), so the break is between the slot and her body.",
        end_x - start_x,
        seen.seen_slot_x,
    );
}

fn body_x(app: &mut App) -> Option<f32> {
    let mut q = app
        .world_mut()
        .query_filtered::<&ae::BodyKinematics, With<PrimaryPlayer>>();
    let world = app.world_mut();
    q.iter(world).next().map(|body| body.pos.x)
}

/// What the in-sim probe saw, recorded from inside the sim schedule.
///
/// read there rather than from the test loop on purpose: `SlotControls` can be
/// session-scoped, so a top-level `get_resource` may answer about a different
/// instance than the one the bodies read.
#[derive(Resource, Default, Clone, Copy)]
struct HeldPress {
    seen_frame_x: f32,
    seen_slot_x: f32,
    driving_bodies: usize,
}

/// Read the frame and the slot from INSIDE the sim, after the commit set.
fn observe_from_inside_the_sim(
    frame: Res<ambition_platformer2d::input::ControlFrame>,
    slots: Option<Res<ambition_platformer2d::characters::control::SlotControls>>,
    driving: Query<&ambition_platformer2d::characters::control::DrivingParticipant>,
    mut held: ResMut<HeldPress>,
) {
    held.seen_frame_x = frame.axis_x;
    held.seen_slot_x = slots
        .map(|s| {
            s.get(ambition_platformer2d::characters::control::PlayerSlot::PRIMARY)
                .axis_x
        })
        .unwrap_or(f32::NAN);
    held.driving_bodies = driving.iter().count();
}

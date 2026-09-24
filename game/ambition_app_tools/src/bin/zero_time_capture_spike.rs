//! Can a GPU readback complete while the simulation is frozen?
//!
//! The move renderer depends on this. A readback is asynchronous, so a driver
//! calls `App::update()` until it lands. With the ordinary period the sim
//! advances during the whole GPU wait (`capture_scene --frames` spaces shots
//! by stride plus GPU time), so a move animation skips frames.
//!
//! The fix is to pump with `ManualDuration(ZERO)`: Bevy advances its clocks
//! by exactly the given duration, so the schedules run and the clocks do not
//! move. `zero_duration_pump` proves the clock half without a GPU; this
//! proves that a real offscreen readback completes under those pumps.
//!
//! A binary, not a test, because it needs a GPU and the ordinary suite must
//! run without a renderer.
//!
//! It counts the pumps. Bevy 0.18's `Readback` re-attempts every render frame
//! until its component is removed, and `request_capture` keeps the entity
//! until completion, so N pumps can enqueue N copies. If the count is large,
//! make the request one-shot before building an animation loop on it.

use ambition_platformer2d::game_shell::{ShellCommand, ShellRouteId};
use bevy::prelude::*;

fn sim_tick(app: &App) -> u64 {
    app.world()
        .get_resource::<ambition_platformer2d::runtime::SimTick>()
        .map(|t| t.0)
        .unwrap_or_default()
}

fn main() {
    let out = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/zero_time_capture.png".to_string());

    let mut app = ambition_app::app::build_visible_app_with(
        ambition_app::app::VisibleRenderMode::OffscreenGpu,
        // `true` boots the launcher (the `--route` capture arm) and needs no
        // `StartRoomOverride`. `false` boots a gameplay room, needs one, and
        // without it never builds a render device.
        true,
        |_app| {},
    );
    // Without this there is no render device: `build_visible_app_with`
    // (OffscreenGpu) alone panics in `bevy_pbr`'s skin batching
    // (*"Res<RenderDevice> failed validation: Resource does not exist"*),
    // because the offscreen surface has no size. `capture_scene` sets it
    // right after the builder.
    app.insert_resource(
        ambition_platformer2d::host::gameplay_presentation::HeadlessDisplaySurface(
            ambition_platformer2d::engine_core::Vec2::new(480.0, 360.0),
        ),
    );
    app.insert_resource(ambition_platformer2d::render::capture::CaptureSettings {
        output: std::path::PathBuf::from(&out),
        size: UVec2::new(480, 360),
        include_ui: false,
    });
    app.init_resource::<ambition_platformer2d::render::capture::CaptureProgress>();
    app.add_systems(
        Startup,
        ambition_platformer2d::render::capture::setup_capture_target
            .after(ambition_app::app::PresentationSetupSet),
    );
    app.add_systems(Update, ambition_platformer2d::render::capture::adopt_cameras_into_capture_target);

    // A hand-driven app must be finalized first. Bevy builds the render
    // device in plugin `finish()`, which `App::run()` performs and a manual
    // `update()` loop does not. `capture_scene` uses `app.run()`, so its
    // runner owns the loop and cannot capture on exact ticks. `finalize` is
    // the repo's seam for this.
    ambition_platformer2d::runtime::finalize(&mut app);

    let canonical = ambition_platformer2d::sim::enable_manual_stepping(&mut app);
    for _ in 0..30 {
        app.update();
    }
    app.world_mut()
        .insert_resource(ambition_demo_smash::smash_roster([
            "npc_pirate_admiral",
            "npc_pirate_admiral",
        ]));
    app.world_mut()
        .write_message(ShellCommand::GoTo(ShellRouteId::new(
            ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
        )));

    let mut live = false;
    for _ in 0..900 {
        app.update();
        let staged = {
            let world = app.world_mut();
            let mut all = world.query::<&ambition_platformer2d::actor::MatchSeat>();
            all.iter(world).count() > 0
        };
        if staged && ambition_platformer2d::rollback::session_is_active(app.world()) {
            live = true;
            break;
        }
    }
    assert!(live, "no live rollback session — nothing below is about a running sim");

    let at_tick = sim_tick(&app);
    println!("[spike] requesting a capture at SimTick {at_tick}");
    {
        let world = app.world_mut();
        let target = world
            .remove_resource::<ambition_platformer2d::render::capture::CaptureTarget>()
            .expect("the capture target exists once Startup has run");
        let mut progress = world
            .remove_resource::<ambition_platformer2d::render::capture::CaptureProgress>()
            .unwrap_or_default();
        let mut commands = world.commands();
        ambition_platformer2d::render::capture::request_capture(&mut commands, &target, &mut progress);
        world.insert_resource(target);
        world.insert_resource(progress);
        world.flush();
    }

    // ── Pump at zero cost ──
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::ZERO,
    ));
    let mut pumps = 0usize;
    let mut done = false;
    while pumps < 600 {
        app.update();
        pumps += 1;
        assert_eq!(
            sim_tick(&app),
            at_tick,
            "a zero-duration pump advanced the simulation on pump {pumps} — the \
             PNG could then no longer name the tick it was taken on"
        );
        if app
            .world()
            .get_resource::<ambition_platformer2d::render::capture::CaptureProgress>()
            .is_some_and(|p| p.completed)
        {
            done = true;
            break;
        }
    }

    if !done {
        println!("[spike] FAIL — the readback never completed in {pumps} zero-time pumps");
        std::process::exit(1);
    }
    println!("[spike] readback completed after {pumps} zero-time pump(s), SimTick still {at_tick}");

    // ── And the clock resumes ──
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(canonical));
    app.update();
    let after = sim_tick(&app);
    if after != at_tick + 1 {
        println!("[spike] FAIL — restoring the canonical period advanced {} tick(s), not 1", after - at_tick);
        std::process::exit(1);
    }
    let wrote = std::path::Path::new(&out).exists();
    println!("[spike] canonical period resumed: one update, one tick ({at_tick} -> {after})");
    println!("[spike] PNG written: {wrote} ({out})");
    println!("[spike] PASS");
}

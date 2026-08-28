//! ⭐⭐ CAN A GPU READBACK COMPLETE WHILE THE SIMULATION IS FROZEN?
//!
//! The whole move-renderer design rests on this. A readback is asynchronous, so
//! a driver must keep calling `App::update()` until it lands — and every
//! existing multi-shot driver does that with the ORDINARY period, so the sim
//! advances for the entire GPU wait. `capture_scene --frames` therefore spaces
//! its shots by `stride + however long the GPU took`, which for a move animation
//! means startup or active frames pass while a PNG is in flight.
//!
//! The proposed fix is to pump with `ManualDuration(ZERO)`: Bevy advances its
//! clocks by exactly the duration given, so the schedules still run and the
//! clocks do not move. `zero_duration_pump` already proves the CLOCK half with
//! no GPU. This proves the half that needs one — that a real offscreen readback
//! actually completes under those pumps.
//!
//! ⛔ A BINARY, NOT A TEST, because it needs a GPU and a machine without a
//! renderer must stay a supported environment for the ordinary suite.
//!
//! ⚠ AND IT COUNTS THE PUMPS. Bevy 0.18's `Readback` re-attempts EVERY RENDER
//! FRAME until its component is removed, and this repo's `request_capture`
//! leaves the entity alive until completion — so N pumps can enqueue N copies of
//! the same texture. If the count is trivial that is operationally harmless; if
//! it is large, the request must be made genuinely one-shot before an animation
//! loop is built on it.

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
        // ⛔ `true` BOOTS THE LAUNCHER, which is the arm a `--route` capture uses
        // and needs no `StartRoomOverride`. `false` boots straight to a gameplay
        // ROOM and requires one; without it the composition takes a path that
        // never builds a render device.
        true,
        |_app| {},
    );
    // ⛔⛔ WITHOUT THIS THERE IS NO RENDER DEVICE. `build_visible_app_with`
    // (OffscreenGpu) alone panics in `bevy_pbr`'s skin batching with
    // *"Res<RenderDevice> failed validation: Resource does not exist"* — the
    // offscreen surface has no size to build itself from. `capture_scene` sets
    // it immediately after the builder and that is why it works; I hit the same
    // panic earlier switching `moveset_takes` to OffscreenGpu and mistook it for
    // the mode being unusable.
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

    // ⛔⛔ A HAND-DRIVEN APP MUST BE FINALIZED FIRST, and this is why the naive
    // `OffscreenGpu` attempt panicked in `bevy_pbr`'s skin batching with
    // *"Res<RenderDevice> failed validation"*. Bevy builds the render device in
    // plugin `finish()`, which `App::run()` performs and a manual `update()` loop
    // never does. `capture_scene` calls `app.run()` and therefore never met this
    // — and that is also precisely why it cannot be reused for tick-exact
    // capture: its runner owns the loop, so a driver cannot decide what a frame
    // costs. `finalize` is the repo's own seam for exactly this.
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

    // ── PUMP AT ZERO COST ──
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

    // ── AND THE CLOCK RESUMES ──
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

// no `bevy::prelude::*` here on purpose: every call this binary makes —
// `App::update`, `App::world`, `World::resource` — is an inherent method on a
// type the demo crate hands back, so the glob import was unused and the
// workspace's no-warnings gate refused it. The dependency stays in the manifest
// because the `visible` path builds a windowed app.

const DEFAULT_TICKS: u32 = 300;

fn main() {
    #[cfg(feature = "visible")]
    if ambition_platformer2d::demo_shell::wants_a_window() {
        ambition_demo_twintrack_app::build_windowed_demo_app().run();
        return;
    }

    let ticks = ambition_platformer2d::demo_shell::headless_ticks(DEFAULT_TICKS);
    let mut app = ambition_demo_twintrack_app::build_demo_app();
    for _ in 0..ticks {
        app.update();
    }
    let view = app
        .world()
        .resource::<ambition_platformer2d::relativity2d::RelativityClockView2d>();
    println!("TwinTrack SR demo — {ticks} fixed host updates");
    println!("  spacetime: {}", view.model_id.unwrap_or("not active"));
    for (label, clock) in &view.clocks {
        println!(
            "  {label:10}: proper_time={:.6}s  speed={:.2}  rate={:.6}",
            clock.proper_time_seconds,
            clock.relative_velocity.length(),
            clock.proper_time_rate,
        );
    }
    println!("Run with `--features visible -- --window` to play the round trip.");
}

//! Outlander in a window — the "visibly" half of Phase 6's "runs visibly and
//! headlessly from the same content". The provider, routes, and session
//! lifecycle are byte-for-byte the headless binary's — both mount the same
//! `OutlanderModule`; only the host FACE differs, and that is now one builder
//! call rather than a composition this binary had to spell out.
//!
//! The game declares its own `game://` asset source. It selects no shared
//! content library, so the artifact needs only this game's assets.

fn main() {
    // `--smoke <frames>`: the same composition with no GPU, driven for a fixed
    // number of frames, so a packaged build can check itself with no display.
    let args: Vec<String> = std::env::args().collect();
    if let Some(at) = args.iter().position(|arg| arg == "--smoke") {
        let frames = args.get(at + 1).and_then(|n| n.parse().ok()).unwrap_or(300);
        match outlander::smoke_windowed_app(frames) {
            Ok(report) => {
                println!(
                    "outlander smoke: session active after {} ticks; own sprite loaded: {}; failed asset loads: {}",
                    report.walk.ticks_to_activate,
                    report.own_sprite_loaded,
                    report.failed_loads.len()
                );
                for failure in &report.failed_loads {
                    println!("  FAILED {failure}");
                }
                if !report.own_sprite_loaded || !report.failed_loads.is_empty() {
                    std::process::exit(1);
                }
            }
            Err(error) => {
                eprintln!("outlander smoke: FAILED: {error}");
                std::process::exit(1);
            }
        }
        return;
    }
    // The composition lives in the lib so the headless render test builds the
    // SAME app. A `main` a test cannot call is a composition nothing
    // verifies.
    outlander::build_windowed_app(true).run();
}

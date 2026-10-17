//! The sprite sheet watch looks at the tree when the operating system reports
//! a change in it, not on a clock: a look is a stat of each sheet, rig,
//! flipbook and page file (0.2 to 0.4 s in a debug build), so a clock spends
//! that each second while nothing changes.
//!
//! Its own test binary: a publication changes what every reader of the
//! process reads.

#![cfg(feature = "content_pack")]

use ambition_platformer2d::content::sprite_reload::{SpriteSheetReloadPlugin, SpriteSheetWatch};
use bevy::prelude::*;
use std::time::{Duration, Instant};

const PROBE: &str = "sprite_watch_reports_probe";

fn sheet_text(collision_scale: &str) -> String {
    format!(
        r#"[(
    target: "{PROBE}",
    image: "{PROBE}_spritesheet.png",
    label_width: 0,
    frame_width: 32,
    frame_height: 32,
    tuning: Some((collision_scale: {collision_scale}, frame_sample_inset: 0)),
    rows: [],
    start: 0.0,
    end: 1.0,
)]"#
    )
}

/// Update until `done` or `longest` passes.
fn update_until(app: &mut App, longest: Duration, done: impl Fn(&App) -> bool) {
    let start = Instant::now();
    while !done(app) && start.elapsed() < longest {
        std::thread::sleep(Duration::from_millis(20));
        app.update();
    }
}

#[test]
fn the_watch_looks_when_a_file_changes_and_not_while_none_does() {
    let owner = std::env::temp_dir().join(format!("sprite_watch_reports_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&owner);
    let sprites = owner.join("assets/sprites");
    std::fs::create_dir_all(&sprites).unwrap();
    let manifest = sprites.join(format!("{PROBE}_spritesheet.ron"));
    std::fs::write(&manifest, sheet_text("1.5")).unwrap();

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<ambition_platformer2d::actor::ActiveSessionScope>();
    app.init_resource::<ambition_sprite_sheet::SheetRegistry>();
    app.add_plugins(SpriteSheetReloadPlugin { owner: Some(owner.clone()), manual: false });
    let looks = |app: &App| app.world().resource::<SpriteSheetWatch>().looks();
    update_until(&mut app, Duration::from_secs(5), |app| looks(app) >= 1);
    assert_eq!(looks(&app), 1, "the premise: the thread takes its baseline");

    // Nothing changes for three seconds: a clock of one second looks three
    // more times.
    let quiet = Instant::now();
    while quiet.elapsed() < Duration::from_secs(3) {
        std::thread::sleep(Duration::from_millis(50));
        app.update();
    }
    assert_eq!(looks(&app), 1, "the watch looked while no file changed");

    // A changed file is seen and published (no session is live).
    std::fs::write(&manifest, sheet_text("2.25")).unwrap();
    update_until(&mut app, Duration::from_secs(5), |app| app.world().resource::<SpriteSheetWatch>().published == 1);
    assert_eq!(app.world().resource::<SpriteSheetWatch>().published, 1, "the changed file was not seen");
    assert_eq!(looks(&app), 2, "one save is one look");
    let _ = std::fs::remove_dir_all(&owner);
}

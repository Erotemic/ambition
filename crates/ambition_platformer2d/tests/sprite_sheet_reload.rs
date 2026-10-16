//! A running game plays an edit to a sprite sheet from the next session: a
//! changed manifest waits while a session is live, is published as one sheet
//! generation when it ends, and states the `SheetRegistry` resource again; a
//! manifest that does not parse is refused and the running sheets stay.
//!
//! Its own test binary: a publication changes what every reader of the
//! process reads, and this one publishes a probe tree of one sheet.

#![cfg(feature = "content_pack")]

use ambition_platformer2d::content::sprite_reload::{SpriteSheetReloadPlugin, SpriteSheetWatch};
use ambition_sprite_sheet::baked_sheet_rons::current;
use bevy::prelude::*;

const PROBE: &str = "sprite_reload_probe";

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

fn collision_scale(app: &App) -> (f32, f32) {
    let of = |registry: &ambition_sprite_sheet::SheetRegistry| {
        registry.get(PROBE).and_then(|record| record.tuning.as_ref()).map(|tuning| tuning.collision_scale).unwrap_or(f32::NAN)
    };
    (of(current().registry()), of(app.world().resource::<ambition_sprite_sheet::SheetRegistry>()))
}

/// Look at the files once more, and take what the look found.
fn look(app: &mut App) {
    app.world().resource::<SpriteSheetWatch>().look_now();
    app.update();
}

#[test]
fn an_edited_sheet_is_played_from_the_next_session_and_a_broken_one_is_refused() {
    let owner = std::env::temp_dir().join(format!("sprite_reload_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&owner);
    let sprites = owner.join("assets/sprites");
    std::fs::create_dir_all(&sprites).unwrap();
    let manifest = sprites.join(format!("{PROBE}_spritesheet.ron"));
    std::fs::write(&manifest, sheet_text("1.5")).unwrap();

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<ambition_platformer2d::actor::ActiveSessionScope>();
    app.init_resource::<ambition_sprite_sheet::SheetRegistry>();
    app.add_plugins(SpriteSheetReloadPlugin { owner: Some(owner.clone()), manual: true });
    look(&mut app);
    assert!(!app.world().resource::<SpriteSheetWatch>().is_staged(), "control: an unchanged tree stages nothing");
    let first = current().id();

    // Inside a session the edit waits: its records feed simulation.
    app.world_mut().resource_mut::<ambition_platformer2d::actor::ActiveSessionScope>().begin();
    std::thread::sleep(std::time::Duration::from_millis(20));
    std::fs::write(&manifest, sheet_text("2.25")).unwrap();
    look(&mut app);
    assert_eq!(current().id(), first, "an edit was published inside a session");
    assert!(app.world().resource::<SpriteSheetWatch>().is_staged(), "the edit was not seen");

    // The session ends: the edit is the next session's sheet.
    app.world_mut().resource_mut::<ambition_platformer2d::actor::ActiveSessionScope>().clear();
    app.update();
    assert_eq!(current().id(), first + 1, "the edit was not published when the session ended");
    assert_eq!(collision_scale(&app), (2.25, 2.25), "the generation or the SheetRegistry resource is not the edited sheet");
    assert_eq!(app.world().resource::<SpriteSheetWatch>().published, 1);

    // A manifest that does not parse is refused; the running sheets stay.
    std::thread::sleep(std::time::Duration::from_millis(20));
    std::fs::write(&manifest, "[ (target: ").unwrap();
    look(&mut app);
    let watch = app.world().resource::<SpriteSheetWatch>();
    assert!(watch.refused.as_deref().is_some_and(|why| why.contains(PROBE)), "the refusal does not name the sheet: {:?}", watch.refused);
    assert_eq!(current().id(), first + 1, "a broken sheet was published");
    assert_eq!(collision_scale(&app), (2.25, 2.25), "a broken sheet changed the running sheets");
    let _ = std::fs::remove_dir_all(&owner);
}

/// The cost of one look at the published tree: the manifest scan and a stat
/// of each manifest and page (the background thread pays it, not a frame). Run by
/// hand: `cargo test -p ambition_platformer2d --features content_pack --test
/// sprite_sheet_reload -- --ignored --nocapture`.
#[test]
#[ignore]
fn measure_one_look_at_the_published_tree() {
    let Some(owner) = ambition_sprite_sheet::baked_sheet_rons::published_tree_owner() else {
        return;
    };
    let _ = current();
    let mut samples = Vec::new();
    for _ in 0..7 {
        let start = std::time::Instant::now();
        let watch = SpriteSheetWatch::new(owner.to_path_buf());
        watch.look_now();
        samples.push(start.elapsed().as_secs_f64() * 1000.0);
        std::hint::black_box(watch);
    }
    samples.sort_by(f64::total_cmp);
    println!("one look: median {:.2} ms, range {:.2}-{:.2} ms (7 samples)", samples[3], samples[0], samples[6]);
    // The look that sees a changed manifest reads the whole table again and
    // parses it, off the frame.
    let mut prepared = Vec::new();
    let mut samples = Vec::new();
    for _ in 0..7 {
        let start = std::time::Instant::now();
        let table = ambition_sprite_sheet::baked_sheet_rons::read_sheet_table_under(owner);
        let table = ambition_sprite_sheet::baked_sheet_rons::prepare_sheet_table(table).expect("the published tree parses");
        samples.push(start.elapsed().as_secs_f64() * 1000.0);
        prepared.push(table);
    }
    samples.sort_by(f64::total_cmp);
    println!("read and parse the table: median {:.2} ms, range {:.2}-{:.2} ms (7 samples)", samples[3], samples[0], samples[6]);
    // The frame at the end of the session only swaps the table in and clones
    // its registry into the resource.
    let mut samples = Vec::new();
    for table in prepared {
        let start = std::time::Instant::now();
        ambition_sprite_sheet::baked_sheet_rons::publish_prepared_sheet_table(table);
        let registry = ambition_sprite_sheet::baked_sheet_registry();
        samples.push(start.elapsed().as_secs_f64() * 1000.0);
        std::hint::black_box(registry);
    }
    samples.sort_by(f64::total_cmp);
    println!("publish on the frame: median {:.2} ms, range {:.2}-{:.2} ms (7 samples)", samples[3], samples[0], samples[6]);
}

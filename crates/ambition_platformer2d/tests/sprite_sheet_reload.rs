//! A running game plays an edit to a sprite sheet from the next session: a
//! changed manifest waits while a session is live, is published as one sheet
//! generation when it ends, and states the `SheetRegistry` resource again; an
//! edit to where frames are drawn from only is played at once; a manifest
//! that does not parse is refused and the running sheets stay.
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

    // An edit to where frames are drawn from only (here the label band; a
    // repack moves rects and pages) is played at once, also inside a
    // session: its pages are loaded again at once, so the old rects would
    // cut the new pixels. An edit to a record that simulation reads waits.
    let session = |app: &mut App, live: bool| {
        let mut scope = app.world_mut().resource_mut::<ambition_platformer2d::actor::ActiveSessionScope>();
        if live { scope.begin(); } else { scope.clear(); }
    };
    let label_width = |app: &App| {
        let of = |registry: &ambition_sprite_sheet::SheetRegistry| registry.get(PROBE).map(|record| record.label_width);
        (of(current().registry()), of(app.world().resource::<ambition_sprite_sheet::SheetRegistry>()))
    };
    session(&mut app, true);
    std::thread::sleep(std::time::Duration::from_millis(20));
    std::fs::write(&manifest, sheet_text("2.25").replace("label_width: 0", "label_width: 8")).unwrap();
    look(&mut app);
    assert_eq!(current().id(), first + 2, "an edit to where frames are drawn from waited for the end of the session");
    assert_eq!(label_width(&app), (Some(8), Some(8)), "the generation or the SheetRegistry resource is not the edited sheet");
    std::thread::sleep(std::time::Duration::from_millis(20));
    std::fs::write(&manifest, sheet_text("3.0").replace("label_width: 0", "label_width: 8")).unwrap();
    look(&mut app);
    assert_eq!((current().id(), collision_scale(&app)), (first + 2, (2.25, 2.25)), "control: an edit to a record simulation reads was published inside a session");
    // The table is read whole: a later edit that takes the mechanical change
    // back is a change of the label band alone, and is played at once.
    std::thread::sleep(std::time::Duration::from_millis(20));
    std::fs::write(&manifest, sheet_text("2.25")).unwrap();
    look(&mut app);
    assert_eq!((current().id(), label_width(&app), collision_scale(&app)), (first + 3, (Some(0), Some(0)), (2.25, 2.25)));
    assert!(!app.world().resource::<SpriteSheetWatch>().is_staged(), "the superseded edit is still staged");
    session(&mut app, false);
    app.update();
    assert_eq!((current().id(), app.world().resource::<SpriteSheetWatch>().published), (first + 3, 3));

    // A manifest that does not parse is refused; the running sheets stay.
    std::thread::sleep(std::time::Duration::from_millis(20));
    std::fs::write(&manifest, "[ (target: ").unwrap();
    look(&mut app);
    let watch = app.world().resource::<SpriteSheetWatch>();
    assert!(watch.refused.as_deref().is_some_and(|why| why.contains(PROBE)), "the refusal does not name the sheet: {:?}", watch.refused);
    assert_eq!(current().id(), first + 3, "a broken sheet was published");
    assert_eq!(collision_scale(&app), (2.25, 2.25), "a broken sheet changed the running sheets");

    // A part flipbook is a part of the generation: its decoded table and the
    // landmarks projected from it are the saved file's. A real flipbook that
    // tracks a hand, under the probe's name.
    use ambition_sprite_sheet::baked_landmarks::{body_landmarks, landmarks_digest};
    use ambition_sprite_sheet::character::rigged::RiggedSpriteAsset;
    let shipped = ambition_sprite_sheet::baked_sheet_rons::published_tree_owner()
        .expect("a development build reads its sprite tree")
        .join("assets/sprites/hand_saint_parts.ron");
    let flipbook_text = std::fs::read_to_string(shipped).expect("the hand saint publishes a flipbook").replace("target: \"hand_saint\"", &format!("target: \"{PROBE}\""));
    let flipbook = sprites.join(format!("{PROBE}_parts.ron"));
    // The feet are read from the decoded table, the frame height by the
    // landmarks.
    let feet = |x: &str| {
        flipbook_text
            .replace("feet_pixel: (104.0, 199.0)", &format!("feet_pixel: ({x}, 199.0)"))
            .replace("frame_size: (232, 208)", if x == "104.0" { "frame_size: (232, 208)" } else { "frame_size: (232, 216)" })
    };
    assert_eq!(flipbook_text.matches("frame_size: (232, 208)").count(), 1, "the probe edits one frame size");
    std::fs::write(&flipbook, feet("104.0")).unwrap();
    std::fs::write(&manifest, sheet_text("2.25")).unwrap();
    look(&mut app);
    app.update();
    assert_eq!(app.world().resource::<SpriteSheetWatch>().refused, None);
    let feet_x = || RiggedSpriteAsset::baked(PROBE).map(|table| table.feet_pixel.x);
    assert_eq!(feet_x(), Some(104.0), "an added flipbook is not decoded from the published generation");
    assert!(body_landmarks(PROBE).is_some(), "the added flipbook's landmarks are not projected");
    let digest = landmarks_digest().to_owned();

    std::thread::sleep(std::time::Duration::from_millis(20));
    std::fs::write(&flipbook, feet("120.0")).unwrap();
    look(&mut app);
    app.update();
    assert_eq!(feet_x(), Some(120.0), "the decoded flipbook is not the saved file");
    assert_ne!(landmarks_digest(), digest, "the landmarks are not projected from the saved flipbook");

    std::thread::sleep(std::time::Duration::from_millis(20));
    std::fs::write(&flipbook, "( schema_version: 3, target: ").unwrap();
    look(&mut app);
    let refused = app.world().resource::<SpriteSheetWatch>().refused.clone();
    assert!(refused.as_deref().is_some_and(|why| why.contains(&format!("{PROBE}_parts.ron"))), "the refusal does not name the flipbook: {refused:?}");
    assert_eq!(feet_x(), Some(120.0), "a broken flipbook changed the running one");
    std::fs::write(&flipbook, feet("120.0")).unwrap();

    // A portrait manifest and a sprite pack catalog are parts of it too: the
    // `PortraitSheetRegistry` resource is stated again with the generation.
    use ambition_sprite_sheet::portrait::{available_portrait_targets, PortraitSheetRegistry};
    use ambition_sprite_sheet::sprite_packs::catalog_for_tier;
    let portrait = sprites.join(format!("{PROBE}_portraits.ron"));
    let portrait_key = format!("sprites/{PROBE}_portraits.ron");
    let portrait_text = |width: u32| {
        format!(
            r#"(target: "{PROBE}", image: "{PROBE}_portraits.png", frame_width: {width}, frame_height: 320, default_clip: "default",
                clips: {{ "default": (duration_ms: 0, looping: false, frames: [(x: 0, y: 0, w: {width}, h: 320)]) }})"#
        )
    };
    let catalog = owner.join("assets/sprite_packs/full/ultrapack.json");
    std::fs::create_dir_all(catalog.parent().unwrap()).unwrap();
    let catalog_text = |page_size: u32| format!(r#"{{"page_size": {page_size}, "scale": 1.0, "pages": [], "targets": {{}}}}"#);
    app.init_resource::<PortraitSheetRegistry>();
    let widths = |app: &App| {
        let of = |registry: &PortraitSheetRegistry| registry.get(&portrait_key).map(|manifest| manifest.frame_width);
        (of(&ambition_sprite_sheet::portrait::baked_portrait_registry()), of(app.world().resource::<PortraitSheetRegistry>()))
    };
    let page_size = || catalog_for_tier("full").map(|catalog| catalog.page_size);
    std::thread::sleep(std::time::Duration::from_millis(20));
    std::fs::write(&portrait, portrait_text(256)).unwrap();
    std::fs::write(&catalog, catalog_text(64)).unwrap();
    look(&mut app);
    assert_eq!(app.world().resource::<SpriteSheetWatch>().refused, None);
    assert_eq!(widths(&app), (Some(256), Some(256)), "an added portrait is not in the generation or the resource");
    assert!(available_portrait_targets().contains(&PROBE), "the portrait vocabulary is not the generation's");
    assert_eq!(page_size(), Some(64), "an added catalog is not in the generation");

    std::thread::sleep(std::time::Duration::from_millis(20));
    std::fs::write(&portrait, portrait_text(128)).unwrap();
    look(&mut app);
    assert_eq!(widths(&app), (Some(128), Some(128)), "the portrait is not the saved file");
    // Alone: a change to another file reads the whole table again.
    std::thread::sleep(std::time::Duration::from_millis(20));
    std::fs::write(&catalog, catalog_text(128)).unwrap();
    look(&mut app);
    assert_eq!(page_size(), Some(128), "the catalog is not the saved file");

    std::thread::sleep(std::time::Duration::from_millis(20));
    std::fs::write(&portrait, "(target: ").unwrap();
    std::fs::write(&catalog, "{ \"page_size\": ").unwrap();
    look(&mut app);
    let refused = app.world().resource::<SpriteSheetWatch>().refused.clone().unwrap_or_default();
    assert!(refused.contains(&portrait_key) && refused.contains("sprite_packs/full/ultrapack.json"), "the refusal does not name both files: {refused}");
    assert_eq!((widths(&app), page_size()), ((Some(128), Some(128)), Some(128)), "a broken portrait or catalog changed the running ones");
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
    // A running game has decoded its flipbooks (the warm-up), so a
    // preparation takes each unchanged table from the current generation.
    for target in ambition_sprite_sheet::baked_part_flipbooks::baked_part_flipbook_targets() {
        let _ = ambition_sprite_sheet::character::rigged::RiggedSpriteAsset::baked(target);
    }
    let mut samples = Vec::new();
    for _ in 0..7 {
        let start = std::time::Instant::now();
        let table = ambition_sprite_sheet::baked_sheet_rons::read_sheet_table_under(owner);
        let table = ambition_sprite_sheet::baked_sheet_rons::prepare_sheet_table(table).expect("the published tree parses");
        samples.push(start.elapsed().as_secs_f64() * 1000.0);
        std::hint::black_box(table);
    }
    samples.sort_by(f64::total_cmp);
    println!("read and parse the table, flipbooks decoded: median {:.2} ms, range {:.2}-{:.2} ms (7 samples)", samples[3], samples[0], samples[6]);
}

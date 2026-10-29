//! Each App selects and replaces its own sprite metadata.

use ambition_sprite_sheet::{SpriteContent, baked_sheet_rons::{SheetTexts, current, publish_sheet_generation}};

fn texts(scale: f32, image: &str) -> SheetTexts {
    let ron = format!(r#"[(target: "body", image: "{image}", label_width: 0,
        frame_width: 32, frame_height: 32,
        tuning: Some((collision_scale: {scale}, frame_sample_inset: 0)),
        rows: [(animation: "idle", row_index: 0, frame_count: 1,
                duration_ms: 100, duration_secs: 0.1)])]"#);
    SheetTexts { sheets: vec![("body", ron.into())], ..Default::default() }
}

#[test]
fn two_apps_keep_independent_content_when_one_publishes_or_refuses_an_edit() {
    let mut first = bevy::app::App::new();
    let mut second = bevy::app::App::new();
    first.insert_resource(SpriteContent::from_texts(texts(1.0, "first.png")).unwrap());
    second.insert_resource(SpriteContent::from_texts(texts(2.0, "second.png")).unwrap());
    fn selected(app: &bevy::app::App) -> &ambition_sprite_sheet::baked_sheet_rons::SheetGeneration { current(app.world().resource::<SpriteContent>()) }
    let previous_content = first.world().resource::<SpriteContent>().clone();
    let previous = current(&previous_content);
    assert_eq!(previous.registry().get("body").unwrap().image, "first.png");
    assert_eq!(selected(&second).registry().get("body").unwrap().image, "second.png");
    let second_identity = selected(&second).mechanics_digest().to_owned();
    publish_sheet_generation(&mut first.world_mut().resource_mut::<SpriteContent>(), texts(3.0, "edited.png")).unwrap();
    assert_eq!(selected(&first).registry().get("body").unwrap().image, "edited.png");
    assert_eq!(previous.registry().get("body").unwrap().image, "first.png");
    assert_eq!(selected(&second).mechanics_digest(), second_identity);
    let accepted = selected(&first).id();
    let broken = SheetTexts { sheets: vec![("body", "[(target:".into())], ..Default::default() };
    assert!(publish_sheet_generation(&mut first.world_mut().resource_mut::<SpriteContent>(), broken).is_err());
    assert_eq!(selected(&first).id(), accepted);
    assert_eq!(selected(&first).registry().get("body").unwrap().image, "edited.png");
    assert_eq!(selected(&second).registry().get("body").unwrap().image, "second.png");
}

#[test]
fn an_empty_selection_reads_no_shared_art_and_packing_does_not_change_mechanics() {
    let empty = SpriteContent::default();
    assert!(current(&empty).registry().get("body").is_none());
    let first = SpriteContent::from_texts(texts(1.0, "first.png")).unwrap();
    let repacked = SpriteContent::from_texts(texts(1.0, "different_page.png")).unwrap();
    assert_eq!(current(&first).mechanics_digest(), current(&repacked).mechanics_digest());
}

#[test]
fn selected_libraries_keep_qualified_geometry_and_images_when_names_collide() {
    use ambition_asset_manager::content_library::{ContentLibraries, ContentLibrary, EmbeddedLibrary, EmbeddedLibraryFile};
    const BODY: &[u8] = br#"[(target: "body", image: "body.png", label_width: 0,
        frame_width: 32, frame_height: 32, rows: [(animation: "idle", row_index: 0,
        frame_count: 1, duration_ms: 100, duration_secs: 0.1)])]"#;
    static PACK: &[u8] = br#"{"page_size":64,"scale":1.0,"pages":["page.png"],"targets":{"body":{"idle":[{"index":0,"page":0,"x":0,"y":0,"w":4,"h":4,"off":[0,0],"src":[8,8],"duration_ms":100}]}}}"#;
    static FIRST: EmbeddedLibrary = EmbeddedLibrary {
        manifest: r#"{"schema":"ambition.content_library.v1","id":"first","version":"1","asset_roots":{"generated":"runtime"}}"#,
        catalog: r#"{"entries":{}}"#,
        files: &[
            EmbeddedLibraryFile { path: "sprites/body_spritesheet.ron", bytes: Some(BODY) },
            EmbeddedLibraryFile { path: "sprite_packs/full/ultrapack.json", bytes: Some(PACK) },
            EmbeddedLibraryFile { path: "sprite_packs/full/page.png", bytes: Some(b"page") },
            EmbeddedLibraryFile { path: "sprites/body.png", bytes: Some(b"first") },
        ],
    };
    static SECOND: EmbeddedLibrary = EmbeddedLibrary {
        manifest: r#"{"schema":"ambition.content_library.v1","id":"second","version":"1","asset_roots":{"generated":"runtime"}}"#,
        catalog: r#"{"entries":{}}"#,
        files: &[
            EmbeddedLibraryFile { path: "sprites/body_spritesheet.ron", bytes: Some(BODY) },
            EmbeddedLibraryFile { path: "sprite_packs/full/ultrapack.json", bytes: Some(PACK) },
            EmbeddedLibraryFile { path: "sprite_packs/full/page.png", bytes: Some(b"page") },
            EmbeddedLibraryFile { path: "sprites/body.png", bytes: Some(b"second") },
        ],
    };
    let mut libraries = ContentLibraries::default();
    libraries.insert(ContentLibrary::from_embedded(&FIRST).unwrap()).unwrap();
    let single = SpriteContent::from_texts(SheetTexts::from_libraries(&libraries).unwrap()).unwrap();
    assert_eq!(current(&single).registry().get("body").unwrap().image, "content://first/sprites/body.png");
    libraries.insert(ContentLibrary::from_embedded(&SECOND).unwrap()).unwrap();
    let both = SpriteContent::from_texts(SheetTexts::from_libraries(&libraries).unwrap()).unwrap();
    let registry = current(&both).registry();
    for library in ["first", "second"] {
        let target = format!("content://{library}/sprites/body");
        let (spec, tier) = ambition_sprite_sheet::character::sheets::try_load_pack_spec_for_target(
            &both, &target, &Default::default(), ambition_sprite_sheet::character::TextureResolutionScale::Full).unwrap();
        assert_eq!(tier, "full");
        assert_eq!(spec.page_images[0], format!("content://{library}/sprite_packs/full/page.png"));
    }
    assert!(registry.get("body").is_none());
    assert!(registry.get("sprites/body").is_none());
    assert_eq!(registry.get("content://first/sprites/body").unwrap().image, "content://first/sprites/body.png");
    assert_eq!(registry.get("content://second/sprites/body").unwrap().image, "content://second/sprites/body.png");
    assert!(current(&both).image_path("sprites/body.png").is_none());
    assert_eq!(libraries.resolve("content://second/sprites/body.png").unwrap().read().unwrap(), b"second");
    assert_eq!(current(&single).registry().get("body").unwrap().image, "content://first/sprites/body.png");
}

#[test]
fn quality_products_do_not_change_the_mechanical_identity() {
    let base = texts(1.0, "full.png");
    let first = SpriteContent::from_texts(base.clone()).unwrap();
    let mut tiers = base;
    tiers.sheets.push(("body.0_5x", texts(2.0, "half.png").sheets[0].1.clone()));
    let second = SpriteContent::from_texts(tiers).unwrap();
    assert_eq!(current(&first).mechanics_digest(), current(&second).mechanics_digest());
    assert_ne!(current(&first).mechanics_digest(), current(&SpriteContent::from_texts(texts(3.0, "full.png")).unwrap()).mechanics_digest());
}

#[test]
#[ignore = "requires the generated shared library"]
fn generated_shared_library_prepares_and_pins_its_images() {
    use ambition_asset_manager::content_library::{ContentLibraries, ContentLibrary};
    let mut libraries = ContentLibraries::default();
    libraries.insert(ContentLibrary::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../game/ambition_content_library/library.json")).unwrap()).unwrap();
    let content = SpriteContent::from_texts(SheetTexts::from_libraries(&libraries).unwrap()).unwrap();
    let generation = current(&content);
    let body = generation.registry().get("player_robot_v3").expect("the selected library's player sheet");
    assert_eq!(body.key, "content://ambition_shared/sprites/player_robot_v3");
    assert!(generation.image_path(&body.image).unwrap().starts_with("content://ambition_shared/__art/bundles/"));
    assert!(generation.registry().get("player_robot_v3.0_5x").unwrap().image.starts_with("content://ambition_shared/sprites_0_5x/"));
    assert!(ambition_sprite_sheet::baked_landmarks::body_landmarks(&content, "player_robot_v3").is_some());
    assert!(ambition_sprite_sheet::character::rigged::RiggedSpriteAsset::baked(&content, "player_robot_v3").is_some());
}

#[test]
fn preparation_reuses_unchanged_records_and_refuses_invalid_companions() {
    use ambition_sprite_sheet::baked_sheet_rons::prepare_sheet_table;
    use ambition_sprite_sheet::baked_part_flipbooks::BakedPartFlipbook;
    let content = SpriteContent::from_texts(texts(1.0, "body.png")).unwrap();
    let unchanged = prepare_sheet_table(&content, SheetTexts::current(&content)).unwrap();
    assert_eq!(unchanged.parsed_now(), 0);
    assert!(!unchanged.changes_mechanics_of(current(&content)));
    let changed = prepare_sheet_table(&content, texts(2.0, "body.png")).unwrap();
    assert_eq!(changed.parsed_now(), 1);
    assert!(changed.changes_mechanics_of(current(&content)));
    for invalid in [
        SheetTexts { body_rigs: vec![("body", "(broken".into())], ..SheetTexts::current(&content) },
        SheetTexts { portraits: vec![("sprites/body_portraits.ron", "(broken".into())], ..SheetTexts::current(&content) },
        SheetTexts { part_flipbooks: vec![("body", BakedPartFlipbook::Ron("(broken".into()), "body_parts.ron")], ..SheetTexts::current(&content) },
    ] {
        assert!(prepare_sheet_table(&content, invalid).is_err());
        assert_eq!(current(&content).registry().get("body").unwrap().image, "body.png");
    }
}

#[test]
#[cfg(target_os = "linux")]
#[ignore = "measure repeated native library preparation"]
fn measure_repeated_native_library_preparation() {
    use ambition_asset_manager::content_library::{ContentLibraries, ContentLibrary};
    let mut libraries = ContentLibraries::default();
    libraries.insert(ContentLibrary::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../game/ambition_content_library/library.json")).unwrap()).unwrap();
    let rss = || std::fs::read_to_string("/proc/self/status").unwrap().lines()
        .find(|line| line.starts_with("VmRSS:")).unwrap().to_owned();
    eprintln!("before: {}", rss());
    for run in 1..=3 {
        let start = std::time::Instant::now();
        let content = SpriteContent::from_texts(SheetTexts::from_libraries(&libraries).unwrap()).unwrap();
        assert!(current(&content).registry().get("player_robot_v3").is_some());
        drop(content);
        eprintln!("run {run}: {}, {:.3} s", rss(), start.elapsed().as_secs_f64());
    }
}

#[test]
fn rig_geometry_reaches_identity_and_rig_quality_and_formatting_do_not() {
    fn rig(x: f32) -> std::sync::Arc<str> {
        format!(r#"(schema_version: 1, target: "body", joints: [(name: "hip", parent: None)],
            attachments: [(name: "hand_near", joint: "hip", offset: ({x:?}, 0.0))], hurt_parts: [],
            clips: {{"idle": (looping: true, frame_duration_s: 0.1,
                frames: [[(translation: (0.0, 0.0), rotation: 0.0, scale: (1.0, 1.0))]])}})"#).into()
    }
    let mut base = texts(1.0, "body.png");
    base.body_rigs.push(("body", rig(1.0)));
    let content = SpriteContent::from_texts(base.clone()).unwrap();
    let mut changed = base.clone();
    changed.body_rigs[0].1 = rig(2.0);
    assert_ne!(current(&content).mechanics_digest(), current(&SpriteContent::from_texts(changed).unwrap()).mechanics_digest());
    let mut presentation = base;
    presentation.body_rigs[0].1 = format!("// A source comment.\n{}", rig(1.0)).into();
    presentation.body_rigs.push(("body.0_5x", rig(3.0)));
    let prepared = ambition_sprite_sheet::baked_sheet_rons::prepare_sheet_table(&content, presentation.clone()).unwrap();
    assert!(!prepared.changes_mechanics_of(current(&content)));
    assert_eq!(current(&content).mechanics_digest(), current(&SpriteContent::from_texts(presentation).unwrap()).mechanics_digest());
}

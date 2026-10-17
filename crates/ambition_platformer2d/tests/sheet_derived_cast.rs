//! A cast reads its body scale and its body rig from its sheets when its
//! definitions are made (`posed_body_world_per_pixel`, `published_body_rig`).
//! A sheet publication states the cast again, so the next session's bodies
//! are not built from the new records with a scale or a rig of the old ones.
//!
//! Its own test binary: a publication changes what every reader of the
//! process reads.

#![cfg(feature = "content_pack")]

use ambition_platformer2d::characters::actor::definition::BodySource;
use ambition_platformer2d::characters::prepared::{close_preparation_barrier_admitting, PreparedCharacterRegistry};
use ambition_platformer2d::content::sprite_reload::{SpriteSheetReloadPlugin, SpriteSheetWatch};
use ambition_platformer2d::content::EmbeddedPack;
use ambition_sprite_sheet::baked_sheet_rons::{current, publish_sheet_generation, read_sheet_table_under};
use bevy::prelude::*;

const PROBE: &str = "sheet_derived_cast_probe";

/// A sheet whose standing body is `height` pixels tall.
fn sheet_text(height: i32) -> String {
    format!(
        r#"[(
    target: "{PROBE}",
    image: "{PROBE}_spritesheet.png",
    label_width: 0,
    frame_width: 64,
    frame_height: 64,
    rows: [],
    start: 0.0,
    end: 1.0,
    body_metrics: Some((body_pixel_bbox: Some((x: 16, y: 0, w: 32, h: {height})), animations: {{}}, authored_body: true)),
)]"#
    )
}

/// A rig of one joint whose near hand is `x` pixels from it.
fn rig_text(x: f32) -> String {
    format!(
        r#"(
    schema_version: 1,
    target: "{PROBE}",
    joints: [(name: "hip", parent: None)],
    attachments: [(name: "hand_near", joint: "hip", offset: ({x:?}, 0.0))],
    hurt_parts: [],
    clips: {{
        "idle": (looping: true, frame_duration_s: 0.1, frames: [[(translation: (0.0, 0.0), rotation: 0.0, scale: (1.0, 1.0))]]),
    }},
)"#
    )
}

/// Where the near hand of the cast's rig is, in world units.
fn hand(app: &App) -> Option<f32> {
    let rig = app.world().resource::<PreparedCharacterRegistry>().get("newcomer")?.body_rig.clone()?;
    rig.attachments().first().map(|attachment| attachment.offset.x)
}

fn world_per_pixel(app: &App) -> Option<f32> {
    match app.world().resource::<PreparedCharacterRegistry>().get("newcomer")?.body {
        Some(BodySource::SpriteAuthored { world_per_pixel }) => Some(world_per_pixel),
        _ => None,
    }
}

#[test]
fn a_sheet_edit_states_again_the_body_scale_of_the_cast_made_from_it() {
    let base = std::env::temp_dir().join(format!("sheet_derived_cast_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let (owner, root) = (base.join("tree"), base.join("pack"));
    let sprites = owner.join("assets/sprites");
    std::fs::create_dir_all(&sprites).unwrap();
    std::fs::create_dir_all(root.join("data")).unwrap();
    let manifest = sprites.join(format!("{PROBE}_spritesheet.ron"));
    std::fs::write(&manifest, sheet_text(32)).unwrap();
    let rig = sprites.join(format!("{PROBE}_body_rig.ron"));
    std::fs::write(&rig, rig_text(3.0)).unwrap();
    std::fs::write(
        root.join("pack.ron"),
        r#"(id: "sheet_derived_cast_probe", version: "0.1.0", namespace: "probe", requires: [], sources: [
            (path: "data/cast.ron", schema: "character_catalog", version: 1),
        ])"#,
    )
    .unwrap();
    std::fs::write(
        root.join("data/cast.ron"),
        format!(
            r#"(
                brain_presets: {{ "stand_still": StandStill }},
                action_set_presets: {{ "peaceful": (move_style: Walk) }},
                characters: {{
                    "newcomer": (
                        display_name: "Newcomer", manifest: "{PROBE}_spritesheet.ron", tier: MainHall,
                        body_kind: Standard, composition: None, default_brain: "stand_still",
                        default_action_set: "peaceful", tags: [],
                        standing_height: Some(64.0), posed_body: Some(OwnHeight),
                    ),
                }},
            )"#
        ),
    )
    .unwrap();
    // The process starts on the probe tree, as a game starts on its own.
    publish_sheet_generation(read_sheet_table_under(&owner)).unwrap_or_else(|refused| panic!("{refused}"));
    let leaked_root: &'static str = Box::leak(root.to_string_lossy().into_owned().into_boxed_str());
    let pack: &'static EmbeddedPack = Box::leak(Box::new(EmbeddedPack::rooted(leaked_root, None)));

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<ambition_platformer2d::actor::ActiveSessionScope>();
    app.init_resource::<ambition_platformer2d::combat::technique::InstalledTechniques>();
    app.init_resource::<ambition_sprite_sheet::SheetRegistry>();
    app.insert_resource(ambition_platformer2d::characters::actor::BodyRigAdmission::ADMIT);
    app.add_plugins(SpriteSheetReloadPlugin { owner: Some(owner.clone()), manual: true });
    pack.cast("probe", None).register(&mut app);
    close_preparation_barrier_admitting(app.world_mut(), &Default::default());
    app.world().resource::<SpriteSheetWatch>().look_now();
    app.update();
    assert_eq!(world_per_pixel(&app), Some(2.0), "the premise: 64 world units over a 32 pixel body");
    assert_eq!(hand(&app), Some(6.0), "the premise: the hand is 3 pixels out at 2.0 world units per pixel");

    std::thread::sleep(std::time::Duration::from_millis(20));
    std::fs::write(&manifest, sheet_text(16)).unwrap();
    app.world().resource::<SpriteSheetWatch>().look_now();
    app.update();
    let published = current().id();
    assert_eq!(app.world().resource::<SpriteSheetWatch>().published, 1, "the edit was not published");
    let authority = ambition_platformer2d::character_sprites::posed_body_world_per_pixel(
        &ambition_platformer2d::characters::actor::character_catalog::lowered_catalog(pack.prepared()).unwrap(),
        "newcomer",
    );
    assert_eq!(authority, Some(4.0), "the premise: the published sheet makes the body 16 pixels tall");
    assert_eq!(world_per_pixel(&app), authority, "the cast keeps a body scale made from sheet generation {}", published - 1);
    assert_eq!(hand(&app), Some(12.0), "the cast's rig keeps the old scale");

    // An edit to the rig alone is a sheet publication too.
    std::thread::sleep(std::time::Duration::from_millis(20));
    std::fs::write(&rig, rig_text(5.0)).unwrap();
    app.world().resource::<SpriteSheetWatch>().look_now();
    app.update();
    assert_eq!(app.world().resource::<SpriteSheetWatch>().published, 2, "the rig edit was not published");
    assert_eq!(
        ambition_sprite_sheet::baked_body_rigs::baked_body_rig(PROBE),
        Some(rig_text(5.0).as_str()),
        "the published generation is not the saved rig"
    );
    assert_eq!(hand(&app), Some(20.0), "the cast keeps the rig of the last generation");

    // A rig that does not prepare is refused, and the cast keeps its rig.
    std::thread::sleep(std::time::Duration::from_millis(20));
    std::fs::write(&rig, rig_text(5.0).replace(r#"joint: "hip""#, r#"joint: "elbow""#)).unwrap();
    app.world().resource::<SpriteSheetWatch>().look_now();
    app.update();
    let watch = app.world().resource::<SpriteSheetWatch>();
    assert!(
        watch.refused.as_deref().is_some_and(|why| why.contains(&format!("{PROBE}_body_rig.ron"))),
        "the refusal does not name the rig: {:?}",
        watch.refused
    );
    assert_eq!((watch.published, hand(&app)), (2, Some(20.0)), "a broken rig reached the cast");
    let _ = std::fs::remove_dir_all(&base);
}

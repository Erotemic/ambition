//! A cast reads its body scale from its sheets when its definitions are made
//! (`posed_body_world_per_pixel`). A sheet publication states the cast again,
//! so the next session's bodies are not built from the new records with a
//! scale made from the old ones.
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
    app.add_plugins(SpriteSheetReloadPlugin { owner: Some(owner.clone()), manual: true });
    pack.cast("probe", None).register(&mut app);
    close_preparation_barrier_admitting(app.world_mut(), &Default::default());
    app.world().resource::<SpriteSheetWatch>().look_now();
    app.update();
    assert_eq!(world_per_pixel(&app), Some(2.0), "the premise: 64 world units over a 32 pixel body");

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
    let _ = std::fs::remove_dir_all(&base);
}

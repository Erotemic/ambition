//! A native library admits files and metadata at the same session boundary.

#![cfg(feature = "content_pack")]
mod common;
use common::*;
use ambition_platformer2d::content::sprite_reload::{SpriteSheetReloadPlugin, SpriteSheetWatch};
use ambition_asset_manager::library_source::LibraryAssets;
use ambition_sprite_sheet::baked_sheet_rons::current;
use bevy::prelude::*;

const KEY: &str = "content://probe/sprites/body";
const IMAGE: &str = "content://probe/sprites/body.png";

fn sheet(scale: f32, label: u32) -> String {
    format!(r#"[(target: "body", image: "body.png", label_width: {label}, frame_width: 32, frame_height: 32,
        tuning: Some((collision_scale: {scale}, frame_sample_inset: 0)),
        rows: [(animation: "idle", row_index: 0, frame_count: 1, duration_ms: 100, duration_secs: 0.1,
            rects: [(x: 0, y: 0, w: 32, h: 32)])]) ]"#)
}

fn save(owner: &std::path::Path, scale: f32, label: u32, image: &[u8]) {
    std::fs::write(owner.join("assets/sprites/body_spritesheet.ron"), sheet(scale, label)).unwrap();
    std::fs::write(owner.join("assets/sprites/body.png"), image).unwrap();
    publish_fixture(owner);
}

fn look(app: &mut App) {
    app.world().resource::<SpriteSheetWatch>().look_now();
    app.update();
}

fn bytes(app: &App) -> Vec<u8> {
    bevy::tasks::block_on(async {
        use bevy::asset::io::Reader;
        let mut source = ambition_asset_manager::library_source::library_asset_source("assets", app.world().resource::<LibraryAssets>().clone(), true);
        let reader = (source.reader)();
        let mut file = reader.read(std::path::Path::new(IMAGE.strip_prefix("content://").unwrap())).await.unwrap();
        let mut bytes = Vec::new();
        Reader::read_to_end(&mut *file, &mut bytes).await.unwrap();
        bytes
    })
}

#[test]
fn native_admission_keeps_files_metadata_and_apps_together() {
    let owner = std::env::temp_dir().join(format!("native_library_reload_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&owner);
    std::fs::create_dir_all(owner.join("assets/sprites")).unwrap();
    library(&owner, true);
    save(&owner, 1.0, 0, b"first");
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    install(&mut app, &owner);
    app.init_resource::<ambition_platformer2d::actor::ActiveSessionScope>();
    app.add_plugins(SpriteSheetReloadPlugin { manual: true });
    app.update();
    look(&mut app);
    let mut other = App::new();
    install(&mut other, &owner);
    let accepted_content = content(&app).clone();
    let accepted = current(&accepted_content);
    let first = accepted.id();
    let old_image = accepted.image_path(IMAGE).unwrap().to_owned();
    assert!(old_image.contains("/__art/bundles/"));
    assert_eq!(bytes(&app), b"first");

    app.world_mut().resource_mut::<ambition_platformer2d::actor::ActiveSessionScope>().begin();
    save(&owner, 2.0, 0, b"second");
    look(&mut app);
    assert!(app.world().resource::<SpriteSheetWatch>().is_staged());
    assert_eq!(current(content(&app)).id(), first);
    assert_eq!(bytes(&app), b"first");
    app.world_mut().resource_mut::<ambition_platformer2d::actor::ActiveSessionScope>().clear();
    app.update();
    let next_content = content(&app).clone();
    let next = current(&next_content);
    assert_eq!(next.id(), first + 1);
    assert_eq!(next.registry().get(KEY).unwrap().tuning.as_ref().unwrap().collision_scale, 2.0);
    assert_eq!(bytes(&app), b"second");
    assert_eq!(current(content(&other)).id(), first);
    assert_eq!(bytes(&other), b"first");
    assert_eq!(accepted.image_path(IMAGE), Some(old_image.as_str()));

    std::fs::write(owner.join("assets/sprites/body_body_rig.ron"), "(broken").unwrap();
    save(&owner, 3.0, 0, b"refused");
    look(&mut app);
    assert!(app.world().resource::<SpriteSheetWatch>().refused.is_some());
    assert_eq!(current(content(&app)).id(), next.id());
    assert_eq!(bytes(&app), b"second");

    std::fs::remove_file(owner.join("assets/sprites/body_body_rig.ron")).unwrap();
    app.world_mut().resource_mut::<ambition_platformer2d::actor::ActiveSessionScope>().begin();
    save(&owner, 2.0, 4, b"repacked");
    look(&mut app);
    assert_eq!(current(content(&app)).id(), next.id() + 1);
    assert_eq!(current(content(&app)).registry().get(KEY).unwrap().label_width, 4);
    assert_eq!(bytes(&app), b"repacked");
    assert!(!app.world().resource::<SpriteSheetWatch>().is_staged());
    std::fs::remove_dir_all(owner).unwrap();
}

#[derive(Asset, TypePath)]
struct ImportedImage(Vec<u8>);

#[derive(Default, TypePath)]
struct ImportedImageLoader;

impl bevy::asset::AssetLoader for ImportedImageLoader {
    type Asset = ImportedImage;
    type Settings = ();
    type Error = std::io::Error;

    async fn load(&self, reader: &mut dyn bevy::asset::io::Reader, _: &(), _: &mut bevy::asset::LoadContext<'_>) -> Result<ImportedImage, Self::Error> {
        let mut bytes = Vec::new();
        bevy::asset::io::Reader::read_to_end(reader, &mut bytes).await?;
        Ok(ImportedImage(bytes))
    }

    fn extensions(&self) -> &[&str] { &["png", "bin"] }
}

#[derive(Default, TypePath)]
struct PixelLoader;

impl bevy::asset::AssetLoader for PixelLoader {
    type Asset = Image;
    type Settings = bevy::image::ImageLoaderSettings;
    type Error = std::io::Error;

    async fn load(&self, reader: &mut dyn bevy::asset::io::Reader, settings: &Self::Settings, _: &mut bevy::asset::LoadContext<'_>) -> Result<Image, Self::Error> {
        let mut bytes = Vec::new();
        bevy::asset::io::Reader::read_to_end(reader, &mut bytes).await?;
        let mut image = Image::default();
        image.data = Some(vec![bytes[0], 0, 0, 255]);
        image.asset_usage = settings.asset_usage;
        Ok(image)
    }

    fn extensions(&self) -> &[&str] { &["png"] }
}

fn wait_for_pixel(app: &mut App, handle: &Handle<Image>, expected: u8) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        app.update();
        if app.world().resource::<Assets<Image>>().get(handle).and_then(|image| image.data.as_ref()).is_some_and(|bytes| bytes[0] == expected) { return; }
        assert!(std::time::Instant::now() < deadline, "the live image did not reach its selected bytes");
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

fn wait_for_imported_image(app: &mut App, handle: &Handle<ImportedImage>, expected: &[u8]) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        app.update();
        if app.world().resource::<Assets<ImportedImage>>().get(handle).is_some_and(|image| image.0 == expected) { return; }
        assert!(std::time::Instant::now() < deadline, "the imported image did not reach the requested bytes");
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

#[test]
fn game_relative_media_use_and_reload_their_selected_library() {
    use ambition_platformer2d::content::libraries::{register_game_source, register_sources};
    use bevy::asset::AssetApp;
    let owner = std::env::temp_dir().join(format!("game_library_reload_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&owner);
    std::fs::create_dir_all(owner.join("assets/sprites")).unwrap();
    std::fs::create_dir_all(owner.join("game")).unwrap();
    library(&owner, true);
    std::fs::create_dir_all(owner.join("assets/audio")).unwrap();
    std::fs::write(owner.join("assets/audio/probe.bin"), b"first bank").unwrap();
    std::fs::create_dir_all(owner.join("assets/backgrounds")).unwrap();
    std::fs::write(owner.join("assets/backgrounds/sky.png"), b"dark").unwrap();
    save(&owner, 1.0, 0, b"first");
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    register_sources(&mut app, "assets");
    register_game_source(&mut app, "game", owner.join("game").to_str().unwrap());
    app.add_plugins(bevy::asset::AssetPlugin::default());
    app.init_asset::<ImportedImage>().init_asset_loader::<ImportedImageLoader>();
    app.init_asset::<Image>().init_asset_loader::<PixelLoader>();
    install(&mut app, &owner);
    app.add_plugins(SpriteSheetReloadPlugin { manual: true });
    app.finish();
    let image = app.world().resource::<AssetServer>().load("game://sprites/body.png");
    let bank = app.world().resource::<AssetServer>().load("game://audio/probe.bin");
    let held = content(&app).clone();
    let frozen = ambition_sprite_sheet::game_assets::load_sheet_image(Some(&held), app.world().resource::<AssetServer>(), "sheet", IMAGE);
    let sky = ambition_sprite_sheet::game_assets::load_sheet_image(None, app.world().resource::<AssetServer>(), "parallax", "content://probe/backgrounds/sky.png");
    wait_for_pixel(&mut app, &frozen, b'f');
    wait_for_pixel(&mut app, &sky, b'd');
    wait_for_imported_image(&mut app, &image, b"first");
    wait_for_imported_image(&mut app, &bank, b"first bank");
    look(&mut app);
    std::fs::write(owner.join("assets/audio/probe.bin"), b"second bank").unwrap();
    std::fs::write(owner.join("assets/backgrounds/sky.png"), b"light").unwrap();
    save(&owner, 1.0, 0, b"second");
    look(&mut app);
    wait_for_imported_image(&mut app, &image, b"second");
    wait_for_imported_image(&mut app, &bank, b"second bank");
    wait_for_pixel(&mut app, &sky, b'l');
    wait_for_pixel(&mut app, &frozen, b'f');
    std::fs::remove_dir_all(owner).unwrap();
}

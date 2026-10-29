use ambition_asset_manager::content_library::{ContentLibraries, ContentLibrary, LibrarySnapshot};
use ambition_asset_manager::library_source::{library_asset_source, LibraryAssets};
use ambition_sprite_sheet::{baked_sheet_rons::SheetTexts, SpriteContent};
use bevy::{asset::AssetApp, prelude::App};

pub fn read() -> (LibrarySnapshot, SpriteContent) {
    let path = std::env::var("AMBITION_CONTENT_LIBRARY")
        .expect("Set AMBITION_CONTENT_LIBRARY to a library.json path");
    let mut libraries = ContentLibraries::default();
    libraries.insert(ContentLibrary::read(path).expect("read the selected library")).unwrap();
    let snapshot = libraries.snapshot().expect("read the selected products");
    let content = SpriteContent::from_texts(SheetTexts::from_snapshot(&snapshot, None).unwrap())
        .expect("admit the selected sprite metadata");
    (snapshot, content)
}

pub fn install(app: &mut App, snapshot: &LibrarySnapshot, content: &SpriteContent) {
    let assets = LibraryAssets::default();
    assets.replace(snapshot.clone());
    app.register_asset_source("content", library_asset_source("assets", assets.clone(), true));
    app.register_asset_source(bevy::asset::io::AssetSourceId::Default,
        library_asset_source("assets", assets.clone(), false));
    app.insert_resource(assets).insert_resource(content.clone());
}

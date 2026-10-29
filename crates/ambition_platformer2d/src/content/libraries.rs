//! Install the content libraries a game pack selects.

use std::path::Path;
use bevy::prelude::*;
use ambition_asset_manager::{content_library::{ContentLibraries, ContentLibrary, EmbeddedLibrary}, library_source::LibraryAssets};
use ambition_sprite_sheet::{SpriteContent, baked_sheet_rons::{SheetTexts, prepare_sheet_table, publish_prepared_sheet_table}};

pub fn install(world: &mut World, root: &Path, paths: &[String], embedded: Option<&'static [EmbeddedLibrary]>) -> Result<(), String> {
    let mut libraries = world.get_resource::<ContentLibraries>().cloned().unwrap_or_default();
    let before = libraries.iter().count();
    if let Some(embedded) = embedded {
        for library in embedded { libraries.insert(ContentLibrary::from_embedded(library)?)?; }
    } else {
        for path in paths { libraries.insert(ContentLibrary::read(root.join(path))?)?; }
    }
    if before == libraries.iter().count() && world.contains_resource::<ContentLibraries>() { return Ok(()); }
    let mut content = world.get_resource::<SpriteContent>().cloned().unwrap_or_default();
    let snapshot = libraries.snapshot()?;
    let prepared = prepare_sheet_table(&content, SheetTexts::from_snapshot(&snapshot, Some(&content))?)
        .map_err(|error| format!("{error:?}"))?;
    let assets = world.get_resource::<LibraryAssets>().cloned().unwrap_or_default();
    assets.replace(snapshot);
    publish_prepared_sheet_table(&mut content, prepared);
    world.insert_resource(assets);
    world.insert_resource(libraries);
    world.insert_resource(content);
    Ok(())
}

/// Register the selected content before Bevy creates its asset readers.
pub fn register_sources(app: &mut App, game_root: &str) {
    use bevy::asset::{AssetApp, io::AssetSourceId};
    use ambition_asset_manager::library_source::library_asset_source;
    app.init_resource::<LibraryAssets>();
    let assets = app.world().resource::<LibraryAssets>().clone();
    app.register_asset_source(AssetSourceId::Default, library_asset_source(game_root, assets.clone(), false));
    app.register_asset_source("content", library_asset_source("assets", assets, true));
}

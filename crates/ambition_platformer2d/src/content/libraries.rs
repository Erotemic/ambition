//! Host library selections and provider imports.

use ambition_asset_manager::{
    content_library::{ContentLibraries, ContentLibrary, EmbeddedLibrary, LibraryManifest},
    library_source::LibraryAssets,
};
use ambition_sprite_sheet::{
    baked_sheet_rons::{prepare_sheet_table, publish_prepared_sheet_table, SheetTexts},
    SpriteContent,
};
use bevy::prelude::*;
use std::path::Path;

/// Select libraries for the host. A conflicting selection is refused.
pub fn install(
    world: &mut World,
    root: &Path,
    paths: &[String],
    embedded: Option<&'static [EmbeddedLibrary]>,
) -> Result<(), String> {
    register(world, root, paths, embedded, Registration::Select)
}

/// Import library identities. Existing host selections remain authoritative.
/// An import supplies its default only when the library has no selection.
pub fn import(
    world: &mut World,
    root: &Path,
    paths: &[String],
    embedded: Option<&'static [EmbeddedLibrary]>,
) -> Result<(), String> {
    register(world, root, paths, embedded, Registration::Import)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Registration {
    Select,
    Import,
}

fn register(
    world: &mut World,
    root: &Path,
    paths: &[String],
    embedded: Option<&'static [EmbeddedLibrary]>,
    registration: Registration,
) -> Result<(), String> {
    let empty = ContentLibraries::default();
    let existing = world.get_resource::<ContentLibraries>();
    let had_selection = existing.is_some();
    let mut libraries = std::borrow::Cow::Borrowed(existing.unwrap_or(&empty));
    let before = libraries.iter().count();
    if let Some(embedded) = embedded {
        for library in embedded {
            if registration == Registration::Import {
                let declaration = LibraryManifest::from_json(library.manifest)?;
                if libraries.get(&declaration.id).is_some() {
                    continue;
                }
            }
            libraries
                .to_mut()
                .insert(ContentLibrary::from_embedded(library)?)?;
        }
    } else {
        for path in paths {
            let path = root.join(path);
            if registration == Registration::Import {
                let declaration = LibraryManifest::read(&path)?;
                if libraries.get(&declaration.id).is_some() {
                    continue;
                }
            }
            libraries.to_mut().insert(ContentLibrary::read(path)?)?;
        }
    }
    if before == libraries.iter().count() && had_selection {
        return Ok(());
    }
    let libraries = libraries.into_owned();
    let mut content = world
        .get_resource::<SpriteContent>()
        .cloned()
        .unwrap_or_default();
    let snapshot = libraries.snapshot()?;
    let prepared = prepare_sheet_table(
        &content,
        SheetTexts::from_snapshot(&snapshot, Some(&content))?,
    )
    .map_err(|error| format!("{error:?}"))?;
    let assets = world
        .get_resource::<LibraryAssets>()
        .cloned()
        .unwrap_or_default();
    assets.replace(snapshot);
    publish_prepared_sheet_table(&mut content, prepared);
    world.insert_resource(assets);
    world.insert_resource(libraries);
    world.insert_resource(content);
    Ok(())
}

/// Register the selected content before Bevy creates its asset readers.
pub fn register_sources(app: &mut App, game_root: &str) {
    use ambition_asset_manager::library_source::library_asset_source;
    use bevy::asset::{io::AssetSourceId, AssetApp};
    app.init_resource::<LibraryAssets>();
    let assets = app.world().resource::<LibraryAssets>().clone();
    app.register_asset_source(
        AssetSourceId::Default,
        library_asset_source(game_root, assets.clone(), false),
    );
    app.register_asset_source("content", library_asset_source("assets", assets, true));
}

pub use ambition_asset_manager::library_source::register_game_source;

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_asset_manager::content_library::{EmbeddedLibraryFile, LibraryFileData};

    const DECLARATION: &str = r#"{
        "schema":"ambition.content_library.v1", "id":"probe", "version":"1",
        "asset_roots":{"generated":"runtime"}
    }"#;
    static SERVED: &[EmbeddedLibrary] = &[EmbeddedLibrary {
        manifest: DECLARATION,
        catalog: r#"{"entries":{}}"#,
        files: &[EmbeddedLibraryFile {
            path: "blob.dat",
            bytes: None,
        }],
    }];

    struct Native(std::path::PathBuf);
    impl Native {
        fn new() -> Self {
            let root =
                std::env::temp_dir().join(format!("host_library_import_{}", std::process::id()));
            std::fs::create_dir_all(root.join("runtime")).unwrap();
            std::fs::write(root.join("library.json"), DECLARATION).unwrap();
            std::fs::write(root.join("runtime/blob.dat"), b"host bytes").unwrap();
            Self(root)
        }
    }
    impl Drop for Native {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn a_provider_import_keeps_the_host_selection_but_an_explicit_conflict_refuses() {
        let native = Native::new();
        let mut app = App::new();
        install(app.world_mut(), &native.0, &["library.json".into()], None).unwrap();
        import(app.world_mut(), Path::new(""), &[], Some(SERVED)).unwrap();
        let mut declaration: serde_json::Value = serde_json::from_str(DECLARATION).unwrap();
        declaration["catalog"] = "unused-catalog.json".into();
        std::fs::write(native.0.join("provider.json"), declaration.to_string()).unwrap();
        import(app.world_mut(), &native.0, &["provider.json".into()], None).unwrap();
        let libraries = app.world().resource::<ContentLibraries>();
        assert_eq!(libraries.iter().count(), 1);
        assert!(!libraries.get("probe").unwrap().is_embedded());
        let file = libraries.resolve("content://probe/blob.dat").unwrap();
        assert_eq!(
            std::fs::read(file.native_path().unwrap()).unwrap(),
            b"host bytes"
        );

        let refusal = install(app.world_mut(), Path::new(""), &[], Some(SERVED)).unwrap_err();
        assert!(refusal.contains("two selections"), "{refusal}");
        let libraries = app.world().resource::<ContentLibraries>();
        assert!(!libraries.get("probe").unwrap().is_embedded());
        let file = libraries.resolve("content://probe/blob.dat").unwrap();
        assert_eq!(
            std::fs::read(file.native_path().unwrap()).unwrap(),
            b"host bytes"
        );
    }

    #[test]
    fn a_standalone_served_provider_supplies_its_library_default() {
        let mut app = App::new();
        import(app.world_mut(), Path::new(""), &[], Some(SERVED)).unwrap();
        let libraries = app.world().resource::<ContentLibraries>();
        assert_eq!(libraries.iter().count(), 1);
        assert!(libraries.get("probe").unwrap().is_embedded());
        let file = libraries.resolve("content://probe/blob.dat").unwrap();
        assert!(matches!(file.data, LibraryFileData::Embedded(None)));
    }

    #[test]
    fn an_import_rejects_an_unsafe_declaration_before_using_an_existing_selection() {
        static UNSAFE: &[EmbeddedLibrary] = &[EmbeddedLibrary {
            manifest: r#"{"schema":"ambition.content_library.v1","id":"probe","version":"1","asset_roots":{"generated":"../outside"}}"#,
            catalog: r#"{"entries":{}}"#,
            files: &[],
        }];
        let mut app = App::new();
        import(app.world_mut(), Path::new(""), &[], Some(SERVED)).unwrap();
        let refusal = import(app.world_mut(), Path::new(""), &[], Some(UNSAFE)).unwrap_err();
        assert!(refusal.contains("Invalid content library"), "{refusal}");
        assert!(app
            .world()
            .resource::<ContentLibraries>()
            .get("probe")
            .unwrap()
            .is_embedded());
    }
}

//! Host library selections and provider imports.

use ambition_asset_manager::{
    content_library::{ContentLibraries, ContentLibrary, EmbeddedLibrary, LibrarySnapshot},
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

/// The sheet table this process last prepared from library files. A boot
/// that holds no table yet prepares against it, so only what changed since is
/// parsed again: `prepare_sheet_table` keeps a sheet or rig whose text is the
/// same and a flipbook whose file time is the same. A test process boots
/// hundreds of Apps from one library set, and a full parse costs seconds.
fn last_prepared() -> std::sync::MutexGuard<'static, Option<SpriteContent>> {
    static LAST: std::sync::Mutex<Option<SpriteContent>> = std::sync::Mutex::new(None);
    LAST.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Registration {
    Select,
    Import,
}

/// Capture the accepted files for the host's selected library choices.
pub fn snapshot(world: &World) -> Result<LibrarySnapshot, String> {
    world.get_resource::<ContentLibraries>().map(ContentLibraries::snapshot)
        .unwrap_or_else(|| Ok(LibrarySnapshot::default()))
}

/// Library files and sheet metadata prepared before publication.
pub struct PreparedLibraries {
    snapshot: LibrarySnapshot,
    content: Option<SpriteContent>,
}

/// Check choices and prepare metadata without changing the live world.
pub fn prepare(world: &World, incoming: &LibrarySnapshot) -> Result<PreparedLibraries, String> {
    let mut selected = match world.get_resource::<LibraryAssets>() {
        Some(assets) => assets.snapshot(),
        None => snapshot(world)?,
    };
    if selected.libraries.iter().next().is_none() { selected = snapshot(world)?; }
    if let Some(libraries) = world.get_resource::<ContentLibraries>() {
        let mut choices = libraries.clone();
        for library in incoming.libraries.iter() { choices.insert(library.clone())?; }
    }
    selected.merge(incoming)?;
    let adds_library = selected.libraries.iter().any(|library| {
        world.get_resource::<ContentLibraries>().and_then(|libraries| libraries.get(&library.manifest.id)).is_none()
    });
    // Existing sheet generations stay under the sprite reload owner.
    if !adds_library && world.contains_resource::<SpriteContent>() {
        return Ok(PreparedLibraries { snapshot: selected, content: None });
    }
    let held = world.get_resource::<SpriteContent>().cloned();
    let previous = held.clone().or_else(|| last_prepared().clone());
    let mut content = held.unwrap_or_default();
    let previous = previous.as_ref().unwrap_or(&content);
    let prepared = prepare_sheet_table(previous, SheetTexts::from_snapshot(&selected, Some(previous))?)
        .map_err(|error| format!("{error:?}"))?;
    publish_prepared_sheet_table(&mut content, prepared);
    Ok(PreparedLibraries { snapshot: selected, content: Some(content) })
}

/// Publish the files that preparation checked. This step reads no source files.
pub fn publish(world: &mut World, prepared: PreparedLibraries) {
    let PreparedLibraries { snapshot, content } = prepared;
    let assets = world.get_resource::<LibraryAssets>().cloned().unwrap_or_default();
    world.insert_resource(snapshot.libraries.clone());
    assets.replace(snapshot);
    world.insert_resource(assets);
    if let Some(content) = content {
        *last_prepared() = Some(content.clone());
        world.insert_resource(content);
    }
}

fn register(
    world: &mut World,
    root: &Path,
    paths: &[String],
    embedded: Option<&'static [EmbeddedLibrary]>,
    registration: Registration,
) -> Result<(), String> {
    let had_selection = world.contains_resource::<ContentLibraries>();
    let mut selected = match world.get_resource::<LibraryAssets>() {
        Some(assets) => assets.snapshot(),
        None => snapshot(world)?,
    };
    if selected.libraries.iter().next().is_none() { selected = snapshot(world)?; }
    let before = selected.libraries.iter().count();
    if let Some(embedded) = embedded {
        for library in embedded {
            if registration == Registration::Import {
                selected.import_embedded(library)?;
            } else {
                selected.insert(ContentLibrary::from_embedded(library)?)?;
            }
        }
    } else {
        for path in paths {
            let path = root.join(path);
            if registration == Registration::Import {
                selected.import_native(&path)?;
            } else {
                selected.insert(ContentLibrary::read(path)?)?;
            }
        }
    }
    if before == selected.libraries.iter().count() && had_selection {
        return Ok(());
    }
    let prepared = prepare(world, &selected)?;
    publish(world, prepared);
    Ok(())
}

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

    /// ASSET-SELECTION's old-publication clause, at the host's launch road:
    /// a store an older tool published (its recorded generation is not the
    /// current tool's, and its digest is not checked) is selected by
    /// `install` as it stands, and the product it names is the file served.
    /// Nothing is rebuilt and nothing is audited.
    #[test]
    fn a_publication_an_older_tool_recorded_starts_without_a_rebuild() {
        let root = std::env::temp_dir().join(format!("host_library_old_publication_{}", std::process::id()));
        let bundle = "k".repeat(64);
        std::fs::create_dir_all(root.join(format!("store/bundles/{bundle}/audio"))).unwrap();
        std::fs::write(
            root.join("library.json"),
            r#"{"schema":"ambition.content_library.v1","id":"probe","version":"1",
                "asset_roots":{"generated":"made"},"publication":"store"}"#,
        )
        .unwrap();
        std::fs::write(root.join(format!("store/bundles/{bundle}/audio/cue.ogg")), b"published bytes").unwrap();
        std::fs::write(
            root.join("store/published.json"),
            serde_json::json!({
                "schema": "ambition.art_publication.v1",
                "generation": "recorded-by-an-older-tool",
                "roots": [],
                "bundles": {"target:cue": bundle},
                "files": {"audio/cue.ogg": {
                    "path": format!("bundles/{bundle}/audio/cue.ogg"),
                    "role": "runtime", "tier": "full",
                    "sha256": "0".repeat(64),
                }},
            })
            .to_string(),
        )
        .unwrap();

        let mut app = App::new();
        let installed = install(app.world_mut(), &root, &["library.json".into()], None);
        let served = app
            .world()
            .get_resource::<ContentLibraries>()
            .map(|libraries| libraries.resolve("content://probe/audio/cue.ogg"));
        let read = served.as_ref().and_then(|file| file.as_ref().ok()).map(|file| (file.generation.clone(), file.read()));
        let _ = std::fs::remove_dir_all(&root);
        installed.expect("the old publication was refused at launch");
        served.expect("libraries were installed").expect("the old publication's product resolves");
        let (generation, bytes) = read.expect("a product to read");
        assert_eq!(generation.as_deref(), Some("recorded-by-an-older-tool"));
        assert_eq!(bytes.unwrap(), b"published bytes", "the product an older tool published is the file served");
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
    #[test]
    fn publication_keeps_the_files_prepared_before_the_store_changed() {
        let root = std::env::temp_dir().join(format!("prepared_library_publication_{}", std::process::id()));
        std::fs::create_dir_all(root.join("store/bundles/old")).unwrap();
        std::fs::create_dir_all(root.join("store/bundles/new")).unwrap();
        std::fs::write(root.join("library.json"), r#"{
            "schema":"ambition.content_library.v1","id":"probe","version":"1",
            "asset_roots":{"generated":"made"},"publication":"store"
        }"#).unwrap();
        std::fs::write(root.join("store/bundles/old/blob.dat"), b"old").unwrap();
        std::fs::write(root.join("store/bundles/new/blob.dat"), b"new").unwrap();
        let select = |generation: &str| {
            std::fs::write(root.join("store/published.json"), serde_json::json!({
                "schema":"ambition.art_publication.v1", "generation":generation,
                "files":{"blob.dat":{
                    "path":format!("bundles/{generation}/blob.dat"),
                    "role":"runtime","tier":"full","sha256":"0".repeat(64)
                }}
            }).to_string()).unwrap();
        };
        select("old");
        let mut app = App::new();
        install(app.world_mut(), &root, &["library.json".into()], None).unwrap();
        select("new");
        let selected = snapshot(app.world()).unwrap();
        let prepared = prepare(app.world(), &selected).unwrap();
        let live = app.world().resource::<LibraryAssets>().snapshot();
        assert_eq!(live.files["content://probe/blob.dat"].read().unwrap(), b"old");
        select("old");
        publish(app.world_mut(), prepared);
        let served = app.world().resource::<LibraryAssets>().snapshot();
        let file = &served.files["content://probe/blob.dat"];
        assert_eq!(file.generation.as_deref(), Some("new"));
        assert_eq!(file.read().unwrap(), b"new");
        assert_eq!(snapshot(app.world()).unwrap().files["content://probe/blob.dat"].read().unwrap(), b"old");
        std::fs::remove_dir_all(root).unwrap();
    }

}

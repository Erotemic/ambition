//! Read an installed library without the checkout that built its index.
#![cfg(feature = "bevy")]

use ambition_asset_manager::{
    AssetEntry, AssetId, AssetKind, AssetManifest, AssetProfile,
    bevy_integration::AmbitionAssetCatalog,
    content_library::{ContentLibraries, ContentLibrary, EmbeddedLibrary, EmbeddedLibraryFile},
    library_source::{LibraryAssets, library_asset_source},
    platformer_assets::Platformer2dAssetCatalog,
};
use futures_lite::io::AsyncReadExt;

static LIBRARY: EmbeddedLibrary = EmbeddedLibrary {
    manifest: r#"{"schema":"ambition.content_library.v1","id":"fixture","version":"1","asset_roots":{"generated":"runtime"}}"#,
    catalog: r#"{"entries":{}}"#,
    files: &[
        EmbeddedLibraryFile { path: "sprites/body.png", bytes: None },
        EmbeddedLibraryFile { path: "sprites_0_5x/body.png", bytes: None },
        EmbeddedLibraryFile { path: "sprites_0_25x/body.png", bytes: None },
        EmbeddedLibraryFile { path: "sprites/absent.png", bytes: None },
        EmbeddedLibraryFile { path: "sprites/embedded.png", bytes: Some(b"embedded pixels") },
    ],
};

#[test]
fn installed_library_images_load_without_the_source_checkout() {
    if std::env::var_os("AMBITION_TEST_INSTALLED_LIBRARY_CHILD").is_some() {
        check_installed_library();
        return;
    }
    let install = tempfile::tempdir().unwrap();
    let elsewhere = tempfile::tempdir().unwrap();
    for (path, bytes) in [("sprites/body.png", b"full pixels".as_slice()),
                          ("sprites_0_5x/body.png", b"half pixels".as_slice())] {
        let file = install.path().join("assets/__libraries/fixture").join(path);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, bytes).unwrap();
    }
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "installed_library_images_load_without_the_source_checkout", "--nocapture"])
        .env("AMBITION_TEST_INSTALLED_LIBRARY_CHILD", "1")
        .env("BEVY_ASSET_ROOT", install.path())
        .current_dir(elsewhere.path())
        .output().unwrap();
    assert!(output.status.success(), "{}\n{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
}

fn check_installed_library() {
    let mut libraries = ContentLibraries::default();
    libraries.insert(ContentLibrary::from_embedded(&LIBRARY).unwrap()).unwrap();
    let snapshot = libraries.snapshot().unwrap();
    let full = "content://fixture/sprites/body.png";
    let half = "content://fixture/sprites_0_5x/body.png";
    for profile in [AssetProfile::DesktopDevLoose, AssetProfile::DesktopInstalled, AssetProfile::SteamDeckInstalled] {
        let mut manifest = AssetManifest::new();
        for (id, path) in [("body", full), ("body.0_5x", half),
                           ("body.0_25x", "content://fixture/sprites_0_25x/body.png")] {
            manifest.insert(AssetEntry::new(id, AssetKind::Image, path));
        }
        let catalog = Platformer2dAssetCatalog::new(AmbitionAssetCatalog::new(manifest), profile)
            .with_snapshot(&snapshot);
        assert_eq!(catalog.try_image_path_for_load(full).as_deref(), Some(full), "{}", profile.label());
        assert_eq!(catalog.try_quality_path_for_load(&AssetId::new("body"), Some("0_5x"), true).as_deref(), Some(half));
        assert_eq!(catalog.try_quality_path_for_load(&AssetId::new("body"), Some("0_25x"), true).as_deref(), Some(full));
        assert_eq!(std::fs::read(catalog.resolve_local_file_path(full).unwrap()).unwrap(), b"full pixels");
        assert!(catalog.try_image_path_for_load("content://fixture/sprites/absent.png").is_none());
        assert!(catalog.try_image_path_for_load("content://other/sprites/body.png").is_none());
        assert!(catalog.try_image_path_for_load("content://fixture/sprites/embedded.png").is_some());
    }
    let state = LibraryAssets::default();
    state.replace(snapshot);
    let mut source = library_asset_source("assets", state, true);
    let reader = (source.reader)();
    futures_lite::future::block_on(async {
        let mut file = reader.read(std::path::Path::new("fixture/sprites/body.png")).await.unwrap();
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).await.unwrap();
        assert_eq!(bytes, b"full pixels");
    });
}

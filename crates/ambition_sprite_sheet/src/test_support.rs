//! An explicit shared-library fixture for corpus tests.

pub fn shared_sprites() -> crate::SpriteContent {
    static CONTENT: std::sync::OnceLock<crate::SpriteContent> = std::sync::OnceLock::new();
    CONTENT.get_or_init(|| {
        use ambition_asset_manager::content_library::{ContentLibraries, ContentLibrary};
        let mut libraries = ContentLibraries::default();
        libraries.insert(ContentLibrary::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../game/ambition_content_library/library.json")).expect("shared library fixture")).unwrap();
        crate::SpriteContent::from_texts(crate::baked_sheet_rons::SheetTexts::from_libraries(&libraries).expect("Generate the shared art before corpus tests")).expect("shared metadata fixture")
    }).clone()
}

/// Select the shared library for a corpus test.
pub fn shared_libraries() -> ambition_asset_manager::content_library::ContentLibraries {
    use ambition_asset_manager::content_library::{ContentLibraries, ContentLibrary};
    let mut libraries = ContentLibraries::default();
    libraries.insert(ContentLibrary::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../game/ambition_content_library/library.json")).expect("shared library fixture")).unwrap();
    libraries
}

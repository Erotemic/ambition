//! A pack's embedded texts, for a crate's `build.rs`.
//!
//! A build that has no source tree (web, Android, a bundled build) embeds its
//! pack with `include_str!`. The list of what to embed is the pack's own
//! manifest: [`embed_pack`] reads `pack.ron` and writes one `include_str!` for
//! it and for each source it declares. A hand list in Rust beside the manifest
//! was a second statement of the pack, and a Rust build for each new file even
//! in a build that reads every file off disk.
//!
//! The generated file is one expression of type
//! `(&'static str, &'static [(&'static str, &'static str)])`: the manifest and
//! each `(declared path, text)`. The caller includes it only under its own
//! `static_content` feature.

use std::path::Path;

/// Where [`embed_pack`] writes the texts of the pack at `root`, relative to
/// `OUT_DIR`. The caller includes `concat!(env!("OUT_DIR"), "/", <this>)`.
pub fn embedded_file(root: &str) -> String {
    format!("content_pack/{root}/embedded.rs")
}

/// Call from `build.rs`. `root` is relative to the crate's directory and holds
/// `pack.ron`.
///
/// Without the crate's `static_content` feature it writes nothing and watches
/// only `build.rs`: the game reads the pack off disk when it compiles it, so
/// an edit to a source or to the manifest is no Rust build. With the feature it
/// writes the embedded texts and watches `pack.ron` (each `include_str!`
/// watches its own source).
///
/// # Panics
///
/// When the manifest is not readable or does not parse: a build that embeds a
/// pack it cannot read would ship no content.
pub fn embed_pack(root: &str) {
    println!("cargo:rerun-if-changed=build.rs");
    if std::env::var_os("CARGO_FEATURE_STATIC_CONTENT").is_none() {
        return;
    }
    let crate_dir = std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR");
    let manifest_path = Path::new(&crate_dir).join(root).join("pack.ron");
    println!("cargo:rerun-if-changed={}", manifest_path.display());
    let text = std::fs::read_to_string(&manifest_path)
        .unwrap_or_else(|error| panic!("{} is not readable: {error}", manifest_path.display()));
    let manifest: crate::ContentPackManifest = ron::from_str(&text)
        .unwrap_or_else(|error| panic!("{} does not parse: {error}", manifest_path.display()));
    embed_libraries(root, &manifest, manifest_path.parent().expect("pack root"));
    let include = |path: &str| format!("include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), {:?}))", format!("/{root}/{path}"));
    let mut code = format!("({}, &[\n", include("pack.ron"));
    for source in &manifest.sources {
        code.push_str(&format!("    ({:?}, {}),\n", source.path, {
            let path = source.resolve_path(manifest_path.parent().unwrap(), &manifest).unwrap_or_else(|error| panic!("{error}"));
            format!("include_str!({:?})", path)
        }));
    }
    code.push_str("] as &[(&str, &str)])\n");
    let out = Path::new(&std::env::var("OUT_DIR").expect("cargo sets OUT_DIR")).join(embedded_file(root));
    std::fs::create_dir_all(out.parent().expect("the file has a directory"))
        .unwrap_or_else(|error| panic!("{} is not writable: {error}", out.display()));
    std::fs::write(&out, code).unwrap_or_else(|error| panic!("{} is not writable: {error}", out.display()));
}

/// The source paths `manifest_ron` declares, in its order; none when it does
/// not parse (the compiler says why).
pub fn declared_paths(manifest_ron: &str) -> Vec<String> {
    ron::from_str::<crate::ContentPackManifest>(manifest_ron)
        .map(|manifest| manifest.sources.into_iter().map(|source| source.path).collect())
        .unwrap_or_default()
}

fn embed_libraries(root: &str, manifest: &crate::ContentPackManifest, pack_root: &Path) {
    use ambition_asset_manager::content_library::{ContentLibrary, LibraryFileData};
    let mut code = String::from("&[\n");
    for path in &manifest.libraries {
        let path = pack_root.join(path).canonicalize().expect("library manifest path");
        let library = ContentLibrary::read(&path).unwrap_or_else(|error| panic!("{error}"));
        println!("cargo:rerun-if-changed={}", path.display());
        if let Some(store) = library.publication_root() { println!("cargo:rerun-if-changed={}", store.join("published.json").display()); }
        let catalog = serde_json::to_string(&library.catalog).expect("library catalog");
        code.push_str(&format!("ambition_platformer2d::asset_manager::content_library::EmbeddedLibrary {{ manifest: include_str!({:?}), catalog: {:?}, files: &[\n", path.to_str().unwrap(), catalog));
        if let Some(catalog) = &library.manifest.catalog { println!("cargo:rerun-if-changed={}", library.root.join(catalog).display()); }
        for (logical, file) in library.files().unwrap_or_else(|error| panic!("{error}")) {
            let LibraryFileData::Native(physical) = file.data else { unreachable!() };
            let metadata = logical.ends_with(".ron") || logical.ends_with(".json");
            let bytes = if metadata || library.manifest.embedded.contains(&logical) {
                format!("Some(include_bytes!({:?}) as &[u8])", physical.to_str().unwrap())
            } else { "None".to_owned() };
            code.push_str(&format!("ambition_platformer2d::asset_manager::content_library::EmbeddedLibraryFile {{ path: {logical:?}, bytes: {bytes} }},\n"));
        }
        code.push_str("] },\n");
    }
    code.push_str("]\n");
    let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join(format!("content_pack/{root}/libraries.rs"));
    std::fs::create_dir_all(out.parent().unwrap()).expect("embedded library output directory");
    std::fs::write(out, code).expect("embedded library output");
}

/// The physical sources selected by a pack manifest.
pub fn source_paths(root: &Path) -> Result<Vec<std::path::PathBuf>, String> {
    let text = std::fs::read_to_string(root.join("pack.ron")).map_err(|error| error.to_string())?;
    let manifest: crate::ContentPackManifest = ron::from_str(&text).map_err(|error| error.to_string())?;
    manifest.sources.iter().map(|source| source.resolve_path(root, &manifest)).collect()
}

//! Embed Ambition's pack in a build that has no source tree.
//!
//! Under `static_content` (web, Android), this writes `embedded_pack.rs`: the
//! manifest `assets/pack.ron` and each source it declares, as `include_str!`.
//! The list is the manifest's, so a source line in `pack.ron` is the only edit
//! that adds a source. A hand list in `pack.rs` beside it was a second
//! statement of the pack, and a Rust build for each new file even in a build
//! that reads every file off disk.
//!
//! Without the feature, this writes `None` and does not watch `pack.ron`: a
//! development build reads the manifest and its sources when the pack
//! compiles, so an edit to either is no Rust build.

use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let out = Path::new(&std::env::var("OUT_DIR").expect("cargo sets OUT_DIR")).join("embedded_pack.rs");
    if std::env::var_os("CARGO_FEATURE_STATIC_CONTENT").is_none() {
        write(&out, "pub(crate) const EMBEDDED_PACK: Option<EmbeddedPack> = None;\n");
        return;
    }
    let manifest_path = Path::new(&std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR"))
        .join("assets")
        .join("pack.ron");
    println!("cargo:rerun-if-changed={}", manifest_path.display());
    let text = std::fs::read_to_string(&manifest_path)
        .unwrap_or_else(|error| panic!("{} is not readable: {error}", manifest_path.display()));
    let manifest: ambition_content_pack::ContentPackManifest = ron::from_str(&text)
        .unwrap_or_else(|error| panic!("{} does not parse: {error}", manifest_path.display()));
    let mut code = String::from(
        "pub(crate) const EMBEDDED_PACK: Option<EmbeddedPack> = Some(EmbeddedPack {\n    \
         manifest: include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/assets/pack.ron\")),\n    \
         sources: &[\n",
    );
    for source in &manifest.sources {
        let path = format!("{:?}", source.path);
        code.push_str(&format!(
            "        ({path}, include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/assets/\", {path}))),\n"
        ));
    }
    code.push_str("    ],\n});\n");
    write(&out, &code);
}

fn write(path: &Path, text: &str) {
    std::fs::write(path, text).unwrap_or_else(|error| panic!("{} is not writable: {error}", path.display()));
}

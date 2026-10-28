//! Embed the selected core products in static builds.
use std::path::PathBuf;

#[allow(dead_code)]
#[path = "src/art_publication.rs"]
mod art_publication;

fn main() {
    let table = "src/platformer_assets/embedded_core.json";
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={table}");
    println!("cargo:rerun-if-changed=src/art_publication.rs");
    println!("cargo:rerun-if-env-changed=AMBITION_ART_STORE");
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!("src/platformer_assets/embedded_core.json")).unwrap();
    let embed = std::env::var_os("CARGO_FEATURE_STATIC_CORE_ASSETS").is_some();
    let assets = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("../ambition_platformer2d_actor_monolith/assets");
    let store = art_publication::store_for_assets(&assets);
    let selected = embed.then(|| art_publication::Selection::read(&store).unwrap_or_else(|error| panic!("{error}")));
    if embed { println!("cargo:rerun-if-changed={}", store.join("published.json").display()); }
    let mut code = String::from("pub mod embedded_core {\n");
    for row in &rows {
        code.push_str(&format!("pub const {}: &str = {:?};\n", row["name"].as_str().unwrap(), row["url"].as_str().unwrap()));
    }
    code.push_str("pub const ALL_URLS: &[&str] = &[\n");
    for row in &rows { code.push_str(&format!("{},\n", row["name"].as_str().unwrap())); }
    code.push_str("];\n}\n");
    if let Some(selected) = selected {
        code.push_str("fn register_embedded_core_assets(app: &mut bevy::prelude::App) {\nlet embedded = app.world_mut().resource_mut::<bevy::asset::io::embedded::EmbeddedAssetRegistry>();\n");
        for row in &rows {
            let logical = row["path"].as_str().unwrap();
            let path = if art_publication::managed(std::path::Path::new(logical)) {
                selected.path(logical).unwrap_or_else(|| panic!("Unpublished embedded product: {logical}"))
            } else { assets.join(logical) };
            println!("cargo:rerun-if-changed={}", path.display());
            code.push_str(&format!("embedded.insert_asset(std::path::PathBuf::new(), std::path::Path::new(embedded_core::{}), include_bytes!({:?}) as &[u8]);\n",
                row["name"].as_str().unwrap(), path.to_string_lossy()));
        }
        code.push_str("}\n");
    }
    std::fs::write(PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("embedded_core.rs"), code).unwrap();
}

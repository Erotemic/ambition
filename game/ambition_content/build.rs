//! Embed Ambition's pack in a build that has no source tree: the list is
//! `assets/pack.ron` itself (`ambition_content_pack::embed`). Under
//! `static_map`, embed each world file the pack's `data/worlds.ron` lists:
//! that file is the one list of the worlds.

use std::path::Path;

#[allow(dead_code)]
#[path = "../../crates/ambition_asset_manager/src/art_publication.rs"]
mod art_publication;

fn main() {
    println!("cargo:rerun-if-env-changed=AMBITION_ART_STORE");
    if std::env::var_os("CARGO_FEATURE_STATIC_CONTENT").is_some() {
        let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../crates/ambition_platformer2d_actor_monolith/assets");
        let store = art_publication::store_for_assets(&assets);
        let selected = art_publication::Selection::read(&store).unwrap_or_else(|error| panic!("{error}"));
        let path = selected.path("vanity_card_made_this_meme/manifest.ron").expect("Publish the director card before embedding content");
        println!("cargo:rerun-if-changed={}", store.join("published.json").display());
        println!("cargo:rustc-env=AMBITION_VANITY_CARD_RON={}", path.display());
    }
    ambition_content_pack::embed::embed_pack("assets");
    embed_worlds();
}

/// Only the file names: the pack's own compile checks the rest.
#[derive(serde::Deserialize)]
struct Worlds {
    worlds: Vec<World>,
}

#[derive(serde::Deserialize)]
struct World {
    file: String,
}

fn embed_worlds() {
    if std::env::var_os("CARGO_FEATURE_STATIC_MAP").is_none() {
        return;
    }
    let crate_dir = std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR");
    let list = Path::new(&crate_dir).join("assets/data/worlds.ron");
    println!("cargo:rerun-if-changed={}", list.display());
    let text = std::fs::read_to_string(&list).unwrap_or_else(|error| panic!("{} is not readable: {error}", list.display()));
    let worlds: Worlds = ron::from_str(&text)
        .unwrap_or_else(|error| panic!("{} does not parse: {error}", list.display()));
    let mut code = String::from("&[\n");
    for world in &worlds.worlds {
        code.push_str(&format!(
            "    ({:?}, include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), {:?}))),\n",
            world.file,
            format!("/assets/worlds/{}", world.file)
        ));
    }
    code.push_str("]\n");
    let out = Path::new(&std::env::var("OUT_DIR").expect("cargo sets OUT_DIR")).join("worlds_embedded.rs");
    std::fs::write(&out, code).unwrap_or_else(|error| panic!("{} is not writable: {error}", out.display()));
}

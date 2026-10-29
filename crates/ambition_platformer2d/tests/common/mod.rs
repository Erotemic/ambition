#![allow(dead_code)]

use bevy::prelude::*;

pub fn library(owner: &std::path::Path, published: bool) {
    std::fs::create_dir_all(owner.join("assets")).unwrap();
    let mut manifest = serde_json::json!({
        "schema":"ambition.content_library.v1", "id":"probe", "version":"1", "asset_roots":{"generated":"assets"}
    });
    if published { manifest["publication"] = "art_publication".into(); }
    std::fs::write(owner.join("library.json"), manifest.to_string()).unwrap();
}

pub fn install(app: &mut App, owner: &std::path::Path) {
    ambition_platformer2d::content::libraries::install(app.world_mut(), owner, &["library.json".into()], None).unwrap();
}

pub fn content(app: &App) -> &ambition_sprite_sheet::SpriteContent {
    app.world().resource::<ambition_sprite_sheet::SpriteContent>()
}

pub fn publish_fixture(owner: &std::path::Path) {
    use std::hash::{Hash, Hasher};
    let assets = owner.join("assets");
    let mut pending = vec![assets.clone()];
    let mut files = std::collections::BTreeMap::new();
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() { pending.push(path); }
            else {
                files.insert(path.strip_prefix(&assets).unwrap().to_string_lossy().replace('\\', "/"), std::fs::read(path).unwrap());
            }
        }
    }
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    files.hash(&mut hash);
    let key = format!("{:016x}", hash.finish());
    let store = owner.join("art_publication");
    let mut selected = serde_json::Map::new();
    for (logical, bytes) in files {
        let physical = format!("bundles/{key}/runtime/full/{logical}");
        let dest = store.join(&physical);
        std::fs::create_dir_all(dest.parent().unwrap()).unwrap();
        if !dest.exists() { std::fs::write(dest, bytes).unwrap(); }
        selected.insert(logical, serde_json::json!({"path":physical,"sha256":"fixture","role":"runtime","tier":"full"}));
    }
    std::fs::write(store.join("next.json"), serde_json::json!({
        "schema":ambition_asset_manager::art_publication::SCHEMA,"generation":key,"files":selected
    }).to_string()).unwrap();
    std::fs::rename(store.join("next.json"),store.join("published.json")).unwrap();
}

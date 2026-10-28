//! Export prepared identities, dependencies and loose sheet resolution for authoring tools.

use ambition_content_pack::PreparedContentPack;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// Describe this compiled pack. Missing local pixels remain inspectable.
pub fn describe(pack: &PreparedContentPack, roots: &[PathBuf]) -> Value {
    let catalog = ambition_characters::actor::character_catalog::content_schema::lowered_catalog(pack);
    let mut requested = BTreeSet::from(["idle".to_string(), "hit".to_string(), "death".to_string(), "death_idle".to_string()]);
    let movesets = ambition_characters::moveset_content_schema::lowered_movesets(pack);
    let mut bindings = BTreeMap::<String, BTreeMap<String, Vec<String>>>::new();
    let characters: Vec<_> = catalog.into_iter().flat_map(|catalog| &catalog.characters).map(|(id, row)| {
        let mut required: Vec<String> = ambition_sprite_sheet::inspection::requirements(row).into_iter().map(String::from).collect();
        required.extend(row.rest_row.iter().cloned());
        required.sort(); required.dedup();
        requested.extend(required.iter().map(|name| name.to_string()));
        let moves: Vec<_> = movesets.and_then(|table| table.get(id)).into_iter().flat_map(|table| &table.moves).map(|action| {
            let key = format!("{id}/{}", action.id);
            let chain: Vec<_> = std::iter::once(action.clip.clip.clone()).chain(action.clip.fallbacks.clone()).collect();
            if let Some(target) = row.manifest_target() {
                bindings.entry(target.to_string()).or_default().insert(key.clone(), chain.clone());
            }
            json!({"key": key, "move": action.id, "chain": chain})
        }).collect();
        json!({"id": id, "provider": pack.namespace.to_string(), "catalog": row,
               "sheet_target": row.manifest_target(), "moves": moves,
               "required_actions": required, "capabilities_complete": row.abilities.is_some()})
    }).collect();
    let actions: Vec<_> = requested.into_iter().collect();
    let mut sheets = serde_json::Map::new();
    let mut failures = Vec::new();
    fn visit(path: &Path, actions: &[String], bindings: &BTreeMap<String, BTreeMap<String, Vec<String>>>, sheets: &mut serde_json::Map<String, Value>, failures: &mut Vec<Value>) {
        let Ok(entries) = std::fs::read_dir(path) else { return; };
        let mut entries: Vec<_> = entries.filter_map(Result::ok).map(|entry| entry.path()).collect();
        entries.sort();
        for path in entries {
            if path.is_dir() { visit(&path, actions, bindings, sheets, failures); }
            else if path.file_name().and_then(|name| name.to_str()).is_some_and(|name| name.ends_with("_spritesheet.ron")) {
                match ambition_sprite_sheet::inspection::sheet(&path, actions, bindings) {
                    Ok(rows) => { sheets.insert(path.to_string_lossy().into_owned(), rows); }
                    Err(error) => failures.push(json!({"path": path, "error": error})),
                }
            } else if path.file_name().and_then(|name| name.to_str()).is_some_and(|name|
                name.ends_with("_portraits.ron") || name.ends_with("_parts.ron") || name.ends_with("_body_rig.ron")) {
                if let Err(error) = ambition_sprite_sheet::inspection::companion(&path) {
                    failures.push(json!({"path":path,"error":error}));
                }
            }
        }
    }
    for root in roots { visit(root, &actions, &bindings, &mut sheets, &mut failures); }
    let mut semantics = BTreeMap::new();
    for records in sheets.values().filter_map(Value::as_array) {
        for record in records {
            if let Some(actions) = record["actions"].as_object() {
                for name in actions.keys() {
                    if let Some(key) = ambition_sprite_sheet::inspection::semantic_key(name) { semantics.insert(name.clone(), key); }
                }
            }
        }
    }
    let worlds = ambition_content_pack::lowered_rules::<ambition_platformer2d_world::world_manifest::AuthoredWorldManifest>(
        pack, ambition_platformer2d_world::world_manifest::WORLD_MANIFEST_SCHEMA)
        .map(|manifest| manifest.worlds.iter().map(|world| json!({"id": world.id, "file": world.file, "required": world.required})).collect::<Vec<_>>());
    json!({"semantics": semantics, "tool_version": env!("CARGO_PKG_VERSION"), "worlds": worlds, "schema": "ambition.art_content.v3", "provider": pack.namespace.to_string(), "pack": pack.id.to_string(),
        "fingerprint": pack.fingerprint.hex(), "characters": characters, "sheets": sheets, "failures": failures,
        "sources": pack.sources.iter().map(|source| json!({"path": source.declared_path, "schema": source.schema.to_string(),
            "fingerprint": source.content_fingerprint})).collect::<Vec<_>>(),
        "assets": pack.assets.values().map(|asset| json!({"path": asset.path,
            "required_by": asset.required_by.iter().map(ToString::to_string).collect::<Vec<_>>()})).collect::<Vec<_>>(),
        "references": pack.resolved_references.iter().map(|reference| json!({"from": reference.declared_by.to_string(),
            "field": reference.field, "to": reference.target.to_string()})).collect::<Vec<_>>()})
}

//! Read published sheets through the runtime parser and pose resolver.

use crate::character::{CharacterAnim, CharacterSheetSpec, SheetTuning};
#[cfg(test)]
use crate::SheetRecord;
use serde_json::{json, Value};
use std::path::Path;

/// The product key that selected this loaded sheet.
pub fn sheet_key(spec: &CharacterSheetSpec) -> &str {
    &spec.record.key
}

/// The content identity of the manifest selected by a presentation.
pub fn fingerprint(spec: &CharacterSheetSpec) -> String {
    let mut record = spec.record.clone();
    record.key.clear();
    blake3::hash(format!("{record:?}").as_bytes()).to_hex().to_string()
}

/// The semantic pose of an accepted row name, including aliases.
pub fn semantic_key(name: &str) -> Option<String> {
    CharacterAnim::from_name(name).map(|pose| format!("{pose:?}"))
}

/// Minimum poses implied by the catalog's declared body capabilities.
pub fn requirements(entry: &ambition_characters::actor::character_catalog::CharacterCatalogEntry) -> Vec<&'static str> {
    let mut actions = vec!["idle", "hit"];
    if entry.part_of.is_none() && !entry.death_traits.as_ref().is_some_and(|traits| traits.never_dies) {
        actions.push("death");
    }
    if let Some(grants) = &entry.abilities {
        let abilities = ambition_platformer2d_core::AbilitySet::compose(grants);
        for (enabled, names) in [
            (abilities.move_horizontal, &["walk"][..]),
            (abilities.jump, &["jump", "fall"][..]),
            (abilities.fly, &["hover"][..]),
            (abilities.dash, &["dash"][..]),
            (abilities.blink, &["blink_out", "blink_in"][..]),
            (abilities.attack, &["slash"][..]),
            (abilities.wall_cling, &["wall_grab"][..]),
        ] {
            if enabled { actions.extend_from_slice(names); }
        }
    }
    actions.sort(); actions.dedup(); actions
}

/// The requested action, its fallback steps and the actual published row.
pub fn action(spec: &CharacterSheetSpec, requested: &str) -> Value {
    let Some(anim) = CharacterAnim::from_name(requested) else {
        let row = spec.row_names().position(|name| name == requested);
        return json!({"requested": requested, "state": if row.is_some() {"direct"} else {"missing"},
                      "row": row.and_then(|slot| spec.row_name(slot)), "steps": []});
    };
    let (resolved, steps) = spec.explain_anim(anim);
    let row = spec.row_for_anim(resolved).and_then(|slot| spec.row_name(slot));
    let state = match row {
        None => "missing",
        Some(_) if resolved != anim => "fallback",
        Some(name) if name == requested => "direct",
        Some(_) => "alias",
    };
    json!({"requested": requested, "state": state, "row": row,
           "steps": steps.iter().map(|step| format!("{step:?}")).collect::<Vec<_>>()})
}

/// Inspect loose metadata. No registry cache or embedded art can replace the file.
pub fn sheet(path: &Path, actions: &[String], bindings: &std::collections::BTreeMap<String, std::collections::BTreeMap<String, Vec<String>>>) -> Result<Value, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let product = path.file_name().and_then(|name| name.to_str()).and_then(|name| name.strip_suffix("_spritesheet.ron"))
        .ok_or_else(|| "A sheet manifest must end in _spritesheet.ron".to_string())?;
    let key = crate::sprite_tree::sheet_key(product, path);
    let index = crate::index_baked_table(&[(&key, &text)]);
    if !index.failed.is_empty() { return Err(format!("{:?}", index.failed)); }
    let records: std::collections::BTreeMap<_, _> = index.sheets.into_iter().collect();
    let mut sheets = Vec::new();
    for (_, record) in records {
        let spec = crate::character::sheets::spec_from_record(&record, &SheetTuning::default());
        let pages: Vec<_> = record.used_pages().iter().map(|page| {
            let name = if record.images.is_empty() { record.image.clone() }
                else { record.images.get(*page as usize).cloned().unwrap_or_default() };
            json!({"page": page, "path": name, "exists": path.parent().unwrap_or(Path::new(".")).join(&name).is_file()})
        }).collect();
        let mut names = actions.to_vec();
        names.extend(record.rows.iter().map(|row| row.animation.clone()));
        names.sort(); names.dedup();
        sheets.push(json!({"key": record.key, "product": product, "target": record.target, "fingerprint": fingerprint(&spec), "pages": pages,
            "rows": record.rows.iter().map(|row| json!({"name": row.animation, "frames": row.frame_count,
                "ms": row.duration_ms, "valid": row.frame_count > 0 && (row.rects.is_empty() || row.rects.len() == row.frame_count as usize)})).collect::<Vec<_>>(),
            "actions": names.iter().map(|name| (name.clone(), action(&spec, name))).collect::<serde_json::Map<_, _>>(),
            "bindings": bindings.get(product).into_iter().flat_map(|table| table.iter()).map(|(id, chain)| {
                let row = spec.clip_slot(chain.iter().map(String::as_str)).and_then(|slot| spec.row_name(slot));
                (id.clone(), json!({"requested": chain.first(), "row": row, "steps": chain,
                    "state": match row { None => "missing", Some(name) if chain.first().is_some_and(|first| first == name) => "direct", Some(_) => "fallback" }}))
            }).collect::<serde_json::Map<_, _>>(),
            "corpse": if spec.row_names().any(|name| name == "death_idle") {"death_idle"} else {"hold_death_final"}}));
    }
    Ok(json!(sheets))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_death_explains_its_idle_fallback_and_a_published_death_is_direct() {
        let make = |names: &[&str]| {
            let text = format!("[(target: \"fixture\", image: \"fixture.png\", frame_width: 8, frame_height: 8, label_width: 0, rows: [{}])]",
                names.iter().enumerate().map(|(i, name)| format!("(animation: \"{name}\", row_index: {i}, frame_count: 1, duration_ms: 100, duration_secs: 0.1)")).collect::<Vec<_>>().join(","));
            let records: Vec<SheetRecord> = ron::from_str(&text).expect("a sheet");
            crate::character::sheets::spec_from_record(&records[0], &SheetTuning::default())
        };
        let missing = action(&make(&["idle", "walk"]), "death");
        assert_eq!(missing["state"], "fallback");
        assert_eq!(missing["steps"], json!(["Death", "Hit", "Idle"]));
        let mut indexed = make(&["idle", "hurt", "death"]);
        let identity = fingerprint(&indexed);
        indexed.record.key = "fixture.half".to_string();
        assert_eq!(fingerprint(&indexed), identity);
        let drawn = action(&indexed, "death");
        assert_eq!(drawn["state"], "direct");
        assert_eq!(drawn["row"], "death");
        assert_eq!(action(&make(&["idle", "hurt"]), "hit")["state"], "alias");
    }
    #[test]
    fn a_shared_generator_target_keeps_its_product_key_and_move_binding() {
        let directory = std::env::temp_dir().join(format!("ambition-art-{}-{}", std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir(&directory).unwrap();
        let path = directory.join("person_spritesheet.ron");
        std::fs::write(&path, r#"[(target: "toon", image: "person.png", frame_width: 8, frame_height: 8, label_width: 0,
            rows: [(animation: "idle", row_index: 0, frame_count: 1, duration_ms: 100, duration_secs: 0.1)])]"#).unwrap();
        let bindings = std::collections::BTreeMap::from([("person".to_string(),
            std::collections::BTreeMap::from([("body/jab".to_string(), vec!["jab".to_string(), "idle".to_string()])]))]);
        let report = sheet(&path, &["death".to_string()], &bindings).unwrap();
        std::fs::remove_dir_all(directory).unwrap();
        assert_eq!(report[0]["key"], "person");
        assert_eq!(report[0]["target"], "toon");
        assert_eq!(report[0]["bindings"]["body/jab"]["state"], "fallback");
        assert_eq!(report[0]["bindings"]["body/jab"]["row"], "idle");
    }

}

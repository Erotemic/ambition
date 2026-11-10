//! The selected-file contract, read by the runtime's reader.
//!
//! Each case under `tests/data/selected_files/` is a library and the members
//! it selects: logical name, origin (a `published` product or a `loose`
//! library file) and generation, or the word an error must contain.
//! `scripts/tests/test_selected_files_contract.py` reads the same cases
//! with the packaging reader, so the two languages agree by fixture, not by
//! a shared service.

use std::collections::BTreeMap;
use std::path::Path;

use ambition_asset_manager::content_library::ContentLibrary;
use serde_json::{json, Value};

fn selected(case: &Path) -> Result<Value, String> {
    let files = ContentLibrary::read(case.join("library.json"))?.files()?;
    let rows: BTreeMap<String, Value> = files
        .into_iter()
        .map(|(logical, file)| {
            let origin = if file.generation.is_some() { "published" } else { "loose" };
            (logical, json!({"origin": origin, "generation": file.generation}))
        })
        .collect();
    Ok(json!({ "files": rows }))
}

#[test]
fn the_runtime_reader_selects_what_each_shared_case_expects() {
    let cases = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/selected_files");
    let mut seen = 0;
    for case in std::fs::read_dir(&cases).expect("the shared cases").map(|entry| entry.unwrap().path()) {
        let expected: Value =
            serde_json::from_str(&std::fs::read_to_string(case.join("expected.json")).unwrap()).unwrap();
        let name = case.file_name().unwrap().to_string_lossy().into_owned();
        match (selected(&case), expected.get("error").and_then(Value::as_str)) {
            (Ok(actual), None) => assert_eq!(actual, expected, "{name}"),
            (Err(error), Some(word)) => assert!(
                error.to_lowercase().contains(word),
                "{name}: the error does not say {word:?}: {error}"
            ),
            (Ok(actual), Some(word)) => panic!("{name}: selected {actual} where the case expects an error ({word})"),
            (Err(error), None) => panic!("{name}: {error}"),
        }
        seen += 1;
    }
    assert!(seen > 0, "the shared cases were not found under {}", cases.display());
}

//! The windowed host is the ENGINE face's presentation companion, not a game:
//! it must never name `ambition_content`. This locks the E5-step-5 exit
//! invariant from the scaffold onward, so the moment fable moves a system that
//! reaches for a content type, this test fails instead of the boundary rotting.

use std::fs;
use std::path::{Path, PathBuf};

fn crate_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

#[test]
fn manifest_does_not_depend_on_ambition_content() {
    let manifest = fs::read_to_string(crate_dir().join("Cargo.toml")).expect("read Cargo.toml");
    // Ignore comment lines — the docs deliberately MENTION the forbidden crate.
    let has_dep = manifest.lines().any(|line| {
        let line = line.trim();
        !line.starts_with('#')
            && (line.starts_with("ambition_content =") || line.starts_with("ambition_content."))
    });
    assert!(
        !has_dep,
        "ambition_host must NOT depend on ambition_content (E5 step-5 boundary)"
    );
}

#[test]
fn sources_name_no_content_crate() {
    let src = crate_dir().join("src");
    let mut stack = vec![src];
    while let Some(path) = stack.pop() {
        let Ok(meta) = fs::metadata(&path) else {
            continue;
        };
        if meta.is_dir() {
            for entry in fs::read_dir(&path).expect("read dir") {
                stack.push(entry.expect("dir entry").path());
            }
        } else if path.extension().is_some_and(|e| e == "rs") {
            let text = fs::read_to_string(&path).expect("read source");
            for line in text.lines() {
                let trimmed = line.trim();
                // Skip doc/comment lines — the crate docs name the forbidden
                // crate to explain the invariant.
                if trimmed.starts_with("//") || trimmed.starts_with('*') {
                    continue;
                }
                assert!(
                    !trimmed.contains("ambition_content"),
                    "ambition_host source names ambition_content in {}: {}",
                    path.display(),
                    trimmed
                );
            }
        }
    }
}

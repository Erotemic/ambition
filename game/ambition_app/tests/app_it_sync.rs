//! Guard that keeps the aggregated `app_it` binary in sync with the `tests/`
//! directory.
//!
//! `ambition_app` sets `autotests = false` and declares a single `[[test]]`
//! (`app_it`) that `mod`-includes every integration source. That collapses ~46
//! Bevy-linking test binaries into one — a big compile win — but it means a new
//! `tests/foo.rs` compiles and runs ONLY if someone also adds `mod foo;` here.
//! With autotests off, a forgotten `mod` line does not warn or fail: the file is
//! simply never built, so its tests silently vanish. This test makes that a hard
//! failure instead.
//!
//! It also fails on the reverse mistakes: a `mod` with no source file, or a
//! duplicate `mod` line.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

/// Parse the top-level `mod <name>;` declarations from `app_it.rs`.
fn declared_modules(app_it_src: &str) -> Vec<String> {
    let mut out = Vec::new();
    for raw in app_it_src.lines() {
        let line = raw.trim();
        // Only plain top-level module declarations. `pub mod`, `mod x { .. }`
        // inline modules, and commented lines are not aggregate includes.
        if let Some(rest) = line.strip_prefix("mod ") {
            if let Some(name) = rest.strip_suffix(';') {
                let name = name.trim();
                if !name.is_empty() {
                    out.push(name.to_string());
                }
            }
        }
    }
    out
}

/// Every module source on disk: top-level `tests/<name>.rs` (except the aggregate
/// entrypoint itself) and directory modules `tests/<name>/mod.rs` (e.g. `common`).
/// Data-only subdirectories with no `mod.rs` (e.g. `fixtures`) are not modules.
fn source_modules(tests_dir: &Path) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for entry in fs::read_dir(tests_dir).expect("read tests/ directory") {
        let path = entry.expect("tests/ dir entry").path();
        if path.is_file() {
            if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                let stem = path.file_stem().and_then(|s| s.to_str()).unwrap();
                if stem != "app_it" {
                    out.insert(stem.to_string());
                }
            }
        } else if path.is_dir() && path.join("mod.rs").is_file() {
            let name = path.file_name().and_then(|s| s.to_str()).unwrap();
            out.insert(name.to_string());
        }
    }
    out
}

#[test]
fn app_it_aggregate_matches_tests_directory() {
    let tests_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let app_it_src = fs::read_to_string(tests_dir.join("app_it.rs")).expect("read tests/app_it.rs");

    let declared = declared_modules(&app_it_src);
    let declared_set: BTreeSet<String> = declared.iter().cloned().collect();
    assert_eq!(
        declared.len(),
        declared_set.len(),
        "app_it.rs declares a duplicate `mod`: {declared:?}"
    );

    let sources = source_modules(&tests_dir);

    let missing_include: Vec<&String> = sources.difference(&declared_set).collect();
    assert!(
        missing_include.is_empty(),
        "tests/ source(s) not `mod`-included in app_it.rs (autotests=false ⇒ their \
         tests are SILENTLY skipped): {missing_include:?}. Add a `mod <name>;` line."
    );

    let dangling_mod: Vec<&String> = declared_set.difference(&sources).collect();
    assert!(
        dangling_mod.is_empty(),
        "app_it.rs declares `mod`(s) with no matching tests/ source: {dangling_mod:?}"
    );
}

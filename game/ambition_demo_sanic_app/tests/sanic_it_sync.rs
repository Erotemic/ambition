//! Guard: the aggregate must stay in sync with the `tests/` directory.
//!
//! This crate sets `autotests = false` and declares ONE `[[test]]` that
//! `mod`-includes every integration source, collapsing 11 Bevy-linking test
//! binaries into one. The cost of that win is a real hazard: with autotests off
//! a new `tests/foo.rs` compiles and runs ONLY if someone also adds `mod foo;`
//! to the aggregate, and a forgotten line does not warn — the file is simply
//! never built and its tests silently vanish. This makes that a hard failure.
//!
//! Same shape as `ambition_app`'s `app_it_sync`, for the same reason.

use std::collections::BTreeSet;

const AGGREGATE: &str = include_str!("sanic_it.rs");

#[test]
fn every_integration_source_is_included_in_the_aggregate() {
    let declared: BTreeSet<String> = AGGREGATE
        .lines()
        .map(str::trim)
        .filter_map(|line| line.strip_prefix("mod ").and_then(|r| r.strip_suffix(';')))
        .map(str::to_string)
        .collect();

    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let on_disk: BTreeSet<String> = std::fs::read_dir(&dir)
        .expect("tests/ is readable")
        .filter_map(Result::ok)
        .filter_map(|e| {
            let p = e.path();
            (p.extension().is_some_and(|x| x == "rs")).then(|| {
                p.file_stem().unwrap().to_string_lossy().into_owned()
            })
        })
        // The aggregate itself is the binary, not a module of it.
        .filter(|name| name != "sanic_it")
        .collect();

    let missing: Vec<_> = on_disk.difference(&declared).collect();
    assert!(
        missing.is_empty(),
        "these tests/*.rs files are NOT in sanic_it.rs and therefore never run: {missing:?}\n\
         add `mod <name>;` to tests/sanic_it.rs"
    );
    let orphaned: Vec<_> = declared.difference(&on_disk).collect();
    assert!(
        orphaned.is_empty(),
        "sanic_it.rs declares modules with no source file: {orphaned:?}"
    );
}

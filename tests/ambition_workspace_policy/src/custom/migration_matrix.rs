//! Completeness checker for the architecture_boundaries.rs migration matrix.
//!
//! `migration_matrix.toml` maps every #[test] that was in the legacy
//! architecture_boundaries.rs to exactly one disposition. This asserts:
//!   * a bijection between the matrix and the frozen canonical name list;
//!   * every `declarative`/`custom` destination resolves to a real policy ID;
//!   * `removed`/`retained` carry a justification note;
//!   * the matrix cannot lie — a `legacy-pending` entry's fn still lives in the
//!     legacy file, and a migrated/removed/retained entry's fn is gone from it.
//!
//! When the legacy file is finally deleted (Task 9), the "still in file" set is
//! empty, so any remaining `legacy-pending` entry fails — the campaign is only
//! done when the matrix has zero pending entries.

use std::collections::BTreeSet;

use serde::Deserialize;

use crate::workspace::{self, Workspace};

const FROZEN_LIST: &str =
    "tests/ambition_workspace_policy/fixtures/architecture_boundaries_source_tests.txt";
const MATRIX: &str = "tests/ambition_workspace_policy/migration_matrix.toml";
const LEGACY_FILE: &str = "game/ambition_app/tests/architecture_boundaries.rs";

#[derive(Debug, Deserialize)]
struct Matrix {
    #[serde(default)]
    entry: Vec<Entry>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    old_test: String,
    disposition: String,
    #[serde(default)]
    policies: Vec<String>,
    #[serde(default)]
    note: String,
}

fn frozen_names(ws: &Workspace) -> BTreeSet<String> {
    let text = std::fs::read_to_string(ws.abs(FROZEN_LIST)).expect("read frozen test list");
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(str::to_string)
        .collect()
}

fn legacy_fn_names(ws: &Workspace) -> BTreeSet<String> {
    let path = ws.abs(LEGACY_FILE);
    let Ok(text) = std::fs::read_to_string(&path) else {
        return BTreeSet::new(); // file deleted (Task 9) — nothing is legacy-pending anymore
    };
    let mut out = BTreeSet::new();
    for line in text.lines() {
        let t = line.trim_start();
        if let Some(rest) = t.strip_prefix("fn ") {
            let name: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if name.starts_with("architecture_boundaries_") {
                out.insert(name);
            }
        }
    }
    out
}

/// The full check. Panics with a specific message on any incompleteness (this is
/// a self-test, not a scanned policy).
pub fn check(ws: &Workspace) {
    let frozen = frozen_names(ws);
    assert_eq!(
        frozen.len(),
        67,
        "the frozen canonical list must hold all 67 original tests"
    );

    let text = std::fs::read_to_string(ws.abs(MATRIX)).expect("read migration_matrix.toml");
    let matrix: Matrix = toml::from_str(&text).expect("parse migration_matrix.toml");

    // Bijection: no dupes, and exactly the frozen set.
    let mut seen = BTreeSet::new();
    for e in &matrix.entry {
        assert!(
            seen.insert(e.old_test.clone()),
            "duplicate matrix entry for `{}`",
            e.old_test
        );
    }
    assert_eq!(
        seen,
        frozen,
        "migration_matrix.toml must map EXACTLY the frozen 67 tests \
         (missing: {:?}; extra: {:?})",
        frozen.difference(&seen).collect::<Vec<_>>(),
        seen.difference(&frozen).collect::<Vec<_>>(),
    );

    // Known destination IDs = every declarative policy ID + every custom-scanner
    // policy ID (derived from `custom::metas()`, so it cannot drift).
    let mut known: BTreeSet<String> = workspace::load_all_policies()
        .into_iter()
        .map(|p| p.id)
        .collect();
    known.extend(super::metas().into_iter().map(|m| m.id));

    let in_legacy = legacy_fn_names(ws);

    for e in &matrix.entry {
        match e.disposition.as_str() {
            "declarative" | "custom" => {
                assert!(
                    !e.policies.is_empty(),
                    "`{}` is {} but names no policies",
                    e.old_test,
                    e.disposition
                );
                for id in &e.policies {
                    assert!(
                        known.contains(id),
                        "`{}` maps to policy `{id}`, which does not exist",
                        e.old_test
                    );
                }
                assert!(
                    !in_legacy.contains(&e.old_test),
                    "`{}` is marked {} but its fn is STILL in {LEGACY_FILE} — remove it",
                    e.old_test,
                    e.disposition
                );
            }
            "removed" | "retained" => {
                assert!(
                    e.note.trim().len() > 10,
                    "`{}` is {} but has no justification note",
                    e.old_test,
                    e.disposition
                );
                assert!(
                    !in_legacy.contains(&e.old_test),
                    "`{}` is marked {} but its fn is STILL in {LEGACY_FILE}",
                    e.old_test,
                    e.disposition
                );
            }
            "legacy-pending" => {
                assert!(
                    in_legacy.contains(&e.old_test),
                    "`{}` is legacy-pending but its fn is NOT in {LEGACY_FILE} \
                     (already migrated? update the matrix)",
                    e.old_test
                );
            }
            other => panic!("`{}` has unknown disposition `{other}`", e.old_test),
        }
    }

    // If the legacy file is gone, nothing may still be legacy-pending.
    if !ws.abs(LEGACY_FILE).exists() {
        let pending: Vec<&str> = matrix
            .entry
            .iter()
            .filter(|e| e.disposition == "legacy-pending")
            .map(|e| e.old_test.as_str())
            .collect();
        assert!(
            pending.is_empty(),
            "the legacy file is deleted but these entries are still legacy-pending: {pending:?}"
        );
    }
}

/// Every fn currently in the legacy file must have a `legacy-pending` matrix
/// entry — so a NEW test added to the legacy file cannot slip the ledger.
pub fn legacy_file_is_fully_tracked(ws: &Workspace) {
    if !ws.abs(LEGACY_FILE).exists() {
        return;
    }
    let text = std::fs::read_to_string(ws.abs(MATRIX)).expect("read matrix");
    let matrix: Matrix = toml::from_str(&text).expect("parse matrix");
    let tracked: BTreeSet<&str> = matrix.entry.iter().map(|e| e.old_test.as_str()).collect();
    for name in legacy_fn_names(ws) {
        assert!(
            tracked.contains(name.as_str()),
            "`{name}` is in {LEGACY_FILE} but absent from the migration matrix"
        );
    }
}

//! **D-B's module-size gate** (`docs/planning/engine/decomposition.md`).
//!
//! The standard: a production module is at most ~1.5k lines. It was DOCUMENTED but
//! never enforced (audit H6), and nine modules had grown past it — `scripts/modules_md.py`
//! mentions the limit in its docstring but only checks crate-root module *maps*, not
//! line counts. This is the executable half: the thing that makes D-B a gate rather
//! than a paragraph, and the reason D-B is REOPENED.
//!
//! **Scope** (the decision the audit asked us to make and document): every PRODUCTION
//! `.rs` under `crates/*/src` and `game/*/src` — engine code AND content/demo code,
//! because the standard says "every engine crate," not only `ambition_actors`.
//! Standalone test files are excluded by path (`tests.rs`, `/tests/`). Inline
//! `#[cfg(test)]` modules count toward their file's size — a 3.7k-line file is hard to
//! navigate no matter how much of it is tests, and this matches how the audit counted.
//!
//! **Generated files and unusually data-heavy declarative modules are NOT inferred.**
//! Per the auditor: the script must not guess a "generated" or "declarative" category
//! heuristically. Such a file goes in [`WAIVERS`] with a reason, so an exception is
//! always a visible review event.
//!
//! The waiver list is BIDIRECTIONAL, like `control_frame_lint.rs`: a file over the
//! limit that is not waived fails (split it, or waive it with a reason); a waiver for a
//! file that is no longer over the limit ALSO fails (the split landed — delete the
//! stale waiver and celebrate). [`WAIVERS`] is a DEBT LEDGER. Shrink it; do not grow it
//! without a reason that survives review.

use std::path::{Path, PathBuf};

/// The line ceiling for one production module. `~1.5k` in the doc; 1500 here.
const LIMIT: usize = 1500;

/// Production modules currently allowed over [`LIMIT`], each with the reason it is not
/// yet split. Every entry is D-B decomposition debt. Two carry the audit's M9 tag
/// (snapshot.rs and moveset.rs are named god modules whose split is scheduled for the
/// N3.2 codec relocation).
const WAIVERS: &[(&str, &str)] = &[
    (
        "crates/ambition_runtime/src/snapshot.rs",
        "N3.1 bootstrap god module (audit M9): public API + wire format + registry + \
         restore/reconciliation + per-crate codecs + diagnostics in one file. The codec \
         trait moves down and the file splits during the N3.2 substrate work (netcode.md).",
    ),
    (
        "crates/ambition_combat/src/moveset.rs",
        "the moveset runtime (audit M9): MoveSpec pricing + playback proper-time clock + \
         strike-volume systems. Splits alongside the N3.2 codec relocation.",
    ),
    (
        "crates/ambition_portal_presentation/src/view_cones.rs",
        "portal view-cone assembly. Presentation (never sim-hashed); split tracked with \
         the portal-presentation work, no natural seam extracted yet.",
    ),
    (
        "crates/ambition_portal_presentation/src/view_cones/geometry.rs",
        "portal view-cone geometry math. Presentation; dense trig with no clean seam yet.",
    ),
    (
        "crates/ambition_characters/src/brain/smash/mod.rs",
        "the smash-brain state machine. A decomposition target; no sub-module extracted yet.",
    ),
    (
        "crates/ambition_engine_core/src/surface.rs",
        "the collision-surface primitive. Core geometry; a split is not yet scoped.",
    ),
    (
        "crates/ambition_entity_catalog/src/lib.rs",
        "data-heavy declarative catalog — the auditor's named exception class (a large \
         table of authored entity definitions, not branching logic).",
    ),
    (
        "game/ambition_app/src/menu/kaleidoscope_app.rs",
        "declarative menu UI (a Lunex node tree). Presentation, data-heavy by nature.",
    ),
    (
        "game/ambition_content/src/falling_sand.rs",
        "falling-sand grid simulation + its visual-sync systems in one module. Content; \
         a sim/presentation split is not yet scoped.",
    ),
];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("repo root")
}

/// A standalone test file, excluded from the size gate: its lines are test scaffolding,
/// not a production module a reader has to navigate.
fn is_test_path(rel: &str) -> bool {
    rel.ends_with("tests.rs") || rel.contains("/tests/")
}

/// Every production `.rs` under `crates/*/src` and `game/*/src`, as
/// `(workspace-relative path, line count)`.
fn production_sources() -> Vec<(String, usize)> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                out.push(path);
            }
        }
    }

    let root = repo_root();
    let mut out = Vec::new();
    for parent in ["crates", "game"] {
        let Ok(members) = std::fs::read_dir(root.join(parent)) else {
            continue;
        };
        for member in members.flatten() {
            let src = member.path().join("src");
            if !src.is_dir() {
                continue;
            }
            let mut files = Vec::new();
            walk(&src, &mut files);
            for path in files {
                let rel = path
                    .strip_prefix(&root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                if is_test_path(&rel) {
                    continue;
                }
                let lines = std::fs::read_to_string(&path)
                    .map(|s| s.lines().count())
                    .unwrap_or(0);
                out.push((rel, lines));
            }
        }
    }
    out
}

/// The walk itself must not silently scan nothing — a size gate over zero files is a
/// green that means nothing.
#[test]
fn the_size_scan_is_not_vacuous() {
    let n = production_sources().len();
    assert!(
        n > 300,
        "only {n} production sources walked — the `crates/*/src` + `game/*/src` walk is \
         probably broken, and the size gate would pass vacuously"
    );
}

/// **The gate.** No production module exceeds the limit unless it is waived with a reason.
#[test]
fn no_production_module_exceeds_the_size_limit_unwaived() {
    let waived: std::collections::HashSet<&str> = WAIVERS.iter().map(|(p, _)| *p).collect();
    let mut violations: Vec<String> = production_sources()
        .into_iter()
        .filter(|(rel, n)| *n > LIMIT && !waived.contains(rel.as_str()))
        .map(|(rel, n)| format!("  {rel}: {n} lines (limit {LIMIT})"))
        .collect();
    violations.sort();
    assert!(
        violations.is_empty(),
        "\nD-B module-size gate — {} module(s) over {LIMIT} lines with no waiver.\n\n\
         Split the module, or — if it is generated or genuinely data-heavy-declarative —\n\
         add it to `WAIVERS` in this file with a one-line reason. A waiver is a visible\n\
         review event, not a rubber stamp.\n\n{}\n",
        violations.len(),
        violations.join("\n"),
    );
}

/// **The other direction.** A waiver for a file that is no longer oversized — because the
/// split landed, or the file moved — is stale and must be deleted. Without this a waiver
/// list only ever grows, and D-B never records that a module was actually decomposed.
#[test]
fn every_waiver_names_a_currently_oversized_file() {
    let sizes: std::collections::HashMap<String, usize> =
        production_sources().into_iter().collect();
    let mut stale: Vec<String> = Vec::new();
    for (path, _reason) in WAIVERS {
        match sizes.get(*path) {
            Some(n) if *n > LIMIT => {}
            Some(n) => stale.push(format!(
                "  {path}: waived, but only {n} lines now — the split landed, DELETE the waiver"
            )),
            None => stale.push(format!(
                "  {path}: waived, but no such production file — it moved or was removed, DELETE the waiver"
            )),
        }
    }
    assert!(
        stale.is_empty(),
        "\nD-B module-size gate — {} stale waiver(s). D-B debt was paid; record it by \
         removing the waiver:\n\n{}\n",
        stale.len(),
        stale.join("\n"),
    );
}

/// Every waiver explains itself. A waiver list of bare paths is a silent allowlist —
/// exactly the count-only-false-green shape the audit (M10) warns against.
#[test]
fn every_waiver_has_a_real_reason() {
    for (path, reason) in WAIVERS {
        assert!(
            reason.trim().len() > 20,
            "the waiver for `{path}` has no real reason — say WHY it is exempt, so the \
             next reader can judge whether the exemption still holds"
        );
    }
}

//! Guardrail: production Rust under `src/` must not re-introduce the
//! legacy `SandboxRuntime` / `FeatureRuntime` god-objects, the
//! `runtime.player` shadow cache, the `feature_runtime_phase`
//! procedural helper, or the monolithic `ae::Player` aggregate. The
//! ECS migration deleted these intentionally; the canonical
//! replacements live on dedicated components and systems
//! (`BodyKinematics`, `BodyGroundState`, …, `BodyComboTrace`,
//! `PlayerClustersMut`, `PlayerClusterScratch`).
//!
//! This test walks the sandbox crate's `src/` tree and fails if any
//! identifier matches. Test files and historical/archived docs are
//! exempt:
//!
//! - test code (`tests/`, `src/**/tests*.rs`, `#[cfg(test)] mod tests`)
//!   keeps short-lived fixture names so we don't ossify them here;
//! - markdown docs and journal entries describe historical state;
//! - explicit `ALLOW_LEGACY_RUNTIME` lines opt a single occurrence out
//!   when there's a legitimate reason (e.g. a public alias used by an
//!   ADR or a backwards-compat shim).

use std::fs;
use std::path::{Path, PathBuf};

const FORBIDDEN: &[&str] = &[
    "SandboxRuntime",
    "FeatureRuntime",
    "feature_runtime_phase",
    "runtime.player",
    // Player ECS migration final step (2026-05-28): the monolithic
    // `ae::Player` aggregate is gone. The cluster components on the
    // player entity (and `PlayerClusterScratch` for tests) are the
    // only path. Re-introducing any of these spellings means a
    // shadow scratchpad has crept back in.
    "ae::Player::new",
    "PlayerClustersMut::to_player",
    "::from_player(",
    "update_player_with_tuning(",
    "update_player_control_with_tuning(",
    "update_player_simulation_with_tuning(",
];

const ALLOW_MARKER: &str = "ALLOW_LEGACY_RUNTIME";

/// Recursively collect every `.rs` file under `dir`.
fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            // Skip nested test directories named `tests` (still scanned
            // by name in src/**/tests*.rs branch below).
            if path.file_name().and_then(|n| n.to_str()) == Some("tests") {
                continue;
            }
            collect_rs_files(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            // Skip `tests.rs` and `*_tests.rs` siblings — those are test
            // modules referenced from production code via `#[cfg(test)]`.
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default();
            if stem == "tests" || stem.ends_with("_tests") {
                continue;
            }
            out.push(path);
        }
    }
}

/// Strip `#[cfg(test)] mod tests { … }` and any line containing
/// the `ALLOW_LEGACY_RUNTIME` opt-out marker before scanning.
fn scannable_lines(src: &str) -> impl Iterator<Item = (usize, &str)> {
    // Treat anything under a `#[cfg(test)]` attribute as test code by
    // dropping from that attribute through the next balanced `}` at
    // column 0. We don't try to parse braces precisely — instead we
    // bracket from the marker to the next `} // end-tests` style
    // closer is overkill. For this codebase, a simpler rule works:
    // drop runs of lines that start at a `#[cfg(test)]` attribute and
    // continue until the file's last brace at indent 0.
    let drop_after = src.find("#[cfg(test)]").unwrap_or(src.len());
    let scan = &src[..drop_after];
    scan.lines()
        .enumerate()
        .filter(|(_, line)| !line.contains(ALLOW_MARKER))
}

#[test]
fn no_legacy_runtime_references_in_production_src() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let src_root = Path::new(manifest_dir).join("src");
    let mut files = Vec::new();
    collect_rs_files(&src_root, &mut files);

    assert!(
        !files.is_empty(),
        "guardrail found no .rs files under {} — wrong path?",
        src_root.display()
    );

    let mut hits: Vec<String> = Vec::new();
    for file in &files {
        let Ok(content) = fs::read_to_string(file) else {
            continue;
        };
        for (line_idx, line) in scannable_lines(&content) {
            for needle in FORBIDDEN {
                if line.contains(needle) {
                    hits.push(format!(
                        "{}:{}: `{}` (line: {})",
                        file.strip_prefix(manifest_dir).unwrap_or(file).display(),
                        line_idx + 1,
                        needle,
                        line.trim(),
                    ));
                }
            }
        }
    }

    assert!(
        hits.is_empty(),
        "legacy runtime identifiers reappeared in production sandbox src; \
         these were deleted by the ECS migration and must not return. \
         Add `ALLOW_LEGACY_RUNTIME` to the line if a single occurrence \
         is intentional (e.g. an archived doc reference).\n\n{}",
        hits.join("\n"),
    );
}

#[test]
fn scanner_catches_each_forbidden_identifier() {
    // Self-test: feed the scanner synthetic source with every banned
    // identifier and assert it reports all of them. Prevents silent
    // breakage if the scanner ever skips a line type it shouldn't.
    let src = "\
pub struct SandboxRuntime;
pub fn touch_runtime() { let _ = runtime.player; }
fn feature_runtime_phase() {}
mod thing { struct FeatureRuntime; }
let p = ae::Player::new(spawn);
let snap = PlayerClustersMut::to_player(&clusters);
let scratch = PlayerKinematics::from_player(&p);
let e = update_player_with_tuning(&world, &mut p, input, dt, tuning);
let e = update_player_control_with_tuning(&world, &mut p, input, dt, tuning);
let e = update_player_simulation_with_tuning(&world, &mut p, input, dt, tuning);
let allowed = SandboxRuntime; // ALLOW_LEGACY_RUNTIME
";
    let lines: Vec<&str> = scannable_lines(src).map(|(_, l)| l).collect();
    let joined = lines.join("\n");

    for needle in FORBIDDEN {
        assert!(
            joined.contains(needle),
            "scanner dropped a line containing `{needle}`",
        );
    }
    // The ALLOW_LEGACY_RUNTIME line must be filtered out.
    assert!(
        !joined.contains("// ALLOW_LEGACY_RUNTIME"),
        "ALLOW_LEGACY_RUNTIME opt-out marker did not suppress its line",
    );
}

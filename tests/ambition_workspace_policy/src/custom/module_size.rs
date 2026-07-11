//! **D-B's module-size gate** (custom scanner half of the module-size policy).
//!
//! The scanner is Rust; the limit + waivers are DATA in
//! `policies/module_size.toml`. It counts every PRODUCTION `.rs` under
//! `crates/*/src` and `game/*/src` (test files excluded centrally via
//! [`workspace::is_test_path`]; inline `#[cfg(test)]` counts toward its file) and
//! reports, as diagnostics on the engine [`Report`]:
//!   * a file over the limit with no waiver;
//!   * a stale waiver (the file is no longer over the limit, or was removed).
//!
//! Vacuity and waiver-reason quality are harness invariants, asserted directly
//! (a size gate over zero files, or a rubber-stamp waiver, is exactly the
//! false-green the audit warned against).

use std::path::Path;

use serde::Deserialize;

use crate::model::{CustomMeta, Report, Scope, Severity};
use crate::workspace::{self, Workspace};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    id: String,
    scope: Scope,
    #[serde(default)]
    owners: Vec<String>,
    #[serde(default)]
    source_doc: String,
    rationale: String,
    limit: usize,
    roots: Vec<String>,
    #[serde(default)]
    waiver: Vec<Waiver>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Waiver {
    path: String,
    reason: String,
}

fn load_config() -> Config {
    let path = workspace::policies_dir().join("module_size.toml");
    let text =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    toml::from_str(&text).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()))
}

/// Every production `.rs` under `<root>/*/src` for each root, as
/// `(workspace-relative path, line count)`. Test files are excluded centrally.
fn production_sources(ws: &Workspace, roots: &[String]) -> Vec<(String, usize)> {
    let mut out = Vec::new();
    for parent in roots {
        let Ok(members) = std::fs::read_dir(ws.abs(parent)) else {
            continue;
        };
        for member in members.flatten() {
            let src = member.path().join("src");
            if !src.is_dir() {
                continue;
            }
            for path in workspace::rust_sources_under(&src) {
                let rel = path
                    .strip_prefix(ws.root())
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/");
                if workspace::is_test_path(&rel) {
                    continue;
                }
                let lines = count_lines(&path);
                out.push((rel, lines));
            }
        }
    }
    out
}

fn count_lines(path: &Path) -> usize {
    std::fs::read_to_string(path)
        .map(|s| s.lines().count())
        .unwrap_or(0)
}

/// Uniform metadata for the module-size policy (its watched roots are the crate
/// parents it scans).
pub fn metas() -> Vec<CustomMeta> {
    let cfg = load_config();
    vec![CustomMeta {
        id: cfg.id,
        scope: cfg.scope,
        owners: cfg.owners,
        watch_paths: cfg.roots,
        source_doc: cfg.source_doc,
        severity: Severity::Error,
    }]
}

/// The gate. Appends oversized + stale-waiver diagnostics to `report`; asserts
/// non-vacuity and waiver-reason quality (config sanity).
pub fn run(ws: &Workspace, report: &mut Report) {
    let cfg = load_config();
    assert_eq!(
        cfg.scope,
        Scope::Engine,
        "module_size.toml must be engine-scoped"
    );

    // Waiver reasons are config sanity — a bare path is a silent allowlist, the
    // count-only-false-green shape audit M10 warns against.
    for w in &cfg.waiver {
        assert!(
            w.reason.trim().len() > 20,
            "the module-size waiver for `{}` has no real reason — say WHY it is exempt",
            w.path
        );
    }

    let sources = production_sources(ws, &cfg.roots);
    // A size gate over zero files is a green that means nothing.
    assert!(
        sources.len() > 300,
        "only {} production sources walked under {:?} — the crates/*/src + game/*/src walk is \
         probably broken, and the size gate would pass vacuously",
        sources.len(),
        cfg.roots,
    );

    evaluate(&cfg, &sources, report);
}

/// Pure evaluation, separated so the poison self-test can drive it with a hostile
/// limit/waiver set.
fn evaluate(cfg: &Config, sources: &[(String, usize)], report: &mut Report) {
    use std::collections::HashMap;
    let waived: HashMap<&str, &str> = cfg
        .waiver
        .iter()
        .map(|w| (w.path.as_str(), w.reason.as_str()))
        .collect();
    let diag = |location: String, detail: String| crate::model::Diagnostic {
        policy_id: cfg.id.clone(),
        owners: cfg.owners.clone(),
        source_doc: cfg.source_doc.clone(),
        rationale: cfg.rationale.clone(),
        location,
        detail,
    };

    // Oversized-and-unwaived.
    let mut oversized: Vec<(String, usize)> = sources
        .iter()
        .filter(|(rel, n)| *n > cfg.limit && !waived.contains_key(rel.as_str()))
        .cloned()
        .collect();
    oversized.sort();
    for (rel, n) in oversized {
        report.push(diag(
            rel,
            format!(
                "{n} lines > limit {} with no waiver — split it, or add it to \
                 module_size.toml with a one-line reason",
                cfg.limit
            ),
        ));
    }

    // Stale waivers (the other direction).
    let sizes: std::collections::HashMap<&str, usize> =
        sources.iter().map(|(r, n)| (r.as_str(), *n)).collect();
    for w in &cfg.waiver {
        match sizes.get(w.path.as_str()) {
            Some(n) if *n > cfg.limit => {}
            Some(n) => report.push(diag(
                w.path.clone(),
                format!("waived, but only {n} lines now — the split landed, DELETE the waiver"),
            )),
            None => report.push(diag(
                w.path.clone(),
                "waived, but no such production file — it moved or was removed, DELETE the waiver"
                    .to_string(),
            )),
        }
    }
}

// ── poison self-test (called from tests/policy.rs) ───────────────────────────

/// Prove the gate reacts: a limit of 1 line makes essentially every file
/// oversized, and a waiver for a nonexistent path is stale. Uses the REAL source
/// walk so it exercises the shape production code has.
pub fn poison_reacts(ws: &Workspace) {
    let cfg = Config {
        id: "poison.module-size".into(),
        scope: Scope::Engine,
        owners: vec![],
        source_doc: String::new(),
        rationale: "poison".into(),
        limit: 1,
        roots: vec!["crates".into(), "game".into()],
        waiver: vec![Waiver {
            path: "crates/does_not_exist/src/ghost.rs".into(),
            reason: "a reason long enough to pass the reason-quality gate".into(),
        }],
    };
    let sources = production_sources(ws, &cfg.roots);
    let mut report = Report::new(Scope::Engine);
    evaluate(&cfg, &sources, &mut report);
    assert!(
        report.len() > 100,
        "module-size gate must flag the flood of >1-line files, got {}",
        report.len()
    );
    assert!(
        report
            .diagnostics()
            .iter()
            .any(|d| d.location == "crates/does_not_exist/src/ghost.rs"),
        "module-size gate must flag the stale waiver for a nonexistent file"
    );
}

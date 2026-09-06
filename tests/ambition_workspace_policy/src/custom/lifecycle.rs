//! The raw-spawn allowlist gate (custom scanner). Room-authored spawn modules
//! under `features/ecs/spawn*` must not add raw `commands.spawn(...)` sites
//! beyond a per-file allowlist — use `SpawnScopedExt` lifecycle helpers instead.
//!
//! ⛔ THE SUBJECT IS A PATH PREFIX, NOT A FILE NAME, AND IT COST THIS GATE THREE
//! MONTHS OF COVERAGE TO LEARN THAT. Until 2026-09-03 the filter asked whether
//! the FILE NAME began with `spawn`, which was the same question while the module
//! was a single `features/ecs/spawn.rs`. `cdd0a0a0d` (2026-06-14) split that file
//! into `spawn/mod.rs` + `spawn/tests.rs`; neither name begins with `spawn`, so
//! the directory named for exactly what this gate governs became invisible to it,
//! and six production files sat ungoverned. Nothing failed — a name-matching gate
//! cannot report the file it stopped matching.
//!
//! ⇒ So the test is now `features/ecs/spawn…` on the path RELATIVE TO the scan
//! root, which answers `spawn_actors.rs` and `spawn/portal_construction.rs` with
//! one rule and cannot be undone by splitting a file into a directory again.
//! The allowlist (path=count) is `docs/architecture/architecture-boundary-allowlist.txt`.

use std::collections::{BTreeMap, BTreeSet};

use crate::model::{CustomMeta, Diagnostic, Report, Scope, Severity};
use crate::workspace::{self, Workspace};

const SPAWN_DIR: &str = "crates/ambition_platformer2d_actor_monolith/src/features/ecs";
const CRATE_SRC: &str = "crates/ambition_platformer2d_actor_monolith/src";
const ALLOWLIST: &str = "docs/architecture/architecture-boundary-allowlist.txt";
const POLICY_ID: &str = "engine.room-feature-spawns";

fn raw_spawn_count(text: &str) -> usize {
    text.matches("commands.spawn(").count()
}

fn read_allowlist(ws: &Workspace) -> BTreeMap<String, usize> {
    let path = ws.abs(ALLOWLIST);
    let text = std::fs::read_to_string(&path).expect("read raw-spawn allowlist");
    let mut out = BTreeMap::new();
    for (idx, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (rel, count) = line
            .split_once('=')
            .unwrap_or_else(|| panic!("{}:{} expected path=count", ALLOWLIST, idx + 1));
        let count = count
            .trim()
            .parse::<usize>()
            .unwrap_or_else(|_| panic!("{}:{} expected integer count", ALLOWLIST, idx + 1));
        let rel = rel.trim().to_string();
        assert!(
            out.insert(rel.clone(), count).is_none(),
            "{ALLOWLIST}:{} duplicate path {rel}",
            idx + 1
        );
    }
    out
}

pub fn metas() -> Vec<CustomMeta> {
    vec![CustomMeta {
        id: POLICY_ID.to_string(),
        scope: Scope::Engine,
        owners: vec!["ambition_platformer2d_actor_monolith".to_string()],
        watch_paths: vec![SPAWN_DIR.to_string(), ALLOWLIST.to_string()],
        source_doc: "docs/architecture/architecture-boundaries.md".to_string(),
        severity: Severity::Error,
    }]
}

pub fn run(ws: &Workspace, report: &mut Report) {
    let spawn_dir = ws.abs(SPAWN_DIR);
    assert!(
        spawn_dir.is_dir(),
        "raw-spawn gate path does not exist: {}",
        spawn_dir.display()
    );
    let allowlist = read_allowlist(ws);
    let src_root = ws.abs(CRATE_SRC);
    let mut scanned = 0usize;
    let mut seen = BTreeSet::new();

    for file in workspace::rust_sources_under(&spawn_dir) {
        // Relative to the SCAN ROOT (`features/ecs`), so `spawn_actors.rs` and
        // `spawn/portal_construction.rs` both begin with `spawn` and a future
        // `spawn/foo/bar.rs` still would. Matching the file name instead is what
        // let the `spawn/` directory escape this gate for three months.
        let under = file
            .strip_prefix(&spawn_dir)
            .expect("scanned file under the spawn scan root")
            .to_string_lossy()
            .replace('\\', "/");
        if !under.starts_with("spawn") {
            continue;
        }
        scanned += 1;
        let text = std::fs::read_to_string(&file).expect("read spawn source");
        let rel = file
            .strip_prefix(&src_root)
            .expect("spawn file under crate src")
            .to_string_lossy()
            .replace('\\', "/");
        seen.insert(rel.clone());
        let actual = raw_spawn_count(&text);
        match allowlist.get(&rel) {
            None => report.push(Diagnostic {
                policy_id: POLICY_ID.to_string(),
                owners: vec!["ambition_platformer2d_actor_monolith".to_string()],
                source_doc: "docs/architecture/architecture-boundaries.md".to_string(),
                rationale: "the room-feature raw-spawn allowlist is an exact inventory; every scanned spawn module must be reviewed explicitly".to_string(),
                location: format!("{CRATE_SRC}/{rel}"),
                detail: format!("missing exact inventory row in {ALLOWLIST}; current raw commands.spawn count is {actual}"),
            }),
            Some(allowed) if actual != *allowed => report.push(Diagnostic {
                policy_id: POLICY_ID.to_string(),
                owners: vec!["ambition_platformer2d_actor_monolith".to_string()],
                source_doc: "docs/architecture/architecture-boundaries.md".to_string(),
                rationale: "room-authored spawn modules must use scoped construction helpers, and the reviewed raw-spawn inventory must not retain excess allowance".to_string(),
                location: format!("{CRATE_SRC}/{rel}"),
                detail: format!(
                    "{actual} raw commands.spawn calls; exact reviewed count is {allowed} (update code or {ALLOWLIST} with justification)"
                ),
            }),
            Some(_) => {}
        }
    }
    for stale in allowlist.keys().filter(|rel| !seen.contains(*rel)) {
        report.push(Diagnostic {
            policy_id: POLICY_ID.to_string(),
            owners: vec!["ambition_platformer2d_actor_monolith".to_string()],
            source_doc: "docs/architecture/architecture-boundaries.md".to_string(),
            rationale: "the room-feature raw-spawn allowlist is an exact inventory and may not retain rows for removed or renamed files".to_string(),
            location: ALLOWLIST.to_string(),
            detail: format!("stale inventory row for missing or unscanned file: {stale}"),
        });
    }
    // ⛔ A FLOOR, NOT `> 0`. The bug this gate carried for three months left it
    // scanning two files and passing, so "scanned something" was true throughout
    // and proved nothing. Nine is what the path rule sees today; a split or a
    // rename may raise it, and only a DELETION should lower it — which is a
    // review, not a silent pass.
    assert!(
        scanned >= 9,
        "raw-spawn gate scanned {scanned} files under {SPAWN_DIR}, expected at          least 9 — a filter that stops matching is how this gate went blind before"
    );
}

pub fn poison_self_tests() {
    assert_eq!(raw_spawn_count("a commands.spawn( x commands.spawn( y"), 2);
    assert_eq!(raw_spawn_count("commands.spawn_room_scoped("), 0);
}

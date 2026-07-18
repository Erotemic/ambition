//! **The raw-spawn allowlist gate** (custom scanner). Room-authored spawn modules
//! under `features/ecs/spawn*.rs` must not add raw `commands.spawn(...)` sites
//! beyond a per-file allowlist — use `SpawnScopedExt` lifecycle helpers instead.
//! The allowlist (path=count) is `docs/architecture/architecture-boundary-allowlist.txt`.

use std::collections::{BTreeMap, BTreeSet};

use crate::model::{CustomMeta, Diagnostic, Report, Scope, Severity};
use crate::workspace::{self, Workspace};

const SPAWN_DIR: &str = "crates/ambition_actors/src/features/ecs";
const CRATE_SRC: &str = "crates/ambition_actors/src";
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
        owners: vec!["ambition_actors".to_string()],
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
        let name = file
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        if !name.starts_with("spawn") {
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
                owners: vec!["ambition_actors".to_string()],
                source_doc: "docs/architecture/architecture-boundaries.md".to_string(),
                rationale: "the room-feature raw-spawn allowlist is an exact inventory; every scanned spawn module must be reviewed explicitly".to_string(),
                location: format!("{CRATE_SRC}/{rel}"),
                detail: format!("missing exact inventory row in {ALLOWLIST}; current raw commands.spawn count is {actual}"),
            }),
            Some(allowed) if actual != *allowed => report.push(Diagnostic {
                policy_id: POLICY_ID.to_string(),
                owners: vec!["ambition_actors".to_string()],
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
            owners: vec!["ambition_actors".to_string()],
            source_doc: "docs/architecture/architecture-boundaries.md".to_string(),
            rationale: "the room-feature raw-spawn allowlist is an exact inventory and may not retain rows for removed or renamed files".to_string(),
            location: ALLOWLIST.to_string(),
            detail: format!("stale inventory row for missing or unscanned file: {stale}"),
        });
    }
    assert!(
        scanned > 0,
        "raw-spawn gate scanned no spawn*.rs files under {SPAWN_DIR} — vacuous"
    );
}

pub fn poison_self_tests() {
    assert_eq!(raw_spawn_count("a commands.spawn( x commands.spawn( y"), 2);
    assert_eq!(raw_spawn_count("commands.spawn_room_scoped("), 0);
}

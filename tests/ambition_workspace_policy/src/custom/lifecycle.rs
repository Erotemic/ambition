//! **The raw-spawn allowlist gate** (custom scanner). Room-authored spawn modules
//! under `features/ecs/spawn*.rs` must not add raw `commands.spawn(...)` sites
//! beyond a per-file allowlist — use `SpawnScopedExt` lifecycle helpers instead.
//! The allowlist (path=count) is `docs/architecture/architecture-boundary-allowlist.txt`.

use std::collections::BTreeMap;

use crate::model::{Diagnostic, Report};
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
        out.insert(rel.trim().to_string(), count);
    }
    out
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
        let actual = raw_spawn_count(&text);
        let allowed = *allowlist.get(&rel).unwrap_or(&0);
        if actual > allowed {
            report.push(Diagnostic {
                policy_id: POLICY_ID.to_string(),
                owners: vec!["ambition_actors".to_string()],
                source_doc: "docs/architecture/architecture-boundaries.md".to_string(),
                rationale: "room-authored spawn modules must not add raw commands.spawn sites; use SpawnScopedExt lifecycle helpers or update the allowlist with justification".to_string(),
                location: format!("{CRATE_SRC}/{rel}"),
                detail: format!(
                    "{actual} raw commands.spawn calls; allowed {allowed} (update {ALLOWLIST} with justification)"
                ),
            });
        }
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

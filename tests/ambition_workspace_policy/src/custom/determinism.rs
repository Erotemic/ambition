//! **The determinism lint set** (netcode N0.3 / ADR 0023) — custom scanner.
//!
//! The four properties that keep level-2 same-build determinism true:
//!   1. no ambient randomness (seeded, snapshot-registered RNG only);
//!   2. no wall-clock reads in the sim;
//!   3. no `std`-hash iteration order semantics;
//!   4. `Entity` is never an ordering key.
//!
//! The source analysis is genuinely semantic (bare-vs-qualified `HashMap`
//! detection, Bevy-hasher discrimination, binding tracking), so it stays Rust.
//! CONFIG (sim roots + scope, excluded subpaths, review marker, forbidden RNG
//! crates/calls, wall-clock reads, source doc) lives in `policies/determinism.toml`.
//!
//! Scoped: `run(ws, Engine, …)` scans the `crates/*` roots; `run(ws, Game, …)`
//! scans the `game/*` content + demo-rule roots. Both share this one compiled
//! scanner but are independently runnable.

use std::collections::BTreeSet;

use serde::Deserialize;

use crate::model::{Diagnostic, Report, Scope};
use crate::workspace::{self, Workspace};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    source_doc: String,
    rationale: String,
    review_marker: String,
    rng_crates: Vec<String>,
    wall_clock: Vec<String>,
    non_sim_subpaths: Vec<String>,
    #[serde(default)]
    banned_rng_call: Vec<BannedCall>,
    #[serde(default)]
    root: Vec<Root>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BannedCall {
    needle: String,
    why: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Root {
    path: String,
    scope: Scope,
}

fn load_config() -> Config {
    let path = workspace::policies_dir().join("determinism.toml");
    let text =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    toml::from_str(&text).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()))
}

/// The determinism variant of "is this a comment line": also treats attributes
/// (`#[`, `#![`) as comments, matching the original scanner so the reviewed-marker
/// block walk behaves identically.
fn is_comment(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("//") || t.starts_with('*') || t.starts_with("#![") || t.starts_with("#[")
}

/// The reviewed-exception opt-out: the marker on the line, or anywhere in the
/// contiguous comment block directly above it.
fn is_reviewed(marker: &str, lines: &[&str], idx: usize) -> bool {
    if lines[idx].contains(marker) {
        return true;
    }
    for line in lines[..idx].iter().rev() {
        if !is_comment(line) {
            return false;
        }
        if line.contains(marker) {
            return true;
        }
    }
    false
}

/// Sim sources for one scope, as `(label, text)`. Test files and non-sim subpaths
/// excluded. Asserts each declared root of this scope contributes files — a scan
/// that reads nothing under a root passes vacuously (the audit N0.3 bug).
fn sim_sources(ws: &Workspace, cfg: &Config, scope: Scope) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for root in cfg.root.iter().filter(|r| r.scope == scope) {
        let src = ws.abs(&root.path).join("src");
        let mut contributed = 0usize;
        for path in workspace::rust_sources_under(&src) {
            let rel = path
                .strip_prefix(&src)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            if workspace::is_test_path(&rel) {
                continue;
            }
            if cfg.non_sim_subpaths.iter().any(|skip| rel.contains(skip)) {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            out.push((format!("{}/src/{rel}", root.path), text));
            contributed += 1;
        }
        assert!(
            contributed > 0,
            "determinism scan reached no sources under `{}` ({} scope) — the root is \
             wrong and the lint would pass vacuously",
            root.path,
            scope.label()
        );
    }
    out
}

fn diag(cfg: &Config, scope: Scope, location: String, detail: String) -> Diagnostic {
    Diagnostic {
        policy_id: format!("{}.determinism", scope.label()),
        owners: vec![],
        source_doc: cfg.source_doc.clone(),
        rationale: cfg.rationale.clone(),
        location,
        detail,
    }
}

/// The whole determinism suite for one scope.
pub fn run(ws: &Workspace, scope: Scope, report: &mut Report) {
    let cfg = load_config();
    let roots: Vec<&Root> = cfg.root.iter().filter(|r| r.scope == scope).collect();
    assert!(
        !roots.is_empty(),
        "no determinism roots for the {} scope",
        scope.label()
    );

    // Rule 1a — no ambient RNG dependency. Manifest paths come straight from the
    // workspace-relative root; NEVER reconstructed by prepending another `crates/`.
    let mut manifests_read = 0usize;
    for root in &roots {
        let manifest = ws.abs(&root.path).join("Cargo.toml");
        let text = std::fs::read_to_string(&manifest).unwrap_or_else(|e| {
            panic!(
                "sim-crate manifest `{}` is unreadable ({e}); a manifest scan that silently \
                 skips passes vacuously (audit N0.3)",
                manifest.display()
            )
        });
        manifests_read += 1;
        for (rel, dep) in rng_dep_hits(&cfg, &text) {
            report.push(diag(
                &cfg,
                scope,
                format!("{}/Cargo.toml:{rel}", root.path),
                format!("depends on ambient RNG `{dep}` — sim randomness must be a seeded, snapshot-registered resource"),
            ));
        }
    }
    assert_eq!(
        manifests_read,
        roots.len(),
        "read {manifests_read} of {} sim manifests — a scan that misses manifests passes vacuously",
        roots.len()
    );

    let sources = sim_sources(ws, &cfg, scope);
    for (file, line, detail) in check_global_rng(&cfg, &sources) {
        report.push(diag(&cfg, scope, format!("{file}:{line}"), detail));
    }
    for (file, line, detail) in check_wall_clock(&cfg, &sources) {
        report.push(diag(&cfg, scope, format!("{file}:{line}"), detail));
    }
    for (file, line, detail) in check_std_hash_iteration(&cfg, &sources) {
        report.push(diag(&cfg, scope, format!("{file}:{line}"), detail));
    }
    for (file, line, detail) in check_entity_sort(&cfg, &sources) {
        report.push(diag(&cfg, scope, format!("{file}:{line}"), detail));
    }
}

// ── Rule 1 — ambient randomness ──────────────────────────────────────────────

/// `(line, dep)` for each ambient-RNG real dependency in a manifest. Dev-deps are
/// fine (a fuzzer proves determinism), so only `[dependencies]` / `.dependencies`.
fn rng_dep_hits(cfg: &Config, manifest: &str) -> Vec<(usize, String)> {
    let mut hits = Vec::new();
    let mut in_deps = false;
    for (i, line) in manifest.lines().enumerate() {
        let t = line.trim();
        if t.starts_with('[') {
            in_deps = t == "[dependencies]" || t.ends_with(".dependencies]");
            continue;
        }
        if !in_deps || t.starts_with('#') {
            continue;
        }
        for rng in &cfg.rng_crates {
            if t.starts_with(&format!("{rng} ")) || t.starts_with(&format!("{rng}=")) {
                hits.push((i + 1, rng.clone()));
            }
        }
    }
    hits
}

fn check_global_rng(cfg: &Config, sources: &[(String, String)]) -> Vec<(String, usize, String)> {
    let mut out = Vec::new();
    for (file, text) in sources {
        let lines: Vec<&str> = text.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            if is_comment(line) || is_reviewed(&cfg.review_marker, &lines, i) {
                continue;
            }
            for banned in &cfg.banned_rng_call {
                if line.contains(&banned.needle) {
                    out.push((
                        file.clone(),
                        i + 1,
                        format!(
                            "`{}` — {} (use a seeded RNG resource)",
                            banned.needle, banned.why
                        ),
                    ));
                }
            }
        }
    }
    out
}

// ── Rule 2 — wall-clock reads ────────────────────────────────────────────────

fn check_wall_clock(cfg: &Config, sources: &[(String, String)]) -> Vec<(String, usize, String)> {
    let mut out = Vec::new();
    for (file, text) in sources {
        let lines: Vec<&str> = text.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            if is_comment(line) || is_reviewed(&cfg.review_marker, &lines, i) {
                continue;
            }
            for needle in &cfg.wall_clock {
                if line.contains(needle) {
                    out.push((
                        file.clone(),
                        i + 1,
                        format!(
                            "`{needle}` — the sim advances on WorldTime/SimTick, not std::time"
                        ),
                    ));
                }
            }
        }
    }
    out
}

// ── Rule 3 — no std-hash-order semantics ─────────────────────────────────────

/// Type names that mean `std`'s hash containers **in this file**: always the FQ
/// paths, plus the bare `HashMap`/`HashSet` when imported from `std::collections`
/// and the file does not also import Bevy's same-named types.
pub(crate) fn std_hash_type_names(text: &str) -> Vec<&'static str> {
    let out = vec!["std::collections::HashMap", "std::collections::HashSet"];
    let imports_bevy = text.contains("platform::collections::HashMap")
        || text.contains("platform::collections::HashSet");
    if imports_bevy {
        return out;
    }
    let mut out = out;
    for bare in ["HashMap", "HashSet"] {
        let imported = text.lines().any(|l| {
            let l = l.trim();
            l.starts_with("use std::collections::")
                && (l.contains(&format!("::{bare};"))
                    || l.contains(&format!("{bare},"))
                    || l.contains(&format!("{bare}}}"))
                    || l.contains(&format!("{bare} as")))
        });
        if imported {
            out.push(bare);
        }
    }
    out
}

/// Names bound in this file to a `std` hash container (Bevy's `FixedHasher` maps
/// are deterministic same-build, so they are legal and NOT tracked).
pub(crate) fn std_hash_bindings(text: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let type_names = std_hash_type_names(text);
    for raw in text.lines() {
        let line = raw.trim();
        if is_comment(raw) {
            continue;
        }
        let mentions_std_hash = type_names.iter().any(|ty| {
            if ty.starts_with("std::") {
                line.contains(ty)
            } else {
                line.contains(&format!("{ty}<"))
            }
        });
        if !mentions_std_hash {
            continue;
        }
        let after_let = line
            .strip_prefix("let mut ")
            .or_else(|| line.strip_prefix("let "));
        let candidate = match after_let {
            Some(rest) => rest,
            None => line.strip_prefix("pub ").unwrap_or(line),
        };
        let ident: String = candidate
            .chars()
            .take_while(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '_')
            .collect();
        let rest = &candidate[ident.len()..];
        if !ident.is_empty()
            && (rest.starts_with(':') || rest.starts_with(" =") || rest.starts_with('='))
        {
            names.insert(ident);
        }
    }
    names
}

fn check_std_hash_iteration(
    cfg: &Config,
    sources: &[(String, String)],
) -> Vec<(String, usize, String)> {
    const ITER_METHODS: &[&str] = &[
        ".iter()",
        ".iter_mut()",
        ".values()",
        ".values_mut()",
        ".keys()",
        ".into_iter()",
        ".drain()",
    ];
    let mut out = Vec::new();
    for (file, text) in sources {
        let names = std_hash_bindings(text);
        if names.is_empty() {
            continue;
        }
        let lines: Vec<&str> = text.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            if is_comment(line) || is_reviewed(&cfg.review_marker, &lines, i) {
                continue;
            }
            for name in &names {
                let iterated = ITER_METHODS
                    .iter()
                    .any(|m| line.contains(&format!("{name}{m}")))
                    || line.contains(&format!("in {name} "))
                    || line.contains(&format!("in &{name}"))
                    || line.trim_end().ends_with(&format!("in {name} {{"));
                if iterated {
                    out.push((
                        file.clone(),
                        i + 1,
                        format!(
                            "iterates `{name}`, a std hash container — RandomState order differs \
                             between runs; use BTreeMap/BTreeSet or bevy::platform::collections"
                        ),
                    ));
                }
            }
        }
    }
    out
}

// ── Rule 4 — Entity is never an ordering key ─────────────────────────────────

fn check_entity_sort(cfg: &Config, sources: &[(String, String)]) -> Vec<(String, usize, String)> {
    let mut out = Vec::new();
    for (file, text) in sources {
        let lines: Vec<&str> = text.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            if is_comment(line) || is_reviewed(&cfg.review_marker, &lines, i) {
                continue;
            }
            let sorts = line.contains("sort_by_key")
                || line.contains("sort_unstable_by_key")
                || line.contains("sort_by(")
                || line.contains("sort_unstable_by(");
            if !sorts {
                continue;
            }
            let leaks =
                line.contains(".index()") || line.contains(".to_bits()") || line.contains("Entity");
            if leaks {
                out.push((
                    file.clone(),
                    i + 1,
                    "sorts by an Entity id (allocation detail) — order by a stable authored/spawn id"
                        .to_string(),
                ));
            }
        }
    }
    out
}

// ── poison / self-tests (called from tests/policy.rs) ────────────────────────

/// Every rule detects its violation, and rule 3 discriminates bare-std from Bevy.
pub fn poison_self_tests() {
    let cfg = load_config();

    // Rule 1a — RNG dependency.
    let manifest = "[dependencies]\nrand = \"0.8\"\nserde = \"1\"\n";
    assert!(
        !rng_dep_hits(&cfg, manifest).is_empty(),
        "RNG-dep check must flag a `rand` dependency"
    );
    // ...but a dev-dependency is fine.
    let dev = "[dev-dependencies]\nrand = \"0.8\"\n";
    assert!(
        rng_dep_hits(&cfg, dev).is_empty(),
        "a `rand` dev-dependency is legal (fuzzers prove determinism)"
    );

    // Rule 1b — global RNG call.
    let src = vec![(
        "x.rs".to_string(),
        "fn f() { let n = rand::random::<u8>(); let _ = thread_rng(); }".to_string(),
    )];
    assert!(
        check_global_rng(&cfg, &src).len() >= 2,
        "global-RNG check must flag rand::random and thread_rng"
    );

    // Rule 2 — wall-clock.
    let src = vec![(
        "x.rs".to_string(),
        "fn f() { let t = Instant::now(); }".to_string(),
    )];
    assert!(
        !check_wall_clock(&cfg, &src).is_empty(),
        "wall-clock check must flag Instant::now"
    );

    // Rule 3 — std-hash iteration detected; Bevy hash NOT a false positive. The
    // field is on its own line, exactly as real code writes it (this is the shape
    // `WorldMemory` used to hide a real hash-order bug behind).
    let std_hash = vec![(
        "x.rs".to_string(),
        "use std::collections::HashMap;\nstruct S {\n    actors: HashMap<String, u8>,\n}\nfn f(s: &S) {\n    for a in s.actors.values() {}\n}".to_string(),
    )];
    assert!(
        !check_std_hash_iteration(&cfg, &std_hash).is_empty(),
        "std-hash iteration must be detected on the bare imported spelling"
    );
    assert!(
        std_hash_bindings("use std::collections::HashMap;\nlet actors: HashMap<u8, u8> = q();")
            .contains("actors"),
        "bare imported HashMap binding is tracked"
    );
    assert!(
        !std_hash_bindings(
            "use bevy::platform::collections::HashMap;\nlet actors: HashMap<u8, u8> = q();"
        )
        .contains("actors"),
        "Bevy's FixedHasher HashMap must NOT be a false positive"
    );
    assert!(
        std_hash_bindings("use std::collections::HashMap;\nlet hashmap_like: Vec<u8> = v();")
            .is_empty(),
        "a lookalike identifier is not a std hash binding"
    );

    // Rule 4 — entity-order sort.
    let src = vec![(
        "x.rs".to_string(),
        "fn f(v: &mut Vec<Entity>) { v.sort_by_key(|e| e.index()); }".to_string(),
    )];
    assert!(
        !check_entity_sort(&cfg, &src).is_empty(),
        "entity-order sort must be detected"
    );

    // The reviewed marker suppresses.
    let reviewed = vec![(
        "x.rs".to_string(),
        format!(
            "// {}: explained\nfn f() {{ let _ = thread_rng(); }}",
            cfg.review_marker
        ),
    )];
    assert!(
        check_global_rng(&cfg, &reviewed).is_empty(),
        "the review marker must suppress a violation"
    );
}

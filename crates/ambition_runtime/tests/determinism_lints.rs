//! **The determinism lint set** (netcode N0.3) — the standing enforcement of the
//! level-2 contract Jon ruled in `docs/planning/engine/netcode.md` Q4:
//!
//! > the SAME binary, on the SAME platform, fed the SAME per-tick input stream,
//! > produces identical sim states.
//!
//! Cross-platform bit-exactness (level 3) is explicitly not promised — but these
//! rules are chosen so it stays reachable without a rewrite.
//!
//! Four properties were measured accidentally-true on 2026-07-09. This file is
//! what makes them STAY true. Each lint is a clippy-style grep with an explicit,
//! justified allowlist; a violation names the file, the line, and the fix.
//!
//! **The rules** (see `docs/adr/0023-same-build-determinism.md`):
//!
//! 1. *No ambient randomness.* Sim randomness is a seeded, snapshot-registered
//!    resource. A global/thread RNG is reproducible for nobody.
//! 2. *No wall-clock reads.* The sim advances on `WorldTime` / `SimTick`. An
//!    `Instant::now()` in a sim system makes the trajectory depend on how fast
//!    the machine ran.
//! 3. *No hash-order semantics.* Iterating a `std` hash container leaks
//!    `RandomState`'s per-PROCESS seed into sim order — two runs of one binary
//!    diverge. Iterate a `BTreeMap`/`BTreeSet`, or a Bevy
//!    `bevy::platform::collections` map (`FixedHasher`), or don't iterate.
//! 4. *`Entity` is never an ordering key.* Entity ids are allocation details, not
//!    identity. Order by a stable authored/spawn id or a slot.
//!
//! Escape hatch: put `AMBITION_REVIEW(determinism)` on the offending line or the
//! line above it, with a comment explaining why the order cannot be observed.
//! That marker is grep-able, so an auditor can review every exception at once.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// The crates whose sources ARE the simulation. Presentation, input-device,
/// audio, render, and menu crates are excluded by construction — they run on the
/// feel clock and their order is never part of a state hash.
const SIM_CRATES: &[&str] = &[
    "ambition_engine_core",
    "ambition_platformer_primitives",
    "ambition_time",
    "ambition_entity_catalog",
    "ambition_world",
    "ambition_characters",
    "ambition_combat",
    "ambition_projectiles",
    "ambition_portal",
    "ambition_encounter",
    "ambition_items",
    "ambition_cutscene",
    "ambition_interaction",
    "ambition_sim_view",
    "ambition_actors",
    "ambition_runtime",
];

/// Paths inside a sim crate that are NOT sim. `ambition_actors` in particular
/// still carries menu UI, dev tooling, audio, and asset loading (the residual
/// adapter shells the decomposition left behind — see tracks.md drift findings).
const NON_SIM_SUBPATHS: &[&str] = &[
    "menu/",       // map/pause UI — presentation
    "dev/",        // trace dump, debug overlays — wall-clock by design
    "audio/",      // cue playback — feel clock
    "assets/",     // asset loading
    "persistence", // save I/O
    "ldtk_world/hot_reload.rs",
];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("repo root")
}

/// Every non-test `.rs` file in the sim crates, as `(crate-relative label, text)`.
fn sim_sources() -> Vec<(String, String)> {
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
    for krate in SIM_CRATES {
        let src = root.join("crates").join(krate).join("src");
        let mut files = Vec::new();
        walk(&src, &mut files);
        for path in files {
            let rel = path
                .strip_prefix(&src)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            // Tests legitimately name every forbidden construct — including the
            // allowlist literals in this very file's siblings.
            if rel.ends_with("tests.rs") || rel.contains("/tests/") || rel.starts_with("tests/") {
                continue;
            }
            if NON_SIM_SUBPATHS.iter().any(|skip| rel.contains(skip)) {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            out.push((format!("{krate}/src/{rel}"), text));
        }
    }
    assert!(
        out.len() > 200,
        "the sim-source walk found only {} files — the crate paths are probably wrong, \
         and a lint that scans nothing passes vacuously",
        out.len()
    );
    out
}

fn is_comment(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("//") || t.starts_with("*") || t.starts_with("#![") || t.starts_with("#[")
}

/// Lines that opt out: the marker is on the line itself, or anywhere in the
/// contiguous comment block directly above it. The marker heads an explanation,
/// and an explanation worth reading is usually more than one line.
fn is_reviewed(lines: &[&str], idx: usize) -> bool {
    const MARKER: &str = "AMBITION_REVIEW(determinism)";
    if lines[idx].contains(MARKER) {
        return true;
    }
    for line in lines[..idx].iter().rev() {
        if !is_comment(line) {
            return false;
        }
        if line.contains(MARKER) {
            return true;
        }
    }
    false
}

fn report(lint: &str, rule: &str, violations: Vec<String>) {
    assert!(
        violations.is_empty(),
        "\n{lint} — {} violation(s).\n\n{rule}\n\nViolations:\n{}\n\n\
         If an occurrence genuinely cannot affect sim order, mark it \
         `AMBITION_REVIEW(determinism)` on the line (or the line above) with a \
         comment saying why.\n",
        violations.len(),
        violations.join("\n"),
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Rule 1 — no ambient randomness.
// ─────────────────────────────────────────────────────────────────────────────

const RNG_CRATES: &[&str] = &["rand", "fastrand", "oorandom", "nanorand", "getrandom"];

#[test]
fn sim_crates_pull_in_no_ambient_rng() {
    let root = repo_root();
    let mut violations = Vec::new();
    for krate in SIM_CRATES {
        let manifest = root.join("crates").join(krate).join("Cargo.toml");
        let Ok(text) = std::fs::read_to_string(&manifest) else {
            continue;
        };
        // Real dependencies only. A `rand` dev-dependency is fine: tests are not
        // the simulation, and a fuzzer that generates inputs is exactly the tool
        // that PROVES determinism rather than breaking it.
        let mut in_deps = false;
        for (i, line) in text.lines().enumerate() {
            let t = line.trim();
            if t.starts_with('[') {
                in_deps = t == "[dependencies]" || t.ends_with(".dependencies]");
                continue;
            }
            if !in_deps || t.starts_with('#') {
                continue;
            }
            for rng in RNG_CRATES {
                if t.starts_with(&format!("{rng} ")) || t.starts_with(&format!("{rng}=")) {
                    violations.push(format!("{krate}/Cargo.toml:{}: depends on `{rng}`", i + 1));
                }
            }
        }
    }
    report(
        "N0.3 rule 1 (no ambient randomness)",
        "Sim randomness must be a SEEDED, snapshot-registered resource (netcode N3.1): \
         a per-owner or per-tick seeded stream, never a global or thread RNG. An \
         unregistered RNG is a determinism bug the N0.4 desync canary will catch — \
         after it has already cost you a debugging session.",
        violations,
    );
}

#[test]
fn sim_sources_call_no_global_rng() {
    const BANNED: &[(&str, &str)] = &[
        ("thread_rng", "thread-local RNG, seeded from the OS"),
        ("rand::random", "global RNG"),
        ("fastrand::", "global RNG"),
        ("getrandom", "OS entropy"),
    ];
    let mut violations = Vec::new();
    for (file, text) in sim_sources() {
        let lines: Vec<&str> = text.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            if is_comment(line) || is_reviewed(&lines, i) {
                continue;
            }
            for (needle, why) in BANNED {
                if line.contains(needle) {
                    violations.push(format!("{file}:{}: `{needle}` — {why}", i + 1));
                }
            }
        }
    }
    report(
        "N0.3 rule 1 (no ambient randomness)",
        "Use a seeded RNG resource. A seed is reproducible today and portable to \
         cross-platform determinism (level 3) later.",
        violations,
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Rule 2 — no wall-clock reads in the sim.
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn sim_sources_read_no_wall_clock() {
    const BANNED: &[&str] = &["Instant::now", "SystemTime::now", "UNIX_EPOCH"];
    let mut violations = Vec::new();
    for (file, text) in sim_sources() {
        let lines: Vec<&str> = text.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            if is_comment(line) || is_reviewed(&lines, i) {
                continue;
            }
            for needle in BANNED {
                if line.contains(needle) {
                    violations.push(format!("{file}:{}: `{needle}`", i + 1));
                }
            }
        }
    }
    report(
        "N0.3 rule 2 (no wall-clock reads in the sim)",
        "The sim advances on `WorldTime` (ADR 0010/0011) and is indexed by `SimTick`. \
         A wall-clock read makes the trajectory depend on how fast the machine ran, \
         which is the opposite of a replay. Note that under fixed tick, `Res<Time>` \
         inside the tick IS the fixed clock, so it is deterministic — this rule is \
         about `std::time`.",
        violations,
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Rule 3 — no hash-order semantics.
// ─────────────────────────────────────────────────────────────────────────────

/// Names bound in this file to a `std` hash container. Bevy's
/// `bevy::platform::collections` maps use `FixedHasher`, whose iteration order is
/// a deterministic function of the insertion sequence on a fixed binary — legal
/// at level 2, and the reason this lint discriminates by hasher, not by shape.
fn std_hash_bindings(text: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for raw in text.lines() {
        let line = raw.trim();
        if is_comment(raw) {
            continue;
        }
        // `let [mut] name ... std::collections::HashMap/HashSet ...`
        // `[pub] name: ... std::collections::HashMap/HashSet ...`  (struct field)
        if !(line.contains("std::collections::HashMap")
            || line.contains("std::collections::HashSet"))
        {
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
        // A binding, not a type position: the identifier is followed by `:` or ` =`.
        if !ident.is_empty()
            && (rest.starts_with(':') || rest.starts_with(" =") || rest.starts_with('='))
        {
            names.insert(ident);
        }
    }
    names
}

#[test]
fn sim_sources_never_iterate_a_std_hash_container() {
    const ITER_METHODS: &[&str] = &[
        ".iter()",
        ".iter_mut()",
        ".values()",
        ".values_mut()",
        ".keys()",
        ".into_iter()",
        ".drain()",
    ];
    let mut violations = Vec::new();
    for (file, text) in sim_sources() {
        let names = std_hash_bindings(&text);
        if names.is_empty() {
            continue;
        }
        let lines: Vec<&str> = text.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            if is_comment(line) || is_reviewed(&lines, i) {
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
                    violations.push(format!(
                        "{file}:{}: iterates `{name}`, a std hash container — {}",
                        i + 1,
                        line.trim()
                    ));
                }
            }
        }
    }
    report(
        "N0.3 rule 3 (no hash-order semantics)",
        "`std::collections::HashMap`/`HashSet` use `RandomState`, seeded per PROCESS: \
         iteration order differs between two runs of the SAME binary on the SAME \
         inputs. If the order is observable — spawn order, message order, who acts \
         first — the sim is not replayable. Use a `BTreeMap`/`BTreeSet` (sorted, and \
         portable to level 3), or `bevy::platform::collections` (`FixedHasher`, \
         deterministic same-build), or keep the hash set as a membership filter and \
         iterate the source sequence instead.",
        violations,
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Rule 4 — `Entity` is never an ordering key.
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn sim_sources_never_sort_by_entity() {
    let mut violations = Vec::new();
    for (file, text) in sim_sources() {
        let lines: Vec<&str> = text.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            if is_comment(line) || is_reviewed(&lines, i) {
                continue;
            }
            let sorts = line.contains("sort_by_key")
                || line.contains("sort_unstable_by_key")
                || line.contains("sort_by(")
                || line.contains("sort_unstable_by(");
            if !sorts {
                continue;
            }
            // `Entity::index()` / `.to_bits()` are the two ways an entity id leaks
            // into an ordering. Naming the type in the closure is the third.
            let leaks =
                line.contains(".index()") || line.contains(".to_bits()") || line.contains("Entity");
            if leaks {
                violations.push(format!("{file}:{}: {}", i + 1, line.trim()));
            }
        }
    }
    report(
        "N0.3 rule 4 (`Entity` is never an ordering key)",
        "Bevy entity ids are allocation details — index + generation, reused from a \
         free list. Sorting by one makes sim order depend on spawn/despawn history \
         rather than on the world. Order by a STABLE id (`ActorConfig.id` / LDtk iid, \
         a `PlayerSlot`, a spawn sequence number) — the same identity vocabulary \
         `SimSnapshot` (N3.1) and rollback both need.",
        violations,
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// The escape hatch is itself auditable.
// ─────────────────────────────────────────────────────────────────────────────

/// Not a lint — an inventory. Prints every reviewed exception so an auditor can
/// read the whole set of "this order cannot be observed" claims in one place.
#[test]
fn reviewed_determinism_exceptions_are_listed() {
    let mut found = Vec::new();
    for (file, text) in sim_sources() {
        for (i, line) in text.lines().enumerate() {
            if line.contains("AMBITION_REVIEW(determinism)") {
                found.push(format!("  {file}:{}: {}", i + 1, line.trim()));
            }
        }
    }
    println!(
        "AMBITION_REVIEW(determinism) exceptions ({}):\n{}",
        found.len(),
        found.join("\n")
    );
}

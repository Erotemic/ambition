//! The ONE workspace-policy test binary — many independently named + filterable
//! `#[test]`s, one compiled binary.
//!
//! The three scope runners aggregate every violation of a scope at once:
//!   cargo test -p ambition_workspace_policy repository_policies
//!   cargo test -p ambition_workspace_policy engine_policies
//!   cargo test -p ambition_workspace_policy game_policies
//!
//! The rest are the runner's own self-tests, each its own `#[test]` so a first
//! panic no longer hides the others: workspace discovery, uniform owner/watch-path
//! validation across declarative AND custom policies, non-vacuous source roots,
//! custom-metadata completeness, the migration-matrix honesty checks, and a poison
//! fixture per rule kind + custom scanner proving each still reacts.

use ambition_workspace_policy::{
    custom, run_declarative, workspace, Policy, Report, Scope, Workspace, WORKSPACE_OWNER,
};

// ── the three scope tests ────────────────────────────────────────────────────

#[test]
fn repository_policies() {
    let ws = Workspace::discover();
    let mut report = Report::new(Scope::Repository);
    run_declarative(&ws, Scope::Repository, &mut report);
    report.assert_ok();
}

#[test]
fn engine_policies() {
    let ws = Workspace::discover();
    let mut report = Report::new(Scope::Engine);
    run_declarative(&ws, Scope::Engine, &mut report);
    // Custom engine scanners share the compiled runner and the same Report.
    custom::module_size::run(&ws, &mut report);
    custom::determinism::run(&ws, Scope::Engine, &mut report);
    custom::control_frame::run(&ws, Scope::Engine, &mut report);
    custom::lifecycle::run(&ws, &mut report);
    custom::content_ownership::run(&ws, &mut report);
    report.assert_ok();
}

#[test]
fn game_policies() {
    let ws = Workspace::discover();
    let mut report = Report::new(Scope::Game);
    run_declarative(&ws, Scope::Game, &mut report);
    // The determinism + control-frame scanners' game-scope roots (content + demo
    // rules) run here, independently of the engine scope.
    custom::determinism::run(&ws, Scope::Game, &mut report);
    custom::control_frame::run(&ws, Scope::Game, &mut report);
    report.assert_ok();
}

// ── the runner's own self-tests + poison ─────────────────────────────────────

const POISON_MANIFEST: &str = "tests/ambition_workspace_policy/fixtures/poison/manifest.toml";
const POISON_SRC: &str = "tests/ambition_workspace_policy/fixtures/poison/src";

/// Parse a poison policy from inline TOML (also exercises the policy parser).
fn poison(toml_str: &str) -> Policy {
    toml::from_str(toml_str).expect("parse poison policy")
}

/// Run one policy and return its report — the harness for every poison check.
fn run_one(policy: &Policy) -> Report {
    let ws = Workspace::discover();
    let mut report = Report::new(policy.scope);
    ambition_workspace_policy::runner::dispatch(&ws, policy, &mut report);
    report
}

// The runner's self-tests are individual `#[test]`s (one compiled binary, but each
// check is separately named + filterable, and a first panic no longer hides the
// rest). The three scope runners above stay aggregating — their `Report`
// intentionally emits ALL violations of a scope at once.

// ── custom scanners' own poison ──────────────────────────────────────────────

#[test]
fn module_size_gate_reacts() {
    custom::module_size::poison_reacts(&Workspace::discover());
}

#[test]
fn determinism_scanners_react() {
    custom::determinism::poison_self_tests();
}

#[test]
fn control_frame_scanner_reacts() {
    custom::control_frame::poison_self_tests();
}

#[test]
fn control_frame_allowlist_is_justified() {
    custom::control_frame::allowlist_is_justified();
}

#[test]
fn raw_spawn_gate_reacts() {
    custom::lifecycle::poison_self_tests();
}

#[test]
fn enemy_config_gate_reacts() {
    custom::content_ownership::poison_self_tests();
}

// ── the architecture-migration matrix is complete and honest ─────────────────

#[test]
fn migration_matrix_is_complete() {
    custom::migration_matrix::check(&Workspace::discover());
}

#[test]
fn legacy_file_is_fully_tracked() {
    custom::migration_matrix::legacy_file_is_fully_tracked(&Workspace::discover());
}

/// Mirrors the retired `legacy_runtime_guardrail.rs` scanner self-test: every
/// banned identifier is caught, and the `ALLOW_LEGACY_RUNTIME` line is suppressed.
#[test]
fn legacy_scanner_catches_each_forbidden_identifier() {
    const FORBIDDEN: &[&str] = &[
        "SandboxRuntime",
        "FeatureRuntime",
        "feature_runtime_phase",
        "runtime.player",
        "ae::Player::new",
        "BodyClustersMut::to_player",
        "::from_player(",
        "update_player_with_tuning(",
        "update_player_control_with_tuning(",
        "update_player_simulation_with_tuning(",
    ];
    let forbid_toml = FORBIDDEN
        .iter()
        .map(|f| format!("{f:?}"))
        .collect::<Vec<_>>()
        .join(", ");
    let p = poison(&format!(
        r#"
        id = "poison.legacy-runtime"
        scope = "game"
        kind = "forbidden-source-reference"
        rationale = "poison"
        roots = ["tests/ambition_workspace_policy/fixtures/poison/legacy"]
        allow_marker = "ALLOW_LEGACY_RUNTIME"
        production_only = true
        forbid = [{forbid_toml}]
    "#
    ));
    let report = run_one(&p);
    for needle in FORBIDDEN {
        assert!(
            report
                .diagnostics()
                .iter()
                .any(|d| d.detail.contains(needle)),
            "legacy scanner dropped `{needle}`"
        );
    }
    // The ALLOW_LEGACY_RUNTIME line must not contribute a diagnostic.
    assert!(
        report
            .diagnostics()
            .iter()
            .all(|d| !d.detail.contains("ALLOW_LEGACY_RUNTIME")),
        "the ALLOW_LEGACY_RUNTIME line was not suppressed"
    );
}

#[test]
fn workspace_discovery_finds_the_root() {
    let ws = Workspace::discover();
    assert!(
        ws.root().join("Cargo.toml").is_file(),
        "workspace root has no Cargo.toml"
    );
    let members = ws.member_dirs();
    assert!(
        members.len() > 30,
        "only {} workspace members parsed — the [workspace] members walk is broken",
        members.len()
    );
    assert!(
        ws.member_names().contains("ambition_workspace_policy"),
        "the policy package must be a registered member (it self-checks in repository.toml too)"
    );
}

/// Owners of BOTH declarative and custom policies must be a real workspace
/// package (or the `workspace` cross-cutting sentinel). One uniform check so a
/// future `xtask test-affected` can trust every policy's ownership.
#[test]
fn every_declared_owner_is_a_real_workspace_package() {
    let ws = Workspace::discover();
    let members = ws.member_names();
    let valid = |o: &str| o == WORKSPACE_OWNER || members.contains(o);
    for policy in workspace::load_all_policies() {
        for owner in &policy.owners {
            assert!(
                valid(owner),
                "policy `{}` names owner `{owner}`, not a workspace package or `{WORKSPACE_OWNER}`",
                policy.id
            );
        }
    }
    for meta in custom::metas() {
        assert!(
            !meta.owners.is_empty(),
            "custom policy `{}` declares no owners",
            meta.id
        );
        for owner in &meta.owners {
            assert!(
                valid(owner),
                "custom policy `{}` names owner `{owner}`, not a workspace package or `{WORKSPACE_OWNER}`",
                meta.id
            );
        }
    }
}

/// Watch paths of BOTH declarative and custom policies must exist.
#[test]
fn every_watch_path_exists() {
    let ws = Workspace::discover();
    for policy in workspace::load_all_policies() {
        for wp in &policy.watch_paths {
            assert!(
                ws.abs(wp).exists(),
                "policy `{}` watches `{wp}`, which does not exist",
                policy.id
            );
        }
    }
    for meta in custom::metas() {
        assert!(
            !meta.watch_paths.is_empty(),
            "custom policy `{}` declares no watch paths",
            meta.id
        );
        for wp in &meta.watch_paths {
            assert!(
                ws.abs(wp).exists(),
                "custom policy `{}` watches `{wp}`, which does not exist",
                meta.id
            );
        }
    }
}

/// Every custom scanner exposes uniform metadata: a stable id, a source doc, and
/// (validated above) owners + watch paths. This is the surface a future
/// `xtask test-affected` selects on, so it must be complete.
#[test]
fn custom_policy_metadata_is_complete() {
    let mut ids = std::collections::BTreeSet::new();
    for meta in custom::metas() {
        assert!(
            !meta.id.is_empty() && meta.id.contains('.'),
            "custom policy id `{}` should be `<scope>.<name>`",
            meta.id
        );
        assert!(
            meta.source_doc.len() > 5,
            "custom policy `{}` has no source_doc",
            meta.id
        );
        assert_eq!(
            meta.id.split('.').next().unwrap(),
            meta.scope.label(),
            "custom policy `{}` id scope-prefix must match its scope",
            meta.id
        );
        assert!(
            ids.insert(meta.id.clone()),
            "duplicate custom id `{}`",
            meta.id
        );
    }
    assert!(
        ids.len() >= 6,
        "expected the 6+ custom policies, got {ids:?}"
    );
}

#[test]
fn every_source_root_contributes_files() {
    let ws = Workspace::discover();
    for policy in workspace::load_all_policies() {
        for root in &policy.roots {
            assert!(
                !ws.rust_sources(root).is_empty(),
                "policy `{}` scans root `{root}`, which contains no .rs files — vacuous",
                policy.id
            );
        }
    }
}

#[test]
fn poison_required_path_reacts() {
    let p = poison(
        r#"
        id = "poison.required-path"
        scope = "repository"
        kind = "required-path"
        rationale = "poison"
        paths = ["tests/ambition_workspace_policy/fixtures/poison/does_not_exist.rs"]
    "#,
    );
    assert!(
        !run_one(&p).is_empty(),
        "required-path must fail on a missing path"
    );
}

#[test]
fn poison_forbidden_path_reacts() {
    let p = poison(&format!(
        r#"
        id = "poison.forbidden-path"
        scope = "repository"
        kind = "forbidden-path"
        rationale = "poison"
        paths = ["{POISON_MANIFEST}"]
    "#
    ));
    assert!(
        !run_one(&p).is_empty(),
        "forbidden-path must fail on an existing path"
    );
}

#[test]
fn poison_workspace_member_reacts() {
    let p = poison(
        r#"
        id = "poison.workspace-member"
        scope = "repository"
        kind = "workspace-member"
        rationale = "poison"
        members = ["ambition_this_crate_does_not_exist"]
    "#,
    );
    assert!(
        !run_one(&p).is_empty(),
        "workspace-member must fail on a non-member"
    );
}

#[test]
fn poison_dependency_denylist_reacts() {
    let p = poison(&format!(
        r#"
        id = "poison.dependency-denylist"
        scope = "engine"
        kind = "dependency-denylist"
        rationale = "poison"
        manifest = "{POISON_MANIFEST}"
        deny = ["ambition_content", "bevy_ecs_ldtk"]
    "#
    ));
    let report = run_one(&p);
    assert!(
        report.len() >= 2,
        "denylist must catch both `ambition_content` and the non-ambition `bevy_ecs_ldtk`, got {}",
        report.len()
    );
}

#[test]
fn poison_dependency_allowlist_reacts() {
    let p = poison(&format!(
        r#"
        id = "poison.dependency-allowlist"
        scope = "engine"
        kind = "dependency-allowlist"
        rationale = "poison"
        manifest = "{POISON_MANIFEST}"
        allow = ["ambition_engine_core"]
    "#
    ));
    let report = run_one(&p);
    // ambition_content, ambition_render, ambition_app are all outside the allow.
    assert!(
        report.len() >= 3,
        "allowlist must catch every ambition dep outside `allow`, got {}",
        report.len()
    );
}

#[test]
fn poison_forbidden_source_reference_reacts() {
    let p = poison(&format!(
        r#"
        id = "poison.forbidden-source-reference"
        scope = "engine"
        kind = "forbidden-source-reference"
        rationale = "poison"
        roots = ["{POISON_SRC}"]
        forbid = ["ambition_content"]
    "#
    ));
    assert!(
        !run_one(&p).is_empty(),
        "forbidden-source-reference must catch the fixture's `use ambition_content::`"
    );
}

#[test]
fn poison_file_contains_reacts() {
    let p = poison(&format!(
        r#"
        id = "poison.file-contains"
        scope = "repository"
        kind = "file-contains"
        rationale = "poison"
        file = "{POISON_MANIFEST}"
        contains = ["this string is definitely not in the manifest"]
    "#
    ));
    assert!(
        !run_one(&p).is_empty(),
        "file-contains must fail when a required string is missing"
    );
}

#[test]
fn poison_file_omits_reacts() {
    let p = poison(&format!(
        r#"
        id = "poison.file-omits"
        scope = "repository"
        kind = "file-omits"
        rationale = "poison"
        file = "{POISON_MANIFEST}"
        forbid = ["ambition_content"]
    "#
    ));
    assert!(
        !run_one(&p).is_empty(),
        "file-omits must fail when a forbidden string is present"
    );
}

#[test]
fn comment_lines_are_exempt_from_source_scan() {
    // The fixture names `ambition_content` in TWO comment lines and ONE code
    // line. A correct scan reports exactly the code line — proving prose is
    // exempt (a broken scan would report 3).
    let p = poison(&format!(
        r#"
        id = "poison.comment-exempt"
        scope = "engine"
        kind = "forbidden-source-reference"
        rationale = "poison"
        roots = ["{POISON_SRC}"]
        forbid = ["ambition_content"]
    "#
    ));
    assert_eq!(
        run_one(&p).len(),
        1,
        "only the single code-line `use ambition_content::` should match; comments are exempt"
    );
}

#[test]
fn whole_ident_does_not_overmatch_but_substring_does() {
    // The fixture names `GroundItemVisual` but never a bare `GroundItem`.
    let whole = poison(&format!(
        r#"
        id = "poison.whole-ident"
        scope = "engine"
        kind = "forbidden-source-reference"
        rationale = "poison"
        roots = ["{POISON_SRC}"]
        forbid = ["GroundItem"]
        whole_ident = true
    "#
    ));
    assert!(
        run_one(&whole).is_empty(),
        "whole-ident `GroundItem` must NOT trip on `GroundItemVisual`"
    );

    let substr = poison(&format!(
        r#"
        id = "poison.substring"
        scope = "engine"
        kind = "forbidden-source-reference"
        rationale = "poison"
        roots = ["{POISON_SRC}"]
        forbid = ["GroundItem"]
    "#
    ));
    assert!(
        !run_one(&substr).is_empty(),
        "substring `GroundItem` MUST trip on `GroundItemVisual`"
    );
}

#[test]
fn allow_marker_suppresses_a_line() {
    // The fixture's `SandboxRuntime` line carries `ALLOW_LEGACY_RUNTIME`.
    let with_marker = poison(&format!(
        r#"
        id = "poison.allow-marker"
        scope = "engine"
        kind = "forbidden-source-reference"
        rationale = "poison"
        roots = ["{POISON_SRC}"]
        forbid = ["SandboxRuntime"]
        allow_marker = "ALLOW_LEGACY_RUNTIME"
    "#
    ));
    assert!(
        run_one(&with_marker).is_empty(),
        "the ALLOW_LEGACY_RUNTIME marker must exempt its line"
    );

    let without = poison(&format!(
        r#"
        id = "poison.no-marker"
        scope = "engine"
        kind = "forbidden-source-reference"
        rationale = "poison"
        roots = ["{POISON_SRC}"]
        forbid = ["SandboxRuntime"]
    "#
    ));
    assert!(
        !run_one(&without).is_empty(),
        "without the marker, the SandboxRuntime line must be caught"
    );
}

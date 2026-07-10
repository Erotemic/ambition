//! The ONE workspace-policy test binary.
//!
//! Four independently filterable tests:
//!   cargo test -p ambition_workspace_policy repository_policies
//!   cargo test -p ambition_workspace_policy engine_policies
//!   cargo test -p ambition_workspace_policy game_policies
//!   cargo test -p ambition_workspace_policy policy_runner_self_tests
//!
//! The scope tests run every declarative policy of that scope (plus, for
//! `engine`, the custom scanners once they migrate). `policy_runner_self_tests`
//! validates the runner itself: real owners, existing watch-paths, non-vacuous
//! source roots, and poison fixtures that prove every rule kind still reacts.

use ambition_workspace_policy::{
    custom, run_declarative, workspace, Policy, Report, Scope, Workspace,
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

#[test]
fn policy_runner_self_tests() {
    workspace_discovery_finds_the_root();
    every_declared_owner_is_a_real_workspace_package();
    every_watch_path_exists();
    every_source_root_contributes_files();

    // Poison: each rule kind must react to a deliberate violation.
    poison_required_path_reacts();
    poison_forbidden_path_reacts();
    poison_workspace_member_reacts();
    poison_dependency_denylist_reacts();
    poison_dependency_allowlist_reacts();
    poison_forbidden_source_reference_reacts();
    poison_file_contains_reacts();
    poison_file_omits_reacts();
    // …and its knobs behave.
    comment_lines_are_exempt_from_source_scan();
    whole_ident_does_not_overmatch_but_substring_does();
    allow_marker_suppresses_a_line();

    legacy_scanner_catches_each_forbidden_identifier();

    // Custom scanners' own poison.
    custom::module_size::poison_reacts(&Workspace::discover());
    custom::determinism::poison_self_tests();
    custom::control_frame::poison_self_tests();
    custom::control_frame::allowlist_is_justified();

    // The architecture-migration matrix is complete and honest.
    let ws = Workspace::discover();
    custom::migration_matrix::check(&ws);
    custom::migration_matrix::legacy_file_is_fully_tracked(&ws);
}

/// Mirrors the retired `legacy_runtime_guardrail.rs` scanner self-test: every
/// banned identifier is caught, and the `ALLOW_LEGACY_RUNTIME` line is suppressed.
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

fn every_declared_owner_is_a_real_workspace_package() {
    let ws = Workspace::discover();
    let members = ws.member_names();
    for policy in workspace::load_all_policies() {
        for owner in &policy.owners {
            assert!(
                members.contains(owner),
                "policy `{}` names owner `{owner}`, which is not a workspace package",
                policy.id
            );
        }
    }
}

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
}

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

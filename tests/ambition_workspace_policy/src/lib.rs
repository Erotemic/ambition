//! Workspace-policy runner.
//!
//! This crate is the ONE home for repository-structure tests. It reads the
//! workspace as data — parsed `Cargo.toml` manifests and a walk of the Rust
//! source tree — and checks declarative policies (loaded from `policies/*.toml`)
//! plus a handful of custom semantic scanners (`custom/`). It links no
//! production Ambition crate, so running the policy suite never compiles
//! `ambition_app`.
//!
//! Layout:
//!   * [`model`]     — what a policy IS ([`model::Policy`]) and what a failure
//!                     looks like ([`model::Diagnostic`] / [`model::Report`]).
//!   * [`workspace`] — repository-root discovery, workspace-member parsing,
//!                     manifest dependency parsing, and Rust source walking with
//!                     centralized exclusions/scanning primitives.
//!   * [`runner`]    — loads a `policies/*.toml` file and dispatches each policy
//!                     to its rule kind.
//!   * [`rules`]     — the declarative rule kinds (path/dependency/member/
//!                     source-reference). Repetitive, data-driven.
//!   * [`custom`]    — one-off semantic scanners kept as readable Rust
//!                     (determinism, control-frame, module-size, …), configured
//!                     by their own data files. Added as rules migrate.
//!
//! Everything is exercised by `tests/policy.rs`, whose four independently
//! filterable tests are `repository_policies`, `engine_policies`,
//! `game_policies`, and `policy_runner_self_tests`.

pub mod custom;
pub mod model;
pub mod rules;
pub mod runner;
pub mod workspace;

pub use model::{
    CustomMeta, Diagnostic, Policy, Report, RuleKind, Scope, Severity, WORKSPACE_OWNER,
};
pub use workspace::Workspace;

/// Load and run every declarative policy of `scope` from the standard
/// `policies/` directory, appending diagnostics to `report`. The custom
/// scanners are invoked separately by the scope test (they carry their own
/// config files), so a reader of `tests/policy.rs` sees the full membership of
/// each scope in one place.
pub fn run_declarative(ws: &Workspace, scope: Scope, report: &mut Report) {
    for policy in workspace::load_scope_policies(scope) {
        runner::dispatch(ws, &policy, report);
    }
}

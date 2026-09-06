//! Repository-structure policy runner.
//!
//! Policies inspect manifests and Rust source without linking production crates.
//! Declarative rules live in `policies/*.toml`; custom scanners cover structural
//! invariants that need semantic source inspection.

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

//! Custom semantic policy scanners.
//!
//! Some invariants are one-off semantic checks that would be unreadable as a
//! generic TOML rule (the determinism lints, the ControlFrame allowlist, the
//! module-size gate, the umbrella/plugin composition shapes). Those stay as
//! named Rust modules here, configured by their own data files under
//! `policies/`. Each is invoked directly from its scope test in
//! `tests/policy.rs`, so a reader sees the full membership of a scope in one
//! place.
//!
//! Populated as the custom scanners migrate (Tasks 5, 7, 8, 9).

pub mod content_ownership;
pub mod control_frame;
pub mod determinism;
pub mod lifecycle;
pub mod migration_matrix;
pub mod module_size;
pub mod session_world;

use crate::model::CustomMeta;

/// Uniform metadata for every custom-scanner policy, so the ownership/watch-path
/// self-test and a future `xtask test-affected` see custom and declarative
/// policies through one surface. (`migration_matrix` is a completeness self-check,
/// not a scanned guard, so it has no CustomMeta.)
pub fn metas() -> Vec<CustomMeta> {
    let mut out = Vec::new();
    out.extend(module_size::metas());
    out.extend(determinism::metas());
    out.extend(control_frame::metas());
    out.extend(lifecycle::metas());
    out.extend(content_ownership::metas());
    out.extend(session_world::metas());
    out
}

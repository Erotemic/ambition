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

pub mod determinism;
pub mod migration_matrix;
pub mod module_size;

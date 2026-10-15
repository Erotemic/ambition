//! `ambition_content` — validate a content pack without rebuilding the engine.
//!
//! The CLI is a diagnostic FRONT DOOR, not the enforcement mechanism. The same
//! `ambition_content_pack::compile` runs in the standard test, in CI, in
//! development reload and in packaging; this binary exists so an author gets
//! the answer in milliseconds instead of a ten-minute rebuild.

fn main() -> std::process::ExitCode {
    ambition_content_cli::main_with(Vec::new)
}

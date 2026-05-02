//! Ambition Tangent Space Sandbox visible binary.
//!
//! This is a thin shim around `ambition_sandbox::app::run_visible`. All the
//! gameplay systems, helpers, and App-builder logic moved to the library
//! crate (`crates/ambition_sandbox/src/app.rs`) in Slice 5 of ADR 0012's
//! events refactor so the headless binary can drive the same simulation
//! loop. See `docs/events_refactor_plan.md`.

fn main() {
    ambition_sandbox::app::run_visible();
}

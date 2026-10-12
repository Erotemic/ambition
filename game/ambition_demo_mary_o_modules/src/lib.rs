//! Super Mary-O's procedural extension modules: the brains of its enemies.
//!
//! A module here is linked into the Mary-O game (`StaticNative`) and is also
//! a loaded module file (`scripts/build_extension_modules.sh
//! ambition_demo_mary_o_modules`). A developer who changes an enemy builds
//! this crate alone, and the running game replaces the linked module with the
//! file. The Mary-O crate does not compile again.
//!
//! Each brain decides in `actor_think`, before the control gate, from the
//! body's facts and its target (`ambition.actor.think`).

pub mod laden_swallow;
pub mod void_lizard;

/// The provider namespace of every module and schema in this crate.
pub const PROVIDER: &str = "mary_o";

/// Every module this crate provides, as constructors.
pub const MODULES: &[fn() -> ambition_extension_sdk::ModuleDescriptor] = &[laden_swallow::module, void_lizard::module];

/// Every module this crate provides.
pub fn modules() -> Vec<ambition_extension_sdk::ModuleDescriptor> {
    MODULES.iter().map(|build| build()).collect()
}

// Built for `wasm32-unknown-unknown`, this crate is a loaded module file
// (`ambition-ext-1`).
ambition_extension_sdk::export_modules!(list: crate::MODULES);

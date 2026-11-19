//! Sanic's procedural extension modules: what a touch on his enemies does.
//!
//! A module here is linked into the Sanic game (`StaticNative`) and is also a
//! loaded module file (`scripts/regen/build_extension_modules.sh
//! ambition_demo_sanic_modules`). A developer who changes a rule builds this
//! crate alone, and the running game replaces the linked module with the
//! file. The Sanic crate does not compile again.
//!
//! A contact rule decides in `body_contact`, before the shared contact damage
//! (`ambition.body.contact`).

pub mod badnik;
pub mod rules;

/// The provider namespace of every module and schema in this crate.
pub const PROVIDER: &str = "sanic";

/// Every module this crate provides, as constructors.
pub const MODULES: &[fn() -> ambition_extension_sdk::ModuleDescriptor] = &[badnik::module];

/// Every module this crate provides.
pub fn modules() -> Vec<ambition_extension_sdk::ModuleDescriptor> {
    MODULES.iter().map(|build| build()).collect()
}

// Built for `wasm32-unknown-unknown`, this crate is a loaded module file
// (`ambition-ext-2`).
ambition_extension_sdk::export_modules!(list: crate::MODULES);

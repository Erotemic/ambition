//! The Smash demo's procedural extension modules: what its specials do.
//!
//! A module here is linked into the Smash game (`StaticNative`) and is also a
//! loaded module file (`scripts/build_extension_modules.sh
//! ambition_demo_smash_modules`). A developer who changes a special builds
//! this crate alone, and the running game replaces the linked module with the
//! file. The Smash crate does not compile again.
//!
//! Each special decides in `technique_execution`, when its fighter presses
//! it, from the fighter's facts and the params its move authors
//! (`ambition.fighter.special`). A special that leaves something on the stage
//! (a plate) asks the world for a module entity, and the module ticks it.

pub mod counter;
pub mod dilation;
pub mod homing;
pub mod limit;
pub mod riposte;
pub mod sing;
pub mod spring;

/// The provider namespace of every module in this crate.
pub const PROVIDER: &str = "smash";

/// Every module this crate provides, as constructors.
pub const MODULES: &[fn() -> ambition_extension_sdk::ModuleDescriptor] = &[
    counter::module,
    dilation::module,
    homing::module,
    limit::module,
    riposte::module,
    sing::module,
    spring::module,
];

/// Every module this crate provides.
pub fn modules() -> Vec<ambition_extension_sdk::ModuleDescriptor> {
    MODULES.iter().map(|build| build()).collect()
}

// Built for `wasm32-unknown-unknown`, this crate is a loaded module file
// (`ambition-ext-1`).
ambition_extension_sdk::export_modules!(list: crate::MODULES);

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
//! (`ambition.fighter.special`), or in `hit_reaction`, when a strike that
//! carries it lands (`ambition.combat.struck`). A special that leaves
//! something on the stage (a plate) asks the world for a module entity, and
//! the module ticks it; one that leaves something on a body (a slow, a mark)
//! holds a status there and keeps its state in records on the body.

pub mod bolt;
pub mod bomb;
pub mod counter;
pub mod dilation;
pub mod homing;
pub mod limit;
pub mod mark;
pub mod mine;
pub mod portal;
pub mod riposte;
pub mod shark;
pub mod sing;
pub mod spring;
pub mod tether;

/// The provider namespace of every module in this crate.
pub const PROVIDER: &str = "smash";

/// Every module this crate provides, as constructors.
pub const MODULES: &[fn() -> ambition_extension_sdk::ModuleDescriptor] = &[
    bolt::module,
    bomb::module,
    counter::module,
    dilation::module,
    homing::module,
    limit::module,
    mark::module,
    mine::module,
    portal::module,
    riposte::module,
    shark::module,
    sing::module,
    spring::module,
    tether::module,
];

/// Every module this crate provides.
pub fn modules() -> Vec<ambition_extension_sdk::ModuleDescriptor> {
    MODULES.iter().map(|build| build()).collect()
}

// Built for `wasm32-unknown-unknown`, this crate is a loaded module file
// (`ambition-ext-1`).
ambition_extension_sdk::export_modules!(list: crate::MODULES);

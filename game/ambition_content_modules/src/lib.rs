//! Ambition's procedural extension modules.
//!
//! Each module is an algorithm with its own state. It reads its trigger and
//! observations, changes its own records and submits typed requests. It
//! names no Bevy type and no engine implementation. The game composition
//! declares each module with `ExtensionAppExt::add_extension_module`.
//!
//! ⚠ These modules are linked into the host executable
//! (`CodeIdentity::StaticNative`). That is the native semantic reference of
//! fast-iteration I4, not the no-relink loop: a change here still rebuilds the
//! host. The same descriptors are what a loaded module will declare (I6/I7).

pub mod echo_fan;
pub mod eye_beam;
pub mod mode_collapse;
pub mod seismic_stomp;
mod strike;

/// The provider namespace of every module and schema in this crate.
pub const PROVIDER: &str = "ambition";

/// Every module this crate provides.
pub fn modules() -> Vec<ambition_extension_sdk::ModuleDescriptor> {
    vec![
        echo_fan::module(),
        eye_beam::module(),
        mode_collapse::module(),
        seismic_stomp::module(),
    ]
}

// Built for `wasm32-unknown-unknown`, this crate is a loaded module file
// (`ambition-ext-1`): `scripts/build_extension_modules.sh`.
ambition_extension_sdk::export_modules!(crate::modules);

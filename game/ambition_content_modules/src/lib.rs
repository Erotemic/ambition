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

pub mod apple_rain;
pub mod beam;
pub mod echo_fan;
pub mod eye_beam;
pub mod fsm;
pub mod gradient_cascade;
pub mod gradient_nova;
pub mod meteor;
pub mod minima_trap;
pub mod mode_collapse;
pub mod overfit_volley;
pub mod overflow_flood;
pub mod saddle_point;
pub mod seismic_stomp;
pub mod sentry;
pub mod shockwave;
mod strike;
pub mod trex;
pub mod volley;
pub mod vortex;
mod wielded;

/// The provider namespace of every module and schema in this crate.
pub const PROVIDER: &str = "ambition";

/// Every module this crate provides, as constructors: a loaded call builds
/// only the module it runs.
pub const MODULES: &[fn() -> ambition_extension_sdk::ModuleDescriptor] = &[
    apple_rain::module,
    beam::module,
    echo_fan::module,
    eye_beam::module,
    fsm::module,
    gradient_cascade::module,
    gradient_nova::module,
    meteor::module,
    minima_trap::module,
    mode_collapse::module,
    overfit_volley::module,
    overflow_flood::module,
    saddle_point::module,
    seismic_stomp::module,
    sentry::module,
    shockwave::module,
    trex::module,
    volley::module,
    vortex::module,
];

/// Every module this crate provides.
pub fn modules() -> Vec<ambition_extension_sdk::ModuleDescriptor> {
    MODULES.iter().map(|build| build()).collect()
}

// Built for `wasm32-unknown-unknown`, this crate is a loaded module file
// (`ambition-ext-1`): `scripts/build_extension_modules.sh`.
ambition_extension_sdk::export_modules!(list: crate::MODULES);

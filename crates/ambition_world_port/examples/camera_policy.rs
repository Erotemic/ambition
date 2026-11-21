//! The native camera policy as an independent Wasm artifact.
#[cfg(target_arch = "wasm32")]
const MODULES: &[fn() -> ambition_extension_sdk::ModuleDescriptor] =
    &[ambition_world_port::camera::module];
ambition_extension_sdk::export_modules!(list: crate::MODULES);

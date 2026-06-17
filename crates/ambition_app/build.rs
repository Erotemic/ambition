//! Build-time configuration for the statically-embedded SFX bank.
//!
//! The `static_sfx_bank` feature embeds the packed `sfx.bank` directly into the
//! binary for targets without a runtime asset directory (Android / wasm). The
//! consumer is `app::scene_setup::try_load_static_sfx_bank`, gated on the
//! `ambition_static_sfx_bank_path` cfg this script sets.
//!
//! A build script's `rustc-cfg` / `rustc-env` only apply to ITS OWN crate, so
//! this must live next to the consumer. It previously lived in
//! `ambition_gameplay_core/build.rs` and silently went dead (cfg never set, env never
//! exported) when the loader moved up to `ambition_app` — which is exactly the
//! `unexpected cfg` warning + dead static-bank path this restores.

use std::path::PathBuf;

fn main() {
    // Declare the cfg name so it is never "unexpected", even on the common
    // build where the feature is off and the cfg stays unset.
    println!("cargo:rustc-check-cfg=cfg(ambition_static_sfx_bank_path)");
    println!("cargo:rerun-if-env-changed=AMBITION_STATIC_SFX_BANK_PATH");

    if std::env::var_os("CARGO_FEATURE_STATIC_SFX_BANK").is_none() {
        return;
    }

    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let configured = std::env::var_os("AMBITION_STATIC_SFX_BANK_PATH")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            // Default to the sandbox content crate's packed bank (the assets
            // live there, not under this crate).
            let default_path = manifest_dir.join("../ambition_gameplay_core/assets/audio/sfx.bank");
            default_path.is_file().then_some(default_path)
        });

    if let Some(path) = configured {
        println!(
            "cargo:rustc-env=AMBITION_STATIC_SFX_BANK_PATH={}",
            path.display()
        );
        println!("cargo:rustc-cfg=ambition_static_sfx_bank_path");
        println!("cargo:rerun-if-changed={}", path.display());
    }
}

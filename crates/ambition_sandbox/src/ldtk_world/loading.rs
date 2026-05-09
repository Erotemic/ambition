//! LDtk file-loading policy.
//!
//! Decides whether the live build reads the checked-in external
//! `assets/ambition/worlds/sandbox.ldtk`, an env-override path, or
//! the statically packed copy embedded into the binary on Android.
//! Pure I/O policy — no validation or runtime conversion lives here.

use std::fs;

#[cfg(all(target_os = "android", feature = "static_map"))]
use super::hot_reload::configured_ldtk_path;
use super::hot_reload::sandbox_ldtk_path;
use super::project::LdtkProject;

impl LdtkProject {
    /// Load the sandbox map using the normal runtime policy.
    ///
    /// Desktop builds default to the external checked-in asset path so LDtk edits
    /// and modded maps do not require recompiling Rust. Android `static_map`
    /// builds default to the embedded map unless a user explicitly passes
    /// `--ldtk`, `--map`, or `AMBITION_LDTK`; the source-tree path is not a
    /// meaningful filesystem location inside the APK.
    pub fn load_default() -> Result<Self, String> {
        #[cfg(all(target_os = "android", feature = "static_map"))]
        if configured_ldtk_path().is_none() {
            return Self::load_static_map();
        }

        let path = sandbox_ldtk_path();
        match Self::load_from_path(&path) {
            Ok(project) => Ok(project),
            Err(error) => {
                #[cfg(feature = "static_map")]
                {
                    eprintln!(
                        "LDtk warning: {error}; falling back to statically packed sandbox.ldtk"
                    );
                    Self::load_static_map().map_err(|fallback_error| {
                        format!(
                            "{error}; statically packed sandbox.ldtk also failed: {fallback_error}"
                        )
                    })
                }
                #[cfg(not(feature = "static_map"))]
                {
                    Err(format!(
                        "{error}. No statically packed fallback is available in this build; \
                         restore the LDtk asset or rebuild with `--features static_map`."
                    ))
                }
            }
        }
    }

    #[cfg(feature = "static_map")]
    pub fn load_static_map() -> Result<Self, String> {
        serde_json::from_str(include_str!("../../assets/ambition/worlds/sandbox.ldtk"))
            .map_err(|error| format!("could not parse statically packed sandbox.ldtk: {error}"))
    }

    pub fn load_from_disk() -> Result<Self, String> {
        Self::load_from_path(sandbox_ldtk_path())
    }

    pub fn load_from_path(path: impl AsRef<std::path::Path>) -> Result<Self, String> {
        let path = path.as_ref();
        let text = fs::read_to_string(path)
            .map_err(|error| format!("could not read LDtk project {}: {error}", path.display()))?;
        serde_json::from_str(&text)
            .map_err(|error| format!("could not parse LDtk project {}: {error}", path.display()))
    }
}

//! LDtk file-loading policy.
//!
//! Decides whether the live build reads the checked-in external
//! `assets/ambition/worlds/sandbox.ldtk`, an env-override path, or
//! the statically packed copy embedded into the binary on Android.
//! Pure I/O policy — no validation or runtime conversion lives here.
//!
//! ## Multi-file world composition
//!
//! Story-content zones (intro, future real-game-map zones) live in
//! separate `.ldtk` source files next to sandbox.ldtk. They are
//! authored against the same project defs (cloned via
//! `python -m ambition_ldtk_tools world init`) so their entity/layer
//! uids match the sandbox's — that means the runtime can simply
//! append their `levels` arrays into the merged in-memory
//! `LdtkProject` without remapping anything. Every level keeps its
//! own iid, its own activeArea, and its own LoadingZone targets, so
//! cross-file room transitions work via the standard target_room
//! mechanism.

use std::fs;
use std::path::Path;

#[cfg(all(target_os = "android", feature = "static_map"))]
use super::hot_reload::configured_ldtk_path;
use super::hot_reload::sandbox_ldtk_path;
use super::project::LdtkProject;

/// Story-content world files appended into the runtime project on top
/// of `sandbox.ldtk`. Paths are relative to the same `worlds/`
/// directory the sandbox map lives in. New zones land here as their
/// `.ldtk` source files get authored. Missing files are tolerated —
/// the project still boots from sandbox.ldtk alone — so a partial
/// checkout doesn't crash startup.
const SECONDARY_WORLD_FILES: &[&str] = &["intro.ldtk"];

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
            let mut project = Self::load_static_map()?;
            merge_static_secondary_worlds(&mut project);
            return Ok(project);
        }

        let path = sandbox_ldtk_path();
        let mut project = match Self::load_from_path(&path) {
            Ok(project) => project,
            Err(error) => {
                #[cfg(feature = "static_map")]
                {
                    eprintln!(
                        "LDtk warning: {error}; falling back to statically packed sandbox.ldtk"
                    );
                    let mut project = Self::load_static_map().map_err(|fallback_error| {
                        format!(
                            "{error}; statically packed sandbox.ldtk also failed: {fallback_error}"
                        )
                    })?;
                    merge_static_secondary_worlds(&mut project);
                    return Ok(project);
                }
                #[cfg(not(feature = "static_map"))]
                {
                    return Err(format!(
                        "{error}. No statically packed fallback is available in this build; \
                         restore the LDtk asset or rebuild with `--features static_map`."
                    ));
                }
            }
        };
        merge_secondary_worlds(&mut project, &path);
        Ok(project)
    }

    #[cfg(feature = "static_map")]
    pub fn load_static_map() -> Result<Self, String> {
        serde_json::from_str(include_str!("../../assets/ambition/worlds/sandbox.ldtk"))
            .map_err(|error| format!("could not parse statically packed sandbox.ldtk: {error}"))
    }

    pub fn load_from_disk() -> Result<Self, String> {
        let path = sandbox_ldtk_path();
        let mut project = Self::load_from_path(&path)?;
        merge_secondary_worlds(&mut project, &path);
        Ok(project)
    }

    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref();
        let text = fs::read_to_string(path)
            .map_err(|error| format!("could not read LDtk project {}: {error}", path.display()))?;
        serde_json::from_str(&text)
            .map_err(|error| format!("could not parse LDtk project {}: {error}", path.display()))
    }
}

/// Walk [`SECONDARY_WORLD_FILES`] and append each present file's levels
/// into `project`. Missing files are skipped with a warning; malformed
/// files are skipped with a warning and the sandbox keeps booting.
fn merge_secondary_worlds(project: &mut LdtkProject, sandbox_path: &Path) {
    let Some(parent) = sandbox_path.parent() else {
        return;
    };
    for name in SECONDARY_WORLD_FILES {
        let path = parent.join(name);
        if !path.exists() {
            continue;
        }
        match LdtkProject::load_from_path(&path) {
            Ok(secondary) => append_levels(project, secondary, name),
            Err(error) => {
                eprintln!(
                    "LDtk warning: could not load secondary world '{name}' from {}: {error}; \
                     continuing without it",
                    path.display()
                );
            }
        }
    }
}

#[cfg(feature = "static_map")]
fn merge_static_secondary_worlds(project: &mut LdtkProject) {
    // Android `static_map` builds embed the sandbox map at compile
    // time. Secondary worlds need the same treatment — `include_str!`
    // each known file when it exists in the workspace. We currently
    // hard-code the intro because there's exactly one secondary
    // file; a build-time codegen pass can replace this when the list
    // grows.
    const INTRO_LDTK_STATIC: &str =
        include_str!("../../assets/ambition/worlds/intro.ldtk");
    match serde_json::from_str::<LdtkProject>(INTRO_LDTK_STATIC) {
        Ok(secondary) => append_levels(project, secondary, "intro.ldtk"),
        Err(error) => eprintln!(
            "LDtk warning: could not parse statically packed intro.ldtk: {error}; \
             continuing without it"
        ),
    }
}

#[cfg(not(feature = "static_map"))]
#[allow(dead_code)]
fn merge_static_secondary_worlds(_project: &mut LdtkProject) {}

fn append_levels(project: &mut LdtkProject, secondary: LdtkProject, source_label: &str) {
    let added = secondary.levels.len();
    project.levels.extend(secondary.levels);
    eprintln!("LDtk: merged {added} level(s) from secondary world '{source_label}'");
}

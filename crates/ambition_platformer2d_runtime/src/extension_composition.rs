//! The extension host in the platformer composition (fast-iteration I4).
//!
//! The runtime chooses where each public phase sits in the simulation
//! schedule, and which domain adapters answer which ports. The host and the
//! adapters name no game algorithm; the game declares modules from its own
//! plugins.
//!
//! | phase | placement | ports |
//! |---|---|---|
//! | `technique_execution` | `CombatSet::ContentSpecials`, gameplay-gated | trigger `ambition.boss.special_cast` (boss domain); request `ambition.projectiles.spawn` (projectile domain) |

use ambition_extension_host::{ExtensionHostPlugin, ExtensionSet};
use ambition_extension_sdk::phases::TECHNIQUE_EXECUTION;
use ambition_platformer2d_shared_tangle::schedule::{CombatSet, GameplayGated, SimScheduleExt};
use bevy::prelude::*;

pub struct ExtensionCompositionPlugin;

impl Plugin for ExtensionCompositionPlugin {
    fn build(&self, app: &mut App) {
        let sim = app.sim_schedule();
        app.add_plugins(ExtensionHostPlugin::new(sim));
        // `technique_execution` guarantees: the asking body's facts are
        // settled, and a request is consumed this tick. `ContentSpecials`
        // sits before the effect and projectile executors that drain it.
        app.configure_sets(
            sim,
            (
                ExtensionSet::Collect(TECHNIQUE_EXECUTION),
                ExtensionSet::Invoke(TECHNIQUE_EXECUTION),
                ExtensionSet::Lower(TECHNIQUE_EXECUTION),
            )
                .in_set(GameplayGated)
                .in_set(CombatSet::ContentSpecials),
        );
        ambition_boss_encounter::extension::install(app);
        ambition_projectiles::extension::install(app);
        #[cfg(feature = "wasm_modules")]
        load_developer_modules(app);
        #[cfg(not(feature = "wasm_modules"))]
        if app
            .world()
            .get_resource::<ExtensionModuleFiles>()
            .is_some_and(|files| !files.0.is_empty())
        {
            panic!(
                "ExtensionModuleFiles names module files, and this build has no WebAssembly \
                 backend: enable the `wasm_modules` feature, or the linked modules would run \
                 instead of the ones asked for"
            );
        }
    }
}

/// The variable that names loaded module files for a developer run.
pub const EXTENSION_MODULES_VAR: &str = "AMBITION_EXTENSION_MODULES";

/// The `.wasm` files (or directories of them) whose modules replace the
/// linked ones. A composition inserts it BEFORE adding the engine plugins;
/// when it is absent, [`EXTENSION_MODULES_VAR`] gives the list. Read once, at
/// plugin build.
#[derive(Resource, Clone, Debug, Default)]
pub struct ExtensionModuleFiles(pub Vec<std::path::PathBuf>);

/// ⭐ THE NO-RELINK LOOP. `AMBITION_EXTENSION_MODULES` is a `:`-separated list
/// of `.wasm` files or directories of them. Each file's modules are declared
/// as EXPLICIT replacements of the modules the game links under the same key,
/// so a developer edits a module, rebuilds only the module crate
/// (`scripts/build_extension_modules.sh`), and runs the same game binary.
///
/// ⛔ A file that does not load stops the run. The developer asked for this
/// file; running the linked module instead would show them behaviour they did
/// not write.
#[cfg(feature = "wasm_modules")]
fn load_developer_modules(app: &mut App) {
    use ambition_extension_host::ExtensionAppExt;
    let listed = match app.world().get_resource::<ExtensionModuleFiles>() {
        Some(files) => files.0.clone(),
        None => match std::env::var(EXTENSION_MODULES_VAR) {
            Ok(list) => list.split(':').filter(|p| !p.is_empty()).map(Into::into).collect(),
            Err(_) => return,
        },
    };
    for path in wasm_files(&listed) {
        let bytes = std::fs::read(&path)
            .unwrap_or_else(|e| panic!("{EXTENSION_MODULES_VAR}: cannot read {}: {e}", path.display()));
        let (backend, modules) = ambition_extension_wasm::WasmModules::load(&bytes)
            .unwrap_or_else(|e| panic!("{EXTENSION_MODULES_VAR}: {} does not load: {e}", path.display()));
        info!(
            "{EXTENSION_MODULES_VAR}: {} provides {:?}",
            path.display(),
            modules.iter().map(|m| m.key.to_string()).collect::<Vec<_>>()
        );
        app.add_loaded_extension_modules(backend, modules, true);
    }
}

#[cfg(feature = "wasm_modules")]
fn wasm_files(listed: &[std::path::PathBuf]) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    for path in listed.iter().cloned() {
        if path.is_dir() {
            let mut found: Vec<_> = std::fs::read_dir(&path)
                .unwrap_or_else(|e| panic!("{EXTENSION_MODULES_VAR}: cannot list {}: {e}", path.display()))
                .filter_map(|entry| entry.ok().map(|e| e.path()))
                .filter(|p| p.extension().is_some_and(|x| x == "wasm"))
                .collect();
            found.sort();
            files.extend(found);
        } else {
            files.push(path);
        }
    }
    files
}

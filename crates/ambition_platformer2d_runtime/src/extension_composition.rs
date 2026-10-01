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
    let mut watched = Vec::new();
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
        watched.push((path.clone(), modified(&path)));
    }
    if watched.is_empty() {
        return;
    }
    // ⭐ HOT RELOAD, ON THE ENGINE'S MECHANICAL-EDIT PROTOCOL (`Q120`): a
    // changed file is PROPOSED, the rollback timeline's owner ADMITS (and may
    // stop its own baseline to do so), and only then is the new code
    // PUBLISHED. A file that fails to load or to admit is reported and the
    // running code stays.
    app.insert_resource(WatchedModuleFiles {
        files: watched,
        frames_until_poll: 0,
    });
    app.init_resource::<ambition_platformer2d_core::PendingMechanicalEdits>();
    app.init_resource::<ambition_platformer2d_core::MechanicalEditAdmission>();
    ambition_platformer2d_shared_tangle::schedule::configure_mechanical_edit_sets(app);
    app.add_systems(
        PreUpdate,
        (
            propose_module_reload.in_set(ambition_platformer2d_core::MechanicalEditSet::Propose),
            publish_module_reload.in_set(ambition_platformer2d_core::MechanicalEditSet::Publish),
        ),
    );
}

/// The loaded files and the modification time each was last read at.
#[cfg(feature = "wasm_modules")]
#[derive(Resource)]
struct WatchedModuleFiles {
    files: Vec<(std::path::PathBuf, Option<std::time::SystemTime>)>,
    frames_until_poll: u32,
}

/// Frames between two looks at the files: a stat per file is cheap, and a
/// third of a second is below a developer's switch from editor to game.
#[cfg(feature = "wasm_modules")]
const POLL_FRAMES: u32 = 20;

#[cfg(feature = "wasm_modules")]
struct ExtensionModuleCode;

#[cfg(feature = "wasm_modules")]
fn module_code_domain() -> ambition_platformer2d_core::MechanicalDomain {
    ambition_platformer2d_core::MechanicalDomain::of::<ExtensionModuleCode>("extension module code")
}

#[cfg(feature = "wasm_modules")]
fn modified(path: &std::path::Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

#[cfg(feature = "wasm_modules")]
fn propose_module_reload(world: &mut World) {
    let changed: Vec<std::path::PathBuf> = {
        let mut watched = world.resource_mut::<WatchedModuleFiles>();
        if watched.frames_until_poll > 0 {
            watched.frames_until_poll -= 1;
            return;
        }
        watched.frames_until_poll = POLL_FRAMES;
        let mut changed = Vec::new();
        for (path, seen) in &mut watched.files {
            let now = modified(path);
            if now.is_some() && now != *seen {
                *seen = now;
                changed.push(path.clone());
            }
        }
        changed
    };
    for path in changed {
        let staged = std::fs::read(&path)
            .map_err(|e| format!("cannot read: {e}"))
            .and_then(|bytes| {
                ambition_extension_wasm::WasmModules::load(&bytes).map_err(|e| e.to_string())
            })
            .and_then(|(backend, modules)| {
                ambition_extension_host::reload::stage_loaded_replacement(world, backend, modules)
            });
        match staged {
            Ok(()) => {
                info!("{EXTENSION_MODULES_VAR}: {} changed; reload proposed", path.display());
                world
                    .resource_mut::<ambition_platformer2d_core::PendingMechanicalEdits>()
                    .propose(module_code_domain());
            }
            Err(reason) => error!(
                "{EXTENSION_MODULES_VAR}: {} changed and is NOT loaded; the running code \
                 stays: {reason}",
                path.display()
            ),
        }
    }
}

#[cfg(feature = "wasm_modules")]
fn publish_module_reload(world: &mut World) {
    if *world.resource::<ambition_platformer2d_core::MechanicalEditAdmission>()
        != ambition_platformer2d_core::MechanicalEditAdmission::Publish
    {
        return;
    }
    if world
        .resource_mut::<ambition_platformer2d_core::PendingMechanicalEdits>()
        .take(module_code_domain())
    {
        ambition_extension_host::reload::publish_staged_replacement(world);
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

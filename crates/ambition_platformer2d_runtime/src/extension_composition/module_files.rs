//! Source files, library dependencies, and atomic module reload candidates.

#[cfg(any(feature = "wasm_modules", feature = "script_modules"))]
use super::remint_session_content;
use bevy::prelude::*;

/// The variable that names loaded module files for a developer run.
pub const EXTENSION_MODULES_VAR: &str = "AMBITION_EXTENSION_MODULES";

/// Module files or directories: `.wasm` builds (`wasm_modules`) and `.rhai`
/// sources (`script_modules`). A script can declare its own module or supply
/// code for a linked module. A composition inserts the list before engine
/// plugins build; the watcher reads it at each poll. When absent,
/// [`EXTENSION_MODULES_VAR`] supplies the list at plugin build.
#[derive(Resource, Clone, Debug, Default)]
pub struct ExtensionModuleFiles(pub Vec<std::path::PathBuf>);

/// Composition paths override process inputs, including an empty list.
pub(super) fn requested_module_files(world: &World) -> Option<Vec<std::path::PathBuf>> {
    world
        .get_resource::<ExtensionModuleFiles>()
        .map(|files| files.0.clone())
        .or_else(|| {
            std::env::var_os(EXTENSION_MODULES_VAR).map(|list| {
                std::env::split_paths(&list)
                    .filter(|path| !path.as_os_str().is_empty())
                    .collect()
            })
        })
}

/// Load the requested entry files and capture their source dependencies.
/// Missing or invalid input refuses the requested composition.
#[cfg(any(feature = "wasm_modules", feature = "script_modules"))]
pub(super) fn load_developer_modules(app: &mut App) {
    use ambition_extension_host::ExtensionAppExt;
    let Some(listed) = requested_module_files(app.world()) else {
        return;
    };
    let mut watched = Vec::new();
    let mut snapshot = SourceSnapshot::default();
    for path in module_files(&listed) {
        let mut used = std::collections::BTreeSet::new();
        snapshot.observe(&path, &mut used);
        let artifact = path.display().to_string();
        match ModuleFileKind::of(&path) {
            #[cfg(feature = "wasm_modules")]
            ModuleFileKind::Wasm => {
                let (backend, modules) = load_wasm(&path)
                    .unwrap_or_else(|e| panic!("{EXTENSION_MODULES_VAR}: {artifact}: {e}"));
                info!(
                    "{EXTENSION_MODULES_VAR}: {artifact} provides {:?}",
                    modules
                        .iter()
                        .map(|m| m.key.to_string())
                        .collect::<Vec<_>>()
                );
                app.add_loaded_extension_modules(&artifact, backend, modules, true);
            }
            #[cfg(feature = "script_modules")]
            ModuleFileKind::Rhai => {
                let prepared = prepare_script(&path, &mut snapshot, &mut used)
                    .unwrap_or_else(|e| panic!("{EXTENSION_MODULES_VAR}: {e}"));
                let script = path.clone();
                app.add_bound_extension_artifact(
                    &artifact,
                    Box::new(move |linked| {
                        let bound = bind_script(&script, &prepared, linked)?;
                        info!(
                            "{EXTENSION_MODULES_VAR}: {} is the code of {:?}",
                            script.display(),
                            bound
                                .1
                                .iter()
                                .map(|m| m.key.to_string())
                                .collect::<Vec<_>>()
                        );
                        Ok(bound)
                    }),
                );
            }
        }
        watched.push(snapshot.files(&path, &used));
    }
    snapshot
        .validate()
        .unwrap_or_else(|e| panic!("{EXTENSION_MODULES_VAR}: {e}"));
    // A composition that names its files (a project) is watched even with
    // none yet: a file added to the list, or to a listed directory, loads.
    if watched.is_empty() && !app.world().contains_resource::<ExtensionModuleFiles>() {
        return;
    }
    // ⭐ HOT RELOAD, ON THE ENGINE'S MECHANICAL-EDIT PROTOCOL (`Q120`): a
    // changed file is PROPOSED, the rollback timeline's owner ADMITS (and may
    // stop its own baseline to do so), and only then is the new code
    // PUBLISHED. A file that fails to load or to admit is reported and the
    // running code stays.
    app.insert_resource(WatchedModuleFiles {
        listed,
        files: watched,
        attempted: None,
        before_staging: None,
        pair_refused: false,
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

/// What a listed file is, by its extension. A file this build cannot load
/// stops the run, naming the feature that would.
#[cfg(any(feature = "wasm_modules", feature = "script_modules"))]
enum ModuleFileKind {
    #[cfg(feature = "wasm_modules")]
    Wasm,
    #[cfg(feature = "script_modules")]
    Rhai,
}

#[cfg(any(feature = "wasm_modules", feature = "script_modules"))]
impl ModuleFileKind {
    const EXTENSIONS: &[&str] = &[
        #[cfg(feature = "wasm_modules")]
        "wasm",
        #[cfg(feature = "script_modules")]
        "rhai",
    ];

    /// [`Self::of`] for a file found while the game runs: one this build
    /// cannot load is reported, and the game goes on.
    fn try_of(path: &std::path::Path) -> Result<Self, String> {
        match path.extension().and_then(|x| x.to_str()) {
            #[cfg(feature = "wasm_modules")]
            Some("wasm") => Ok(Self::Wasm),
            #[cfg(feature = "script_modules")]
            Some("rhai") => Ok(Self::Rhai),
            _ => Err(format!(
                "not a module file this build loads (it loads {:?})",
                Self::EXTENSIONS
            )),
        }
    }

    fn of(path: &std::path::Path) -> Self {
        match path.extension().and_then(|x| x.to_str()) {
            #[cfg(feature = "wasm_modules")]
            Some("wasm") => Self::Wasm,
            #[cfg(feature = "script_modules")]
            Some("rhai") => Self::Rhai,
            #[cfg(not(feature = "wasm_modules"))]
            Some("wasm") => panic!(
                "{EXTENSION_MODULES_VAR}: {} is a WebAssembly module, and this build has no \
                 WebAssembly backend (the `wasm_modules` feature)",
                path.display()
            ),
            #[cfg(not(feature = "script_modules"))]
            Some("rhai") => panic!(
                "{EXTENSION_MODULES_VAR}: {} is a Rhai script, and this build has no script \
                 backend (the `script_modules` feature)",
                path.display()
            ),
            _ => panic!(
                "{EXTENSION_MODULES_VAR}: {} is not a module file this build loads (it loads \
                 {:?})",
                path.display(),
                Self::EXTENSIONS
            ),
        }
    }
}

#[cfg(feature = "wasm_modules")]
fn load_wasm(
    path: &std::path::Path,
) -> Result<
    (
        std::sync::Arc<dyn ambition_extension_host::ModuleBackend>,
        Vec<ambition_extension_sdk::ModuleDescriptor>,
    ),
    String,
> {
    let bytes = std::fs::read(path).map_err(|e| format!("cannot read: {e}"))?;
    let (backend, modules) = ambition_extension_wasm::WasmModules::load(&bytes)
        .map_err(|e| format!("does not load: {e}"))?;
    Ok((backend, modules))
}

#[cfg(any(feature = "wasm_modules", feature = "script_modules"))]
type ModuleBackend = (
    std::sync::Arc<dyn ambition_extension_host::ModuleBackend>,
    Vec<ambition_extension_sdk::ModuleDescriptor>,
);

#[cfg(feature = "script_modules")]
fn prepare_script(
    path: &std::path::Path,
    snapshot: &mut SourceSnapshot,
    used: &mut std::collections::BTreeSet<std::path::PathBuf>,
) -> Result<ambition_extension_rhai::PreparedScript, String> {
    let source = snapshot.read(path, used)?;
    let directory = path.parent().unwrap_or_else(|| std::path::Path::new("."));
    super::prepare_platformer_script(
        &source.name,
        &source.text,
        |id| snapshot.read(&directory.join(id), used),
    )
}

#[cfg(feature = "script_modules")]
fn bind_script(
    path: &std::path::Path,
    prepared: &ambition_extension_rhai::PreparedScript,
    linked: &[ambition_extension_host::DeclaredModule],
) -> Result<ModuleBackend, String> {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default();
    let (backend, modules) = prepared.bind_file(stem, linked)?;
    Ok((backend, modules))
}

#[cfg(any(feature = "wasm_modules", feature = "script_modules"))]
#[derive(Default)]
struct SourceSnapshot {
    stamps: std::collections::BTreeMap<std::path::PathBuf, Option<std::time::SystemTime>>,
    #[cfg(feature = "script_modules")]
    text: std::collections::BTreeMap<std::path::PathBuf, Result<String, String>>,
}

#[cfg(any(feature = "wasm_modules", feature = "script_modules"))]
impl SourceSnapshot {
    fn observe(
        &mut self,
        path: &std::path::Path,
        used: &mut std::collections::BTreeSet<std::path::PathBuf>,
    ) {
        used.insert(path.to_owned());
        self.stamps
            .entry(path.to_owned())
            .or_insert_with(|| modified(path));
    }

    #[cfg(feature = "script_modules")]
    fn read(
        &mut self,
        path: &std::path::Path,
        used: &mut std::collections::BTreeSet<std::path::PathBuf>,
    ) -> Result<ambition_extension_rhai::LibrarySource, String> {
        self.observe(path, used);
        let text = self
            .text
            .entry(path.to_owned())
            .or_insert_with(|| {
                std::fs::read_to_string(path)
                    .map_err(|e| format!("cannot read {}: {e}", path.display()))
            })
            .clone()?;
        Ok(ambition_extension_rhai::LibrarySource {
            name: path.display().to_string(),
            text,
        })
    }

    fn files(
        &self,
        entry: &std::path::Path,
        used: &std::collections::BTreeSet<std::path::PathBuf>,
    ) -> ArtifactFiles {
        ArtifactFiles {
            entry: entry.to_owned(),
            sources: used
                .iter()
                .map(|path| (path.clone(), self.stamps[path]))
                .collect(),
        }
    }

    fn validate(&self) -> Result<(), String> {
        for (path, stamp) in &self.stamps {
            if modified(path) != *stamp {
                return Err(format!(
                    "{} changed during source preparation; retry the candidate",
                    path.display()
                ));
            }
        }
        Ok(())
    }
}

#[cfg(any(feature = "wasm_modules", feature = "script_modules"))]
#[derive(Clone, Debug, PartialEq, Eq)]
struct ArtifactFiles {
    entry: std::path::PathBuf,
    sources: Vec<(std::path::PathBuf, Option<std::time::SystemTime>)>,
}

#[cfg(any(feature = "wasm_modules", feature = "script_modules"))]
fn prepare_module_file(
    path: &std::path::Path,
    linked: &[ambition_extension_host::DeclaredModule],
    snapshot: &mut SourceSnapshot,
    used: &mut std::collections::BTreeSet<std::path::PathBuf>,
) -> Result<ModuleBackend, String> {
    snapshot.observe(path, used);
    #[cfg(not(feature = "script_modules"))]
    let _ = linked;
    match ModuleFileKind::try_of(path)? {
        #[cfg(feature = "wasm_modules")]
        ModuleFileKind::Wasm => load_wasm(path),
        #[cfg(feature = "script_modules")]
        ModuleFileKind::Rhai => bind_script(path, &prepare_script(path, snapshot, used)?, linked),
    }
}

/// Prepare a module file for inspection or reload. A script can declare its
/// own module or replace a linked module by file stem. The result still
/// needs admission before publication.
#[cfg(any(feature = "wasm_modules", feature = "script_modules"))]
pub fn load_module_file(
    path: &std::path::Path,
    linked: &[ambition_extension_host::DeclaredModule],
) -> Result<
    (
        std::sync::Arc<dyn ambition_extension_host::ModuleBackend>,
        Vec<ambition_extension_sdk::ModuleDescriptor>,
    ),
    String,
> {
    let mut snapshot = SourceSnapshot::default();
    let loaded = prepare_module_file(path, linked, &mut snapshot, &mut Default::default())?;
    snapshot.validate()?;
    Ok(loaded)
}

/// The loaded files and the modification time each was last read at.
#[cfg(any(feature = "wasm_modules", feature = "script_modules"))]
#[derive(Resource)]
struct WatchedModuleFiles {
    /// The list as it was read at build: the variable's. When the
    /// composition has `ExtensionModuleFiles`, each poll reads that instead,
    /// so a project that changes its list is followed.
    listed: Vec<std::path::PathBuf>,
    files: Vec<ArtifactFiles>,
    frames_until_poll: u32,
    attempted: Option<Vec<ArtifactFiles>>,
    /// The admitted set before the last staging, put back when a candidate
    /// staged with content is discarded, so its files count as changed again.
    before_staging: Option<Vec<ArtifactFiles>>,
    /// A candidate staged with content was discarded: the next change to
    /// either side asks for both again ([`module_pair_was_refused`]).
    pair_refused: bool,
}

/// Frames between two looks at the files: a stat per file is cheap, and a
/// third of a second is below a developer's switch from editor to game.
#[cfg(any(feature = "wasm_modules", feature = "script_modules"))]
const POLL_FRAMES: u32 = 20;

#[cfg(any(feature = "wasm_modules", feature = "script_modules"))]
struct ExtensionModuleCode;

#[cfg(any(feature = "wasm_modules", feature = "script_modules"))]
fn module_code_domain() -> ambition_platformer2d_core::MechanicalDomain {
    ambition_platformer2d_core::MechanicalDomain::of::<ExtensionModuleCode>("extension module code")
}

#[cfg(any(feature = "wasm_modules", feature = "script_modules"))]
fn modified(path: &std::path::Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

/// What changed among the module files since the admitted set.
#[cfg(any(feature = "wasm_modules", feature = "script_modules"))]
pub struct ModuleFileChanges {
    pub changed: Vec<std::path::PathBuf>,
    pub removed: Vec<std::path::PathBuf>,
    /// The file set this look saw, admitted when the changes are staged.
    current: Vec<ArtifactFiles>,
}

/// Present when the content owner looks at the module files together with its
/// own sources (`ambition_content::content_watch`), so a script and the
/// content it uses are one candidate. The module poller then stays out: two
/// pollers on two clocks would see one save in different frames.
#[cfg(any(feature = "wasm_modules", feature = "script_modules"))]
#[derive(Resource)]
pub struct ModuleFilesLookedWithContent;

/// Look at the module files now: a file added to the list or to a listed
/// directory is new, one gone from them leaves, each against the last
/// admitted set. `None` when nothing changed, the composition watches no
/// files, or this set was already tried and refused (`retry` asks again
/// anyway, for a refused pair whose other half changed).
#[cfg(any(feature = "wasm_modules", feature = "script_modules"))]
pub fn look_at_module_files(world: &mut World, retry: bool) -> Option<ModuleFileChanges> {
    world.get_resource::<WatchedModuleFiles>()?;
    let listed = match world.get_resource::<ExtensionModuleFiles>() {
        Some(files) => files.0.clone(),
        None => world.resource::<WatchedModuleFiles>().listed.clone(),
    };
    let watched = world.resource::<WatchedModuleFiles>();
    let mut stamps = std::collections::BTreeMap::new();
    let current: Vec<_> = discover_module_files(&listed)
        .into_iter()
        .map(|entry| {
            let prior = watched
                .attempted
                .as_ref()
                .and_then(|tried| tried.iter().find(|row| row.entry == entry))
                .or_else(|| watched.files.iter().find(|row| row.entry == entry));
            let paths: Vec<_> = prior
                .map(|row| row.sources.iter().map(|(path, _)| path.clone()).collect())
                .unwrap_or_else(|| vec![entry.clone()]);
            let sources = paths
                .into_iter()
                .map(|path| {
                    let stamp = *stamps
                        .entry(path.clone())
                        .or_insert_with(|| modified(&path));
                    (path, stamp)
                })
                .collect();
            ArtifactFiles { entry, sources }
        })
        .collect();
    if current == watched.files || (!retry && watched.attempted.as_ref() == Some(&current)) {
        return None;
    }
    let changed = current
        .iter()
        .filter(|row| !watched.files.contains(row))
        .map(|row| row.entry.clone())
        .collect();
    let removed = watched
        .files
        .iter()
        .filter(|row| !current.iter().any(|present| present.entry == row.entry))
        .map(|row| row.entry.clone())
        .collect();
    let mut watched = world.resource_mut::<WatchedModuleFiles>();
    watched.attempted = Some(current.clone());
    Some(ModuleFileChanges {
        changed,
        removed,
        current,
    })
}

/// Load the changed files and stage them, with the removed ones, as ONE
/// candidate (`stage_module_changes`). `Ok` names what was staged; nothing is
/// published until [`propose_staged_modules`] or [`publish_paired_modules`].
///
/// One observed file set is one candidate: a file that does not load refuses
/// it, every running module stays, and the set is not tried again until a
/// file changes.
#[cfg(any(feature = "wasm_modules", feature = "script_modules"))]
pub fn stage_module_file_changes(
    world: &mut World,
    changes: ModuleFileChanges,
) -> Result<Vec<String>, String> {
    let linked = ambition_extension_host::linked_modules(world);
    let mut loaded = Vec::new();
    let mut snapshot = SourceSnapshot::default();
    let mut current = changes.current;
    for path in &changes.changed {
        let mut used = std::collections::BTreeSet::new();
        let prepared = prepare_module_file(path, &linked, &mut snapshot, &mut used);
        let mut sources = snapshot.files(path, &used);
        if prepared.is_err() {
            if let Some(before) = current.iter().find(|row| row.entry == *path) {
                for (known, _) in &before.sources {
                    if !used.contains(known) {
                        sources.sources.push((known.clone(), modified(known)));
                    }
                }
                sources.sources.sort_by(|a, b| a.0.cmp(&b.0));
            }
        }
        if let Some(row) = current.iter_mut().find(|row| row.entry == *path) {
            *row = sources;
        }
        match prepared {
            Ok((backend, modules)) => {
                loaded.push(ambition_extension_host::reload::LoadedArtifact {
                    artifact: std::sync::Arc::from(path.display().to_string()),
                    backend,
                    modules,
                })
            }
            Err(reason) => {
                world.resource_mut::<WatchedModuleFiles>().attempted = Some(current);
                return Err(format!("{} does not load: {reason}", path.display()));
            }
        }
    }
    if let Err(reason) = snapshot.validate() {
        world.resource_mut::<WatchedModuleFiles>().attempted = Some(current);
        return Err(reason);
    }
    if loaded.is_empty() && changes.removed.is_empty() {
        return Err("no changed file loads".to_string());
    }
    let mut names: Vec<String> = loaded.iter().map(|a| a.artifact.to_string()).collect();
    let removed: Vec<std::sync::Arc<str>> = changes
        .removed
        .iter()
        .map(|path| std::sync::Arc::from(path.display().to_string()))
        .collect();
    names.extend(removed.iter().map(|name| format!("{name} (removed)")));
    let saved = crate::module_save_road::current_saved_rows(world);
    if let Err(reason) =
        ambition_extension_host::reload::stage_module_changes(world, loaded, removed, &saved)
    {
        world.resource_mut::<WatchedModuleFiles>().attempted = Some(current);
        return Err(format!("{}: {reason}", names.join(", ")));
    }
    let mut watched = world.resource_mut::<WatchedModuleFiles>();
    let before = std::mem::replace(&mut watched.files, current);
    watched.before_staging.get_or_insert(before);
    watched.attempted = None;
    Ok(names)
}

/// A candidate staged with content was discarded since the last successful
/// pair: the next change to either side asks for both again.
#[cfg(any(feature = "wasm_modules", feature = "script_modules"))]
pub fn module_pair_was_refused(world: &World) -> bool {
    world
        .get_resource::<WatchedModuleFiles>()
        .is_some_and(|watched| watched.pair_refused)
}

/// The pair was asked for again and staged: the refusal is answered.
#[cfg(any(feature = "wasm_modules", feature = "script_modules"))]
pub fn forget_module_pair_refusal(world: &mut World) {
    if let Some(mut watched) = world.get_resource_mut::<WatchedModuleFiles>() {
        watched.pair_refused = false;
    }
}

/// Ask the timeline's owner to admit the staged modules on their own.
#[cfg(any(feature = "wasm_modules", feature = "script_modules"))]
pub fn propose_staged_modules(world: &mut World) {
    if !world.resource::<ambition_extension_host::reload::StagedModuleReplacement>().is_staged() {
        world.resource_mut::<WatchedModuleFiles>().before_staging = None;
        return;
    }

    world
        .resource_mut::<ambition_platformer2d_core::PendingMechanicalEdits>()
        .propose(module_code_domain());
}

/// Publish the staged modules at the commit of the content generation they
/// were staged with: the boundary that publishes the pack publishes the code.
#[cfg(any(feature = "wasm_modules", feature = "script_modules"))]
pub fn publish_paired_modules(world: &mut World) -> bool {
    let published = ambition_extension_host::reload::publish_staged_replacement(world);
    if published {
        if let Some(mut watched) = world.get_resource_mut::<WatchedModuleFiles>() {
            watched.before_staging = None;
        }
        remint_session_content(world);
    }
    published
}

/// Drop the staged modules of a content generation that will not activate.
/// Their files count as changed again, refused as tried: the next change to
/// either side asks for the pair again ([`module_pair_was_refused`]).
#[cfg(any(feature = "wasm_modules", feature = "script_modules"))]
pub fn discard_paired_modules(world: &mut World) -> bool {
    let discarded = ambition_extension_host::reload::discard_staged_replacement(world);
    if let Some(mut watched) = world.get_resource_mut::<WatchedModuleFiles>() {
        if let Some(before) = watched.before_staging.take() {
            let tried = std::mem::replace(&mut watched.files, before);
            watched.attempted = Some(tried);
        }
        watched.pair_refused = true;
    }
    discarded
}

#[cfg(any(feature = "wasm_modules", feature = "script_modules"))]
fn propose_module_reload(world: &mut World) {
    if world.contains_resource::<ModuleFilesLookedWithContent>() {
        return;
    }
    {
        let mut watched = world.resource_mut::<WatchedModuleFiles>();
        if watched.frames_until_poll > 0 {
            watched.frames_until_poll -= 1;
            return;
        }
        watched.frames_until_poll = POLL_FRAMES;
    }
    let Some(changes) = look_at_module_files(world, false) else {
        return;
    };
    match stage_module_file_changes(world, changes) {
        Ok(names) => {
            info!("{EXTENSION_MODULES_VAR}: {names:?} changed; reload proposed");
            propose_staged_modules(world);
        }
        Err(reason) => error!(
            "{EXTENSION_MODULES_VAR}: changed modules are NOT loaded; the complete running module \
             set stays: {reason}"
        ),
    }
}

#[cfg(any(feature = "wasm_modules", feature = "script_modules"))]
fn publish_module_reload(world: &mut World) {
    if *world.resource::<ambition_platformer2d_core::MechanicalEditAdmission>()
        != ambition_platformer2d_core::MechanicalEditAdmission::Publish
    {
        return;
    }
    if world
        .resource_mut::<ambition_platformer2d_core::PendingMechanicalEdits>()
        .take(module_code_domain())
        && ambition_extension_host::reload::publish_staged_replacement(world)
    {
        world.resource_mut::<WatchedModuleFiles>().before_staging = None;
        remint_session_content(world);
    }
}

/// The module files `listed` names now, while the game runs: as
/// [`module_files`], but a directory that cannot be read gives none, and a
/// listed file that is gone is gone.
#[cfg(any(feature = "wasm_modules", feature = "script_modules"))]
fn discover_module_files(listed: &[std::path::PathBuf]) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    for path in listed {
        if path.is_dir() {
            let Ok(entries) = std::fs::read_dir(path) else {
                continue;
            };
            let mut found: Vec<_> = entries
                .filter_map(|entry| entry.ok().map(|e| e.path()))
                .filter(|p| {
                    p.extension()
                        .and_then(|x| x.to_str())
                        .is_some_and(|x| ModuleFileKind::EXTENSIONS.contains(&x))
                })
                .collect();
            found.sort();
            files.extend(found);
        } else if path.exists() {
            files.push(path.clone());
        }
    }
    let mut seen = std::collections::BTreeSet::new();
    files.retain(|path| seen.insert(path.canonicalize().unwrap_or_else(|_| path.clone())));
    files
}

#[cfg(any(feature = "wasm_modules", feature = "script_modules"))]
fn module_files(listed: &[std::path::PathBuf]) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    for path in listed.iter().cloned() {
        if path.is_dir() {
            let mut found: Vec<_> = std::fs::read_dir(&path)
                .unwrap_or_else(|e| {
                    panic!(
                        "{EXTENSION_MODULES_VAR}: cannot list {}: {e}",
                        path.display()
                    )
                })
                .filter_map(|entry| entry.ok().map(|e| e.path()))
                .filter(|p| {
                    p.extension()
                        .and_then(|x| x.to_str())
                        .is_some_and(|x| ModuleFileKind::EXTENSIONS.contains(&x))
                })
                .collect();
            found.sort();
            files.extend(found);
        } else {
            files.push(path);
        }
    }
    let mut seen = std::collections::BTreeSet::new();
    files.retain(|path| seen.insert(path.canonicalize().unwrap_or_else(|_| path.clone())));
    files
}

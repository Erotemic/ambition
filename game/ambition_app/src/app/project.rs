//! A project: a game an already built host runs, with no game Rust (scripting
//! audit P3, `docs/planning/engine/runtime-authoring-audit-2026-11-14/execution-plan.md`).
//!
//! `project.ron` joins what the host already reads:
//!
//! ```ron
//! (
//!     schema: "ambition.project.v1",
//!     name: "Sentry drill",
//!     // A content pack root: a directory with its own `pack.ron`.
//!     content: Some("content"),
//!     // LDtk files named by the content pack's world_manifest source.
//!     worlds: Some("worlds"),
//!     // Behaviour modules: `.rhai` sources (and `.wasm` builds), or
//!     // directories of them. A script can declare its own module or supply
//!     // code for the linked module its file stem names.
//!     modules: ["modules"],
//!     // The room the run starts in.
//!     entry_room: Some("hall_of_characters"),
//!     // The bag every session begins with: catalog ids and counts. Absent,
//!     // the game's own starting bag.
//!     starting_items: Some([("bomb", 3), ("lantern", 1)]),
//! )
//! ```
//!
//! Paths are relative to the manifest's directory. This adapter lowers the
//! manifest into the composition inputs that already exist: the module files
//! (`ExtensionModuleFiles`), the selected pack (`select_pack`), the start
//! room (`StartRoomOverride`) and the starting bag (`PreparedStartingBag`,
//! which the bag's owner reads in place of the game's Rust callback). It is
//! part of the host: a project implements no `GameModule`.
//! The optional `worlds` directory supplies `WorldManifest` roots. Without
//! it, the selected pack names worlds in the installed host's directory.
//!
//! Run: `ambition_game_bin --project path/to/project.ron` (or its directory).

use std::path::{Path, PathBuf};

use bevy::prelude::App;

/// The manifest format this host reads.
pub const PROJECT_SCHEMA: &str = "ambition.project.v1";

/// The file a project directory is read from.
pub const PROJECT_FILE: &str = "project.ron";

/// `project.ron`, as written.
#[derive(serde::Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectManifest {
    pub schema: String,
    pub name: String,
    #[serde(default)]
    pub content: Option<String>,
    /// The directory that holds the selected pack's LDtk world files.
    #[serde(default)]
    pub worlds: Option<String>,
    #[serde(default)]
    pub modules: Vec<String>,
    #[serde(default)]
    pub entry_room: Option<String>,
    #[serde(default)]
    pub starting_items: Option<Vec<(String, u32)>>,
}

/// A manifest with its paths resolved and checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    pub name: String,
    /// The manifest this was read from.
    pub manifest: PathBuf,
    pub content_root: Option<PathBuf>,
    pub worlds_root: Option<PathBuf>,
    pub module_files: Vec<PathBuf>,
    pub entry_room: Option<String>,
    /// Catalog ids and counts; `None` keeps the game's starting bag.
    pub starting_items: Option<Vec<(String, u32)>>,
}

impl Project {
    /// Read the project at `path`: its `project.ron`, or that directory's.
    pub fn load(path: &Path) -> Result<Self, String> {
        let manifest = if path.is_dir() { path.join(PROJECT_FILE) } else { path.to_path_buf() };
        let text = std::fs::read_to_string(&manifest)
            .map_err(|err| format!("{}: cannot read the project: {err}", manifest.display()))?;
        Self::from_manifest(&manifest, &text)
    }

    /// Resolve `text`, read from `manifest`.
    pub fn from_manifest(manifest: &Path, text: &str) -> Result<Self, String> {
        let at = manifest.display();
        let parsed: ProjectManifest = ron::from_str(text).map_err(|err| format!("{at}: {err}"))?;
        if parsed.schema != PROJECT_SCHEMA {
            return Err(format!(
                "{at}: schema \"{}\" is not one this host reads (it reads \"{PROJECT_SCHEMA}\")",
                parsed.schema
            ));
        }
        let dir = manifest.parent().unwrap_or(Path::new("."));
        let mut problems = Vec::new();
        let content_root = parsed.content.as_deref().map(|content| dir.join(content));
        if let Some(root) = &content_root {
            if !root.join("pack.ron").is_file() {
                problems.push(format!("content \"{}\" has no pack.ron", root.display()));
            }
        }
        let worlds_root = parsed.worlds.as_deref().map(|worlds| dir.join(worlds));
        let worlds_root = match worlds_root {
            Some(root) if root.is_dir() => match root.canonicalize() {
                Ok(root) if root.file_name().is_some() => Some(root),
                _ => {
                    problems.push(format!("worlds \"{}\" must name a directory below a filesystem root", root.display()));
                    None
                }
            },
            Some(root) => {
                problems.push(format!("worlds \"{}\" is not a directory", root.display()));
                None
            }
            None => None,
        };
        let module_files: Vec<PathBuf> = parsed.modules.iter().map(|module| dir.join(module)).collect();
        for (written, path) in parsed.modules.iter().zip(&module_files) {
            if !path.exists() {
                problems.push(format!("module \"{written}\" does not exist ({})", path.display()));
            }
        }
        if let Some(room) = &parsed.entry_room {
            if room.trim().is_empty() {
                problems.push("entry_room is empty".to_string());
            }
        }
        // Whether an id names an item is the catalog's to say, when the pack
        // is selected (`PreparedStartingBag::bag`); the shape is checked here.
        for (id, count) in parsed.starting_items.iter().flatten() {
            if id.trim().is_empty() {
                problems.push("starting_items has an empty item id".to_string());
            }
            if *count == 0 {
                problems.push(format!("starting_items gives \"{id}\" a count of 0"));
            }
        }
        if !problems.is_empty() {
            return Err(format!("{at}:\n  {}", problems.join("\n  ")));
        }
        Ok(Self {
            name: parsed.name,
            manifest: manifest.to_path_buf(),
            content_root,
            worlds_root,
            module_files,
            entry_room: parsed.entry_room,
            starting_items: parsed.starting_items,
        })
    }

    /// Put the project's inputs into `app`, before its plugins build.
    pub fn compose(&self, app: &mut App) -> Result<(), String> {
        if let Some(root) = &self.content_root {
            let pack = ambition_content::pack::compile_pack_from(root)?;
            ambition_content::pack::select_pack(app, std::sync::Arc::new(pack)).map_err(|refused| refused.to_string())?;
        }
        let mut authored_entry = None;
        if let Some(root) = &self.worlds_root {
            use ambition_platformer2d::world::world_manifest::WorldRoots;
            let pack = ambition_content::pack::select(app.world_mut());
            let folder = root.file_name().expect("the world directory has a name").to_string_lossy();
            let manifest = ambition_content::worlds::manifest_from_pack(&pack, &WorldRoots {
                loose_dir: root,
                asset_prefix: &format!("{PROJECT_ASSET_SOURCE}://{folder}/"),
                embedded_prefix: "",
                embedded_text: &|_| None,
            }).map_err(|error| format!("{}: {error}", self.manifest.display()))?;
            authored_entry = Some(manifest.entry_room.clone());
            app.insert_resource(manifest);
        }
        // Even with no modules: the list is the project's (not the developer
        // variable's), and the host watches it for files added later.
        app.insert_resource(ambition_platformer2d::extension::ExtensionModuleFiles(self.module_files.clone()));
        app.insert_resource(ProjectWatch {
            seen: modified(&self.manifest),
            project: self.clone(),
            frames_until_poll: POLL_FRAMES,
        });
        app.add_systems(bevy::prelude::Update, watch_project);
        if let Some(items) = &self.starting_items {
            app.insert_resource(ambition_platformer2d::item_runtime::starting_bag::PreparedStartingBag(items.clone()));
        }
        // A `--start-room` typed beside `--project` is the request.
        #[cfg(not(target_arch = "wasm32"))]
        let typed_room = super::cli::cli_start_room_arg().is_some();
        #[cfg(target_arch = "wasm32")]
        let typed_room = false;
        if let Some(room) = self.entry_room.as_ref().or(authored_entry.as_ref()).filter(|_| !typed_room) {
            app.insert_resource(super::StartRoomOverride(room.clone()));
            app.insert_resource(super::StartRoomMustResolve);
        }
        Ok(())
    }

    /// Register the project's world files and their relative assets before
    /// Bevy builds `AssetPlugin`. These files are external to the host bundle.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn register_world_asset_source(&self, app: &mut App) {
        if let Some(root) = &self.worlds_root {
            let assets = root.parent().expect("the world directory has a parent").to_string_lossy();
            ambition_platformer2d::asset_manager::library_source::register_external_source(app, PROJECT_ASSET_SOURCE, &assets);
        }
    }

    /// Build a desktop app that starts in this project. Register the asset
    /// source before Bevy creates its readers, then compose the game inputs.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn app(&self, render: super::cli::VisibleRenderMode) -> App {
        super::cli::build_visible_app_for_project(render, false, Some(self), |_| {})
    }
}

/// The source of external project worlds and assets they reference.
pub const PROJECT_ASSET_SOURCE: &str = "project";

/// The tools' road: a stepping harness (`inspect_geometry`,
/// `extension_inspect`, the RL drivers) on this project, its inputs composed
/// before Ambition's. An option the caller states (a start room, module
/// files) is the caller's.
#[cfg(feature = "rl_sim")]
impl Project {
    pub fn sim(
        &self,
        options: crate::rl_sim::Platformer2dSimHarnessOptions,
    ) -> Result<crate::rl_sim::Platformer2dSimHarness, String> {
        let stated_files = !options.extension_module_files.is_empty();
        crate::rl_sim::Platformer2dSimHarness::build(options, |app, options| {
            let files = app.world().get_resource::<ambition_platformer2d::extension::ExtensionModuleFiles>().cloned();
            self.compose(app)?;
            if let Some(files) = files.filter(|_| stated_files) {
                app.insert_resource(files);
            }
            crate::rl_sim::ambition_sim_composition(app, options)
        })
    }
}

/// [`Project::sim`] for the project at `path`, or the plain harness.
#[cfg(feature = "rl_sim")]
pub fn sim_for(
    path: Option<&Path>,
    options: crate::rl_sim::Platformer2dSimHarnessOptions,
) -> Result<crate::rl_sim::Platformer2dSimHarness, String> {
    use crate::rl_sim::AmbitionSim as _;
    match path {
        Some(path) => Project::load(path)?.sim(options),
        None => crate::rl_sim::Platformer2dSimHarness::new_with_options(options),
    }
}

/// The running project and when its manifest was last read.
#[derive(bevy::prelude::Resource)]
pub struct ProjectWatch {
    pub project: Project,
    seen: Option<std::time::SystemTime>,
    frames_until_poll: u32,
}

/// Frames between two looks at the manifest, as the module watcher's.
const POLL_FRAMES: u32 = 20;

fn modified(path: &Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

/// ⭐ A MODULE ADDED TO OR REMOVED FROM THE MANIFEST, WHILE THE GAME RUNS:
/// the new list becomes `ExtensionModuleFiles`, which the module watcher
/// reads at its next poll (a new file loads, a gone one leaves and the
/// linked module of its key runs again). A manifest that does not load
/// leaves the running project as it is. A new content root or entry room is
/// read at the next start.
fn watch_project(
    mut watch: bevy::prelude::ResMut<ProjectWatch>,
    files: Option<bevy::prelude::ResMut<ambition_platformer2d::extension::ExtensionModuleFiles>>,
) {
    if watch.frames_until_poll > 0 {
        watch.frames_until_poll -= 1;
        return;
    }
    watch.frames_until_poll = POLL_FRAMES;
    let now = modified(&watch.project.manifest);
    if now.is_none() || now == watch.seen {
        return;
    }
    watch.seen = now;
    let next = match Project::load(&watch.project.manifest) {
        Ok(next) => next,
        Err(error) => {
            bevy::log::error!("the project changed and does not load; the running project stays: {error}");
            return;
        }
    };
    if next.content_root != watch.project.content_root || next.worlds_root != watch.project.worlds_root || next.entry_room != watch.project.entry_room {
        bevy::log::warn!("project \"{}\": a new content root, world directory, or entry room is read at the next start", next.name);
    }
    if next.module_files != watch.project.module_files {
        bevy::log::info!("project \"{}\": its modules are now {:?}", next.name, next.module_files);
        if let Some(mut files) = files {
            files.0 = next.module_files.clone();
        }
    }
    watch.project = next;
}

/// The project `--project PATH` names, read once. A project that does not
/// load stops the run: running without it would show behaviour the author
/// did not write.
#[cfg(not(target_arch = "wasm32"))]
pub fn cli_project() -> Option<&'static Project> {
    static PROJECT: std::sync::OnceLock<Option<Project>> = std::sync::OnceLock::new();
    PROJECT
        .get_or_init(|| {
            let args: Vec<String> = std::env::args().skip(1).collect();
            let path = parse_project_arg(&args)?;
            match Project::load(Path::new(&path)) {
                Ok(project) => {
                    eprintln!("ambition_app: project \"{}\" ({})", project.name, project.manifest.display());
                    Some(project)
                }
                Err(error) => {
                    eprintln!("ambition_app: --project {path}: {error}");
                    std::process::exit(2);
                }
            }
        })
        .as_ref()
}

/// Compose the `--project` inputs, when there is one.
#[cfg(not(target_arch = "wasm32"))]
pub fn compose_cli_project(app: &mut App) {
    if let Some(project) = cli_project() {
        if let Err(error) = project.compose(app) {
            eprintln!("ambition_app: project \"{}\": {error}", project.name);
            std::process::exit(2);
        }
    }
}

pub(super) fn parse_project_arg(args: &[String]) -> Option<String> {
    args.iter().enumerate().find_map(|(i, arg)| match arg.as_str() {
        "--project" => args.get(i + 1).cloned(),
        _ => arg.strip_prefix("--project=").map(str::to_string),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn example() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/projects/sentry_drill")
    }

    #[test]
    fn the_example_project_resolves_against_its_own_directory() {
        let project = Project::load(&example()).expect("the example project loads");
        assert_eq!(project.manifest, example().join(PROJECT_FILE));
        assert_eq!(project.module_files, vec![example().join("modules/sentry.rhai")]);
        assert!(project.entry_room.is_some());
    }

    #[test]
    fn a_project_names_what_is_wrong_with_it() {
        let manifest = Path::new("/nowhere/project.ron");
        let refused = Project::from_manifest(
            manifest,
            r#"(schema: "ambition.project.v1", name: "x", content: Some("c"), modules: ["m.rhai"])"#,
        )
        .unwrap_err();
        assert!(refused.contains("content \"/nowhere/c\" has no pack.ron"), "{refused}");
        assert!(refused.contains("module \"m.rhai\" does not exist"), "{refused}");
        let refused = Project::from_manifest(manifest, r#"(schema: "ambition.project.v9", name: "x")"#).unwrap_err();
        assert!(refused.contains("not one this host reads"), "{refused}");
        let refused = Project::from_manifest(manifest, r#"(schema: "ambition.project.v1", name: "x", mods: [])"#)
            .unwrap_err();
        assert!(refused.contains("mods"), "{refused}");
    }

    #[test]
    fn a_module_added_to_the_manifest_reaches_the_module_list() {
        let dir = std::env::temp_dir().join(format!("ambition_project_watch_{}", std::process::id()));
        std::fs::create_dir_all(dir.join("modules")).unwrap();
        std::fs::write(dir.join("modules/sentry.rhai"), "").unwrap();
        let write = |modules: &str| {
            let text = format!(r#"(schema: "ambition.project.v1", name: "watched", modules: [{modules}])"#);
            std::fs::write(dir.join(PROJECT_FILE), text).unwrap();
        };
        write("");
        let project = Project::load(&dir).unwrap();
        let mut app = App::new();
        project.compose(&mut app).unwrap();
        let files = |app: &App| app.world().resource::<ambition_platformer2d::extension::ExtensionModuleFiles>().0.clone();
        assert!(files(&app).is_empty());
        // A file system's clock can be coarse: the edit must look newer.
        std::thread::sleep(std::time::Duration::from_millis(20));
        write(r#""modules/sentry.rhai""#);
        for _ in 0..=POLL_FRAMES {
            app.update();
        }
        assert_eq!(files(&app), vec![dir.join("modules/sentry.rhai")]);
        // A manifest that does not load leaves the list as it is.
        std::thread::sleep(std::time::Duration::from_millis(20));
        write(r#""modules/gone.rhai""#);
        for _ in 0..=POLL_FRAMES {
            app.update();
        }
        assert_eq!(files(&app), vec![dir.join("modules/sentry.rhai")]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_flag_takes_both_spellings() {
        let args = |s: &[&str]| s.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(parse_project_arg(&args(&["--project", "a"])), Some("a".into()));
        assert_eq!(parse_project_arg(&args(&["--headless", "--project=b"])), Some("b".into()));
        assert_eq!(parse_project_arg(&args(&["--headless"])), None);
    }
}

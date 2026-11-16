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
//!     // Behaviour modules: `.rhai` sources (and `.wasm` builds), or
//!     // directories of them. Each replaces the linked module its file names.
//!     modules: ["modules"],
//!     // The room the run starts in.
//!     entry_room: Some("hall_of_characters"),
//! )
//! ```
//!
//! Paths are relative to the manifest's directory. This adapter lowers the
//! manifest into the composition inputs that already exist: the module files
//! (`ExtensionModuleFiles`), the selected pack (`select_pack`) and the start
//! room (`StartRoomOverride`). It is part of the host: a project implements
//! no `GameModule`.
//!
//! The run: `ambition_app --project path/to/project.ron` (or its directory).

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
    #[serde(default)]
    pub modules: Vec<String>,
    #[serde(default)]
    pub entry_room: Option<String>,
}

/// A manifest with its paths resolved and checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    pub name: String,
    /// The manifest this was read from.
    pub manifest: PathBuf,
    pub content_root: Option<PathBuf>,
    pub module_files: Vec<PathBuf>,
    pub entry_room: Option<String>,
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
        if !problems.is_empty() {
            return Err(format!("{at}:\n  {}", problems.join("\n  ")));
        }
        Ok(Self {
            name: parsed.name,
            manifest: manifest.to_path_buf(),
            content_root,
            module_files,
            entry_room: parsed.entry_room,
        })
    }

    /// Put the project's inputs into `app`, before its plugins build.
    pub fn compose(&self, app: &mut App) -> Result<(), String> {
        if let Some(root) = &self.content_root {
            let pack = ambition_content::pack::compile_pack_from(root)?;
            ambition_content::pack::select_pack(app, std::sync::Arc::new(pack)).map_err(|refused| refused.to_string())?;
        }
        if !self.module_files.is_empty() {
            app.insert_resource(ambition_platformer2d::extension::ExtensionModuleFiles(self.module_files.clone()));
        }
        // A `--start-room` typed beside `--project` is the request.
        #[cfg(not(target_arch = "wasm32"))]
        let typed_room = super::cli::cli_start_room_arg().is_some();
        #[cfg(target_arch = "wasm32")]
        let typed_room = false;
        if let Some(room) = self.entry_room.as_ref().filter(|_| !typed_room) {
            app.insert_resource(super::StartRoomOverride(room.clone()));
            app.insert_resource(super::StartRoomMustResolve);
        }
        Ok(())
    }
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
    fn the_flag_takes_both_spellings() {
        let args = |s: &[&str]| s.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(parse_project_arg(&args(&["--project", "a"])), Some("a".into()));
        assert_eq!(parse_project_arg(&args(&["--headless", "--project=b"])), Some("b".into()));
        assert_eq!(parse_project_arg(&args(&["--headless"])), None);
    }
}

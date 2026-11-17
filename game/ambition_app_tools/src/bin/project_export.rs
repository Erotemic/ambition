//! Export a project through its manifest and the selected library file sets.

use std::{collections::{BTreeMap, BTreeSet}, path::{Component, Path, PathBuf}};
use ambition_app::app::project::{Project, ProjectImplementation, ProjectManifest};
use ambition_platformer2d::{asset_manager::content_library::{ContentLibraries, ContentLibrary}, content::ContentPackDraft};
use clap::Parser;

#[derive(Parser)]
struct Args {
    #[arg(long)]
    project: PathBuf,
    #[arg(long)]
    out: PathBuf,
}

fn main() {
    let args = Args::parse();
    if let Err(reason) = export(&args.project, &args.out) {
        eprintln!("project_export: {reason}");
        std::process::exit(1);
    }
}

fn write(path: &Path, bytes: impl AsRef<[u8]>) -> Result<(), String> {
    std::fs::create_dir_all(path.parent().unwrap()).map_err(|error| error.to_string())?;
    std::fs::write(path, bytes).map_err(|error| format!("{}: {error}", path.display()))
}

fn copy(from: &Path, to: &Path) -> Result<(), String> {
    std::fs::create_dir_all(to.parent().unwrap()).map_err(|error| error.to_string())?;
    std::fs::copy(from, to).map(|_| ()).map_err(|error| format!("{} -> {}: {error}", from.display(), to.display()))
}

fn copy_tree(from: &Path, to: &Path) -> Result<(), String> {
    std::fs::create_dir_all(to).map_err(|error| error.to_string())?;
    for row in std::fs::read_dir(from).map_err(|error| error.to_string())? {
        let row = row.map_err(|error| error.to_string())?;
        let kind = row.file_type().map_err(|error| error.to_string())?;
        let destination = to.join(row.file_name());
        if kind.is_dir() { copy_tree(&row.path(), &destination)?; }
        else if row.path().is_file() { copy(&row.path(), &destination)?; }
        else { return Err(format!("cannot export non-file {}; use a directory with regular files", row.path().display())); }
    }
    Ok(())
}

fn export(input: &Path, out: &Path) -> Result<(), String> {
    let project = Project::load(input)?;
    let content = project.content_root.as_ref()
        .ok_or("export needs an explicit content pack; set `content` in project.ron")?;
    let worlds = project.worlds_root.as_ref()
        .ok_or("export needs an explicit world directory; set `worlds` in project.ron")?;
    let original = ambition_content::pack::compile_pack_from(content)?;
    let mut manifest: ProjectManifest = ron::from_str(&std::fs::read_to_string(&project.manifest)
        .map_err(|error| error.to_string())?).map_err(|error| error.to_string())?;
    let draft = ContentPackDraft::read_from_dir(content).map_err(|error| error.to_string())?;
    let mut pack = draft.manifest.clone();
    if out.exists() { return Err(format!("{} already exists; select a new export directory", out.display())); }
    std::fs::create_dir_all(out).map_err(|error| error.to_string())?;

    let mut seen = BTreeSet::new();
    for (index, mut source) in std::mem::take(&mut pack.sources).into_iter().enumerate() {
        let from = source.resolve_path(content, &draft.manifest)?;
        if !seen.insert(from.canonicalize().map_err(|error| error.to_string())?) { continue; }
        if source.library.is_some() {
            pack.sources.push(source);
            continue;
        }
        let path = Path::new(&source.path);
        let destination = if path.components().all(|part| matches!(part, Component::Normal(_))) {
            path.to_owned()
        } else {
            PathBuf::from(format!("sources/{index:03}")).join(from.file_name().ok_or("a source must name a file")?)
        };
        copy(&from, &out.join("content").join(&destination))?;
        source.path = destination.to_str().ok_or("a source path must be UTF-8")?.to_owned();
        pack.sources.push(source);
    }

    let mut selected = ContentLibraries::default();
    for declared in &draft.manifest.libraries {
        selected.insert(ContentLibrary::read(content.join(declared))?)?;
    }
    let mut libraries = Vec::new();
    for library in selected.iter() {
        let id = &library.manifest.id;
        let mut declaration = library.manifest.clone();
        for (logical, file) in library.files()? {
            let from = file.native_path().ok_or("export needs native library files")?;
            copy(from, &out.join("assets/__libraries").join(id).join(logical))?;
        }
        declaration.asset_roots = [("generated".to_owned(), format!("__libraries/{id}"))].into();
        declaration.publication = None;
        declaration.producers.clear();
        if let Some(path) = &library.manifest.catalog {
            let name = format!("library-{id}-catalog.json");
            copy(&library.root.join(path), &out.join("assets").join(&name))?;
            declaration.catalog = Some(name);
        }
        declaration.provenance = None;
        let name = format!("library-{id}.json");
        write(&out.join("assets").join(&name), serde_json::to_vec_pretty(&declaration).map_err(|error| error.to_string())?)?;
        libraries.push(format!("../assets/{name}"));
    }
    pack.libraries = libraries;
    write(&out.join("content/pack.ron"), ron::ser::to_string_pretty(&pack, Default::default()).map_err(|error| error.to_string())?)?;
    copy_tree(worlds, &out.join("worlds"))?;

    // Keep file stems and declaration order. A script can bind a linked module
    // by its file stem, and the host runs entries in declaration order.
    let mut moved = BTreeMap::new();
    let mut modules = Vec::new();
    for (index, input) in project.module_files.iter().enumerate() {
        if input.is_dir() {
            let directory = format!("modules/{index:03}");
            std::fs::create_dir_all(out.join(&directory)).map_err(|error| error.to_string())?;
            let mut entries = std::fs::read_dir(input).map_err(|error| error.to_string())?
                .map(|entry| entry.map(|entry| entry.path())).collect::<Result<Vec<_>, _>>()
                .map_err(|error| error.to_string())?;
            entries.sort();
            for file in entries {
                if !file.is_file() || !matches!(file.extension().and_then(|extension| extension.to_str()), Some("rhai" | "wasm")) { continue; }
                copy_module(&file, &directory, out, &mut moved)?;
            }
            modules.push(directory);
        } else {
            modules.push(copy_module(input, &format!("modules/{index:03}"), out, &mut moved)?);
        }
    }
    let project_root = project.manifest.parent().unwrap_or(Path::new("."));
    for selection in &mut manifest.implementations {
        if let ProjectImplementation::File(path) = &mut selection.implementation {
            let original = project_root.join(&*path).canonicalize().map_err(|error| error.to_string())?;
            *path = moved.get(&original).ok_or_else(|| format!("selected module {} was not exported", original.display()))?.clone();
        }
    }
    manifest.content = Some("content".into());
    manifest.worlds = Some("worlds".into());
    manifest.modules = modules;
    write(&out.join("project.ron"), ron::ser::to_string_pretty(&manifest, Default::default()).map_err(|error| error.to_string())?)?;
    let exported = Project::load(out)?;
    let compiled = ambition_content::pack::compile_pack_from(exported.content_root.as_ref().unwrap())?;
    if !original.content.iter().map(|(id, row)| (id, &row.canonical))
        .eq(compiled.content.iter().map(|(id, row)| (id, &row.canonical))) {
        return Err("export changed the pack's content definitions; keep source-relative references inside the project or a declared library".into());
    }
    write(&out.join("project-export.json"), serde_json::to_vec_pretty(&serde_json::json!({
        "project": exported.name, "pack": compiled.id.to_string(),
        "implementations": manifest.implementations,
    })).map_err(|error| error.to_string())?)?;
    println!("project: {}; content pack: {}; exported: {}", exported.name, compiled.id, out.display());
    Ok(())
}

fn copy_module(from: &Path, directory: &str, out: &Path, moved: &mut BTreeMap<PathBuf, String>) -> Result<String, String> {
    let canonical = from.canonicalize().map_err(|error| error.to_string())?;
    if let Some(path) = moved.get(&canonical) { return Ok(path.clone()); }
    let path = Path::new(directory).join(from.file_name().ok_or("a module must name a file")?);
    copy(from, &out.join(&path))?;
    let path = path.to_str().ok_or("a module path must be UTF-8")?.to_owned();
    moved.insert(canonical, path.clone());
    Ok(path)
}

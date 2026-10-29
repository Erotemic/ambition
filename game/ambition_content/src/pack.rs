//! Ambition's own content pack: the compile that is the load path.
//!
//! ## One manifest, two source origins
//!
//! A shipped binary must embed its content (a wasm bundle has no asset
//! directory), and the CLI must read a directory. The manifest and the
//! pipeline must be the same for both, so `pack.ron` is embedded from the file
//! the CLI reads, and both go through `ambition_content_pack::compile`.
//!
//! ## Adding a source is one line in `pack.ron`
//!
//! The manifest is the list. A build that reads content off disk reads
//! `pack.ron` and each source it declares when the pack compiles, so a new
//! source is no Rust build. A build that embeds its content (`static_content`)
//! gets the same list from `build.rs`, which reads `pack.ron`. A missing schema
//! registration makes `content_pack_registry` fail before the game runs.

use ambition_content_pack::{CompileFailure, PreparedContentPack};

/// The pack a build with no source tree embeds: `assets/pack.ron` and each
/// `(declared path, text)` it declares, written by `build.rs` under
/// `static_content` (`ambition_content_pack::embed`). `None` in a build that
/// reads its content off disk.
#[cfg(feature = "static_content")]
const EMBEDDED_PACK: Option<(&str, &[(&str, &str)])> = Some(include!(concat!(env!("OUT_DIR"), "/content_pack/assets/embedded.rs")));
#[cfg(not(feature = "static_content"))]
const EMBEDDED_PACK: Option<(&str, &[(&str, &str)])> = None;

use ambition_content_pack::embed::declared_paths;

/// The manifest this build compiles: the embedded one, or `assets/pack.ron`
/// as it is on disk now.
fn manifest_ron() -> String {
    match EMBEDDED_PACK {
        Some((manifest, _)) => manifest.to_string(),
        None => source_text("pack.ron", None),
    }
}

/// One source of the pack: the embedded text, or the file as it is on disk
/// now. `declared` is the path as `pack.ron` spells it.
pub(crate) fn pack_source_text(declared: &str) -> String {
    match EMBEDDED_PACK {
        Some((_, sources)) => sources
            .iter()
            .find(|(path, _)| *path == declared)
            .map(|(_, text)| text.to_string())
            .unwrap_or_else(|| {
                panic!("`{declared}` is not a source `pack.ron` declares, so this build did not embed it")
            }),
        None => source_text(declared, None),
    }
}

/// One declared source's text: embedded when this build baked it in, otherwise
/// read from the same file off disk.
///
/// The declared path is the on-disk path: `pack.ron` spells sources as they
/// sit under this crate's `assets/`, so there is no second location to sync.
///
/// `CARGO_MANIFEST_DIR` is a compile-time string, not a file dependency, so a
/// file change causes no rebuild. See
/// `build.rs`.
///
/// A missing file is fatal. Content that silently lost a family is the
/// "silent partial start" [`compile_pack`] refuses.
pub(crate) fn source_text(declared_path: &str, embedded: Option<&'static str>) -> String {
    if let Some(text) = embedded {
        return text.to_string();
    }
    let path = source_root().join(declared_path);
    std::fs::read_to_string(&path).unwrap_or_else(|err| {
        panic!(
            "content source {declared_path} is neither embedded in this build nor \
             readable at {}: {err}\n\
             Desktop development reads generated content off disk so a regen does \
             not rebuild the crate; build with --features ambition_content/static_content \
             to embed it instead.",
            path.display()
        )
    })
}

/// The directory every declared source is read from when this build does not
/// embed it: the root [`compile_pack_from`] takes to play the files on disk.
pub fn source_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets")
}

/// Every source path `pack.ron` declares, relative to [`source_root`].
pub fn declared_source_paths() -> Vec<String> {
    declared_paths(&manifest_ron())
}

/// Every source path the `pack.ron` under `root` declares. None when that
/// manifest is absent or does not parse; [`compile_pack_from`] says why.
pub fn declared_source_paths_in(root: &std::path::Path) -> Vec<String> {
    std::fs::read_to_string(root.join("pack.ron"))
        .map(|manifest| declared_paths(&manifest))
        .unwrap_or_default()
}

/// Every source `pack.ron` declares, paired with its text.
fn embedded_sources() -> Vec<(String, String)> {
    declared_source_paths()
        .into_iter()
        .map(|path| {
            let text = pack_source_text(&path);
            (path, text)
        })
        .collect()
}

/// The schemas Ambition's own pack is compiled against: the engine's, and
/// Ambition's own (`ambition_content_schemas`, the list its check binary gives
/// the content CLI).
///
/// The one answer to which crates own which schemas.
pub fn pack_schemas() -> ambition_content_pack::SchemaRegistry {
    let mut schemas = ambition_platformer2d::content::engine_schemas();
    for schema in ambition_content_schemas::ambition_schemas() {
        schemas
            .register(schema)
            .unwrap_or_else(|refused| panic!("an Ambition schema has an engine schema's id: {refused:?}"));
    }
    schemas
}

/// Compile Ambition's embedded pack.
///
/// Assets are not checked here, on purpose. Art may be absent on a fresh
/// clone (AGENTS.md: git-ignored payloads, "degrade visibly when a file is
/// absent"), and this compiler must explain a missing sheet, not stop the game.
/// The CLI's strict mode is where art is a gate.
pub fn compile_pack() -> Result<PreparedContentPack, CompileFailure> {
    compile_pack_with(|_, text| text)
}

/// The same compile, with every source passed through `edit` first.
///
/// One compile road, so a second pack is the same pack with an edit. A test
/// that assembled its own draft would compare two compiles, not two contents:
/// a manifest or schema difference would look like a content difference.
///
/// `edit` sees the declared path and the source text; returning the text
/// unchanged gives the shipped pack.
pub fn compile_pack_with(
    mut edit: impl FnMut(&str, String) -> String,
) -> Result<PreparedContentPack, CompileFailure> {
    // the manifest is a DIAGNOSTIC, not a panic, since gave the compiler its own
    // embedded-pack road.
    let sources: Vec<(String, String)> = embedded_sources()
        .into_iter()
        .map(|(path, text)| {
            let edited = edit(&path, text);
            (path, edited)
        })
        .collect();
    let mut draft = ambition_content_pack::ContentPackDraft::from_manifest_ron(&manifest_ron(), sources)?;
    if EMBEDDED_PACK.is_none() { draft.root = source_root(); }
    ambition_content_pack::compile(
        &draft,
        &pack_schemas(),
        &ambition_content_pack::AssetsUnchecked,
    )
}

/// Compile `manifest` with the text `text_of` gives for each source it
/// declares.
fn compile_manifest(
    root: &std::path::Path,
    manifest: &str,
    mut text_of: impl FnMut(&str) -> String,
) -> Result<PreparedContentPack, CompileFailure> {
    let sources: Vec<(String, String)> = declared_paths(manifest)
        .into_iter()
        .map(|path| {
            let text = text_of(&path);
            (path, text)
        })
        .collect();
    let mut draft = ambition_content_pack::ContentPackDraft::from_manifest_ron(manifest, sources)?;
    draft.root = root.to_path_buf();
    ambition_content_pack::compile(&draft, &pack_schemas(), &ambition_content_pack::AssetsUnchecked)
}

/// The same compile, with the manifest no longer declaring `omit`.
///
/// `compile_pack_with` edits source text and cannot change the declaration
/// list, so it cannot express a candidate that DROPS a family. This one does:
/// each omitted path loses its manifest line and its source. For a witness of
/// what a reload does when N+1 stops declaring something N declared.
///
/// A path that no manifest line names is refused, so a renamed source cannot
/// turn a witness into a no-op.
pub fn compile_pack_omitting(omit: &[&str]) -> Result<PreparedContentPack, CompileFailure> {
    compile_pack_omitting_with(omit, |_, text| text)
}

/// [`compile_pack_omitting`] with an edit to each remaining source's text, as
/// [`compile_pack_with`] does: a candidate that changes one family and drops
/// another.
pub fn compile_pack_omitting_with(
    omit: &[&str],
    mut edit: impl FnMut(&str, String) -> String,
) -> Result<PreparedContentPack, CompileFailure> {
    let mut remaining = omit.to_vec();
    let manifest: String = manifest_ron()
        .lines()
        .filter(|line| {
            let named = omit.iter().position(|path| line.contains(&format!("path: \"{path}\"")));
            if let Some(index) = named {
                remaining.retain(|path| *path != omit[index]);
            }
            named.is_none()
        })
        .map(|line| format!("{line}\n"))
        .collect();
    assert!(
        remaining.is_empty(),
        "no manifest line declares {remaining:?}, so the pack would omit nothing"
    );
    let sources: Vec<(String, String)> = embedded_sources()
        .into_iter()
        .filter(|(path, _)| !omit.contains(&path.as_str()))
        .map(|(path, text)| {
            let text = edit(&path, text);
            (path, text)
        })
        .collect();
    let draft = ambition_content_pack::ContentPackDraft::from_manifest_ron(&manifest, sources)?;
    ambition_content_pack::compile(
        &draft,
        &pack_schemas(),
        &ambition_content_pack::AssetsUnchecked,
    )
}

/// Compile the pack reading every declared source from `root`.
///
/// This lets a prebuilt host play an edited file (fast-iteration I2). The
/// content directory is an argument, not `env!("CARGO_MANIFEST_DIR")` baked at
/// build time.
///
/// Every missing file is named, and none is inherited from the binary. A
/// per-file fallback would compile a mixed pack, and which half you played
/// would depend on which files existed. A root supplies the whole pack or
/// fails. The list is the root's own `pack.ron`: a source declared there is
/// played with no build.
pub fn compile_pack_from(root: &std::path::Path) -> Result<PreparedContentPack, String> {
    let manifest_path = root.join("pack.ron");
    let manifest = std::fs::read_to_string(&manifest_path)
        .map_err(|err| format!("{} does not supply this pack: {}: {err}", root.display(), manifest_path.display()))?;
    let mut missing: Vec<String> = Vec::new();
    let mut unreadable: Vec<String> = Vec::new();
    let compiled = compile_manifest(root, &manifest, |declared| {
        let path = root.join(declared);
        match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                missing.push(declared.to_string());
                String::new()
            }
            Err(err) => {
                unreadable.push(format!("{declared}: {err}"));
                String::new()
            }
        }
    });
    // Report the root's own problems before the compiler's. An absent file
    // reaches the compiler as an empty source, whose diagnostic is a parse error
    // at byte 0: true, but useless to someone who mistyped a directory.
    if !missing.is_empty() || !unreadable.is_empty() {
        let mut report = format!("{} does not supply this pack:", root.display());
        for path in &missing {
            report.push_str(&format!("\n  missing: {path}"));
        }
        for problem in &unreadable {
            report.push_str(&format!("\n  unreadable: {problem}"));
        }
        return Err(report);
    }
    compiled.map_err(|failure| failure.to_string())
}

/// Write every source this build carries into `root`, at its declared path.
///
/// The other half of the loop, so [`compile_pack_from`] can refuse a partial
/// root and still be usable: start from what the binary has, edit one file,
/// point the host at it.
pub fn export_sources_to(root: &std::path::Path) -> std::io::Result<usize> {
    let mut manifest: ambition_content_pack::ContentPackManifest = ron::from_str(&manifest_ron())
        .map_err(std::io::Error::other)?;
    for library in &mut manifest.libraries {
        *library = source_root().join(&*library).canonicalize()?.to_string_lossy().into_owned();
    }
    let manifest = ron::ser::to_string_pretty(&manifest, ron::ser::PrettyConfig::default().depth_limit(2))
        .map_err(std::io::Error::other)?;
    std::fs::create_dir_all(root)?;
    std::fs::write(root.join("pack.ron"), manifest)?;
    let mut written = 0;
    for (declared, text) in embedded_sources() {
        let path = root.join(&declared);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, text)?;
        written += 1;
    }
    Ok(written)
}

/// Ambition's canonical SHIPPED pack, compiled once per process.
///
/// ⛔ BOOT-SCOPED INSPECTION ONLY. The subject of a caller is "the pack this
/// product ships": a source-content test, an offline validator, a tool. It is
/// NOT "the pack of this App". An App's content is whatever
/// [`SelectedContentPack`] holds: read it with [`selected`] (or take
/// `Res<SelectedContentPack>` in a system), and install it with [`select`]
/// during composition. A runtime reader that calls this function gives every
/// App in the process the first App's content.
///
/// The name says this on purpose. `crate::pack_selection_tests` scans the
/// production source and refuses a call outside its short, reasoned list.
///
/// Fails loudly: a silent partial start (content that lost a character or an
/// item) would be worse.
pub fn shipped() -> &'static PreparedContentPack {
    boot_pack()
}

/// The process's boot pack, behind an `Arc` so an App can hold it without a
/// second compile.
///
/// Private. [`shipped`] is the read for a caller whose subject is the shipped
/// product; [`select`] is the only road by which an App gets this value, and
/// only when nothing chose a pack for it. Handing out the `Arc` would make
/// "which pack is this App's" answerable from anywhere.
fn boot_pack() -> &'static std::sync::Arc<PreparedContentPack> {
    static PREPARED: std::sync::OnceLock<std::sync::Arc<PreparedContentPack>> =
        std::sync::OnceLock::new();
    PREPARED.get_or_init(|| {
        std::sync::Arc::new(compile_pack().unwrap_or_else(|failure| {
            panic!("Ambition's own content pack does not compile:\n{failure}")
        }))
    })
}

/// This App's content pack (fast-iteration I3, step 1).
///
/// A process-global `OnceLock` cannot be re-selected: two Apps in one process
/// would share one pack, a reload would have nowhere to put a new one, and a
/// test could not give a composition its own content. I3's acceptance: *"Two
/// Apps can select different packs without contamination."*
///
/// Immutable data is shared; activation authority is not. The `Arc` means
/// selecting the boot pack costs no second compile. What is App-scoped is
/// which pack this App answers with.
#[derive(bevy::prelude::Resource, Clone)]
pub struct SelectedContentPack(pub std::sync::Arc<PreparedContentPack>);

impl SelectedContentPack {
    pub fn get(&self) -> &PreparedContentPack {
        &self.0
    }
}

/// The canonical identity line, not only the fingerprint: a desync report
/// needs the pack's id and version beside the hex digest.
pub(crate) fn identity_line(pack: &PreparedContentPack) -> String {
    format!("{} {} {}", pack.id, pack.version, pack.fingerprint)
}

/// Why [`select_pack`] refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionRefused {
    /// The pack this App already installed content from.
    pub in_use: String,
    /// The pack the caller asked for.
    pub requested: String,
}

impl std::fmt::Display for SelectionRefused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "this App already installed content from pack [{}]; selecting [{}] now would \
             name one generation while the installed families serve another. Select before \
             composing, or reload through `crate::reload`",
            self.in_use, self.requested
        )
    }
}

/// Marks a selection as consumed: some App-owned content reader ([`select`])
/// derived a family from it. Private, so only this module can seal.
#[derive(bevy::prelude::Resource)]
struct SelectionSealed;

/// Give this App a pack of its own, BEFORE any content is installed from it.
///
/// Selection is an input to composition, not a way to revise a composition.
/// The first [`select`] (every plugin `build` and `register` calls it) seals
/// the selection. After that, choosing a DIFFERENT pack is refused and changes
/// nothing: it would leave `SelectedContentPack` and the published identity
/// naming N+1 while the installed families serve N, and a lazy reader (the
/// quest book) would then read N+1. Choosing the same pack again is harmless.
///
/// Before the seal a selection may be replaced freely. Revising a sealed App
/// is [`crate::reload`]'s job, at its activation boundary, through the private
/// `install_selection`.
pub fn select_pack(
    app: &mut bevy::prelude::App,
    pack: std::sync::Arc<PreparedContentPack>,
) -> Result<(), SelectionRefused> {
    let world = app.world_mut();
    if world.contains_resource::<SelectionSealed>() {
        let in_use = world.resource::<SelectedContentPack>();
        if !std::sync::Arc::ptr_eq(&in_use.0, &pack) && in_use.0.fingerprint != pack.fingerprint {
            return Err(SelectionRefused {
                in_use: identity_line(&in_use.0),
                requested: identity_line(&pack),
            });
        }
        return Ok(());
    }
    install_selection(world, pack);
    Ok(())
}

/// Install a selection and publish its identity to the engine.
///
/// The engine is told which pack, not given the pack. Pack compilation stays
/// here; `ambition_platformer2d_provider` needs only the pack's identity to
/// fingerprint a prepared session, so identity and selection travel
/// together.
///
/// Without this, two sessions prepared under different move tables would share
/// one `PreparedContentIdentity`, and the rollback contract that refuses
/// *"prepared content changed while the session was active"* compares exactly
/// that identity.
pub(crate) fn install_selection(
    world: &mut bevy::ecs::world::World,
    pack: std::sync::Arc<PreparedContentPack>,
) {
    #[cfg(feature = "static_content")]
    let embedded = pack.root.as_os_str().is_empty().then_some(include!(concat!(env!("OUT_DIR"), "/content_pack/assets/libraries.rs")) as &'static [ambition_platformer2d::asset_manager::content_library::EmbeddedLibrary]);
    #[cfg(not(feature = "static_content"))]
    let embedded = None;
    ambition_platformer2d::content::libraries::install(world, &pack.root, &pack.libraries, embedded)
        .unwrap_or_else(|error| panic!("Content libraries: {error}"));
    world.insert_resource(ambition_platformer2d_runtime::SelectedContentIdentity(
        identity_line(&pack),
    ));
    world.insert_resource(SelectedContentPack(pack));
}

/// This App's pack, selecting the process's boot pack if nothing chose one.
///
/// Every App-owned install (a plugin's `build`, a `register` function) calls
/// this once and derives its family from the result. The first caller in an
/// App fixes the selection for the others, so no two families of one App can
/// read different packs.
///
/// The fallback is an insert, not a read-through. A read-through would answer
/// from the boot pack while the App believed it had a selection, so later
/// `selected` calls could disagree with the first.
///
/// Selection is not publication, and a consumed selection is sealed: a later
/// [`select_pack`] of another pack is refused.
pub fn select(world: &mut bevy::ecs::world::World) -> std::sync::Arc<PreparedContentPack> {
    if !world.contains_resource::<SelectedContentPack>() {
        install_selection(world, std::sync::Arc::clone(boot_pack()));
    }
    // Consuming the selection seals it: see [`select_pack`].
    world.insert_resource(SelectionSealed);
    std::sync::Arc::clone(&world.resource::<SelectedContentPack>().0)
}

/// This App's pack, or `None` when nothing has selected one.
pub fn selected(world: &bevy::ecs::world::World) -> Option<&PreparedContentPack> {
    world
        .get_resource::<SelectedContentPack>()
        .map(SelectedContentPack::get)
}

#[cfg(test)]
#[path = "pack_selection_tests.rs"]
mod pack_selection_tests;

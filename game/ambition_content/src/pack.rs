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

use ambition_content_pack::{ContentPackManifest, Diagnostic};

/// The manifest this build compiles: the embedded one, or `assets/pack.ron`
/// as it is on disk now.
fn manifest_ron() -> String {
    match EMBEDDED_PACK {
        Some((manifest, _)) => manifest.to_string(),
        None => source_text("pack.ron", None),
    }
}

/// The manifest this build can play, and the warnings from its reading.
///
/// A build that reads its sources off disk removes each generated library
/// source that is absent (the music registry before the music is rendered)
/// and keeps one warning for each
/// (`ContentPackManifest::take_ungenerated`). The pack then compiles without
/// that family, and the game runs without it. A build that embeds its
/// sources has each one, because `build.rs` refuses to embed a file that is
/// absent.
fn playable_manifest() -> Result<(ContentPackManifest, Vec<Diagnostic>), ambition_content_pack::CompileFailure> {
    let mut manifest = ContentPackManifest::from_ron(&manifest_ron())?;
    let warnings = match EMBEDDED_PACK {
        Some(_) => Vec::new(),
        None => manifest.take_ungenerated(&source_root()),
    };
    Ok((manifest, warnings))
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
/// A source that names a library is read from that library.
///
/// `CARGO_MANIFEST_DIR` is a compile-time string, not a file dependency, so a
/// file change causes no rebuild. See
/// `build.rs`.
///
/// ## Absent files
///
/// * An AUTHORED source that is absent is fatal. It is an authoring bug, and
///   content that lost a family without a word is the "silent partial start"
///   [`compile_pack`] refuses.
/// * A GENERATED library source (a producer makes it, for example the music
///   registry) is not fatal. The compile does not ask for it when it is
///   absent: [`playable_manifest`] removes it with a loud warning, and the
///   game runs without that family. Only a caller that asks for it by name
///   gets here, and the panic names the command that makes it.
///   `python3 scripts/content_status.py` tells what is absent without a
///   build.
pub(crate) fn source_text(declared_path: &str, embedded: Option<&'static str>) -> String {
    if let Some(text) = embedded {
        return text.to_string();
    }
    let root = source_root();
    let manifest: ContentPackManifest = ron::from_str(
        &std::fs::read_to_string(root.join("pack.ron")).expect("pack manifest")
    ).expect("pack manifest");
    let path = manifest.sources.iter().find(|source| source.path == declared_path)
        .map(|source| source.resolve_path(&root, &manifest)
            .unwrap_or_else(|error| panic!("content source {declared_path}: {error}")))
        .unwrap_or_else(|| root.join(declared_path));
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

/// Every source path this build plays, relative to [`source_root`]: each one
/// `pack.ron` declares, less the generated sources that are absent.
pub fn declared_source_paths() -> Vec<String> {
    playable_manifest()
        .map(|(manifest, _)| manifest.sources.into_iter().map(|source| source.path).collect())
        .unwrap_or_default()
}

/// Every source path the `pack.ron` under `root` declares. None when that
/// manifest is absent or does not parse; [`compile_pack_from`] says why.
pub fn declared_source_paths_in(root: &std::path::Path) -> Vec<String> {
    std::fs::read_to_string(root.join("pack.ron"))
        .map(|manifest| ambition_content_pack::embed::declared_paths(&manifest))
        .unwrap_or_default()
}

/// Every source `manifest` declares, paired with its text.
fn embedded_sources(manifest: &ContentPackManifest) -> Vec<(String, String)> {
    manifest
        .sources
        .iter()
        .map(|source| (source.path.clone(), pack_source_text(&source.path)))
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
/// The CLI's strict mode is where art is a gate. For the same reason, a
/// generated library source that is absent is a warning on the prepared pack
/// ([`playable_manifest`]), not a refusal.
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
    let (manifest, warnings) = playable_manifest()?;
    let sources: Vec<(String, String)> = embedded_sources(&manifest)
        .into_iter()
        .map(|(path, text)| {
            let edited = edit(&path, text);
            (path, edited)
        })
        .collect();
    let mut draft = ambition_content_pack::ContentPackDraft::from_sources(manifest, sources)?;
    draft.warnings = warnings;
    if EMBEDDED_PACK.is_none() { draft.root = source_root(); }
    ambition_content_pack::compile(
        &draft,
        &pack_schemas(),
        &ambition_content_pack::AssetsUnchecked,
    )
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
    let declared = ContentPackManifest::from_ron(&manifest_ron())?;
    let remaining: Vec<&&str> = omit
        .iter()
        .filter(|path| !declared.sources.iter().any(|source| source.path == **path))
        .collect();
    assert!(
        remaining.is_empty(),
        "no manifest line declares {remaining:?}, so the pack would omit nothing"
    );
    let (mut manifest, warnings) = playable_manifest()?;
    manifest.sources.retain(|source| !omit.contains(&source.path.as_str()));
    let sources: Vec<(String, String)> = embedded_sources(&manifest)
        .into_iter()
        .map(|(path, text)| {
            let text = edit(&path, text);
            (path, text)
        })
        .collect();
    let mut draft = ambition_content_pack::ContentPackDraft::from_sources(manifest, sources)?;
    draft.warnings = warnings;
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
/// played with no build. The one exception is a generated library source
/// that is absent: the pack compiles without it, with a warning, as
/// [`compile_pack`] does.
pub fn compile_pack_from(root: &std::path::Path) -> Result<PreparedContentPack, String> {
    let manifest_path = root.join("pack.ron");
    let manifest = std::fs::read_to_string(&manifest_path)
        .map_err(|err| format!("{} does not supply this pack: {}: {err}", root.display(), manifest_path.display()))?;
    let mut manifest = ContentPackManifest::from_ron(&manifest).map_err(|failure| failure.to_string())?;
    let warnings = manifest.take_ungenerated(root);
    let mut missing: Vec<String> = Vec::new();
    let mut unreadable: Vec<String> = Vec::new();
    let mut sources: Vec<(String, String)> = Vec::new();
    for source in &manifest.sources {
        let path = match source.resolve_path(root, &manifest) {
            Ok(path) => path,
            Err(err) => {
                unreadable.push(format!("{}: {err}", source.path));
                continue;
            }
        };
        match std::fs::read_to_string(&path) {
            Ok(text) => sources.push((source.path.clone(), text)),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => missing.push(source.path.clone()),
            Err(err) => unreadable.push(format!("{}: {err}", source.path)),
        }
    }
    // Report the root's own problems before the compiler's. An absent file
    // is a missing source, not a parse error at byte 0 of an empty text: the
    // second is true, but useless to someone who mistyped a directory.
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
    let mut draft = ambition_content_pack::ContentPackDraft::from_sources(manifest, sources)
        .map_err(|failure| failure.to_string())?;
    draft.root = root.to_path_buf();
    draft.warnings = warnings;
    ambition_content_pack::compile(&draft, &pack_schemas(), &ambition_content_pack::AssetsUnchecked)
        .map_err(|failure| failure.to_string())
}

/// Write every source this build carries into `root`, at its declared path.
///
/// The other half of the loop, so [`compile_pack_from`] can refuse a partial
/// root and still be usable: start from what the binary has, edit one file,
/// point the host at it. A generated source that is absent is not exported.
/// Its declaration keeps its library, so the exported root reads it from the
/// library when it is generated, and compiles without it until then.
pub fn export_sources_to(root: &std::path::Path) -> std::io::Result<usize> {
    let as_io = |failure: CompileFailure| std::io::Error::other(failure.to_string());
    let (playable, _) = playable_manifest().map_err(as_io)?;
    let sources = embedded_sources(&playable);
    let mut manifest = ContentPackManifest::from_ron(&manifest_ron()).map_err(as_io)?;
    for library in &mut manifest.libraries {
        *library = source_root().join(&*library).canonicalize()?.to_string_lossy().into_owned();
    }
    for source in &mut manifest.sources {
        if playable.sources.iter().any(|exported| exported.path == source.path) {
            source.library = None;
        }
    }
    let manifest = ron::ser::to_string_pretty(&manifest, ron::ser::PrettyConfig::default().depth_limit(2))
        .map_err(std::io::Error::other)?;
    std::fs::create_dir_all(root)?;
    std::fs::write(root.join("pack.ron"), manifest)?;
    let mut written = 0;
    for (declared, text) in sources {
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

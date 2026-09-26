//! Ambition's own content pack: the compile that is the load path.
//!
//! ## One manifest, two source origins
//!
//! A shipped binary must embed its content (a wasm bundle has no asset
//! directory), and the CLI must read a directory. The manifest and the
//! pipeline must be the same for both, so `pack.ron` is embedded from the file
//! the CLI reads, and both go through `ambition_content_pack::compile`.
//!
//! ## Adding a source is two edits here, and both fail loudly
//!
//! A new line in `pack.ron` needs a matching entry in [`embedded_sources`].
//! Without it, the compiler refuses at startup with "the manifest declares `X`
//! but no source was supplied for it". A missing schema registration makes
//! `content_pack_registry` fail before the game runs.

use ambition_content_pack::{CompileFailure, PreparedContentPack};

/// The pack manifest, embedded from the SAME file the CLI reads off disk.
const PACK_MANIFEST_RON: &str = include_str!("../assets/pack.ron");

/// The declared path of each source, exactly as `pack.ron` spells it.
///
/// A mismatch is caught by the compiler's "no source supplied" refusal, not
/// by an empty family.
pub(crate) const CATALOG_SOURCE_PATH: &str = "data/character_catalog.ron";
const ITEMS_SOURCE_PATH: &str = "data/items.ron";
const BOSS_PROFILES_SOURCE_PATH: &str = "data/boss_profiles.ron";
const BOSS_SEEDS_SOURCE_PATH: &str = "data/boss_seeds.ron";
const BOSS_VALIDATOR_BANDS_SOURCE_PATH: &str = "data/boss_validator_bands.ron";
const FIGHTER_BRAIN_LADDER_SOURCE_PATH: &str = "data/fighter_brain_ladder.ron";
const MUSIC_REGISTRY_SOURCE_PATH: &str = "audio/music_registry.ron";
const SFX_REGISTRY_SOURCE_PATH: &str = "audio/sfx_registry.ron";

/// The authored encounter wave timelines.
const ENCOUNTER_WAVES_SOURCE_PATH: &str = "data/encounters/goblin_encounter.ron";

/// The authored item grid (compile-time include; the loose file stays on disk
/// so the CLI and the Python tooling read the same bytes).
pub const ITEMS_RON: &str = include_str!("../assets/data/items.ron");

/// Every move table this provider authors, as content, not code.
///
/// One list: the path and embedded static derive from it. `pack.ron` stays a
/// separate statement (it is the pack's manifest, not this crate's), and the
/// compiler's "no source was supplied" refusal keeps them in step at startup.
///
/// Gated like the music registry: off for desktop development, so a move edit
/// costs a file write and a pack recompile instead of a Rust rebuild; on for
/// builds with no filesystem (web, Android).
/// `dev/measurements/m0_move_edit_loop.sh` measures the edit loop.
///
/// The name here is the file's, not the character's: a file names the
/// characters it is for by entity id.
#[cfg(feature = "static_content")]
const MIGRATED_MOVESETS: &[(&str, Option<&'static str>)] = &[
    (
        "alice",
        Some(include_str!("../assets/data/movesets/alice.ron")),
    ),
    ("bob", Some(include_str!("../assets/data/movesets/bob.ron"))),
    (
        "carl_stargan",
        Some(include_str!("../assets/data/movesets/carl_stargan.ron")),
    ),
    (
        "cellular_automaton",
        Some(include_str!(
            "../assets/data/movesets/cellular_automaton.ron"
        )),
    ),
    (
        "director",
        Some(include_str!("../assets/data/movesets/director.ron")),
    ),
    (
        "emmy_noether",
        Some(include_str!("../assets/data/movesets/emmy_noether.ron")),
    ),
    (
        "goblin",
        Some(include_str!("../assets/data/movesets/goblin.ron")),
    ),
    (
        "imperfect_cellular_automaton",
        Some(include_str!(
            "../assets/data/movesets/imperfect_cellular_automaton.ron"
        )),
    ),
    (
        "medic",
        Some(include_str!("../assets/data/movesets/medic.ron")),
    ),
    (
        "ninja_shadow_oni_leader",
        Some(include_str!(
            "../assets/data/movesets/ninja_shadow_oni_leader.ron"
        )),
    ),
    (
        "officer",
        Some(include_str!("../assets/data/movesets/officer.ron")),
    ),
    (
        "oiler",
        Some(include_str!("../assets/data/movesets/oiler.ron")),
    ),
    (
        "patent_clerk",
        Some(include_str!("../assets/data/movesets/patent_clerk.ron")),
    ),
    (
        "performer",
        Some(include_str!("../assets/data/movesets/performer.ron")),
    ),
    (
        "pirate_admiral",
        Some(include_str!("../assets/data/movesets/pirate_admiral.ron")),
    ),
    (
        "player_robot",
        Some(include_str!("../assets/data/movesets/player_robot.ron")),
    ),
    (
        "pointed_polygon",
        Some(include_str!("../assets/data/movesets/pointed_polygon.ron")),
    ),
    (
        "projectile_polygon",
        Some(include_str!(
            "../assets/data/movesets/projectile_polygon.ron"
        )),
    ),
    (
        "pugnacious_polygon",
        Some(include_str!(
            "../assets/data/movesets/pugnacious_polygon.ron"
        )),
    ),
];
#[cfg(not(feature = "static_content"))]
const MIGRATED_MOVESETS: &[(&str, Option<&'static str>)] = &[
    ("alice", None),
    ("bob", None),
    ("carl_stargan", None),
    ("cellular_automaton", None),
    ("director", None),
    ("emmy_noether", None),
    ("goblin", None),
    ("imperfect_cellular_automaton", None),
    ("medic", None),
    ("ninja_shadow_oni_leader", None),
    ("officer", None),
    ("oiler", None),
    ("patent_clerk", None),
    ("performer", None),
    ("pirate_admiral", None),
    ("player_robot", None),
    ("pointed_polygon", None),
    ("projectile_polygon", None),
    ("pugnacious_polygon", None),
];

/// The authored fighter difficulty ladder.
///
/// Declared here so the game reads it as content, instead of consulting the
/// `FighterBrainProfile::for_level` floor.
pub const FIGHTER_BRAIN_LADDER_RON: &str = include_str!("../assets/data/fighter_brain_ladder.ron");

/// One declared source's text: embedded when this build baked it in, otherwise
/// read from the same file off disk.
///
/// The declared path is the on-disk path: `pack.ron` spells sources as they
/// sit under this crate's `assets/`, so there is no second location to sync.
///
/// `CARGO_MANIFEST_DIR` is a compile-time string, not a file dependency, so a
/// file change causes no rebuild. See
/// [`crate::audio_registries::MUSIC_REGISTRY_RON_STATIC`].
///
/// A missing file is fatal. Content that silently lost a family is the
/// "silent partial start" [`compile_pack`] refuses.
fn source_text(declared_path: &str, embedded: Option<&'static str>) -> String {
    if let Some(text) = embedded {
        return text.to_string();
    }
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join(declared_path);
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

/// Every source `pack.ron` declares, paired with its embedded text.
fn embedded_sources() -> impl IntoIterator<Item = (String, String)> {
    // Boss encounters come from one table, `bosses::BOSS_ENCOUNTERS`, where path
    // and bytes travel together. An index into a separate list could attach one
    // file's contents to another file's path without any error, because the
    // runtime resolves rows by their internal ids.
    let mut sources: Vec<(String, String)> = vec![
        (
            CATALOG_SOURCE_PATH.to_string(),
            crate::character_catalog::CHARACTER_CATALOG_RON.to_string(),
        ),
        (ITEMS_SOURCE_PATH.to_string(), ITEMS_RON.to_string()),
        (
            ENCOUNTER_WAVES_SOURCE_PATH.to_string(),
            crate::ENCOUNTER_WAVES_RON.to_string(),
        ),
        (
            FIGHTER_BRAIN_LADDER_SOURCE_PATH.to_string(),
            FIGHTER_BRAIN_LADDER_RON.to_string(),
        ),
        (
            BOSS_PROFILES_SOURCE_PATH.to_string(),
            crate::bosses::BOSS_PROFILES_RON.to_string(),
        ),
        (
            BOSS_SEEDS_SOURCE_PATH.to_string(),
            crate::bosses::BOSS_SEEDS_RON.to_string(),
        ),
        (
            BOSS_VALIDATOR_BANDS_SOURCE_PATH.to_string(),
            crate::bosses::BOSS_VALIDATOR_BANDS_RON.to_string(),
        ),
        (
            MUSIC_REGISTRY_SOURCE_PATH.to_string(),
            source_text(
                MUSIC_REGISTRY_SOURCE_PATH,
                crate::audio_registries::MUSIC_REGISTRY_RON_STATIC,
            ),
        ),
        (
            SFX_REGISTRY_SOURCE_PATH.to_string(),
            crate::audio_registries::SFX_REGISTRY_RON.to_string(),
        ),
    ];
    sources.extend(MIGRATED_MOVESETS.iter().map(|(table, embedded)| {
        let path = format!("data/movesets/{table}.ron");
        let text = source_text(&path, *embedded);
        (path, text)
    }));
    sources.extend(
        crate::bosses::BOSS_ENCOUNTERS
            .iter()
            .map(|(path, ron)| ((*path).to_string(), (*ron).to_string())),
    );
    sources
}

/// The schemas Ambition's own pack is compiled against.
///
/// The one answer to which crates own which schemas.
pub fn pack_schemas() -> ambition_content_pack::SchemaRegistry {
    ambition_platformer2d::content::engine_schemas()
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
    let draft =
        ambition_content_pack::ContentPackDraft::from_manifest_ron(PACK_MANIFEST_RON, sources)?;
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
/// fails.
pub fn compile_pack_from(root: &std::path::Path) -> Result<PreparedContentPack, String> {
    let mut missing: Vec<String> = Vec::new();
    let mut unreadable: Vec<String> = Vec::new();
    let compiled = compile_pack_with(|declared, _built_in| {
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

/// The prepared pack, compiled once per process.
///
/// Every family's install reads this one value. Compiling per family would
/// multiply the cost and let two families disagree about their pack.
///
/// Fails loudly: a silent partial start (content that lost a character or an
/// item) would be worse.
pub fn prepared() -> &'static PreparedContentPack {
    boot_pack()
}

/// The process's boot pack, behind an `Arc` so an App can hold it without a
/// second compile.
///
/// Private. [`prepared`] is the read for a family not yet migrated to
/// App-scoped selection; [`selected`] is the read for one that is. Handing out
/// the `Arc` would make "which pack is this App's" answerable from anywhere.
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

/// Give this App a pack of its own, replacing any previous selection.
///
/// Selection is not publication. Installing a pack here changes what later
/// reads answer; it does not revise a cast already published. That is
/// [`crate::reload`]'s job, so a reload can refuse without having already
/// replaced the App's content.
pub fn select_pack(app: &mut bevy::prelude::App, pack: std::sync::Arc<PreparedContentPack>) {
    install_selection(app.world_mut(), pack);
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
    world.insert_resource(ambition_platformer2d_runtime::SelectedContentIdentity(
        identity_line(&pack),
    ));
    world.insert_resource(SelectedContentPack(pack));
}

/// This App's pack, selecting the process's boot pack if nothing chose one.
///
/// The fallback is an insert, not a read-through. A read-through would answer
/// from the boot pack while the App believed it had a selection, so later
/// `selected` calls could disagree with the first.
pub fn select(world: &mut bevy::ecs::world::World) -> &PreparedContentPack {
    if !world.contains_resource::<SelectedContentPack>() {
        install_selection(world, std::sync::Arc::clone(boot_pack()));
    }
    world.resource::<SelectedContentPack>().get()
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

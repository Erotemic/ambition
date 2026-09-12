//! Ambition's own content pack — the compile that IS the load path.
//!
//! ## One manifest, two source origins
//!
//! A shipped binary must EMBED its content — there is no asset directory
//! beside a wasm bundle — and the CLI must READ a directory. What must not
//! differ is the manifest that says what the pack IS, or the pipeline that
//! judges it. So `pack.ron` is embedded from the same file the CLI reads, and
//! both go through `ambition_content_pack::compile`.
//!
//! ## Adding a source is TWO edits here, and both fail loudly
//!
//! A new line in `pack.ron` needs a matching entry in [`embedded_sources`]. Miss
//! it and the compiler refuses with "the manifest declares `X` but no source was
//! supplied for it" at startup — not a silent empty family. Miss the schema
//! registration instead and `content_pack_registry` goes red before you ever run
//! the game.

use ambition_content_pack::{CompileFailure, PreparedContentPack};

/// The pack manifest, embedded from the SAME file the CLI reads off disk.
const PACK_MANIFEST_RON: &str = include_str!("../assets/pack.ron");

/// The declared path of each source, exactly as `pack.ron` spells it.
///
/// A mismatch is caught by the compiler's own "no source supplied" refusal
/// rather than by silently loading an empty family.
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

/// Every move table this provider authors, as CONTENT rather than code.
///
/// ⭐⭐ ONE LIST, because "which tables are migrated" was three answers per table
/// — a path const, an embedded static and a `pack.ron` line — and two of the
/// three are derivable from the first. `pack.ron` stays its own statement (it is
/// the PACK's manifest, not this crate's), and the compiler's *"the manifest
/// declares X but no source was supplied for it"* refusal is what keeps the two
/// in step, loudly, at startup.
///
/// ⭐ GATED THE WAY THE MUSIC REGISTRY IS: OFF for desktop development so a move
/// edit costs a file write and a pack recompile rather than a Rust rebuild and a
/// link, ON for builds with no filesystem to read from (web, Android). That gate
/// is what makes fast-iteration I2's claim literal — MEASURED at 0.61 s against
/// 6.30 s for the same edit through the Rust table
/// (`dev/measurements/m0_move_edit_loop.sh`).
///
/// ⛔ THE NAME HERE IS THE TABLE's, NOT THE CHARACTER'S. See
/// `crate::authored_movesets::TABLE_CHARACTERS`.
#[cfg(feature = "static_content")]
const MIGRATED_MOVESETS: &[(&str, Option<&'static str>)] = &[
    (
        "alice",
        Some(include_str!("../assets/data/movesets/alice.ron")),
    ),
    (
        "author",
        Some(include_str!("../assets/data/movesets/author.ron")),
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
        "emmy_noether",
        Some(include_str!("../assets/data/movesets/emmy_noether.ron")),
    ),
    (
        "goblin",
        Some(include_str!("../assets/data/movesets/goblin.ron")),
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
    ("author", None),
    ("bob", None),
    ("carl_stargan", None),
    ("cellular_automaton", None),
    ("emmy_noether", None),
    ("goblin", None),
    ("medic", None),
    ("ninja_shadow_oni_leader", None),
    ("officer", None),
    ("oiler", None),
    ("patent_clerk", None),
    ("performer", None),
    ("pirate_admiral", None),
    ("pointed_polygon", None),
    ("projectile_polygon", None),
    ("pugnacious_polygon", None),
];

/// The authored fighter difficulty ladder.
///
/// this file existed and nothing read it. A content test parsed it; the
/// game did not, and `FighterBrainProfile::for_level` — which documents itself as
/// the floor a game overrides — was consulted at both production call sites
/// instead. Declaring it here is what makes it content rather than a document.
pub const FIGHTER_BRAIN_LADDER_RON: &str = include_str!("../assets/data/fighter_brain_ladder.ron");

/// One declared source's text: embedded when this build baked it in, otherwise
/// read from the same file off disk.
///
/// The declared path IS the on-disk path — `pack.ron` spells sources exactly as
/// they sit under this crate's `assets/` — so a source needs no second location
/// to be kept in sync with.
///
/// `CARGO_MANIFEST_DIR` is a compile-time STRING, not a file dependency, so
/// resolving through it costs no rebuild when the file changes. That is the
/// whole point: see [`crate::audio_registries::MUSIC_REGISTRY_RON_STATIC`].
///
/// A missing file is fatal and says so. Content that silently lost a family is
/// exactly the "silent partial start" [`compile_pack`] already refuses.
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
    // the boss encounters are appended from ONE table, not written out
    // here beside `BOSS_ENCOUNTER_RONS[n]`. Path and bytes travel together in
    // `bosses::BOSS_ENCOUNTERS`, so reordering that list cannot attach a file's
    // contents to another file's diagnostic path — which the index form allowed,
    // silently, because the runtime resolves rows by their internal ids.
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
/// Three lists that all had to agree, with nothing making them agree, and `engine_schemas()` itself
/// had ZERO callers: the SDK's declared answer to "which crates own which schemas" was dead code
/// while two other places answered it privately.
pub fn pack_schemas() -> ambition_content_pack::SchemaRegistry {
    ambition_platformer2d::content::engine_schemas()
}

/// Compile Ambition's embedded pack.
///
/// Now the runtime takes what the compiler lowered.
///
/// assets are UNCHECKED here on purpose. A shipped binary's art may
/// legitimately be absent on a fresh clone (AGENTS.md: git-ignored payloads,
/// "degrade visibly when a file is absent"), and refusing to boot over a missing
/// sheet would make this compiler the thing that stops the game rather than the
/// thing that explains it. The CLI's strict mode is where art is a gate.
pub fn compile_pack() -> Result<PreparedContentPack, CompileFailure> {
    compile_pack_with(|_, text| text)
}

/// The same compile, with every source passed through `edit` first.
///
/// ⭐⭐ **ONE COMPILE ROAD, SO A SECOND PACK IS THE SAME PACK WITH AN EDIT.** A
/// test that assembled its own draft would be comparing two compiles rather than
/// two CONTENTS — a manifest or schema-registry difference would read as a
/// content difference, which is exactly the confusion a pack-selection test must
/// not be able to make.
///
/// ⚠ `edit` sees the DECLARED PATH and the source text; returning the text
/// unchanged is the shipped pack.
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
/// ⭐⭐ **THIS IS WHAT MAKES "A PREBUILT HOST PLAYS THE EDITED FILE" A FACT
/// RATHER THAN A PLAN** (fast-iteration I2's acceptance). Where content comes
/// from was `env!("CARGO_MANIFEST_DIR")` — a constant baked at BUILD time, so
/// a shipped binary read a path on the machine that compiled it and a running
/// host had no way to be pointed anywhere else. A directory is an argument now.
///
/// ⛔⛔ **EVERY MISSING FILE IS NAMED, AND NONE IS SILENTLY INHERITED.** Falling
/// back to this build's own text per file would compile a MIXED pack out of a
/// directory and a binary, and "which half did I just play" would be decided by
/// which files happened to exist — the same failure the artifact envelope
/// refuses a duplicate section for. A root either supplies the pack or it does
/// not.
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
    // ⛔ THE ROOT'S OWN PROBLEMS ARE REPORTED BEFORE THE COMPILER'S. An absent
    // file reaches the compiler as an empty source, whose diagnostic is a parse
    // error pointing at byte 0 — true, and useless to somebody who mistyped a
    // directory.
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
/// ⭐ THE OTHER HALF OF THE LOOP, and the reason [`compile_pack_from`] can
/// refuse a partial root without being unusable: a developer (or a test) starts
/// from what the binary already has, edits one file, and points the host at it.
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
/// Every family's install reads the SAME prepared value rather than compiling
/// its own: compiling per family would multiply the cost by the family count
/// and — worse — let two families disagree about which pack they came from.
///
/// A loud stop on failure, and the repo has already chosen once between this and
/// a half-built start: *"a silent partial start would be worse than a loud
/// stop"*. Content that silently lost a character or an item is exactly that.
pub fn prepared() -> &'static PreparedContentPack {
    boot_pack()
}

/// The process's boot pack, behind an `Arc` so an App can hold it without a
/// second compile.
///
/// ⛔ PRIVATE. [`prepared`] is the read for a family that has not been migrated
/// to App-scoped selection; [`selected`] is the read for one that has. Handing
/// the `Arc` out generally would make "which pack is this App's" answerable from
/// anywhere again, which is the thing step 1 removes.
fn boot_pack() -> &'static std::sync::Arc<PreparedContentPack> {
    static PREPARED: std::sync::OnceLock<std::sync::Arc<PreparedContentPack>> =
        std::sync::OnceLock::new();
    PREPARED.get_or_init(|| {
        std::sync::Arc::new(compile_pack().unwrap_or_else(|failure| {
            panic!("Ambition's own content pack does not compile:\n{failure}")
        }))
    })
}

/// THIS App's content pack — fast-iteration I3, step 1.
///
/// ⭐⭐ **A PROCESS-GLOBAL `OnceLock` CANNOT BE RE-SELECTED, AND THAT IS THE
/// WHOLE OF THE PROBLEM.** The first caller compiles the pack and every later
/// caller gets that value forever. Two Apps in one process therefore share one
/// pack whether they agree or not, a reload has nowhere to put a new one, and a
/// test cannot hand a composition content of its own. I3's acceptance says it in
/// one line: *"Two Apps can select different packs without contamination."*
///
/// ⛔ IMMUTABLE DATA IS SHARED; ACTIVATION AUTHORITY IS NOT (step 1's own
/// words). The `Arc` means selecting the boot pack costs no second compile —
/// what is App-scoped is WHICH pack this App answers with, not a copy of it.
#[derive(bevy::prelude::Resource, Clone)]
pub struct SelectedContentPack(pub std::sync::Arc<PreparedContentPack>);

impl SelectedContentPack {
    pub fn get(&self) -> &PreparedContentPack {
        &self.0
    }
}

/// Give this App a pack of its own, replacing any previous selection.
///
/// ⚠ SELECTION IS NOT PUBLICATION. Installing a pack here changes what LATER
/// reads answer; it does not revise a cast already published. That is
/// [`crate::reload`]'s job, and keeping them separate is what lets a reload
/// refuse without having already replaced the App's content.
pub fn select_pack(app: &mut bevy::prelude::App, pack: std::sync::Arc<PreparedContentPack>) {
    install_selection(app.world_mut(), pack);
}

/// Install a selection and publish its identity to the engine.
///
/// ⭐⭐ **THE ENGINE IS TOLD WHICH PACK, NOT GIVEN THE PACK.** Content-pack
/// compilation lives here and must stay here; what
/// `ambition_platformer2d_provider` needs in order to fingerprint a prepared
/// session is the pack's IDENTITY, so the two travel together and nothing
/// downstream has to reach back up for it.
///
/// ⛔⛤ **AND THEY DID NOT TRAVEL TOGETHER UNTIL NOW.** Every section of a
/// `PreparedContent` was an App REGISTRY, so the authored pack reached the game
/// without reaching the engine's content fingerprint: two sessions prepared
/// under different move tables shared one `PreparedContentIdentity`, and the
/// rollback timeline contract that refuses *"prepared content changed while the
/// session was active"* compares exactly that identity.
pub(crate) fn install_selection(
    world: &mut bevy::ecs::world::World,
    pack: std::sync::Arc<PreparedContentPack>,
) {
    world.insert_resource(ambition_platformer2d_runtime::SelectedContentIdentity(
        // ⚠ THE CANONICAL IDENTITY LINE, not the fingerprint alone. A hex digest
        // is unreadable in a desync report, and the pack's id and version are
        // what a human needs to see beside it.
        format!("{} {} {}", pack.id, pack.version, pack.fingerprint),
    ));
    world.insert_resource(SelectedContentPack(pack));
}

/// This App's pack, selecting the process's boot pack if nothing chose one.
///
/// ⛔ THE FALLBACK IS AN INSERT, NOT A READ-THROUGH. A read-through would let an
/// App answer from the boot pack forever while believing it had a selection, so
/// every later `selected` call could give a different answer than the first.
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

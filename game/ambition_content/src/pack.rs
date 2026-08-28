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
    // the manifest is a DIAGNOSTIC, not a panic, since gave the compiler its own
    // embedded-pack road.
    let draft = ambition_content_pack::ContentPackDraft::from_manifest_ron(
        PACK_MANIFEST_RON,
        embedded_sources(),
    )?;
    ambition_content_pack::compile(
        &draft,
        &pack_schemas(),
        &ambition_content_pack::AssetsUnchecked,
    )
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
    static PREPARED: std::sync::OnceLock<PreparedContentPack> = std::sync::OnceLock::new();
    PREPARED.get_or_init(|| {
        compile_pack().unwrap_or_else(|failure| {
            panic!("Ambition's own content pack does not compile:\n{failure}")
        })
    })
}

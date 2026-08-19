//! **THIS DEMO'S CHARACTER PACKAGE — George's values, as content.**
//!
//! ⭐⭐ **D166's first facet, from the consuming side.** The queue's row draws
//! three layers and says the middle one does not exist yet:
//!
//! ```text
//! Smash capability   defines the facet's semantics    ambition_characters::smash_fighter
//!         ^
//! character package  authors George's VALUES          assets/fighters/george_booul.ron
//!         v
//! Smash preparation  produces runtime MoveSpecs       CaptureKitAuthoring::into_repertoire
//! ```
//!
//! This module is the character-package layer. It compiles this demo's own pack
//! through `ambition_platformer2d::content` — the same compiler Ambition's pack
//! uses, reached through the facade rather than through the game crate — and
//! hands the prepared kit to [`crate::george_booul_moveset`].
//!
//! ## ⚠ The E9 oracle still holds, and it is what shaped this
//!
//! This crate's manifest says *"`ambition_platformer2d` + `bevy` and nothing
//! else. If authoring a STOCKS match needs a type the umbrella does not
//! re-export, that is an engine leak and it fails to compile HERE."* Compiling a
//! pack needed one: parsing `pack.ron` was a `ron::from_str` at every pack
//! owner, and `ron` is not a facade re-export. The leak was closed at the
//! FACADE — [`ContentPackDraft::from_manifest_ron`] parses the manifest and
//! reports a diagnostic where two call sites used to parse it and panic — rather
//! than by adding a dependency here.
//!
//! ## ⛔ A missing facet is LOUD
//!
//! [`capture_kit`] refuses rather than returning `None`. An absent capture kit
//! is not a fighter without a grab — every road that reaches it is downstream of
//! this pack being embedded in this binary, so absence means the manifest and
//! [`embedded_sources`] disagree, and the fighter losing his grab silently is
//! exactly the failure the content compiler exists to convert into a sentence.

use ambition_platformer2d::characters::smash_capture::SmashCaptureRepertoire;
use ambition_platformer2d::characters::smash_fighter::content_schema::lowered_smash_fighters;
use ambition_platformer2d::characters::smash_fighter::SmashFighterFacet;
use ambition_platformer2d::content::{CompileFailure, PreparedContentPack};

/// The pack manifest, embedded from the SAME file the CLI reads off disk.
const PACK_MANIFEST_RON: &str = include_str!("../assets/pack.ron");

/// The declared path of each source, exactly as `pack.ron` spells it. A
/// mismatch is the compiler's own "no source supplied" refusal rather than a
/// silently absent fighter.
const GEORGE_FACET_PATH: &str = "fighters/george_booul.ron";
const GEORGE_FACET_RON: &str = include_str!("../assets/fighters/george_booul.ron");

/// Every source `pack.ron` declares, paired with its embedded text.
fn embedded_sources() -> impl IntoIterator<Item = (String, String)> {
    [(GEORGE_FACET_PATH.to_string(), GEORGE_FACET_RON.to_string())]
}

/// Compile this demo's embedded pack.
///
/// ⚠ assets are UNCHECKED here for the same reason they are in Ambition's own
/// pack: a shipped binary's art may legitimately be absent on a fresh clone, and
/// refusing to boot over a missing sheet would make the content compiler the
/// thing that stops the game rather than the thing that explains it.
pub fn compile_pack() -> Result<PreparedContentPack, CompileFailure> {
    let draft = ambition_platformer2d::content::ContentPackDraft::from_manifest_ron(
        PACK_MANIFEST_RON,
        embedded_sources(),
    )?;
    ambition_platformer2d::content::compile(
        &draft,
        &ambition_platformer2d::content::engine_schemas(),
        &ambition_platformer2d::content::AssetsUnchecked,
    )
}

/// The prepared pack, compiled once per process.
///
/// A loud stop on failure, the same choice Ambition's pack made: content that
/// silently lost a fighter's grab is worse than a stop that names the file.
pub fn prepared() -> &'static PreparedContentPack {
    static PREPARED: std::sync::OnceLock<PreparedContentPack> = std::sync::OnceLock::new();
    PREPARED.get_or_init(|| {
        compile_pack().unwrap_or_else(|failure| {
            panic!("the smash demo's content pack does not compile:\n{failure}")
        })
    })
}

/// One character's authored platform-fighter facet, or `None` if this pack does
/// not author one for them.
pub fn fighter_facet(character: &str) -> Option<&'static SmashFighterFacet> {
    lowered_smash_fighters(prepared())?.get(character)
}

/// **The prepared capture kit for a character this pack authors.**
///
/// # Panics
///
/// If no facet names `character`. See the module note: absence here is a
/// build-artifact bug, not a content decision, and the message lists what the
/// pack does define so the answer is in the failure.
pub fn capture_kit(character: &str) -> SmashCaptureRepertoire {
    let Some(facet) = fighter_facet(character) else {
        let authored: Vec<&str> = lowered_smash_fighters(prepared())
            .map(|book| book.keys().map(String::as_str).collect())
            .unwrap_or_default();
        panic!(
            "the smash demo's pack authors no platform-fighter facet for `{character}`; \
             it authors {authored:?}. Add the file to `assets/`, declare it in \
             `assets/pack.ron`, and embed it in `smash_pack::embedded_sources`."
        );
    };
    facet.capture.clone().into_repertoire()
}

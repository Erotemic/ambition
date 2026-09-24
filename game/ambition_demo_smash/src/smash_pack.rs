//! Smash demo content pack for George Booul.
//!
//! The demo compiles its embedded `pack.ron` through the platformer facade and
//! prepares George's platform-fighter facet. George's authored values live with
//! the character in the sprite-authoring submodule; this demo selects them.

use ambition_platformer2d::entity_catalog::smash_capture::SmashCaptureRepertoire;
use ambition_platformer2d::characters::smash_fighter::content_schema::lowered_smash_fighters;
use ambition_platformer2d::characters::smash_fighter::SmashFighterFacet;
use ambition_platformer2d::content::{CompileFailure, PreparedContentPack};

/// The pack manifest, embedded from the same file the CLI reads off disk.
const PACK_MANIFEST_RON: &str = include_str!("../assets/pack.ron");

/// The declared path of each source, as `pack.ron` spells it. A mismatch gives
/// the compiler's "no source supplied" error instead of a missing fighter.
const GEORGE_FACET_PATH: &str =
    "../../../tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/data/characters/george_booul/smash_fighter.ron";
/// George's authored fighter facet, selected here but owned by character authoring.
const GEORGE_FACET_RON: &str = include_str!(
    "../../../tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/data/characters/george_booul/smash_fighter.ron"
);

/// Every source `pack.ron` declares, paired with its embedded text.
fn embedded_sources() -> impl IntoIterator<Item = (String, String)> {
    [(GEORGE_FACET_PATH.to_string(), GEORGE_FACET_RON.to_string())]
}

/// Compile the embedded pack without requiring art assets to exist locally.
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

/// Compile and cache the pack once; invalid authored content is fatal.
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

/// The body a character's authored facet states for its fighter self, layered
/// over the base platform-fighter body.
///
/// This is the other half of `MatchParticipant::body`. A catalog row's feel
/// applies everywhere the character appears, so a character that also fights
/// states its fighter body in its own package, and the roster gives it to the
/// seat.
///
/// `None` when the pack has no facet for the character or the facet states no
/// body: keep the current body.
pub fn fighter_body(character: &str) -> Option<ambition_platformer2d::engine_core::MovementTuning> {
    fighter_facet(character)?
        .body
        .as_ref()
        // The base is the player-grade body, not the actor baseline
        // (`BodyMovementTuning::BASELINE`, the wandering-enemy body with an
        // eighth of the player's ground acceleration). An authored body states
        // its differences from a fighter, so they layer onto a fighter.
        .map(|body| body.over(ambition_platformer2d::engine_core::DEFAULT_TUNING))
}

/// How hard a character is to launch, where its authored facet states it.
///
/// The character owns its weight; a game must not set `Vitals::knockback_weight`
/// from outside (see `character-authoring-package.md`).
///
/// `None` when the pack has no facet for the character or the facet states no
/// weight: keep the current weight (the reference body by default).
pub fn fighter_knockback_weight(character: &str) -> Option<f32> {
    fighter_facet(character)?.knockback_weight
}

/// The prepared capture kit for a character this pack authors.
///
/// # Panics
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

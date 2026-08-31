//! Smash demo content pack for George Booul.
//!
//! The demo compiles its embedded `pack.ron` through the platformer facade and
//! prepares George's platform-fighter facet. George's authored values live with
//! the character in the sprite-authoring submodule; this demo selects them.

use ambition_platformer2d::characters::smash_capture::SmashCaptureRepertoire;
use ambition_platformer2d::characters::smash_fighter::content_schema::lowered_smash_fighters;
use ambition_platformer2d::characters::smash_fighter::SmashFighterFacet;
use ambition_platformer2d::content::{CompileFailure, PreparedContentPack};

/// The pack manifest, embedded from the SAME file the CLI reads off disk.
const PACK_MANIFEST_RON: &str = include_str!("../assets/pack.ron");

/// The declared path of each source, exactly as `pack.ron` spells it. A
/// mismatch is the compiler's own "no source supplied" refusal rather than a
/// silently absent fighter.
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

/// The BODY a character's authored facet states for its FIGHTER self, layered
/// over the body every platform fighter on this stage starts from.
///
/// ⭐⭐ THIS IS THE OTHER HALF OF `MatchParticipant::body`. A catalog row's feel
/// is that character's feel everywhere it appears — a hub, a room, a stage — so
/// a character that walks around a hub and also fights states its fighter body
/// in its own package instead, and the roster hands it to the seat.
///
/// `None` for a character this pack authors no facet for, and for one whose
/// facet states no body: both mean *keep whatever body you already had*.
pub fn fighter_body(character: &str) -> Option<ambition_platformer2d::engine_core::MovementTuning> {
    fighter_facet(character)?
        .body
        .as_ref()
        // ⛔ THE BASE IS THE PLAYER-GRADE BODY, NOT THE ACTOR BASELINE, and the
        // difference is the whole reason this road exists: a seat that reaches
        // preparation with nothing composes over the WANDERING-ENEMY body
        // (`BodyMovementTuning::BASELINE` — an eighth of the player's ground
        // acceleration). A fighter that has bothered to author a body is
        // stating its DIFFERENCES from a fighter, so the differences layer onto
        // a fighter.
        .map(|body| body.over(ambition_platformer2d::engine_core::DEFAULT_TUNING))
}

/// How hard a character is to LAUNCH, where its authored facet states it.
///
/// ⭐⭐ THE LAST PER-ID CHARACTER TABLE IN THIS DEMO CAME OUT THROUGH HERE.
/// `smash_reading_of_character` was a `match definition.id` writing
/// `Vitals::knockback_weight` — an ordinary character fact the engine already
/// owns — for a character the demo does not own. A game describing a character's
/// weight from outside is the falsifier `character-authoring-package.md` names.
///
/// `None` for a character this pack authors no facet for, and for one whose
/// facet states no weight: both mean *keep whatever weight you already had*,
/// which for a fighter that has never thought about it is the reference body.
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

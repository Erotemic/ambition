//! Smash demo content pack for George Booul.
//!
//! The demo compiles its embedded `pack.ron` through the platformer facade:
//! every fighter's move table, and George's platform-fighter facet. George's authored values live with
//! the character in the sprite-authoring submodule; this demo selects them.

use ambition_platformer2d::characters::smash_fighter::content_schema::lowered_smash_fighters;
use ambition_platformer2d::characters::smash_fighter::SmashFighterFacet;
use ambition_platformer2d::content::EmbeddedPack;

/// The George paths leave the demo on purpose: the demo selects George's
/// values from the character-authoring submodule, it does not own them.
macro_rules! george {
    ($file:literal) => {
        concat!(
            "../../../tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/data/characters/george_booul/",
            $file
        )
    };
}

/// The demo's pack: `assets/pack.ron` and every source it declares, with the
/// path `pack.ron` spells. A path that does not match gives the compiler's "no
/// source supplied" refusal instead of a missing fighter.
///
/// The demo registers each fighter with `PACK.moveset(id)`, and the tests read
/// the same, so they guard the files the demo plays.
pub static PACK: EmbeddedPack = EmbeddedPack::new(
    include_str!("../assets/pack.ron"),
    &[
        (george!("smash_fighter.ron"), include_str!(george!("smash_fighter.ron"))),
        (george!("smash_moveset.ron"), include_str!(george!("smash_moveset.ron"))),
        (
            "data/movesets/smash_duelist_a.ron",
            include_str!("../assets/data/movesets/smash_duelist_a.ron"),
        ),
        (
            "data/movesets/smash_duelist_b.ron",
            include_str!("../assets/data/movesets/smash_duelist_b.ron"),
        ),
    ],
);

/// One character's authored platform-fighter facet, or `None` if this pack does
/// not author one for them.
pub fn fighter_facet(character: &str) -> Option<&'static SmashFighterFacet> {
    lowered_smash_fighters(PACK.prepared())?.get(character)
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

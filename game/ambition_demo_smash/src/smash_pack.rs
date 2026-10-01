//! The Smash demo's content pack.
//!
//! The demo compiles its embedded `pack.ron` through the platformer facade: its
//! cast (the character catalog), every fighter's move table and every
//! fighter's platform-fighter facet. George's authored values live with the
//! character in the sprite-authoring submodule; this demo selects them.

use ambition_platformer2d::characters::smash_fighter::content_schema::fighter_body as fighter_body_in;
use ambition_platformer2d::content::EmbeddedPack;

/// The demo's pack: `assets/pack.ron` and every source it declares, with the
/// path `pack.ron` spells. A path that does not match gives the compiler's "no
/// source supplied" refusal instead of a missing fighter.
///
/// The demo registers each fighter with `PACK.moveset(id)`, and the tests read
/// the same, so they guard the files the demo plays.
///
/// The George paths leave the demo on purpose: the demo selects George's
/// values from the character-authoring submodule, it does not own them.
pub static PACK: EmbeddedPack = ambition_platformer2d::content_pack! {
    root: "assets",
    sources: [
        "data/character_catalog.ron",
        "data/fighters/smash_duelist_a.ron",
        "data/fighters/smash_duelist_b.ron",
        "../../../tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/data/characters/george_booul/smash_fighter.ron",
        "../../../tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/data/characters/george_booul/smash_moveset.ron",
        "data/movesets/smash_duelist_a.ron",
        "data/movesets/smash_duelist_b.ron",
        "audio/music_registry.ron",
    ],
};

/// The body a character plays on as a fighter, as this pack's facet states
/// it: the other half of `MatchParticipant::body`. The facet's owner
/// (`smash_fighter::content_schema::fighter_body`) says what the facet means;
/// this names the pack.
pub fn fighter_body(character: &str) -> Option<ambition_platformer2d::engine_core::MovementTuning> {
    fighter_body_in(PACK.prepared(), character)
}

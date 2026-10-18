//! The Smash demo's content pack.
//!
//! The demo compiles its embedded `pack.ron` through the platformer facade: its
//! cast (the character catalog), every fighter's move table and every
//! fighter's platform-fighter facet. George's authored values live with the
//! character in the sprite-authoring submodule; this demo selects them.

use ambition_platformer2d::characters::smash_fighter::content_schema::fighter_body as fighter_body_in;
use ambition_platformer2d::content::EmbeddedPack;

/// The demo's pack: `assets/pack.ron` and every source it declares. The
/// manifest is the one list, so a source line added there is played with no
/// Rust build.
///
/// The demo registers each fighter with `PACK.moveset(id)`, and the tests read
/// the same, so they guard the files the demo plays.
///
/// The George paths in `pack.ron` leave the demo on purpose: the demo selects George's
/// values from the character-authoring submodule, it does not own them.
pub static PACK: EmbeddedPack = ambition_platformer2d::content_pack! {
    root: "assets",
    // The stages and the demo's own rules are this demo's content.
    schemas: [crate::stages::smash_stages_schema, crate::rules::smash_rules_schema, crate::text::smash_text_schema],
};

/// The body a character plays on as a fighter, as this pack's facet states
/// it: the other half of `MatchParticipant::body`. The facet's owner
/// (`smash_fighter::content_schema::fighter_body`) says what the facet means;
/// this names the pack.
pub fn fighter_body(character: &str) -> Option<ambition_platformer2d::engine_core::MovementTuning> {
    fighter_body_in(PACK.prepared(), character)
}

/// What "CPU level N" means in Smash's rooms, as its pack states it
/// (`data/fighter_brain_ladder.ron`).
pub fn fighter_ladder() -> ambition_platformer2d::characters::brain::fighter::FighterBrainLadder {
    ambition_platformer2d::combat::brain::fighter::content_schema::lowered_fighter_brain_ladder(PACK.prepared())
        .cloned()
        .expect("Smash's pack states its CPU ladder (`data/fighter_brain_ladder.ron`)")
}

/// The rules of Smash's matches, as its pack states them
/// (`assets/data/match_rules.ron`): stocks, clocks, the percent pool, the
/// fighter body, the ability floor and ceiling, fighter contact.
pub fn match_rules() -> ambition_platformer2d::versus_match::match_rules::AuthoredMatchRules {
    *ambition_platformer2d::versus_match::match_rules::lowered_match_rules(PACK.prepared())
        .expect("Smash's pack states the rules of its matches (`data/match_rules.ron`)")
}

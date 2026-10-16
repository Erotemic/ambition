//! Mary-O's AI Slop — the plain stompable walker, authored as pure content.
//!
//! Mary-O has TWO enemies. Solid Snake is the shell enemy: a head-stomp makes it
//! withdraw into a kickable shell (see `snake.rs`). AI Slop is the SIMPLE one — the
//! throwaway mob a head-stomp just SQUASHES flat (similar to a Goomba). It renders
//! from the published `ai_slop` sheet.
//!
//! Unlike the snake, a squashed AI Slop does not become anything — it just dies. The
//! squash is the module `ambition_demo_mary_o_modules::squash`, bound to the body's
//! authored brain key in `body_contact`, so it never touches a snake (which owns its
//! own shell rule); the two enemies never share a code path.
//!
//! Every type it names comes through the `ambition_platformer2d` umbrella — the E9 oracle.

use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::entity_catalog::placements::CharacterBrain;

/// The roster brain key the AI Slop archetype is filed under, namespaced so it
/// never collides with a host provider's roster. The squash module binds it.
pub use ambition_demo_mary_o_modules::squash::AI_SLOP_BRAIN_KEY;

/// The `ai_slop` sheet TARGET (also the catalog id) — the generated sheet the
/// enemy render resolves for an AI Slop.
pub const AI_SLOP_SHEET_TARGET: &str = "ai_slop";

// Demo-owned hostile archetype: ONE 1-HP `Wanderer` that walks forward and reverses
// at walls (`aggro_radius`/`attack_range` are ignored by that template). It carries
// no `melee`, so its only offense is the default-on body contact.
//
// The ROW is authored with no outer braces, so it can register on its own for a
// single-enemy test OR fold into the combined Mary-O roster fragment — one fragment
// per provider, since assembly rejects a second from the same provider.

// `AI_SLOP_TILE_COLUMNS`, `ai_slop_stair_steps()` and
// `stair_slop_spawn_positions()` are GONE. Where a slop stands is authored in
// the level; how big it is, is authored in its catalog row.

/// Half-extents of an AI Slop's idle body, in world units.
///
/// The same resolution construction performs (`posed_body_geometry` at the
/// scale its row names), so a level or a test that asks how big a slop is
/// compares against the body the cast builds, not a second derivation.
pub fn ai_slop_half_size() -> ae::Vec2 {
    let scale = crate::pack::PACK.posed_body_world_per_pixel(AI_SLOP_SHEET_TARGET);
    ambition_platformer2d::character_sprites::posed_body_geometry(
        AI_SLOP_SHEET_TARGET,
        ambition_platformer2d::sprite_sheet::character::CharacterAnim::Idle,
        scale,
    )
    .expect("the ai_slop sheet is baked")
    .collision
        * 0.5
}

/// Is this actor an AI Slop?
///
/// Both mobs made the same mistake twice, so the reasoning is written once.
///
/// The level authors them now, and the engine's own authored-enemy construction builds them —
/// this crate no longer stages a second copy.
pub fn is_ai_slop_brain(brain: &CharacterBrain) -> bool {
    matches!(brain, CharacterBrain::Custom(key) if key == AI_SLOP_BRAIN_KEY)
}

#[cfg(test)]
mod tests;

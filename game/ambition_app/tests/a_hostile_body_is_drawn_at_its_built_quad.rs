#![cfg(feature = "rl_sim")]
//! A hostile body is drawn at the quad it was built from.
//!
//! The seed resolves a body's sprite quad in the same call that sizes its
//! collider. The hostile spawn sites then resolved it a second time from the
//! placement's NAME, a free label. Measured 2026-09-25 over the 43 enemy
//! placements: 39 labels happened to be their character's display name, and
//! the four `Skirmisher` placements of `npc_pirate_raider` resolved to nothing
//! and spawned with no `ActorRenderSize` at all.

use crate::common::{base, fixed_60hz_room_sim};
use ambition_platformer2d::characters::actor::WornCharacter;
use ambition_platformer2d::combat::components::ActorRenderSize;
use bevy::prelude::*;

#[test]
fn a_skirmisher_is_drawn_at_the_quad_its_character_resolves() {
    const CHARACTER: &str = "npc_pirate_raider";
    let mut sim = fixed_60hz_room_sim("volatile_cache");
    sim.step_n(base(), 30);

    let world = sim.world_mut();
    let quads: Vec<Option<Vec2>> = world
        .query::<(&WornCharacter, Option<&ActorRenderSize>)>()
        .iter(world)
        .filter(|(worn, _)| worn.id() == CHARACTER)
        .map(|(_, quad)| quad.map(|quad| quad.0))
        .collect();
    assert!(
        quads.len() >= 2,
        "`volatile_cache` stages its two `Skirmisher` placements, or this arm is \
         vacuous ({} found)",
        quads.len()
    );

    // The authority: the catalog join by CHARACTER, the resolution the seed
    // sized the collider from. A `Standard` row states its standing height, so
    // the placement box does not enter the answer.
    let expected = ambition_platformer2d::sprite_sheet::character::catalog_join::sprite_body_collision_for_character_id_from_data(
        world.resource::<ambition_platformer2d::sprite_sheet::character::sheets::AuthoredSheets>(),
        world
            .resource::<ambition_platformer2d::characters::actor::character_catalog::CharacterCatalog>()
            .data(),
        CHARACTER,
        Vec2::ZERO,
    )
    .expect("the raider's catalog row names a sheet with body metrics")
    .render_size;
    for quad in quads {
        assert_eq!(
            quad,
            Some(expected),
            "a `{CHARACTER}` body is drawn at a quad its character did not resolve"
        );
    }
}

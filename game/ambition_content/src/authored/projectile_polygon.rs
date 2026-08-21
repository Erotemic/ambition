//! Projectile Polygon — bestial ranged reference archetype.
//!
//! The third Fighting Polygon is a non-humanoid beast biped: a faceted T-rex-like
//! body with heavy hind legs, a balancing tail, and a head-mounted cannon.
//! Its combat distinction is a body-authored projectile emitted from that cannon.

use ambition_characters::actor::CharacterLocomotion;
use ambition_characters::brain::{ActionSet, MoveStyleSpec, RangedActionSpec};
use ambition_platformer2d_actor_monolith::character_runtime::CharacterDefinition;

pub(crate) fn author(_id: &str, definition: CharacterDefinition) -> CharacterDefinition {
    let mut definition = definition
        .with_locomotion(CharacterLocomotion {
            run_speed: 225.0,
            move_style: MoveStyleSpec::Walk,
            ..Default::default()
        })
        .with_ranged_vfx("polygon_bolt")
        .with_action_set(ActionSet {
            ranged: Some(RangedActionSpec::bolt(540.0, 4)),
            move_style: MoveStyleSpec::Walk,
            ..Default::default()
        })
        .with_moveset(crate::projectile_polygon_moveset::projectile_polygon_moveset());
    definition.vitals.max_health = Some(5);
    definition
}

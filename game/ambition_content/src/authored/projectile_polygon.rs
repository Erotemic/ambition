//! Projectile Polygon — bestial ranged reference archetype.
//!
//! The third Fighting Polygon is a non-humanoid beast biped: a faceted T-rex-like
//! body with heavy hind legs, a balancing tail, and a head-mounted cannon.
//! Its combat distinction is a body-authored projectile emitted from that cannon.

use ambition_characters::actor::CharacterLocomotion;
use ambition_characters::brain::{ActionSet, MoveStyleSpec, RangedActionSpec, RangedCharge};
use ambition_platformer2d::character::CharacterDefinition;

/// His cannon, and what holding the button buys.
///
/// ⭐ THE CHARGE SHOT IS THE WHOLE CHARACTER. He is the trio's ranged member and
/// the grid's only fighter whose combat distinction is a body-authored
/// projectile; a neutral special that fired the same pellet however long you
/// held it would make that distinction a bullet point rather than a mechanic.
///
/// The numbers are the genre's shape rather than measurements: a full hold is
/// worth about three and a half times a tap, arrives half again as fast, and
/// is more than twice the size — which together is why a fully charged shot is
/// something you respect rather than shield out of habit. The five looks are
/// the sheet's five tiers; see `crate::projectiles`.
fn charged_cannon() -> RangedActionSpec {
    RangedActionSpec::bolt(540.0, 4)
        // ⛔ THE FLIGHT IS AUTHORED SO THE SIZE HAS SOMETHING TO SCALE. `None`
        // means "whatever the firing pool's default is", and a default is not a
        // number this shot owns — `size_mult` applied to it would be applied to
        // nothing. Stated here, at the pool's own straight envelope, so the tap
        // is byte-identical to the shot he fired before charging existed and a
        // full hold has a base to grow from.
        .with_flight(ambition_characters::brain::ProjectileFlight::STRAIGHT)
        .with_charge(RangedCharge {
            damage_mult: 3.5,
            speed_mult: 1.5,
            size_mult: 2.4,
            visuals: (1..=5)
                .map(|tier| format!("polygon_charge_shot_tier{tier}"))
                .collect(),
        })
}

pub(crate) fn author(_id: &str, definition: CharacterDefinition) -> CharacterDefinition {
    let mut definition = definition
        .with_locomotion(CharacterLocomotion {
            run_speed: 225.0,
            move_style: MoveStyleSpec::Walk,
            ..Default::default()
        })
        .with_ranged_vfx("polygon_bolt")
        .with_action_set(ActionSet {
            ranged: Some(charged_cannon()),
            move_style: MoveStyleSpec::Walk,
            ..Default::default()
        })
        .with_moveset(crate::projectile_polygon_moveset::projectile_polygon_moveset());
    definition.vitals.max_health = Some(5);
    definition
}

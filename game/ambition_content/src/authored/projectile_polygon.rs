//! Projectile Polygon: bestial ranged reference archetype.
//!
//! The third Fighting Polygon is a non-humanoid beast biped: a faceted T-rex-like
//! body with heavy hind legs, a balancing tail, and a head-mounted cannon.
//! Its combat distinction is a body-authored projectile emitted from that cannon.

use ambition_characters::actor::CharacterLocomotion;
use ambition_characters::brain::{ActionSet, MoveStyleSpec, RangedActionSpec, RangedCharge};
use ambition_platformer2d::character::CharacterDefinition;

/// His cannon, and what holding the button buys.
///
/// The charge shot is the character. He is the trio's ranged member and the
/// only fighter whose combat distinction is a body-authored projectile; a
/// neutral special that fired the same pellet however long you held it would
/// make that distinction cosmetic.
///
/// The numbers follow the genre's shape: a full hold is worth about three and
/// a half taps, travels half again as fast and is more than twice the size, so
/// a full charge must be respected, not shielded by habit. The five looks are
/// the sheet's five tiers; see `crate::projectiles`.
fn charged_cannon() -> RangedActionSpec {
    RangedActionSpec::bolt(540.0, 4)
        // The flight is authored so the size has something to scale. `None` means
        // the firing pool's default, which this shot does not own, and `size_mult`
        // on it would scale nothing. This is the pool's straight envelope, so a tap
        // is unchanged and a full hold has a base to grow from.
        .with_flight(ambition_characters::brain::ProjectileFlight::STRAIGHT)
        .with_charge(RangedCharge {
            damage_mult: 3.5,
            speed_mult: 1.5,
            size_mult: 2.4,
            visuals: (1..=5)
                .map(|tier| format!("polygon_charge_shot_tier{tier}"))
                .collect(),
        })
        // The shot leaves the cannon, not the stomach. The module doc, the smash
        // select grid and the roster test all say he fires from a head-mounted
        // cannon; `Muzzle::BodyOrigin` would spawn it eight pixels above his
        // middle.
        //
        // The tier visuals bloom around the spawn point, so a charge at his midriff
        // would have to be drawn as a body-wide aura, not a ball at a barrel.
        //
        // Tuning, not architecture. `0.22` forward and `0.34` up put the muzzle at
        // a beast biped's head; adjust them against the sprite. What is fixed is
        // that the action states them.
        .with_muzzle(ambition_characters::brain::action_set::Muzzle::Offset {
            x: 0.22,
            y: -0.34,
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
        });
    // His moves are content, not code (fast-iteration I2, step 5): the table is
    // `assets/data/movesets/projectile_polygon.ron`, declared in `pack.ron`,
    // validated by the `moveset` schema and applied in
    // `crate::character_catalog::authored_intrinsics`, the one seam every
    // buildable character passes through. The Rust table is only the exporter's
    // source and the parity oracle's subject; the host reads neither, so editing
    // it changes nothing until it is re-exported.
    definition.vitals.max_health = Some(5);
    definition
}

#[cfg(test)]
mod tests {
    use ambition_characters::brain::action_set::Muzzle;

    /// The shot leaves the cannon, and the cannon is above the middle.
    ///
    /// The module doc, the smash select grid and the roster moveset test all
    /// describe this fighter as firing from a head-mounted cannon; this test keeps
    /// the simulation in agreement.
    ///
    /// It asserts the sign and the model, not the numbers. `0.22` / `-0.34` are
    /// tuned against a sprite and may move. What must not move: the muzzle is
    /// authored, expressed as a fraction of body height, and above the origin. A
    /// test that pinned the constants would fail on every art change.
    #[test]
    fn the_charge_shot_leaves_a_cannon_above_the_body_origin() {
        let spec = super::charged_cannon();
        let discharge = spec
            .discharge
            .expect("the charge shot states how it leaves the body");
        let Muzzle::Offset { x, y } = discharge.muzzle else {
            panic!(
                "the charge shot fires from {:?}, so a fighter whose identity is \
                 a head-mounted cannon launches it from his midriff again",
                discharge.muzzle
            );
        };
        assert!(
            y < 0.0,
            "the cannon sits at or below the body origin (y = {y}), which is the \
             stomach — up is negative here, as `BodyOrigin`'s own -8.0 is"
        );
        assert!(
            x > 0.0,
            "the cannon is not forward of the body (x = {x}), so the shot would \
             be born behind the barrel it is drawn leaving"
        );
        // A fraction, not pixels. Anything past 1.0 is a pixel value in a
        // normalized field and would put the muzzle a full body height away.
        assert!(
            x.abs() <= 1.0 && y.abs() <= 1.0,
            "the muzzle offset ({x}, {y}) is not a fraction of body height — a \
             pixel value here is scaled BY the height and lands a body away"
        );
    }
}

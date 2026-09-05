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
        // ⭐⭐ THE SHOT LEAVES THE CANNON NOW, not the stomach. This crate, the
        // smash select grid and the roster test all describe this fighter the
        // same way — "a non-humanoid beast biped whose neutral special fires
        // from a head-mounted cannon" — and until this line the shot spawned at
        // `Muzzle::BodyOrigin`, eight pixels above his middle, because that was
        // the only thing a ranged action without a drawn weapon could say.
        //
        // ⛔ THE CHARGE ART WAS BUILT TO HIDE IT. The tier visuals bloom around
        // the spawn point, so a charge that grew out of his midriff had to be
        // drawn as a body-wide aura rather than a ball at a barrel. That is the
        // shape of the problem worth naming: the simulation was right, the
        // authored spatial contract was too weak to say what everyone had
        // written down in prose.
        //
        // ⚠ TUNING, NOT ARCHITECTURE. `0.22` forward and `0.34` up are chosen to
        // sit at the head of a beast biped and are a KNOB — adjust them against
        // the sprite rather than treating them as a derived constant. What is
        // not a knob is that the action states them at all.
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
        })
        .with_moveset(crate::projectile_polygon_moveset::projectile_polygon_moveset());
    definition.vitals.max_health = Some(5);
    definition
}

#[cfg(test)]
mod tests {
    use ambition_characters::brain::action_set::Muzzle;

    /// The shot leaves the CANNON, and the cannon is above the middle.
    ///
    /// ⭐⭐ THREE PLACES IN THIS REPOSITORY DESCRIBE THIS FIGHTER AS FIRING FROM
    /// A HEAD-MOUNTED CANNON — this module's own doc, the smash select grid, and
    /// the roster moveset test — and until 2026-09-05 the shot spawned at
    /// `Muzzle::BodyOrigin`, eight pixels above his middle. The prose and the
    /// simulation disagreed, and the prose was the part everybody read.
    ///
    /// ⛔ ASSERTS THE SIGN AND THE MODEL, NOT THE NUMBERS. `0.22` / `-0.34` are
    /// tuning against a sprite and must stay free to move; what may not move is
    /// that the muzzle is AUTHORED, is expressed as a fraction of body height
    /// rather than pixels, and is ABOVE the origin. A test pinning the constants
    /// would fail on every art adjustment and teach whoever hits it to edit the
    /// expectation.
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
        // ⛔ A FRACTION, NOT PIXELS. Anything past 1.0 is a pixel value that
        // slipped into a normalized field — it would put the muzzle a full body
        // height away and read as the shot spawning off-screen.
        assert!(
            x.abs() <= 1.0 && y.abs() <= 1.0,
            "the muzzle offset ({x}, {y}) is not a fraction of body height — a \
             pixel value here is scaled BY the height and lands a body away"
        );
    }
}

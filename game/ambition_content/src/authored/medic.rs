//! The Medic: easter-egg brawler humanoid, hand-drawn.
//!
//! A field paramedic off duty, fighting the way she works: open palms, both
//! hands on anything heavy, and a vocabulary of pushes, lifts and compressions
//! rather than punches. She is the Pugnacious Polygon's archetype as a
//! different person: unarmed, close-range, same skeleton and clip vocabulary.
//!
//! Her specials all trade in one currency. Adrenaline spends a point of her
//! margin for frame advantage, Field Dressing kneels and repays two,
//! Tourniquet drags a fighter into palm range, and Rescue Lift costs nothing.
//! `crate::medic_moveset` is the table; the price and the repayment are one
//! technique with a sign, `smash.vitality`.
//!
//! Nothing may depend on her being selectable. She is meant to be found.

use ambition_platformer2d::character::CharacterDefinition;

pub(crate) fn author(_id: &str, definition: CharacterDefinition) -> CharacterDefinition {
    let mut definition = definition
        .with_locomotion(ambition_characters::actor::CharacterLocomotion {
            // Faster than the archetype she borrows; this number is hers. Her
            // moveset is the brawler's retimed shorter (50ms jabs against his 58), so
            // his speed would contradict every clip she publishes.
            run_speed: 258.0,
            move_style: ambition_characters::brain::MoveStyleSpec::Walk,
            ..Default::default()
        });
    // Its moves are content, not code (fast-iteration I2, step 5): the table is
    // `assets/data/movesets/medic.ron`, declared in `pack.ron`, validated by
    // the `moveset` schema and applied in
    // `crate::character_catalog::authored_intrinsics`, the one seam every
    // buildable character passes through. The Rust table is only the exporter's
    // source and the parity oracle's subject; the host reads neither, so editing
    // it changes nothing until it is re-exported.
    // Light: she trades the Officer's two points of stock for the tempo her
    // clips already spend, which is the same trade her neutral special makes
    // inside a single match.
    definition.vitals.max_health = Some(5);
    definition
}

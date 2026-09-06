//! The Medic — easter-egg brawler humanoid, hand-drawn.
//!
//! A field paramedic off duty, fighting the way she works: open palms, both
//! hands on anything heavy, and a vocabulary of pushes, lifts and compressions
//! rather than punches. She is the Pugnacious Polygon's archetype wearing a
//! different person — unarmed, close-range, same skeleton and clip vocabulary.
//!
//! ⭐⭐ HER SPECIALS ARE HERS, AND THEY ALL TRADE IN THE SAME CURRENCY.
//! ADRENALINE spends a point of her margin and buys frame advantage with it,
//! FIELD DRESSING kneels and repays two, TOURNIQUET drags a fighter into the
//! range where her palms are worth something, and RESCUE LIFT is the only one
//! that costs her nothing. `crate::medic_moveset` is the table; the price and
//! the repayment are one technique with a sign, `smash.vitality`.
//!
//! Nothing may depend on her being selectable. She is meant to be found.

use ambition_platformer2d::character::CharacterDefinition;

pub(crate) fn author(_id: &str, definition: CharacterDefinition) -> CharacterDefinition {
    let mut definition = definition
        .with_locomotion(ambition_characters::actor::CharacterLocomotion {
            // ⭐ FASTER THAN THE ARCHETYPE SHE BORROWS, and this is the one
            // number that is hers. Her whole authored moveset is the brawler's
            // retimed shorter — 50ms jabs against his 58 — so a body that moved
            // at his speed would contradict every clip she publishes.
            run_speed: 258.0,
            move_style: ambition_characters::brain::MoveStyleSpec::Walk,
            ..Default::default()
        })
        .with_moveset(crate::medic_moveset::medic_moveset());
    // Light: she trades the Officer's two points of stock for the tempo her
    // clips already spend, which is the same trade her neutral special makes
    // inside a single match.
    definition.vitals.max_health = Some(5);
    definition
}

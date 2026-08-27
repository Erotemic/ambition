//! The Medic — easter-egg brawler humanoid, hand-drawn.
//!
//! A field paramedic off duty, fighting the way she works: open palms, both
//! hands on anything heavy, and a vocabulary of pushes, lifts and compressions
//! rather than punches. She is the Pugnacious Polygon's archetype wearing a
//! different person — unarmed, close-range, same skeleton and clip vocabulary.
//!
//! ⚠ HER SPECIALS COST NOTHING YET. ADRENALINE spends a slice of her own margin
//! to buy tempo and FIELD DRESSING kneels and gives it back; both are authored
//! as clips and neither publishes a hit volume, because neither hits anybody.
//! The rules that make that a DECISION are gameplay, and they are not written.
//! Until they are she borrows the archetype's specials.
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

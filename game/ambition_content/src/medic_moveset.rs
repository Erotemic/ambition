//! The Medic — the brawler archetype's table, under her own name.
//!
//! Unarmed, on the Pugnacious Polygon's skeleton and its clip vocabulary. She
//! throws the archetype's punches at the archetype's timings because they are
//! literally the archetype's punches; what differs is who is throwing them and
//! what the air does about it, which is the sprite sheet's business and not
//! this table's.
//!
//! ⚠ HER SPECIALS ARE NOT IN HERE YET. ADRENALINE, TOURNIQUET, FIELD DRESSING
//! and RESCUE LIFT are authored as CLIPS and hit volumes in the sprite
//! repository; the rules that make one of them cost health and another give it
//! back are gameplay and belong in this table. Until someone writes them she
//! borrows the archetype's specials, the same way the Officer does.
//!
//! See [`crate::archetype_moveset`] for why the ids are renamed rather than
//! shared or copied.

use ambition_platformer2d::entity_catalog::MovesetContract;

/// Complete brawler-fundamentals repertoire, attributed to the Medic.
pub fn medic_moveset() -> MovesetContract {
    crate::archetype_moveset::under_own_name(
        crate::pugnacious_polygon_moveset::pugnacious_polygon_moveset(),
        &["polygon_brawler", "pugnacious_polygon"],
        "medic",
    )
}

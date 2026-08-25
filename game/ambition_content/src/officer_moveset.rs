//! The Officer — the brawler archetype's table, under his own name.
//!
//! Unarmed, on the Pugnacious Polygon's skeleton and its clip vocabulary. He
//! throws the archetype's punches at the archetype's timings because they are
//! literally the archetype's punches; what differs is who is throwing them and
//! what the air does about it, which is the sprite sheet's business and not
//! this table's.
//!
//! See [`crate::archetype_moveset`] for why the ids are renamed rather than
//! shared or copied.

use ambition_platformer2d::entity_catalog::MovesetContract;

/// Complete brawler-fundamentals repertoire, attributed to the Officer.
pub fn officer_moveset() -> MovesetContract {
    crate::archetype_moveset::under_own_name(
        crate::pugnacious_polygon_moveset::pugnacious_polygon_moveset(),
        &["polygon_brawler", "pugnacious_polygon"],
        "officer",
    )
}

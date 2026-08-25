//! The Author — the sword archetype's table, wielded with a pen.
//!
//! His rig is the Pointed Polygon's, retargeted: the pen occupies the arming
//! sword's exact axis and length, which is why every one of the archetype's
//! 136 clips reads correctly on him without a pose being re-authored. The
//! spacing that follows from that reach is the spacing he fights at, so his
//! frame data IS the archetype's rather than a copy of it that will drift.
//!
//! What is his own is the NAME on it. See [`crate::archetype_moveset`].

use ambition_platformer2d::entity_catalog::MovesetContract;

/// Complete sword-fundamentals repertoire, attributed to the Author.
pub fn author_moveset() -> MovesetContract {
    crate::archetype_moveset::under_own_name(
        crate::pointed_polygon_moveset::pointed_polygon_moveset(),
        &["polygon", "pointed_polygon"],
        "author",
    )
}

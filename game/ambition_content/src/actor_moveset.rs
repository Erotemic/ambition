//! The Actor — the sword archetype's table, under her own name.
//!
//! ⭐ AND SHE CARRIES NO SWORD. The Pointed Polygon's frame data retargets onto
//! her for the reason the Author's does: his pen occupies the arming sword's
//! exact axis, and her conjured blade of stage light occupies it too — authored
//! as the swing's own axis extended past her hand, so the reach the table
//! assumes is the reach the sheet draws.
//!
//! ⚠ HER SPECIALS ARE NOT IN HERE YET. MONOLOGUE, THE LINE, THE TRAP and
//! CURTAIN CALL are authored as CLIPS and hit volumes in the sprite repository;
//! the rules — a trap door that moves her, a flyline that does not hit anyone —
//! are gameplay and belong in this table. Until someone writes them she borrows
//! the archetype's specials, the same way the Author does.
//!
//! See [`crate::archetype_moveset`] for why the ids are renamed rather than
//! shared or copied.

use ambition_platformer2d::entity_catalog::MovesetContract;

/// Complete sword-fundamentals repertoire, attributed to the Actor.
pub fn actor_moveset() -> MovesetContract {
    crate::archetype_moveset::under_own_name(
        crate::pointed_polygon_moveset::pointed_polygon_moveset(),
        &["polygon", "pointed_polygon"],
        "actor",
    )
}

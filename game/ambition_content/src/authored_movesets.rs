//! Every moveset THIS CRATE authors, in one list.
//!
//! ⛔⛔ NOT THE SELECTABLE CAST, and the distinction cost a real proof. The
//! shark's one-hit survivability census scanned this list and read as a
//! statement about the game; the Smash roster also seats Pointed, Projectile and
//! Pugnacious Polygon, the Author, the Actor, the Officer, the Medic, Mary-O and
//! Sanic, and none of them are here. Twenty-one fighters are selectable; this
//! list holds a subset, and a hand-kept subset narrows in silence because the
//! crate that owns the list cannot know a fighter was added somewhere else.
//!
//! ⭐ THE CAST HAS AN AUTHORITY AND IT IS NOT A TABLE: `SmashRoster::assemble`
//! against a live `PreparedCharacterRegistry`, then each prepared character's
//! `kit.projectable_moveset()`. It costs an app, which is why this list existed
//! — but a census is worth an app, and
//! `a_recovery_mount_cannot_be_deleted_by_one_hit` now pays it.
//!
//! ⇒ WHAT THIS LIST IS FOR is the question it can actually answer: does every
//! move THIS CRATE authors drive its own seam correctly (`moveset_sound`). That
//! subject and this list are the same thing by construction.

use ambition_entity_catalog::MovesetContract;

/// Every table in this crate that authors move events, by the name a failure
/// should print.
pub fn tables() -> Vec<(&'static str, MovesetContract)> {
    vec![
        ("alice", crate::alice_moveset::alice_moveset()),
        ("bob", crate::bob_moveset::bob_moveset()),
        (
            "carl_stargan",
            crate::carl_stargan_moveset::carl_stargan_moveset(),
        ),
        (
            "cellular_automaton",
            crate::cellular_automaton_moveset::cellular_pulse_moveset(),
        ),
        ("goblin", crate::goblin_moveset::goblin_moveset()),
        (
            "ninja_shadow_oni_leader",
            crate::ninja_shadow_oni_leader_moveset::ninja_shadow_oni_leader_moveset(),
        ),
        (
            "emmy_noether",
            crate::emmy_noether_moveset::emmy_noether_moveset(),
        ),
        ("oiler", crate::oiler_moveset::oiler_moveset()),
        (
            "patent_clerk",
            crate::patent_clerk_moveset::patent_clerk_moveset(),
        ),
        (
            "pirate_admiral",
            crate::pirate_admiral_moveset::pirate_admiral_moveset(),
        ),
        (
            "player_robot",
            crate::player_robot_moveset::player_robot_moveset(),
        ),
        (
            "theorem_chain",
            crate::player_robot_moveset::theorem_chain_moveset(),
        ),
    ]
}

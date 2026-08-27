//! Every moveset this crate authors, in one list.
//!
//! ⭐⭐ ONE LIST, TWO CUSTOMERS, and that is the whole reason it is not private
//! to the test that first needed it. A cast-wide census — "does any authored
//! move exceed X" — is only as honest as its subject, and a census that names
//! one fighter answers a question about one fighter while reading like a
//! statement about the game. The shark's one-hit survivability property was
//! asserted against the Pirate Admiral's moveset alone and was false for George
//! Booul the whole time (GPT 5.6, 2026-08-27).
//!
//! ⛔ IT IS STILL HAND-KEPT, and there is no registry to derive it from: a
//! fighter's repertoire is a Rust fn, and nothing maps character id → moveset
//! outside a running app. A new fighter added here is one line; a new fighter
//! NOT added here is a census that quietly narrows. ⚠ `ambition_demo_smash`
//! authors its own (George Booul), so a whole-cast census must add that crate's
//! too — see `a_recovery_mount_cannot_be_deleted_by_one_hit`.

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
        ("emmy_noether", crate::emmy_noether_moveset::emmy_noether_moveset()),
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

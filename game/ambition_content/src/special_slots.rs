//! Replacing one special in a table a fighter BORROWED.
//!
//! ⛔⛔ THE OLD MOVE LEAVES, IT IS NOT SHADOWED. A contract carries its moves in
//! a list and its bindings in a table, so re-pointing the verb and leaving the
//! old [`MoveSpec`] behind produces a table with an unreachable move in it —
//! which every census that walks `moves` then reports as part of this fighter's
//! kit.
//!
//! ⭐ THIS WAS THE AUTHOR'S PRIVATE HELPER. The third fighter to need it is the
//! point at which a copy becomes a rule, and a rule about which of three places
//! a move id lives in is exactly the kind that goes wrong quietly — see
//! [`MovesetContract::remap_move_ids`] for the same lesson learned once already.

use ambition_platformer2d::entity_catalog::{MoveSpec, MovesetContract};

/// Point `verb` at `replacement`, and remove whatever it displaced.
///
/// ⛔ THE DISPLACED MOVE ONLY GOES IF NOTHING ELSE BINDS IT. A table may
/// legitimately answer two verbs with one move — a down special that means the
/// same thing grounded and airborne is one move under two verbs — and dropping
/// it on the first replacement would leave the second verb pointing at nothing.
pub fn replace_special(set: &mut MovesetContract, verb: &str, replacement: MoveSpec) {
    let displaced = set.verbs.get(verb).cloned();
    set.verbs
        .insert(verb.to_string(), replacement.id.clone());
    if let Some(old) = displaced {
        let still_bound = set.verbs.values().any(|id| *id == old);
        if !still_bound {
            set.moves.retain(|m| m.id != old);
        }
    }
    set.moves.push(replacement);
}

/// Which verbs a fighter's four specials answer, for a test that wants to walk
/// them. `special_air_down` is the airborne half of the down slot; a fighter
/// whose down special means one thing in both postures still binds both.
pub const SPECIAL_VERBS: &[&str] = &[
    "special_neutral",
    "special_side",
    "special_down",
    "special_air_down",
    "special_up",
];

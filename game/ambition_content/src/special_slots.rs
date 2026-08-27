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

/// Which verbs a fighter's four specials answer.
///
/// ⛔⛔ THE NEUTRAL IS `special` AND THE SIDE IS `special_forward`. This list
/// shipped saying `special_neutral` and `special_side`, which are not names any
/// repertoire binds — `SmashRepertoire::into_contract` binds `special`,
/// `special_forward`, `special_up`, `special_down` and `special_air_down`, and
/// nothing pointed that out because nothing read the list. A
/// [`replace_special`] aimed at a name nothing answers to is the quiet failure
/// it invites: the new move goes into the table, a verb no press produces gets
/// bound to it, and the archetype's own special stays bound and keeps coming
/// out.
///
/// `special_air_down` is the airborne half of the down slot; a fighter whose
/// down special means one thing in both postures still binds both.
pub const SPECIAL_VERBS: &[&str] = &[
    "special",
    "special_forward",
    "special_down",
    "special_air_down",
    "special_up",
];

#[cfg(test)]
mod tests {
    /// ⛔ EVERY NAME IN [`super::SPECIAL_VERBS`] IS ONE A SHIPPED TABLE ACTUALLY
    /// BINDS. A constant listing verbs is only useful if it is the same
    /// vocabulary the repertoire lowers to, and the way that stops being true is
    /// silently — this list was wrong on two of five entries from the day it was
    /// written.
    #[test]
    fn every_special_verb_is_one_a_shipped_fighter_answers() {
        let brawler = crate::pugnacious_polygon_moveset::pugnacious_polygon_moveset();
        let pointed = crate::pointed_polygon_moveset::pointed_polygon_moveset();
        for verb in super::SPECIAL_VERBS {
            assert!(
                brawler.verbs.contains_key(*verb) || pointed.verbs.contains_key(*verb),
                "`{verb}` is not a verb either reference fighter binds, so a \
                 `replace_special` aimed at it would bind a press nobody makes"
            );
        }
    }
}

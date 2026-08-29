//! One fighter borrowing another's TIMINGS, under its own name.
//!
//! The two easter-egg fighters are the polygon archetypes' art with different
//! people drawn on it. Their sprite rigs say so in as many words — *"he binds
//! to the same humanoid motion library as the polygon reference fighters, so
//! his moveset is theirs until he earns bespoke posing"* — and every clip they
//! publish is the archetype's clip retargeted, frame for frame.
//!
//! ⛔ SO THE TABLE IS NOT COPIED. A second seven-hundred-line file that starts
//! byte-identical to the first is a table that drifts: the archetype gets tuned,
//! the copy does not, and nothing says they were ever supposed to agree.
//!
//! ⛔⛔ AND IT IS NOT SHARED VERBATIM EITHER. A move id is what a causal log
//! attributes a hit to, what a cue table addresses, and what a cancel window
//! names. Two fighters answering to `polygon_jab` are two fighters a trace
//! cannot tell apart. [`under_own_name`] is the whole difference: the frame
//! data stays the archetype's and the NAMES become the borrower's, so a fighter
//! that later wants its own jab replaces one move rather than forking a file.

use ambition_platformer2d::entity_catalog::MovesetContract;

/// The archetype's table with every move id re-prefixed for the fighter that
/// borrows it.
///
/// `archetype` is the set of prefixes the source table's ids carry; `owner`
/// replaces whichever one an id starts with. A SET and not one prefix, because
/// a shipped table uses more than one: the Pointed Polygon's normals are
/// `polygon_*` while its taunt and dash attack are `pointed_polygon_*`, and the
/// brawler's split the same way. Longest match wins, so a prefix that is
/// another's suffix cannot claim its ids.
///
/// Renames three things, because a move id appears in three places and missing
/// one is a press that resolves to nothing:
///
/// * `moves[].id` — the move itself,
/// * `verbs` — what a press resolves to,
/// * a `Cancelable` window's `into` list, when it names a move rather than a
///   verb class.
///
/// ⛔ PANICS on a move whose id carries none of the prefixes. A half-applied
/// rename is the failure this exists to prevent, and it is the kind that
/// surfaces as one dead button in a match rather than as a red test. It fired
/// the first time this ran, on exactly the two ids that break the pattern.
pub fn under_own_name(
    mut contract: MovesetContract,
    archetype: &[&str],
    owner: &str,
) -> MovesetContract {
    // ⭐⭐ THE TRAVERSAL IS THE SCHEMA'S, and this function is only the PREFIX
    // POLICY. It used to walk the three places a move id lives — `moves[].id`,
    // `verbs`, and a `Cancelable` window's `into` list — from a crate that
    // authors content, so every future id-bearing field on a `MoveSpec` was an
    // obligation on a file that would never hear about it.
    // `MovesetContract::remap_move_ids` owns the walk now, beside the type that
    // owns the fields.
    // Longest first: `polygon` is a prefix of nothing here, but `polygon` and
    // `polygon_brawler` are one edit away from being each other's problem.
    let mut prefixes: Vec<&str> = archetype.to_vec();
    prefixes.sort_by_key(|p| std::cmp::Reverse(p.len()));
    let rename = |id: &str| -> String {
        for prefix in &prefixes {
            if let Some(rest) = id.strip_prefix(prefix) {
                return format!("{owner}{rest}");
            }
        }
        panic!(
            "moveset id `{id}` carries none of the archetype prefixes {archetype:?}, so \
             renaming it for `{owner}` would leave the two fighters sharing a name"
        )
    };
    contract.remap_move_ids(rename);
    debug_assert_eq!(
        contract
            .moves
            .iter()
            .map(|mv| mv.id.as_str())
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        contract.moves.len(),
        "the rename collapsed two moves onto one id"
    );
    contract
}

/// The move id with any owner prefix removed, so a borrowed move can be found
/// beside the archetype's own.
///
/// Ids are `<owner>_<slot>`; the archetype's are `polygon_<slot>`. Taking
/// everything after the FIRST underscore is enough to pair them and is what the
/// rename itself does in reverse.
/// A move id with its owner prefix removed, so a renamed move can be recognised
/// as the same SLOT its archetype named.
///
/// ⛔⛔ THE PREFIXES ARE THE BORROWER'S OWN, PASSED IN. A hardcoded list paired
/// 23 of the Author's 26 moves (his archetype carries `polygon_` AND
/// `pointed_polygon_`) and 0 of the Officer's (whose archetype carries
/// `polygon_brawler_`). The prefixes each borrower renames are stated once, in
/// that borrower's own file, and asking for them is the only way this cannot
/// drift.
///
/// Longest first, so `pointed_polygon_` is not eaten by `polygon_`.
#[cfg(test)]
fn strip_owner_prefix<'a>(id: &'a str, prefixes: &[&str]) -> &'a str {
    let mut sorted: Vec<&str> = prefixes.to_vec();
    sorted.sort_by_key(|p| std::cmp::Reverse(p.len()));
    for prefix in sorted {
        if let Some(rest) = id.strip_prefix(&format!("{prefix}_")) {
            return rest;
        }
    }
    id
}

#[cfg(test)]
mod tests {
    /// EVERY BORROWED TABLE RENAMES CLEAN, and no easter egg answers to a name
    /// its archetype answers to.
    ///
    /// ⛔ THIS IS THE TEST THAT WAS MISSING. The first version of `under_own_name`
    /// took ONE prefix, and both shipped tables use two — their taunt and dash
    /// attack are named after the CHARACTER (`pointed_polygon_taunt`) while the
    /// rest are named after the archetype (`polygon_jab`). Nothing said so until
    /// the panic fired inside a headless boot, nineteen tests deep.
    #[test]
    fn a_borrowed_table_renames_every_id_and_collides_with_nothing() {
        for (borrowed, archetype, owner, prefixes, owned_slots) in [
            (
                crate::author_moveset::author_moveset(),
                crate::pointed_polygon_moveset::pointed_polygon_moveset(),
                "author",
                &["polygon", "pointed_polygon"][..],
                // His up-B is his own; everything else is the archetype's.
                1,
            ),
            (
                crate::officer_moveset::officer_moveset(),
                crate::pugnacious_polygon_moveset::pugnacious_polygon_moveset(),
                "officer",
                &["polygon_brawler", "pugnacious_polygon"][..],
                // His side-B is the draw; everything else is the archetype's.
                1,
            ),
            (
                crate::performer_moveset::performer_moveset(),
                crate::pointed_polygon_moveset::pointed_polygon_moveset(),
                "performer",
                &["polygon", "pointed_polygon"][..],
                // All four specials are hers, and the down slot is a posture
                // pair: five verbs she authored rather than borrowed.
                5,
            ),
            (
                crate::medic_moveset::medic_moveset(),
                crate::pugnacious_polygon_moveset::pugnacious_polygon_moveset(),
                "medic",
                &["polygon_brawler", "pugnacious_polygon"][..],
                // All four specials are hers, and the down slot is a posture
                // pair: five verbs she authored rather than borrowed.
                5,
            ),
        ] {
            assert_eq!(
                borrowed.moves.len(),
                archetype.moves.len(),
                "{owner} lost or gained a move in the rename"
            );
            for mv in &borrowed.moves {
                assert!(
                    mv.id.starts_with(owner),
                    "{owner} answers to `{}`, which is not its own name",
                    mv.id
                );
            }
            let theirs: std::collections::BTreeSet<&str> =
                archetype.moves.iter().map(|mv| mv.id.as_str()).collect();
            for mv in &borrowed.moves {
                assert!(
                    !theirs.contains(mv.id.as_str()),
                    "{owner} and its archetype both answer to `{}`",
                    mv.id
                );
            }
            // EVERY PRESS STILL RESOLVES. A rename that moved the ids and not the
            // verb table is a fighter with a full moveset and no buttons.
            assert_eq!(
                borrowed.verbs.len(),
                archetype.verbs.len(),
                "{owner} lost a verb binding"
            );
            let ids: std::collections::BTreeSet<&str> =
                borrowed.moves.iter().map(|mv| mv.id.as_str()).collect();
            for (verb, target) in &borrowed.verbs {
                assert!(
                    ids.contains(target.as_str()),
                    "{owner}'s `{verb}` resolves to `{target}`, which is not a move it has"
                );
            }
            // ...and the FRAME DATA is the archetype's, which is the whole point
            // of borrowing rather than copying.
            //
            // ⛔⛔ MATCHED BY THE VERB, NOT BY POSITION AND NOT BY ID. This used
            // to `zip` the two move lists, which reads as "the same moves in the
            // same order" and is only true while the borrower changes NOTHING:
            // the Author replaced his up-B with a teleport (2026-08-27) and every
            // move after the one he removed compared against its neighbour,
            // reporting a drift in `author_low_arc`, a move nobody touched.
            //
            // ⛔ AND NOT BY STRIPPED ID EITHER, which was the next thing tried:
            // the archetype's own ids carry two different owner prefixes
            // (`polygon_` and `pointed_polygon_`), so cutting at the first
            // underscore paired 23 of 26 and silently skipped the rest.
            //
            // ⭐ THE VERB IS THE EXACT PAIRING and it is a better statement
            // besides: the same BUTTON gives you the same frame data. A slot the
            // borrower deliberately owns is exempt by construction — its move has
            // nothing to have drifted from — and that is what "a fighter who
            // borrows a table may still own a slot in it" means.
            let mut compared = 0usize;
            for (verb, target) in &borrowed.verbs {
                let (Some(mine), Some(theirs)) = (
                    borrowed.moves.iter().find(|mv| mv.id == *target),
                    archetype
                        .verbs
                        .get(verb)
                        .and_then(|id| archetype.moves.iter().find(|mv| mv.id == *id)),
                ) else {
                    continue;
                };
                // The borrower's OWN move for this slot: a different move, not a
                // renamed one, so there is nothing to compare.
                if !mine.id.ends_with(super::strip_owner_prefix(&theirs.id, prefixes)) {
                    continue;
                }
                compared += 1;
                assert_eq!(
                    mine.duration_s, theirs.duration_s,
                    "{owner}'s `{verb}` (`{}`) drifted from the archetype's timing",
                    mine.id
                );
            }
            // ⛔⛔ EXACT, NOT A TOLERANCE. This used to allow a slack of two,
            // which was sized when one fighter owned one slot — and the moment
            // the Performer owned three it could not tell "she authored her own down
            // and up specials" from "the rename quietly stopped lining up",
            // which is the only thing this assertion exists to catch. A fighter
            // states how many slots are HERS and the rest must match.
            assert_eq!(
                compared + owned_slots,
                archetype.verbs.len(),
                "{owner} matched {compared} of the archetype's {} bound verbs \
                 and claims {owned_slots} of its own — a rename that stopped \
                 lining up makes this check vacuous",
                archetype.verbs.len()
            );
        }
    }
}

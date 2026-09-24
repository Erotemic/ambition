//! One fighter borrowing another's timings, under its own name.
//!
//! The easter-egg fighters are the polygon archetypes' art with different
//! people drawn on it: their rigs bind to the same humanoid motion library,
//! and every clip they publish is the archetype's clip, retargeted frame for
//! frame.
//!
//! So the table is not copied: a copy would drift when the archetype is
//! tuned.
//!
//! It is not shared verbatim either. A move id is what a causal log
//! attributes a hit to, what a cue table addresses, and what a cancel window
//! names; two fighters answering to `polygon_jab` cannot be told apart in a
//! trace. [`under_own_name`] keeps the archetype's frame data and gives the
//! moves the borrower's names, so a fighter can later replace one move
//! without forking a file.

use ambition_entity_catalog::MovesetContract;

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
/// Panics on a move whose id carries none of the prefixes. A half-applied
/// rename would show up as one dead button in a match, not as a red test.
pub fn under_own_name(
    mut contract: MovesetContract,
    archetype: &[&str],
    owner: &str,
) -> MovesetContract {
    // `MovesetContract::remap_move_ids` owns the traversal of id-bearing
    // fields, beside the type that owns them; this function is only the prefix
    // policy.
    // Longest first, so `polygon` cannot claim `polygon_brawler` ids.
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

/// A move id with its owner prefix removed, so a renamed move can be matched
/// to the slot its archetype named.
///
/// The prefixes are the borrower's own, passed in. The Director's archetype
/// uses `polygon_` and `pointed_polygon_`; the Officer's uses
/// `polygon_brawler_`. A hardcoded list would miss some.
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
    /// Every borrowed table renames cleanly, and no borrower answers to a name its
    /// archetype answers to.
    ///
    /// Shipped tables use two prefixes: the taunt and dash attack are named after
    /// the character (`pointed_polygon_taunt`), the rest after the archetype
    /// (`polygon_jab`).
    #[test]
    fn a_borrowed_table_renames_every_id_and_collides_with_nothing() {
        for (borrowed, archetype, owner, prefixes, owned_slots, retimed) in [
            (
                crate::director_moveset::director_moveset(),
                crate::pointed_polygon_moveset::pointed_polygon_moveset(),
                "director",
                &["polygon", "pointed_polygon"][..],
                // Three of his four specials are his own (the teleport up-B, the ambush
                // counter, the steered thought); the neutral is still the archetype's.
                // Declared, not derived, so taking a slot must be stated here.
                3,
                // No normal of his is re-timed: his frame data is the archetype's.
                &[][..],
            ),
            (
                crate::officer_moveset::officer_moveset(),
                crate::pugnacious_polygon_moveset::pugnacious_polygon_moveset(),
                "officer",
                &["polygon_brawler", "pugnacious_polygon"][..],
                // Three of four specials are his: the gust neutral, the draw side-B, the
                // riot shield down-B. The recovery is still the archetype's. Declared, so
                // the assertion below can be exact.
                3,
                // No normal of his is re-timed.
                &[][..],
            ),
            (
                crate::performer_moveset::performer_moveset(),
                crate::pointed_polygon_moveset::pointed_polygon_moveset(),
                "performer",
                &["polygon", "pointed_polygon"][..],
                // All four specials are hers, and the down slot is a posture
                // pair: five verbs she authored rather than borrowed.
                5,
                // Every normal is hers too, by timing rather than by id. Her tilts,
                // smashes and aerials are re-timed to her own clips (40 ms poses, extended
                // stage-light blades, 160–280 ms active, from `performer_stage_v1`), but
                // still use the archetype's names. A borrower that re-times a slot has
                // stopped borrowing it, so it is declared here like `owned_slots`.
                //
                // Her timing has its own test:
                // `normal_contact_windows_match_the_authored_light_and_pose_clock` checks it
                // against the animation clock.
                &[
                    "attack_air",
                    "attack_air_back",
                    "attack_air_down",
                    "attack_air_forward",
                    "attack_air_up",
                    "attack_down",
                    "attack_forward",
                    "attack_up",
                    "smash_down",
                    "smash_forward",
                    "smash_up",
                ][..],
            ),
            (
                crate::medic_moveset::medic_moveset(),
                crate::pugnacious_polygon_moveset::pugnacious_polygon_moveset(),
                "medic",
                &["polygon_brawler", "pugnacious_polygon"][..],
                // All four specials are hers, and the down slot is a posture
                // pair: five verbs she authored rather than borrowed.
                5,
                // No normal of hers is re-timed.
                &[][..],
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
            // Every press still resolves. Moving the ids without the verb table would
            // leave a full moveset with no buttons.
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
            // And the frame data is the archetype's.
            //
            // Match by verb, not by list position or stripped id. Zipping by position
            // breaks when a borrower replaces a move; stripping at the first underscore
            // fails because the archetype's ids use two prefixes. The same button giving
            // the same frame data is the exact pairing, and a slot the borrower owns is
            // exempt because its move has nothing to drift from.
            let mut compared = 0usize;
            let mut retimed_seen = 0usize;
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
                // The borrower's own move for this slot, not a renamed one: nothing to
                // compare.
                if !mine.id.ends_with(super::strip_owner_prefix(&theirs.id, prefixes)) {
                    continue;
                }
                // A slot the borrower re-timed on purpose. Assert it still differs: if the
                // archetype is re-timed to match, the stale exemption would hide the next
                // real drift.
                if retimed.contains(&verb.as_str()) {
                    retimed_seen += 1;
                    assert_ne!(
                        mine.duration_s, theirs.duration_s,
                        "{owner}'s `{verb}` (`{}`) is declared re-timed and now \
                         matches the archetype again — take it off the list, or \
                         the exemption is hiding the next drift",
                        mine.id
                    );
                    continue;
                }
                compared += 1;
                assert_eq!(
                    mine.duration_s, theirs.duration_s,
                    "{owner}'s `{verb}` (`{}`) drifted from the archetype's timing",
                    mine.id
                );
            }
            // Exact, not a tolerance: a fighter states how many slots are hers, and a
            // tolerance could not tell "she authored her own specials" from "the rename
            // stopped lining up".
            assert_eq!(
                compared + owned_slots + retimed_seen,
                archetype.verbs.len(),
                "{owner} matched {compared} of the archetype's {} bound verbs, \
                 claims {owned_slots} of its own and {retimed_seen} re-timed — a \
                 rename that stopped lining up makes this check vacuous",
                archetype.verbs.len()
            );
            // Every declared re-timed verb was reached, so a typo or dropped verb does
            // not sit in the list exempting nothing.
            assert_eq!(
                retimed_seen,
                retimed.len(),
                "{owner} declares {} re-timed verbs and the walk reached \
                 {retimed_seen} of them, so a declared name binds nothing",
                retimed.len()
            );
        }
    }
}

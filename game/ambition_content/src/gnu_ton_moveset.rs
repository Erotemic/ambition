//! Tests of the moves the game ships for `npc_gnu_ton`: the scholar GNU-ton as
//! a fighter.
//!
//! The table is content: `assets/data/movesets/gnu_ton.ron`. These tests read
//! it through [`crate::authored_movesets::shipped`], the table the pack loads.
//! The same table is what he fights with on the giant's back in his boss
//! fight (`crate::bosses::gnu_ton::duel`).

#[cfg(test)]
mod tests {
    use ambition_entity_catalog::WindowTag;

    const FIGHTER: &str = crate::bosses::gnu_ton::duel::FIGHTER;

    fn spec_len(table: &ambition_entity_catalog::MovesetContract, id: &str) -> f32 {
        table.move_by_id(id).map_or(f32::MAX, |spec| spec.duration_s)
    }

    /// Each press of the platform-fighter kit has a move of his own: no press
    /// falls through to a jab, and no throw is a dead input.
    #[test]
    fn he_answers_each_verb_of_the_repertoire() {
        let table = crate::authored_movesets::shipped(FIGHTER);
        for verb in ambition_entity_catalog::smash_repertoire::REPERTOIRE_VERBS {
            let id = table.verbs.get(*verb).unwrap_or_else(|| panic!("no move for the verb `{verb}`"));
            assert!(table.move_by_id(id).is_some(), "the verb `{verb}` names `{id}`, which is not in his table");
        }
    }

    /// The moves the boss chooses on the giant's back are moves of this table,
    /// and each can hurt: the conductor module names them by id, and the two
    /// crates do not see each other's lists.
    #[test]
    fn each_move_of_the_duel_is_a_strike_of_his_table() {
        let table = crate::authored_movesets::shipped(FIGHTER);
        for (id, rest, open) in ambition_content_modules::gnu_ton::choreography::arms::DUEL_MOVES {
            // He is open for the whole of the move, and for a time after it.
            assert!(open > spec_len(&table, id), "his book comes up before `{id}` is over");
            assert!(open < rest, "he chooses again before his book comes up after `{id}`");
            let spec = table.move_by_id(id).unwrap_or_else(|| panic!("the duel names `{id}`, which is not in his table"));
            let strikes = spec.windows.iter().any(|w| matches!(w.tag, WindowTag::Active) && !w.volumes.is_empty());
            assert!(strikes, "`{id}` has no live volume: he would swing and hurt nothing");
            // He chooses again after the move is over, and a rest.
            assert!(
                rest > spec.duration_s,
                "he chooses again {rest} s after `{id}`, and the move is {} s long: no rest to punish",
                spec.duration_s
            );
        }
    }
}

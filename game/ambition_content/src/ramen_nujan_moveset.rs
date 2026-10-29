//! Tests of the moves the game ships for `npc_ramen_nujan`: Ramen-nujan as a
//! fighter.
//!
//! The table is content: `assets/data/movesets/ramen_nujan.ron`. These tests
//! read it through [`crate::authored_movesets::shipped`], the table the pack
//! loads.

#[cfg(test)]
mod tests {
    use ambition_entity_catalog::{MoveSpec, WindowTag};

    const FIGHTER: &str = "npc_ramen_nujan";

    /// The live windows of `spec` that can hurt, in the order of their start.
    fn blows(spec: &MoveSpec) -> Vec<(f32, f32)> {
        let mut live: Vec<(f32, f32)> =
            spec.windows.iter().filter(|w| matches!(w.tag, WindowTag::Active) && !w.volumes.is_empty()).map(|w| (w.start_s, w.end_s)).collect();
        live.sort_by(|a, b| a.0.total_cmp(&b.0));
        live
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

    /// Every clip he names is a row his sheet carries: a move whose row is not
    /// there draws a fallback row, and nothing says so.
    #[test]
    fn every_clip_names_a_row_his_sheet_carries() {
    let sprite_content = ambition_platformer2d::sprite_sheet::test_support::shared_sprites();
        let table = crate::authored_movesets::shipped(FIGHTER);
        let record = ambition_platformer2d::sprite_sheet::character::sheets::record_for_sheet_key(&sprite_content, "ramen_nujan")
            .expect("his sheet is baked into the registry");
        let rows: std::collections::BTreeSet<&str> = record.rows.iter().map(|r| r.animation.as_str()).collect();
        for spec in &table.moves {
            assert!(rows.contains(spec.clip.clip.as_str()), "`{}` draws `{}`, which his sheet does not publish", spec.id, spec.clip.clip);
        }
    }

    /// His jab is a string of three: each of the first two can be left for
    /// the next when it lands, and the next is a move of his table.
    #[test]
    fn his_jab_is_a_string_of_three() {
        let table = crate::authored_movesets::shipped(FIGHTER);
        for (from, to) in [("jab", "jab_2"), ("jab_2", "jab_3")] {
            let spec = table.move_by_id(from).unwrap_or_else(|| panic!("no `{from}`"));
            let leaves_for = spec.windows.iter().any(|w| matches!(&w.tag, WindowTag::Cancelable { into, .. } if into.iter().any(|id| id == to)));
            assert!(leaves_for, "`{from}` can not be left for `{to}`");
            assert!(table.move_by_id(to).is_some(), "`{from}` is left for `{to}`, which is not in his table");
        }
    }

    /// 1729 is two blows. A second blow of one move strikes a body the first
    /// struck only when there is a gap between their windows.
    #[test]
    fn the_taxicab_is_two_blows_with_a_gap_between_them() {
        let table = crate::authored_movesets::shipped(FIGHTER);
        let spec = table.move_by_id("taxicab").expect("his side special");
        let live = blows(spec);
        assert_eq!(live.len(), 2, "1729 is the sum of two cubes two ways: {live:?}");
        assert!(live[1].0 > live[0].1, "no gap between the two blows: the second can not strike what the first struck ({live:?})");
    }

    /// Each blow is over before its move is: a volume that is live after the
    /// move ended is one the fighter can not be punished for.
    #[test]
    fn each_blow_is_inside_its_move() {
        let table = crate::authored_movesets::shipped(FIGHTER);
        for spec in &table.moves {
            for (start, end) in blows(spec) {
                assert!(start < end && end <= spec.duration_s + 1e-4, "`{}` is {} s long and is live from {start} to {end}", spec.id, spec.duration_s);
            }
        }
    }
}

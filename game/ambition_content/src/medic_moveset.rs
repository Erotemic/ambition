//! Tests of the moves the game ships for `medic`.
//!
//! The table is content: `assets/data/movesets/medic.ron`. These tests read it
//! through [`crate::authored_movesets::shipped`], the table the pack loads.



use ambition_entity_catalog::MovesetContract;

#[cfg(test)]
mod tourniquet_tests {

    /// The drag does not weaken as the victim takes damage.
    ///
    /// `strike` stores a builder growth of zero as `None`, and `None` means the
    /// ruleset's growth applies. This asserts `Some(0.0)`, not "small": a
    /// magnitude check would pass a plausible small growth.
    #[test]
    fn the_tourniquet_pulls_the_same_at_every_percent() {
        let strap = crate::authored_movesets::shipped("medic")
            .move_by_id("medic_tourniquet")
            .expect("medic_tourniquet exists")
            .clone();
        let volumes: Vec<_> = strap.windows.iter().flat_map(|w| w.volumes.iter()).collect();
        assert!(!volumes.is_empty(), "the strap still has a hitbox");
        for volume in volumes {
            assert_eq!(
                volume.knockback_growth,
                Some(0.0),
                "`None` here means the stage decides, which is the bug this \
                 move's own comment describes"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_entity_catalog::smash_vitality::{VitalityParams, VITALITY};
    use ambition_entity_catalog::MoveEventKind;

    fn vitality_of(set: &MovesetContract, id: &str) -> VitalityParams {
        set.moves
            .iter()
            .find(|m| m.id == id)
            .unwrap_or_else(|| panic!("`{id}` is in the table"))
            .events
            .iter()
            .find_map(|ev| match &ev.kind {
                MoveEventKind::Effect(effect) if effect.key == VITALITY => {
                    Some(effect.params.hydrate().expect("vitality params hydrate"))
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("`{id}` authors a health change"))
    }

    /// All five special verbs are hers. `special_air_down` comes before
    /// `special_down` in the brawler's chain, so replacing only the grounded half
    /// would send an airborne press to the archetype's move.
    #[test]
    fn every_special_slot_answers_to_a_move_of_her_own() {
        let set = crate::authored_movesets::shipped("medic");
        for (verb, expected) in [
            ("special", "medic_adrenaline"),
            ("special_forward", "medic_tourniquet"),
            ("special_down", "medic_field_dressing"),
            ("special_air_down", "medic_field_dressing_air"),
            ("special_up", "medic_rescue_lift"),
        ] {
            assert_eq!(
                set.verbs.get(verb).map(String::as_str),
                Some(expected),
                "`{verb}` must answer to her own move"
            );
        }
        assert!(
            !set.moves.iter().any(|m| m.id.starts_with("medic_polygon")
                || m.id.contains("shoulderrush")
                || m.id.contains("uppercut")),
            "the archetype's specials left the table, they were not shadowed"
        );
    }

    /// She pays for one and is paid by the other. One technique serves both, so
    /// the risk is a sign error: an Adrenaline that healed would be a free press
    /// with a cancel window.
    #[test]
    fn adrenaline_costs_and_the_dressing_gives_back() {
        let set = crate::authored_movesets::shipped("medic");
        let cost = vitality_of(&set, "medic_adrenaline");
        assert!(
            cost.change < 0,
            "adrenaline is a PRICE, and it authored {}",
            cost.change
        );
        let heal = vitality_of(&set, "medic_field_dressing");
        assert!(
            heal.change > 0,
            "the dressing is a RESTORE, and it authored {}",
            heal.change
        );
        assert_eq!(
            vitality_of(&set, "medic_field_dressing_air").change,
            heal.change,
            "both halves of the down slot mend the same amount"
        );
        // The exchange rate: one heal costs more frames than the presses it
        // repays.
        assert!(
            heal.change > -cost.change,
            "a dressing that gave back less than one adrenaline costs makes the \
             kit a treadmill"
        );
    }

    /// The floor is never zero on a price. `VitalityParams::floor` defaults to
    /// `0` and the engine clamps it to `1`, but content should not rely on a clamp
    /// elsewhere to disagree with it.
    #[test]
    fn her_price_names_a_floor_that_cannot_kill_her() {
        let set = crate::authored_movesets::shipped("medic");
        assert!(
            vitality_of(&set, "medic_adrenaline").floor >= 1,
            "a neutral special that can finish a stock by being pressed is not \
             a cost"
        );
    }

    /// The strap pulls. With the default launch direction it would send the
    /// victim away, and nothing else would look wrong.
    #[test]
    fn the_tourniquet_launches_its_victim_toward_her() {
        let set = crate::authored_movesets::shipped("medic");
        let strap = set
            .moves
            .iter()
            .find(|m| m.id == "medic_tourniquet")
            .expect("the tourniquet");
        let dirs: Vec<(f32, f32)> = strap
            .windows
            .iter()
            .flat_map(|w| w.volumes.iter())
            .filter_map(|v| v.launch_dir)
            .collect();
        assert!(!dirs.is_empty(), "the strap has a live volume");
        for dir in dirs {
            assert!(
                dir.0 < 0.0,
                "the strap must launch back along her own axis, and it authored {dir:?}"
            );
        }
    }

    /// The recovery is free: a price on a recovery is only collected at the
    /// ledge.
    #[test]
    fn the_rescue_lift_costs_her_nothing() {
        let set = crate::authored_movesets::shipped("medic");
        let lift = set
            .moves
            .iter()
            .find(|m| m.id == "medic_rescue_lift")
            .expect("the lift");
        assert!(
            !lift.events.iter().any(|ev| matches!(
                &ev.kind,
                MoveEventKind::Effect(effect) if effect.key == VITALITY
            )),
            "her way home is the one move that does not take health"
        );
    }
}

//! Tests of the moves the game ships for `perfect_cellular_automaton`.
//!
//! The table is content: `assets/data/movesets/cellular_automaton.ron`. These tests read it
//! through [`crate::authored_movesets::shipped`], the table the pack loads.




use ambition_entity_catalog::{
    MovesetContract, WindowTag,
};


#[cfg(test)]
mod tests {
    use super::*;

    // Verb binding is checked by construction: `SmashRepertoire` owns the verb
    // strings and is a struct with no `Default`, so a missing slot is a compile
    // error. Coverage in every posture is checked by
    // `ambition_entity_catalog::smash_repertoire` and by
    // `smash_roster_movesets::report_the_smash_kit_every_selectable_fighter_has`.

    /// The pulse keeps the archetype row's numbers: a 0.40s tell, a 0.14s window,
    /// 3 damage, 140 flat knockback.
    #[test]
    fn the_signature_move_still_carries_the_rows_verbatim_numbers() {
        let moveset = crate::authored_movesets::shipped("perfect_cellular_automaton");
        let pulse = moveset
            .move_by_id("cellular_pulse")
            .expect("the signature move");
        assert_eq!(pulse.duration_s, 0.85);
        let active = pulse
            .windows
            .iter()
            .find(|w| matches!(w.tag, WindowTag::Active))
            .expect("the pulse has an active window");
        assert_eq!((active.start_s, active.end_s), (0.40, 0.54));
        let volume = active.volumes.first().expect("one forward volume");
        assert_eq!(volume.damage, 3);
        assert_eq!(volume.knockback, 140.0);
        assert_eq!(
            volume.knockback_growth, None,
            "the row authors no growth, so the STAGE decides it"
        );
    }

    /// A boss telegraphs: its heaviest reads start slower than the goblin's whole
    /// jab. Pinned against a fighter built for this stage, not a constant.
    #[test]
    fn its_smashes_telegraph_more_than_a_fighters_do() {
        let pca = crate::authored_movesets::shipped("perfect_cellular_automaton");
        let goblin = crate::authored_movesets::shipped("goblin");
        let startup = |set: &MovesetContract, id: &str| {
            set.move_by_id(id)
                .unwrap_or_else(|| panic!("{id} exists"))
                .windows
                .iter()
                .find(|w| matches!(w.tag, WindowTag::Active))
                .expect("a strike has an active window")
                .start_s
        };
        assert!(
            startup(&pca, "generation_wipe") > startup(&goblin, "smash_forward"),
            "the automaton's kill move comes out faster than the goblin's, so it \
             is a fighter with a boss's health rather than a boss"
        );
    }

    /// The collapse must converge. Its only mechanic was once a strike with
    /// `launch_dir: (0.7, -0.68)`, which throws victims away; this test keeps the
    /// move gathering.
    #[test]
    fn the_generation_collapse_gathers_before_it_launches() {
        let set = crate::authored_movesets::shipped("perfect_cellular_automaton");
        let collapse = set
            .moves
            .iter()
            .find(|m| m.id == "generation_collapse")
            .expect("it has a grounded down special");

        let holds: Vec<ambition_entity_catalog::AutolinkVolume> = collapse
            .windows
            .iter()
            .flat_map(|w| w.volumes.iter())
            .filter_map(|v| match v.reaction {
                Some(ambition_entity_catalog::VolumeReaction::Autolink(a)) => {
                    Some(a)
                }
                _ => None,
            })
            .collect();
        assert!(
            holds.len() >= 2,
            "a cone that closes has more than one generation: {} holding pulse(s)",
            holds.len()
        );

        // The anchor's x is zero: `autolink_anchor_world` mirrors it with the
        // attacker's facing, so a non-zero x would make the gather point depend on
        // facing.
        for hold in &holds {
            assert_eq!(
                hold.anchor.0, 0.0,
                "the gather point moves with facing, so the collapse is a poke"
            );
            assert!(hold.pull > 0.0, "a hold that does not pull holds nothing");
        }

        // The finisher still launches. A move that only gathered would never let
        // go.
        assert!(
            collapse
                .windows
                .iter()
                .flat_map(|w| w.volumes.iter())
                .any(|v| v.reaction.is_none() && v.damage > 0),
            "nothing in the collapse launches, so it gathers forever"
        );
    }
}

//! Tests of the moves the game ships for `special_patent_clerk`.
//!
//! The table is content: `assets/data/movesets/patent_clerk.ron`. These tests read it
//! through [`crate::authored_movesets::shipped`], the table the pack loads.




use ambition_entity_catalog::MovesetContract;


#[cfg(test)]
mod tests {

    /// The same launch for every observer. `fixed_knockback` zeroes the growth on
    /// every volume of `light_argument`.
    ///
    /// Asserts zero, not "small": `growth < 1.0` would pass a move that still
    /// scales.
    #[test]
    fn the_light_argument_launches_the_same_at_every_percent() {
        let set = crate::authored_movesets::shipped("special_patent_clerk");
        let argument = set
            .move_by_id("light_argument")
            .expect("light_argument exists");
        let volumes: Vec<_> = argument
            .windows
            .iter()
            .flat_map(|w| w.volumes.iter())
            .collect();
        assert!(!volumes.is_empty(), "it is still a strike");
        for volume in volumes {
            assert_eq!(
                volume.knockback_growth,
                Some(0.0),
                "a frame-dependent launch is the one thing the postulate forbids"
            );
        }
    }
    /// The clerk's counter desyncs the attacker's clock.
    ///
    /// The `answers_the_attacker` assertion is the key: a Witch-Time that slowed
    /// its own caster would be a self-inflicted stun.
    #[test]
    fn the_clerks_windup_desyncs_the_clock_of_whoever_swings_into_it() {
        use ambition_entity_catalog::smash_counter::{CounterParams, COUNTER};
        use ambition_entity_catalog::smash_time_dilation::{
            TimeDilationParams, TIME_DILATION,
        };
        let set = crate::authored_movesets::shipped("special_patent_clerk");
        let down = set
            .moves
            .iter()
            .find(|m| m.id == "synchronize_clocks")
            .expect("his down-B is in the table");
        let stance = down
            .windows
            .iter()
            .find_map(|w| {
                let effect = w.sustain_effect.as_ref()?;
                (effect.key == COUNTER).then(|| effect.params.hydrate::<CounterParams>())
            })
            .expect("his down-B holds no counter stance, so `clock_desync` is art again")
            .expect("the stance's params hydrate");

        assert!(
            stance.answers_the_attacker,
            "the stance answers its OWNER — a Witch-Time that slows its own caster \
             is a self-inflicted stun"
        );
        assert_eq!(
            stance.response, TIME_DILATION,
            "the stance answers with {} rather than a dilation",
            stance.response
        );
        let dilation = stance
            .response_params
            .hydrate::<TimeDilationParams>()
            .expect("the dilation params hydrate");
        assert!(
            dilation.problems().is_empty(),
            "the authored dilation is not one: {:?}",
            dilation.problems()
        );
        // A read, not a stun: long enough to punish, short enough that the slowed
        // fighter is still playing.
        assert!(
            dilation.seconds > 0.2 && dilation.seconds < 1.0,
            "a {}s slow is not a read", dilation.seconds
        );
    }

    use super::*;
    use ambition_entity_catalog::{MoveSpec, WindowTag};

    fn find(set: &MovesetContract, id: &str) -> MoveSpec {
        set.moves
            .iter()
            .find(|m| m.id == id)
            .unwrap_or_else(|| panic!("{id} exists"))
            .clone()
    }

    fn startup(m: &MoveSpec) -> f32 {
        m.windows
            .iter()
            .find(|w| matches!(w.tag, WindowTag::Active))
            .expect("a strike has an active window")
            .start_s
    }

    fn damage(m: &MoveSpec) -> i32 {
        m.windows
            .iter()
            .flat_map(|w| w.volumes.iter())
            .map(|v| v.damage)
            .max()
            .unwrap_or(0)
    }

    fn launch(m: &MoveSpec) -> f32 {
        m.windows
            .iter()
            .flat_map(|w| w.volumes.iter())
            .map(|v| v.knockback)
            .fold(0.0f32, f32::max)
    }

    // Verb binding is checked by construction: `SmashRepertoire` owns the verb
    // strings and is a struct with no `Default`, so a missing slot is a compile
    // error. Coverage in every posture is checked by
    // `ambition_entity_catalog::smash_repertoire` and by
    // `smash_roster_movesets::report_the_smash_kit_every_selectable_fighter_has`.

    /// Heavyweight with finishers: the table must mean it.
    #[test]
    fn the_clerk_is_slower_and_hits_harder_than_the_admiral() {
        let clerk = crate::authored_movesets::shipped("special_patent_clerk");
        let admiral = crate::authored_movesets::shipped("npc_pirate_admiral");

        assert!(
            startup(&find(&clerk, "jab")) > startup(&find(&admiral, "jab")),
            "the heaviest body has the slowest fast option"
        );
        assert!(
            startup(&find(&clerk, "smash_forward")) > startup(&find(&admiral, "smash_forward")),
            "and the longest commitment on its kill move"
        );
        assert!(
            damage(&find(&clerk, "smash_forward")) > damage(&find(&admiral, "smash_forward")),
            "which is what it is paid for"
        );
    }

    /// Controller: the tilts set up and do not finish. A tilt as strong as a
    /// smash would make the smash pointless, so the gap is asserted.
    #[test]
    fn the_tilts_set_up_and_the_smashes_finish() {
        let clerk = crate::authored_movesets::shipped("special_patent_clerk");
        let strongest_tilt = ["tilt_up", "tilt_down"]
            .into_iter()
            .map(|id| launch(&find(&clerk, id)))
            .fold(0.0f32, f32::max);
        let weakest_smash = ["smash_forward", "smash_up", "smash_down"]
            .into_iter()
            .map(|id| launch(&find(&clerk, id)))
            .fold(f32::MAX, f32::min);
        assert!(
            strongest_tilt * 2.0 < weakest_smash,
            "a controller's tilts must be worth less than half its finishers \
             ({strongest_tilt} vs {weakest_smash}), or the finisher is decoration"
        );
    }

    /// Armour covers the crossing and nothing on either side. Armour over the
    /// whole move would make his startup and locked tail unpunishable, which is
    /// the price the pass must pay.
    #[test]
    fn his_pass_is_armoured_only_while_he_is_crossing() {
        let set = crate::authored_movesets::shipped("special_patent_clerk");
        let pass = find(&set, "reference_frame");
        let armor: Vec<(f32, f32)> = pass
            .windows
            .iter()
            .filter(|w| matches!(w.tag, WindowTag::Armor))
            .map(|w| (w.start_s, w.end_s))
            .collect();
        assert_eq!(armor.len(), 1, "expected exactly one armour window: {armor:?}");
        let (start, end) = armor[0];

        // The impulse fires at 0.20 and the active window ends at 0.31.
        assert!(
            (start - 0.20).abs() < 1e-4 && (end - 0.31).abs() < 1e-4,
            "armour runs {start}s..{end}s, not the pass"
        );
        // The startup is still punishable.
        assert!(start > 0.0, "armour covers his wind-up");
        // The locked tail is still a free hit: `committed_tail` runs the move to
        // 0.66s, so armour ending at 0.31 leaves a third of a second.
        assert!(
            end < pass.duration_s,
            "armour runs to {end}s on a {}s move, so his recovery is covered too",
            pass.duration_s
        );
    }

    /// He is still hit: armour is not i-frames. An added `Invuln` window would
    /// pass the test above.
    #[test]
    fn the_armoured_pass_grants_no_invulnerability() {
        let set = crate::authored_movesets::shipped("special_patent_clerk");
        let pass = find(&set, "reference_frame");
        assert!(
            !pass
                .windows
                .iter()
                .any(|w| matches!(w.tag, WindowTag::Invuln)),
            "the pass carries i-frames, so he is not taking the hit at all"
        );
    }
}

//! The player robot's actions, and the tests that pin its two move tables.
//!
//! The move tables are content. `assets/data/movesets/player_robot.ron`
//! carries v3's platform-fighter table and v2's theorem chain, and the lineage
//! takes them through the pack seam (`authored_intrinsics`) like every other
//! fighter. The tests here read the table the pack ships, so they guard the
//! file the game plays.
//!
//! A move states what it is, never what a mode does with it. Startup, active
//! frames, recovery, hitbox geometry, damage, base launch, growth, landing lag
//! and auto-cancel belong to the swing. Percent, stocks, blast zones, DI and
//! knockback-growth strength belong to the ruleset, declared per stage
//! (`DeclaredCombatRules`). So Ambition reads this table as Hollow-Knight
//! combat and Smash reads it as a platform fighter.

#[cfg(test)]
use crate::authored_movesets::shipped;

#[cfg(test)]
mod stabilizer_tests {
    use super::*;

    /// It plants itself and takes the hit, but only while it is planting.
    ///
    /// The end of the armour is the test. Armour over the active frames would
    /// remove the whiff risk. A guard that only found a `WindowTag::Armor` window
    /// would pass against that move.
    #[test]
    fn the_stabilizer_slam_is_armoured_only_while_it_plants() {
        use ambition_entity_catalog::WindowTag;
        let slam = shipped("player_robot_v3")
            .move_by_id("stabilizer_slam")
            .expect("stabilizer_slam exists")
            .clone();
        let armor = slam
            .windows
            .iter()
            .find(|w| w.tag == WindowTag::Armor)
            .expect("a machine that plants itself has armour");
        let active = slam
            .windows
            .iter()
            .find(|w| w.tag == WindowTag::Active && !w.volumes.is_empty())
            .expect("the floor still answers");
        assert!(
            armor.end_s <= active.start_s,
            "armour must close as the floor answers: armour ends {}, hit opens {}",
            armor.end_s,
            active.start_s,
        );
        assert!(armor.start_s > 0.0, "anything faster must still beat it");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Verb binding is checked by construction: `SmashRepertoire` owns the verb
    // strings and is a struct with no `Default`, so a missing slot is a compile
    // error. Coverage in every posture is checked by
    // `ambition_entity_catalog::smash_repertoire` and by
    // `smash_roster_movesets::report_the_smash_kit_every_selectable_fighter_has`.

    /// The protagonist states its own verbs, so a match stops guessing.
    ///
    /// A character that authors no verbs takes the migration bridge in
    /// `seat_abilities`: the mode's declared set, stamped on verbatim. The robot
    /// authors its own because it is the body both games share.
    ///
    /// `reset` must be absent: it is a debug affordance, and a character that
    /// authored it would give every game that seats the robot a way to teleport
    /// home.
    ///
    /// `fly` is present. The robot is a grounded-base hybrid that takes to the
    /// air for vertical space, and the duel arena's exhibition robot uses it.
    #[test]
    fn the_robot_authors_its_verbs_rather_than_taking_a_match_s_word_for_them() {
        let v3 = crate::player_robot_lineage::definition(&crate::player_robot_lineage::V3);
        let verbs = v3.abilities.expect("v3 states what its body can do");
        assert!(verbs.jump && verbs.dash && verbs.attack && verbs.shield && verbs.dodge);
        assert!(verbs.blink, "blinking is what the robot IS");
        assert!(verbs.fly, "the grounded-base hybrid lost its fly toggle");
        assert!(
            !verbs.reset,
            "a debug affordance became part of the character, so every game that \
             seats the robot now receives a way to teleport home"
        );

        // A retired incarnation shares the verbs, not the moves. v0, v2 and v3 are
        // one robot at three ages, so its body's abilities belong to the lineage:
        // the duel arena fields v2, which must blink and dash. The current frame
        // data is v3's alone; giving a retired incarnation today's timings would
        // invent content.
        let v2 = crate::player_robot_lineage::definition(&crate::player_robot_lineage::V2);
        assert!(
            v2.abilities.is_some_and(|verbs| verbs.blink && verbs.dash),
            "the exhibition robot lost the verbs its archetype row granted it"
        );
        assert!(
            v2.moveset
                .as_ref()
                .is_some_and(|set| set.move_for_verb("special").is_some()),
            "v2 lost the theorem chain, which is the only proof in the repo that \
             a moveset expresses a multi-hit combo as data"
        );
        assert!(
            v2.moveset
                .as_ref()
                .is_some_and(|set| set.move_for_verb("smash_forward").is_none()),
            "v2 was handed v3's platform-fighter table"
        );
    }

    /// A move can be a combo, as data. The second hit must hurt more, or the pair
    /// is a stutter, not a chain.
    #[test]
    fn the_theorem_chain_is_two_hits_on_one_timeline() {
        use ambition_entity_catalog::WindowTag;
        let set = shipped("player_robot_v2");
        let mv = set
            .move_for_verb("special")
            .expect("the chain is bound to the special verb");
        assert_eq!(mv.id, "theorem_chain");
        let hits: Vec<i32> = mv
            .windows
            .iter()
            .filter(|w| matches!(w.tag, WindowTag::Active) && !w.volumes.is_empty())
            .map(|w| w.volumes[0].damage)
            .collect();
        assert_eq!(
            hits.len(),
            2,
            "a chain with one Active window is a swing: {hits:?}"
        );
        assert!(
            hits[1] > hits[0],
            "the follow-up does not hit harder than the poke, so the pair is a \
             stutter rather than a chain: {hits:?}"
        );
    }

    /// The robot's projectile has its own look, stated by the character.
    ///
    /// The character-first constructor once wrote an empty string here, so the
    /// robot fired a plain rock instead of the Hadouken.
    #[test]
    fn the_robot_states_what_its_projectile_looks_like() {
        for incarnation in crate::player_robot_lineage::LINEAGE {
            let definition = crate::player_robot_lineage::definition(incarnation);
            assert_eq!(
                definition.ranged_vfx.as_deref(),
                Some("hadouken"),
                "`{}` fires an unadorned projectile",
                incarnation.id
            );
        }
    }

    /// The repertoire is a smash table, and its d-air shows it.
    ///
    /// Same press, same geometry, two readings; only the mode chooses. If someone
    /// retunes the spike so it no longer points down, this test tells them that
    /// another game reads that direction.
    #[test]
    fn the_down_air_is_a_spike_which_is_what_a_pogo_mode_has_to_reinterpret() {
        let set = shipped("player_robot_v3");
        let d_air = set
            .move_for_verb("attack_air_down")
            .expect("the repertoire binds a down-air");
        let launch = d_air
            .windows
            .iter()
            .flat_map(|window| window.volumes.iter())
            .find_map(|volume| volume.launch_dir)
            .expect("the spike states its direction rather than deriving it");
        assert!(
            launch.1 > 0.0,
            "the d-air stopped pointing down, so it is no longer the spike the \
             pogo mode has to reinterpret: {launch:?}"
        );
    }
}

#[cfg(test)]
mod clip_binding_tests {
    /// Every canonical robot move asks for its own row, not a shared `"attack"`.
    ///
    /// This asserts the request, not the drawing. Whether a row exists depends on
    /// the sheet (`SheetRecord::first_bound_row`); what a character asks for is a
    /// fact about the character.
    #[test]
    fn every_canonical_move_names_its_own_clip() {
        let moveset = super::shipped("player_robot_v3");
        for (id, clip) in [
            ("jab", "jab"),
            ("tilt_up", "attack_up"),
            ("tilt_down", "attack_down"),
            ("smash_forward", "smash_forward"),
            ("smash_up", "smash_up"),
            ("smash_down", "smash_down"),
            ("air_neutral", "air_neutral"),
            ("air_forward", "air_forward"),
            ("air_back", "air_back"),
            ("air_up", "air_up"),
            ("air_down", "air_down"),
        ] {
            let spec = moveset
                .moves
                .iter()
                .find(|m| m.id == id)
                .unwrap_or_else(|| panic!("the robot authors no move `{id}`"));
            assert_eq!(
                spec.clip.clip, clip,
                "`{id}` asks for `{}` — a sheet that draws eleven distinct moves \
                 will draw one",
                spec.clip.clip
            );
            // and the chain must still reach a sheet that has none of them.
            assert!(
                spec.clip.fallbacks.iter().any(|f| f == "idle"),
                "`{id}` can fall all the way through to nothing"
            );
        }
    }
}

/// The robot's canonical repertoire: the actions it has.
///
/// Which of them are unlocked now is runtime progression, answered separately
/// by `ActionSet::gated_by`.
pub fn player_robot_action_set() -> ambition_characters::brain::ActionSet {
    use ambition_characters::brain::{
        ActionSet, MeleeActionSpec, MoveStyleSpec, RangedActionSpec, SpecialActionSpec, SwipeSpec,
    };
    ActionSet {
        melee: Some(MeleeActionSpec::Swipe(SwipeSpec {
            windup_s: 0.0,
            active_s: 0.10,
            recover_s: 0.18,
            damage: 1,
            reach_px: 36.0,
        })),
        // The Hadouken. How it fires (hold to build, release) is
        // `ranged_execution: ChargedProjectile` on the definition, not this slot.
        ranged: Some(RangedActionSpec::bolt(600.0, 1)),
        move_style: MoveStyleSpec::Walk,
        special: Some(SpecialActionSpec::Special("bubble_shield".to_string())),
    }
}

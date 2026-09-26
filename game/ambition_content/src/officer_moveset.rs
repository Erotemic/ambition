//! Tests of the moves the game ships for `officer`.
//!
//! The table is content: `assets/data/movesets/officer.ron`. These tests read it
//! through [`crate::authored_movesets::shipped`], the table the pack loads.

use ambition_entity_catalog::MoveEventKind;

#[cfg(test)]
mod tests {
    use super::*;

    /// The verb is `special_forward`, not `special_side`. `SmashRepertoire`
    /// binds `special`, `special_forward`, `special_up` and `special_down`
    /// (+ `special_air_down`). A rebind in the file aimed at an unused name would
    /// add an unreachable move and leave the archetype's side special bound.
    #[test]
    fn the_draw_answers_the_side_special_and_the_shoulder_rush_is_gone() {
        let set = crate::authored_movesets::shipped("officer");
        assert_eq!(
            set.verbs.get("special_forward").map(String::as_str),
            Some("officer_the_draw"),
            "his side special is the draw"
        );
        assert!(
            !set.moves.iter().any(|m| m.id.contains("shoulderrush")),
            "the archetype's shoulder rush left the table, it was not shadowed"
        );
    }

    /// The shot fires where the art flashes: `shoot.spec.json` marks frame 6
    /// active, and frame 6 at 58ms is 0.348s. This fails if either side moves
    /// alone.
    #[test]
    fn the_round_leaves_on_the_frame_the_muzzle_flares() {
        let set = crate::authored_movesets::shipped("officer");
        let draw = set
            .moves
            .iter()
            .find(|m| m.id == "officer_the_draw")
            .expect("the draw");
        let fired: Vec<f32> = draw
            .events
            .iter()
            .filter(|ev| matches!(ev.kind, MoveEventKind::Ranged))
            .map(|ev| ev.at_s)
            .collect();
        assert_eq!(fired.len(), 1, "one draw, one round");
        assert!(
            (fired[0] - 0.348).abs() < 1e-4,
            "the round leaves at {}s and the muzzle flares at 0.348s",
            fired[0]
        );
        assert!(
            fired[0] < draw.duration_s,
            "a shot scheduled past the move's end never fires"
        );
    }

    /// It carries no strike: the round is the damage.
    #[test]
    fn the_draw_hits_nobody_with_his_body() {
        let set = crate::authored_movesets::shipped("officer");
        let draw = set
            .moves
            .iter()
            .find(|m| m.id == "officer_the_draw")
            .expect("the draw");
        assert!(
            draw.windows.iter().all(|w| w.volumes.is_empty()),
            "the draw is hitless — the projectile is the damage"
        );
    }

    /// All three windbox invariants, because each is silent when broken:
    /// damage above zero makes the catalog reject the move; growth above zero
    /// shoves a damaged fighter further (a hit's rule, not wind's); and without
    /// `repeating` the wall becomes a single shove.
    #[test]
    fn his_neutral_is_a_wall_of_air_that_hurts_nobody() {
        let set = crate::authored_movesets::shipped("officer");
        assert_eq!(
            set.verbs.get("special").map(String::as_str),
            Some("officer_disperse"),
            "his neutral must be the shove"
        );
        let gust = set
            .moves
            .iter()
            .find(|m| m.id == "officer_disperse")
            .expect("…and the move it names must be in the table");

        let volumes: Vec<&ambition_entity_catalog::HitVolume> = gust
            .windows
            .iter()
            .flat_map(|window| window.volumes.iter())
            .collect();
        assert!(!volumes.is_empty(), "the gust spawns nothing at all");
        for volume in &volumes {
            let wind = volume
                .windbox()
                .expect("every volume of a gust is a windbox, not a hit");
            assert!(
                wind.repeating,
                "the gust does not sustain, so it is a one-shot shove wearing a \
                 wall's timing"
            );
            assert_eq!(volume.damage, 0, "a windbox that damages will not load");
            assert_eq!(
                volume.knockback_growth,
                Some(0.0),
                "the shove grows with the victim's damage, which is a hit's rule"
            );
            assert!(
                volume.launch_dir.is_some(),
                "the push direction is derived from geometry, so the gust blows \
                 whichever way you walked into it"
            );
        }

        // The archetype's haymaker is removed, not left unreachable, where every
        // census over `moves` would count it.
        assert!(
            !set.moves.iter().any(|m| m.id == "officer_haymaker"),
            "the displaced haymaker is still in the table"
        );
    }
}

/// Does the CPU actually press his gust?
///
/// The option-list test in `options/tests.rs` uses a synthetic two-move kit.
/// This uses the real candidates `attack_kit_of` builds from this table,
/// where `officer_tilt_forward` (7 damage, faster, 48px reach) is the rival
/// at the blast line.
#[cfg(test)]
mod he_uses_the_gust {

    use ambition_characters::actor::attack_gesture::AttackDir;
    use ambition_characters::actor::ActorFaction;
    use ambition_characters::brain::attack_kit::{
        ActionLegality, AttackBinding, AttackCandidate, AttackVerb,
    };
    use ambition_characters::brain::fighter::options::{generate_options, UtilityWeights};
    use ambition_characters::brain::fighter::Situation;
    use ambition_characters::perception::{
        PerceivedActor, Perceived, SelfView, StageView, WorldView,
    };
    use ambition_platformer2d_core as ae;

    /// The launch conditions a CONTENT claim is about: a fresh reference body
    /// under the undeclared ruleset.
    ///
    /// Named, not implied: `LaunchEnvelope::at` takes explicit conditions. An
    /// authoring claim is about this reference world; a scorer must state its
    /// own.
    fn fresh(victim_damage: i32) -> ambition_entity_catalog::launch::LaunchConditions {
        ambition_entity_catalog::launch::LaunchConditions::AGAINST_A_FRESH_REFERENCE_BODY
            .at_damage(victim_damage)
    }

    /// The stage the readings below are in. `Aabb2d::new` takes a CENTRE and a
    /// HALF-SIZE, so this is `x ∈ 0..800`, `y ∈ 0..600`.
    ///
    /// `StageView::distance_to_edge` takes the nearest of all four sides, so at
    /// `y = 300` a body is never more than 300 from an edge, and
    /// `foe_edge_proximity` never falls below `1 - 300/400 = 0.25` on this floor.
    /// The centre-stage readings are that floor, not zero.
    const HALF_STAGE: f32 = 400.0;

    /// The press road, not a hand-picked list. Mirrors `attack_kit_of` in the
    /// actor tick (the same `move_for_attack` over the same three verbs and five
    /// directions), so every scored move is reachable by a button.
    fn kit() -> Vec<AttackCandidate> {
        let set = crate::authored_movesets::shipped("officer");
        let mut kit: Vec<AttackCandidate> = Vec::new();
        for (verb, verb_name) in [
            (AttackVerb::Basic, ambition_entity_catalog::ATTACK_VERB),
            (AttackVerb::Smash, ambition_entity_catalog::SMASH_VERB),
            (AttackVerb::Special, ambition_entity_catalog::SPECIAL_VERB),
        ] {
            for direction in [
                AttackDir::Neutral,
                AttackDir::Forward,
                AttackDir::Back,
                AttackDir::Up,
                AttackDir::Down,
            ] {
                let Some(spec) = set.move_for_attack(verb_name, direction, true, false) else {
                    continue;
                };
                if kit.iter().any(|c| c.move_id == spec.id) {
                    continue;
                }
                kit.push(AttackCandidate {
                    move_id: spec.id.clone(),
                    frames: spec.frame_data(),
                    binding: AttackBinding { verb, direction },
                    legality: ActionLegality::Now,
                    // A fixture kit: nothing in it has landed anything.
                    wear: ambition_characters::brain::attack_kit::MoveWear::FRESH,
                });
            }
        }
        kit
    }

    fn view(me_x: f32, foe_x: f32) -> WorldView {
        WorldView {
            self_view: SelfView {
                pos: ae::Vec2::new(me_x, 300.0),
                // He faces the foe. Body-local `+x` is facing, so a fixed 1.0 would make
                // the left blast line answer differently from the right.
                facing: if foe_x >= me_x { 1.0 } else { -1.0 },
                gravity_down: ae::Vec2::new(0.0, 1.0),
                alive: true,
                on_ground: true,
                health_max: 100,
                ..Default::default()
            },
            stage: StageView {
                bounds: ae::Aabb::new(
                    ae::Vec2::new(HALF_STAGE, 300.0),
                    ae::Vec2::new(HALF_STAGE, 300.0),
                ),
            },
            actors: vec![PerceivedActor {
                id: "foe".to_string(),
                pos: ae::Vec2::new(foe_x, 300.0),
                faction: ActorFaction::Enemy,
                hostile_to_self: true,
                alive: true,
                on_ground: true,
                health_max: 100,
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    fn best_at(kit: &[AttackCandidate], me: f32, foe: f32) -> String {
        let v = view(me, foe);
        let opts = generate_options(
            Perceived::cheating(&v),
            Situation::Neutral,
            kit,
            &UtilityWeights::v1(),
        );
        opts.attacks
            .first()
            .map(|a| a.move_id.clone())
            .expect("his kit offers something at every range tested here")
    }

    const GUST: &str = "officer_disperse";

    #[test]
    fn he_shoves_at_the_ledge_and_punches_at_centre() {
        let kit = kit();

        // ── Beside the blast line, both sides ───────────────────────────────
        // Right: the foe is 40px from `x = 800`. Left: the mirror, because the
        // body-local frame could make a spacing move handed.
        assert_eq!(best_at(&kit, 700.0, 760.0), GUST, "right ledge");
        assert_eq!(best_at(&kit, 740.0, 790.0), GUST, "right ledge, closer in");
        assert_eq!(best_at(&kit, 100.0, 40.0), GUST, "left ledge");

        // ── Not at centre stage: a CPU that opened every exchange with a
        // damageless shove would be a worse fighter.
        for (me, foe) in [(345.0, 400.0), (400.0, 440.0), (400.0, 360.0)] {
            assert_ne!(
                best_at(&kit, me, foe),
                GUST,
                "the gust won at centre stage from {me} against {foe}"
            );
        }
    }

    /// Anti-vacuity: `assert_ne!` above also passes if the gust is not an option
    /// at all (the range filter removes a shove thrown from outside its box).
    /// This checks the centre-stage result is a ranking, not an absence.
    #[test]
    fn the_gust_is_on_the_table_at_centre_stage_and_simply_loses() {
        let kit = kit();
        let v = view(345.0, 400.0);
        let opts = generate_options(
            Perceived::cheating(&v),
            Situation::Neutral,
            &kit,
            &UtilityWeights::v1(),
        );
        let winner = opts.attacks.first().expect("his kit offers something");
        let gust = opts
            .attacks
            .iter()
            .find(|a| a.move_id == GUST)
            .expect("the gust is offered at centre stage — it just does not win");
        // A ranking, not a sign test: "the gust scored above zero" depends on the
        // weight under test.
        assert_ne!(winner.move_id, GUST, "the winner is not the gust");
        assert!(
            gust.score < winner.score,
            "the gust scored {} and {} scored {}, so they did not compare",
            gust.score,
            winner.move_id,
            winner.score
        );
    }

    /// The population the weight affects, kept beside the reading that set it.
    /// If a third push move lands, `displacement_value`'s band was measured
    /// without it, and this test says so.
    #[test]
    fn exactly_two_moves_in_the_roster_author_a_push_region() {
        let mut pushers: Vec<String> = Vec::new();
        for (table, set) in crate::authored_movesets::tables() {
            for m in &set.moves {
                if m.frame_data().push_coverage.is_some() {
                    pushers.push(format!("{table}/{}", m.id));
                }
            }
        }
        pushers.sort();
        assert_eq!(
            pushers,
            vec!["goblin/dirt_kick".to_string(), "officer/officer_disperse".to_string()],
            "the roster's push-carrying moves changed; `displacement_value`'s \
             1.25..2.6 band was measured against exactly these two"
        );
    }

    /// At 150% he stops poking and swings for the kill.
    ///
    /// `kill_potential` must depend on the move, not only on the foe's percent
    /// (which is the same for every candidate). It combines the foe's percent
    /// with this move's launch, compared against the kit's best launch at that
    /// percent.
    ///
    /// The finisher is the up smash: under the real launch law his down smash is
    /// `(142, 2.82)` and his up smash `(158, 5.83)`, 565 against 1032 at 150%.
    ///
    /// The 80% row checks that the switch happens at a measurable point; 0% and
    /// 150% alone would pass a switch at 1% or at 149%.
    #[test]
    fn a_fresh_opponent_gets_a_poke_and_a_damaged_one_gets_a_smash() {
        let kit = kit();
        let best = |dmg: i32| {
            let mut v = view(400.0, 440.0);
            v.actors[0].damage_taken = dmg;
            generate_options(
                Perceived::cheating(&v),
                Situation::Neutral,
                &kit,
                &UtilityWeights::v1(),
            )
            .attacks
            .first()
            .map(|a| a.move_id.clone())
            .expect("his kit answers at this gap")
        };
        // Same gap, same stage, same kit — only the percent differs.
        assert_eq!(best(0), "officer_jab", "he should poke a fresh opponent");
        assert_eq!(
            best(40),
            "officer_jab",
            "and still poke a lightly damaged one — above the band's ceiling he \
             starts winding up a smash here, which is the pushy CPU nobody asked for"
        );
        assert_eq!(
            best(80),
            "officer_smash_up",
            "by 80% the answer is the move that launches, not the fast one"
        );
        assert_eq!(
            best(150),
            "officer_smash_up",
            "and it stays the answer, because his up smash is the one whose \
             launch keeps climbing: (158, 5.83) against the down smash's (142, 2.82)"
        );
    }

    /// His biggest knockback number is not a finisher. `officer_disperse` has
    /// `knockback: 96` but `knockback_growth: Some(0.0)`, so it shoves the same at
    /// 200% as at 0%. Using `max_knockback` would make the gust his answer to a
    /// damaged opponent.
    #[test]
    fn the_gust_earns_no_kill_credit_however_hurt_the_opponent_is() {
        let gust = crate::authored_movesets::shipped("officer")
            .moves
            .iter()
            .find(|m| m.id == "officer_disperse")
            .expect("the gust")
            .frame_data();
        assert_eq!(gust.max_knockback, 96.0, "the authored shove");
        assert_eq!(
            gust.launch.at(fresh(0)),
            0.0,
            "a windbox is not a hit, so it carries no launch a kill question may spend"
        );
        assert_eq!(
            gust.launch.at(fresh(150)),
            gust.launch.at(fresh(0)),
            "and no amount of damage on the victim changes that"
        );
        assert!(!gust.launch.grows_under(fresh(0)));

        // Control: an ordinary move of his. A derivation that returned zero for
        // everything would pass the arm above.
        let jab = crate::authored_movesets::shipped("officer")
            .moves
            .iter()
            .find(|m| m.id == "officer_jab")
            .expect("the jab")
            .frame_data();
        assert_eq!(jab.launch.at(fresh(0)), jab.max_knockback);
        assert!(jab.max_knockback > 0.0, "the jab launches at all");
    }

    /// The same shove from the other side is a rescue, so it has no ledge value.
    ///
    /// Coverage says the push reaches the foe and proximity says they are near a
    /// blast line; neither says the push sends them that way. The gust's
    /// `push_dir` is fixed, so an Officer on the outboard side of a cornered foe
    /// shoves them back toward centre.
    ///
    /// The ledge arms above test one orientation twice (attacker inboard); a
    /// mirror is not a control for direction of intent.
    #[test]
    fn a_shove_that_pushes_the_cornered_foe_back_inboard_is_not_ledge_control() {
        let kit = kit();
        // The foe is cornered at 760, as in the winning arm above. The Officer is
        // at 790, outboard, so he faces left and his gust blows toward centre.
        let best = best_at(&kit, 790.0, 760.0);
        assert_ne!(
            best, GUST,
            "the gust won from the outboard side, where its authored push \
             sends a cornered opponent back to safety"
        );

        // Control: the same foe at the same edge, attacked from inboard. Without
        // it, a gust that never wins would pass.
        assert_eq!(
            best_at(&kit, 700.0, 760.0),
            GUST,
            "the inboard control stopped winning, so the arm above is about \
             the weight and not about the direction"
        );
    }
}

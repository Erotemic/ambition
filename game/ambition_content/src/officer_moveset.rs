//! The Officer: the brawler archetype's table under his own name, plus his
//! own specials.
//!
//! Unarmed for every normal, on the Pugnacious Polygon's skeleton and clip
//! vocabulary. He uses the archetype's punches and timings; the difference is
//! in the sprite sheet, not this table.
//!
//! See [`crate::archetype_moveset`] for why the ids are renamed rather than
//! shared or copied.

use ambition_entity_catalog::{MoveEvent, MoveEventKind, MoveSpec, MovesetContract};

/// These four numbers come from the art. `shoot.clip.json` runs 12 frames at
/// 58ms: it raises `sidearm_vis` on frame 2, marks `hitbox.active` and draws
/// the muzzle flare on frame 6, and returns to `hand_vis` on frame 11. Firing
/// on any other frame would put the round away from the flash.
const DRAW_AT_S: f32 = 0.116;
const FIRE_AT_S: f32 = 0.348;
const HOLSTER_AT_S: f32 = 0.638;
const DRAW_ENDS_S: f32 = 0.696;

/// Complete brawler-fundamentals repertoire, attributed to the Officer, with his
/// own side special in place of the archetype's shoulder rush.
pub fn officer_moveset() -> MovesetContract {
    let mut set = crate::archetype_moveset::under_own_name(
        crate::pugnacious_polygon_moveset::pugnacious_polygon_moveset(),
        &["polygon_brawler", "pugnacious_polygon"],
        "officer",
    );
    crate::special_slots::replace_special(&mut set, "special", the_order_to_disperse());
    crate::special_slots::replace_special(&mut set, "special_forward", the_draw());
    crate::special_slots::replace_special(&mut set, "special_down", the_riot_shield());
    set
}

/// Neutral special: he shoves the room back, and it does not hurt anybody.
///
/// A windbox (`VolumeReaction::Windbox`; see
/// `ambition_entity_catalog::authoring::gust`). `hit_reaction` treats it as a
/// push, not a hit (`flinchless: hitbox.windbox().is_some()`).
///
/// It fits his kit: a sidearm, a riot shield, and "get back".
///
/// The cost: it replaces the brawler's `haymaker` (`damage: 13,
/// knockback: 142`) and does no damage. He keeps every smash, tilt and
/// `the_draw`, so he still has finishers, but he trades a kill button for a
/// space-making button.
///
/// Sustained, so it pushes every frame you stand in it: a wall, not a shove.
/// That is why `repeating` is authored; a one-shot version would be a worse
/// haymaker.
fn the_order_to_disperse() -> ambition_entity_catalog::MoveSpec {
    ambition_entity_catalog::authoring::gust(
        ambition_entity_catalog::authoring::Gust {
            id: "officer_disperse",
            clip: "attack_side",
            // Slower to open than the haymaker it replaces: holding ground is
            // not a read, and he should not win the exchange by pressing first.
            startup_s: 0.20,
            // Long: the haymaker's danger lasted 0.08s; this lasts over four times as
            // long, because a gust is an area you cannot walk through.
            active_s: 0.34,
            recover_s: 0.30,
            // In front and slightly low: he pushes at chest height with a shield.
            offset: (30.0, 2.0),
            half_extents: (30.0, 22.0),
            // Firm enough to break a rush and take someone off a ledge, well short of a
            // launch. It must never be a kill button.
            push: 96.0,
            // Away and slightly up, so a shoved fighter is briefly airborne. Authored,
            // not derived: wind blows one way, whichever side you came from.
            push_dir: (1.0, -0.22),
            sustained: true,
        },
    )
}

/// Down special: he plants a riot shield and EATS what is thrown at him.
///
/// A counter stance whose response is to absorb. A stance already gets a
/// reflector (the projectile road gates on the same `parrying()` window);
/// `absorbs_projectiles` makes the caught shot disappear instead, through the
/// same `intercept_projectile` operation the parry uses.
///
/// It suits him: absorbing rewards standing your ground with a riot shield,
/// and his other authored move is a gun, so he answers ranged pressure at
/// both ends.
///
/// Absorbing a shot leaves him near the thrower, so the standing grab is the
/// follow-up.
fn the_riot_shield() -> ambition_entity_catalog::MoveSpec {
    ambition_entity_catalog::smash_counter::counter_move(
        "officer_riot_shield",
        "special",
        // Slower to plant than a sword counter: a commitment to a position, not a
        // read on one attack.
        0.12,
        0.20,
        0.38,
        ambition_entity_catalog::smash_counter::CounterParams {
            // A heartbeat, not a duration: `parry_window_timer` decays, and the stance
            // re-arms it every live frame.
            window_s: 0.05,
            // Its own answer, as every counter but the clerk's is.
            answers_the_attacker: false,
            response: ambition_entity_catalog::smash_capture::CAPTURE_ATTEMPT.to_string(),
            response_params: ambition_entity_catalog::ParamValue::from_typed(
                &ambition_entity_catalog::smash_capture::CaptureAttemptParams {
                    offset: (26.0, 0.0),
                    half_extents: (22.0, 22.0),
                    hold_offset: (18.0, -2.0),
                },
            )
            .expect("the riot shield's capture params serialize"),
            // The point of this move.
            absorbs_projectiles: true,
        },
    )
}

/// Side special: he draws the sidearm and fires one round.
///
/// Jon: *"give him a side b that pulls out and shoots a gun."*
///
/// No `equips`, unlike the admiral's `run_out_the_guns` (otherwise the
/// template). The admiral's gun-sword is a separate prop sprite. The
/// Officer's sidearm is part of his own sheet (`holster` and `sidearm` parts
/// on an opacity channel, raised by the `shoot` clip); equipping a held item
/// would draw a second gun.
///
/// So the capability is the body's: `MoveEventKind::Ranged` fires the owner's
/// ranged action, which his character states in `crate::authored::officer`.
///
/// No melee volume: the round is the damage.
///
/// No forward impulse: he plants his feet to draw, so the shot does not go
/// further out of a run.
fn the_draw() -> MoveSpec {
    let mut spec = ambition_entity_catalog::authoring::hitless_special(
        "officer_the_draw",
        "shoot",
        FIRE_AT_S,
        DRAW_ENDS_S,
    );
    spec.display_name = Some("The Draw".to_string());
    spec.events.push(MoveEvent {
        at_s: FIRE_AT_S,
        kind: MoveEventKind::Ranged,
    });
    let spec = ambition_entity_catalog::authoring::sfx(spec, DRAW_AT_S, "player.attack.charge");
    // The shot's sound is not authored here. A `Ranged` event takes the report
    // and the projectile from the weapon that fired.
    let spec = ambition_entity_catalog::authoring::vfx(spec, FIRE_AT_S, "muzzle_flash");
    ambition_entity_catalog::authoring::committed_tail(spec, HOLSTER_AT_S, 0.35)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The verb is `special_forward`, not `special_side`. `SmashRepertoire`
    /// binds `special`, `special_forward`, `special_up` and `special_down`
    /// (+ `special_air_down`). A `replace_special` aimed at an unused name would
    /// add an unreachable move and leave the archetype's side special bound.
    #[test]
    fn the_draw_answers_the_side_special_and_the_shoulder_rush_is_gone() {
        let set = officer_moveset();
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
        let set = officer_moveset();
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
        let set = officer_moveset();
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
        let set = officer_moveset();
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
    use super::officer_moveset;

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
        let set = officer_moveset();
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
        let gust = officer_moveset()
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
        let jab = officer_moveset()
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

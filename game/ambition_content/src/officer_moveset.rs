//! The Officer — the brawler archetype's table, under his own name, plus the
//! one move that is his.
//!
//! Unarmed for every normal, on the Pugnacious Polygon's skeleton and its clip
//! vocabulary. He throws the archetype's punches at the archetype's timings
//! because they are literally the archetype's punches; what differs is who is
//! throwing them and what the air does about it, which is the sprite sheet's
//! business and not this table's.
//!
//! See [`crate::archetype_moveset`] for why the ids are renamed rather than
//! shared or copied.

use ambition_entity_catalog::{MoveEvent, MoveEventKind, MoveSpec, MovesetContract};

/// ⭐⭐ THESE FOUR NUMBERS ARE READ OFF THE ART, NOT CHOSEN. `shoot.clip.json`
/// runs 12 frames at 58ms; it raises `sidearm_vis` on frame 2, marks its
/// `hitbox.active` and draws its muzzle flare on frame 6, and drops the gun back
/// to `hand_vis` on frame 11. A table that fired on any other frame would put
/// the round somewhere the flash is not, which is the one disagreement between
/// an animation and a moveset that a player sees immediately and cannot name.
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
/// ⭐⭐ THE FIRST AUTHORED WINDBOX ON THE ROSTER. `VolumeReaction::Windbox` has
/// been shipped for a long time — down to a validation error for a windbox that
/// carries damage, and `hit_reaction`'s `flinchless: hitbox.windbox().is_some()`
/// saying *"this is a push, not a hit"* — and NOTHING used it. Measured
/// 2026-09-05: zero authored windboxes anywhere. ⇒ This move costs authoring
/// only; see `ambition_entity_catalog::authoring::gust`.
///
/// ⭐ IT MAKES HIS KIT ONE IDEA. He already draws a sidearm and plants a riot
/// shield; a crowd-control officer's third tool is *get back*, not a haymaker.
///
/// ⛔⛔ IT DISPLACES A KO MOVE AND THAT IS THE REAL COST, stated rather than
/// buried: the brawler's `haymaker` is `damage: 13, knockback: 142` and this
/// does no damage at all. He keeps every smash, every tilt and `the_draw`, so
/// he is not short of ways to finish — but a player who picks him is trading a
/// button that kills for a button that CREATES SPACE, and that is a real
/// character decision rather than a free addition. ⚠ Unlike the mine and Sing,
/// there was no empty slot to take: the archetype fills all four specials.
///
/// ⚠ SUSTAINED, SO IT PUSHES EVERY FRAME YOU STAND IN IT. That is what makes it
/// a wall rather than a shove, and it is the whole reason `repeating` is an
/// authored bool: a one-shot version of this move would be a worse haymaker.
fn the_order_to_disperse() -> ambition_entity_catalog::MoveSpec {
    ambition_entity_catalog::authoring::gust(
        ambition_entity_catalog::authoring::Gust {
            id: "officer_disperse",
            clip: "attack_side",
            // Slower to open than the haymaker it replaces: holding ground is
            // not a read, and he should not win the exchange by pressing first.
            startup_s: 0.20,
            // ⭐ LONG. The haymaker's danger lasted 0.08s; this lasts more than
            // four times that, because a gust is an area you cannot walk through
            // rather than a moment you can be caught by.
            active_s: 0.34,
            recover_s: 0.30,
            // Out in front and slightly low — he is pushing at chest height with
            // a shield, not swinging at a head.
            offset: (30.0, 2.0),
            half_extents: (30.0, 22.0),
            // Firm enough to break a rush and take somebody off a ledge, well
            // short of a launch: this must never be a kill button, or the trade
            // above stops being a trade.
            push: 96.0,
            // Away and a little up, so a shoved fighter is briefly airborne and
            // has to land before they can re-approach. ⛔ Authored rather than
            // derived: wind blows ONE WAY, whichever side you walked in from.
            push_dir: (1.0, -0.22),
            sustained: true,
        },
    )
}

/// Down special: he plants a riot shield and EATS what is thrown at him.
///
/// ⭐⭐ A COUNTER STANCE WHOSE ANSWER IS TO SWALLOW. The engine already gives a
/// counter a reflector for free — the projectile road gates on the same
/// `parrying()` window a stance opens — so this move exists to prove the other
/// response: `absorbs_projectiles` makes the caught shot go AWAY instead of
/// back, through the one `intercept_projectile` operation the parry already
/// uses.
///
/// ⛔ AND IT IS THE RIGHT FIGHTER FOR IT. Reflecting is loud and rewards a read;
/// absorbing is quiet and rewards STANDING THERE, which is what a man with a
/// riot shield does. The Officer's other authored move is a gun — a fighter who
/// answers ranged pressure at both ends is a coherent one.
///
/// ⚠ HIS SPECIALS WERE THE BRAWLER ARCHETYPE'S until now, which is the case Jon
/// named: *"we have a lot of characters with boring specials."* This replaces a
/// borrowed generic with something only he does.
///
/// ⚠ The response is the standing grab: absorbing a shot leaves him next to
/// whoever threw it, and a stance that ate the projectile and did nothing else
/// would be a wall rather than a decision.
fn the_riot_shield() -> ambition_entity_catalog::MoveSpec {
    ambition_entity_catalog::smash_counter::counter_move(
        "officer_riot_shield",
        "special",
        // Slower to plant than a sword counter: this is a commitment to a
        // POSITION, not a read on one attack.
        0.12,
        0.20,
        0.38,
        ambition_entity_catalog::smash_counter::CounterParams {
            // A heartbeat, not a duration — `parry_window_timer` decays, and the
            // stance re-arms it every frame it is live.
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
            // ⭐ THE WHOLE POINT OF THIS MOVE.
            absorbs_projectiles: true,
        },
    )
}

/// Side special: he draws the sidearm and fires one round.
///
/// ⭐⭐ JON'S DESIGN, 2026-08-26: *"we should also polish the officer and give
/// him a side b that pulls out and shoots a gun."*
///
/// ⛔⛔ NO `equips`, AND THAT IS THE ONE PLACE THIS DIVERGES FROM THE ADMIRAL'S
/// `run_out_the_guns`, WHICH IS OTHERWISE THE TEMPLATE. His gun-sword is a
/// PROP — a separate sprite a move puts in an empty hand — so his move draws it
/// with `MoveSpec::equips`. The Officer's sidearm is part of HIS OWN SHEET: the
/// rig carries `holster` and `sidearm` parts on an opacity channel, and the
/// `shoot` clip raises them. Equipping a held item on top would register a
/// second gun and draw it beside the one his hand is already holding.
///
/// ⭐ SO THE CAPABILITY IS THE BODY'S. `MoveEventKind::Ranged` fires the owner's
/// ranged action, and with nothing brandished that is the action set his
/// character states — see `crate::authored::officer`, which is where a character
/// says what it DOES.
///
/// ⛔ NO MELEE VOLUME. This replaces `officer_shoulderrush`, a body-to-body
/// charge whose damage was a hitbox, and a move that both fired a round and
/// carried a strike would be two moves wearing one button. The round IS the
/// damage, as it is for every ranged move in the tree.
///
/// ⛔ AND NO FORWARD IMPULSE, WHICH THE SHOULDER RUSH HAD. A draw is a move you
/// plant your feet for; carrying the rush's momentum into it would make the
/// shot longest out of a run, and this move's whole read is that he stopped.
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
    // ⭐ THE SHOT'S OWN SOUND IS NOT AUTHORED HERE. A `Ranged` event routes both
    // the report and the projectile off the weapon that fired it, so a pistol
    // sounds like a pistol without this table naming a cue it would then have to
    // keep in step with the weapon.
    let spec = ambition_entity_catalog::authoring::vfx(spec, FIRE_AT_S, "muzzle_flash");
    ambition_entity_catalog::authoring::committed_tail(spec, HOLSTER_AT_S, 0.35)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ⛔⛔ THE VERB IS `special_forward`, NOT `special_side`. `SmashRepertoire`
    /// binds the four specials as `special`, `special_forward`, `special_up` and
    /// `special_down` (+ `special_air_down`), and a `replace_special` aimed at a
    /// name nothing answers to inserts a move, binds a verb no press produces,
    /// and leaves the archetype's own side special still bound — a fighter with
    /// a new move nobody can reach and an old one that still comes out.
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

    /// ⛔⛔ THE SHOT FIRES WHERE THE ART FLASHES. `shoot.spec.json` marks frame 6
    /// active and draws the muzzle there; frame 6 at 58ms is 0.348s. This is the
    /// arm that goes red if either side moves without the other.
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

    /// ⛔ AND IT CARRIES NO STRIKE. The round is the damage; a hitbox as well
    /// would be two moves on one button, and it is the shape the shoulder rush
    /// this replaced actually had.
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

    /// ⛔⛔ ALL THREE WINDBOX INVARIANTS, because each one is SILENT when broken.
    /// Damage above zero makes the catalog reject the move; growth above zero
    /// shoves a damaged fighter further than a fresh one, which is a hit's rule
    /// and not wind's; and a missing `repeating` turns a wall you cannot walk
    /// through into a single shove that is strictly worse than the haymaker it
    /// replaced. ⇒ A test that only found a `Windbox` reaction would pass
    /// against all three.
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

        // ⛔ AND THE ARCHETYPE'S HAYMAKER IS GONE rather than left unreachable,
        // where every census that walks `moves` reports it as part of his kit.
        assert!(
            !set.moves.iter().any(|m| m.id == "officer_haymaker"),
            "the displaced haymaker is still in the table"
        );
    }
}

/// Does the CPU actually PRESS his gust, or does it merely appear in a list?
///
/// ⛔⛤ THE OPTION-LIST TEST NEXT DOOR (`options/tests.rs`) SAID YES AND WAS
/// MEASURING A FIXTURE. It scored a synthetic gust against a synthetic 40px
/// jab, so "the shove wins at the ledge" was a fact about a two-move kit.
/// Against the ELEVEN candidates `attack_kit_of` builds from this table, the
/// rival beside the blast line is `officer_tilt_forward` — 7 damage, faster,
/// and reaching 48px — and at the shipped weight the gust placed fourth. ⇒ A
/// feature is not delivered when its unit test passes; it is delivered when
/// the character uses it.
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
    /// ⚠ **NAMED RATHER THAN IMPLIED.** `LaunchEnvelope::at` used to take a
    /// bare damage number and silently assume weight `1.0`, no percent scale,
    /// the identity growth curve and no rage — which is how the fighter brain
    /// came to rank its finishers under a law the hit resolver does not use.
    /// An authoring claim IS about that world; a scorer is not, and now both
    /// have to say which.
    fn fresh(victim_damage: i32) -> ambition_entity_catalog::launch::LaunchConditions {
        ambition_entity_catalog::launch::LaunchConditions::AGAINST_A_FRESH_REFERENCE_BODY
            .at_damage(victim_damage)
    }

    /// The stage the readings below are in. `Aabb2d::new` takes a CENTRE and a
    /// HALF-SIZE, so this is `x ∈ 0..800`, `y ∈ 0..600`.
    ///
    /// ⚠ `StageView::distance_to_edge` takes the nearest of all FOUR sides, so
    /// a body standing at `y = 300` is never more than 300 from an edge and
    /// `foe_edge_proximity` never falls below `1 - 300/400 = 0.25` anywhere on
    /// this floor. The centre-stage readings below are that floor, not zero.
    const HALF_STAGE: f32 = 400.0;

    /// ⚠ THE PRESS ROAD, NOT A HAND-PICKED LIST. Mirrors `attack_kit_of` in the
    /// actor tick — the same `move_for_attack` resolution over the same three
    /// verbs and five directions — so a move this test scores is a move a
    /// button reaches.
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
                });
            }
        }
        kit
    }

    fn view(me_x: f32, foe_x: f32) -> WorldView {
        WorldView {
            self_view: SelfView {
                pos: ae::Vec2::new(me_x, 300.0),
                // He faces the foe. Body-local `+x` is FACING, so a body left
                // at 1.0 would read a foe on its left as behind every forward
                // volume and the left blast line would answer differently from
                // the right one for no reason in the game.
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

        // ── BESIDE THE BLAST LINE, BOTH OF THEM ─────────────────────────────
        // Right: the foe stands 40px from `x = 800`. Left: the mirror, which
        // is here because the body-local frame is the one place a spacing
        // move could come out handed.
        assert_eq!(best_at(&kit, 700.0, 760.0), GUST, "right ledge");
        assert_eq!(best_at(&kit, 740.0, 790.0), GUST, "right ledge, closer in");
        assert_eq!(best_at(&kit, 100.0, 40.0), GUST, "left ledge");

        // ── AND NOT AT CENTRE STAGE, which is the other half of "intentional".
        // A CPU that opened every exchange with a damageless shove would be a
        // worse fighter, not a smarter one.
        for (me, foe) in [(345.0, 400.0), (400.0, 440.0), (400.0, 360.0)] {
            assert_ne!(
                best_at(&kit, me, foe),
                GUST,
                "the gust won at centre stage from {me} against {foe}"
            );
        }
    }

    /// ⛔ THE ANTI-VACUITY HALF. `assert_ne!` above passes just as well if the
    /// gust is not an option at all — which is what the range filter does to a
    /// pure shove thrown from outside its own push box. This pins that the
    /// centre-stage readings are a RANKING and not an absence.
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
        // ⚠ A RANKING, NOT A SIGN TEST. An earlier draft asserted the gust
        // scored above zero, which is a claim about `displacement_value`'s
        // magnitude and therefore moves with the very weight under test — it
        // reddened at 1.0 for a reason that had nothing to do with vacuity.
        // What this arm owes is that the centre-stage answer is a COMPARISON.
        assert_ne!(winner.move_id, GUST, "the winner is not the gust");
        assert!(
            gust.score < winner.score,
            "the gust scored {} and {} scored {}, so they did not compare",
            gust.score,
            winner.move_id,
            winner.score
        );
    }

    /// ⚠ THE POPULATION THE WEIGHT MOVES, kept beside the reading that set it.
    /// If a third push move lands, `displacement_value`'s band was measured
    /// without it and this test says so before a balance pass wonders why.
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

    /// **AT 150% HE STOPS POKING AND SWINGS FOR THE KILL.**
    ///
    /// ⛔⛤ HE DID NOT, AND THE WEIGHT THAT SHOULD HAVE MADE HIM WAS DEAD.
    /// `kill_potential` was `foe.damage_frac()` — a fact about the opponent,
    /// identical for every candidate — and an attack's score is only ever
    /// compared with another attack's. Every rung of the authored ladder tunes
    /// that weight from 0.0 to 0.4 and none of it could move a decision:
    /// measured at 150%, he opened with `officer_jab` at the same gap where he
    /// opens with it at 0%.
    ///
    /// ⇒ A kill is the foe's percent AND the launch this move carries, so the
    /// feature is now shared against the kit's best launch AT THAT PERCENT.
    ///
    /// ⛔⛤ **AND THE MOVE HE SWINGS IS THE UP SMASH, NOT THE DOWN SMASH — the
    /// arm said `officer_smash_down` while the launch summary was
    /// `max(base knockback)`.** Under the real law his down smash is `(142,
    /// 2.82)` and his up smash is `(158, 5.83)`: 565 against 1032 at 150%. The
    /// biggest base belongs to the FORWARD smash and the biggest launch, at any
    /// percent worth finishing at, to the up smash. The old expectation was a
    /// reading of the defect.
    ///
    /// ⚠ AND THE 80% ROW IS HERE BECAUSE THE SWITCH HAS TO HAPPEN SOMEWHERE
    /// MEASURABLE — a test that only reads 0% and 150% passes for a weight that
    /// switches at 1% as happily as for one that switches at 149%.
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

    /// ⛔⛤ **AND HIS BIGGEST KNOCKBACK NUMBER IS NOT A FINISHER**, which is
    /// the trap this feature had to avoid. `officer_disperse` authors
    /// `knockback: 96` — more than his jab, his tilts and his uppercut — and
    /// `knockback_growth: Some(0.0)`, so it shoves exactly as far at 200% as at
    /// 0%. A kill question fed `max_knockback` would have made the gust his
    /// best answer to a damaged opponent, which is the opposite of true.
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
        assert!(!gust.launch.grows());

        // ⭐ THE CONTROL IS AN ORDINARY MOVE OF HIS, not another set one: the
        // two fields agree everywhere the launch grows, and a derivation that
        // returned zero for everything would satisfy the arm above forever.
        let jab = officer_moveset()
            .moves
            .iter()
            .find(|m| m.id == "officer_jab")
            .expect("the jab")
            .frame_data();
        assert_eq!(jab.launch.at(fresh(0)), jab.max_knockback);
        assert!(jab.max_knockback > 0.0, "the jab launches at all");
    }

    /// **AND THE SAME SHOVE FROM THE OTHER SIDE IS A RESCUE, SO IT IS NOT
    /// WORTH LEDGE VALUE.**
    ///
    /// ⛔⛤ THE FIRST VERSION OF `displacement_value` PAID FOR BOTH. It was
    /// `coverage_fit(push) × how near the foe is to a blast line` — coverage
    /// says the push REACHES them and proximity says they are near going off,
    /// and neither says the push sends them THAT WAY. Wind blows one way: the
    /// gust's `push_dir` is authored precisely so it does not flip to suit the
    /// geometry. So an Officer who has crossed to the OUTBOARD side of a
    /// cornered opponent shoves them back toward centre stage — same coverage,
    /// same proximity, opposite worth — and scored it as ledge control.
    ///
    /// ⚠ THE ARMS ABOVE COULD NOT SEE THIS. They test the right ledge and its
    /// mirror, which are the same orientation twice: attacker inboard, foe
    /// cornered. A mirror is not a control for handedness of INTENT.
    #[test]
    fn a_shove_that_pushes_the_cornered_foe_back_inboard_is_not_ledge_control() {
        let kit = kit();
        // The foe is cornered at 760, 40px from the right blast line — the
        // SAME foe position as the winning arm above. The Officer has crossed
        // to 790, outboard of them, so he faces left and his forward gust
        // blows toward centre.
        let best = best_at(&kit, 790.0, 760.0);
        assert_ne!(
            best, GUST,
            "the gust won from the outboard side, where its authored push \
             sends a cornered opponent back to safety"
        );

        // ⭐ THE CONTROL IS THE SAME FOE AT THE SAME EDGE PROXIMITY, attacked
        // from inboard. Without it this arm is satisfied by a gust that never
        // wins anywhere, which is what the weight was doing before 1.8.
        assert_eq!(
            best_at(&kit, 700.0, 760.0),
            GUST,
            "the inboard control stopped winning, so the arm above is about \
             the weight and not about the direction"
        );
    }
}

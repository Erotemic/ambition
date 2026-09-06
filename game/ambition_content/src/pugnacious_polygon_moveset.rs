//! Pugnacious Polygon — brawler archetype repertoire.
//!
//! A complete fundamentals brawler table. It mirrors the sword reference's typed
//! vocabulary but expresses every slot with close-range body mechanics: punches,
//! kicks, uppercuts, shoulder rushes, grabs, pummels, and all four throws.

use ambition_characters::moveset_authoring::Strike;
use ambition_characters::moveset_authoring::{committed_tail, gravity_modifier, impulse, strike};
use ambition_characters::smash_capture::{
    author_pummel, author_standing_grab, author_throw, capture_beat, grab_shell,
    CaptureAttemptParams, CaptureCues, CapturePummelParams, CaptureThrowParams,
    SmashCaptureRepertoire,
};
use ambition_characters::smash_repertoire::{
    DownSpecial, NeutralSpecial, SmashRepertoire, UpSpecial,
};
use ambition_platformer2d::entity_catalog::{ImpulseMode, MovesetContract};

pub fn pugnacious_polygon_moveset() -> MovesetContract {
    let jab = strike(Strike {
        id: "polygon_brawler_jab",
        clip: "jab",
        startup_s: 0.04,
        active_s: 0.05,
        recover_s: 0.10,
        offset: (22.0, -2.0),
        half_extents: (18.0, 15.0),
        damage: 4,
        knockback: 52.0,
        knockback_growth: 1.10,
        launch_dir: Some((1.0, -0.18)),
        on_hit: None,
    });
    // ⭐⭐ THE SECOND PUNCH OF THE STRING, AND THE ROSTER'S FIRST HIT-CONFIRM.
    // Bound to no verb: it is reached only by re-pressing attack inside the
    // first jab's cancel window, and only when that jab CONNECTED.
    //
    // ⛔⛔ `CancelCondition::OnHit` HAD ZERO CUSTOMERS UNTIL THIS — measured
    // 2026-09-05, along with `OnWhiff` and `OnBlock`. The condition's own doc
    // describes exactly this move ("combo confirm — jab chains into jab2 on
    // hit") and nothing in the roster had ever asked for it, so the genre's
    // most-pressed sequence was shipped as a capability nobody used.
    //
    // ⭐ AND THE BLOCKED CASE IS THE POINT, not a side effect. A string that
    // continued on `Always` would swing the second punch into a raised shield,
    // which hands the defender a free punish and takes the read out of the
    // exchange. Confirmed, the string continues; blocked, she owes the jab's
    // recovery. That is shield pressure, and it is the whole reason the
    // condition exists.
    //
    // ⚠ TWO PUNCHES, NOT A LADDER. The bound is visible in the data — jab2
    // authors no cancel window of its own — so nobody has to read the runtime to
    // learn where the string ends.
    let jab2 = strike(Strike {
        id: "polygon_brawler_jab2",
        clip: "attack_side",
        startup_s: 0.05,
        active_s: 0.05,
        recover_s: 0.14,
        offset: (24.0, -2.0),
        half_extents: (19.0, 15.0),
        // Bigger than the opener and smaller than her forward tilt: a confirm
        // should beat repeating the jab and should not beat committing to a
        // real button.
        damage: 5,
        knockback: 78.0,
        knockback_growth: 1.30,
        launch_dir: Some((1.0, -0.30)),
        on_hit: None,
    });
    // ⛔ THE WINDOW OPENS WHERE THE ACTIVE FRAMES CLOSE (0.04 + 0.05) and runs to
    // the end of the move (0.19). Opened earlier it would let her cancel the jab
    // before it could connect, which is a cancel out of STARTUP — the thing that
    // makes a move safe on whiff and is not what a confirm is for.
    let jab = ambition_characters::moveset_authoring::cancelable(
        jab,
        0.09,
        0.19,
        &["polygon_brawler_jab2"],
        ambition_platformer2d::entity_catalog::CancelCondition::OnHit,
    );
    let forward_tilt = strike(Strike {
        id: "polygon_brawler_tilt_forward",
        clip: "attack_side",
        startup_s: 0.07,
        active_s: 0.07,
        recover_s: 0.15,
        offset: (27.0, -1.0),
        half_extents: (21.0, 16.0),
        damage: 7,
        knockback: 82.0,
        knockback_growth: 1.52,
        launch_dir: Some((1.0, -0.27)),
        on_hit: None,
    });
    let up_tilt = strike(Strike {
        id: "polygon_brawler_tilt_up",
        clip: "attack_up",
        startup_s: 0.07,
        active_s: 0.07,
        recover_s: 0.15,
        offset: (8.0, -25.0),
        half_extents: (20.0, 24.0),
        damage: 6,
        knockback: 86.0,
        knockback_growth: 1.58,
        launch_dir: Some((0.10, -1.0)),
        on_hit: None,
    });
    let down_tilt = strike(Strike {
        id: "polygon_brawler_tilt_down",
        clip: "attack_down",
        startup_s: 0.06,
        active_s: 0.06,
        recover_s: 0.14,
        offset: (24.0, 12.0),
        half_extents: (22.0, 11.0),
        damage: 5,
        knockback: 68.0,
        knockback_growth: 1.31,
        launch_dir: Some((1.0, -0.16)),
        on_hit: None,
    });

    let mut forward_smash = strike(Strike {
        id: "polygon_brawler_smash_forward",
        clip: "smash_forward",
        startup_s: 0.22,
        active_s: 0.08,
        recover_s: 0.29,
        offset: (30.0, -3.0),
        half_extents: (24.0, 20.0),
        damage: 16,
        knockback: 162.0,
        knockback_growth: 3.25,
        launch_dir: Some((1.0, -0.31)),
        on_hit: None,
    });
    forward_smash.smash_charge_mult = 1.75;
    let mut up_smash = strike(Strike {
        id: "polygon_brawler_smash_up",
        clip: "smash_up",
        startup_s: 0.20,
        active_s: 0.08,
        recover_s: 0.28,
        offset: (5.0, -29.0),
        half_extents: (23.0, 29.0),
        damage: 15,
        knockback: 158.0,
        knockback_growth: 3.15,
        launch_dir: Some((0.0, -1.0)),
        on_hit: None,
    });
    up_smash.smash_charge_mult = 1.75;
    let mut down_smash = strike(Strike {
        id: "polygon_brawler_smash_down",
        clip: "smash_down",
        startup_s: 0.19,
        active_s: 0.09,
        recover_s: 0.29,
        offset: (0.0, 13.0),
        half_extents: (34.0, 13.0),
        damage: 13,
        knockback: 142.0,
        knockback_growth: 2.82,
        launch_dir: Some((0.80, -0.60)),
        on_hit: None,
    });
    down_smash.smash_charge_mult = 1.75;

    let neutral_air = strike(Strike {
        id: "polygon_brawler_air_neutral",
        clip: "air_neutral",
        startup_s: 0.05,
        active_s: 0.10,
        recover_s: 0.14,
        offset: (7.0, 0.0),
        half_extents: (24.0, 23.0),
        damage: 7,
        knockback: 79.0,
        knockback_growth: 1.50,
        launch_dir: None,
        on_hit: None,
    });
    let forward_air = strike(Strike {
        id: "polygon_brawler_air_forward",
        clip: "air_forward",
        startup_s: 0.08,
        active_s: 0.08,
        recover_s: 0.17,
        offset: (28.0, -3.0),
        half_extents: (22.0, 18.0),
        damage: 9,
        knockback: 108.0,
        knockback_growth: 2.05,
        launch_dir: Some((1.0, -0.32)),
        on_hit: None,
    });
    let back_air = strike(Strike {
        id: "polygon_brawler_air_back",
        clip: "air_back",
        startup_s: 0.09,
        active_s: 0.07,
        recover_s: 0.18,
        offset: (-26.0, -1.0),
        half_extents: (22.0, 17.0),
        damage: 10,
        knockback: 124.0,
        knockback_growth: 2.30,
        launch_dir: Some((-1.0, -0.30)),
        on_hit: None,
    });
    let up_air = strike(Strike {
        id: "polygon_brawler_air_up",
        clip: "air_up",
        startup_s: 0.06,
        active_s: 0.08,
        recover_s: 0.14,
        offset: (2.0, -27.0),
        half_extents: (21.0, 23.0),
        damage: 8,
        knockback: 99.0,
        knockback_growth: 1.88,
        launch_dir: Some((0.0, -1.0)),
        on_hit: None,
    });
    let mut down_air = strike(Strike {
        id: "polygon_brawler_air_down",
        clip: "air_down",
        startup_s: 0.11,
        active_s: 0.08,
        recover_s: 0.21,
        offset: (2.0, 25.0),
        half_extents: (21.0, 21.0),
        damage: 10,
        knockback: 126.0,
        knockback_growth: 2.35,
        launch_dir: Some((0.0, 1.0)),
        on_hit: None,
    });
    down_air.landing_lag_s = Some(0.25);

    // NEUTRAL — `polygon_brawler_haymaker`. A CHARGED PUNCH.
    //
    // ⭐⭐ IT WAS ONE OF FIVE SPECIALS IN THE WHOLE ROSTER WITH NOTHING ON IT.
    // `the_census_of_specials_that_carry_no_technique` reports what a special
    // carries BESIDES a technique — extra windows, a charge, an impulse, events
    // — and this move had none of them: a single hitbox on its own button, on
    // the fighter whose entire identity is the size of one punch.
    //
    // ⛔ A CHARGE IS NOT A TECHNIQUE AND DOES NOT WANT TO BE. `smash_charge` is
    // engine-shipped timeline machinery: the move freezes at `hold_at_s` while
    // Attack is held and `smash_charge_mult` interpolates damage AND knockback
    // by how far the clock got. Nothing new is owed for this — which is the
    // point of authoring it here rather than inventing a technique for it.
    //
    // ⚠ IT DOES NOT STORE, and that is the deliberate contrast with the
    // Projectile Polygon's neutral-B, which IS authored to store — the
    // maintainer's request and his words for it live in
    // `projectile_polygon_moveset.rs`, where the fighter he spoke about is.
    // A ranged fighter banks a shot and picks its moment; a brawler commits in
    // front of you and either lands it or wears the recovery. Storing would turn
    // the read into a resource.
    //
    // ⛔ AND HIS NAME STAYS OUT OF THIS FILE ON PURPOSE.
    // `test_the_reviews_page_agrees_with_the_code_about_whose_moves_these_are`
    // reads the maintainer's NAME in a moveset file as "he has spoken about this
    // fighter", and this brawler is on the free-to-change list. Quoting him here
    // about ANOTHER fighter's move would tell a future polish pass that these
    // moves are his. It caught me doing exactly that.
    let haymaker = strike(Strike {
        id: "polygon_brawler_haymaker",
        clip: "attack_side",
        startup_s: 0.16,
        active_s: 0.08,
        recover_s: 0.27,
        offset: (29.0, -4.0),
        half_extents: (24.0, 19.0),
        damage: 13,
        knockback: 142.0,
        knockback_growth: 2.75,
        launch_dir: Some((1.0, -0.28)),
        on_hit: None,
    });
    let haymaker = ambition_characters::moveset_authoring::charge(
        haymaker,
        ambition_characters::moveset_authoring::Charge {
            // Early in the 0.16s startup: the wind-up is visible before the
            // freeze, so the opponent sees it begin rather than a statue appear.
            hold_at_s: 0.06,
            // A LONG hold, because the whole move is the threat of it. 1.2s is
            // far longer than the Polygon's fill per tier and it is meant to be:
            // this is a punch you have to be made to respect, not one you sneak
            // out.
            max_hold_s: 1.2,
            stores: false,
            // ⭐ ROOTED, which is the rule every smash in the game follows and
            // doubly right here: a brawler planting his feet to wind up is the
            // tell the opponent is reading, and a charge you could walk around
            // with would be a threat with no commitment behind it.
            roots: true,
            sustain: ambition_platformer2d::entity_catalog::ChargeSustain::WhileHeld,
            // The button that charges it is the SPECIAL, not a smash gesture.
            gesture: ambition_platformer2d::entity_catalog::ChargeGesture::Special,
            // 1.6x at a full hold — 13 damage becomes 20, and the knockback too.
            multiplier: 1.6,
        },
    );
    let neutral_special = committed_tail(haymaker, 0.58, 0.18);
    // SIDE — `polygon_brawler_collar`. A COMMAND GRAB, replacing a shoulder rush
    // that was a dash with a hitbox on it.
    //
    // ⭐⭐ IT IS THE ARCHETYPE'S MISSING VERB. A brawler's identity in this genre
    // is that blocking is not safe against them: the grab that travels is what
    // makes shielding a decision instead of a default, and this table had five
    // specials that were all hitboxes — every one of them answerable by holding
    // shield. ⇒ The census (`the_census_of_specials_that_carry_no_technique`)
    // named this fighter as five-for-five bare, which is what sent me here.
    //
    // ⛔ IT COSTS NOTHING NEW. `smash.capture_attempt` is shipped, the stand-in's
    // `lunge_grab` already authors a travelling one, and this fighter already has
    // a pummel and all four throws — so the follow-ups exist the moment the grab
    // does. That is the whole ease-of-authoring claim in one move: name the key,
    // fill three fields, keep the hold.
    //
    // ⚠ THE HOLD MATCHES HIS STANDING GRAB, deliberately and for the reason the
    // stand-in's does: the throws are SHARED, so a captive held somewhere else
    // would make them read differently depending on which grab caught them.
    //
    // ⛔ THE PRICE IS THE RECOVERY, at 0.42s against a 0.07s catch. A command
    // grab that is safe on whiff removes the shield mixup it exists to create,
    // because there is then no reason to ever not throw it.
    let mut side_special = author_standing_grab(
        grab_shell("polygon_brawler_collar", "attack_side", 0.16, 0.07, 0.42),
        CaptureAttemptParams {
            // 58px of reach against his standing grab's 34: it has to catch
            // somebody the lunge is arriving at, not somebody already in range.
            offset: (32.0, 0.0),
            half_extents: (26.0, 20.0),
            hold_offset: (14.0, 3.0),
        },
    );
    // ⛔ ADDITIVE, for the reason the stand-in's carries: a grab that deleted
    // your run would make dashing into it worse than walking, and a
    // dash-cancelled command grab covering more ground is correct.
    side_special.start_impulse = Some((360.0, 0.0));
    let mut uppercut = strike(Strike {
        id: "polygon_brawler_uppercut",
        clip: "attack_up",
        startup_s: 0.08,
        active_s: 0.12,
        recover_s: 0.19,
        offset: (5.0, -20.0),
        half_extents: (22.0, 27.0),
        damage: 9,
        knockback: 104.0,
        knockback_growth: 1.88,
        launch_dir: Some((0.08, -1.0)),
        on_hit: None,
    });
    uppercut.landing_lag_s = Some(0.25);
    let up_special = impulse(uppercut, 0.08, (0.0, -745.0), ImpulseMode::Set);
    // ⭐⭐ AND THEN HE HANGS THERE. The rise is unchanged; what is new is the
    // DESCENT, and it is the first authored customer for
    // `MoveEventKind::GravityModifier`.
    //
    // ⛔ THIS FIGHTER WAS PICKED BY MEASUREMENT, NOT BY TASTE. The
    // comment-vs-mechanics sweep scored every moveset in this crate, and his was
    // the only one with ZERO claim markers across ten comment lines — five
    // specials that are a haymaker, a shoulder rush, an uppercut, a ground slam
    // and a body drop. Entirely honest and entirely dull, which is a different
    // defect from the five that lied and the one the goal names in as many
    // words: *"many have boring specials."*
    //
    // ⭐ THE FLOAT STARTS AFTER THE HIT, not at the press: the uppercut is still
    // a committal rising attack that can be beaten, and the reward for landing
    // it — or for surviving it — is the way home. 0.35 gravity for 1.1s is
    // roughly a doubled descent, long enough to change a ledge read and short
    // enough that it cannot stall out a whole stock.
    //
    // ⛔ IT OUTLIVES THE MOVE, WHICH IS THE POINT AND IS WHY IT IS AN EVENT. The
    // uppercut's own timeline ends at 0.39s; the parasol is still running for
    // nearly a second after that. A `WindowTag` could not say this, and the
    // movement domain — not this move — is what ends it.
    let up_special = gravity_modifier(up_special, 0.20, 0.35, 1.1);

    let grounded_down_special = committed_tail(
        strike(Strike {
            id: "polygon_brawler_ground_slam",
            clip: "attack_down",
            startup_s: 0.13,
            active_s: 0.10,
            recover_s: 0.25,
            offset: (0.0, 15.0),
            half_extents: (31.0, 13.0),
            damage: 11,
            knockback: 105.0,
            knockback_growth: 1.95,
            launch_dir: Some((0.70, -0.72)),
            on_hit: None,
        }),
        0.54,
        0.05,
    );
    // ⭐⭐ AND THE SLAM SENDS A SHOCK ALONG THE GROUND. The fists were the whole
    // move: one box under him, on the fighter whose archetype is the size of an
    // impact. `the_census_of_specials_that_carry_no_technique` named this as one
    // of only four specials in the roster carrying no technique AND no other
    // authoring — a hitbox on a different button.
    //
    // ⛔ IT COSTS NO NEW ENGINE. `smash.riposte_strike` spawns an ordinary body
    // strike at an authored reach, and its own module says it is not limited to
    // counters: a counter reaches it as a `response`, a MOVE reaches it from its
    // timeline, and both arrive as the same `ActorActionMessage`. This is that
    // second road's first customer.
    //
    // ⛔ AND `multihit` COULD NOT SAY THIS. Its pulses are a LEAD-IN — it shifts
    // the finisher back by the pulse train's length, because a multi-hit is a
    // wind-up into a finisher. A shock that follows the impact is the other
    // direction.
    //
    // ⚠ THE SHOCK RUNS THE WAY HE FACES, not both ways, because the technique
    // places one box at a facing-relative reach. That is a real limitation and
    // it is also the right move here: a directional slam that covered both sides
    // would beat a shield in front AND punish a wake-up behind, which is two
    // options on one button.
    let grounded_down_special = ambition_characters::smash_riposte::author_cut(
        grounded_down_special,
        // Just after the fists land (startup 0.13 + the tail's shift): the shock
        // is a consequence of the impact and must read as one.
        0.24,
        ambition_characters::smash_riposte::RiposteStrikeParams {
            // Weaker than the 11 the slam itself deals: the shock is the reach,
            // not the payoff.
            damage: 6,
            // ⛔ A FEEL MULTIPLIER, NOT A LAUNCH SPEED. The slam above authors
            // `knockback: 105.0` on a `Strike`, where that field IS a speed;
            // copying it here is the units error three shipped moves made.
            knockback: 1.15,
            // Out in front, spanning x 16..104 body-local: it covers the ground
            // a shielding opponent was standing on, which is what a brawler's
            // slam is for.
            reach: 60.0,
            half_extents: (44.0, 10.0),
            // ⭐ BLUNT, and it is the other half of the same fix: this shock and
            // the swordfighter's riposte are the SAME mechanic — a technique-
            // spawned body strike — so the only thing that makes them different
            // events to a player is this string. The floor answering a slam is
            // not a blade.
            hit_sfx: Some("world.rock.hit".to_string()),
            lifetime_s: 0.10,
        },
    );
    let mut airborne_down_special = strike(Strike {
        id: "polygon_brawler_body_drop",
        clip: "air_down",
        startup_s: 0.10,
        active_s: 0.11,
        recover_s: 0.22,
        offset: (0.0, 24.0),
        half_extents: (24.0, 24.0),
        damage: 12,
        knockback: 132.0,
        knockback_growth: 2.42,
        launch_dir: Some((0.0, 1.0)),
        on_hit: None,
    });
    airborne_down_special.landing_lag_s = Some(0.29);
    let airborne_down_special =
        impulse(airborne_down_special, 0.10, (0.0, 1080.0), ImpulseMode::Set);

    let grab = author_standing_grab(
        grab_shell("polygon_brawler_grab", "grab", 0.06, 0.05, 0.21),
        CaptureAttemptParams {
            offset: (15.0, 1.0),
            half_extents: (19.0, 16.0),
            hold_offset: (14.0, 3.0),
        },
    );
    let pummel = author_pummel(
        capture_beat("polygon_brawler_pummel", "pummel", 0.15),
        0.06,
        CapturePummelParams { damage: 4 },
    );
    let forward_throw = author_throw(
        capture_beat("polygon_brawler_throw_forward", "throw_forward", 0.24),
        0.11,
        CaptureThrowParams {
            damage: 8,
            knockback: 116.0,
            knockback_growth: 2.22,
            launch_dir: (1.0, -0.30),
        },
    );
    let back_throw = author_throw(
        capture_beat("polygon_brawler_throw_back", "throw_back", 0.26),
        0.12,
        CaptureThrowParams {
            damage: 9,
            knockback: 126.0,
            knockback_growth: 2.35,
            launch_dir: (-1.0, -0.27),
        },
    );
    let up_throw = author_throw(
        capture_beat("polygon_brawler_throw_up", "throw_up", 0.25),
        0.11,
        CaptureThrowParams {
            damage: 8,
            knockback: 120.0,
            knockback_growth: 2.28,
            launch_dir: (0.0, -1.0),
        },
    );
    let down_throw = author_throw(
        capture_beat("polygon_brawler_throw_down", "throw_down", 0.27),
        0.12,
        CaptureThrowParams {
            damage: 7,
            knockback: 88.0,
            knockback_growth: 1.82,
            launch_dir: (0.28, -0.96),
        },
    );

    let mut contract = SmashRepertoire {
        // the genre shapes, deliberately. This is the unarmed half of the
        // REFERENCE pair, so its taunt and dash attack are the ones a new
        // humanoid should copy before it has a reason to differ. Five fighters
        // own a `DashAttackShape` because their own laws refused the generic
        // one; a reference rig has no such law to refuse it.
        taunt: ambition_characters::moveset_authoring::taunt("pugnacious_polygon_taunt", 0.9),
        dash_attack: ambition_characters::moveset_authoring::dash_attack(
            "pugnacious_polygon_dash_attack",
            ambition_characters::moveset_authoring::DashAttackShape::GENRE,
            8,
            90.0,
        ),
        jab,
        forward_tilt,
        up_tilt,
        down_tilt,
        forward_smash,
        up_smash,
        down_smash,
        neutral_air,
        forward_air,
        back_air,
        up_air,
        down_air,
        neutral_special: NeutralSpecial::Authored(neutral_special),
        side_special,
        up_special: UpSpecial::Standard(up_special),
        down_special: DownSpecial::ByPosture {
            grounded: grounded_down_special,
            airborne: airborne_down_special,
        },
        capture: SmashCaptureRepertoire {
            cues: CaptureCues::GENERIC,
            grab,
            pummel,
            forward_throw,
            back_throw: Some(back_throw),
            up_throw: Some(up_throw),
            down_throw: Some(down_throw),
        },
    }
    .into_contract();
    // Reachable only through the first jab's confirm window, so it joins the
    // moves without claiming a press of its own.
    contract.moves.push(jab2);
    contract
}

#[cfg(test)]
mod tests {

    /// ⭐⭐ THE SLAM'S SHOCK REACHES GROUND THE SLAM ITSELF CANNOT, which is the
    /// entire reason it exists — and the pair of numbers that says so is split
    /// across two different authoring vocabularies, so nothing but a test holds
    /// them together.
    ///
    /// ⛔ AND IT MUST STAY THE WEAKER HALF. A follow-up that hit harder than the
    /// impact it follows would make the slam a delivery mechanism for its own
    /// tail, and the fists are the move.
    #[test]
    fn his_ground_slam_sends_a_shock_that_outreaches_the_fists_and_hits_softer() {
        use ambition_characters::smash_riposte::{RiposteStrikeParams, RIPOSTE_STRIKE};

        let set = pugnacious_polygon_moveset();
        let slam = set
            .moves
            .iter()
            .find(|m| m.id == "polygon_brawler_ground_slam")
            .expect("his grounded down-B");

        let shock: RiposteStrikeParams = slam
            .events
            .iter()
            .find_map(|event| match &event.kind {
                ambition_platformer2d::entity_catalog::MoveEventKind::Effect(effect)
                    if effect.key == RIPOSTE_STRIKE =>
                {
                    effect.params.hydrate().ok()
                }
                _ => None,
            })
            .expect("his slam sends a shock");

        // The fists: the widest volume the move authors on its own timeline.
        let fists_reach = slam
            .windows
            .iter()
            .flat_map(|window| window.volumes.iter())
            // ⛔ `leading_edge_x`, NOT THE SUM. This spelled
            // `offset.0 + half_extents.0` and `test_the_grab_reach_is_one_formula`
            // caught it: the same arithmetic on a different type is the same
            // duplication, and this file names `CaptureAttemptParams` elsewhere
            // so it is squarely in that guard's population.
            .map(|volume| volume.shape.leading_edge_x())
            .fold(f32::MIN, f32::max);
        assert!(
            fists_reach > f32::MIN,
            "the slam has no hitbox of its own any more, so this test is \
             comparing the shock against nothing",
        );
        assert!(
            shock.reach + shock.half_extents.0 > fists_reach,
            "the shock reaches {}px and the fists {fists_reach}px — a follow-up \
             that covers no new ground is a second hit on the same square",
            shock.reach + shock.half_extents.0,
        );

        let fists_damage = slam
            .windows
            .iter()
            .flat_map(|window| window.volumes.iter())
            .map(|volume| volume.damage)
            .max()
            .expect("the slam deals damage");
        assert!(
            (shock.damage as i32) < fists_damage,
            "the shock deals {} against the fists' {fists_damage}: the tail \
             must not outhit the impact it follows",
            shock.damage,
        );
        assert!(
            shock.problems().is_empty(),
            "the shock is authored unusably: {}",
            shock.problems().join("; "),
        );
    }

    /// ⭐⭐ HIS PUNCH CHARGES, AND IT DOES NOT STORE — the second half is the
    /// design claim, and it is about TWO fighters at once.
    ///
    /// The Projectile Polygon's neutral-B is authored to STORE, at the
    /// maintainer's request (his words are in that fighter's own file). A
    /// ranged fighter banks a shot and picks its moment. A brawler commits in
    /// front of you and either lands it or wears the recovery — storing would
    /// turn the read into a resource. ⇒ Prose in two files cannot hold that
    /// apart; this can, and it fails if either fighter is retuned toward the
    /// other.
    #[test]
    fn his_haymaker_charges_and_deliberately_does_not_store() {
        let set = pugnacious_polygon_moveset();
        let haymaker = set
            .moves
            .iter()
            .find(|m| m.id == "polygon_brawler_haymaker")
            .expect("his neutral-B");
        let charge = haymaker
            .smash_charge
            .as_ref()
            .expect("his neutral-B charges");
        assert!(
            !charge.stores,
            "his haymaker stores its charge, which makes the commitment a \
             resource and takes the read out of the move",
        );
        assert!(
            charge.roots,
            "a charge that does not root him is a threat with no commitment",
        );
        assert!(
            haymaker.smash_charge_mult > 1.0,
            "charging his punch pays {}x, so holding it is strictly worse than \
             throwing it",
            haymaker.smash_charge_mult,
        );

        // The other half of the contrast, asserted rather than described.
        let hers = crate::projectile_polygon_moveset::projectile_polygon_moveset();
        let shot = hers
            .moves
            .iter()
            .find(|m| m.id == "polygon_projectile_charge_shot")
            .expect("her neutral-B");
        assert!(
            shot.smash_charge.as_ref().is_some_and(|c| c.stores),
            "her charge shot stopped storing, so the brawler's not-storing says \
             nothing any more — the storing was asked for on THAT move \
             specifically",
        );
    }

    /// Every capture attempt a move authors, by move id.
    fn capture_of(set: &MovesetContract, id: &str) -> CaptureAttemptParams {
        set.moves
            .iter()
            .find(|m| m.id == id)
            .unwrap_or_else(|| panic!("`{id}` is not in this table"))
            .windows
            .iter()
            .filter_map(|window| window.sustain_effect.as_ref())
            .find(|effect| effect.key == ambition_characters::smash_capture::CAPTURE_ATTEMPT)
            .and_then(|effect| effect.params.hydrate().ok())
            .unwrap_or_else(|| panic!("`{id}` carries no capture attempt"))
    }

    /// ⭐⭐ HIS SIDE-B IS A COMMAND GRAB, WHICH IS THE POINT OF THIS FIGHTER.
    /// Five specials that are all hitboxes are five specials answerable by
    /// holding shield; the grab that travels is what makes shielding a decision.
    ///
    /// ⛔ AND THE HOLD IS THE ASSERTION THAT WAS ONLY PROSE. Both grabs feed the
    /// SAME four throws, so a captive held somewhere else would make the throws
    /// read differently depending on which grab caught them. That sentence is in
    /// the stand-in's comment, in this move's comment, and until now in no test.
    #[test]
    fn his_side_b_is_a_command_grab_that_shares_the_hold_with_his_standing_one() {
        let set = pugnacious_polygon_moveset();
        let standing = capture_of(&set, "polygon_brawler_grab");
        let command = capture_of(&set, "polygon_brawler_collar");

        assert_eq!(
            command.hold_offset, standing.hold_offset,
            "the command grab holds captives at {:?} and the standing grab at \
             {:?} — the throws are shared, so they would read differently \
             depending on which grab caught you",
            command.hold_offset, standing.hold_offset,
        );
        assert!(
            command.reach_x() > standing.reach_x(),
            "the command grab reaches {}px and the standing grab {}px — a \
             command grab that closes no distance is a worse standing grab",
            command.reach_x(),
            standing.reach_x(),
        );
        let travels = set
            .moves
            .iter()
            .find(|m| m.id == "polygon_brawler_collar")
            .and_then(|m| m.start_impulse);
        assert!(
            travels.is_some_and(|(x, _)| x > 0.0),
            "his command grab does not travel ({travels:?}), so it is a standing \
             grab on a different button",
        );
    }
    use super::*;

    /// ⭐⭐ HIS UP-B OPENS A PARASOL THAT OUTLIVES IT, and the DURATION is the
    /// assertion rather than the presence.
    ///
    /// ⛔ A `WindowTag` would satisfy "the move slows his fall" and fail this,
    /// which is exactly the simplification a later reader will reach for. The
    /// regime has to still be running after the move's own timeline ends, or the
    /// descent it exists to give him is over before he starts falling.
    #[test]
    fn the_uppercut_leaves_him_floating_for_longer_than_the_move_lasts() {
        use ambition_platformer2d::entity_catalog::MoveEventKind;
        let moves = pugnacious_polygon_moveset();
        let up = moves
            .moves
            .iter()
            .find(|m| m.id == "polygon_brawler_uppercut")
            .expect("his up-B is in the table");
        let (at_s, scale, seconds) = up
            .events
            .iter()
            .find_map(|e| match e.kind {
                MoveEventKind::GravityModifier { scale, seconds } => Some((e.at_s, scale, seconds)),
                _ => None,
            })
            .expect(
                "his up-B authors no gravity modifier, so the roster's dullest \
                 recovery is a bare uppercut again",
            );
        assert!(
            scale < 1.0 && scale > 0.0,
            "a modifier of {scale} does not SLOW a fall — 1.0 is a no-op and \
             0.0 is a hover, and neither is a parasol"
        );
        let move_ends = up.duration_s;
        assert!(
            at_s + seconds > move_ends,
            "the float ({seconds}s from {at_s}s) is spent by the time the move \
             ends at {move_ends}s, so it can only slow a fall he is not having \
             yet — the whole reason this is an event and not a window is that it \
             has to outlast the move"
        );
    }

    #[test]
    fn the_reference_brawler_answers_the_complete_typed_repertoire() {
        let moves = pugnacious_polygon_moveset();
        for id in [
            "polygon_brawler_jab",
            "polygon_brawler_tilt_forward",
            "polygon_brawler_tilt_up",
            "polygon_brawler_tilt_down",
            "polygon_brawler_smash_forward",
            "polygon_brawler_smash_up",
            "polygon_brawler_smash_down",
            "polygon_brawler_air_neutral",
            "polygon_brawler_air_forward",
            "polygon_brawler_air_back",
            "polygon_brawler_air_up",
            "polygon_brawler_air_down",
            "polygon_brawler_haymaker",
            "polygon_brawler_collar",
            "polygon_brawler_uppercut",
            "polygon_brawler_ground_slam",
            "polygon_brawler_body_drop",
            "polygon_brawler_grab",
            "polygon_brawler_pummel",
            "polygon_brawler_throw_forward",
            "polygon_brawler_throw_back",
            "polygon_brawler_throw_up",
            "polygon_brawler_throw_down",
            "pugnacious_polygon_taunt",
            "pugnacious_polygon_dash_attack",
        ] {
            assert!(moves.moves.iter().any(|m| m.id == id), "missing {id}");
        }
    }
}

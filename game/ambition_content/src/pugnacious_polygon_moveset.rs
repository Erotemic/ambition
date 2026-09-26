//! Pugnacious Polygon: brawler archetype repertoire.
//!
//! A complete fundamentals brawler table. It mirrors the sword reference's typed
//! vocabulary but expresses every slot with close-range body mechanics: punches,
//! kicks, uppercuts, shoulder rushes, grabs, pummels, and all four throws.

use ambition_entity_catalog::authoring::Strike;
use ambition_entity_catalog::authoring::{committed_tail, gravity_modifier, impulse, strike};
use ambition_entity_catalog::smash_capture::{
    author_pummel, author_standing_grab, author_throw, capture_beat, grab_shell,
    CaptureAttemptParams, CaptureCues, CapturePummelParams, CaptureThrowParams,
    SmashCaptureRepertoire,
};
use ambition_entity_catalog::smash_repertoire::{
    DownSpecial, NeutralSpecial, SmashRepertoire, UpSpecial,
};
use ambition_entity_catalog::{ImpulseMode, MovesetContract};

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
    // The second punch of the string, a hit-confirm. It is bound to no verb:
    // it is reached only by pressing attack again inside the first jab's cancel
    // window, and only when that jab connected (`CancelCondition::OnHit`).
    //
    // The blocked case is the point. A string that continued on `Always` would
    // swing the second punch into a raised shield and give the defender a free
    // punish. Confirmed, the string continues; blocked, she pays the jab's
    // recovery. That is shield pressure.
    //
    // Two punches, not a ladder: jab2 authors no cancel window of its own, so
    // the data shows where the string ends.
    let jab2 = strike(Strike {
        id: "polygon_brawler_jab2",
        clip: "attack_side",
        startup_s: 0.05,
        active_s: 0.05,
        recover_s: 0.14,
        offset: (24.0, -2.0),
        half_extents: (19.0, 15.0),
        // Bigger than the opener and smaller than her forward tilt: a confirm
        // should beat repeating the jab, not beat committing to a real button.
        damage: 5,
        knockback: 78.0,
        knockback_growth: 1.30,
        launch_dir: Some((1.0, -0.30)),
        on_hit: None,
    });
    // The window opens where the active frames close (0.04 + 0.05) and runs to
    // the end of the move (0.19). Opened earlier, it would allow a cancel out of
    // startup, which is what makes a move safe on whiff.
    let jab = ambition_entity_catalog::authoring::cancelable(
        jab,
        0.09,
        0.19,
        &["polygon_brawler_jab2"],
        ambition_entity_catalog::CancelCondition::OnHit,
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
        knockback_growth: 5.83,
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

    // Neutral: `polygon_brawler_haymaker`, a charged punch.
    //
    // Before the charge it had nothing but one hitbox
    // (`the_census_of_specials_that_carry_no_technique`).
    //
    // A charge is not a technique. `smash_charge` is engine timeline machinery:
    // the move freezes at `hold_at_s` while Attack is held, and
    // `smash_charge_mult` scales damage and knockback by how far the clock got.
    //
    // It does not store, in contrast with the Projectile Polygon's neutral-B,
    // which does (the maintainer's request is in
    // `projectile_polygon_moveset.rs`). A ranged fighter banks a shot and picks
    // its moment; a brawler commits in front of you and lands it or pays the
    // recovery.
    //
    // The maintainer's name stays out of this file on purpose.
    // `test_the_reviews_page_agrees_with_the_code_about_whose_moves_these_are`
    // reads his name in a moveset file as "he has spoken about this fighter",
    // and this brawler is on the free-to-change list.
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
    let haymaker = ambition_entity_catalog::authoring::charge(
        haymaker,
        ambition_entity_catalog::authoring::Charge {
            // Early in the 0.16s startup, so the opponent sees the wind-up begin
            // before the freeze.
            hold_at_s: 0.06,
            // A long hold: the move is the threat of it. 1.2s is far longer than the
            // Polygon's fill per tier, on purpose.
            max_hold_s: 1.2,
            stores: false,
            // Rooted, like every smash: planting the feet is the tell the opponent
            // reads. A charge you could walk around with has no commitment.
            roots: true,
            sustain: ambition_entity_catalog::ChargeSustain::WhileHeld,
            // The button that charges it is the SPECIAL, not a smash gesture.
            gesture: ambition_entity_catalog::ChargeGesture::Special,
            // 1.6x at a full hold — 13 damage becomes 20, and the knockback too.
            multiplier: 1.6,
        },
    );
    let neutral_special = committed_tail(haymaker, 0.58, 0.18);
    // Side: `polygon_brawler_collar`, a command grab.
    //
    // A brawler must make blocking unsafe. The travelling grab makes shielding
    // a decision; hitbox-only specials are all answered by holding shield.
    //
    // It needs no new engine work: `smash.capture_attempt` exists, the
    // stand-in's `lunge_grab` already travels, and this fighter already has a
    // pummel and four throws.
    //
    // The hold matches his standing grab because the throws are shared: a
    // captive held elsewhere would make them read differently per grab.
    //
    // The price is the recovery: 0.42s against a 0.07s catch. A command grab
    // that is safe on whiff would remove the shield mix-up it exists to make.
    let mut side_special = author_standing_grab(
        grab_shell("polygon_brawler_collar", "attack_side", 0.16, 0.07, 0.42),
        CaptureAttemptParams {
            // 58px of reach against his standing grab's 34: it must catch someone the
            // lunge is arriving at, not someone already in range.
            offset: (32.0, 0.0),
            half_extents: (26.0, 20.0),
            hold_offset: (14.0, 3.0),
        },
    );
    // Additive, like the stand-in's: a grab that deleted your run would make
    // dashing into it worse than walking.
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
    // Then he hangs there. The rise is unchanged; the descent is new, through
    // `MoveEventKind::GravityModifier`.
    //
    // The float starts after the hit, not at the press: the uppercut is still
    // a committal rising attack that can be beaten. 0.35 gravity for 1.1s is
    // about a doubled descent: long enough to change a ledge read, short enough
    // that it cannot stall a whole stock.
    //
    // It outlives the move, so it is an event. The uppercut ends at 0.39s; the
    // parasol runs for nearly a second after that. A `WindowTag` cannot say this,
    // and the movement domain ends it.
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
    // The slam sends a shock along the ground.
    //
    // No new engine: `smash.riposte_strike` spawns an ordinary body strike at
    // an authored reach. A counter reaches it as a `response`, a move reaches it
    // from its timeline, and both arrive as the same `ActorActionMessage`.
    //
    // `multihit` cannot say this: its pulses are a lead-in that shifts the
    // finisher later. A shock that follows the impact goes the other way.
    //
    // The shock runs the way he faces, not both ways, because the technique
    // places one box at a facing-relative reach. A slam covering both sides
    // would beat a shield in front and punish a wake-up behind: two options on
    // one button.
    let grounded_down_special = ambition_entity_catalog::smash_riposte::author_cut(
        grounded_down_special,
        // Just after the fists land (startup 0.13 plus the tail's shift): the
        // shock must read as a result of the impact.
        0.24,
        ambition_entity_catalog::smash_riposte::RiposteStrikeParams {
            // Weaker than the slam's 11: the shock is the reach, not the payoff.
            damage: 6,
            // A feel multiplier, not a launch speed. The slam's `knockback: 105.0` on a
            // `Strike` is a speed; do not copy it here.
            knockback: 1.15,
            // Out in front, spanning x 16..104 body-local: it covers the ground a
            // shielding opponent stood on.
            reach: 60.0,
            half_extents: (44.0, 10.0),
            // Blunt. This shock and the swordfighter's riposte are the same mechanic (a
            // technique-spawned body strike), so this string is the only difference a
            // player hears.
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
        // The genre shapes, on purpose. This is the unarmed half of the reference
        // pair, so a new humanoid should copy its taunt and dash attack until it has
        // a reason to differ.
        taunt: ambition_entity_catalog::authoring::taunt("pugnacious_polygon_taunt", 0.9),
        dash_attack: ambition_entity_catalog::authoring::dash_attack(
            "pugnacious_polygon_dash_attack",
            ambition_entity_catalog::authoring::DashAttackShape::GENRE,
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
    /// The launch conditions a CONTENT claim is about: a fresh reference body
    /// under the undeclared ruleset.
    ///
    /// Named, not implied: an authoring claim is about this world, and a scorer
    /// is not, so each must say which conditions it uses.
    fn fresh(victim_damage: i32) -> ambition_entity_catalog::launch::LaunchConditions {
        ambition_entity_catalog::launch::LaunchConditions::AGAINST_A_FRESH_REFERENCE_BODY
            .at_damage(victim_damage)
    }

    /// His two smashes cross, so a kill question must not read base knockback.
    ///
    /// Forward smash is `(162, 3.25)` and up smash is `(158, 5.83)`. The forward
    /// smash has the bigger base; past a few points of damage the up smash has
    /// the bigger launch. A brain feature folded to `max(base)` ranks his
    /// finisher backwards.
    ///
    /// The test is about the order, not the numbers: it asserts that the lines
    /// cross and which side wins on each side. Retuning either smash keeps it
    /// green while the roster still has a base/growth tradeoff; it fails if the
    /// derivation discards growth.
    #[test]
    fn his_up_smash_out_launches_his_forward_smash_once_the_opponent_is_worn() {
        let set = crate::authored_movesets::shipped("pugnacious_polygon");
        let frames = |id: &str| {
            set.moves
                .iter()
                .find(|m| m.id == id)
                .unwrap_or_else(|| panic!("{id} is on his table"))
                .frame_data()
        };
        let forward = frames("polygon_brawler_smash_forward").launch;
        let up = frames("polygon_brawler_smash_up").launch;

        assert!(
            forward.at(fresh(0)) > up.at(fresh(0)),
            "against a FRESH opponent the forward smash is the harder launch — \
             if this flips, the arm below is no longer measuring a crossing"
        );
        assert!(
            up.at(fresh(120)) > forward.at(fresh(120)),
            "against a worn one the up smash is, and a scorer reading base \
             knockback alone would never pick it: forward={}, up={}",
            forward.at(fresh(120)),
            up.at(fresh(120))
        );
        assert!(up.grows_under(fresh(0)) && forward.grows_under(fresh(0)));
    }

    /// The slam's shock reaches ground the slam cannot. The two numbers are in
    /// two authoring vocabularies, so only this test holds them together.
    ///
    /// It must stay the weaker half: a follow-up that hit harder would make the
    /// slam a delivery mechanism for its own tail.
    #[test]
    fn his_ground_slam_sends_a_shock_that_outreaches_the_fists_and_hits_softer() {
        use ambition_entity_catalog::smash_riposte::{RiposteStrikeParams, RIPOSTE_STRIKE};

        let set = crate::authored_movesets::shipped("pugnacious_polygon");
        let slam = set
            .moves
            .iter()
            .find(|m| m.id == "polygon_brawler_ground_slam")
            .expect("his grounded down-B");

        let shock: RiposteStrikeParams = slam
            .events
            .iter()
            .find_map(|event| match &event.kind {
                ambition_entity_catalog::MoveEventKind::Effect(effect)
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
            // `leading_edge_x`, not `offset.0 + half_extents.0`:
            // `test_the_grab_reach_is_one_formula` requires the one formula, and this
            // file names `CaptureAttemptParams`, so that guard covers it.
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

    /// His punch charges and does not store.
    ///
    /// The Projectile Polygon's neutral-B is authored to store, at the
    /// maintainer's request. A ranged fighter banks a shot; a brawler commits.
    /// This test fails if either fighter is retuned toward the other.
    #[test]
    fn his_haymaker_charges_and_deliberately_does_not_store() {
        let set = crate::authored_movesets::shipped("pugnacious_polygon");
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
        let hers = crate::authored_movesets::shipped("projectile_polygon");
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
            .find(|effect| effect.key == ambition_entity_catalog::smash_capture::CAPTURE_ATTEMPT)
            .and_then(|effect| effect.params.hydrate().ok())
            .unwrap_or_else(|| panic!("`{id}` carries no capture attempt"))
    }

    /// His side-B is a command grab.
    ///
    /// Both grabs feed the same four throws, so the hold must match: a captive
    /// held elsewhere would make the throws read differently per grab.
    #[test]
    fn his_side_b_is_a_command_grab_that_shares_the_hold_with_his_standing_one() {
        let set = crate::authored_movesets::shipped("pugnacious_polygon");
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

    /// The brain can see how far the command grab reaches. A capture rides an
    /// Active window's `sustain_effect`, not its volume list, so `frame_data`
    /// folded over `volumes` alone gave every grab no region. Only the neutral
    /// grab (through `GRAB_VERB`) was patched; this one is bound to `attack_side`,
    /// so it was offered at every gap and priced at zero.
    ///
    /// The standing grab is the control: a derivation that fixes only the broken
    /// move is another special case.
    #[test]
    fn both_of_his_grabs_tell_the_brain_the_distance_they_close() {
        let set = crate::authored_movesets::shipped("pugnacious_polygon");
        for id in ["polygon_brawler_collar", "polygon_brawler_grab"] {
            let spec = set
                .moves
                .iter()
                .find(|m| m.id == id)
                .unwrap_or_else(|| panic!("`{id}` is on his table"));
            let frames = spec.frame_data();
            let authored = capture_of(&set, id).reach_x();
            assert_eq!(
                frames.reach, authored,
                "`{id}` reaches {authored}px and its frame data says {}",
                frames.reach,
            );
            let coverage = frames
                .coverage
                .unwrap_or_else(|| panic!("`{id}` has no region, so no gap can miss it"));
            assert_eq!(coverage.max.0, authored, "`{id}`'s region stops short of its reach");
            assert!(
                frames.ignores_guard,
                "`{id}` is a capture, and a raised shield is not the answer to one",
            );
        }
    }
    use super::*;

    /// His up-B opens a parasol that outlives it; the duration is the assertion,
    /// not the presence.
    ///
    /// A `WindowTag` would satisfy "the move slows his fall" and fail this. The
    /// regime must still run after the move's timeline ends.
    #[test]
    fn the_uppercut_leaves_him_floating_for_longer_than_the_move_lasts() {
        use ambition_entity_catalog::MoveEventKind;
        let moves = crate::authored_movesets::shipped("pugnacious_polygon");
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
        let moves = crate::authored_movesets::shipped("pugnacious_polygon");
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

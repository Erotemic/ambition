//! Bob's repertoire: the engineer, and the one who receives.
//!
//! ## The character, from his own name
//!
//! Alice sends and Bob receives. Hers is about getting something across; his
//! is about what happens when it arrives: he is slower to start than anyone
//! on the grid except the automaton, he commits longer, and his connect is
//! the hardest single hit among the Hall's people. An engineer does not
//! fence; he assembles.
//!
//! ```text
//!            reach   jab startup   f-smash damage   the trade
//!   alice     28 px     0.05 s          13          reach and recovery
//!   bob       26 px     0.07 s          16          slow, and it lands
//! ```
//!
//! Every one is a row a shipped generic sheet carries.

use ambition_entity_catalog::authoring::Strike;
use ambition_entity_catalog::smash_capture::{
    author_pummel, author_standing_grab, author_throw, capture_beat, grab_shell,
    CaptureAttemptParams, CaptureCues, CapturePummelParams, CaptureThrowParams,
    SmashCaptureRepertoire,
};
use ambition_entity_catalog::smash_repertoire::{
    DownSpecial, NeutralSpecial, SmashRepertoire, UpSpecial,
};
use ambition_entity_catalog::{AutolinkVolume, ImpulseMode, MovesetContract};

use ambition_entity_catalog::authoring::{
    committed_tail, impulse, multihit, on_contact, sfx, strike, strike_tag, vfx_at, Pulse,
};

const SHOP_FX: f32 = 0.85;
const RIG_FX: f32 = 1.2;

/// See the module doc. Sixteen presses.
pub fn bob_moveset() -> MovesetContract {
    // JAB — `tap_test`. He taps it to hear whether it is sound. Slower than
    // anybody else's jab, which is the whole character in the first press.
    let jab = strike(Strike {
        id: "tap_test",
        clip: "jab",
        startup_s: 0.07,
        active_s: 0.05,
        recover_s: 0.15,
        offset: (22.0, 0.0),
        half_extents: (16.0, 13.0),
        damage: 3,
        knockback: 50.0,
        knockback_growth: 1.05,
        launch_dir: None,
        on_hit: None,
    });
    let jab = strike_tag(jab, ambition_entity_catalog::authoring::SLASH_POKE_VFX);
    let jab = vfx_at(jab, 0.07, "hit_metal", (22.0, 0.0), SHOP_FX);
    let jab = on_contact(jab, "player.hit");

    // FORWARD TILT — `wrench_swing`. The tool, used as one.
    let f_tilt = strike(Strike {
        id: "wrench_swing",
        clip: "attack_side",
        startup_s: 0.10,
        active_s: 0.08,
        recover_s: 0.19,
        offset: (28.0, -2.0),
        half_extents: (20.0, 15.0),
        damage: 7,
        knockback: 78.0,
        knockback_growth: 1.32,
        launch_dir: Some((1.0, -0.30)),
        on_hit: None,
    });
    let f_tilt = vfx_at(f_tilt, 0.10, "hit_metal", (28.0, -2.0), SHOP_FX);
    let f_tilt = on_contact(f_tilt, "player.hit");

    // UP TILT — `pressure_release`. He opens a valve and it goes up.
    let u_tilt = strike(Strike {
        id: "pressure_release",
        clip: "attack_up",
        startup_s: 0.09,
        active_s: 0.08,
        recover_s: 0.19,
        offset: (6.0, -26.0),
        half_extents: (17.0, 22.0),
        damage: 6,
        knockback: 78.0,
        knockback_growth: 1.34,
        launch_dir: Some((0.12, -1.0)),
        on_hit: None,
    });
    let u_tilt = vfx_at(u_tilt, 0.09, "steam_vent", (6.0, -26.0), SHOP_FX);
    let u_tilt = on_contact(u_tilt, "player.hit");

    // DOWN TILT — `shim`. A wedge, driven in at floor level.
    let d_tilt = strike(Strike {
        id: "shim",
        clip: "attack_down",
        startup_s: 0.09,
        active_s: 0.06,
        recover_s: 0.18,
        offset: (24.0, 14.0),
        half_extents: (20.0, 10.0),
        damage: 5,
        knockback: 58.0,
        knockback_growth: 1.18,
        launch_dir: Some((0.9, -0.32)),
        on_hit: None,
    });
    let d_tilt = vfx_at(d_tilt, 0.09, "gear_scatter", (24.0, 14.0), SHOP_FX);
    let d_tilt = on_contact(d_tilt, "player.hit");

    // FORWARD SMASH — `rivet_smash`. The hardest single hit among the Hall's
    // people, and the longest wind-up to go with it.
    let f_smash = strike(Strike {
        id: "rivet_smash",
        clip: "smash_forward",
        startup_s: 0.21,
        active_s: 0.09,
        recover_s: 0.32,
        offset: (36.0, -2.0),
        half_extents: (26.0, 22.0),
        damage: 16,
        knockback: 134.0,
        knockback_growth: 3.19,
        launch_dir: Some((0.95, -0.45)),
        on_hit: None,
    });
    let f_smash = vfx_at(f_smash, 0.21, "electric_burst", (36.0, -2.0), RIG_FX);
    let f_smash = sfx(f_smash, 0.21, "player.attack.charge");
    let f_smash = on_contact(f_smash, "player.hit");

    // UP SMASH — `derrick_lift`. He raises the frame overhead and lets it
    // settle.
    let u_smash = strike(Strike {
        id: "derrick_lift",
        clip: "smash_up",
        startup_s: 0.19,
        active_s: 0.10,
        recover_s: 0.30,
        offset: (2.0, -32.0),
        half_extents: (22.0, 32.0),
        damage: 14,
        knockback: 126.0,
        knockback_growth: 5.87,
        launch_dir: Some((0.10, -1.0)),
        on_hit: None,
    });
    let u_smash = vfx_at(u_smash, 0.19, "gear_scatter", (2.0, -32.0), RIG_FX);
    let u_smash = on_contact(u_smash, "player.hit");

    // DOWN SMASH — `ground_anchor`. Two bolts, one either side, into the
    // floor.
    let d_smash = strike(Strike {
        id: "ground_anchor",
        clip: "smash_down",
        startup_s: 0.20,
        active_s: 0.09,
        recover_s: 0.32,
        offset: (0.0, 19.0),
        half_extents: (38.0, 13.0),
        damage: 13,
        knockback: 118.0,
        knockback_growth: 2.75,
        launch_dir: Some((0.8, -0.55)),
        on_hit: None,
    });
    let d_smash = vfx_at(d_smash, 0.20, "shockwave", (0.0, 19.0), RIG_FX);
    let d_smash = on_contact(d_smash, "player.hit");

    // NEUTRAL AIR — `loose_bearing`. Something comes off and goes round him.
    let n_air = strike(Strike {
        id: "loose_bearing",
        clip: "air_neutral",
        startup_s: 0.08,
        active_s: 0.10,
        recover_s: 0.19,
        offset: (0.0, 0.0),
        half_extents: (25.0, 22.0),
        damage: 6,
        knockback: 70.0,
        knockback_growth: 1.38,
        launch_dir: Some((0.55, -0.72)),
        on_hit: None,
    });
    let n_air = vfx_at(n_air, 0.08, "gear_scatter", (0.0, 0.0), SHOP_FX);
    let n_air = on_contact(n_air, "player.hit");

    // FORWARD AIR — `swing_arm`. A long arc from the shoulder.
    let f_air = strike(Strike {
        id: "swing_arm",
        clip: "air_forward",
        startup_s: 0.10,
        active_s: 0.08,
        recover_s: 0.21,
        offset: (28.0, -4.0),
        half_extents: (22.0, 18.0),
        damage: 9,
        knockback: 96.0,
        knockback_growth: 1.75,
        launch_dir: Some((0.95, -0.42)),
        on_hit: None,
    });
    let f_air = vfx_at(f_air, 0.10, "hit_metal", (28.0, -4.0), SHOP_FX);
    let f_air = on_contact(f_air, "player.hit");

    // BACK AIR — `counterweight`. He swings the mass the other way and it
    // takes whoever was there.
    let b_air = strike(Strike {
        id: "counterweight",
        clip: "air_back",
        startup_s: 0.11,
        active_s: 0.07,
        recover_s: 0.22,
        offset: (-28.0, -2.0),
        half_extents: (22.0, 18.0),
        damage: 10,
        knockback: 104.0,
        knockback_growth: 1.90,
        launch_dir: Some((-0.95, -0.38)),
        on_hit: None,
    });
    let b_air = vfx_at(b_air, 0.11, "hit_metal", (-28.0, -2.0), SHOP_FX);
    let b_air = on_contact(b_air, "player.hit");

    // UP AIR — `jack_stand`. Straight up, on the hard part.
    let u_air = strike(Strike {
        id: "jack_stand",
        clip: "air_up",
        startup_s: 0.09,
        active_s: 0.08,
        recover_s: 0.19,
        offset: (2.0, -26.0),
        half_extents: (19.0, 23.0),
        damage: 8,
        knockback: 86.0,
        knockback_growth: 1.64,
        launch_dir: Some((0.08, -1.0)),
        on_hit: None,
    });
    let u_air = vfx_at(u_air, 0.09, "electric_arc", (2.0, -26.0), SHOP_FX);
    let u_air = on_contact(u_air, "player.hit");

    // DOWN AIR — `pile_driver`. He puts his whole weight through it.
    let d_air = strike(Strike {
        id: "pile_driver",
        clip: "air_down",
        startup_s: 0.13,
        active_s: 0.07,
        recover_s: 0.24,
        offset: (2.0, 25.0),
        half_extents: (20.0, 20.0),
        damage: 11,
        knockback: 114.0,
        knockback_growth: 2.05,
        launch_dir: Some((0.0, 1.0)),
        on_hit: None,
    });
    let d_air = vfx_at(d_air, 0.13, "shockwave", (2.0, 25.0), SHOP_FX);
    let d_air = on_contact(d_air, "player.hit");

    // NEUTRAL — `rivet_gun`. Held down and driven home. His longest active
    // window: it is not one hit, it is the tool running.
    let n_b = strike(Strike {
        id: "rivet_gun",
        clip: "attack",
        startup_s: 0.20,
        active_s: 0.14,
        recover_s: 0.30,
        offset: (30.0, -2.0),
        half_extents: (26.0, 18.0),
        damage: 12,
        knockback: 112.0,
        knockback_growth: 2.00,
        launch_dir: Some((0.92, -0.44)),
        on_hit: None,
    });
    // It is a real multi-hit: "not one hit, the tool running". One long window
    // or touching windows land once (see `Pulse`), so `multihit` builds
    // separated windows, like Oiler's `convergence`.
    //
    // The finisher is unchanged. The pulses go in front at 2 chip each: a
    // multi-hit's pulses should not hurt more than its finisher.
    //
    // The anchor's x is zero: `autolink_anchor_world` mirrors it with facing,
    // and a non-zero x would move the hold point with his facing.
    let n_b = multihit(
        n_b,
        3,
        Pulse {
            // Slightly tighter than the finisher: the work is held where the tool is.
            offset: (26.0, -2.0),
            half_extents: (22.0, 16.0),
            damage: 2,
            // Separated: touching windows land once.
            active_s: 0.030,
            gap_s: 0.028,
            autolink: AutolinkVolume {
                anchor: (0.0, -4.0),
                // He is planted, so he has no motion to pass on; the hold is all
                // correction.
                carry: 0.0,
                pull: 19.0,
                max_speed: 900.0,
            },
        },
    );
    let n_b = committed_tail(n_b, 0.70, 0.05);
    let n_b = vfx_at(n_b, 0.20, "electric_burst", (30.0, -2.0), RIG_FX);
    let n_b = sfx(n_b, 0.20, "player.directional_special");
    let n_b = on_contact(n_b, "player.hit");

    // SIDE — `piston_charge`. He is committed the instant it fires, and the
    // tail damps to nothing: an engineer's dash has no take-backs.
    let side_b = strike(Strike {
        id: "piston_charge",
        clip: "attack_side",
        startup_s: 0.16,
        active_s: 0.10,
        recover_s: 0.28,
        offset: (28.0, 0.0),
        half_extents: (24.0, 20.0),
        damage: 12,
        knockback: 114.0,
        knockback_growth: 2.05,
        launch_dir: Some((0.95, -0.35)),
        on_hit: None,
    });
    let side_b = impulse(side_b, 0.16, (620.0, 0.0), ImpulseMode::Set);
    let side_b = committed_tail(side_b, 0.66, 0.0);
    let side_b = vfx_at(side_b, 0.16, "steam_vent", (-16.0, 0.0), RIG_FX);
    let side_b = sfx(side_b, 0.16, "player.dash");
    let side_b = on_contact(side_b, "player.hit");

    // Up: `steam_lift`, the recovery. Boiler pressure, spent at once. Higher
    // than Alice's and costlier to land, the same bargain as his whole kit.
    let mut up_b = strike(Strike {
        id: "steam_lift",
        clip: "attack_up",
        startup_s: 0.09,
        active_s: 0.12,
        recover_s: 0.22,
        offset: (0.0, -12.0),
        half_extents: (21.0, 32.0),
        damage: 8,
        knockback: 90.0,
        knockback_growth: 1.68,
        launch_dir: Some((0.10, -1.0)),
        on_hit: None,
    });
    up_b.landing_lag_s = Some(0.34);
    let up_b = impulse(up_b, 0.09, (0.0, -800.0), ImpulseMode::Set);
    let up_b = committed_tail(up_b, 0.54, 0.10);
    let up_b = vfx_at(up_b, 0.09, "steam_vent", (0.0, 18.0), RIG_FX);
    let up_b = sfx(up_b, 0.09, "player.fly.start");
    let up_b = on_contact(up_b, "player.hit");

    // DOWN — `bulkhead_drop`. He drops a plate. Grounded-only, because the
    // move is that there is a floor to drop it onto.
    let down_b = strike(Strike {
        id: "bulkhead_drop",
        clip: "attack_down",
        startup_s: 0.18,
        active_s: 0.10,
        recover_s: 0.32,
        offset: (0.0, 20.0),
        half_extents: (34.0, 14.0),
        damage: 12,
        knockback: 104.0,
        knockback_growth: 1.80,
        launch_dir: Some((0.7, -0.66)),
        on_hit: None,
    });
    // The plate stays: the slam also places a launch pad.
    //
    // The slam is unchanged; the plate is an added event, like the mine on the
    // Polygon's down smash.
    //
    // It throws anybody: a persistent actuator another fighter can use. Three
    // uses, eight seconds, and whoever steps on it goes up, including his
    // opponent.
    let down_b = ambition_entity_catalog::smash_spring::author_place_spring(
        down_b,
        // The frame the plate meets the floor: the same instant as the slam's
        // shockwave and landing puff, so it appears where the impact is drawn.
        0.18,
        ambition_entity_catalog::smash_spring::PlaceSpringParams {
            // Up is negative y. A real reposition, but below his own `steam_lift`, so
            // the plate does not beat his recovery. The guard below checks this
            // relationship against `steam_lift`'s impulse.
            launch: (0.0, -720.0),
            // A plate, not a platform: wide enough to step on, thin enough to miss.
            half_extents: (26.0, 6.0),
            // Short: a plate that outlived its exchange would be terrain he authored,
            // and terrain belongs to another authority.
            lifetime_s: 8.0,
            uses: 3,
            // At his feet, where the slam landed.
            offset: (0.0, 20.0),
            // So the other player sees it arrive. It draws nothing of its own (see
            // `PlaceSpringParams::vfx`).
            vfx: "steam_vent".to_string(),
        },
    );
    let down_b = committed_tail(down_b, 0.70, 0.0);
    let down_b = vfx_at(down_b, 0.18, "shockwave", (0.0, 20.0), RIG_FX);
    let down_b = vfx_at(down_b, 0.18, "landing_puff", (0.0, 22.0), SHOP_FX);
    let down_b = on_contact(down_b, "player.hit");

    // Down-B has two forms, like Bowser's: a slam in the air, an arc and slam
    // on the ground. Context-dependent specials are acceptable, though most
    // should not be.
    //
    // A special gated to one posture is not answered in the other: the
    // directional chain falls through to the neutral special.
    // `special_air_down` comes before `special_down` in that chain.
    // Down, in the air: `bulkhead_dive`. He rides the plate down.
    let mut air_down_b = strike(Strike {
        id: "bulkhead_dive",
        clip: "air_down",
        startup_s: 0.12,
        active_s: 0.10,
        recover_s: 0.26,
        offset: (0.0, 24.0),
        half_extents: (22.0, 22.0),
        damage: 11,
        knockback: 106.0,
        knockback_growth: 1.80,
        launch_dir: Some((0.0, 1.0)),
        on_hit: None,
    });
    air_down_b.landing_lag_s = Some(0.34);
    let air_down_b = impulse(air_down_b, 0.12, (0.0, 1300.0), ImpulseMode::Set);
    let air_down_b = vfx_at(air_down_b, 0.12, "shockwave", (0.0, 22.0), SHOP_FX);
    let air_down_b = on_contact(air_down_b, "player.hit");
    // Bob's capture kit: heavy and slow. The longest reach and the hardest
    // single pummel, paid for with the worst startup and recovery.
    // The grab draws `attack`, not `grab`: these sheets publish no `grab` row,
    // and each table's clip guard refuses unpublished rows.
    let grab = author_standing_grab(
        grab_shell("bob_grab", "attack", 0.09, 0.06, 0.24),
        CaptureAttemptParams {
            offset: (12.0, 1.0),
            half_extents: (22.0, 17.0),
            hold_offset: (13.0, 3.0),
        },
    );
    let pummel = author_pummel(
        capture_beat("bob_pummel", "attack", 0.28),
        0.12,
        CapturePummelParams { damage: 5 },
    );
    let forward_throw = author_throw(
        capture_beat("bob_fthrow", "attack", 0.3),
        0.16,
        CaptureThrowParams {
            damage: 10,
            knockback: 132.0,
            knockback_growth: 1.8,
            launch_dir: (0.7, -0.7),
        },
    );

    let back_throw = author_throw(
        capture_beat("bob_bthrow", "attack", 0.32),
        0.17,
        CaptureThrowParams {
            damage: 11,
            knockback: 142.56,
            knockback_growth: 1.89,
            launch_dir: (-1.0, -0.43),
        },
    );

    let up_throw = author_throw(
        capture_beat("bob_uthrow", "attack", 0.31),
        0.16,
        CaptureThrowParams {
            damage: 10,
            knockback: 137.28,
            knockback_growth: 1.84,
            launch_dir: (0.0, -1.0),
        },
    );

    let down_throw = author_throw(
        capture_beat("bob_dthrow", "attack", 0.33),
        0.17,
        CaptureThrowParams {
            damage: 8,
            knockback: 97.68,
            knockback_growth: 1.44,
            launch_dir: (0.28, -0.92),
        },
    );

    SmashRepertoire {
        taunt: ambition_entity_catalog::authoring::taunt("bob_taunt", 0.9),
        dash_attack: ambition_entity_catalog::authoring::dash_attack(
            "bob_dash_attack",
            ambition_entity_catalog::authoring::DashAttackShape::GENRE,
            9,
            97.5,
        ),
        jab,
        forward_tilt: f_tilt,
        up_tilt: u_tilt,
        down_tilt: d_tilt,
        forward_smash: f_smash,
        up_smash: u_smash,
        down_smash: d_smash,
        neutral_air: n_air,
        forward_air: f_air,
        back_air: b_air,
        up_air: u_air,
        down_air: d_air,
        neutral_special: NeutralSpecial::Authored(n_b),
        side_special: side_b,
        up_special: UpSpecial::Standard(up_b),
        // Every smash fighter has a grab. The values are per character on purpose.
        capture: SmashCaptureRepertoire {
            cues: CaptureCues::GENERIC,
            grab,
            pummel,
            forward_throw,
            back_throw: Some(back_throw),
            up_throw: Some(up_throw),
            down_throw: Some(down_throw),
        },
        down_special: DownSpecial::ByPosture {
            grounded: down_b,
            airborne: air_down_b,
        },
    }
    .into_contract()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Verb binding is checked by construction: `SmashRepertoire` owns the verb
    // strings and is a struct with no `Default`, so a missing slot is a compile
    // error. Coverage in every posture is checked by
    // `ambition_entity_catalog::smash_repertoire` and by
    // `smash_roster_movesets::report_the_smash_kit_every_selectable_fighter_has`.

    /// He commits for longer than she does, on every press they both have.
    /// The pair's other half is asserted in `alice_moveset`; this is the axis
    /// that is his.
    #[test]
    fn bob_is_slower_to_start_than_alice_on_every_shared_press() {
        let bob = bob_moveset();
        let alice = crate::alice_moveset::alice_moveset();
        let startup = |set: &MovesetContract, verb: &str| {
            set.move_for_verb(verb)
                .unwrap_or_else(|| panic!("{verb} is bound"))
                .windows
                .iter()
                .find(|w| {
                    matches!(
                        w.tag,
                        ambition_entity_catalog::WindowTag::Active
                    )
                })
                .expect("a strike has an active window")
                .start_s
        };
        for verb in ["attack", "attack_forward", "smash_forward", "attack_air"] {
            assert!(
                startup(&bob, verb) > startup(&alice, verb),
                "`{verb}` comes out at least as fast for the engineer as for the \
                 cryptographer, so the pair is one table twice"
            );
        }
    }

    /// "Not one hit, the tool running": the neutral multi-hits. One contiguous
    /// window lands once (see `Pulse`).
    #[test]
    fn his_rivet_gun_runs_rather_than_landing_once() {
        let set = bob_moveset();
        let gun = set
            .moves
            .iter()
            .find(|m| m.id == "rivet_gun")
            .expect("his neutral special");

        let mut hitting: Vec<(f32, f32)> = gun
            .windows
            .iter()
            .filter(|w| w.volumes.iter().any(|v| v.damage > 0))
            .map(|w| (w.start_s, w.end_s))
            .collect();
        hitting.sort_by(|a, b| a.0.total_cmp(&b.0));
        assert!(
            hitting.len() >= 3,
            "a tool that runs has more than {} hitting window(s)",
            hitting.len()
        );

        // The gaps are the move, not the count: touching windows land once, so
        // assert the separation.
        for pair in hitting.windows(2) {
            let gap = pair[1].0 - pair[0].1;
            assert!(
                gap > 0.0,
                "windows {:?} and {:?} touch, so the runtime hands the hit set \
                 forward and the tool lands once",
                pair[0],
                pair[1]
            );
        }

        // The pulses hold: an intermediate hit that launches throws the victim out
        // of the later windows.
        let holding = gun
            .windows
            .iter()
            .flat_map(|w| w.volumes.iter())
            .filter(|v| v.reaction.is_some())
            .count();
        assert!(holding >= 3, "only {holding} of the pulses hold their victim");

        // The finisher is unchanged: an addition, not a rebalance.
        assert!(
            gun.windows
                .iter()
                .flat_map(|w| w.volumes.iter())
                .any(|v| v.reaction.is_none() && v.damage == 12),
            "the rivet no longer drives home at its authored 12"
        );
    }

    /// The plate is an addition and the slam is unchanged. Both in one test: a
    /// check for the plate alone would pass a down-B that lost its hitbox.
    #[test]
    fn his_bulkhead_drop_still_slams_and_now_leaves_the_plate_it_names() {
        let set = bob_moveset();
        let drop = set
            .moves
            .iter()
            .find(|m| m.id == "bulkhead_drop")
            .expect("his grounded down special");

        // The slam, unchanged.
        assert!(
            drop.windows
                .iter()
                .flat_map(|w| w.volumes.iter())
                .any(|v| v.damage == 12),
            "the slam lost its authored damage"
        );

        let plate: ambition_entity_catalog::smash_spring::PlaceSpringParams = drop
            .events
            .iter()
            .find_map(|event| match &event.kind {
                ambition_entity_catalog::MoveEventKind::Effect(effect)
                    if effect.key
                        == ambition_entity_catalog::smash_spring::PLACE_SPRING =>
                {
                    effect.params.hydrate().ok()
                }
                _ => None,
            })
            .expect("he drops a plate, which his comment has always said");

        // It throws upward. Up is negative y.
        assert!(plate.launch.1 < 0.0, "the plate throws downward: {:?}", plate.launch);
        assert!(plate.uses > 0, "a plate nobody can use is an invisible object");

        // Short-lived: a plate that outlived its exchange would be terrain.
        assert!(
            plate.lifetime_s <= 12.0,
            "the plate lasts {}s, which is stage geometry rather than a move",
            plate.lifetime_s
        );

        // It must not out-launch his own recovery.
        let lift = set
            .moves
            .iter()
            .find(|m| m.id == "steam_lift")
            .expect("his recovery");
        let rise = lift
            .events
            .iter()
            .find_map(|event| match &event.kind {
                ambition_entity_catalog::MoveEventKind::Impulse { local, .. } => {
                    Some(local.1.abs())
                }
                _ => None,
            })
            .unwrap_or(f32::INFINITY);
        assert!(
            plate.launch.1.abs() < rise,
            "the plate ({}) throws harder than his recovery ({rise})",
            plate.launch.1.abs(),
        );
    }
}

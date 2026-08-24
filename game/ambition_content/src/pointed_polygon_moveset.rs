//! Pointed Polygon — sword archetype repertoire.
//!
//! This is intentionally a FUNDAMENTALS table. The character exists partly as a
//! safe animation reference for future humanoids, so every common Smash verb has
//! a clear, conventional answer rather than a gimmick. The distinctive choice is
//! reach: the sword extends ordinary humanoid spacing without turning the fighter
//! into a heavyweight or projectile character.

use ambition_characters::moveset_authoring::Strike;
use ambition_characters::moveset_authoring::{committed_tail, impulse, multihit, strike, Pulse};
use ambition_characters::smash_capture::{
    author_pummel, author_standing_grab, author_throw, capture_beat, grab_shell,
    CaptureAttemptParams, CaptureCues, CapturePummelParams, CaptureThrowParams,
    SmashCaptureRepertoire,
};
use ambition_characters::smash_repertoire::{DownSpecial, NeutralSpecial, SmashRepertoire};
use ambition_platformer2d::entity_catalog::AutolinkVolume;
use ambition_platformer2d::entity_catalog::{ImpulseMode, MovesetContract};

/// Complete sword-fundamentals repertoire: every typed Smash slot plus all four throws.
pub fn pointed_polygon_moveset() -> MovesetContract {
    // Grounded normals.
    let jab = strike(Strike {
        id: "polygon_jab",
        clip: "jab",
        startup_s: 0.05,
        active_s: 0.05,
        recover_s: 0.12,
        offset: (30.0, -1.0),
        half_extents: (23.0, 13.0),
        damage: 3,
        knockback: 48.0,
        knockback_growth: 1.05,
        launch_dir: Some((1.0, -0.18)),
        on_hit: None,
    });
    let forward_tilt = strike(Strike {
        id: "polygon_tilt_forward",
        clip: "attack_side",
        startup_s: 0.08,
        active_s: 0.07,
        recover_s: 0.17,
        offset: (36.0, -3.0),
        half_extents: (27.0, 14.0),
        damage: 5,
        knockback: 72.0,
        knockback_growth: 1.38,
        launch_dir: Some((1.0, -0.28)),
        on_hit: None,
    });
    let up_tilt = strike(Strike {
        id: "polygon_tilt_up",
        clip: "attack_up",
        startup_s: 0.08,
        active_s: 0.07,
        recover_s: 0.17,
        offset: (10.0, -30.0),
        half_extents: (22.0, 23.0),
        damage: 5,
        knockback: 78.0,
        knockback_growth: 1.45,
        launch_dir: Some((0.15, -1.0)),
        on_hit: None,
    });
    let down_tilt = strike(Strike {
        id: "polygon_tilt_down",
        clip: "attack_down",
        startup_s: 0.07,
        active_s: 0.07,
        recover_s: 0.16,
        offset: (30.0, 11.0),
        half_extents: (25.0, 10.0),
        damage: 4,
        knockback: 58.0,
        knockback_growth: 1.22,
        launch_dir: Some((1.0, -0.18)),
        on_hit: None,
    });

    // Smashes: legible, committed kill swings.
    let mut forward_smash = strike(Strike {
        id: "polygon_smash_forward",
        clip: "smash_forward",
        startup_s: 0.24,
        active_s: 0.08,
        recover_s: 0.30,
        offset: (44.0, -4.0),
        half_extents: (31.0, 19.0),
        damage: 14,
        knockback: 148.0,
        knockback_growth: 3.05,
        launch_dir: Some((1.0, -0.36)),
        on_hit: None,
    });
    forward_smash.smash_charge_mult = 1.7;
    let mut up_smash = strike(Strike {
        id: "polygon_smash_up",
        clip: "smash_up",
        startup_s: 0.22,
        active_s: 0.08,
        recover_s: 0.29,
        offset: (4.0, -34.0),
        half_extents: (24.0, 29.0),
        damage: 13,
        knockback: 146.0,
        knockback_growth: 2.95,
        launch_dir: Some((0.0, -1.0)),
        on_hit: None,
    });
    up_smash.smash_charge_mult = 1.7;
    let mut down_smash = strike(Strike {
        id: "polygon_smash_down",
        clip: "smash_down",
        startup_s: 0.20,
        active_s: 0.09,
        recover_s: 0.31,
        offset: (0.0, 13.0),
        half_extents: (38.0, 12.0),
        damage: 12,
        knockback: 132.0,
        knockback_growth: 2.72,
        launch_dir: Some((0.85, -0.52)),
        on_hit: None,
    });
    down_smash.smash_charge_mult = 1.7;

    // Aerials.
    let neutral_air = strike(Strike {
        id: "polygon_air_neutral",
        clip: "air_neutral",
        startup_s: 0.06,
        active_s: 0.10,
        recover_s: 0.15,
        offset: (9.0, 0.0),
        half_extents: (27.0, 22.0),
        damage: 6,
        knockback: 70.0,
        knockback_growth: 1.42,
        launch_dir: None,
        on_hit: None,
    });
    let forward_air = strike(Strike {
        id: "polygon_air_forward",
        clip: "air_forward",
        startup_s: 0.09,
        active_s: 0.08,
        recover_s: 0.18,
        offset: (36.0, -4.0),
        half_extents: (27.0, 17.0),
        damage: 8,
        knockback: 98.0,
        knockback_growth: 1.94,
        launch_dir: Some((1.0, -0.34)),
        on_hit: None,
    });
    let back_air = strike(Strike {
        id: "polygon_air_back",
        clip: "air_back",
        startup_s: 0.10,
        active_s: 0.07,
        recover_s: 0.20,
        offset: (-31.0, -1.0),
        half_extents: (24.0, 16.0),
        damage: 9,
        knockback: 116.0,
        knockback_growth: 2.20,
        launch_dir: Some((-1.0, -0.32)),
        on_hit: None,
    });
    let up_air = strike(Strike {
        id: "polygon_air_up",
        clip: "air_up",
        startup_s: 0.07,
        active_s: 0.08,
        recover_s: 0.15,
        offset: (2.0, -30.0),
        half_extents: (22.0, 23.0),
        damage: 7,
        knockback: 91.0,
        knockback_growth: 1.80,
        launch_dir: Some((0.0, -1.0)),
        on_hit: None,
    });
    let mut down_air = strike(Strike {
        id: "polygon_air_down",
        clip: "air_down",
        startup_s: 0.12,
        active_s: 0.08,
        recover_s: 0.23,
        offset: (4.0, 27.0),
        half_extents: (20.0, 21.0),
        damage: 9,
        knockback: 118.0,
        knockback_growth: 2.25,
        launch_dir: Some((0.0, 1.0)),
        on_hit: None,
    });
    down_air.landing_lag_s = Some(0.24);

    // Specials deliberately teach common sword archetype motion.
    let neutral_special = committed_tail(
        strike(Strike {
            id: "polygon_point",
            clip: "slash",
            startup_s: 0.14,
            active_s: 0.08,
            recover_s: 0.24,
            offset: (48.0, -3.0),
            half_extents: (28.0, 12.0),
            damage: 10,
            knockback: 112.0,
            knockback_growth: 2.15,
            launch_dir: Some((1.0, -0.18)),
            on_hit: None,
        }),
        0.52,
        0.20,
    );

    let side_special = impulse(
        committed_tail(
            strike(Strike {
                id: "polygon_vector_lunge",
                clip: "attack_side",
                startup_s: 0.13,
                active_s: 0.10,
                recover_s: 0.24,
                offset: (41.0, -2.0),
                half_extents: (30.0, 16.0),
                damage: 9,
                knockback: 104.0,
                knockback_growth: 2.02,
                launch_dir: Some((1.0, -0.28)),
                on_hit: None,
            }),
            0.58,
            0.12,
        ),
        0.13,
        (520.0, 0.0),
        ImpulseMode::Set,
    );

    // ⭐ THE RISING SPIN: four holding pulses, then one launch.
    //
    // It used to be a single hit on the way up, which meant the move either
    // connected once and sent the victim away or missed entirely — the climb had
    // no reason to be long. Four autolink pulses make the rise itself the
    // mechanic: each one re-aims the victim at a point just in front of the
    // spinning fighter, so it comes UP with the move and the finisher has
    // something to launch.
    //
    // ⛔ NOT a capture. Nothing is held: each pulse is an ordinary weak hit whose
    // reaction happens to aim inward, and the victim keeps every verb it has —
    // it can DI, it can tech the ending, and it falls out the moment the pulses
    // stop reaching it.
    let mut rising_edge = strike(Strike {
        id: "polygon_rising_edge",
        clip: "attack_up",
        startup_s: 0.09,
        // The FINISHER, unchanged in character: the launch that ends the spin.
        active_s: 0.10,
        recover_s: 0.20,
        offset: (5.0, -19.0),
        half_extents: (22.0, 27.0),
        damage: 7,
        knockback: 88.0,
        knockback_growth: 1.65,
        launch_dir: Some((0.10, -1.0)),
        on_hit: None,
    });
    rising_edge.landing_lag_s = Some(0.25);
    let rising_edge = multihit(
        rising_edge,
        4,
        Pulse {
            // Slightly wider than the finisher and centred on the body: the
            // pulses have to keep reaching a victim that is being carried.
            offset: (2.0, -12.0),
            half_extents: (26.0, 30.0),
            damage: 2,
            // Separated windows, because the runtime's re-hit rule refuses a
            // contiguous track — four touching windows would land once.
            active_s: 0.035,
            gap_s: 0.030,
            autolink: AutolinkVolume {
                // Just in front of the spin and a little below its centre, so
                // the victim rides at the height the finisher's box covers.
                anchor: (14.0, 6.0),
                // The whole of the climb. The correction only closes a gap, and
                // this fighter is rising at 760 px/s — anything less and the
                // victim is left underneath its own move.
                carry: 1.0,
                pull: 22.0,
                max_speed: 900.0,
            },
        },
    );
    let up_special = impulse(rising_edge, 0.09, (0.0, -760.0), ImpulseMode::Set);

    let grounded_down_special = committed_tail(
        strike(Strike {
            id: "polygon_low_arc",
            clip: "attack_down",
            startup_s: 0.12,
            active_s: 0.09,
            recover_s: 0.25,
            offset: (18.0, 13.0),
            half_extents: (34.0, 11.0),
            damage: 8,
            knockback: 82.0,
            knockback_growth: 1.55,
            launch_dir: Some((0.8, -0.55)),
            on_hit: None,
        }),
        0.52,
        0.05,
    );
    let mut airborne_down_special = strike(Strike {
        id: "polygon_falling_edge",
        clip: "air_down",
        startup_s: 0.10,
        active_s: 0.10,
        recover_s: 0.23,
        offset: (0.0, 25.0),
        half_extents: (21.0, 22.0),
        damage: 9,
        knockback: 105.0,
        knockback_growth: 1.95,
        launch_dir: Some((0.0, 1.0)),
        on_hit: None,
    });
    airborne_down_special.landing_lag_s = Some(0.27);
    let airborne_down_special =
        impulse(airborne_down_special, 0.10, (0.0, 1050.0), ImpulseMode::Set);

    // Capture kit. Unlike several older fighters, the reference archetype answers
    // every throw direction so animation authors have a safe pose for all four.
    let grab = author_standing_grab(
        grab_shell("polygon_grab", "grab", 0.07, 0.05, 0.22),
        CaptureAttemptParams {
            offset: (16.0, 1.0),
            half_extents: (19.0, 15.0),
            hold_offset: (15.0, 3.0),
        },
    );
    let pummel = author_pummel(
        capture_beat("polygon_pummel", "pummel", 0.16),
        0.07,
        CapturePummelParams { damage: 3 },
    );
    let forward_throw = author_throw(
        capture_beat("polygon_throw_forward", "throw_forward", 0.25),
        0.12,
        CaptureThrowParams {
            damage: 7,
            knockback: 108.0,
            knockback_growth: 2.15,
            launch_dir: (1.0, -0.34),
        },
    );
    let back_throw = author_throw(
        capture_beat("polygon_throw_back", "throw_back", 0.27),
        0.13,
        CaptureThrowParams {
            damage: 8,
            knockback: 116.0,
            knockback_growth: 2.25,
            launch_dir: (-1.0, -0.30),
        },
    );
    let up_throw = author_throw(
        capture_beat("polygon_throw_up", "throw_up", 0.26),
        0.12,
        CaptureThrowParams {
            damage: 7,
            knockback: 112.0,
            knockback_growth: 2.18,
            launch_dir: (0.0, -1.0),
        },
    );
    let down_throw = author_throw(
        capture_beat("polygon_throw_down", "throw_down", 0.28),
        0.13,
        CaptureThrowParams {
            damage: 6,
            knockback: 82.0,
            knockback_growth: 1.75,
            launch_dir: (0.35, -0.92),
        },
    );

    SmashRepertoire {
        // See `select.rs` for the same shape: a stale copy is a revert with no diff to review.
        taunt: ambition_characters::moveset_authoring::taunt("pointed_polygon_taunt", 0.9),
        // the genre shape, deliberately: this character is the REFERENCE rig,
        // so its dash attack is the one a new humanoid should copy before it has
        // a reason to differ. A bespoke reach here would be a number nobody
        // chose being copied into every fighter that starts from these poses.
        dash_attack: ambition_characters::moveset_authoring::dash_attack(
            "pointed_polygon_dash_attack",
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
        up_special,
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
    .into_contract()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_reference_sword_fighter_answers_the_complete_typed_repertoire() {
        let moves = pointed_polygon_moveset();
        for id in [
            "polygon_jab",
            "pointed_polygon_dash_attack",
            "pointed_polygon_taunt",
            "polygon_tilt_forward",
            "polygon_tilt_up",
            "polygon_tilt_down",
            "polygon_smash_forward",
            "polygon_smash_up",
            "polygon_smash_down",
            "polygon_air_neutral",
            "polygon_air_forward",
            "polygon_air_back",
            "polygon_air_up",
            "polygon_air_down",
            "polygon_point",
            "polygon_vector_lunge",
            "polygon_rising_edge",
            "polygon_low_arc",
            "polygon_falling_edge",
            "polygon_grab",
            "polygon_pummel",
            "polygon_throw_forward",
            "polygon_throw_back",
            "polygon_throw_up",
            "polygon_throw_down",
        ] {
            assert!(moves.moves.iter().any(|m| m.id == id), "missing {id}");
        }
    }
}

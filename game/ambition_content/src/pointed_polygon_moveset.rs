//! Pointed Polygon — sword archetype repertoire.
//!
//! This is intentionally a FUNDAMENTALS table. The character exists partly as a
//! safe animation reference for future humanoids, so every common Smash verb has
//! a clear, conventional answer rather than a gimmick. The distinctive choice is
//! reach: the sword extends ordinary humanoid spacing without turning the fighter
//! into a heavyweight or projectile character.

use ambition_characters::moveset_authoring::{committed_tail, impulse, strike};
use ambition_characters::smash_capture::{
    author_pummel, author_standing_grab, author_throw, capture_beat, grab_shell,
    CaptureAttemptParams, CaptureCues, CapturePummelParams, CaptureThrowParams,
    SmashCaptureRepertoire,
};
use ambition_characters::smash_repertoire::{DownSpecial, NeutralSpecial, SmashRepertoire};
use ambition_platformer2d::entity_catalog::{ImpulseMode, MovesetContract};

/// Complete sword-fundamentals repertoire: every typed Smash slot plus all four throws.
pub fn pointed_polygon_moveset() -> MovesetContract {
    // Grounded normals.
    let jab = strike(
        "polygon_jab", "jab", 0.05, 0.05, 0.12,
        (30.0, -1.0), (23.0, 13.0), 3, 48.0, 1.05, Some((1.0, -0.18)), None,
    );
    let forward_tilt = strike(
        "polygon_tilt_forward", "attack_side", 0.08, 0.07, 0.17,
        (36.0, -3.0), (27.0, 14.0), 5, 72.0, 1.38, Some((1.0, -0.28)), None,
    );
    let up_tilt = strike(
        "polygon_tilt_up", "attack_up", 0.08, 0.07, 0.17,
        (10.0, -30.0), (22.0, 23.0), 5, 78.0, 1.45, Some((0.15, -1.0)), None,
    );
    let down_tilt = strike(
        "polygon_tilt_down", "attack_down", 0.07, 0.07, 0.16,
        (30.0, 11.0), (25.0, 10.0), 4, 58.0, 1.22, Some((1.0, -0.18)), None,
    );

    // Smashes: legible, committed kill swings.
    let mut forward_smash = strike(
        "polygon_smash_forward", "smash_forward", 0.24, 0.08, 0.30,
        (44.0, -4.0), (31.0, 19.0), 14, 148.0, 3.05, Some((1.0, -0.36)), None,
    );
    forward_smash.smash_charge_mult = 1.7;
    let mut up_smash = strike(
        "polygon_smash_up", "smash_up", 0.22, 0.08, 0.29,
        (4.0, -34.0), (24.0, 29.0), 13, 146.0, 2.95, Some((0.0, -1.0)), None,
    );
    up_smash.smash_charge_mult = 1.7;
    let mut down_smash = strike(
        "polygon_smash_down", "smash_down", 0.20, 0.09, 0.31,
        (0.0, 13.0), (38.0, 12.0), 12, 132.0, 2.72, Some((0.85, -0.52)), None,
    );
    down_smash.smash_charge_mult = 1.7;

    // Aerials.
    let neutral_air = strike(
        "polygon_air_neutral", "air_neutral", 0.06, 0.10, 0.15,
        (9.0, 0.0), (27.0, 22.0), 6, 70.0, 1.42, None, None,
    );
    let forward_air = strike(
        "polygon_air_forward", "air_forward", 0.09, 0.08, 0.18,
        (36.0, -4.0), (27.0, 17.0), 8, 98.0, 1.94, Some((1.0, -0.34)), None,
    );
    let back_air = strike(
        "polygon_air_back", "air_back", 0.10, 0.07, 0.20,
        (-31.0, -1.0), (24.0, 16.0), 9, 116.0, 2.20, Some((-1.0, -0.32)), None,
    );
    let up_air = strike(
        "polygon_air_up", "air_up", 0.07, 0.08, 0.15,
        (2.0, -30.0), (22.0, 23.0), 7, 91.0, 1.80, Some((0.0, -1.0)), None,
    );
    let mut down_air = strike(
        "polygon_air_down", "air_down", 0.12, 0.08, 0.23,
        (4.0, 27.0), (20.0, 21.0), 9, 118.0, 2.25, Some((0.0, 1.0)), None,
    );
    down_air.landing_lag_s = Some(0.24);

    // Specials deliberately teach common sword archetype motion.
    let neutral_special = committed_tail(
        strike(
            "polygon_point", "slash", 0.14, 0.08, 0.24,
            (48.0, -3.0), (28.0, 12.0), 10, 112.0, 2.15,
            Some((1.0, -0.18)), None,
        ),
        0.52,
        0.20,
    );

    let side_special = impulse(
        committed_tail(
            strike(
                "polygon_vector_lunge", "attack_side", 0.13, 0.10, 0.24,
                (41.0, -2.0), (30.0, 16.0), 9, 104.0, 2.02,
                Some((1.0, -0.28)), None,
            ),
            0.58,
            0.12,
        ),
        0.13,
        (520.0, 0.0),
        ImpulseMode::Set,
    );

    let mut rising_edge = strike(
        "polygon_rising_edge", "attack_up", 0.09, 0.12, 0.20,
        (5.0, -19.0), (22.0, 27.0), 7, 88.0, 1.65,
        Some((0.10, -1.0)), None,
    );
    rising_edge.landing_lag_s = Some(0.25);
    let up_special = impulse(rising_edge, 0.09, (0.0, -760.0), ImpulseMode::Set);

    let grounded_down_special = committed_tail(
        strike(
            "polygon_low_arc", "attack_down", 0.12, 0.09, 0.25,
            (18.0, 13.0), (34.0, 11.0), 8, 82.0, 1.55,
            Some((0.8, -0.55)), None,
        ),
        0.52,
        0.05,
    );
    let mut airborne_down_special = strike(
        "polygon_falling_edge", "air_down", 0.10, 0.10, 0.23,
        (0.0, 25.0), (21.0, 22.0), 9, 105.0, 1.95,
        Some((0.0, 1.0)), None,
    );
    airborne_down_special.landing_lag_s = Some(0.27);
    let airborne_down_special = impulse(
        airborne_down_special,
        0.10,
        (0.0, 1050.0),
        ImpulseMode::Set,
    );

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
        taunt: ambition_characters::moveset_authoring::taunt(
            "pointed_polygon_taunt",
            0.9,
        ),
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

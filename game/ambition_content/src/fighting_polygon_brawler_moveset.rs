//! Fighting Polygon — brawler archetype repertoire.
//!
//! A complete fundamentals brawler table. It mirrors the sword reference's typed
//! vocabulary but expresses every slot with close-range body mechanics: punches,
//! kicks, uppercuts, shoulder rushes, grabs, pummels, and all four throws.

use ambition_characters::moveset_authoring::{committed_tail, impulse, strike};
use ambition_characters::smash_capture::{
    author_pummel, author_standing_grab, author_throw, capture_beat, grab_shell,
    CaptureAttemptParams, CaptureCues, CapturePummelParams, CaptureThrowParams,
    SmashCaptureRepertoire,
};
use ambition_characters::smash_repertoire::{DownSpecial, NeutralSpecial, SmashRepertoire};
use ambition_platformer2d::entity_catalog::{ImpulseMode, MovesetContract};

pub fn fighting_polygon_brawler_moveset() -> MovesetContract {
    let jab = strike(
        "polygon_brawler_jab", "jab", 0.04, 0.05, 0.10,
        (22.0, -2.0), (18.0, 15.0), 4, 52.0, 1.10, Some((1.0, -0.18)), None,
    );
    let forward_tilt = strike(
        "polygon_brawler_tilt_forward", "attack_side", 0.07, 0.07, 0.15,
        (27.0, -1.0), (21.0, 16.0), 7, 82.0, 1.52, Some((1.0, -0.27)), None,
    );
    let up_tilt = strike(
        "polygon_brawler_tilt_up", "attack_up", 0.07, 0.07, 0.15,
        (8.0, -25.0), (20.0, 24.0), 6, 86.0, 1.58, Some((0.10, -1.0)), None,
    );
    let down_tilt = strike(
        "polygon_brawler_tilt_down", "attack_down", 0.06, 0.06, 0.14,
        (24.0, 12.0), (22.0, 11.0), 5, 68.0, 1.31, Some((1.0, -0.16)), None,
    );

    let mut forward_smash = strike(
        "polygon_brawler_smash_forward", "smash_forward", 0.22, 0.08, 0.29,
        (30.0, -3.0), (24.0, 20.0), 16, 162.0, 3.25, Some((1.0, -0.31)), None,
    );
    forward_smash.smash_charge_mult = 1.75;
    let mut up_smash = strike(
        "polygon_brawler_smash_up", "smash_up", 0.20, 0.08, 0.28,
        (5.0, -29.0), (23.0, 29.0), 15, 158.0, 3.15, Some((0.0, -1.0)), None,
    );
    up_smash.smash_charge_mult = 1.75;
    let mut down_smash = strike(
        "polygon_brawler_smash_down", "smash_down", 0.19, 0.09, 0.29,
        (0.0, 13.0), (34.0, 13.0), 13, 142.0, 2.82, Some((0.80, -0.60)), None,
    );
    down_smash.smash_charge_mult = 1.75;

    let neutral_air = strike(
        "polygon_brawler_air_neutral", "air_neutral", 0.05, 0.10, 0.14,
        (7.0, 0.0), (24.0, 23.0), 7, 79.0, 1.50, None, None,
    );
    let forward_air = strike(
        "polygon_brawler_air_forward", "air_forward", 0.08, 0.08, 0.17,
        (28.0, -3.0), (22.0, 18.0), 9, 108.0, 2.05, Some((1.0, -0.32)), None,
    );
    let back_air = strike(
        "polygon_brawler_air_back", "air_back", 0.09, 0.07, 0.18,
        (-26.0, -1.0), (22.0, 17.0), 10, 124.0, 2.30, Some((-1.0, -0.30)), None,
    );
    let up_air = strike(
        "polygon_brawler_air_up", "air_up", 0.06, 0.08, 0.14,
        (2.0, -27.0), (21.0, 23.0), 8, 99.0, 1.88, Some((0.0, -1.0)), None,
    );
    let mut down_air = strike(
        "polygon_brawler_air_down", "air_down", 0.11, 0.08, 0.21,
        (2.0, 25.0), (21.0, 21.0), 10, 126.0, 2.35, Some((0.0, 1.0)), None,
    );
    down_air.landing_lag_s = Some(0.25);

    let neutral_special = committed_tail(
        strike(
            "polygon_brawler_haymaker", "attack_side", 0.16, 0.08, 0.27,
            (29.0, -4.0), (24.0, 19.0), 13, 142.0, 2.75,
            Some((1.0, -0.28)), None,
        ),
        0.58,
        0.18,
    );
    let side_special = impulse(
        committed_tail(
            strike(
                "polygon_brawler_shoulderrush", "attack_side", 0.11, 0.11, 0.23,
                (24.0, -2.0), (26.0, 22.0), 11, 116.0, 2.18,
                Some((1.0, -0.23)), None,
            ),
            0.55,
            0.10,
        ),
        0.11,
        (590.0, 0.0),
        ImpulseMode::Set,
    );
    let mut uppercut = strike(
        "polygon_brawler_uppercut", "attack_up", 0.08, 0.12, 0.19,
        (5.0, -20.0), (22.0, 27.0), 9, 104.0, 1.88,
        Some((0.08, -1.0)), None,
    );
    uppercut.landing_lag_s = Some(0.25);
    let up_special = impulse(uppercut, 0.08, (0.0, -745.0), ImpulseMode::Set);

    let grounded_down_special = committed_tail(
        strike(
            "polygon_brawler_ground_slam", "attack_down", 0.13, 0.10, 0.25,
            (0.0, 15.0), (31.0, 13.0), 11, 105.0, 1.95,
            Some((0.70, -0.72)), None,
        ),
        0.54,
        0.05,
    );
    let mut airborne_down_special = strike(
        "polygon_brawler_body_drop", "air_down", 0.10, 0.11, 0.22,
        (0.0, 24.0), (24.0, 24.0), 12, 132.0, 2.42,
        Some((0.0, 1.0)), None,
    );
    airborne_down_special.landing_lag_s = Some(0.29);
    let airborne_down_special = impulse(
        airborne_down_special, 0.10, (0.0, 1080.0), ImpulseMode::Set,
    );

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
        CaptureThrowParams { damage: 8, knockback: 116.0, knockback_growth: 2.22, launch_dir: (1.0, -0.30) },
    );
    let back_throw = author_throw(
        capture_beat("polygon_brawler_throw_back", "throw_back", 0.26),
        0.12,
        CaptureThrowParams { damage: 9, knockback: 126.0, knockback_growth: 2.35, launch_dir: (-1.0, -0.27) },
    );
    let up_throw = author_throw(
        capture_beat("polygon_brawler_throw_up", "throw_up", 0.25),
        0.11,
        CaptureThrowParams { damage: 8, knockback: 120.0, knockback_growth: 2.28, launch_dir: (0.0, -1.0) },
    );
    let down_throw = author_throw(
        capture_beat("polygon_brawler_throw_down", "throw_down", 0.27),
        0.12,
        CaptureThrowParams { damage: 7, knockback: 88.0, knockback_growth: 1.82, launch_dir: (0.28, -0.96) },
    );

    SmashRepertoire {
        // ⚠ the genre shapes, deliberately. This is the unarmed half of the
        // REFERENCE pair, so its taunt and dash attack are the ones a new
        // humanoid should copy before it has a reason to differ. Five fighters
        // own a `DashAttackShape` because their own laws refused the generic
        // one; a reference rig has no such law to refuse it.
        taunt: ambition_characters::moveset_authoring::taunt(
            "fighting_polygon_brawler_taunt",
            0.9,
        ),
        dash_attack: ambition_characters::moveset_authoring::dash_attack(
            "fighting_polygon_brawler_dash_attack",
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
        down_special: DownSpecial::ByPosture { grounded: grounded_down_special, airborne: airborne_down_special },
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
    fn the_reference_brawler_answers_the_complete_typed_repertoire() {
        let moves = fighting_polygon_brawler_moveset();
        for id in [
            "polygon_brawler_jab", "polygon_brawler_tilt_forward", "polygon_brawler_tilt_up",
            "polygon_brawler_tilt_down", "polygon_brawler_smash_forward", "polygon_brawler_smash_up",
            "polygon_brawler_smash_down", "polygon_brawler_air_neutral", "polygon_brawler_air_forward",
            "polygon_brawler_air_back", "polygon_brawler_air_up", "polygon_brawler_air_down",
            "polygon_brawler_haymaker", "polygon_brawler_shoulderrush", "polygon_brawler_uppercut",
            "polygon_brawler_ground_slam", "polygon_brawler_body_drop", "polygon_brawler_grab",
            "polygon_brawler_pummel", "polygon_brawler_throw_forward", "polygon_brawler_throw_back",
            "polygon_brawler_throw_up", "polygon_brawler_throw_down",
            "fighting_polygon_brawler_taunt", "fighting_polygon_brawler_dash_attack",
        ] {
            assert!(moves.moves.iter().any(|m| m.id == id), "missing {id}");
        }
    }
}

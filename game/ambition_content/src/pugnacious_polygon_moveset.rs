//! Pugnacious Polygon — brawler archetype repertoire.
//!
//! A complete fundamentals brawler table. It mirrors the sword reference's typed
//! vocabulary but expresses every slot with close-range body mechanics: punches,
//! kicks, uppercuts, shoulder rushes, grabs, pummels, and all four throws.

use ambition_characters::moveset_authoring::Strike;
use ambition_characters::moveset_authoring::{committed_tail, impulse, strike};
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

    let neutral_special = committed_tail(
        strike(Strike {
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
        }),
        0.58,
        0.18,
    );
    let side_special = impulse(
        committed_tail(
            strike(Strike {
                id: "polygon_brawler_shoulderrush",
                clip: "attack_side",
                startup_s: 0.11,
                active_s: 0.11,
                recover_s: 0.23,
                offset: (24.0, -2.0),
                half_extents: (26.0, 22.0),
                damage: 11,
                knockback: 116.0,
                knockback_growth: 2.18,
                launch_dir: Some((1.0, -0.23)),
                on_hit: None,
            }),
            0.55,
            0.10,
        ),
        0.11,
        (590.0, 0.0),
        ImpulseMode::Set,
    );
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

    SmashRepertoire {
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
    .into_contract()
}

#[cfg(test)]
mod tests {
    use super::*;

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
            "polygon_brawler_shoulderrush",
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

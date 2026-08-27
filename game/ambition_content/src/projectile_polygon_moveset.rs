//! Projectile Polygon — ranged beast-biped fundamentals repertoire.
//!
//! A complete ranged-fundamentals table for the non-humanoid member of the
//! polygon reference trio. Neutral special is a real projectile release from a
//! head-mounted cannon; the rest of the kit stays readable so the move library
//! remains useful as a bestial pose reference rather than a one-off gimmick.

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
use ambition_platformer2d::entity_catalog::{
    ChargeGesture, ClipBinding, ImpulseMode, MoveEvent, MoveEventKind, MoveSpec, MoveWindow,
    MovesetContract, SmashChargeSpec, WindowTag,
};

/// How long the fill row runs, in seconds: 14 frames at 62ms, from
/// `polygon_charge_shot`'s own authoring.
///
/// ⭐ THE ART AND THE MECHANIC AGREE BY CONSTRUCTION. The VFX row was drawn as
/// one long climb rather than a loop precisely so a player could read "nearly
/// there" off the ring count without a meter, and that only works if the fill
/// finishes exactly when the charge does.
const CHARGE_FILL_S: f32 = 14.0 * 0.062;

/// Where in the windup he latches. Early, so the charge reads as "the shot
/// started and stopped" rather than "the shot is about to leave and stopped" —
/// the same rule the smash charge pose states.
const CHARGE_HOLD_AT_S: f32 = 0.10;

/// Where the charge is drawn, body-local (`+x` toward facing, `+y` gravity-down).
///
/// ⛔ THIS IS THE SPAWN POINT, NOT THE CANNON. See [`charge_shot`]'s note: the
/// shared fire site launches from the body centre plus `(0, -8)`, and an effect
/// that disagreed with it would show the ball jumping across his body on
/// release.
const MUZZLE: (f32, f32) = (0.0, -8.0);

/// When the shot leaves, measured from the move's start. Everything before it is
/// windup that plays out on release.
const CHARGE_FIRE_AT_S: f32 = 0.26;

/// THE CHARGE BALL — the genre's held neutral-B, and this fighter's identity.
///
/// Hold Special and the timeline freezes at [`CHARGE_HOLD_AT_S`] while the ball
/// builds at the muzzle; let go (or hit the maximum, which fires on its own —
/// a full charge is LOADED, not stored) and the rest of the windup plays into
/// the shot. What comes out is scaled by how long it was held: see
/// `crate::authored::projectile_polygon`'s `charged_cannon`.
///
/// ⭐ AND IT STORES. See `stores` on the policy below: a full charge WAITS
/// rather than firing itself, and an interrupted one is banked for the next
/// press. That is Samus/Mewtwo parity and it is what makes the ball a plan.
///
/// ⛔ NO `smash_charge_mult`. That number scales the damage of a melee volume
/// this move does not have — the payoff is entirely in the projectile — so
/// setting one here would be a multiplier that multiplies nothing. The
/// explicitly authored `smash_charge` is what says this move holds.
///
/// ⚠ THE BALL BUILDS WHERE THE SHOT LEAVES, which is not where his cannon is.
/// `spawn_projectiles_from_brain_actions` launches every ordinary shot from the
/// body centre plus a shared `(0, -8)`, and that offset belongs to the engine
/// rather than to this fighter. Drawing the charge at the head-mounted cannon
/// and then firing from the chest would read as the ball teleporting, so the
/// effect matches the spawn instead. A per-action MUZZLE offset would fix both
/// ends at once; it does not exist, and inventing one here would be a fighter
/// reaching into the shared fire site.
///
/// ⚠ THE FILL IS FIRED ONCE, AT THE LATCH, and that is a real limitation rather
/// than a design. A move event is a point on a timeline and a charge is a
/// stretch of one, so nothing re-fires the VFX if the hold outlasts the row;
/// what makes it read anyway is that the row was drawn to the same length as
/// the hold. A hold that could outlast its own fill would want a sustained
/// presentation channel, which does not exist yet.
fn charge_shot() -> MoveSpec {
    MoveSpec {
        id: "polygon_projectile_charge_shot".to_string(),
        display_name: Some("Charge Shot".to_string()),
        clip: ClipBinding {
            clip: "shoot".to_string(),
            fallbacks: vec!["attack_side".to_string(), "idle".to_string()],
        },
        duration_s: 0.58,
        windows: vec![
            MoveWindow {
                start_s: 0.0,
                end_s: CHARGE_FIRE_AT_S,
                tag: WindowTag::Startup,
                volumes: vec![],
                sustain_effect: None,
                motion_scale: 1.0,
            },
            // No Active volume: the projectile IS the damage, as it is for every
            // ranged move. The recovery is the settle he owes for committing.
            MoveWindow {
                start_s: CHARGE_FIRE_AT_S,
                end_s: 0.58,
                tag: WindowTag::Recovery,
                volumes: vec![],
                sustain_effect: None,
                motion_scale: 1.0,
            },
        ],
        events: vec![
            // The intake first: loose energy pulled toward the muzzle, so the
            // player knows the button took before the ball is visible.
            MoveEvent {
                at_s: 0.01,
                kind: MoveEventKind::Vfx {
                    effect: "charge_intake".to_string(),
                    at: MUZZLE,
                    scale: 1.0,
                    sfx: None,
                },
            },
            MoveEvent {
                at_s: CHARGE_HOLD_AT_S * 0.5,
                kind: MoveEventKind::Vfx {
                    effect: "charge_build".to_string(),
                    at: MUZZLE,
                    scale: 1.0,
                    sfx: None,
                },
            },
            MoveEvent {
                at_s: CHARGE_FIRE_AT_S,
                kind: MoveEventKind::Vfx {
                    effect: "charge_release".to_string(),
                    at: MUZZLE,
                    scale: 1.0,
                    sfx: None,
                },
            },
            MoveEvent {
                at_s: CHARGE_FIRE_AT_S,
                kind: MoveEventKind::Ranged,
            },
        ],
        gates: Default::default(),
        start_impulse: None,
        smash_charge_mult: 1.0,
        smash_charge: Some(SmashChargeSpec {
            hold_at_s: CHARGE_HOLD_AT_S,
            max_hold_s: CHARGE_FILL_S,
            // ⭐⭐ IT STORES, AND THAT IS THE OTHER HALF OF THE CHARACTER. Jon,
            // 2026-08-27: *"This should have parity with samus / mewtwo 'b', so
            // that means it needs to be able to store a charge and fire at
            // different sizes."* Firing at different sizes was already here —
            // `charged_cannon`'s `RangedCharge` ladder and the sheet's five
            // tiers. Storing was not, and without it the only way to reach a
            // full ball was to stand still for the whole fill with nobody
            // touching you, which on a platform-fighter stage is never.
            //
            // ⛔ AND IT REPLACES A COMMENT THAT SAID THE OPPOSITE. The doc above
            // read *"a full charge is LOADED, not stored"*, which was true of the
            // engine and wrong for this character: at maximum the shot now
            // WAITS, and getting hit banks it instead of wasting it.
            stores: true,
        }),
        charge_gesture: ChargeGesture::Special,
        repeat: None,
        landing_lag_s: None,
        autocancel_after_s: None,
        sprite_spin_hz: None,
        equips: None,
    }
}

pub fn projectile_polygon_moveset() -> MovesetContract {
    let jab = strike(Strike {
        id: "polygon_projectile_jab",
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
        id: "polygon_projectile_tilt_forward",
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
        id: "polygon_projectile_tilt_up",
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
        id: "polygon_projectile_tilt_down",
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
        id: "polygon_projectile_smash_forward",
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
        id: "polygon_projectile_smash_up",
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
        id: "polygon_projectile_smash_down",
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
        id: "polygon_projectile_air_neutral",
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
        id: "polygon_projectile_air_forward",
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
        id: "polygon_projectile_air_back",
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
        id: "polygon_projectile_air_up",
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
        id: "polygon_projectile_air_down",
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

    let neutral_special = charge_shot();

    let side_special = impulse(
        committed_tail(
            strike(Strike {
                id: "polygon_projectile_vector_rush",
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
        id: "polygon_projectile_recoil_lift",
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
            id: "polygon_projectile_low_burst",
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
        id: "polygon_projectile_downward_vector",
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
        grab_shell("polygon_projectile_grab", "grab", 0.06, 0.05, 0.21),
        CaptureAttemptParams {
            offset: (15.0, 1.0),
            half_extents: (19.0, 16.0),
            hold_offset: (14.0, 3.0),
        },
    );
    let pummel = author_pummel(
        capture_beat("polygon_projectile_pummel", "pummel", 0.15),
        0.06,
        CapturePummelParams { damage: 4 },
    );
    let forward_throw = author_throw(
        capture_beat("polygon_projectile_throw_forward", "throw_forward", 0.24),
        0.11,
        CaptureThrowParams {
            damage: 8,
            knockback: 116.0,
            knockback_growth: 2.22,
            launch_dir: (1.0, -0.30),
        },
    );
    let back_throw = author_throw(
        capture_beat("polygon_projectile_throw_back", "throw_back", 0.26),
        0.12,
        CaptureThrowParams {
            damage: 9,
            knockback: 126.0,
            knockback_growth: 2.35,
            launch_dir: (-1.0, -0.27),
        },
    );
    let up_throw = author_throw(
        capture_beat("polygon_projectile_throw_up", "throw_up", 0.25),
        0.11,
        CaptureThrowParams {
            damage: 8,
            knockback: 120.0,
            knockback_growth: 2.28,
            launch_dir: (0.0, -1.0),
        },
    );
    let down_throw = author_throw(
        capture_beat("polygon_projectile_throw_down", "throw_down", 0.27),
        0.12,
        CaptureThrowParams {
            damage: 7,
            knockback: 88.0,
            knockback_growth: 1.82,
            launch_dir: (0.28, -0.96),
        },
    );

    SmashRepertoire {
        // The genre shapes are deliberate: this is still a reusable reference fighter.
        // Projectile identity belongs to the head cannon and shoot pose, not
        // to making every grounded movement action species-specific.
        taunt: ambition_characters::moveset_authoring::taunt("projectile_polygon_taunt", 0.9),
        dash_attack: ambition_characters::moveset_authoring::dash_attack(
            "projectile_polygon_dash_attack",
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
    fn the_reference_projectile_fighter_answers_the_complete_typed_repertoire() {
        let moves = projectile_polygon_moveset();
        for id in [
            "polygon_projectile_jab",
            "polygon_projectile_tilt_forward",
            "polygon_projectile_tilt_up",
            "polygon_projectile_tilt_down",
            "polygon_projectile_smash_forward",
            "polygon_projectile_smash_up",
            "polygon_projectile_smash_down",
            "polygon_projectile_air_neutral",
            "polygon_projectile_air_forward",
            "polygon_projectile_air_back",
            "polygon_projectile_air_up",
            "polygon_projectile_air_down",
            // ⭐ THE CHARGE SHOT REPLACED THE PLAIN ONE. This row still named
            // `polygon_projectile_shot` after `charge_shot()` became the neutral
            // special, and the id exists nowhere else in the tree — the list was
            // the last reference to a move that had been renamed out.
            "polygon_projectile_charge_shot",
            "polygon_projectile_vector_rush",
            "polygon_projectile_recoil_lift",
            "polygon_projectile_low_burst",
            "polygon_projectile_downward_vector",
            "polygon_projectile_grab",
            "polygon_projectile_pummel",
            "polygon_projectile_throw_forward",
            "polygon_projectile_throw_back",
            "polygon_projectile_throw_up",
            "polygon_projectile_throw_down",
            "projectile_polygon_taunt",
            "projectile_polygon_dash_attack",
        ] {
            assert!(moves.moves.iter().any(|m| m.id == id), "missing {id}");
        }
    }
}

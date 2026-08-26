//! Sanic's repertoire, for the stage he visits rather than the one he lives
//! on.
//!
//! A move table is *what the attack IS*; the ability is *whether this body may attack at all*. At
//! home the answer is no and these sixteen moves are unreachable; on a stage that GRANTS the verb
//! (`MatchAbilities::levelled`) they are what he swings.
//!
//! and it is not his spin dash. `declare_sanic_techniques` puts spin dash
//! and the transform on his body as TECHNIQUES — named actions his own game
//! resolves Attack and Utility onto — and they stay exactly where they are. The
//! side special below is a different object that happens to look like one, which
//! is the honest way to give a crossover stage a signature move without two
//! authorities owning it.
//!
//! ## The character
//!
//! Speed, and the cost of it. The fastest startups on the grid after the Shadow
//! Oni's, the least commitment of anybody — almost nothing here has a locked
//! tail — and the weakest single hit in the game. He does not win an exchange;
//! he has three before you finish one.

use ambition_platformer2d::characters::moveset_authoring::Strike;
use ambition_platformer2d::characters::moveset_authoring::{
    committed_tail, impulse, on_contact, sfx, strike, vfx_at,
};
use ambition_platformer2d::characters::smash_capture::{
    author_pummel, author_standing_grab, author_throw, capture_beat, grab_shell,
    CaptureAttemptParams, CaptureCues, CapturePummelParams, CaptureThrowParams,
    SmashCaptureRepertoire,
};
use ambition_platformer2d::characters::smash_repertoire::{
    DownSpecial, NeutralSpecial, SmashRepertoire, UpSpecial,
};
use ambition_platformer2d::entity_catalog::{ImpulseMode, MovesetContract};

const RUSH_FX: f32 = 0.85;
const BOOM_FX: f32 = 1.2;

/// See the module doc. Sixteen presses.
pub fn sanic_moveset() -> MovesetContract {
    // JAB — `quick_jab`. Four frames. It is not meant to do anything except
    // arrive before yours.
    let jab = strike(Strike {
        id: "quick_jab",
        clip: "jab",
        startup_s: 0.04,
        active_s: 0.04,
        recover_s: 0.11,
        offset: (22.0, 0.0),
        half_extents: (16.0, 12.0),
        damage: 2,
        knockback: 42.0,
        knockback_growth: 1.02,
        launch_dir: None,
        on_hit: None,
    });
    let jab = vfx_at(jab, 0.04, "air_slice", (22.0, 0.0), RUSH_FX);
    let jab = on_contact(jab, "player.hit");

    // FORWARD TILT — `run_up_kick`. He is already moving; this is what that
    // looks like when it hits something.
    let f_tilt = strike(Strike {
        id: "run_up_kick",
        clip: "attack_side",
        startup_s: 0.06,
        active_s: 0.06,
        recover_s: 0.14,
        offset: (28.0, 2.0),
        half_extents: (20.0, 13.0),
        damage: 5,
        knockback: 62.0,
        knockback_growth: 1.22,
        launch_dir: Some((1.0, -0.26)),
        on_hit: None,
    });
    let f_tilt = vfx_at(f_tilt, 0.06, "dash_streak", (28.0, 2.0), RUSH_FX);
    let f_tilt = on_contact(f_tilt, "player.hit");

    // UP TILT — `heel_flick`. Up and behind, off the run.
    let u_tilt = strike(Strike {
        id: "heel_flick",
        clip: "attack_up",
        startup_s: 0.06,
        active_s: 0.06,
        recover_s: 0.15,
        offset: (4.0, -24.0),
        half_extents: (16.0, 20.0),
        damage: 4,
        knockback: 68.0,
        knockback_growth: 1.28,
        launch_dir: Some((0.10, -1.0)),
        on_hit: None,
    });
    let u_tilt = vfx_at(u_tilt, 0.06, "wind_curl", (4.0, -24.0), RUSH_FX);
    let u_tilt = on_contact(u_tilt, "player.hit");

    // DOWN TILT — `skid`. He stops, briefly, and the ground does not.
    let d_tilt = strike(Strike {
        id: "skid",
        clip: "attack_down",
        startup_s: 0.05,
        active_s: 0.06,
        recover_s: 0.14,
        offset: (26.0, 14.0),
        half_extents: (22.0, 9.0),
        damage: 4,
        knockback: 50.0,
        knockback_growth: 1.12,
        launch_dir: Some((0.95, -0.26)),
        on_hit: None,
    });
    let d_tilt = vfx_at(d_tilt, 0.05, "skid_puff", (26.0, 14.0), RUSH_FX);
    let d_tilt = on_contact(d_tilt, "player.hit");

    // FORWARD SMASH — `sonic_boom`. The one moment he stops being quick and
    // becomes hard. still the second-weakest forward smash on the grid.
    let f_smash = strike(Strike {
        id: "sonic_boom",
        clip: "smash_forward",
        startup_s: 0.15,
        active_s: 0.08,
        recover_s: 0.28,
        offset: (34.0, 0.0),
        half_extents: (26.0, 18.0),
        damage: 11,
        knockback: 118.0,
        knockback_growth: 2.15,
        launch_dir: Some((0.95, -0.40)),
        on_hit: None,
    });
    let f_smash = vfx_at(f_smash, 0.15, "sonic_boom", (34.0, 0.0), BOOM_FX);
    let f_smash = sfx(f_smash, 0.15, "player.attack.charge");
    let f_smash = on_contact(f_smash, "player.hit");

    // UP SMASH — `updraft`. A vertical burst off a standing start.
    let u_smash = strike(Strike {
        id: "updraft",
        clip: "smash_up",
        startup_s: 0.14,
        active_s: 0.09,
        recover_s: 0.26,
        offset: (2.0, -30.0),
        half_extents: (20.0, 30.0),
        damage: 10,
        knockback: 112.0,
        knockback_growth: 2.12,
        launch_dir: Some((0.08, -1.0)),
        on_hit: None,
    });
    let u_smash = vfx_at(u_smash, 0.14, "wind_curl", (2.0, -30.0), BOOM_FX);
    let u_smash = on_contact(u_smash, "player.hit");

    // DOWN SMASH — `split_kick`. Both directions at once, low.
    let d_smash = strike(Strike {
        id: "split_kick",
        clip: "smash_down",
        startup_s: 0.15,
        active_s: 0.08,
        recover_s: 0.28,
        offset: (0.0, 18.0),
        half_extents: (36.0, 11.0),
        damage: 10,
        knockback: 104.0,
        knockback_growth: 1.95,
        launch_dir: Some((0.85, -0.52)),
        on_hit: None,
    });
    let d_smash = vfx_at(d_smash, 0.15, "sonic_ripple", (0.0, 18.0), BOOM_FX);
    let d_smash = on_contact(d_smash, "player.hit");

    // NEUTRAL AIR — `air_spin`. The ball, in the air, around him.
    let n_air = strike(Strike {
        id: "air_spin",
        clip: "air_neutral",
        startup_s: 0.05,
        active_s: 0.11,
        recover_s: 0.14,
        offset: (0.0, 0.0),
        half_extents: (24.0, 22.0),
        damage: 5,
        knockback: 64.0,
        knockback_growth: 1.34,
        launch_dir: Some((0.55, -0.72)),
        on_hit: None,
    });
    let n_air = vfx_at(n_air, 0.05, "sonic_ripple", (0.0, 0.0), RUSH_FX);
    let n_air = on_contact(n_air, "player.hit");

    // FORWARD AIR — `homing_cut`. He arrives at you rather than swinging at
    // where you are.
    let f_air = strike(Strike {
        id: "homing_cut",
        clip: "air_forward",
        startup_s: 0.07,
        active_s: 0.07,
        recover_s: 0.16,
        offset: (28.0, 0.0),
        half_extents: (21.0, 17.0),
        damage: 7,
        knockback: 86.0,
        knockback_growth: 1.65,
        launch_dir: Some((0.95, -0.42)),
        on_hit: None,
    });
    let f_air = vfx_at(f_air, 0.07, "dash_streak", (28.0, 0.0), RUSH_FX);
    let f_air = on_contact(f_air, "player.hit");

    // BACK AIR — `trailing_heel`. What is behind him is behind him because
    // he already passed it.
    let b_air = strike(Strike {
        id: "trailing_heel",
        clip: "air_back",
        startup_s: 0.08,
        active_s: 0.06,
        recover_s: 0.18,
        offset: (-28.0, 0.0),
        half_extents: (21.0, 16.0),
        damage: 8,
        knockback: 94.0,
        knockback_growth: 1.78,
        launch_dir: Some((-0.95, -0.36)),
        on_hit: None,
    });
    let b_air = vfx_at(b_air, 0.08, "air_slice", (-28.0, 0.0), RUSH_FX);
    let b_air = on_contact(b_air, "player.hit");

    // UP AIR — `corkscrew`. Straight up, spinning.
    let u_air = strike(Strike {
        id: "corkscrew",
        clip: "air_up",
        startup_s: 0.06,
        active_s: 0.08,
        recover_s: 0.15,
        offset: (2.0, -26.0),
        half_extents: (18.0, 22.0),
        damage: 6,
        knockback: 80.0,
        knockback_growth: 1.58,
        launch_dir: Some((0.08, -1.0)),
        on_hit: None,
    });
    let u_air = vfx_at(u_air, 0.06, "wind_curl", (2.0, -26.0), RUSH_FX);
    let u_air = on_contact(u_air, "player.hit");

    // DOWN AIR — `drill_dive`. Straight down, and he keeps going.
    let d_air = strike(Strike {
        id: "drill_dive",
        clip: "air_down",
        startup_s: 0.09,
        active_s: 0.09,
        recover_s: 0.19,
        offset: (0.0, 25.0),
        half_extents: (19.0, 20.0),
        damage: 9,
        knockback: 104.0,
        knockback_growth: 1.92,
        launch_dir: Some((0.0, 1.0)),
        on_hit: None,
    });
    let d_air = vfx_at(d_air, 0.09, "sonic_ripple", (0.0, 25.0), RUSH_FX);
    let d_air = on_contact(d_air, "player.hit");

    // NEUTRAL — `spin_charge`. He winds up in place and the release is the
    // only real commitment in his kit.
    let n_b = strike(Strike {
        id: "spin_charge",
        clip: "attack",
        startup_s: 0.18,
        active_s: 0.10,
        recover_s: 0.28,
        offset: (26.0, 6.0),
        half_extents: (24.0, 16.0),
        damage: 10,
        knockback: 100.0,
        knockback_growth: 1.80,
        launch_dir: Some((0.9, -0.40)),
        on_hit: None,
    });
    // either posture, and it has to be: gated to the ground, an airborne
    // neutral-B walked the chain past it and found NOTHING — the last candidate
    // for `special_air` is `special` itself. A spin charge in the air is a spin
    // charge; there is nothing about it that needs a floor.
    let n_b = committed_tail(n_b, 0.60, 0.05);
    let n_b = vfx_at(n_b, 0.04, "charge_pulse", (0.0, 8.0), RUSH_FX);
    let n_b = sfx(n_b, 0.04, "player.attack.charge");
    let n_b = vfx_at(n_b, 0.18, "sonic_ripple", (26.0, 6.0), BOOM_FX);
    let n_b = on_contact(n_b, "player.hit");

    // SIDE — `blue_streak`. The fastest crossing on the grid, and the tail
    // barely damps: he can still steer out of it, which is the whole difference
    // between him and the engineer's piston.
    let side_b = strike(Strike {
        id: "blue_streak",
        clip: "attack_side",
        startup_s: 0.09,
        active_s: 0.10,
        recover_s: 0.20,
        offset: (28.0, 2.0),
        half_extents: (24.0, 18.0),
        damage: 9,
        knockback: 98.0,
        knockback_growth: 1.82,
        launch_dir: Some((0.95, -0.32)),
        on_hit: None,
    });
    let side_b = impulse(side_b, 0.09, (860.0, 0.0), ImpulseMode::Set);
    let side_b = committed_tail(side_b, 0.46, 0.45);
    let side_b = vfx_at(side_b, 0.09, "dash_streak", (0.0, 0.0), BOOM_FX);
    let side_b = sfx(side_b, 0.09, "player.dash");
    let side_b = on_contact(side_b, "player.hit");

    // UP — `spring_launch`. THE RECOVERY. The springs his own course is full
    // of, carried. The highest lift on the grid and the cheapest landing, which
    // is what the lightest, fastest fighter's way home should be.
    let mut up_b = strike(Strike {
        id: "spring_launch",
        clip: "attack_up",
        startup_s: 0.05,
        active_s: 0.12,
        recover_s: 0.15,
        offset: (0.0, -12.0),
        half_extents: (18.0, 30.0),
        damage: 6,
        knockback: 78.0,
        knockback_growth: 1.55,
        launch_dir: Some((0.10, -1.0)),
        on_hit: None,
    });
    up_b.landing_lag_s = Some(0.18);
    let up_b = impulse(up_b, 0.05, (0.0, -880.0), ImpulseMode::Set);
    let up_b = committed_tail(up_b, 0.42, 0.40);
    let up_b = vfx_at(up_b, 0.05, "release_ring", (0.0, 16.0), RUSH_FX);
    let up_b = sfx(up_b, 0.05, "player.double_jump");
    let up_b = on_contact(up_b, "player.hit");

    // DOWN — `ball_drop`. He curls and drops. Airborne-only: on the ground
    // it would be a worse down smash, and the move is the fall.
    let mut down_b = strike(Strike {
        id: "ball_drop",
        clip: "air_down",
        startup_s: 0.07,
        active_s: 0.14,
        recover_s: 0.20,
        offset: (0.0, 24.0),
        half_extents: (20.0, 22.0),
        damage: 10,
        knockback: 108.0,
        knockback_growth: 1.95,
        launch_dir: Some((0.0, 1.0)),
        on_hit: None,
    });
    down_b.landing_lag_s = Some(0.26);
    let down_b = impulse(down_b, 0.07, (0.0, 1300.0), ImpulseMode::Set);
    let down_b = vfx_at(down_b, 0.07, "sonic_ripple", (0.0, 12.0), RUSH_FX);
    let down_b = on_contact(down_b, "player.hit");

    // effect on ground. Think of bowser down b. In the air he just does a
    // downward slam, but on the ground, it causes him to jump in an arc and then
    // slam. Specials can have different effects in different contexts that
    // should be ok, and makes for a richer smash game, although in most cases
    // they shouldn't be context dependent."*
    let ground_down_b = strike(Strike {
        id: "ball_hop",
        clip: "attack_down",
        startup_s: 0.18,
        active_s: 0.14,
        recover_s: 0.24,
        offset: (0.0, 22.0),
        half_extents: (20.0, 22.0),
        damage: 10,
        knockback: 108.0,
        knockback_growth: 1.95,
        launch_dir: Some((0.0, 1.0)),
        on_hit: None,
    });
    // `Add` for the hop, `Set` for the slam — the same split George's
    // grounded down-B needs and for the same reason: `lift_speed` is derived
    // from `Set` impulses, so a hop written that way would advertise this move
    // to the recovery policy as a way home. `spring_launch` is his way home.
    let ground_down_b = impulse(ground_down_b, 0.04, (180.0, -560.0), ImpulseMode::Add);
    let ground_down_b = impulse(ground_down_b, 0.18, (0.0, 1300.0), ImpulseMode::Set);
    let ground_down_b = committed_tail(ground_down_b, 0.62, 0.10);
    let ground_down_b = vfx_at(ground_down_b, 0.04, "skid_puff", (0.0, 18.0), RUSH_FX);
    let ground_down_b = sfx(ground_down_b, 0.04, "player.double_jump");
    let ground_down_b = vfx_at(ground_down_b, 0.18, "sonic_ripple", (0.0, 16.0), RUSH_FX);
    let ground_down_b = on_contact(ground_down_b, "player.hit");

    // SANIC'S CAPTURE KIT. Fastest startup, shortest reach, weakest throw and the
    // longest recovery. He gets there first and cannot do much with it, which is the
    // joke and also the balance.
    // the grab draws `attack`, not `grab`: these sheets publish no `grab` row,
    // and each table's own `every_clip_names_a_row_..._sheet_carries` guard says
    // so. `ClipBinding`'s fallbacks would have covered it at runtime, but a move
    // that NAMES a row nobody publishes is a lie the guard is right to refuse.
    let grab = author_standing_grab(
        grab_shell("sanic_grab", "attack", 0.05, 0.04, 0.24),
        CaptureAttemptParams {
            offset: (12.0, 1.0),
            half_extents: (17.0, 14.0),
            hold_offset: (13.0, 3.0),
        },
    );
    let pummel = author_pummel(
        capture_beat("sanic_pummel", "attack", 0.12),
        0.05,
        CapturePummelParams { damage: 2 },
    );
    let forward_throw = author_throw(
        capture_beat("sanic_fthrow", "attack", 0.22),
        0.11,
        CaptureThrowParams {
            damage: 6,
            knockback: 98.0,
            knockback_growth: 2.3,
            launch_dir: (1.0, -0.3),
        },
    );

    let back_throw = author_throw(
        capture_beat("sanic_bthrow", "attack", 0.24),
        0.12,
        CaptureThrowParams {
            damage: 7,
            knockback: 105.84,
            knockback_growth: 2.42,
            launch_dir: (-1.0, -0.25),
        },
    );

    let up_throw = author_throw(
        capture_beat("sanic_uthrow", "attack", 0.23),
        0.11,
        CaptureThrowParams {
            damage: 6,
            knockback: 101.92,
            knockback_growth: 2.35,
            launch_dir: (0.0, -1.0),
        },
    );

    let down_throw = author_throw(
        capture_beat("sanic_dthrow", "attack", 0.25),
        0.12,
        CaptureThrowParams {
            damage: 4,
            knockback: 72.52,
            knockback_growth: 1.84,
            launch_dir: (0.4, -0.92),
        },
    );

    SmashRepertoire {
        taunt: ambition_platformer2d::characters::moveset_authoring::taunt("sanic_taunt", 0.9),

        dash_attack: ambition_platformer2d::characters::moveset_authoring::dash_attack(
            "sanic_dash_attack",
            ambition_platformer2d::characters::moveset_authoring::DashAttackShape::GENRE,
            7,
            77.5,
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
        // AUTHORED, at the rule that every fighter in the smash roster have a grab. The
        // transitional `None` is gone: capture was proven on George and the Pirate Admiral, and
        // the point of proving it was to stop being the only two.
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
            grounded: ground_down_b,
            airborne: down_b,
        },
    }
    .into_contract()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d::entity_catalog::WindowTag;

    // Fourteen fighters each carried a copy of it: every bound verb names a move
    // this table defines, and the table binds the whole vocabulary. Both are now
    // unwritable defects rather than tested ones. `SmashRepertoire` owns the verb
    // strings, so there is no string in this file to misspell; it is a struct
    // with no `Default` and no private fields, so a missing or renamed slot is a
    // COMPILE error here. What the fourteen copies stood for — that every press
    // is answered, in every posture it is asked in — is checked once, by
    // `ambition_characters::smash_repertoire`, and by the host ratchet
    // `smash_roster_movesets::report_the_smash_kit_every_selectable_fighter_has`.

    /// SPEED IS THE WHOLE CHARACTER, and it is checkable. Every one of his
    /// grounded normals starts in under seven frames at 60Hz. A retune that made
    /// him ordinary would pass every other test in this file.
    #[test]
    fn his_normals_come_out_faster_than_a_tenth_of_a_second() {
        let moveset = sanic_moveset();
        for id in ["quick_jab", "run_up_kick", "heel_flick", "skid"] {
            let startup = moveset
                .move_by_id(id)
                .unwrap_or_else(|| panic!("{id} exists"))
                .windows
                .iter()
                .find(|w| matches!(w.tag, WindowTag::Active))
                .expect("a strike has an active window")
                .start_s;
            assert!(
                startup <= 0.06,
                "`{id}` starts in {startup}s, which is not a fast character"
            );
        }
    }
}

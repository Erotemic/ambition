//! Mary-O's platform-fighter repertoire.
//!
//! Her home character catalog grants `RunJump`, which excludes attack; the
//! moveset therefore does not grant combat capability by itself. A host that
//! grants attack can use this light, quick kit with strong down-air pressure.

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

const STEP_FX: f32 = 0.8;
const FLAME_FX: f32 = 1.1;

/// Build Mary-O's platform-fighter moveset.
pub fn mary_o_moveset() -> MovesetContract {
    // JAB — `hop_kick`. A quick boot. She does not have a punch.
    let jab = strike(Strike {
        id: "hop_kick",
        clip: "jab",
        startup_s: 0.05,
        active_s: 0.05,
        recover_s: 0.13,
        offset: (22.0, 2.0),
        half_extents: (16.0, 12.0),
        damage: 3,
        knockback: 46.0,
        knockback_growth: 1.05,
        launch_dir: None,
        on_hit: None,
    });
    let jab = vfx_at(jab, 0.05, "poof_small", (22.0, 2.0), STEP_FX);
    let jab = on_contact(jab, "player.hit");

    // FORWARD TILT — `sweep_kick`. Low and out, at the height a goomba is.
    let f_tilt = strike(Strike {
        id: "sweep_kick",
        clip: "attack_side",
        startup_s: 0.07,
        active_s: 0.07,
        recover_s: 0.16,
        offset: (28.0, 6.0),
        half_extents: (20.0, 13.0),
        damage: 5,
        knockback: 66.0,
        knockback_growth: 1.24,
        launch_dir: Some((1.0, -0.30)),
        on_hit: None,
    });
    let f_tilt = vfx_at(f_tilt, 0.07, "skid_puff", (28.0, 6.0), STEP_FX);
    let f_tilt = on_contact(f_tilt, "player.hit");

    // UP TILT — `header`. The brick-breaking motion, aimed at a person.
    let u_tilt = strike(Strike {
        id: "header",
        clip: "attack_up",
        startup_s: 0.07,
        active_s: 0.07,
        recover_s: 0.16,
        offset: (4.0, -26.0),
        half_extents: (16.0, 20.0),
        damage: 5,
        knockback: 72.0,
        knockback_growth: 1.30,
        launch_dir: Some((0.10, -1.0)),
        on_hit: None,
    });
    let u_tilt = vfx_at(u_tilt, 0.07, "four_point_glint", (4.0, -26.0), STEP_FX);
    let u_tilt = on_contact(u_tilt, "player.hit");

    // DOWN TILT — `slide`. The crouch-slide, which in her game is how you
    // get under things.
    let d_tilt = strike(Strike {
        id: "slide",
        clip: "attack_down",
        startup_s: 0.06,
        active_s: 0.07,
        recover_s: 0.16,
        offset: (26.0, 14.0),
        half_extents: (22.0, 9.0),
        damage: 4,
        knockback: 52.0,
        knockback_growth: 1.14,
        launch_dir: Some((0.95, -0.28)),
        on_hit: None,
    });
    let d_tilt = vfx_at(d_tilt, 0.06, "skid_puff", (26.0, 14.0), STEP_FX);
    let d_tilt = on_contact(d_tilt, "player.hit");

    // FORWARD SMASH — `shell_kick`. She kicks something that is not there
    // and it goes anyway. the weakest forward smash on the grid: she is the
    // lightest fighter on it and the numbers say so rather than a comment.
    let f_smash = strike(Strike {
        id: "shell_kick",
        clip: "smash_forward",
        startup_s: 0.16,
        active_s: 0.09,
        recover_s: 0.28,
        offset: (34.0, 2.0),
        half_extents: (26.0, 18.0),
        damage: 11,
        knockback: 116.0,
        knockback_growth: 2.10,
        launch_dir: Some((0.95, -0.40)),
        on_hit: None,
    });
    let f_smash = vfx_at(f_smash, 0.16, "starburst", (34.0, 2.0), FLAME_FX);
    let f_smash = sfx(f_smash, 0.16, "player.attack.charge");
    let f_smash = on_contact(f_smash, "player.hit");

    // UP SMASH — `block_punch`. Straight up into where the block would be.
    let u_smash = strike(Strike {
        id: "block_punch",
        clip: "smash_up",
        startup_s: 0.15,
        active_s: 0.10,
        recover_s: 0.27,
        offset: (2.0, -32.0),
        half_extents: (20.0, 30.0),
        damage: 11,
        knockback: 114.0,
        knockback_growth: 2.15,
        launch_dir: Some((0.08, -1.0)),
        on_hit: None,
    });
    let u_smash = vfx_at(u_smash, 0.15, "starburst", (2.0, -32.0), STEP_FX);
    let u_smash = on_contact(u_smash, "player.hit");

    // DOWN SMASH — `ground_pound`. Both feet, once, hard.
    let d_smash = strike(Strike {
        id: "ground_pound",
        clip: "smash_down",
        startup_s: 0.17,
        active_s: 0.09,
        recover_s: 0.30,
        offset: (0.0, 20.0),
        half_extents: (36.0, 12.0),
        damage: 10,
        knockback: 108.0,
        knockback_growth: 1.95,
        launch_dir: Some((0.8, -0.55)),
        on_hit: None,
    });
    let d_smash = vfx_at(d_smash, 0.17, "shockwave", (0.0, 20.0), FLAME_FX);
    let d_smash = vfx_at(d_smash, 0.17, "landing_puff", (0.0, 22.0), STEP_FX);
    let d_smash = on_contact(d_smash, "player.hit");

    // NEUTRAL AIR — `tumble`. The somersault her jump already does.
    let n_air = strike(Strike {
        id: "tumble",
        clip: "air_neutral",
        startup_s: 0.06,
        active_s: 0.11,
        recover_s: 0.15,
        offset: (0.0, 0.0),
        half_extents: (24.0, 22.0),
        damage: 6,
        knockback: 66.0,
        knockback_growth: 1.36,
        launch_dir: Some((0.55, -0.75)),
        on_hit: None,
    });
    let n_air = vfx_at(n_air, 0.06, "wind_curl", (0.0, 0.0), STEP_FX);
    let n_air = on_contact(n_air, "player.hit");

    // FORWARD AIR — `drop_kick`. Both feet, forward and down.
    let f_air = strike(Strike {
        id: "drop_kick",
        clip: "air_forward",
        startup_s: 0.08,
        active_s: 0.08,
        recover_s: 0.17,
        offset: (28.0, 2.0),
        half_extents: (21.0, 17.0),
        damage: 8,
        knockback: 90.0,
        knockback_growth: 1.68,
        launch_dir: Some((0.9, -0.42)),
        on_hit: None,
    });
    let f_air = vfx_at(f_air, 0.08, "poof_small", (28.0, 2.0), STEP_FX);
    let f_air = on_contact(f_air, "player.hit");

    // BACK AIR — `mule_kick`. She does not turn round.
    let b_air = strike(Strike {
        id: "mule_kick",
        clip: "air_back",
        startup_s: 0.09,
        active_s: 0.06,
        recover_s: 0.19,
        offset: (-28.0, 0.0),
        half_extents: (21.0, 16.0),
        damage: 9,
        knockback: 98.0,
        knockback_growth: 1.82,
        launch_dir: Some((-0.95, -0.38)),
        on_hit: None,
    });
    let b_air = vfx_at(b_air, 0.09, "poof_small", (-28.0, 0.0), STEP_FX);
    let b_air = on_contact(b_air, "player.hit");

    // UP AIR — `flip_kick`. Over the top, at whatever is above her.
    let u_air = strike(Strike {
        id: "flip_kick",
        clip: "air_up",
        startup_s: 0.07,
        active_s: 0.08,
        recover_s: 0.16,
        offset: (2.0, -26.0),
        half_extents: (19.0, 22.0),
        damage: 7,
        knockback: 82.0,
        knockback_growth: 1.60,
        launch_dir: Some((0.08, -1.0)),
        on_hit: None,
    });
    let u_air = vfx_at(u_air, 0.07, "wind_curl", (2.0, -26.0), STEP_FX);
    let u_air = on_contact(u_air, "player.hit");

    // DOWN AIR — `stomp`. THE MOVE SHE COMES FROM. The hardest down-air on
    // the grid, because landing on things is the entire genre she is a
    // protagonist of. Everything else in her kit is light; this is not.
    let d_air = strike(Strike {
        id: "stomp",
        clip: "air_down",
        startup_s: 0.09,
        active_s: 0.09,
        recover_s: 0.20,
        offset: (0.0, 26.0),
        half_extents: (20.0, 20.0),
        damage: 12,
        knockback: 122.0,
        knockback_growth: 2.15,
        launch_dir: Some((0.0, 1.0)),
        on_hit: None,
    });
    let d_air = vfx_at(d_air, 0.09, "landing_puff", (0.0, 26.0), STEP_FX);
    let d_air = sfx(d_air, 0.09, "player.land");
    let d_air = on_contact(d_air, "player.hit");

    // NEUTRAL — `fireball`. The power-up her game is built around, thrown
    // rather than carried. a swung volume rather than a spawned projectile:
    // spawning one would be a second authority on a pattern her own game already
    // owns.
    let n_b = strike(Strike {
        id: "fireball",
        clip: "attack",
        startup_s: 0.14,
        active_s: 0.10,
        recover_s: 0.26,
        offset: (32.0, 4.0),
        half_extents: (24.0, 16.0),
        damage: 9,
        knockback: 96.0,
        knockback_growth: 1.72,
        launch_dir: Some((0.9, -0.45)),
        on_hit: None,
    });
    let n_b = committed_tail(n_b, 0.56, 0.20);
    let n_b = vfx_at(n_b, 0.14, "ember_wisp", (32.0, 4.0), FLAME_FX);
    let n_b = sfx(n_b, 0.14, "player.directional_special");
    let n_b = on_contact(n_b, "player.hit");

    // SIDE — `cape_spin`. She turns once and whatever was next to her is on
    // the other side of the argument.
    let side_b = strike(Strike {
        id: "cape_spin",
        clip: "attack_side",
        startup_s: 0.12,
        active_s: 0.10,
        recover_s: 0.24,
        offset: (26.0, 0.0),
        half_extents: (24.0, 20.0),
        damage: 9,
        knockback: 100.0,
        knockback_growth: 1.85,
        launch_dir: Some((0.92, -0.40)),
        on_hit: None,
    });
    let side_b = impulse(side_b, 0.12, (520.0, 0.0), ImpulseMode::Set);
    let side_b = committed_tail(side_b, 0.54, 0.20);
    let side_b = vfx_at(side_b, 0.12, "wind_curl", (0.0, 0.0), FLAME_FX);
    let side_b = on_contact(side_b, "player.hit");

    // UP — `spring_jump`. THE RECOVERY, and it is a JUMP, which is the only
    // shape her recovery could honestly take. High and cheap to land: the
    // lightest fighter's way home.
    let mut up_b = strike(Strike {
        id: "spring_jump",
        clip: "attack_up",
        startup_s: 0.06,
        active_s: 0.12,
        recover_s: 0.16,
        offset: (0.0, -12.0),
        half_extents: (18.0, 30.0),
        damage: 7,
        knockback: 82.0,
        knockback_growth: 1.58,
        launch_dir: Some((0.10, -1.0)),
        on_hit: None,
    });
    up_b.landing_lag_s = Some(0.20);
    let up_b = impulse(up_b, 0.06, (0.0, -800.0), ImpulseMode::Set);
    let up_b = committed_tail(up_b, 0.46, 0.30);
    let up_b = vfx_at(up_b, 0.06, "landing_puff", (0.0, 18.0), STEP_FX);
    let up_b = sfx(up_b, 0.06, "player.double_jump");
    let up_b = on_contact(up_b, "player.hit");

    // DOWN — `pipe_drop`. She goes down and the floor objects. Grounded-only
    // — the joke needs a pipe to be standing on.
    let down_b = strike(Strike {
        id: "pipe_drop",
        clip: "attack_down",
        startup_s: 0.16,
        active_s: 0.09,
        recover_s: 0.30,
        offset: (0.0, 18.0),
        half_extents: (30.0, 13.0),
        damage: 9,
        knockback: 92.0,
        knockback_growth: 1.68,
        launch_dir: Some((0.7, -0.62)),
        on_hit: None,
    });
    let down_b = committed_tail(down_b, 0.62, 0.0);
    let down_b = vfx_at(down_b, 0.16, "smoke_puff", (0.0, 18.0), FLAME_FX);
    let down_b = on_contact(down_b, "player.hit");

    // effect on ground. Think of bowser down b. In the air he just does a
    // downward slam, but on the ground, it causes him to jump in an arc and then
    // slam. Specials can have different effects in different contexts that
    // should be ok, and makes for a richer smash game, although in most cases
    // they shouldn't be context dependent."*
    //
    // a special gated to ONE posture is not answered in the other — the
    // directional chain walks straight past it to the NEUTRAL special, so a
    // player pressing down-B in the air got the neutral-B. `special_air_down`
    // sits ahead of `special_down` in that chain and has the whole time; this is
    // the two-form move it exists for.
    // DOWN, IN THE AIR — `pipe_dive`. No pipe up here, so she brings the
    // drop instead of the pipe. The genre's own move, and the one press of hers
    // that should never have been missing in the air.
    let mut air_down_b = strike(Strike {
        id: "pipe_dive",
        clip: "air_down",
        startup_s: 0.09,
        active_s: 0.10,
        recover_s: 0.22,
        offset: (0.0, 24.0),
        half_extents: (19.0, 21.0),
        damage: 10,
        knockback: 104.0,
        knockback_growth: 1.82,
        launch_dir: Some((0.0, 1.0)),
        on_hit: None,
    });
    air_down_b.landing_lag_s = Some(0.24);
    let air_down_b = impulse(air_down_b, 0.09, (0.0, 1250.0), ImpulseMode::Set);
    let air_down_b = vfx_at(air_down_b, 0.09, "smoke_puff", (0.0, 22.0), STEP_FX);
    let air_down_b = on_contact(air_down_b, "player.hit");

    // MARY_O'S CAPTURE KIT. The smallest capture box on the roster, because she is
    // the smallest body on it. Everything else is deliberately ordinary.
    // the grab draws `attack`, not `grab`: these sheets publish no `grab` row,
    // and each table's own `every_clip_names_a_row_..._sheet_carries` guard says
    // so. `ClipBinding`'s fallbacks would have covered it at runtime, but a move
    // that NAMES a row nobody publishes is a lie the guard is right to refuse.
    let grab = author_standing_grab(
        grab_shell("mary_o_grab", "attack", 0.07, 0.05, 0.19),
        CaptureAttemptParams {
            offset: (12.0, 1.0),
            half_extents: (16.0, 13.0),
            hold_offset: (13.0, 3.0),
        },
    );
    let pummel = author_pummel(
        capture_beat("mary_o_pummel", "attack", 0.2),
        0.09,
        CapturePummelParams { damage: 3 },
    );
    let forward_throw = author_throw(
        capture_beat("mary_o_fthrow", "attack", 0.25),
        0.13,
        CaptureThrowParams {
            damage: 7,
            knockback: 112.0,
            knockback_growth: 2.0,
            launch_dir: (0.8, -0.6),
        },
    );

    let back_throw = author_throw(
        capture_beat("mary_o_bthrow", "attack", 0.27),
        0.14,
        CaptureThrowParams {
            damage: 8,
            knockback: 120.96,
            knockback_growth: 2.1,
            launch_dir: (-1.0, -0.37),
        },
    );

    let up_throw = author_throw(
        capture_beat("mary_o_uthrow", "attack", 0.26),
        0.13,
        CaptureThrowParams {
            damage: 7,
            knockback: 116.48,
            knockback_growth: 2.04,
            launch_dir: (0.0, -1.0),
        },
    );

    let down_throw = author_throw(
        capture_beat("mary_o_dthrow", "attack", 0.28),
        0.14,
        CaptureThrowParams {
            damage: 5,
            knockback: 82.88,
            knockback_growth: 1.6,
            launch_dir: (0.32, -0.92),
        },
    );

    SmashRepertoire {
        taunt: ambition_platformer2d::characters::moveset_authoring::taunt("mary_o_taunt", 0.9),
        dash_attack: ambition_platformer2d::characters::moveset_authoring::dash_attack(
            "mary_o_dash_attack",
            ambition_platformer2d::characters::moveset_authoring::DashAttackShape::GENRE,
            7,
            82.5,
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
            grounded: down_b,
            airborne: air_down_b,
        },
    }
    .into_contract()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Fourteen fighters each carried a copy of it: every bound verb names a move
    // this table defines, and the table binds the whole vocabulary. Both are now
    // unwritable defects rather than tested ones. `SmashRepertoire` owns the verb
    // strings, so there is no string in this file to misspell; it is a struct
    // with no `Default` and no private fields, so a missing or renamed slot is a
    // COMPILE error here. What the fourteen copies stood for — that every press
    // is answered, in every posture it is asked in — is checked once, by
    // `ambition_platformer2d::characters::smash_repertoire`, and by the host ratchet
    // `smash_roster_movesets::report_the_smash_kit_every_selectable_fighter_has`.

    /// THE STOMP IS HER HARDEST HIT, which is the identity claim the module
    /// doc makes and the one a retune would quietly lose.
    #[test]
    fn the_down_air_hits_harder_than_anything_else_she_has() {
        let moveset = mary_o_moveset();
        let damage = |id: &str| {
            moveset
                .move_by_id(id)
                .unwrap_or_else(|| panic!("{id} exists"))
                .windows
                .iter()
                .flat_map(|w| w.volumes.iter())
                .map(|v| v.damage)
                .max()
                .unwrap_or(0)
        };
        let stomp = damage("stomp");
        for other in ["shell_kick", "block_punch", "ground_pound", "fireball"] {
            assert!(
                stomp >= damage(other),
                "`{other}` hits harder than the stomp, so the platformer \
                 protagonist's best move is no longer falling on somebody"
            );
        }
    }
}

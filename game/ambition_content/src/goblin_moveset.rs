//! The goblin's repertoire — the third character in the game to state its own
//! moves, and the first ENEMY to.
//!
//! That floor is one `simple_melee` swipe handed to every seated fighter whose character says
//! nothing, and its goal is DELETION: the count falls by one each time somebody writes a table.
//! The goblin is the cheapest next one — Ambition's own, already on the grid, already authoring
//! its body (170 px/s, 5 HP, 0.70 contact) and its controller policy (the shared
//! `medium_striker`).
//!
//! it is NOT the robot's table with different numbers. A goblin is small,
//! fast and scrappy: it gets in, it pokes, and its punish window is short because
//! it cannot afford a long one. Against the robot that reads as
//!
//! ```text
//!            reach     jab startup   f-smash damage   b-air
//!   robot     26 px       0.05 s          15          10
//!   goblin    22 px       0.04 s          12          9
//! ```
//!
//! — a shorter, faster, weaker fighter that has to be closer to matter. The
//! differences are the authored ones; the SHAPE is `strike`'s, which is the whole
//! reason that helper was pulled out of the robot's file.
//!
//! the clip names are the standard vocabulary and the fallback chain does the
//! rest. The goblin sheet does not have 132 rows; `strike` names `smash_forward`
//! and settles for `attack_side`, then `attack`, then `slash`, then `idle`. A
//! missing clip costs the move its picture, never its gameplay.

use ambition_characters::smash_capture::{
    author_pummel, author_standing_grab, author_throw, capture_beat, grab_shell,
    CaptureAttemptParams, CaptureCues, CapturePummelParams, CaptureThrowParams,
    SmashCaptureRepertoire,
};
use ambition_characters::smash_repertoire::{DownSpecial, NeutralSpecial, SmashRepertoire};
use ambition_platformer2d::entity_catalog::MovesetContract;

use ambition_characters::moveset_authoring::{
    committed_tail, impulse, on_contact, sfx, strike, vfx_at,
};
use ambition_platformer2d::entity_catalog::ImpulseMode;

/// See the module doc. Sixteen moves, the genre's standard verb map.
pub fn goblin_moveset() -> MovesetContract {
    // ── grounded ─────────────────────────────────────────────────────────────
    //
    // Faster than the robot's and weaker, which is the whole character in one
    // move: the goblin's jab is the thing it throws while walking into you.
    let jab = strike(
        "jab",
        "jab",
        0.04,
        0.05,
        0.12,
        (22.0, 0.0),
        (16.0, 13.0),
        2,
        45.0,
        1.05,
        None,
        None,
    );

    // An upward poke that beats a shorthop. Small volume — it is an
    // anti-air, not a wall.
    let up_tilt = strike(
        "tilt_up",
        "attack_up",
        0.07,
        0.07,
        0.16,
        (10.0, -24.0),
        (16.0, 18.0),
        4,
        70.0,
        1.30,
        Some((0.15, -1.0)),
        None,
    );

    // Low and forward: the goblin's ground game is knee height, which is where a
    // small body's reach actually is.
    let down_tilt = strike(
        "tilt_down",
        "attack_down",
        0.06,
        0.06,
        0.15,
        (22.0, 12.0),
        (18.0, 10.0),
        3,
        50.0,
        1.15,
        Some((1.0, -0.25)),
        None,
    );

    // ── smashes ──────────────────────────────────────────────────────────────
    //
    // committed and NOT safe. The goblin's kill move costs it 0.30s of
    // recovery against a body that only has 5 HP to trade with — throwing this
    // and missing is how a goblin dies, which is what makes landing it exciting
    // rather than routine.
    let mut f_smash = strike(
        "smash_forward",
        "smash_forward",
        0.28,
        0.06,
        0.30,
        (34.0, -2.0),
        (24.0, 18.0),
        12,
        135.0,
        2.85,
        Some((1.0, -0.40)),
        None,
    );
    f_smash.smash_charge_mult = 1.7;

    let mut up_smash = strike(
        "smash_up",
        "smash_up",
        0.24,
        0.07,
        0.28,
        (6.0, -30.0),
        (20.0, 26.0),
        11,
        140.0,
        2.70,
        Some((0.0, -1.0)),
        None,
    );
    up_smash.smash_charge_mult = 1.7;

    // Both sides, low — the goblin's answer to being surrounded, and its
    // ledge-guard.
    let mut down_smash = strike(
        "smash_down",
        "smash_down",
        0.22,
        0.08,
        0.32,
        (0.0, 14.0),
        (32.0, 12.0),
        10,
        125.0,
        2.55,
        Some((0.9, -0.55)),
        None,
    );
    down_smash.smash_charge_mult = 1.7;

    // ── aerials ──────────────────────────────────────────────────────────────
    let n_air = strike(
        "air_neutral",
        "air_neutral",
        0.05,
        0.10,
        0.13,
        (0.0, 0.0),
        (22.0, 20.0),
        4,
        60.0,
        1.25,
        None,
        None,
    );

    let f_air = strike(
        "air_forward",
        "air_forward",
        0.08,
        0.07,
        0.16,
        (26.0, -2.0),
        (20.0, 16.0),
        6,
        85.0,
        1.70,
        Some((1.0, -0.30)),
        None,
    );

    // the goblin's best kill option, and it faces the wrong way — the classic
    // trade. Committing to a back-air means committing to not looking at them.
    let b_air = strike(
        "air_back",
        "air_back",
        0.10,
        0.06,
        0.20,
        (-28.0, 0.0),
        (20.0, 16.0),
        9,
        120.0,
        2.40,
        Some((-1.0, -0.35)),
        None,
    );

    let u_air = strike(
        "air_up",
        "air_up",
        0.06,
        0.08,
        0.14,
        (2.0, -26.0),
        (18.0, 20.0),
        5,
        80.0,
        1.80,
        Some((0.0, -1.0)),
        None,
    );

    // Straight down and hard. no `on_hit` rebound: the robot's down-air says it
    // is capable of bouncing its attacker and this one does not, because a goblin
    // that could pogo off a body would out-recover a character built around
    // recovery being its problem.
    let d_air = strike(
        "air_down",
        "air_down",
        0.11,
        0.07,
        0.22,
        (4.0, 24.0),
        (18.0, 18.0),
        8,
        110.0,
        2.10,
        Some((0.0, 1.0)),
        None,
    );


    // a forward tilt, because without one the commonest press in the genre
    // falls down the directional chain to the jab. A goblin's is a scrappy
    // shove — shorter and faster than anybody else's, like the rest of its kit.
    let f_tilt = strike(
        "tilt_forward",
        "attack_side",
        0.06,
        0.06,
        0.14,
        (24.0, -2.0),
        (18.0, 12.0),
        4,
        60.0,
        1.20,
        Some((1.0, -0.25)),
        None,
    );
    let f_tilt = vfx_at(f_tilt, 0.06, "air_slice", (24.0, -2.0), 0.7);
    let f_tilt = sfx(f_tilt, 0.06, "enemy.goblin.attack");
    let f_tilt = on_contact(f_tilt, "enemy.goblin.hit");

    // NEUTRAL — `scrap_flail`. No technique at all: it turns its whole body
    // into the swing and hopes. Wide, slow for a goblin, and the only move in
    // its kit that covers both sides of it.
    let n_b = strike(
        "scrap_flail",
        "attack",
        0.10,
        0.10,
        0.26,
        (14.0, 0.0),
        (26.0, 20.0),
        7,
        88.0,
        1.70,
        Some((0.85, -0.50)),
        None,
    );
    let n_b = committed_tail(n_b, 0.55, 0.15);
    let n_b = vfx_at(n_b, 0.10, "air_slice", (14.0, 0.0), 1.1);
    let n_b = sfx(n_b, 0.10, "enemy.goblin.attack");
    let n_b = on_contact(n_b, "enemy.goblin.hit");

    // SIDE — `headlong_charge`. It runs at you. `ImpulseMode::Set`, so a
    // goblin already falling gets the same charge as a standing one — the
    // difference between a committed move and a suggestion — and the tail damps
    // steering to 0.1 rather than 0.0, because a scrappy fighter that could not
    // adjust at all would be a heavyweight.
    let side_b = strike(
        "headlong_charge",
        "attack_side",
        0.14,
        0.10,
        0.24,
        (24.0, 2.0),
        (22.0, 16.0),
        8,
        100.0,
        1.90,
        Some((0.95, -0.35)),
        None,
    );
    let side_b = impulse(side_b, 0.14, (560.0, 0.0), ImpulseMode::Set);
    let side_b = committed_tail(side_b, 0.60, 0.10);
    let side_b = vfx_at(side_b, 0.14, "dash_streak", (0.0, 0.0), 1.0);
    let side_b = sfx(side_b, 0.14, "enemy.goblin.attack");
    let side_b = on_contact(side_b, "enemy.goblin.hit");

    // UP — `scramble_leap`. THE RECOVERY, and the reason this batch is not
    // cosmetic: with no special at all, a goblin knocked off the stage had a
    // double jump and nothing else. It claws upward — weaker than a heavyweight's
    // lift and cheaper to land, which is the small fighter's bargain.
    let mut up_b = strike(
        "scramble_leap",
        "attack_up",
        0.08,
        0.12,
        0.18,
        (0.0, -10.0),
        (18.0, 26.0),
        6,
        78.0,
        1.50,
        Some((0.10, -1.0)),
        None,
    );
    up_b.landing_lag_s = Some(0.24);
    let up_b = impulse(up_b, 0.08, (0.0, -720.0), ImpulseMode::Set);
    let up_b = committed_tail(up_b, 0.46, 0.25);
    let up_b = vfx_at(up_b, 0.08, "landing_puff", (0.0, 16.0), 0.9);
    let up_b = sfx(up_b, 0.08, "enemy.goblin.jump");
    let up_b = on_contact(up_b, "enemy.goblin.hit");

    // DOWN — `dirt_kick`. It kicks the ground at you. Wide, low and flat,
    // and grounded-only because the whole move is that there is ground.
    let down_b = strike(
        "dirt_kick",
        "attack_down",
        0.12,
        0.08,
        0.28,
        (18.0, 16.0),
        (30.0, 10.0),
        6,
        70.0,
        1.35,
        Some((0.70, -0.60)),
        None,
    );
    let down_b = committed_tail(down_b, 0.55, 0.0);
    let down_b = vfx_at(down_b, 0.12, "sand_burst", (18.0, 16.0), 1.0);
    let down_b = sfx(down_b, 0.12, "enemy.goblin.attack");
    let down_b = on_contact(down_b, "enemy.goblin.hit");

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
    // DOWN, IN THE AIR — `dive_stomp`. It cannot kick the ground from up
    // there, so it becomes the ground: knees up, straight down, and whoever is
    // under it is the floor.
    let mut air_down_b = strike(
        "dive_stomp",
        "air_down",
        0.08,
        0.10,
        0.22,
        (0.0, 24.0),
        (18.0, 20.0),
        7,
        84.0,
        1.60,
        Some((0.0, 1.0)),
        None,
    );
    air_down_b.landing_lag_s = Some(0.22);
    let air_down_b = impulse(air_down_b, 0.08, (0.0, 1150.0), ImpulseMode::Set);
    let air_down_b = vfx_at(air_down_b, 0.08, "sand_burst", (0.0, 20.0), 0.9);
    let air_down_b = sfx(air_down_b, 0.08, "enemy.goblin.attack");
    let air_down_b = on_contact(air_down_b, "enemy.goblin.hit");

    // GOBLIN'S CAPTURE KIT. Short reach, fast everything, weak throw: it wins the
    // grab and cannot finish with it. The flattest launch on the roster.
    // the grab draws `attack`, not `grab`: these sheets publish no `grab` row,
    // and each table's own `every_clip_names_a_row_..._sheet_carries` guard says
    // so. `ClipBinding`'s fallbacks would have covered it at runtime, but a move
    // that NAMES a row nobody publishes is a lie the guard is right to refuse.
    let grab = author_standing_grab(
        grab_shell("goblin_grab", "attack", 0.06, 0.04, 0.22),
        CaptureAttemptParams {
            offset: (12.0, 1.0),
            half_extents: (17.0, 14.0),
            hold_offset: (13.0, 3.0),
        },
    );
    let pummel = author_pummel(
        capture_beat("goblin_pummel", "attack", 0.14),
        0.06,
        CapturePummelParams { damage: 2 },
    );
    let forward_throw = author_throw(
        capture_beat("goblin_fthrow", "attack", 0.23),
        0.12,
        CaptureThrowParams {
            damage: 6,
            knockback: 100.0,
            knockback_growth: 2.2,
            launch_dir: (1.0, -0.35),
        },
    );

    let back_throw = author_throw(
        capture_beat("goblin_bthrow", "attack", 0.25),
        0.13,
        CaptureThrowParams {
            damage: 7,
            knockback: 108.0,
            knockback_growth: 2.31,
            launch_dir: (-1.0, -0.25),
        },
    );

    let up_throw = author_throw(
        capture_beat("goblin_uthrow", "attack", 0.24),
        0.12,
        CaptureThrowParams {
            damage: 6,
            knockback: 104.0,
            knockback_growth: 2.24,
            launch_dir: (0.0, -1.0),
        },
    );

    let down_throw = author_throw(
        capture_beat("goblin_dthrow", "attack", 0.26),
        0.13,
        CaptureThrowParams {
            damage: 4,
            knockback: 74.0,
            knockback_growth: 1.76,
            launch_dir: (0.4, -0.92),
        },
    );
    SmashRepertoire {
        taunt: ambition_characters::moveset_authoring::taunt("goblin_taunt", 0.9),
        dash_attack: ambition_characters::moveset_authoring::dash_attack(
            "goblin_dash_attack",
            ambition_characters::moveset_authoring::DashAttackShape::GENRE,
            6,
            75.0,
        ),
        jab,
        forward_tilt: f_tilt,
        up_tilt,
        down_tilt,
        forward_smash: f_smash,
        up_smash,
        down_smash,
        neutral_air: n_air,
        forward_air: f_air,
        back_air: b_air,
        up_air: u_air,
        down_air: d_air,
        neutral_special: NeutralSpecial::Authored(n_b),
        side_special: side_b,
        up_special: up_b,
        // AUTHORED, at the rule that every fighter in the smash roster have a grab. The
        // transitional `None` is gone: capture was proven on George and the Pirate Admiral, and
        // the whole point of proving it was to stop being the only two.
        //
        // the VALUES are per character on purpose. A roster whose grabs are
        // twelve copies of one number set is one grab wearing twelve names.
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
    // `ambition_characters::smash_repertoire`, and by the host ratchet
    // `smash_roster_movesets::report_the_smash_kit_every_selectable_fighter_has`.

    /// The goblin is not the robot with different numbers.
    ///
    /// the point of a per-character table is that the characters differ, and a
    /// table copied wholesale would pass every other test in this file. This pins
    /// the identity the module doc claims: shorter reach, faster jab, weaker kill.
    #[test]
    fn the_goblin_is_shorter_faster_and_weaker_than_the_robot() {
        let goblin = goblin_moveset();
        let robot = crate::player_robot_moveset::player_robot_moveset();
        let find = |set: &MovesetContract, id: &str| {
            set.moves
                .iter()
                .find(|m| m.id == id)
                .unwrap_or_else(|| panic!("{id} exists"))
                .clone()
        };

        let (g_jab, r_jab) = (find(&goblin, "jab"), find(&robot, "jab"));
        let startup = |m: &ambition_platformer2d::entity_catalog::MoveSpec| {
            m.windows
                .iter()
                .find(|w| {
                    matches!(
                        w.tag,
                        ambition_platformer2d::entity_catalog::WindowTag::Active
                    )
                })
                .expect("a strike has an active window")
                .start_s
        };
        assert!(
            startup(&g_jab) < startup(&r_jab),
            "the goblin's jab comes out faster"
        );

        let reach = |m: &ambition_platformer2d::entity_catalog::MoveSpec| {
            m.windows
                .iter()
                .flat_map(|w| w.volumes.iter())
                .map(|v| match v.shape {
                    ambition_platformer2d::entity_catalog::VolumeShape::Rect {
                        offset,
                        half_extents,
                    } => offset.0.abs() + half_extents.0,
                    _ => 0.0,
                })
                .fold(0.0f32, f32::max)
        };
        assert!(
            reach(&g_jab) < reach(&r_jab),
            "and it reaches less far, which is what makes it have to get close"
        );

        let damage = |m: &ambition_platformer2d::entity_catalog::MoveSpec| {
            m.windows
                .iter()
                .flat_map(|w| w.volumes.iter())
                .map(|v| v.damage)
                .max()
                .unwrap_or(0)
        };
        assert!(
            damage(&find(&goblin, "smash_forward")) < damage(&find(&robot, "smash_forward")),
            "and its kill move hits softer — a small fighter trades reach and \
             power for speed, or it is just the robot in a different sheet"
        );
    }
}

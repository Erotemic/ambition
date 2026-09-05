//! Goblin-authored platform-fighter repertoire.
//!
//! The goblin is a short-range, fast-startup, lower-damage fighter. Moves use
//! the shared `strike` authoring shape and standard animation fallback vocabulary,
//! so missing specialized clips affect presentation rather than gameplay.

use ambition_characters::moveset_authoring::Strike;
use ambition_characters::smash_capture::{
    author_pummel, author_standing_grab, author_throw, capture_beat, grab_shell,
    CaptureAttemptParams, CaptureCues, CapturePummelParams, CaptureThrowParams,
    SmashCaptureRepertoire,
};
use ambition_characters::smash_repertoire::{
    DownSpecial, NeutralSpecial, SmashRepertoire, UpSpecial,
};
use ambition_platformer2d::entity_catalog::MovesetContract;

use ambition_characters::moveset_authoring::{
    committed_tail, impulse, on_contact, sfx, strike, vfx_at,
};
use ambition_platformer2d::entity_catalog::ImpulseMode;

/// The goblin's standard platform-fighter verb map.
pub fn goblin_moveset() -> MovesetContract {
    // ── grounded ─────────────────────────────────────────────────────────────
    //
    // Faster than the robot's and weaker, which is the whole character in one
    // move: the goblin's jab is the thing it throws while walking into you.
    let jab = strike(Strike {
        id: "jab",
        clip: "jab",
        startup_s: 0.04,
        active_s: 0.05,
        recover_s: 0.12,
        offset: (22.0, 0.0),
        half_extents: (16.0, 13.0),
        damage: 2,
        knockback: 45.0,
        knockback_growth: 1.05,
        launch_dir: None,
        on_hit: None,
    });

    // An upward poke that beats a shorthop. Small volume — it is an
    // anti-air, not a wall.
    let up_tilt = strike(Strike {
        id: "tilt_up",
        clip: "attack_up",
        startup_s: 0.07,
        active_s: 0.07,
        recover_s: 0.16,
        offset: (10.0, -24.0),
        half_extents: (16.0, 18.0),
        damage: 4,
        knockback: 70.0,
        knockback_growth: 1.30,
        launch_dir: Some((0.15, -1.0)),
        on_hit: None,
    });

    // Low and forward: the goblin's ground game is knee height, which is where a
    // small body's reach actually is.
    let down_tilt = strike(Strike {
        id: "tilt_down",
        clip: "attack_down",
        startup_s: 0.06,
        active_s: 0.06,
        recover_s: 0.15,
        offset: (22.0, 12.0),
        half_extents: (18.0, 10.0),
        damage: 3,
        knockback: 50.0,
        knockback_growth: 1.15,
        launch_dir: Some((1.0, -0.25)),
        on_hit: None,
    });

    // ── smashes ──────────────────────────────────────────────────────────────
    //
    // committed and NOT safe. The goblin's kill move costs it 0.30s of
    // recovery against a body that only has 5 HP to trade with — throwing this
    // and missing is how a goblin dies, which is what makes landing it exciting
    // rather than routine.
    let mut f_smash = strike(Strike {
        id: "smash_forward",
        clip: "smash_forward",
        startup_s: 0.28,
        active_s: 0.06,
        recover_s: 0.30,
        offset: (34.0, -2.0),
        half_extents: (24.0, 18.0),
        damage: 12,
        knockback: 135.0,
        knockback_growth: 2.85,
        launch_dir: Some((1.0, -0.40)),
        on_hit: None,
    });
    f_smash.smash_charge_mult = 1.7;

    let mut up_smash = strike(Strike {
        id: "smash_up",
        clip: "smash_up",
        startup_s: 0.24,
        active_s: 0.07,
        recover_s: 0.28,
        offset: (6.0, -30.0),
        half_extents: (20.0, 26.0),
        damage: 11,
        knockback: 140.0,
        knockback_growth: 2.70,
        launch_dir: Some((0.0, -1.0)),
        on_hit: None,
    });
    up_smash.smash_charge_mult = 1.7;

    // Both sides, low — the goblin's answer to being surrounded, and its
    // ledge-guard.
    let mut down_smash = strike(Strike {
        id: "smash_down",
        clip: "smash_down",
        startup_s: 0.22,
        active_s: 0.08,
        recover_s: 0.32,
        offset: (0.0, 14.0),
        half_extents: (32.0, 12.0),
        damage: 10,
        knockback: 125.0,
        knockback_growth: 2.55,
        launch_dir: Some((0.9, -0.55)),
        on_hit: None,
    });
    down_smash.smash_charge_mult = 1.7;

    // ── aerials ──────────────────────────────────────────────────────────────
    let n_air = strike(Strike {
        id: "air_neutral",
        clip: "air_neutral",
        startup_s: 0.05,
        active_s: 0.10,
        recover_s: 0.13,
        offset: (0.0, 0.0),
        half_extents: (22.0, 20.0),
        damage: 4,
        knockback: 60.0,
        knockback_growth: 1.25,
        launch_dir: None,
        on_hit: None,
    });

    let f_air = strike(Strike {
        id: "air_forward",
        clip: "air_forward",
        startup_s: 0.08,
        active_s: 0.07,
        recover_s: 0.16,
        offset: (26.0, -2.0),
        half_extents: (20.0, 16.0),
        damage: 6,
        knockback: 85.0,
        knockback_growth: 1.70,
        launch_dir: Some((1.0, -0.30)),
        on_hit: None,
    });

    // the goblin's best kill option, and it faces the wrong way — the classic
    // trade. Committing to a back-air means committing to not looking at them.
    let b_air = strike(Strike {
        id: "air_back",
        clip: "air_back",
        startup_s: 0.10,
        active_s: 0.06,
        recover_s: 0.20,
        offset: (-28.0, 0.0),
        half_extents: (20.0, 16.0),
        damage: 9,
        knockback: 120.0,
        knockback_growth: 2.40,
        launch_dir: Some((-1.0, -0.35)),
        on_hit: None,
    });

    let u_air = strike(Strike {
        id: "air_up",
        clip: "air_up",
        startup_s: 0.06,
        active_s: 0.08,
        recover_s: 0.14,
        offset: (2.0, -26.0),
        half_extents: (18.0, 20.0),
        damage: 5,
        knockback: 80.0,
        knockback_growth: 1.80,
        launch_dir: Some((0.0, -1.0)),
        on_hit: None,
    });

    // Straight down and hard. no `on_hit` rebound: the robot's down-air says it
    // is capable of bouncing its attacker and this one does not, because a goblin
    // that could pogo off a body would out-recover a character built around
    // recovery being its problem.
    let d_air = strike(Strike {
        id: "air_down",
        clip: "air_down",
        startup_s: 0.11,
        active_s: 0.07,
        recover_s: 0.22,
        offset: (4.0, 24.0),
        half_extents: (18.0, 18.0),
        damage: 8,
        knockback: 110.0,
        knockback_growth: 2.10,
        launch_dir: Some((0.0, 1.0)),
        on_hit: None,
    });

    // a forward tilt, because without one the commonest press in the genre
    // falls down the directional chain to the jab. A goblin's is a scrappy
    // shove — shorter and faster than anybody else's, like the rest of its kit.
    let f_tilt = strike(Strike {
        id: "tilt_forward",
        clip: "attack_side",
        startup_s: 0.06,
        active_s: 0.06,
        recover_s: 0.14,
        offset: (24.0, -2.0),
        half_extents: (18.0, 12.0),
        damage: 4,
        knockback: 60.0,
        knockback_growth: 1.20,
        launch_dir: Some((1.0, -0.25)),
        on_hit: None,
    });
    let f_tilt = vfx_at(f_tilt, 0.06, "air_slice", (24.0, -2.0), 0.7);
    let f_tilt = sfx(f_tilt, 0.06, "enemy.goblin.attack");
    let f_tilt = on_contact(f_tilt, "enemy.goblin.hit");

    // NEUTRAL — `scrap_flail`. No technique at all: it turns its whole body
    // into the swing and hopes. Wide, slow for a goblin, and the only move in
    // its kit that covers both sides of it.
    let n_b = strike(Strike {
        id: "scrap_flail",
        clip: "attack",
        startup_s: 0.10,
        active_s: 0.10,
        recover_s: 0.26,
        offset: (14.0, 0.0),
        half_extents: (26.0, 20.0),
        damage: 7,
        knockback: 88.0,
        knockback_growth: 1.70,
        launch_dir: Some((0.85, -0.50)),
        on_hit: None,
    });
    let n_b = committed_tail(n_b, 0.55, 0.15);
    let n_b = vfx_at(n_b, 0.10, "air_slice", (14.0, 0.0), 1.1);
    let n_b = sfx(n_b, 0.10, "enemy.goblin.attack");
    let n_b = on_contact(n_b, "enemy.goblin.hit");

    // SIDE — `headlong_charge`. It runs at you. `ImpulseMode::Set`, so a
    // goblin already falling gets the same charge as a standing one — the
    // difference between a committed move and a suggestion — and the tail damps
    // steering to 0.1 rather than 0.0, because a scrappy fighter that could not
    // adjust at all would be a heavyweight.
    let side_b = strike(Strike {
        id: "headlong_charge",
        clip: "attack_side",
        startup_s: 0.14,
        active_s: 0.10,
        recover_s: 0.24,
        offset: (24.0, 2.0),
        half_extents: (22.0, 16.0),
        damage: 8,
        knockback: 100.0,
        knockback_growth: 1.90,
        launch_dir: Some((0.95, -0.35)),
        on_hit: None,
    });
    let side_b = impulse(side_b, 0.14, (560.0, 0.0), ImpulseMode::Set);
    let side_b = committed_tail(side_b, 0.60, 0.10);
    let side_b = vfx_at(side_b, 0.14, "dash_streak", (0.0, 0.0), 1.0);
    let side_b = sfx(side_b, 0.14, "enemy.goblin.attack");
    let side_b = on_contact(side_b, "enemy.goblin.hit");

    // UP — `scramble_leap`. THE RECOVERY, and the reason this batch is not
    // cosmetic: with no special at all, a goblin knocked off the stage had a
    // double jump and nothing else. It claws upward — weaker than a heavyweight's
    // lift and cheaper to land, which is the small fighter's bargain.
    let mut up_b = strike(Strike {
        id: "scramble_leap",
        clip: "attack_up",
        startup_s: 0.08,
        active_s: 0.12,
        recover_s: 0.18,
        offset: (0.0, -10.0),
        half_extents: (18.0, 26.0),
        damage: 6,
        knockback: 78.0,
        knockback_growth: 1.50,
        launch_dir: Some((0.10, -1.0)),
        on_hit: None,
    });
    up_b.landing_lag_s = Some(0.24);
    let up_b = impulse(up_b, 0.08, (0.0, -720.0), ImpulseMode::Set);
    let up_b = committed_tail(up_b, 0.46, 0.25);
    let up_b = vfx_at(up_b, 0.08, "landing_puff", (0.0, 16.0), 0.9);
    let up_b = sfx(up_b, 0.08, "enemy.goblin.jump");
    let up_b = on_contact(up_b, "enemy.goblin.hit");

    // DOWN — `dirt_kick`. It kicks the ground at you. Wide, low and flat,
    // and grounded-only because the whole move is that there is ground.
    let down_b = strike(Strike {
        id: "dirt_kick",
        clip: "attack_down",
        startup_s: 0.12,
        active_s: 0.08,
        recover_s: 0.28,
        offset: (18.0, 16.0),
        half_extents: (30.0, 10.0),
        damage: 6,
        knockback: 70.0,
        knockback_growth: 1.35,
        launch_dir: Some((0.70, -0.60)),
        on_hit: None,
    });
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
    let mut air_down_b = strike(Strike {
        id: "dive_stomp",
        clip: "air_down",
        startup_s: 0.08,
        active_s: 0.10,
        recover_s: 0.22,
        offset: (0.0, 24.0),
        half_extents: (18.0, 20.0),
        damage: 7,
        knockback: 84.0,
        knockback_growth: 1.60,
        launch_dir: Some((0.0, 1.0)),
        on_hit: None,
    });
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

    // ⭐⭐ THE CARGO CARRY — proof move 5, and the goblin gets it because hauling
    // somebody off is what a goblin DOES. Down + Attack inside a grab no longer
    // throws: it hoists the captive onto its back and lets the goblin WALK.
    //
    // ⛔⛔ IT DISPLACES THE WEAKEST THING IN THE KIT AND THAT IS DELIBERATE. The
    // move it replaces was `damage: 4, knockback: 74` on the fallback `"attack"`
    // clip — the lowest-damage throw on the roster, generic in every field. ⇒ I
    // looked for an EMPTY slot first, the way the mine and Sing found one, and
    // there is none: every fighter authors all four throws. So the next-best
    // rule is to spend the least, and this was the least.
    //
    // ⭐ AND THE OTHER THREE THROWS ARE THE EXIT. Nothing new had to be wired to
    // put the captive down: while carrying, forward/back/up + Attack still throw,
    // because a carry is an ordinary hold with two terms changed. Down enters,
    // any direction leaves — which is the genre's own grammar, arrived at by not
    // inventing one.
    //
    // ⚠ NO DAMAGE ON THE CARRY. Taking the weight is not a hit, and a carry that
    // also chipped would make entering it strictly better than the throw it
    // replaced instead of a different choice.
    let down_throw = ambition_characters::smash_capture::author_carry(
        capture_beat("goblin_dthrow", "attack", 0.26),
        0.13,
        ambition_characters::smash_capture::CaptureCarryParams {
            // Up and over the shoulder. `+y` is gravity-DOWN, so the negative
            // lifts them; slightly forward so the goblin is not wearing them.
            hold_offset: (6.0, -18.0),
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
        up_special: UpSpecial::Standard(up_b),
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

    /// ⛔⛔ A CARRY, NOT A THROW, AND NOT BOTH. The failure this guards against is
    /// the quiet one: leaving `author_throw` beside `author_carry` so the down
    /// press hoists the captive AND launches it, which reads in play as a carry
    /// that randomly does not work.
    #[test]
    fn the_goblins_down_throw_hauls_instead_of_launching() {
        let moves = goblin_moveset();
        let beat = moves
            .moves
            .iter()
            .find(|m| m.id == "goblin_dthrow")
            .expect("the goblin has a down-throw slot");
        let keys: Vec<&str> = beat
            .events
            .iter()
            .filter_map(|event| match &event.kind {
                ambition_entity_catalog::MoveEventKind::Effect(effect) => {
                    Some(effect.key.as_str())
                }
                _ => None,
            })
            .collect();
        assert!(
            keys.contains(&ambition_characters::smash_capture::CAPTURE_CARRY),
            "the goblin's down press does not take the weight: {keys:?}"
        );
        assert!(
            !keys.contains(&ambition_characters::smash_capture::CAPTURE_THROW),
            "the goblin's down press both hauls AND throws: {keys:?}"
        );
    }
}

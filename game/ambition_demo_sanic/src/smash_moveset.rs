//! **Sanic's repertoire, for the stage he visits rather than the one he lives
//! on.**
//!
//! ⭐ **written 2026-08-16** (Jon: *"Let's complete the kit for all
//! characters."*). A census of the smash grid found him at **0/16** — no table,
//! no action set, and no unarmed floor reaching his body, so every press was
//! silence. He was one of four in that state.
//!
//! ⛔⛔ **AND IT CHANGES NOTHING ON THE SPEEDWAY.** His catalog rows author
//! `abilities: Some([RunJump])` — Jon, the same day: *"Sanic should never have
//! fly, blink, or wall climb in any iteration"* — and `RunJump` carries no
//! `attack`. A move table is *what the attack IS*; the ability is *whether this
//! body may attack at all*. At home the answer is no and these sixteen moves are
//! unreachable; on a stage that GRANTS the verb (`MatchAbilities::levelled`)
//! they are what he swings.
//!
//! ⚠ **and it is not his spin dash.** `declare_sanic_techniques` puts spin dash
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

use ambition_platformer2d::characters::moveset_authoring::{
    committed_tail, impulse, on_contact, sfx, strike, vfx_at,
};
use ambition_platformer2d::characters::smash_repertoire::{
    DownSpecial, NeutralSpecial, SmashRepertoire,
};
use ambition_platformer2d::entity_catalog::{ImpulseMode, MovesetContract};

const RUSH_FX: f32 = 0.85;
const BOOM_FX: f32 = 1.2;

/// See the module doc. Sixteen presses.
pub fn sanic_moveset() -> MovesetContract {
    // **JAB — `quick_jab`.** Four frames. It is not meant to do anything except
    // arrive before yours.
    let jab = strike(
        "quick_jab",
        "jab",
        0.04,
        0.04,
        0.11,
        (22.0, 0.0),
        (16.0, 12.0),
        2,
        42.0,
        1.02,
        None,
        None,
    );
    let jab = vfx_at(jab, 0.04, "air_slice", (22.0, 0.0), RUSH_FX);
    let jab = on_contact(jab, "player.hit");

    // **FORWARD TILT — `run_up_kick`.** He is already moving; this is what that
    // looks like when it hits something.
    let f_tilt = strike(
        "run_up_kick",
        "attack_side",
        0.06,
        0.06,
        0.14,
        (28.0, 2.0),
        (20.0, 13.0),
        5,
        62.0,
        1.22,
        Some((1.0, -0.26)),
        None,
    );
    let f_tilt = vfx_at(f_tilt, 0.06, "dash_streak", (28.0, 2.0), RUSH_FX);
    let f_tilt = on_contact(f_tilt, "player.hit");

    // **UP TILT — `heel_flick`.** Up and behind, off the run.
    let u_tilt = strike(
        "heel_flick",
        "attack_up",
        0.06,
        0.06,
        0.15,
        (4.0, -24.0),
        (16.0, 20.0),
        4,
        68.0,
        1.28,
        Some((0.10, -1.0)),
        None,
    );
    let u_tilt = vfx_at(u_tilt, 0.06, "wind_curl", (4.0, -24.0), RUSH_FX);
    let u_tilt = on_contact(u_tilt, "player.hit");

    // **DOWN TILT — `skid`.** He stops, briefly, and the ground does not.
    let d_tilt = strike(
        "skid",
        "attack_down",
        0.05,
        0.06,
        0.14,
        (26.0, 14.0),
        (22.0, 9.0),
        4,
        50.0,
        1.12,
        Some((0.95, -0.26)),
        None,
    );
    let d_tilt = vfx_at(d_tilt, 0.05, "skid_puff", (26.0, 14.0), RUSH_FX);
    let d_tilt = on_contact(d_tilt, "player.hit");

    // **FORWARD SMASH — `sonic_boom`.** The one moment he stops being quick and
    // becomes hard. ⚠ still the second-weakest forward smash on the grid.
    let f_smash = strike(
        "sonic_boom",
        "smash_forward",
        0.15,
        0.08,
        0.28,
        (34.0, 0.0),
        (26.0, 18.0),
        11,
        118.0,
        2.15,
        Some((0.95, -0.40)),
        None,
    );
    let f_smash = vfx_at(f_smash, 0.15, "sonic_boom", (34.0, 0.0), BOOM_FX);
    let f_smash = sfx(f_smash, 0.15, "player.attack.charge");
    let f_smash = on_contact(f_smash, "player.hit");

    // **UP SMASH — `updraft`.** A vertical burst off a standing start.
    let u_smash = strike(
        "updraft",
        "smash_up",
        0.14,
        0.09,
        0.26,
        (2.0, -30.0),
        (20.0, 30.0),
        10,
        112.0,
        2.12,
        Some((0.08, -1.0)),
        None,
    );
    let u_smash = vfx_at(u_smash, 0.14, "wind_curl", (2.0, -30.0), BOOM_FX);
    let u_smash = on_contact(u_smash, "player.hit");

    // **DOWN SMASH — `split_kick`.** Both directions at once, low.
    let d_smash = strike(
        "split_kick",
        "smash_down",
        0.15,
        0.08,
        0.28,
        (0.0, 18.0),
        (36.0, 11.0),
        10,
        104.0,
        1.95,
        Some((0.85, -0.52)),
        None,
    );
    let d_smash = vfx_at(d_smash, 0.15, "sonic_ripple", (0.0, 18.0), BOOM_FX);
    let d_smash = on_contact(d_smash, "player.hit");

    // **NEUTRAL AIR — `air_spin`.** The ball, in the air, around him.
    let n_air = strike(
        "air_spin",
        "air_neutral",
        0.05,
        0.11,
        0.14,
        (0.0, 0.0),
        (24.0, 22.0),
        5,
        64.0,
        1.34,
        Some((0.55, -0.72)),
        None,
    );
    let n_air = vfx_at(n_air, 0.05, "sonic_ripple", (0.0, 0.0), RUSH_FX);
    let n_air = on_contact(n_air, "player.hit");

    // **FORWARD AIR — `homing_cut`.** He arrives at you rather than swinging at
    // where you are.
    let f_air = strike(
        "homing_cut",
        "air_forward",
        0.07,
        0.07,
        0.16,
        (28.0, 0.0),
        (21.0, 17.0),
        7,
        86.0,
        1.65,
        Some((0.95, -0.42)),
        None,
    );
    let f_air = vfx_at(f_air, 0.07, "dash_streak", (28.0, 0.0), RUSH_FX);
    let f_air = on_contact(f_air, "player.hit");

    // **BACK AIR — `trailing_heel`.** What is behind him is behind him because
    // he already passed it.
    let b_air = strike(
        "trailing_heel",
        "air_back",
        0.08,
        0.06,
        0.18,
        (-28.0, 0.0),
        (21.0, 16.0),
        8,
        94.0,
        1.78,
        Some((-0.95, -0.36)),
        None,
    );
    let b_air = vfx_at(b_air, 0.08, "air_slice", (-28.0, 0.0), RUSH_FX);
    let b_air = on_contact(b_air, "player.hit");

    // **UP AIR — `corkscrew`.** Straight up, spinning.
    let u_air = strike(
        "corkscrew",
        "air_up",
        0.06,
        0.08,
        0.15,
        (2.0, -26.0),
        (18.0, 22.0),
        6,
        80.0,
        1.58,
        Some((0.08, -1.0)),
        None,
    );
    let u_air = vfx_at(u_air, 0.06, "wind_curl", (2.0, -26.0), RUSH_FX);
    let u_air = on_contact(u_air, "player.hit");

    // **DOWN AIR — `drill_dive`.** Straight down, and he keeps going.
    let d_air = strike(
        "drill_dive",
        "air_down",
        0.09,
        0.09,
        0.19,
        (0.0, 25.0),
        (19.0, 20.0),
        9,
        104.0,
        1.92,
        Some((0.0, 1.0)),
        None,
    );
    let d_air = vfx_at(d_air, 0.09, "sonic_ripple", (0.0, 25.0), RUSH_FX);
    let d_air = on_contact(d_air, "player.hit");

    // **NEUTRAL — `spin_charge`.** He winds up in place and the release is the
    // only real commitment in his kit.
    let n_b = strike(
        "spin_charge",
        "attack",
        0.18,
        0.10,
        0.28,
        (26.0, 6.0),
        (24.0, 16.0),
        10,
        100.0,
        1.80,
        Some((0.9, -0.40)),
        None,
    );
    // ⚠ **either posture, and it has to be**: gated to the ground, an airborne
    // neutral-B walked the chain past it and found NOTHING — the last candidate
    // for `special_air` is `special` itself. A spin charge in the air is a spin
    // charge; there is nothing about it that needs a floor.
    let n_b = committed_tail(n_b, 0.60, 0.05);
    let n_b = vfx_at(n_b, 0.04, "charge_pulse", (0.0, 8.0), RUSH_FX);
    let n_b = sfx(n_b, 0.04, "player.attack.charge");
    let n_b = vfx_at(n_b, 0.18, "sonic_ripple", (26.0, 6.0), BOOM_FX);
    let n_b = on_contact(n_b, "player.hit");

    // **SIDE — `blue_streak`.** The fastest crossing on the grid, and the tail
    // barely damps: he can still steer out of it, which is the whole difference
    // between him and the engineer's piston.
    let side_b = strike(
        "blue_streak",
        "attack_side",
        0.09,
        0.10,
        0.20,
        (28.0, 2.0),
        (24.0, 18.0),
        9,
        98.0,
        1.82,
        Some((0.95, -0.32)),
        None,
    );
    let side_b = impulse(side_b, 0.09, (860.0, 0.0), ImpulseMode::Set);
    let side_b = committed_tail(side_b, 0.46, 0.45);
    let side_b = vfx_at(side_b, 0.09, "dash_streak", (0.0, 0.0), BOOM_FX);
    let side_b = sfx(side_b, 0.09, "player.dash");
    let side_b = on_contact(side_b, "player.hit");

    // **UP — `spring_launch`. THE RECOVERY.** The springs his own course is full
    // of, carried. The highest lift on the grid and the cheapest landing, which
    // is what the lightest, fastest fighter's way home should be.
    let mut up_b = strike(
        "spring_launch",
        "attack_up",
        0.05,
        0.12,
        0.15,
        (0.0, -12.0),
        (18.0, 30.0),
        6,
        78.0,
        1.55,
        Some((0.10, -1.0)),
        None,
    );
    up_b.landing_lag_s = Some(0.18);
    let up_b = impulse(up_b, 0.05, (0.0, -880.0), ImpulseMode::Set);
    let up_b = committed_tail(up_b, 0.42, 0.40);
    let up_b = vfx_at(up_b, 0.05, "release_ring", (0.0, 16.0), RUSH_FX);
    let up_b = sfx(up_b, 0.05, "player.double_jump");
    let up_b = on_contact(up_b, "player.hit");

    // **DOWN — `ball_drop`.** He curls and drops. Airborne-only: on the ground
    // it would be a worse down smash, and the move is the fall.
    let mut down_b = strike(
        "ball_drop",
        "air_down",
        0.07,
        0.14,
        0.20,
        (0.0, 24.0),
        (20.0, 22.0),
        10,
        108.0,
        1.95,
        Some((0.0, 1.0)),
        None,
    );
    down_b.landing_lag_s = Some(0.26);
    let down_b = impulse(down_b, 0.07, (0.0, 1300.0), ImpulseMode::Set);
    let down_b = vfx_at(down_b, 0.07, "sonic_ripple", (0.0, 12.0), RUSH_FX);
    let down_b = on_contact(down_b, "player.hit");

    // ── 2026-08-16: THE OTHER POSTURE ────────────────────────────────────────
    //
    // Jon: *"A down-b that has special airborne properties should also have an
    // effect on ground. Think of bowser down b. In the air he just does a
    // downward slam, but on the ground, it causes him to jump in an arc and then
    // slam. Specials can have different effects in different contexts that
    // should be ok, and makes for a richer smash game, although in most cases
    // they shouldn't be context dependent."*
    //
    // ⛔ a special gated to ONE posture is not answered in the other — the
    // directional chain walks straight past it to the NEUTRAL special, so a
    // player pressing down-B in the air got the neutral-B. `special_air_down`
    // sits ahead of `special_down` in that chain and has the whole time; this is
    // the two-form move it exists for.
    // **DOWN, ON THE GROUND — `ball_hop`. THE BOWSER SHAPE, and Jon named it**:
    // *"In the air he just does a downward slam, but on the ground, it causes
    // him to jump in an arc and then slam."* With a floor already under him the
    // drop has nowhere to go, so he leaves it first — a short hop, then the same
    // curl. ⚠ the impulse carries him UP and slightly forward; the slam is the
    // same shape as `ball_drop` and lands later for it.
    let ground_down_b = strike(
        "ball_hop",
        "attack_down",
        0.18,
        0.14,
        0.24,
        (0.0, 22.0),
        (20.0, 22.0),
        10,
        108.0,
        1.95,
        Some((0.0, 1.0)),
        None,
    );
    // ⚠ **`Add` for the hop, `Set` for the slam** — the same split George's
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

    SmashRepertoire {
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
        up_special: up_b,
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

    // ⭐⭐ **RETIRED 2026-08-16 — the per-file verb-map test.**
    //
    // Fourteen fighters each carried a copy of it: every bound verb names a move
    // this table defines, and the table binds the whole vocabulary. Both are now
    // unwritable defects rather than tested ones. `SmashRepertoire` owns the verb
    // strings, so there is no string in this file to misspell; it is a struct
    // with no `Default` and no private fields, so a missing or renamed slot is a
    // COMPILE error here. What the fourteen copies stood for — that every press
    // is answered, in every posture it is asked in — is checked once, by
    // `ambition_characters::smash_repertoire`, and by the host ratchet
    // `smash_roster_movesets::report_the_smash_kit_every_selectable_fighter_has`.

    /// **SPEED IS THE WHOLE CHARACTER, and it is checkable.** Every one of his
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

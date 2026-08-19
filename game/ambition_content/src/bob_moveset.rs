//! **Bob's repertoire** — the engineer, and the one who RECEIVES.
//!
//! ⭐ **written 2026-08-16**, beside [`crate::alice_moveset`] and for the same
//! reason: measured before a line of it, Bob was one of four fighters on the
//! grid at **0/16** — no table, no action set, and no unarmed floor reaching his
//! body either. Every press was silence.
//!
//! ## The character, from his own name
//!
//! Alice sends and Bob receives, and the pair's split is the design. Where hers
//! is about getting something across, his is about what happens when it ARRIVES:
//! he is slower to start than anybody on the grid bar the automaton, he commits
//! for longer, and when he connects it is the hardest single hit among the
//! Hall's people. An engineer does not fence. He assembles, and then it is
//! assembled.
//!
//! ```text
//!            reach   jab startup   f-smash damage   the trade
//!   alice     28 px     0.05 s          13          reach and recovery
//!   bob       26 px     0.07 s          16          slow, and it lands
//! ```
//!
//! ⚠ **his effects are the generic vocabulary, deliberately** — no authored FX
//! sheet of his own yet, and Jon's *"It doesn't have to be fancy"* is the licence
//! for `gear_scatter`, `electric_arc`, `steam_vent`, `hit_metal` and `shockwave`.
//! Every one is a row a shipped generic sheet carries.

use ambition_characters::smash_capture::{
    author_pummel, author_standing_grab, author_throw, capture_beat, grab_shell,
    CaptureAttemptParams, CapturePummelParams, CaptureThrowParams, SmashCaptureRepertoire,
};
use ambition_characters::smash_repertoire::{DownSpecial, NeutralSpecial, SmashRepertoire};
use ambition_platformer2d::entity_catalog::{ImpulseMode, MovesetContract};

use ambition_characters::moveset_authoring::{
    committed_tail, impulse, on_contact, sfx, strike, strike_tag, vfx_at,
};

const SHOP_FX: f32 = 0.85;
const RIG_FX: f32 = 1.2;

/// See the module doc. Sixteen presses.
pub fn bob_moveset() -> MovesetContract {
    // **JAB — `tap_test`.** He taps it to hear whether it is sound. Slower than
    // anybody else's jab, which is the whole character in the first press.
    let jab = strike(
        "tap_test",
        "jab",
        0.07,
        0.05,
        0.15,
        (22.0, 0.0),
        (16.0, 13.0),
        3,
        50.0,
        1.05,
        None,
        None,
    );
    let jab = strike_tag(jab, ambition_characters::moveset_prefabs::SLASH_POKE_VFX);
    let jab = vfx_at(jab, 0.07, "hit_metal", (22.0, 0.0), SHOP_FX);
    let jab = on_contact(jab, "player.hit");

    // **FORWARD TILT — `wrench_swing`.** The tool, used as one.
    let f_tilt = strike(
        "wrench_swing",
        "attack_side",
        0.10,
        0.08,
        0.19,
        (28.0, -2.0),
        (20.0, 15.0),
        7,
        78.0,
        1.32,
        Some((1.0, -0.30)),
        None,
    );
    let f_tilt = vfx_at(f_tilt, 0.10, "hit_metal", (28.0, -2.0), SHOP_FX);
    let f_tilt = on_contact(f_tilt, "player.hit");

    // **UP TILT — `pressure_release`.** He opens a valve and it goes up.
    let u_tilt = strike(
        "pressure_release",
        "attack_up",
        0.09,
        0.08,
        0.19,
        (6.0, -26.0),
        (17.0, 22.0),
        6,
        78.0,
        1.34,
        Some((0.12, -1.0)),
        None,
    );
    let u_tilt = vfx_at(u_tilt, 0.09, "steam_vent", (6.0, -26.0), SHOP_FX);
    let u_tilt = on_contact(u_tilt, "player.hit");

    // **DOWN TILT — `shim`.** A wedge, driven in at floor level.
    let d_tilt = strike(
        "shim",
        "attack_down",
        0.09,
        0.06,
        0.18,
        (24.0, 14.0),
        (20.0, 10.0),
        5,
        58.0,
        1.18,
        Some((0.9, -0.32)),
        None,
    );
    let d_tilt = vfx_at(d_tilt, 0.09, "gear_scatter", (24.0, 14.0), SHOP_FX);
    let d_tilt = on_contact(d_tilt, "player.hit");

    // **FORWARD SMASH — `rivet_smash`.** The hardest single hit among the Hall's
    // people, and the longest wind-up to go with it.
    let f_smash = strike(
        "rivet_smash",
        "smash_forward",
        0.21,
        0.09,
        0.32,
        (36.0, -2.0),
        (26.0, 22.0),
        16,
        134.0,
        2.30,
        Some((0.95, -0.45)),
        None,
    );
    let f_smash = vfx_at(f_smash, 0.21, "electric_burst", (36.0, -2.0), RIG_FX);
    let f_smash = sfx(f_smash, 0.21, "player.attack.charge");
    let f_smash = on_contact(f_smash, "player.hit");

    // **UP SMASH — `derrick_lift`.** He raises the frame overhead and lets it
    // settle.
    let u_smash = strike(
        "derrick_lift",
        "smash_up",
        0.19,
        0.10,
        0.30,
        (2.0, -32.0),
        (22.0, 32.0),
        14,
        126.0,
        2.28,
        Some((0.10, -1.0)),
        None,
    );
    let u_smash = vfx_at(u_smash, 0.19, "gear_scatter", (2.0, -32.0), RIG_FX);
    let u_smash = on_contact(u_smash, "player.hit");

    // **DOWN SMASH — `ground_anchor`.** Two bolts, one either side, into the
    // floor.
    let d_smash = strike(
        "ground_anchor",
        "smash_down",
        0.20,
        0.09,
        0.32,
        (0.0, 19.0),
        (38.0, 13.0),
        13,
        118.0,
        2.05,
        Some((0.8, -0.55)),
        None,
    );
    let d_smash = vfx_at(d_smash, 0.20, "shockwave", (0.0, 19.0), RIG_FX);
    let d_smash = on_contact(d_smash, "player.hit");

    // **NEUTRAL AIR — `loose_bearing`.** Something comes off and goes round him.
    let n_air = strike(
        "loose_bearing",
        "air_neutral",
        0.08,
        0.10,
        0.19,
        (0.0, 0.0),
        (25.0, 22.0),
        6,
        70.0,
        1.38,
        Some((0.55, -0.72)),
        None,
    );
    let n_air = vfx_at(n_air, 0.08, "gear_scatter", (0.0, 0.0), SHOP_FX);
    let n_air = on_contact(n_air, "player.hit");

    // **FORWARD AIR — `swing_arm`.** A long arc from the shoulder.
    let f_air = strike(
        "swing_arm",
        "air_forward",
        0.10,
        0.08,
        0.21,
        (28.0, -4.0),
        (22.0, 18.0),
        9,
        96.0,
        1.75,
        Some((0.95, -0.42)),
        None,
    );
    let f_air = vfx_at(f_air, 0.10, "hit_metal", (28.0, -4.0), SHOP_FX);
    let f_air = on_contact(f_air, "player.hit");

    // **BACK AIR — `counterweight`.** He swings the mass the other way and it
    // takes whoever was there.
    let b_air = strike(
        "counterweight",
        "air_back",
        0.11,
        0.07,
        0.22,
        (-28.0, -2.0),
        (22.0, 18.0),
        10,
        104.0,
        1.90,
        Some((-0.95, -0.38)),
        None,
    );
    let b_air = vfx_at(b_air, 0.11, "hit_metal", (-28.0, -2.0), SHOP_FX);
    let b_air = on_contact(b_air, "player.hit");

    // **UP AIR — `jack_stand`.** Straight up, on the hard part.
    let u_air = strike(
        "jack_stand",
        "air_up",
        0.09,
        0.08,
        0.19,
        (2.0, -26.0),
        (19.0, 23.0),
        8,
        86.0,
        1.64,
        Some((0.08, -1.0)),
        None,
    );
    let u_air = vfx_at(u_air, 0.09, "electric_arc", (2.0, -26.0), SHOP_FX);
    let u_air = on_contact(u_air, "player.hit");

    // **DOWN AIR — `pile_driver`.** He puts his whole weight through it.
    let d_air = strike(
        "pile_driver",
        "air_down",
        0.13,
        0.07,
        0.24,
        (2.0, 25.0),
        (20.0, 20.0),
        11,
        114.0,
        2.05,
        Some((0.0, 1.0)),
        None,
    );
    let d_air = vfx_at(d_air, 0.13, "shockwave", (2.0, 25.0), SHOP_FX);
    let d_air = on_contact(d_air, "player.hit");

    // **NEUTRAL — `rivet_gun`.** Held down and driven home. His longest active
    // window: it is not one hit, it is the tool running.
    let n_b = strike(
        "rivet_gun",
        "attack",
        0.20,
        0.14,
        0.30,
        (30.0, -2.0),
        (26.0, 18.0),
        12,
        112.0,
        2.00,
        Some((0.92, -0.44)),
        None,
    );
    let n_b = committed_tail(n_b, 0.70, 0.05);
    let n_b = vfx_at(n_b, 0.20, "electric_burst", (30.0, -2.0), RIG_FX);
    let n_b = sfx(n_b, 0.20, "player.directional_special");
    let n_b = on_contact(n_b, "player.hit");

    // **SIDE — `piston_charge`.** He is committed the instant it fires, and the
    // tail damps to nothing: an engineer's dash has no take-backs.
    let side_b = strike(
        "piston_charge",
        "attack_side",
        0.16,
        0.10,
        0.28,
        (28.0, 0.0),
        (24.0, 20.0),
        12,
        114.0,
        2.05,
        Some((0.95, -0.35)),
        None,
    );
    let side_b = impulse(side_b, 0.16, (620.0, 0.0), ImpulseMode::Set);
    let side_b = committed_tail(side_b, 0.66, 0.0);
    let side_b = vfx_at(side_b, 0.16, "steam_vent", (-16.0, 0.0), RIG_FX);
    let side_b = sfx(side_b, 0.16, "player.dash");
    let side_b = on_contact(side_b, "player.hit");

    // **UP — `steam_lift`. THE RECOVERY.** Boiler pressure, spent all at once.
    // It goes higher than Alice's curve and costs more to land, which is the
    // same bargain his whole kit makes.
    let mut up_b = strike(
        "steam_lift",
        "attack_up",
        0.09,
        0.12,
        0.22,
        (0.0, -12.0),
        (21.0, 32.0),
        8,
        90.0,
        1.68,
        Some((0.10, -1.0)),
        None,
    );
    up_b.landing_lag_s = Some(0.34);
    let up_b = impulse(up_b, 0.09, (0.0, -800.0), ImpulseMode::Set);
    let up_b = committed_tail(up_b, 0.54, 0.10);
    let up_b = vfx_at(up_b, 0.09, "steam_vent", (0.0, 18.0), RIG_FX);
    let up_b = sfx(up_b, 0.09, "player.fly.start");
    let up_b = on_contact(up_b, "player.hit");

    // **DOWN — `bulkhead_drop`.** He drops a plate. Grounded-only, because the
    // move is that there is a floor to drop it onto.
    let down_b = strike(
        "bulkhead_drop",
        "attack_down",
        0.18,
        0.10,
        0.32,
        (0.0, 20.0),
        (34.0, 14.0),
        12,
        104.0,
        1.80,
        Some((0.7, -0.66)),
        None,
    );
    let down_b = committed_tail(down_b, 0.70, 0.0);
    let down_b = vfx_at(down_b, 0.18, "shockwave", (0.0, 20.0), RIG_FX);
    let down_b = vfx_at(down_b, 0.18, "landing_puff", (0.0, 22.0), SHOP_FX);
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
    // **DOWN, IN THE AIR — `bulkhead_dive`.** He does not drop the plate; he
    // rides it down.
    let mut air_down_b = strike(
        "bulkhead_dive",
        "air_down",
        0.12,
        0.10,
        0.26,
        (0.0, 24.0),
        (22.0, 22.0),
        11,
        106.0,
        1.80,
        Some((0.0, 1.0)),
        None,
    );
    air_down_b.landing_lag_s = Some(0.34);
    let air_down_b = impulse(air_down_b, 0.12, (0.0, 1300.0), ImpulseMode::Set);
    let air_down_b = vfx_at(air_down_b, 0.12, "shockwave", (0.0, 22.0), SHOP_FX);
    let air_down_b = on_contact(air_down_b, "player.hit");
    // **BOB'S CAPTURE KIT.** Heavy and slow: the longest reach and the hardest single
    // pummel, paid for with the worst startup and recovery. One beat, and it hurts.
    // ⚠ the grab draws `attack`, not `grab`: these sheets publish no `grab` row,
    // and each table's own `every_clip_names_a_row_..._sheet_carries` guard says
    // so. `ClipBinding`'s fallbacks would have covered it at runtime, but a move
    // that NAMES a row nobody publishes is a lie the guard is right to refuse.
    let grab = author_standing_grab(
        grab_shell("bob_grab", "attack", 0.09, 0.06, 0.24),
        CaptureAttemptParams {
            offset: (12.0, 1.0),
            half_extents: (22.0, 17.0),
            hold_offset: (13.0, 3.0),
        },
    );
    let pummel = author_pummel(
        capture_beat("bob_pummel", "attack", 0.28),
        0.12,
        CapturePummelParams { damage: 5 },
    );
    let forward_throw = author_throw(
        capture_beat("bob_fthrow", "attack", 0.3),
        0.16,
        CaptureThrowParams {
            damage: 10,
            knockback: 132.0,
            knockback_growth: 1.8,
            launch_dir: (0.7, -0.7),
        },
    );

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
        // ⭐ **AUTHORED 2026-08-19, at Jon's ask that every fighter in the smash
        // roster have a grab.** The transitional `None` is gone: capture was
        // proven on George and the Pirate Admiral, and the whole point of
        // proving it was to stop being the only two.
        //
        // ⚠ the VALUES are per character on purpose. A roster whose grabs are
        // twelve copies of one number set is one grab wearing twelve names.
        capture: SmashCaptureRepertoire {
            grab,
            pummel,
            forward_throw,
            // ⛔ back/up/down stay `None` and that is still the authored answer,
            // not an omission: an unauthored throw does NOTHING rather than
            // falling back to a pummel, which tells a player this fighter has
            // none instead of telling them it has a bad one.
            back_throw: None,
            up_throw: None,
            down_throw: None,
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

    /// **He commits for longer than she does**, on every press they both have.
    /// The pair's other half is asserted in `alice_moveset`; this is the axis
    /// that is his.
    #[test]
    fn bob_is_slower_to_start_than_alice_on_every_shared_press() {
        let bob = bob_moveset();
        let alice = crate::alice_moveset::alice_moveset();
        let startup = |set: &MovesetContract, verb: &str| {
            set.move_for_verb(verb)
                .unwrap_or_else(|| panic!("{verb} is bound"))
                .windows
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
        for verb in ["attack", "attack_forward", "smash_forward", "attack_air"] {
            assert!(
                startup(&bob, verb) > startup(&alice, verb),
                "`{verb}` comes out at least as fast for the engineer as for the \
                 cryptographer, so the pair is one table twice"
            );
        }
    }
}

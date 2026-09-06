//! Bob's repertoire — the engineer, and the one who RECEIVES.
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
//! Every one is a row a shipped generic sheet carries.

use ambition_characters::moveset_authoring::Strike;
use ambition_characters::smash_capture::{
    author_pummel, author_standing_grab, author_throw, capture_beat, grab_shell,
    CaptureAttemptParams, CaptureCues, CapturePummelParams, CaptureThrowParams,
    SmashCaptureRepertoire,
};
use ambition_characters::smash_repertoire::{
    DownSpecial, NeutralSpecial, SmashRepertoire, UpSpecial,
};
use ambition_platformer2d::entity_catalog::{AutolinkVolume, ImpulseMode, MovesetContract};

use ambition_characters::moveset_authoring::{
    committed_tail, impulse, multihit, on_contact, sfx, strike, strike_tag, vfx_at, Pulse,
};

const SHOP_FX: f32 = 0.85;
const RIG_FX: f32 = 1.2;

/// See the module doc. Sixteen presses.
pub fn bob_moveset() -> MovesetContract {
    // JAB — `tap_test`. He taps it to hear whether it is sound. Slower than
    // anybody else's jab, which is the whole character in the first press.
    let jab = strike(Strike {
        id: "tap_test",
        clip: "jab",
        startup_s: 0.07,
        active_s: 0.05,
        recover_s: 0.15,
        offset: (22.0, 0.0),
        half_extents: (16.0, 13.0),
        damage: 3,
        knockback: 50.0,
        knockback_growth: 1.05,
        launch_dir: None,
        on_hit: None,
    });
    let jab = strike_tag(jab, ambition_characters::moveset_prefabs::SLASH_POKE_VFX);
    let jab = vfx_at(jab, 0.07, "hit_metal", (22.0, 0.0), SHOP_FX);
    let jab = on_contact(jab, "player.hit");

    // FORWARD TILT — `wrench_swing`. The tool, used as one.
    let f_tilt = strike(Strike {
        id: "wrench_swing",
        clip: "attack_side",
        startup_s: 0.10,
        active_s: 0.08,
        recover_s: 0.19,
        offset: (28.0, -2.0),
        half_extents: (20.0, 15.0),
        damage: 7,
        knockback: 78.0,
        knockback_growth: 1.32,
        launch_dir: Some((1.0, -0.30)),
        on_hit: None,
    });
    let f_tilt = vfx_at(f_tilt, 0.10, "hit_metal", (28.0, -2.0), SHOP_FX);
    let f_tilt = on_contact(f_tilt, "player.hit");

    // UP TILT — `pressure_release`. He opens a valve and it goes up.
    let u_tilt = strike(Strike {
        id: "pressure_release",
        clip: "attack_up",
        startup_s: 0.09,
        active_s: 0.08,
        recover_s: 0.19,
        offset: (6.0, -26.0),
        half_extents: (17.0, 22.0),
        damage: 6,
        knockback: 78.0,
        knockback_growth: 1.34,
        launch_dir: Some((0.12, -1.0)),
        on_hit: None,
    });
    let u_tilt = vfx_at(u_tilt, 0.09, "steam_vent", (6.0, -26.0), SHOP_FX);
    let u_tilt = on_contact(u_tilt, "player.hit");

    // DOWN TILT — `shim`. A wedge, driven in at floor level.
    let d_tilt = strike(Strike {
        id: "shim",
        clip: "attack_down",
        startup_s: 0.09,
        active_s: 0.06,
        recover_s: 0.18,
        offset: (24.0, 14.0),
        half_extents: (20.0, 10.0),
        damage: 5,
        knockback: 58.0,
        knockback_growth: 1.18,
        launch_dir: Some((0.9, -0.32)),
        on_hit: None,
    });
    let d_tilt = vfx_at(d_tilt, 0.09, "gear_scatter", (24.0, 14.0), SHOP_FX);
    let d_tilt = on_contact(d_tilt, "player.hit");

    // FORWARD SMASH — `rivet_smash`. The hardest single hit among the Hall's
    // people, and the longest wind-up to go with it.
    let f_smash = strike(Strike {
        id: "rivet_smash",
        clip: "smash_forward",
        startup_s: 0.21,
        active_s: 0.09,
        recover_s: 0.32,
        offset: (36.0, -2.0),
        half_extents: (26.0, 22.0),
        damage: 16,
        knockback: 134.0,
        knockback_growth: 2.30,
        launch_dir: Some((0.95, -0.45)),
        on_hit: None,
    });
    let f_smash = vfx_at(f_smash, 0.21, "electric_burst", (36.0, -2.0), RIG_FX);
    let f_smash = sfx(f_smash, 0.21, "player.attack.charge");
    let f_smash = on_contact(f_smash, "player.hit");

    // UP SMASH — `derrick_lift`. He raises the frame overhead and lets it
    // settle.
    let u_smash = strike(Strike {
        id: "derrick_lift",
        clip: "smash_up",
        startup_s: 0.19,
        active_s: 0.10,
        recover_s: 0.30,
        offset: (2.0, -32.0),
        half_extents: (22.0, 32.0),
        damage: 14,
        knockback: 126.0,
        knockback_growth: 2.28,
        launch_dir: Some((0.10, -1.0)),
        on_hit: None,
    });
    let u_smash = vfx_at(u_smash, 0.19, "gear_scatter", (2.0, -32.0), RIG_FX);
    let u_smash = on_contact(u_smash, "player.hit");

    // DOWN SMASH — `ground_anchor`. Two bolts, one either side, into the
    // floor.
    let d_smash = strike(Strike {
        id: "ground_anchor",
        clip: "smash_down",
        startup_s: 0.20,
        active_s: 0.09,
        recover_s: 0.32,
        offset: (0.0, 19.0),
        half_extents: (38.0, 13.0),
        damage: 13,
        knockback: 118.0,
        knockback_growth: 2.05,
        launch_dir: Some((0.8, -0.55)),
        on_hit: None,
    });
    let d_smash = vfx_at(d_smash, 0.20, "shockwave", (0.0, 19.0), RIG_FX);
    let d_smash = on_contact(d_smash, "player.hit");

    // NEUTRAL AIR — `loose_bearing`. Something comes off and goes round him.
    let n_air = strike(Strike {
        id: "loose_bearing",
        clip: "air_neutral",
        startup_s: 0.08,
        active_s: 0.10,
        recover_s: 0.19,
        offset: (0.0, 0.0),
        half_extents: (25.0, 22.0),
        damage: 6,
        knockback: 70.0,
        knockback_growth: 1.38,
        launch_dir: Some((0.55, -0.72)),
        on_hit: None,
    });
    let n_air = vfx_at(n_air, 0.08, "gear_scatter", (0.0, 0.0), SHOP_FX);
    let n_air = on_contact(n_air, "player.hit");

    // FORWARD AIR — `swing_arm`. A long arc from the shoulder.
    let f_air = strike(Strike {
        id: "swing_arm",
        clip: "air_forward",
        startup_s: 0.10,
        active_s: 0.08,
        recover_s: 0.21,
        offset: (28.0, -4.0),
        half_extents: (22.0, 18.0),
        damage: 9,
        knockback: 96.0,
        knockback_growth: 1.75,
        launch_dir: Some((0.95, -0.42)),
        on_hit: None,
    });
    let f_air = vfx_at(f_air, 0.10, "hit_metal", (28.0, -4.0), SHOP_FX);
    let f_air = on_contact(f_air, "player.hit");

    // BACK AIR — `counterweight`. He swings the mass the other way and it
    // takes whoever was there.
    let b_air = strike(Strike {
        id: "counterweight",
        clip: "air_back",
        startup_s: 0.11,
        active_s: 0.07,
        recover_s: 0.22,
        offset: (-28.0, -2.0),
        half_extents: (22.0, 18.0),
        damage: 10,
        knockback: 104.0,
        knockback_growth: 1.90,
        launch_dir: Some((-0.95, -0.38)),
        on_hit: None,
    });
    let b_air = vfx_at(b_air, 0.11, "hit_metal", (-28.0, -2.0), SHOP_FX);
    let b_air = on_contact(b_air, "player.hit");

    // UP AIR — `jack_stand`. Straight up, on the hard part.
    let u_air = strike(Strike {
        id: "jack_stand",
        clip: "air_up",
        startup_s: 0.09,
        active_s: 0.08,
        recover_s: 0.19,
        offset: (2.0, -26.0),
        half_extents: (19.0, 23.0),
        damage: 8,
        knockback: 86.0,
        knockback_growth: 1.64,
        launch_dir: Some((0.08, -1.0)),
        on_hit: None,
    });
    let u_air = vfx_at(u_air, 0.09, "electric_arc", (2.0, -26.0), SHOP_FX);
    let u_air = on_contact(u_air, "player.hit");

    // DOWN AIR — `pile_driver`. He puts his whole weight through it.
    let d_air = strike(Strike {
        id: "pile_driver",
        clip: "air_down",
        startup_s: 0.13,
        active_s: 0.07,
        recover_s: 0.24,
        offset: (2.0, 25.0),
        half_extents: (20.0, 20.0),
        damage: 11,
        knockback: 114.0,
        knockback_growth: 2.05,
        launch_dir: Some((0.0, 1.0)),
        on_hit: None,
    });
    let d_air = vfx_at(d_air, 0.13, "shockwave", (2.0, 25.0), SHOP_FX);
    let d_air = on_contact(d_air, "player.hit");

    // NEUTRAL — `rivet_gun`. Held down and driven home. His longest active
    // window: it is not one hit, it is the tool running.
    let n_b = strike(Strike {
        id: "rivet_gun",
        clip: "attack",
        startup_s: 0.20,
        active_s: 0.14,
        recover_s: 0.30,
        offset: (30.0, -2.0),
        half_extents: (26.0, 18.0),
        damage: 12,
        knockback: 112.0,
        knockback_growth: 2.00,
        launch_dir: Some((0.92, -0.44)),
        on_hit: None,
    });
    // ⭐⭐ AND NOW IT ACTUALLY RUNS. The comment above has always said "it is not
    // one hit, it is the tool running" — and the move was ONE `strike` with a
    // single 0.14s Active window, which the runtime's re-hit rule lands EXACTLY
    // ONCE. ⇒ The `Pulse` doc says so in as many words: "a multi-hit that
    // authored one long window, or windows that touch, lands exactly once." So
    // the comment named the precise property the engine refuses.
    //
    // ⭐ THE FIX IS THE ONE THE ENGINE ASKS FOR: separated windows, which is what
    // `multihit` builds. Oiler's `convergence` carries the same explanation
    // beside it — that move genuinely multi-hits BECAUSE of the gap — so the
    // vocabulary and the precedent were both already here.
    //
    // ⛔ THE FINISHER IS UNTOUCHED. Every number on the strike above is as it
    // was; the pulses go IN FRONT at 2 chip each, which is the shape a multi-hit
    // has in this repository — "the move is paid for by its finisher, and a
    // multi-hit whose pulses hurt is a better move than its own ending."
    //
    // ⛔ THE ANCHOR'S x IS ZERO. `autolink_anchor_world` mirrors it with his
    // facing, so a non-zero x makes the hold point depend on which way he is
    // looking. A tool that pulls its work toward the man holding it does not
    // care which way he turned.
    let n_b = multihit(
        n_b,
        3,
        Pulse {
            // A little tighter than the finisher: the work is held where the
            // tool is, not where the swing ends.
            offset: (26.0, -2.0),
            half_extents: (22.0, 16.0),
            damage: 2,
            // Separated, because touching windows are one hit wearing three
            // windows' timing.
            active_s: 0.030,
            gap_s: 0.028,
            autolink: AutolinkVolume {
                anchor: (0.0, -4.0),
                // He is planted while it runs — there is no motion of his own to
                // hand on, so the hold is entirely the correction's.
                carry: 0.0,
                pull: 19.0,
                max_speed: 900.0,
            },
        },
    );
    let n_b = committed_tail(n_b, 0.70, 0.05);
    let n_b = vfx_at(n_b, 0.20, "electric_burst", (30.0, -2.0), RIG_FX);
    let n_b = sfx(n_b, 0.20, "player.directional_special");
    let n_b = on_contact(n_b, "player.hit");

    // SIDE — `piston_charge`. He is committed the instant it fires, and the
    // tail damps to nothing: an engineer's dash has no take-backs.
    let side_b = strike(Strike {
        id: "piston_charge",
        clip: "attack_side",
        startup_s: 0.16,
        active_s: 0.10,
        recover_s: 0.28,
        offset: (28.0, 0.0),
        half_extents: (24.0, 20.0),
        damage: 12,
        knockback: 114.0,
        knockback_growth: 2.05,
        launch_dir: Some((0.95, -0.35)),
        on_hit: None,
    });
    let side_b = impulse(side_b, 0.16, (620.0, 0.0), ImpulseMode::Set);
    let side_b = committed_tail(side_b, 0.66, 0.0);
    let side_b = vfx_at(side_b, 0.16, "steam_vent", (-16.0, 0.0), RIG_FX);
    let side_b = sfx(side_b, 0.16, "player.dash");
    let side_b = on_contact(side_b, "player.hit");

    // UP — `steam_lift`. THE RECOVERY. Boiler pressure, spent all at once.
    // It goes higher than Alice's curve and costs more to land, which is the
    // same bargain his whole kit makes.
    let mut up_b = strike(Strike {
        id: "steam_lift",
        clip: "attack_up",
        startup_s: 0.09,
        active_s: 0.12,
        recover_s: 0.22,
        offset: (0.0, -12.0),
        half_extents: (21.0, 32.0),
        damage: 8,
        knockback: 90.0,
        knockback_growth: 1.68,
        launch_dir: Some((0.10, -1.0)),
        on_hit: None,
    });
    up_b.landing_lag_s = Some(0.34);
    let up_b = impulse(up_b, 0.09, (0.0, -800.0), ImpulseMode::Set);
    let up_b = committed_tail(up_b, 0.54, 0.10);
    let up_b = vfx_at(up_b, 0.09, "steam_vent", (0.0, 18.0), RIG_FX);
    let up_b = sfx(up_b, 0.09, "player.fly.start");
    let up_b = on_contact(up_b, "player.hit");

    // DOWN — `bulkhead_drop`. He drops a plate. Grounded-only, because the
    // move is that there is a floor to drop it onto.
    let down_b = strike(Strike {
        id: "bulkhead_drop",
        clip: "attack_down",
        startup_s: 0.18,
        active_s: 0.10,
        recover_s: 0.32,
        offset: (0.0, 20.0),
        half_extents: (34.0, 14.0),
        damage: 12,
        knockback: 104.0,
        knockback_growth: 1.80,
        launch_dir: Some((0.7, -0.66)),
        on_hit: None,
    });
    // ⭐⭐ AND THE PLATE STAYS. His comment has always said "he DROPS a plate" and
    // "the move is that there is a floor to drop it ONTO" — an object, described
    // in a move that only ever slammed. ⇒ Unlike the ninja's counter ring, that
    // was not a LIE: `shockwave` and `landing_puff` are honest cues for a slam,
    // and the sentence reads as an animation. But it is the sentence a launch pad
    // was already waiting inside.
    //
    // ⭐ THE SLAM IS UNTOUCHED — every number above is as it was, and the plate is
    // an event ADDED to the move. Same shape as the mine on the Polygon's
    // down-smash: the swing still pays for itself and the object is the bonus.
    //
    // ⭐⭐ IT THROWS ANYBODY, AND THAT IS THE ROW IT PROVES. The campaign asks for
    // "a fighter can create a persistent world actuator ANOTHER fighter interacts
    // with" — so a plate that served only Bob would be a second recovery wearing
    // an object's clothes. ⇒ Three uses, eight seconds, and whoever steps on it
    // goes up: his opponent gets it too, and an engineer who leaves a hazard on
    // the floor and forgets about it is the joke landing correctly.
    let down_b = ambition_characters::smash_spring::author_place_spring(
        down_b,
        // The frame the plate meets the floor — the same instant the slam's
        // shockwave and landing puff fire, so the object appears where the
        // impact was drawn.
        0.18,
        ambition_characters::smash_spring::PlaceSpringParams {
            // Up is NEGATIVE y. Hard enough to be a real reposition and short of
            // his own `steam_lift`, so the plate is a tool rather than a better
            // recovery than his recovery.
            //
            // ⛔ I WROTE THAT SENTENCE AND THEN AUTHORED 860 AGAINST HIS 800, and
            // the guard below caught it in the same commit — which is the ideal
            // case and worth leaving visible. ⇒ The number is not the point; the
            // RELATIONSHIP is, so the guard asserts it against `steam_lift`'s
            // impulse rather than against a constant nobody would re-derive.
            launch: (0.0, -720.0),
            // A plate, not a platform: wide enough to step on and thin enough to
            // miss.
            half_extents: (26.0, 6.0),
            // ⚠ SHORT. A plate that outlived the exchange it was dropped in would
            // be terrain he authored, and terrain is somebody else's authority.
            lifetime_s: 8.0,
            uses: 3,
            // At his feet, where the slam landed.
            offset: (0.0, 20.0),
            // ⛔ SO THE OTHER PLAYER SEES THE PLATE ARRIVE. It draws nothing of
            // its own — see `PlaceSpringParams::vfx`.
            vfx: "steam_vent".to_string(),
        },
    );
    let down_b = committed_tail(down_b, 0.70, 0.0);
    let down_b = vfx_at(down_b, 0.18, "shockwave", (0.0, 20.0), RIG_FX);
    let down_b = vfx_at(down_b, 0.18, "landing_puff", (0.0, 22.0), SHOP_FX);
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
    // DOWN, IN THE AIR — `bulkhead_dive`. He does not drop the plate; he
    // rides it down.
    let mut air_down_b = strike(Strike {
        id: "bulkhead_dive",
        clip: "air_down",
        startup_s: 0.12,
        active_s: 0.10,
        recover_s: 0.26,
        offset: (0.0, 24.0),
        half_extents: (22.0, 22.0),
        damage: 11,
        knockback: 106.0,
        knockback_growth: 1.80,
        launch_dir: Some((0.0, 1.0)),
        on_hit: None,
    });
    air_down_b.landing_lag_s = Some(0.34);
    let air_down_b = impulse(air_down_b, 0.12, (0.0, 1300.0), ImpulseMode::Set);
    let air_down_b = vfx_at(air_down_b, 0.12, "shockwave", (0.0, 22.0), SHOP_FX);
    let air_down_b = on_contact(air_down_b, "player.hit");
    // BOB'S CAPTURE KIT. Heavy and slow: the longest reach and the hardest single
    // pummel, paid for with the worst startup and recovery. One beat, and it hurts.
    // the grab draws `attack`, not `grab`: these sheets publish no `grab` row,
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

    let back_throw = author_throw(
        capture_beat("bob_bthrow", "attack", 0.32),
        0.17,
        CaptureThrowParams {
            damage: 11,
            knockback: 142.56,
            knockback_growth: 1.89,
            launch_dir: (-1.0, -0.43),
        },
    );

    let up_throw = author_throw(
        capture_beat("bob_uthrow", "attack", 0.31),
        0.16,
        CaptureThrowParams {
            damage: 10,
            knockback: 137.28,
            knockback_growth: 1.84,
            launch_dir: (0.0, -1.0),
        },
    );

    let down_throw = author_throw(
        capture_beat("bob_dthrow", "attack", 0.33),
        0.17,
        CaptureThrowParams {
            damage: 8,
            knockback: 97.68,
            knockback_growth: 1.44,
            launch_dir: (0.28, -0.92),
        },
    );

    SmashRepertoire {
        taunt: ambition_characters::moveset_authoring::taunt("bob_taunt", 0.9),
        dash_attack: ambition_characters::moveset_authoring::dash_attack(
            "bob_dash_attack",
            ambition_characters::moveset_authoring::DashAttackShape::GENRE,
            9,
            97.5,
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

    /// He commits for longer than she does, on every press they both have.
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

    /// ⛔⛔ "IT IS NOT ONE HIT, IT IS THE TOOL RUNNING" — the comment on his
    /// neutral, and for a long time it was false in the most specific way
    /// possible: the move was ONE `strike` with a single contiguous active
    /// window, which is exactly what the runtime's re-hit rule lands ONCE.
    /// `Pulse`'s own doc says so — *"a multi-hit that authored one long window,
    /// or windows that touch, lands exactly once."* ⇒ The sentence named the
    /// property the engine refuses.
    #[test]
    fn his_rivet_gun_runs_rather_than_landing_once() {
        let set = bob_moveset();
        let gun = set
            .moves
            .iter()
            .find(|m| m.id == "rivet_gun")
            .expect("his neutral special");

        let mut hitting: Vec<(f32, f32)> = gun
            .windows
            .iter()
            .filter(|w| w.volumes.iter().any(|v| v.damage > 0))
            .map(|w| (w.start_s, w.end_s))
            .collect();
        hitting.sort_by(|a, b| a.0.total_cmp(&b.0));
        assert!(
            hitting.len() >= 3,
            "a tool that runs has more than {} hitting window(s)",
            hitting.len()
        );

        // ⛔⛔ THE GAPS ARE THE MOVE, NOT THE COUNT. Three windows that TOUCH
        // land once, exactly as one long window does — so this asserts the
        // separation rather than the number of windows, which is the thing the
        // runtime actually reads.
        for pair in hitting.windows(2) {
            let gap = pair[1].0 - pair[0].1;
            assert!(
                gap > 0.0,
                "windows {:?} and {:?} touch, so the runtime hands the hit set \
                 forward and the tool lands once",
                pair[0],
                pair[1]
            );
        }

        // ⭐ AND THE PULSES HOLD. A multi-hit whose intermediate hits launch is
        // a move that throws its victim out of its own later windows.
        let holding = gun
            .windows
            .iter()
            .flat_map(|w| w.volumes.iter())
            .filter(|v| v.reaction.is_some())
            .count();
        assert!(holding >= 3, "only {holding} of the pulses hold their victim");

        // ⛔ THE FINISHER IS UNTOUCHED — this was an addition, not a rebalance.
        assert!(
            gun.windows
                .iter()
                .flat_map(|w| w.volumes.iter())
                .any(|v| v.reaction.is_none() && v.damage == 12),
            "the rivet no longer drives home at its authored 12"
        );
    }

    /// ⛔⛔ THE PLATE IS AN ADDITION AND THE SLAM IS UNTOUCHED — both halves in one
    /// test, because they are one claim. A guard that only found the plate would
    /// pass against a down-B that had quietly lost its hitbox to make room, which
    /// is the change this move was authored to avoid.
    #[test]
    fn his_bulkhead_drop_still_slams_and_now_leaves_the_plate_it_names() {
        let set = bob_moveset();
        let drop = set
            .moves
            .iter()
            .find(|m| m.id == "bulkhead_drop")
            .expect("his grounded down special");

        // The slam, unchanged.
        assert!(
            drop.windows
                .iter()
                .flat_map(|w| w.volumes.iter())
                .any(|v| v.damage == 12),
            "the slam lost its authored damage"
        );

        let plate: ambition_platformer2d::characters::smash_spring::PlaceSpringParams = drop
            .events
            .iter()
            .find_map(|event| match &event.kind {
                ambition_platformer2d::entity_catalog::MoveEventKind::Effect(effect)
                    if effect.key
                        == ambition_platformer2d::characters::smash_spring::PLACE_SPRING =>
                {
                    effect.params.hydrate().ok()
                }
                _ => None,
            })
            .expect("he drops a plate, which his comment has always said");

        // ⭐ IT THROWS, AND UPWARD. Up is NEGATIVE y everywhere in this codebase,
        // and a plate that launched DOWN would be a hole.
        assert!(plate.launch.1 < 0.0, "the plate throws downward: {:?}", plate.launch);
        assert!(plate.uses > 0, "a plate nobody can use is an invisible object");

        // ⛔ SHORT-LIVED, and the bound is the point rather than the number: a
        // plate that outlived the exchange it was dropped in would be TERRAIN a
        // fighter authored, and terrain is somebody else's authority.
        assert!(
            plate.lifetime_s <= 12.0,
            "the plate lasts {}s, which is stage geometry rather than a move",
            plate.lifetime_s
        );

        // ⚠ AND IT MUST NOT OUT-LAUNCH HIS OWN RECOVERY, or the down-B is a
        // better `steam_lift` that also hits.
        let lift = set
            .moves
            .iter()
            .find(|m| m.id == "steam_lift")
            .expect("his recovery");
        let rise = lift
            .events
            .iter()
            .find_map(|event| match &event.kind {
                ambition_platformer2d::entity_catalog::MoveEventKind::Impulse { local, .. } => {
                    Some(local.1.abs())
                }
                _ => None,
            })
            .unwrap_or(f32::INFINITY);
        assert!(
            plate.launch.1.abs() < rise,
            "the plate ({}) throws harder than his recovery ({rise})",
            plate.launch.1.abs(),
        );
    }
}

//! Oiler's authored Smash repertoire.
//!
//! Oiler's moves favor wide active windows: every striking move meets
//! [`TOLERANCE_S`], while knockback growth stays within the normal tolerance band
//! except for the forward smash. VFX rows normally derive their cue name; the oil
//! geyser stream names its `.loop` cue explicitly with `vfx_cued`.

use ambition_entity_catalog::authoring::Strike;
use ambition_entity_catalog::authoring::{SLASH_ARC_VFX, SLASH_POKE_VFX};
use ambition_entity_catalog::smash_capture::{
    author_pummel, author_standing_grab, author_throw, capture_beat, grab_shell,
    CaptureAttemptParams, CaptureCues, CapturePummelParams, CaptureThrowParams,
    SmashCaptureRepertoire,
};
use ambition_entity_catalog::smash_repertoire::{
    DownSpecial, NeutralSpecial, SmashRepertoire, UpSpecial,
};
use ambition_entity_catalog::{
    HitVolume, ImpulseMode, MoveSpec, MoveWindow, MovesetContract, VolumeShape, WindowTag,
};

use ambition_entity_catalog::authoring::{
    committed_tail, impulse, on_contact, strike, strike_tag, vfx, vfx_cued,
};

/// The tolerance band: the least time any Oiler move keeps a hitbox in the
/// world, summed over its active windows.
///
/// This defines the character. Retune within the band, never below it: a
/// mechanic whose windows closed as fast as a goblin's would be a slower
/// goblin.
pub const TOLERANCE_S: f32 = 0.10;

/// The one bolt torqued to spec. No move but the forward smash may grow
/// harder than this with the victim's damage.
pub const WITHIN_TOLERANCE_GROWTH: f32 = 2.10;

/// And what the forward smash grows at instead. The gap between the two is the
/// whole reason Oiler has to land a specific move to take a stock.
pub const TORQUE_GROWTH: f32 = 3.30;

/// The rise the geyser commands, engine units per second against gravity.
///
/// Authored as a speed and applied with [`ImpulseMode::Set`], so a falling
/// Oiler gets the same climb as a standing one. An additive impulse would be
/// weakest when he needs it most.
pub const GEYSER_SPEED: f32 = 980.0;

/// When the column arrives — after a windup you can see and hear (the ground
/// swells first: `oil_geyser_emerge`).
pub const GEYSER_AT_S: f32 = 0.22;

/// When the move lets go. `the_geyser_is_a_save_and_not_a_flight` checks the
/// arithmetic.
pub const GEYSER_ENDS_S: f32 = 1.20;

/// See the module doc. Sixteen moves: the genre's standard verb map plus four
/// specials.
pub fn oiler_moveset() -> MovesetContract {
    // ── the ground game: a spanner at arm's length ────────────────────────────
    //
    // Every clip name here is a row the rig publishes. The structural fallback
    // chain (`attack_side` → `attack` → `slash` → `idle`) is what a move falls
    // back to when a row is missing.
    //
    // The up and down families share one row each. The sheet has one upward
    // swing, so the up-tilt and up-smash both draw it.

    // A knuckle-rap with the wrench still in hand. Nearly harmless, and out for
    // longer than most fighters' smashes.
    let jab = strike(Strike {
        id: "jab",
        clip: "attack_side",
        startup_s: 0.06,
        active_s: 0.10,
        recover_s: 0.16,
        offset: (28.0, 0.0),
        half_extents: (18.0, 14.0),
        damage: 3,
        knockback: 46.0,
        knockback_growth: 1.00,
        launch_dir: None,
        on_hit: None,
    });
    let jab = strike_tag(jab, SLASH_POKE_VFX);
    let jab = vfx(jab, 0.06, "friction_tick");

    // A forward tilt, so the most common press does not fall down the
    // directional chain to the jab. A stride and a flat swing.
    let mut f_tilt = strike(Strike {
        id: "tilt_forward",
        clip: "attack_side",
        startup_s: 0.11,
        active_s: 0.11,
        recover_s: 0.20,
        offset: (34.0, -2.0),
        half_extents: (22.0, 15.0),
        damage: 6,
        knockback: 78.0,
        knockback_growth: 1.55,
        launch_dir: Some((1.0, -0.30)),
        on_hit: None,
    });
    // A short additive stride: it adds to his walk, so the swing covers more
    // ground out of a dash.
    f_tilt.start_impulse = Some((150.0, 0.0));
    let f_tilt = vfx(f_tilt, 0.11, "wrench_strike");
    let f_tilt = on_contact(f_tilt, "player.robot.slash.impact.metal.chink");

    // The needle sweeping the dial: an overhead arc that beats a shorthop and
    // stays out long enough to catch the second one.
    let up_tilt = strike(Strike {
        id: "tilt_up",
        clip: "attack_up",
        startup_s: 0.10,
        active_s: 0.12,
        recover_s: 0.20,
        offset: (8.0, -28.0),
        half_extents: (18.0, 24.0),
        damage: 6,
        knockback: 82.0,
        knockback_growth: 1.60,
        launch_dir: Some((0.1, -1.0)),
        on_hit: None,
    });
    let up_tilt = vfx(up_tilt, 0.10, "gauge_sweep");
    let up_tilt = on_contact(up_tilt, "player.robot.slash.impact.metal.chink");

    // Oil dragged along the floor at ankle height. The lowest, longest-lived box
    // in the table and the one that beats a ledge get-up.
    let down_tilt = strike(Strike {
        id: "tilt_down",
        clip: "attack_down",
        startup_s: 0.09,
        active_s: 0.12,
        recover_s: 0.19,
        offset: (26.0, 14.0),
        half_extents: (22.0, 10.0),
        damage: 5,
        knockback: 62.0,
        knockback_growth: 1.35,
        launch_dir: Some((1.0, -0.20)),
        on_hit: None,
    });
    let down_tilt = strike_tag(down_tilt, SLASH_POKE_VFX);
    let down_tilt = vfx(down_tilt, 0.09, "oil_drip");

    // ── the smashes ──────────────────────────────────────────────────────────

    // The only move whose knockback grows past [`WITHIN_TOLERANCE_GROWTH`], so
    // the only one that closes a stock. Everything else is damage he must convert
    // with this move.
    let mut f_smash = strike(Strike {
        id: "smash_forward",
        clip: "smash_forward",
        startup_s: 0.26,
        active_s: 0.10,
        recover_s: 0.34,
        offset: (40.0, -4.0),
        half_extents: (28.0, 20.0),
        damage: 15,
        knockback: 150.0,
        knockback_growth: TORQUE_GROWTH,
        launch_dir: Some((1.0, -0.42)),
        on_hit: None,
    });
    f_smash.smash_charge_mult = 1.7;
    let f_smash = vfx(f_smash, 0.0, "tolerance_brackets");
    let f_smash = vfx(f_smash, 0.26, "wrench_strike");
    let f_smash = vfx(f_smash, 0.28, "brass_spark");
    let f_smash = on_contact(f_smash, "player.robot.slash.impact.metal.gong");

    // A bearing thrown straight up out of the housing.
    let mut up_smash = strike(Strike {
        id: "smash_up",
        clip: "attack_up",
        startup_s: 0.24,
        active_s: 0.11,
        recover_s: 0.32,
        offset: (6.0, -34.0),
        half_extents: (22.0, 30.0),
        damage: 12,
        knockback: 130.0,
        knockback_growth: 2.05,
        launch_dir: Some((0.0, -1.0)),
        on_hit: None,
    });
    up_smash.smash_charge_mult = 1.7;
    let up_smash = vfx(up_smash, 0.24, "bearing_ping");
    let up_smash = on_contact(up_smash, "player.robot.slash.impact.metal.gong");

    // Oil slapped out both sides at his feet — the widest box in the table and
    // his answer to being surrounded on a ledge.
    let mut down_smash = strike(Strike {
        id: "smash_down",
        clip: "attack_down",
        startup_s: 0.22,
        active_s: 0.12,
        recover_s: 0.34,
        offset: (0.0, 16.0),
        half_extents: (38.0, 12.0),
        damage: 11,
        knockback: 120.0,
        knockback_growth: 1.95,
        launch_dir: Some((0.9, -0.50)),
        on_hit: None,
    });
    down_smash.smash_charge_mult = 1.7;
    let down_smash = vfx(down_smash, 0.22, "oil_splash");
    let down_smash = vfx(down_smash, 0.30, "oil_slick");
    let down_smash = on_contact(down_smash, "player.robot.slash.impact.metal.gong");

    // ── the aerials ──────────────────────────────────────────────────────────
    //
    // Every aerial autocancels later than its landing lag, so his air game is a
    // commitment and a rising short-hop aerial is not a free approach.

    // The longest hitbox in the table: he swings the spanner all the way
    // round. `unit_circle_rotation` is the row this move was named for.
    let mut n_air = strike(Strike {
        id: "air_neutral",
        clip: "attack_side",
        startup_s: 0.07,
        active_s: 0.16,
        recover_s: 0.16,
        offset: (0.0, 0.0),
        half_extents: (26.0, 22.0),
        damage: 5,
        knockback: 62.0,
        knockback_growth: 1.30,
        launch_dir: None,
        on_hit: None,
    });
    n_air.landing_lag_s = Some(0.12);
    n_air.autocancel_after_s = Some(0.26);
    let n_air = vfx(n_air, 0.07, "unit_circle_rotation");

    let mut f_air = strike(Strike {
        id: "air_forward",
        clip: "attack_side",
        startup_s: 0.12,
        active_s: 0.11,
        recover_s: 0.20,
        offset: (30.0, -2.0),
        half_extents: (24.0, 18.0),
        damage: 8,
        knockback: 96.0,
        knockback_growth: 1.80,
        launch_dir: Some((1.0, -0.30)),
        on_hit: None,
    });
    f_air.landing_lag_s = Some(0.16);
    f_air.autocancel_after_s = Some(0.30);
    let f_air = vfx(f_air, 0.12, "curve_trace");
    let f_air = on_contact(f_air, "player.robot.slash.impact.metal.chink");

    // His hardest hit after the torque smash, facing the wrong way.
    let mut b_air = strike(Strike {
        id: "air_back",
        clip: "attack_side",
        startup_s: 0.13,
        active_s: 0.10,
        recover_s: 0.22,
        offset: (-30.0, 0.0),
        half_extents: (24.0, 18.0),
        damage: 9,
        knockback: 108.0,
        knockback_growth: WITHIN_TOLERANCE_GROWTH,
        launch_dir: Some((-1.0, -0.34)),
        on_hit: None,
    });
    b_air.landing_lag_s = Some(0.18);
    b_air.autocancel_after_s = Some(0.32);
    let b_air = vfx(b_air, 0.13, "bearing_ping");
    let b_air = on_contact(b_air, "player.robot.slash.impact.metal.gong");

    let mut u_air = strike(Strike {
        id: "air_up",
        clip: "attack_up",
        startup_s: 0.09,
        active_s: 0.12,
        recover_s: 0.18,
        offset: (2.0, -30.0),
        half_extents: (20.0, 24.0),
        damage: 6,
        knockback: 84.0,
        knockback_growth: 1.70,
        launch_dir: Some((0.0, -1.0)),
        on_hit: None,
    });
    u_air.landing_lag_s = Some(0.14);
    u_air.autocancel_after_s = Some(0.28);
    let u_air = vfx(u_air, 0.09, "chalk_spiral");

    // No pogo rebound: bouncing off a victim would out-recover the geyser.
    let mut d_air = strike(Strike {
        id: "air_down",
        clip: "attack_down",
        startup_s: 0.13,
        active_s: 0.12,
        recover_s: 0.24,
        offset: (4.0, 28.0),
        half_extents: (20.0, 20.0),
        damage: 10,
        knockback: 112.0,
        knockback_growth: 1.85,
        launch_dir: Some((0.0, 1.0)),
        on_hit: None,
    });
    d_air.landing_lag_s = Some(0.26);
    d_air.autocancel_after_s = Some(0.36);
    let d_air = vfx(d_air, 0.13, "oil_drip");
    let d_air = on_contact(d_air, "player.robot.slash.impact.metal.chink");

    // ── The four specials ─────────────────────────────────────────────────────
    //
    // Four mechanisms: one lands three times on one press and never moves him;
    // one commands a slide he can steer; one commands a rise he cannot; one adds
    // to what he was already doing, at the press.
    // `the_four_specials_are_four_mechanisms` asserts each and that no two share
    // one.

    // Neutral: `convergence`. Three taps at closing intervals, each harder than
    // the last.
    //
    // It multi-hits because of the gaps. Touching windows share one hit set, so a
    // swing sampled at keyframes does not hit once per segment. A window after a
    // gap rehits. The 0.06s and 0.04s gaps are the move.
    let convergence = strike(Strike {
        id: "convergence",
        clip: "special",
        startup_s: 0.14,
        active_s: 0.06,
        recover_s: 0.30,
        offset: (30.0, -2.0),
        half_extents: (24.0, 18.0),
        damage: 3,
        knockback: 40.0,
        knockback_growth: 1.10,
        launch_dir: None,
        on_hit: None,
    });
    // Tag before the later terms are pushed: the first two are jabs and the third
    // is the swing they converge on.
    let mut convergence = strike_tag(convergence, SLASH_POKE_VFX);
    // The second and third terms are windows, not more moves: same press, same
    // clock, closing gaps.
    convergence.windows.push(converging_term(
        0.26,
        0.32,
        4,
        55.0,
        1.30,
        None,
        SLASH_POKE_VFX,
    ));
    convergence.windows.push(converging_term(
        0.36,
        0.44,
        8,
        112.0,
        WITHIN_TOLERANCE_GROWTH,
        Some((0.9, -0.50)),
        SLASH_ARC_VFX,
    ));
    debug_assert!(
        convergence.duration_s >= 0.44,
        "the last term must fit inside the move"
    );
    let convergence = vfx(convergence, 0.0, "tolerance_brackets");
    let convergence = vfx(convergence, 0.14, "convergence_ticks");
    let convergence = vfx(convergence, 0.26, "convergence_ticks");
    let convergence = vfx(convergence, 0.36, "error_term_collapse");
    let convergence = on_contact(convergence, "player.robot.slash.impact.metal.chink");

    // Side: `slick_dash`. He oils the floor under himself and goes.
    //
    // The tail does not lock his steering. Other committed charges end with
    // `motion_scale: 0.0`; this one leaves it at 1.0, so he keeps steering but
    // cannot stop. It is the one displacing move that can be aimed after it
    // starts, and the one most likely to carry him off the stage.
    let side_b = strike(Strike {
        id: "slick_dash",
        clip: "special",
        startup_s: 0.16,
        active_s: 0.12,
        recover_s: 0.26,
        offset: (32.0, 4.0),
        half_extents: (26.0, 18.0),
        damage: 10,
        knockback: 105.0,
        knockback_growth: 1.90,
        launch_dir: Some((0.9, -0.35)),
        on_hit: None,
    });
    // Exactly horizontal, so the recovery search is never offered it as a way
    // home. His way home is the geyser.
    let side_b = impulse(side_b, 0.16, (720.0, 0.0), ImpulseMode::Set);
    let side_b = committed_tail(side_b, 0.66, 1.0);
    let side_b = vfx(side_b, 0.16, "oil_slick");
    let side_b = vfx(side_b, 0.34, "oil_drip");
    let side_b = on_contact(side_b, "player.robot.slash.impact.metal.chink");

    // Up: the geyser. He does not jump; a column throws him. The rise is
    // commanded (`Set`) at [`GEYSER_AT_S`], after a visible windup, so a falling
    // Oiler climbs as far as a standing one.
    //
    // The three rows play in order: `oil_geyser_emerge` while the ground swells
    // (the tell), `oil_geyser_stream` three times over the climb so the column
    // reads as continuous, and `oil_geyser_impact` at the crest.
    //
    // It is not flight. With no `Cancelable` window he cannot re-press until the
    // move ends, and the move outlasts its arc, so repeated use loses height. A
    // test holds this; no cooldown or rollback state is needed.
    let mut up_b = strike(Strike {
        id: "oil_geyser",
        clip: "special",
        startup_s: GEYSER_AT_S,
        active_s: 0.14,
        recover_s: 0.18,
        offset: (0.0, 16.0),
        half_extents: (20.0, 36.0),
        damage: 8,
        knockback: 92.0,
        knockback_growth: 1.85,
        launch_dir: Some((0.0, -1.0)),
        on_hit: None,
    });
    // Landing out of the column costs. Onstage that makes it a bad panic button;
    // offstage it is irrelevant, which is the right shape for a way home.
    up_b.landing_lag_s = Some(0.30);
    let up_b = impulse(up_b, GEYSER_AT_S, (0.0, -GEYSER_SPEED), ImpulseMode::Set);
    // The helpless tail. `0.12` leaves him able to nudge where he lands and
    // nothing more, which is what makes edgeguarding this possible.
    let up_b = committed_tail(up_b, GEYSER_ENDS_S, 0.12);
    let up_b = vfx(up_b, 0.06, "oil_geyser_emerge");
    let up_b = vfx_cued(
        up_b,
        GEYSER_AT_S,
        "oil_geyser_stream",
        (0.0, 0.0),
        1.0,
        // The `.loop` suffix is real (see the module doc); the derived cue would
        // miss the bank.
        "vfx.oiler.oil_geyser_stream.loop",
    );
    // The column's two re-strikes carry no cue: the loop above still runs, and
    // the derived cue would miss the bank.
    let up_b = vfx(up_b, 0.44, "oil_geyser_stream");
    let up_b = vfx(up_b, 0.66, "oil_geyser_stream");
    let up_b = vfx(up_b, 0.88, "oil_geyser_impact");
    let up_b = on_contact(up_b, "player.hit");
    // The column stays for a moment as a pool (roster decision #20,
    // Jon's to overrule).
    //
    // This gives him an expressive special (see `authored_movesets`'s census).
    // A geyser is a launcher, so it reuses `smash.place_spring`: a timed,
    // use-limited actuator with an arming delay, which keeps it from throwing
    // the fighter who made it.
    //
    // Modest on purpose: one use, 2.2s, and it throws straight up slower than
    // his own climb. It punishes whoever chases him offstage; it is not a second
    // recovery.
    let up_b = ambition_entity_catalog::smash_spring::author_place_spring(
        up_b,
        // At the crest, where `oil_geyser_impact` draws.
        0.88,
        ambition_entity_catalog::smash_spring::PlaceSpringParams {
            // Up is negative y. Below `GEYSER_SPEED`: the pool must not beat the move
            // that made it.
            launch: (0.0, -700.0),
            half_extents: (26.0, 8.0),
            lifetime_s: 2.2,
            uses: 1,
            // Under him, on the floor he left.
            offset: (0.0, 18.0),
            // So the other player sees it. A plate draws nothing of its own (see
            // `PlaceSpringParams::vfx`).
            vfx: "oil_slick".to_string(),
        },
    );

    // Down: `pressure_vent`. He opens a valve and everything goes at once.
    //
    // The only move displaced by `start_impulse`: it fires at the press and adds
    // to his current fall, unlike the geyser's `Set`. So it is a fast-fall
    // punish, not a way home: from a standstill it barely moves him, out of a
    // dive it drives him down.
    let mut down_b = strike(Strike {
        id: "pressure_vent",
        clip: "special",
        startup_s: 0.10,
        active_s: 0.12,
        recover_s: 0.26,
        offset: (0.0, 22.0),
        half_extents: (30.0, 18.0),
        damage: 9,
        knockback: 100.0,
        knockback_growth: 1.80,
        launch_dir: Some((0.0, 1.0)),
        on_hit: None,
    });
    down_b.start_impulse = Some((0.0, 520.0));
    let down_b = vfx(down_b, 0.10, "pressure_vent");
    let down_b = vfx(down_b, 0.12, "brass_spark");
    let down_b = on_contact(down_b, "player.robot.slash.impact.metal.gong");

    // Oiler's capture kit: middle of the roster on every axis, with a slightly
    // taller box. His sheet has no grab family and no plain `attack`, so all
    // three beats draw `attack_side`; his clip guard refuses unpublished rows.
    let grab = author_standing_grab(
        grab_shell("oiler_grab", "attack_side", 0.08, 0.11, 0.20),
        CaptureAttemptParams {
            offset: (12.0, 1.0),
            half_extents: (19.0, 17.0),
            hold_offset: (13.0, 3.0),
        },
    );
    let pummel = author_pummel(
        capture_beat("oiler_pummel", "attack_side", 0.22),
        0.1,
        CapturePummelParams { damage: 4 },
    );
    let forward_throw = author_throw(
        capture_beat("oiler_fthrow", "attack_side", 0.27),
        0.14,
        CaptureThrowParams {
            damage: 8,
            knockback: 114.0,
            knockback_growth: 2.1,
            launch_dir: (0.75, -0.65),
        },
    );

    let back_throw = author_throw(
        capture_beat("oiler_bthrow", "attack_side", 0.29),
        0.15,
        CaptureThrowParams {
            damage: 9,
            knockback: 123.12,
            knockback_growth: 2.21,
            launch_dir: (-1.0, -0.4),
        },
    );

    let up_throw = author_throw(
        capture_beat("oiler_uthrow", "attack_side", 0.28),
        0.14,
        CaptureThrowParams {
            damage: 8,
            knockback: 118.56,
            knockback_growth: 2.14,
            launch_dir: (0.0, -1.0),
        },
    );

    let down_throw = author_throw(
        capture_beat("oiler_dthrow", "attack_side", 0.3),
        0.15,
        CaptureThrowParams {
            damage: 6,
            knockback: 84.36,
            knockback_growth: 1.68,
            launch_dir: (0.3, -0.92),
        },
    );
    let repertoire = SmashRepertoire {
        taunt: ambition_entity_catalog::authoring::taunt("oiler_taunt", 0.9),
        // 0.11 active, not the usual 0.09: every reaching move of his holds its box
        // for `TOLERANCE_S` (see the `debug_assert` at the end of this function).
        dash_attack: ambition_entity_catalog::authoring::dash_attack(
            "oiler_dash_attack",
            ambition_entity_catalog::authoring::DashAttackShape {
                active_s: 0.11,
                ..ambition_entity_catalog::authoring::DashAttackShape::GENRE
            },
            8,
            97.5,
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
        neutral_special: NeutralSpecial::Authored(convergence),
        side_special: side_b,
        up_special: UpSpecial::Standard(up_b),
        // Every smash fighter has a grab. The values are per character on purpose.
        capture: SmashCaptureRepertoire {
            // His own bearing, pummel and geyser effects; his kit guard requires every
            // effect to come from his own sheet.
            cues: CaptureCues {
                reach: "bearing_ping",
                impact: "friction_tick",
                release: "oil_geyser_impact",
            },
            grab,
            pummel,
            forward_throw,
            back_throw: Some(back_throw),
            up_throw: Some(up_throw),
            down_throw: Some(down_throw),
        },
        down_special: DownSpecial::OneForm(down_b),
    }
    .into_contract();

    // Check the tolerance band where it is authored, over the whole table.
    //
    // The band covers moves that hold a box. A pummel and a throw have no Active
    // window by construction (see `capture_beat`), so they are skipped. His grab
    // does reach and honours the band (`active_s` 0.11). A reaching move inside
    // the band still fails.
    debug_assert!(
        repertoire
            .moves
            .iter()
            .filter(|m| m.windows.iter().any(|w| matches!(w.tag, WindowTag::Active)))
            .all(|m| total_active_s(m) + 1e-4 >= TOLERANCE_S),
        "an Oiler move closed its window inside the tolerance band"
    );

    repertoire
}

/// One later term of `convergence` — an Active window separated from the last by
/// a real gap, which is what makes the move rehit.
fn converging_term(
    start_s: f32,
    end_s: f32,
    damage: i32,
    knockback: f32,
    knockback_growth: f32,
    launch_dir: Option<(f32, f32)>,
    tag: &str,
) -> MoveWindow {
    MoveWindow {
        start_s,
        end_s,
        tag: WindowTag::Active,
        volumes: vec![HitVolume {
            // An ordinary hit, not a gust.
            shape: VolumeShape::Rect {
                offset: (30.0, -2.0),
                half_extents: (24.0, 18.0),
            },
            damage,
            knockback,
            // The builder's zero means "this stage decides" (see
            // `HitVolume::knockback_growth`). Fixed knockback needs a direct volume.
            knockback_growth: (knockback_growth > 0.0).then_some(knockback_growth),
            launch_dir,
            on_hit: None,
            vfx: Some(tag.to_string()),
            hit_sfx: None,
            reaction: None,
        }],
        motion_scale: 0.35,
        sustain_effect: None,
    }
}

/// How long this move keeps a hitbox in the world, summed over its active
/// windows. The measurement [`TOLERANCE_S`] is about.
fn total_active_s(m: &MoveSpec) -> f32 {
    m.windows
        .iter()
        .filter(|w| matches!(w.tag, WindowTag::Active))
        .map(|w| w.end_s - w.start_s)
        .sum()
}

#[cfg(test)]
mod tests {
    /// The geyser leaves a pool, and it must not be a second way home.
    ///
    /// The pool's launch must be weaker than his own climb. Otherwise the recovery
    /// would improve with repeated use, which the geyser's design forbids.
    #[test]
    fn the_geysers_pool_throws_less_hard_than_the_geyser_itself() {
        use ambition_entity_catalog::MoveEventKind;
        let set = crate::authored_movesets::shipped("npc_oiler");
        let up = set
            .moves
            .iter()
            .find(|m| m.id == "oil_geyser")
            .expect("his up-B is in the table");
        let placed = up
            .events
            .iter()
            .find_map(|e| match &e.kind {
                MoveEventKind::Effect(effect)
                    if effect.key == ambition_entity_catalog::smash_spring::PLACE_SPRING =>
                {
                    Some((
                        e.at_s,
                        effect
                            .params
                            .hydrate::<ambition_entity_catalog::smash_spring::PlaceSpringParams>(),
                    ))
                }
                _ => None,
            })
            .expect("the geyser leaves nothing behind, so the column is one the caster alone met");
        let (at_s, params) = placed;
        let params = params.expect("the pool's params hydrate");

        assert!(
            params.launch.1 < 0.0,
            "the pool throws DOWNWARD ({:?}) — up is negative y",
            params.launch
        );
        assert!(
            -params.launch.1 < super::GEYSER_SPEED,
            "the pool throws at {} against the geyser's own {} — a plate stronger \
             than the move that made it is a recovery that improves by being used \
             twice",
            -params.launch.1,
            super::GEYSER_SPEED
        );
        // It lands at the crest, where `oil_geyser_impact` draws, not under him at
        // the press.
        assert!(
            at_s > 0.5,
            "the pool is placed at {at_s}s, before the column has finished climbing"
        );
        assert_eq!(params.uses, 1, "a multi-use geyser pool is a platform");
    }

    use super::*;
    use ambition_entity_catalog::{AttackDir, MoveEventKind};

    fn find(set: &MovesetContract, id: &str) -> MoveSpec {
        set.moves
            .iter()
            .find(|m| m.id == id)
            .unwrap_or_else(|| panic!("{id} exists"))
            .clone()
    }

    fn growth(m: &MoveSpec) -> f32 {
        m.windows
            .iter()
            .flat_map(|w| w.volumes.iter())
            .filter_map(|v| v.knockback_growth)
            .fold(0.0f32, f32::max)
    }

    // Verb binding is checked by construction: `SmashRepertoire` owns the verb
    // strings and is a struct with no `Default`, so a missing slot is a compile
    // error. Coverage in every posture is checked by
    // `ambition_entity_catalog::smash_repertoire` and by
    // `smash_roster_movesets::report_the_smash_kit_every_selectable_fighter_has`.

    /// The tolerance band, as an assertion: no Oiler move closes its window inside
    /// [`TOLERANCE_S`].
    ///
    /// The goblin is the control. If its windows were this wide too, the band
    /// would describe the helper, not the character.
    #[test]
    fn every_move_holds_its_hitbox_for_the_tolerance_band() {
        let oiler = crate::authored_movesets::shipped("npc_oiler");
        // Only moves that hold a box, as in the builder's own assertion. Pummels and
        // throws have no Active window (see `capture_beat`).
        let mut reaching = 0;
        for m in oiler
            .moves
            .iter()
            .filter(|m| m.windows.iter().any(|w| matches!(w.tag, WindowTag::Active)))
        {
            reaching += 1;
            let held = total_active_s(m);
            assert!(
                held + 1e-4 >= TOLERANCE_S,
                "`{}` keeps a box in the world for {held}s, inside the band this \
                 fighter works to ({TOLERANCE_S}s)",
                m.id
            );
        }
        // Zero floor: a filter that removed every move would pass trivially.
        assert!(
            reaching >= 16,
            "only {reaching} Oiler moves hold a box at all — the band is being \
             asserted over a population that shrank"
        );

        let goblin = crate::authored_movesets::shipped("goblin");
        let tighter = goblin
            .moves
            .iter()
            .filter(|m| total_active_s(m) + 1e-4 < TOLERANCE_S)
            .count();
        assert!(
            tighter >= 8,
            "only {tighter} goblin moves close inside the band, so the band is a \
             property of `strike` rather than a property of Oiler"
        );
    }

    /// Exactly one move is a kill move. The forward smash grows at
    /// [`TORQUE_GROWTH`] and nothing else may pass [`WITHIN_TOLERANCE_GROWTH`].
    ///
    /// The goblin is the control: it has four moves above the same line.
    #[test]
    fn only_one_move_grows_past_the_tolerance_band() {
        let oiler = crate::authored_movesets::shipped("npc_oiler");
        let torqued: Vec<&str> = oiler
            .moves
            .iter()
            .filter(|m| growth(m) > WITHIN_TOLERANCE_GROWTH)
            .map(|m| m.id.as_str())
            .collect();
        assert_eq!(
            torqued,
            vec!["smash_forward"],
            "exactly one move may close a stock; these grow past \
             {WITHIN_TOLERANCE_GROWTH}"
        );
        assert!(growth(&find(&oiler, "smash_forward")) >= TORQUE_GROWTH);

        let goblin = crate::authored_movesets::shipped("goblin");
        let goblin_torqued = goblin
            .moves
            .iter()
            .filter(|m| growth(m) > WITHIN_TOLERANCE_GROWTH)
            .count();
        assert!(
            goblin_torqued >= 3,
            "the goblin is supposed to have an ordinary spread of kill options \
             ({goblin_torqued} above the line); if it does not, this test is \
             asserting a property of the threshold rather than of Oiler"
        );
    }

    /// The geyser is a save, not a flight.
    ///
    /// He cannot re-press while the move plays (no `Cancelable` window), so the
    /// only question is whether one full cycle gains height. It cannot: the move
    /// outlasts its arc. So no cooldown, per-airtime counter or rollback state is
    /// needed.
    #[test]
    fn the_geyser_is_a_save_and_not_a_flight() {
        let g = ambition_platformer2d::engine_core::DEFAULT_TUNING.gravity;
        let to_apex = GEYSER_SPEED / g;
        let tail = GEYSER_ENDS_S - GEYSER_AT_S;
        assert!(
            tail > 2.0 * to_apex,
            "the column climbs for {to_apex:.3}s and is handed back {tail:.3}s \
             after the burst; anything at or under {:.3}s returns Oiler higher \
             than it found him, every press, which is flight",
            2.0 * to_apex
        );
        // Landing out of it costs, so it is a bad panic button on the stage.
        let up_b = find(&crate::authored_movesets::shipped("npc_oiler"), "oil_geyser");
        assert!(up_b.landing_lag_s.unwrap_or(0.0) > 0.0);
        assert_eq!(up_b.duration_s, GEYSER_ENDS_S);
        assert!(
            up_b.motion_scale_at(GEYSER_ENDS_S - 0.01) < 0.5,
            "the ride down is supposed to be helpless"
        );
    }

    /// The rise is commanded, not added, and it is the only one.
    ///
    /// Under `ImpulseMode::Add` a falling Oiler would climb only what was left
    /// over. `Set` makes the climb a property of the move. `lift_speed` is
    /// derived from `Set` impulses only, so this also shows the brain and the
    /// recovery probe can see the move.
    #[test]
    fn the_geyser_commands_its_rise_and_is_the_only_way_home() {
        let set = crate::authored_movesets::shipped("npc_oiler");
        let up_b = find(&set, "oil_geyser");
        let burst = up_b
            .events
            .iter()
            .find_map(|e| match &e.kind {
                MoveEventKind::Impulse { local, mode } => Some((e.at_s, *local, *mode)),
                _ => None,
            })
            .expect("the recovery special displaces its owner");
        assert_eq!(burst.2, ImpulseMode::Set);
        assert!(burst.1 .1 < 0.0, "the burst must point AGAINST gravity");
        assert_eq!(burst.0, GEYSER_AT_S);

        let frames = up_b.frame_data();
        assert_eq!(frames.lift_speed, GEYSER_SPEED);
        assert_eq!(frames.lift_at_s, GEYSER_AT_S);

        // Control: nothing else advertises a lift.
        let others: Vec<&str> = set
            .moves
            .iter()
            .filter(|m| m.id != "oil_geyser" && m.frame_data().lift_speed > 0.0)
            .map(|m| m.id.as_str())
            .collect();
        assert!(
            others.is_empty(),
            "these moves also claim to be ways home: {others:?}"
        );
    }

    /// The geyser plays its three rows, in order: the ground swells, the column
    /// runs for the whole climb, the crest breaks last.
    #[test]
    fn the_geyser_stages_its_three_rows_in_order() {
        let up_b = find(&crate::authored_movesets::shipped("npc_oiler"), "oil_geyser");
        let at = |row: &str| -> Vec<f32> {
            up_b.events
                .iter()
                .filter_map(|e| match &e.kind {
                    MoveEventKind::Vfx { effect, .. } if effect == row => Some(e.at_s),
                    _ => None,
                })
                .collect()
        };
        let emerge = at("oil_geyser_emerge");
        let stream = at("oil_geyser_stream");
        let impact = at("oil_geyser_impact");
        assert_eq!(emerge.len(), 1, "one swell, before anything else");
        assert!(
            stream.len() >= 3,
            "the column must be re-struck across the climb or it reads as a \
             single puff: {stream:?}"
        );
        assert_eq!(impact.len(), 1);
        assert!(
            emerge[0] < GEYSER_AT_S,
            "the swell is the TELL: it has to arrive before the burst does"
        );
        assert!(stream.iter().all(|t| *t >= GEYSER_AT_S));
        assert!(impact[0] > *stream.last().unwrap(), "the crest breaks last");
        assert!(
            impact[0] < up_b.duration_s,
            "and inside the move, or nothing plays it"
        );
    }

    /// Four specials, four mechanisms: one lands three times and never displaces
    /// him, one commands a steerable slide, one commands a rise he cannot steer,
    /// one adds to his motion at the press. No two share a mechanism.
    #[test]
    fn the_four_specials_are_four_mechanisms() {
        let set = crate::authored_movesets::shipped("npc_oiler");
        let commanded = |id: &str| -> Option<(f32, f32)> {
            find(&set, id).events.iter().find_map(|e| match &e.kind {
                MoveEventKind::Impulse {
                    local,
                    mode: ImpulseMode::Set,
                } => Some(*local),
                _ => None,
            })
        };

        // Neutral: no displacement; it lands three times instead.
        let convergence = find(&set, "convergence");
        assert!(commanded("convergence").is_none());
        assert!(convergence.start_impulse.is_none());
        let terms: Vec<(f32, f32)> = convergence
            .windows
            .iter()
            .filter(|w| matches!(w.tag, WindowTag::Active))
            .map(|w| (w.start_s, w.end_s))
            .collect();
        assert_eq!(terms.len(), 3, "the neutral special's idea IS the series");
        // The gaps make it rehit: contiguous windows share one hit set.
        let gaps: Vec<f32> = terms.windows(2).map(|p| p[1].0 - p[0].1).collect();
        assert!(
            gaps.iter().all(|g| *g > 0.0),
            "a series with no gap between its terms is ONE hit: {gaps:?}"
        );
        assert!(
            gaps[1] < gaps[0],
            "the terms are supposed to CONVERGE ({gaps:?})"
        );
        let damages: Vec<i32> = convergence
            .windows
            .iter()
            .filter(|w| matches!(w.tag, WindowTag::Active))
            .filter_map(|w| w.volumes.first().map(|v| v.damage))
            .collect();
        assert!(
            damages.windows(2).all(|p| p[1] > p[0]),
            "and each term must be worth more than the last: {damages:?}"
        );

        // Side: a commanded slide whose tail leaves his steering alone.
        let side = commanded("slick_dash").expect("the side special travels");
        assert!(side.0 > 0.0 && side.1 == 0.0, "flat, and forward");
        let slick = find(&set, "slick_dash");
        // The tail must exist; otherwise `strike`'s own 1.0 recovery window answers
        // below.
        assert!(
            slick.duration_s > 0.60,
            "the slide is supposed to outlast its own swing ({}s)",
            slick.duration_s
        );
        assert_eq!(
            slick.motion_scale_at(slick.duration_s - 0.01),
            1.0,
            "oil takes your brakes, not your steering — a locked tail makes this \
             the same move as everybody else's charge"
        );

        // Up: a rise only, with a tail that does lock, measured the same way.
        let up = commanded("oil_geyser").expect("the Up-B displaces");
        assert!(up.1 < 0.0 && up.0 == 0.0);
        let geyser = find(&set, "oil_geyser");
        assert!(geyser.motion_scale_at(geyser.duration_s - 0.01) < 0.5);

        // Down: displaced at the press, additively (the only `start_impulse`).
        assert!(commanded("pressure_vent").is_none());
        let vent = find(&set, "pressure_vent")
            .start_impulse
            .expect("the vent shoves at the press");
        assert!(vent.1 > 0.0, "and it shoves DOWNWARD");
        let pressers: Vec<&str> = set
            .moves
            .iter()
            .filter(|m| m.start_impulse.is_some())
            .map(|m| m.id.as_str())
            .collect();
        assert_eq!(
            pressers,
            vec!["tilt_forward", "pressure_vent"],
            "only the stride into the forward tilt and the vent shove at the \
             press; a second SPECIAL doing it makes two of these one move"
        );
    }

    // Burst sounds are covered by `a_paired_burst_is_heard_exactly_once`
    // (`src/moveset_sound.rs`), which runs these tables through the real
    // dispatcher and counts what reaches the SFX channel.

    /// Oiler's art is his own, and it all exists. An effect no shipped sheet
    /// carries never plays; another fighter's bursts give him no look.
    /// `is_authored_effect` reads the baked manifests, so this asks what the
    /// renderer asks.
    #[test]
    fn the_kit_looks_like_oiler_and_the_art_all_ships() {
        let set = crate::authored_movesets::shipped("npc_oiler");
        let mut effects = std::collections::BTreeSet::new();
        // Collect problems across every move, then assert once, so one run reports
        // every move that references a renamed effect.
        let mut problems: Vec<String> = Vec::new();
        for m in &set.moves {
            problems.extend(m.presentation_problems(
                ambition_platformer2d::sprite_sheet::fx::is_authored_effect,
            ));
            for ev in &m.events {
                if let MoveEventKind::Vfx { effect, .. } = &ev.kind {
                    effects.insert(effect.clone());
                }
            }
        }
        // Before the palette checks below: a renamed effect fails those too, with a
        // less helpful message.
        assert!(problems.is_empty(), "{problems:?}");
        assert!(
            effects.len() >= 12,
            "a jab, a smash, a launcher, four specials and a recovery cannot all \
             look the same: {effects:?}"
        );
        // Every effect comes from his own sheet.
        for effect in &effects {
            let authored = ambition_platformer2d::sprite_sheet::fx::authored_effect(effect)
                .unwrap_or_else(|| panic!("`{effect}` ships"));
            assert_eq!(
                authored.sheet, "oiler_vfx",
                "`{effect}` is drawn from `{}` — Oiler has his own sheet",
                authored.sheet
            );
        }

        // A heavy landing is heard apart from a poke landing.
        let heavy_hit = |id: &str| -> Option<String> {
            find(&set, id)
                .windows
                .iter()
                .flat_map(|w| w.volumes.iter())
                .find_map(|v| v.hit_sfx.clone())
        };
        assert_ne!(heavy_hit("smash_forward"), heavy_hit("tilt_forward"));
        assert!(heavy_hit("smash_forward").is_some());
        assert!(heavy_hit("jab").is_none(), "a knuckle-rap does not clang");
    }

    /// Every press a body can make reaches a move, in both postures. The CPU kit
    /// builder and a human's stick use the same function.
    #[test]
    fn both_postures_reach_at_least_eight_distinct_moves() {
        let set = crate::authored_movesets::shipped("npc_oiler");
        let reachable = |grounded: bool| -> std::collections::BTreeSet<String> {
            let mut ids = std::collections::BTreeSet::new();
            for base in ["attack", "smash", "special"] {
                for dir in [
                    AttackDir::Neutral,
                    AttackDir::Forward,
                    AttackDir::Back,
                    AttackDir::Up,
                    AttackDir::Down,
                ] {
                    if let Some(m) = set.move_for_directional_verb(base, dir, grounded) {
                        ids.insert(m.id.clone());
                    }
                }
            }
            ids
        };
        let on_ground = reachable(true);
        let airborne = reachable(false);
        assert!(
            on_ground.len() >= 8,
            "a grounded Oiler reaches {on_ground:?}"
        );
        assert!(
            airborne.len() >= 8,
            "an airborne Oiler reaches {airborne:?}"
        );
        // The recovery is reachable from both postures.
        assert!(on_ground.contains("oil_geyser"));
        assert!(airborne.contains("oil_geyser"));
        // The forward press does not fall through to the jab.
        assert_eq!(
            set.move_for_directional_verb("attack", AttackDir::Forward, true)
                .map(|m| m.id.as_str()),
            Some("tilt_forward")
        );
    }

    /// Every move names a clip the sheet draws. Without the fight rows, every
    /// swing would fall down the structural chain to `idle`.
    ///
    /// The oracle is the baked sheet record, so this fails if the sheet is
    /// republished without them.
    #[test]
    fn every_move_names_a_row_the_published_sheet_carries() {
        let record =
            ambition_platformer2d::sprite_sheet::character::sheets::record_for_sheet_key("oiler")
                .expect("Oiler's sheet is baked into the binary");
        let rows: std::collections::BTreeSet<&str> = record
            .rows
            .iter()
            .map(|row| row.animation.as_str())
            .collect();
        assert!(
            rows.contains("idle") && rows.contains("walk"),
            "this is not Oiler's sheet: {rows:?}"
        );
        for m in &crate::authored_movesets::shipped("npc_oiler").moves {
            let chain: Vec<&str> = std::iter::once(m.clip.clip.as_str())
                .chain(m.clip.fallbacks.iter().map(String::as_str))
                .collect();
            let drawn = chain.iter().find(|row| rows.contains(*row));
            assert!(
                drawn.is_some_and(|row| *row != "idle"),
                "`{}` draws {chain:?}, and the published sheet answers none of it \
                 before `idle` — so the move draws the standing pose. Rows: {rows:?}",
                m.id,
            );
        }
    }
}

//! **Oiler's repertoire** — the maintenance mechanic, as a fighter.
//!
//! Jon, 2026-08-16: *"give oiler a full interesting smash kit. We have lots of
//! new vfx and sfx to pull from... Especially oil geyser."*
//!
//! ⭐ **the fifth adopter removed from `smash_fighter_kit()`** (campaign P3.24).
//! Oiler stood on the Super Smash Siblings grid with a `peaceful` catalog row —
//! no melee at all — while twenty-three of his own effects sat rendered and
//! unbound on `oiler_vfx_spritesheet.ron` and twenty-four matching cues sat in
//! the packed bank. This table is the wire between them.
//!
//! ## The character, from his own row
//!
//! > *"Quit messing with my tools!"* — *"Lubrication is 90% of uptime."*
//!
//! An Euler parody who fixes machines for a living. Two ideas run the table:
//!
//! ```text
//!   TOLERANCE   every move holds its hitbox out ≥ 0.10s   he is forgiving
//!   TORQUE      exactly ONE move is torqued to kill       he is not scary
//! ```
//!
//! A mechanic works to a tolerance band: near enough is the job. So every one
//! of Oiler's sixteen moves keeps a box in the world for at least
//! [`TOLERANCE_S`] — the widest timing windows on the grid — and in exchange his
//! knockback GROWTH stays inside [`WITHIN_TOLERANCE_GROWTH`] everywhere except
//! the forward smash, the one bolt he takes to spec. That is a fighter who is
//! very hard to whiff with and very hard to close a stock with: he racks damage
//! all match and then has to land one specific move.
//!
//! ⛔ **not the goblin's table renumbered**, and both claims above are tested
//! COMPARATIVELY against it for exactly that reason — the goblin is the other
//! fighter this crate authors from the same [`crate::moveset_authoring`]
//! primitives, so a table copied and retuned would pass everything else here.
//!
//! ## The effects are the move, not decoration
//!
//! ⭐ **an effect is a NAME.** `oil_geyser_emerge` addresses a row on a shipped
//! FX sheet and needs no table, enum or registry to reach the screen; the
//! authored string goes on the wire as its hash and presentation resolves it
//! against the rows the art actually carries. Every effect this table names is
//! one of Oiler's own twenty-three.
//!
//! ⛔⛔ **and a `Vfx` event is SILENT on its own.** The paired-cue lookup only
//! runs on the `FxRequest` path; a move's `Vfx` event writes `VfxMessage::Effect`
//! straight through and spawns no sound. So every burst in this table is
//! authored as a PAIR — a `Vfx` and an `Sfx` at the same instant — and
//! `every_burst_in_this_table_is_heard` is the guard that keeps it that way.
//!
//! ⚠ **four cues carry a `.loop` suffix the row name does not**
//! (`vfx.oiler.oil_geyser_stream.loop`), because the sound was rendered as a
//! loop and the sprite row was not. The derived `vfx.<family>.<row>` cue misses
//! for those four, which is precisely why they are spelled out here.

use ambition_characters::moveset_prefabs::{SLASH_ARC_VFX, SLASH_POKE_VFX};
use ambition_characters::smash_repertoire::{DownSpecial, NeutralSpecial, SmashRepertoire};
use ambition_platformer2d::entity_catalog::{
    HitVolume, ImpulseMode, MoveSpec, MoveWindow, MovesetContract, VolumeShape, WindowTag,
};

use ambition_characters::moveset_authoring::{
    committed_tail, impulse, on_contact, sfx, strike, strike_tag, vfx,
};

/// **The tolerance band**: the least time any Oiler move keeps a hitbox in the
/// world, summed over its active windows.
///
/// ⚠ this is the character, not a tuning constant that happens to bracket the
/// numbers. Retuning Oiler means moving a move within the band, never under it —
/// a mechanic whose windows closed as fast as a goblin's would just be a slower
/// goblin.
pub const TOLERANCE_S: f32 = 0.10;

/// **The one bolt torqued to spec.** No move but the forward smash may grow
/// harder than this with the victim's damage.
pub const WITHIN_TOLERANCE_GROWTH: f32 = 2.10;

/// And what the forward smash grows at instead. The gap between the two is the
/// whole reason Oiler has to land a specific move to take a stock.
pub const TORQUE_GROWTH: f32 = 3.30;

/// **The rise the geyser commands**, engine units per second against gravity.
///
/// ⭐ authored as a SPEED and applied with [`ImpulseMode::Set`], which is what
/// makes it a recovery: an Oiler pressing this while falling at terminal
/// velocity gets exactly the climb a standing one does. An additive impulse
/// would be weakest precisely when it is the only thing between him and the
/// blast zone.
pub const GEYSER_SPEED: f32 = 980.0;

/// When the column arrives — after a windup you can see and hear (the ground
/// swells first: `oil_geyser_emerge`).
pub const GEYSER_AT_S: f32 = 0.22;

/// And when the move lets go. ⛔ **not a feel number.** Under the engine
/// baseline the burst climbs `GEYSER_SPEED² / 2g ≈ 213px` and takes
/// `GEYSER_SPEED / g ≈ 0.44s` to do it, so a tail shorter than twice that hands
/// Oiler back before he has fallen the height it bought — and a move that
/// returns you higher than it found you, every press, is flight. The guard
/// `the_geyser_is_a_save_and_not_a_flight` holds the arithmetic.
pub const GEYSER_ENDS_S: f32 = 1.20;

/// See the module doc. Sixteen moves: the genre's standard verb map plus four
/// specials.
pub fn oiler_moveset() -> MovesetContract {
    // ── the ground game: a spanner at arm's length ────────────────────────────
    //
    // ⚠ **every clip name here is a row the rig actually publishes.** Oiler's
    // sheet grew `attack_side` / `attack_up` / `attack_down` / `smash_forward` /
    // `special` alongside `idle`/`walk`/`talk`/`interact` for this table; the
    // structural fallback chain (`attack_side` → `attack` → `slash` → `idle`) is
    // what a move settles for, and settling for `idle` was every swing he had.
    //
    // ⛔ the up and down families SHARE one row each. The sheet has one upward
    // swing, so the up-tilt and the up-smash both draw it — an honest statement
    // that the art is thinner than the table, rather than a `smash_up` clip name
    // that would quietly fall all the way back to a side swing.

    // A knuckle-rap with the wrench still in hand. Nearly harmless, and out for
    // longer than most fighters' smashes.
    let jab = strike(
        "jab",
        "attack_side",
        0.06,
        0.10,
        0.16,
        (28.0, 0.0),
        (18.0, 14.0),
        3,
        46.0,
        1.00,
        None,
        None,
    );
    let jab = strike_tag(jab, SLASH_POKE_VFX);
    let jab = vfx(jab, 0.06, "friction_tick");
    let jab = sfx(jab, 0.06, "vfx.oiler.friction_tick");

    // ⛔ **a forward tilt, because without one the commonest press in the genre
    // falls down the directional chain to the jab** — the hole George Booul's
    // table had for a week. A stride and a flat swing.
    let mut f_tilt = strike(
        "tilt_forward",
        "attack_side",
        0.11,
        0.11,
        0.20,
        (34.0, -2.0),
        (22.0, 15.0),
        6,
        78.0,
        1.55,
        Some((1.0, -0.30)),
        None,
    );
    // A short stride, ADDITIVE: it contributes to whatever walk he brought into
    // it rather than replacing it, so the same swing covers more ground out of a
    // dash.
    f_tilt.start_impulse = Some((150.0, 0.0));
    let f_tilt = vfx(f_tilt, 0.11, "wrench_strike");
    let f_tilt = sfx(f_tilt, 0.11, "vfx.oiler.wrench_strike");
    let f_tilt = on_contact(f_tilt, "player.robot.slash.impact.metal.chink");

    // The needle sweeping the dial: an overhead arc that beats a shorthop and
    // stays out long enough to catch the second one.
    let up_tilt = strike(
        "tilt_up",
        "attack_up",
        0.10,
        0.12,
        0.20,
        (8.0, -28.0),
        (18.0, 24.0),
        6,
        82.0,
        1.60,
        Some((0.1, -1.0)),
        None,
    );
    let up_tilt = vfx(up_tilt, 0.10, "gauge_sweep");
    let up_tilt = sfx(up_tilt, 0.10, "vfx.oiler.gauge_sweep");
    let up_tilt = on_contact(up_tilt, "player.robot.slash.impact.metal.chink");

    // Oil dragged along the floor at ankle height. The lowest, longest-lived box
    // in the table and the one that beats a ledge get-up.
    let down_tilt = strike(
        "tilt_down",
        "attack_down",
        0.09,
        0.12,
        0.19,
        (26.0, 14.0),
        (22.0, 10.0),
        5,
        62.0,
        1.35,
        Some((1.0, -0.20)),
        None,
    );
    let down_tilt = strike_tag(down_tilt, SLASH_POKE_VFX);
    let down_tilt = vfx(down_tilt, 0.09, "oil_drip");
    let down_tilt = sfx(down_tilt, 0.09, "vfx.oiler.oil_drip");

    // ── the smashes ──────────────────────────────────────────────────────────

    // ⭐⭐ **THE ONE BOLT TORQUED TO SPEC.** The only move in this table whose
    // knockback grows past [`WITHIN_TOLERANCE_GROWTH`], and therefore the only
    // one that closes a stock. Everything else Oiler does is damage he then has
    // to convert with this, once, correctly — which is what makes a fighter who
    // never whiffs still have to earn something.
    let mut f_smash = strike(
        "smash_forward",
        "smash_forward",
        0.26,
        0.10,
        0.34,
        (40.0, -4.0),
        (28.0, 20.0),
        15,
        150.0,
        TORQUE_GROWTH,
        Some((1.0, -0.42)),
        None,
    );
    f_smash.smash_charge_mult = 1.7;
    let f_smash = vfx(f_smash, 0.0, "tolerance_brackets");
    let f_smash = sfx(f_smash, 0.0, "vfx.oiler.tolerance_brackets");
    let f_smash = vfx(f_smash, 0.26, "wrench_strike");
    let f_smash = sfx(f_smash, 0.26, "vfx.oiler.wrench_strike");
    let f_smash = vfx(f_smash, 0.28, "brass_spark");
    let f_smash = sfx(f_smash, 0.28, "vfx.oiler.brass_spark");
    let f_smash = on_contact(f_smash, "player.robot.slash.impact.metal.gong");

    // A bearing thrown straight up out of the housing.
    let mut up_smash = strike(
        "smash_up",
        "attack_up",
        0.24,
        0.11,
        0.32,
        (6.0, -34.0),
        (22.0, 30.0),
        12,
        130.0,
        2.05,
        Some((0.0, -1.0)),
        None,
    );
    up_smash.smash_charge_mult = 1.7;
    let up_smash = vfx(up_smash, 0.24, "bearing_ping");
    let up_smash = sfx(up_smash, 0.24, "vfx.oiler.bearing_ping");
    let up_smash = on_contact(up_smash, "player.robot.slash.impact.metal.gong");

    // Oil slapped out both sides at his feet — the widest box in the table and
    // his answer to being surrounded on a ledge.
    let mut down_smash = strike(
        "smash_down",
        "attack_down",
        0.22,
        0.12,
        0.34,
        (0.0, 16.0),
        (38.0, 12.0),
        11,
        120.0,
        1.95,
        Some((0.9, -0.50)),
        None,
    );
    down_smash.smash_charge_mult = 1.7;
    let down_smash = vfx(down_smash, 0.22, "oil_splash");
    let down_smash = sfx(down_smash, 0.22, "vfx.oiler.oil_splash");
    let down_smash = vfx(down_smash, 0.30, "oil_slick");
    let down_smash = sfx(down_smash, 0.30, "vfx.oiler.oil_slick");
    let down_smash = on_contact(down_smash, "player.robot.slash.impact.metal.gong");

    // ── the aerials ──────────────────────────────────────────────────────────
    //
    // ⚠ every one of them autocancels LATER than its landing lag is long, so
    // Oiler's air game is a real commitment and a rising short-hop aerial is not
    // a free approach.

    // ⭐ the longest hitbox in the table: he swings the spanner all the way
    // round himself. `unit_circle_rotation` is exactly what that looks like, and
    // it is the row this move was named for.
    let mut n_air = strike(
        "air_neutral",
        "attack_side",
        0.07,
        0.16,
        0.16,
        (0.0, 0.0),
        (26.0, 22.0),
        5,
        62.0,
        1.30,
        None,
        None,
    );
    n_air.landing_lag_s = Some(0.12);
    n_air.autocancel_after_s = Some(0.26);
    let n_air = vfx(n_air, 0.07, "unit_circle_rotation");
    let n_air = sfx(n_air, 0.07, "vfx.oiler.unit_circle_rotation");

    let mut f_air = strike(
        "air_forward",
        "attack_side",
        0.12,
        0.11,
        0.20,
        (30.0, -2.0),
        (24.0, 18.0),
        8,
        96.0,
        1.80,
        Some((1.0, -0.30)),
        None,
    );
    f_air.landing_lag_s = Some(0.16);
    f_air.autocancel_after_s = Some(0.30);
    let f_air = vfx(f_air, 0.12, "curve_trace");
    let f_air = sfx(f_air, 0.12, "vfx.oiler.curve_trace");
    let f_air = on_contact(f_air, "player.robot.slash.impact.metal.chink");

    // The hardest thing he can throw that is not the torque smash — and it faces
    // the wrong way, which is the genre's oldest trade.
    let mut b_air = strike(
        "air_back",
        "attack_side",
        0.13,
        0.10,
        0.22,
        (-30.0, 0.0),
        (24.0, 18.0),
        9,
        108.0,
        WITHIN_TOLERANCE_GROWTH,
        Some((-1.0, -0.34)),
        None,
    );
    b_air.landing_lag_s = Some(0.18);
    b_air.autocancel_after_s = Some(0.32);
    let b_air = vfx(b_air, 0.13, "bearing_ping");
    let b_air = sfx(b_air, 0.13, "vfx.oiler.bearing_ping");
    let b_air = on_contact(b_air, "player.robot.slash.impact.metal.gong");

    let mut u_air = strike(
        "air_up",
        "attack_up",
        0.09,
        0.12,
        0.18,
        (2.0, -30.0),
        (20.0, 24.0),
        6,
        84.0,
        1.70,
        Some((0.0, -1.0)),
        None,
    );
    u_air.landing_lag_s = Some(0.14);
    u_air.autocancel_after_s = Some(0.28);
    let u_air = vfx(u_air, 0.09, "chalk_spiral");
    let u_air = sfx(u_air, 0.09, "vfx.oiler.chalk_spiral");

    // ⚠ no pogo rebound. A body that could bounce off a victim would out-recover
    // the geyser, and the geyser is supposed to be the decision.
    let mut d_air = strike(
        "air_down",
        "attack_down",
        0.13,
        0.12,
        0.24,
        (4.0, 28.0),
        (20.0, 20.0),
        10,
        112.0,
        1.85,
        Some((0.0, 1.0)),
        None,
    );
    d_air.landing_lag_s = Some(0.26);
    d_air.autocancel_after_s = Some(0.36);
    let d_air = vfx(d_air, 0.13, "oil_drip");
    let d_air = sfx(d_air, 0.13, "vfx.oiler.oil_drip");
    let d_air = on_contact(d_air, "player.robot.slash.impact.metal.chink");

    // ── THE FOUR SPECIALS ────────────────────────────────────────────────────
    //
    // ⭐ **four MECHANISMS, and none of them is another one rotated.** One lands
    // three times on one press and never moves him; one commands a slide he can
    // still steer; one commands a rise he cannot; one adds to whatever he was
    // already doing, at the press. `the_four_specials_are_four_mechanisms`
    // asserts each of those four properties and that no two moves share one.

    // **NEUTRAL — `convergence`.** Three taps at closing intervals, each harder
    // than the last, and the error term collapses on the third.
    //
    // ⭐ **it genuinely multi-hits, and the reason is a GAP.** The move runtime
    // hands a hit set forward between windows that touch, precisely so a swing
    // sampled at keyframes cannot bill a victim once per segment — and a window
    // that starts after a gap is a box that went away and came back, which
    // rehits. So the empty 0.06s and 0.04s between these three is the move.
    let convergence = strike(
        "convergence",
        "special",
        0.14,
        0.06,
        0.30,
        (30.0, -2.0),
        (24.0, 18.0),
        3,
        40.0,
        1.10,
        None,
        None,
    );
    // ⚠ tagged BEFORE the later terms are pushed: the first two are jabs and the
    // third is the swing they were converging on, so re-tagging afterwards would
    // flatten exactly the distinction.
    let mut convergence = strike_tag(convergence, SLASH_POKE_VFX);
    // The second and third terms, authored as windows rather than as two more
    // moves: same press, same clock, closing gaps.
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
    let convergence = sfx(convergence, 0.0, "vfx.oiler.tolerance_brackets");
    let convergence = vfx(convergence, 0.14, "convergence_ticks");
    let convergence = sfx(convergence, 0.14, "vfx.oiler.convergence_ticks");
    let convergence = vfx(convergence, 0.26, "convergence_ticks");
    let convergence = sfx(convergence, 0.26, "vfx.oiler.convergence_ticks");
    let convergence = vfx(convergence, 0.36, "error_term_collapse");
    let convergence = sfx(convergence, 0.36, "vfx.oiler.error_term_collapse");
    let convergence = on_contact(convergence, "player.robot.slash.impact.metal.chink");

    // **SIDE — `slick_dash`.** He oils the floor under himself and goes.
    //
    // ⭐⭐ **the tail does NOT lock his steering, and that inversion is the
    // move.** Every other committed charge in this repo ends in a
    // `motion_scale: 0.0` tail — *you decided, now live with it*. Oil is the
    // opposite failure: you keep every bit of input authority you had and none
    // of your ability to STOP. So the tail runs long and leaves `motion_scale`
    // at 1.0, which reads as a slide he is steering and cannot abort — and makes
    // this the one displacing move in the table that can be aimed after it
    // starts, at the cost of being the one that most easily carries him off the
    // stage.
    let side_b = strike(
        "slick_dash",
        "special",
        0.16,
        0.12,
        0.26,
        (32.0, 4.0),
        (26.0, 18.0),
        10,
        105.0,
        1.90,
        Some((0.9, -0.35)),
        None,
    );
    // ⚠ exactly horizontal, so it advertises no lift and the recovery search is
    // never offered a way home that is really a way off. That is a CONTENT
    // decision: Oiler's way home is the geyser.
    let side_b = impulse(side_b, 0.16, (720.0, 0.0), ImpulseMode::Set);
    let side_b = committed_tail(side_b, 0.66, 1.0);
    let side_b = vfx(side_b, 0.16, "oil_slick");
    let side_b = sfx(side_b, 0.16, "vfx.oiler.oil_slick");
    let side_b = vfx(side_b, 0.34, "oil_drip");
    let side_b = sfx(side_b, 0.34, "vfx.oiler.oil_drip");
    let side_b = on_contact(side_b, "player.robot.slash.impact.metal.chink");

    // **UP — `oil_geyser`. THE RECOVERY, and the move Jon asked for by name.**
    //
    // ⭐⭐ he does not jump; the stage throws him. A pressure line lets go under
    // his feet and he rides the column. The rise is COMMANDED (`Set`) at
    // [`GEYSER_AT_S`], after a windup you can see, so a falling Oiler gets
    // exactly the climb a standing one does.
    //
    // ⭐ **the three-row set is one staged effect and it plays in order**:
    // `oil_geyser_emerge` while the ground swells (the tell, and the other
    // player's cue to go edgeguard), `oil_geyser_stream` three times over the
    // climb so the column reads as continuous rather than as one puff, and
    // `oil_geyser_impact` at the crest. Those three rows were authored as a
    // sequence; this is the sequence.
    //
    // ⛔ **it is not flight, and the arithmetic is the reason rather than a
    // cooldown.** No `Cancelable` window means he cannot re-press until the move
    // ends, and the move outlasts its own arc — so repeated use LOSES height.
    // That is a property of the numbers, held by a test, and it costs no
    // rollback state at all.
    let mut up_b = strike(
        "oil_geyser",
        "special",
        GEYSER_AT_S,
        0.14,
        0.18,
        (0.0, 16.0),
        (20.0, 36.0),
        8,
        92.0,
        1.85,
        Some((0.0, -1.0)),
        None,
    );
    // Landing out of the column costs. Onstage that makes it a bad panic button;
    // offstage it is irrelevant, which is the right shape for a way home.
    up_b.landing_lag_s = Some(0.30);
    let up_b = impulse(up_b, GEYSER_AT_S, (0.0, -GEYSER_SPEED), ImpulseMode::Set);
    // The helpless tail. `0.12` leaves him able to nudge where he lands and
    // nothing more, which is what makes edgeguarding this possible.
    let up_b = committed_tail(up_b, GEYSER_ENDS_S, 0.12);
    let up_b = vfx(up_b, 0.06, "oil_geyser_emerge");
    let up_b = sfx(up_b, 0.06, "vfx.oiler.oil_geyser_emerge");
    let up_b = vfx(up_b, GEYSER_AT_S, "oil_geyser_stream");
    // ⛔ the `.loop` suffix is REAL — see the module doc. The derived cue name
    // for this row misses the bank; this one does not.
    let up_b = sfx(up_b, GEYSER_AT_S, "vfx.oiler.oil_geyser_stream.loop");
    let up_b = vfx(up_b, 0.44, "oil_geyser_stream");
    let up_b = vfx(up_b, 0.66, "oil_geyser_stream");
    let up_b = vfx(up_b, 0.88, "oil_geyser_impact");
    let up_b = sfx(up_b, 0.88, "vfx.oiler.oil_geyser_impact");
    let up_b = on_contact(up_b, "player.hit");

    // **DOWN — `pressure_vent`.** He cracks a valve and everything in the seal
    // goes at once.
    //
    // ⭐ the only move in the table displaced by `start_impulse`: it fires at the
    // PRESS and it ADDS, so it contributes to whatever fall he was already in
    // rather than replacing it. That is the exact opposite of the geyser's `Set`
    // and it is why this is a fast-fall punish rather than a second way home —
    // thrown from a standstill it barely moves him, thrown out of a dive it
    // drives him through the floor.
    let mut down_b = strike(
        "pressure_vent",
        "special",
        0.10,
        0.12,
        0.26,
        (0.0, 22.0),
        (30.0, 18.0),
        9,
        100.0,
        1.80,
        Some((0.0, 1.0)),
        None,
    );
    down_b.start_impulse = Some((0.0, 520.0));
    let down_b = vfx(down_b, 0.10, "pressure_vent");
    let down_b = sfx(down_b, 0.10, "vfx.oiler.pressure_vent");
    let down_b = vfx(down_b, 0.12, "brass_spark");
    let down_b = sfx(down_b, 0.12, "vfx.oiler.brass_spark");
    let down_b = on_contact(down_b, "player.robot.slash.impact.metal.gong");

    let repertoire = SmashRepertoire {
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
        up_special: up_b,
        down_special: DownSpecial::OneForm(down_b),
    }
    .into_contract();

    // ⭐ **the tolerance band is checked WHERE IT IS AUTHORED**, not only in the
    // test module. A move edited under it stops being Oiler's before anything
    // else notices, and this is the last place that still holds the whole table
    // at once.
    debug_assert!(
        repertoire
            .moves
            .iter()
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
            shape: VolumeShape::Rect {
                offset: (30.0, -2.0),
                half_extents: (24.0, 18.0),
            },
            damage,
            knockback,
            knockback_growth,
            launch_dir,
            on_hit: None,
            vfx: Some(tag.to_string()),
            hit_sfx: None,
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
    use super::*;
    use ambition_platformer2d::entity_catalog::{AttackDir, MoveEventKind};

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
            .map(|v| v.knockback_growth)
            .fold(0.0f32, f32::max)
    }

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

    /// **THE TOLERANCE BAND, AS AN ASSERTION.**
    ///
    /// ⭐ the claim the module doc makes and the first thing a careless retune
    /// would take away: no Oiler move closes its window inside
    /// [`TOLERANCE_S`]. A move that drifted under it would be a perfectly
    /// reasonable poke and would quietly make him somebody else.
    ///
    /// ⛔ **the poison is the GOBLIN**, the other fighter this crate authors
    /// from the same primitives. If its windows were this wide too, the band
    /// would be describing the helper rather than the character.
    #[test]
    fn every_move_holds_its_hitbox_for_the_tolerance_band() {
        let oiler = oiler_moveset();
        for m in &oiler.moves {
            let held = total_active_s(m);
            assert!(
                held + 1e-4 >= TOLERANCE_S,
                "`{}` keeps a box in the world for {held}s, inside the band this \
                 fighter works to ({TOLERANCE_S}s)",
                m.id
            );
        }

        let goblin = crate::goblin_moveset::goblin_moveset();
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

    /// **EXACTLY ONE BOLT IS TORQUED TO SPEC.**
    ///
    /// ⭐ the other half of the character: forgiving everywhere, lethal in one
    /// place. The forward smash grows at [`TORQUE_GROWTH`] and nothing else may
    /// pass [`WITHIN_TOLERANCE_GROWTH`], so Oiler racks damage all match and
    /// then has to land one specific move to convert it.
    ///
    /// ⛔ **the poison is the goblin again**, which has FOUR moves above the
    /// same line — an ordinary fighter's spread. Without it "one kill move" is
    /// just a description of a low table.
    #[test]
    fn only_one_move_grows_past_the_tolerance_band() {
        let oiler = oiler_moveset();
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

        let goblin = crate::goblin_moveset::goblin_moveset();
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

    /// **THE GEYSER IS A SAVE, NOT A FLIGHT — and the arithmetic is the reason.**
    ///
    /// ⭐⭐ this is what lets the Up-B exist with **no cooldown, no per-airtime
    /// counter and no new rollback state**. He cannot re-press while the move is
    /// playing (no `Cancelable` window), so the only question is whether one full
    /// cycle gains height. It cannot: the move outlasts its own arc, so by the
    /// time he may press again he has fallen back through everything the column
    /// bought and then some.
    ///
    /// ⛔ the failure this forbids is silent and total — shorten the tail and
    /// spamming Up-B becomes hovering, which ends the stage as a platform
    /// fighter.
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
        // Landing out of it costs, so it is a bad panic button ON the stage.
        let up_b = find(&oiler_moveset(), "oil_geyser");
        assert!(up_b.landing_lag_s.unwrap_or(0.0) > 0.0);
        // And the tail is helpless: a recovery you can steer freely out of is a
        // second jump. ⛔ measured at the END of the move, not by finding SOME
        // Recovery window — `strike` builds one of those at `motion_scale: 1.0`
        // for every move in the repo, so `any(..)` would be answering about the
        // helper.
        assert_eq!(up_b.duration_s, GEYSER_ENDS_S);
        assert!(
            up_b.motion_scale_at(GEYSER_ENDS_S - 0.01) < 0.5,
            "the ride down is supposed to be helpless"
        );
    }

    /// **THE RISE IS COMMANDED, NOT CONTRIBUTED — and it is the only one.**
    ///
    /// ⛔ the whole difference between a recovery and a hop. Under
    /// `ImpulseMode::Add` an Oiler falling at terminal velocity would climb at
    /// whatever was left over. `Set` makes the climb a property of the MOVE.
    ///
    /// ⭐ and the same fact is what every policy layer reads: `lift_speed` is
    /// derived from `Set` impulses only, so this is also the assertion that the
    /// brain and the recovery probe can SEE this move.
    #[test]
    fn the_geyser_commands_its_rise_and_is_the_only_way_home() {
        let set = oiler_moveset();
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

        // ⛔ **the poison: nothing ELSE advertises a lift.** A table where every
        // move looked like a recovery would satisfy the assertion above and tell
        // a policy layer nothing.
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

    /// **THE GEYSER PLAYS ALL THREE OF ITS AUTHORED ROWS, IN ORDER.**
    ///
    /// ⭐ `oil_geyser_{emerge,stream,impact}` were rendered as a staged effect
    /// and this is the move that stages them: the ground swells before the
    /// column exists, the column runs for the whole climb, and the crest breaks
    /// last. ⛔ a reordering — or the stream firing once and stopping — is
    /// exactly the "decorative particle" this move was asked not to be, and
    /// nothing else in the tree would notice.
    #[test]
    fn the_geyser_stages_its_three_rows_in_order() {
        let up_b = find(&oiler_moveset(), "oil_geyser");
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

    /// **FOUR SPECIALS, FOUR MECHANISMS.**
    ///
    /// ⛔ four specials built out of the same strike with different offsets would
    /// be one move rotated four ways. So the assertion is about MECHANISM: one
    /// lands three times on one press and never displaces him, one commands a
    /// slide he can still steer, one commands a rise he cannot, and one adds to
    /// whatever he was already doing at the press. No two share a mechanism.
    #[test]
    fn the_four_specials_are_four_mechanisms() {
        let set = oiler_moveset();
        let commanded = |id: &str| -> Option<(f32, f32)> {
            find(&set, id).events.iter().find_map(|e| match &e.kind {
                MoveEventKind::Impulse {
                    local,
                    mode: ImpulseMode::Set,
                } => Some(*local),
                _ => None,
            })
        };

        // Neutral: no displacement of any kind — it lands THREE times instead.
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
        // ⭐ and the GAPS are what make it rehit — contiguous windows hand their
        // hit set forward, so a series with no gap would bill once.
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
        // ⛔ the tail has to EXIST — a `committed_tail` that never extended the
        // move would leave `strike`'s own 1.0 recovery window answering below.
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

        // Up: a rise, and only a rise, with a tail that DOES lock — measured the
        // same way, so the two answers are comparable.
        let up = commanded("oil_geyser").expect("the Up-B displaces");
        assert!(up.1 < 0.0 && up.0 == 0.0);
        let geyser = find(&set, "oil_geyser");
        assert!(geyser.motion_scale_at(geyser.duration_s - 0.01) < 0.5);

        // Down: displaced at the PRESS and additively — the only `start_impulse`
        // in the table, so it cannot be any of the three above.
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

    /// **EVERY BURST IN THIS TABLE IS HEARD.**
    ///
    /// ⛔⛔ **the defect this exists to forbid is silent by construction.** The
    /// paired `vfx.<family>.<row>` cue is only looked up on the `FxRequest`
    /// path; a move's `Vfx` event writes `VfxMessage::Effect` directly and
    /// spawns no sound at all. So a perfectly correct effect name plays a
    /// perfectly silent animation, and the only way to notice is to watch the
    /// match with the volume up.
    ///
    /// ⚠ the geyser's stream is the one exception and it is authored as one: it
    /// is struck three times to keep the column alive, and its loop cue is
    /// started once rather than restarted on every strike.
    #[test]
    fn every_burst_in_this_table_is_heard() {
        let set = oiler_moveset();
        for m in &set.moves {
            let cues: std::collections::BTreeSet<String> = m
                .events
                .iter()
                .filter_map(|e| match &e.kind {
                    MoveEventKind::Sfx { cue } => Some(cue.clone()),
                    _ => None,
                })
                .collect();
            let bursts: std::collections::BTreeSet<String> = m
                .events
                .iter()
                .filter_map(|e| match &e.kind {
                    MoveEventKind::Vfx { effect, .. } => Some(effect.clone()),
                    _ => None,
                })
                .collect();
            assert!(
                !bursts.is_empty(),
                "`{}` throws no effect at all — it is one of Oiler's, and he has \
                 twenty-three of them",
                m.id
            );
            for burst in &bursts {
                assert!(
                    cues.iter().any(|c| c.contains(burst.as_str())),
                    "`{}` shows `{burst}` and never names a cue for it, so it \
                     plays in silence",
                    m.id
                );
            }
        }
    }

    /// **THE ART IS OILER'S OWN, AND IT ALL EXISTS.**
    ///
    /// Two claims that fail together: a table naming an effect no shipped sheet
    /// carries has feedback that silently never plays, and a table drawing
    /// somebody else's bursts is a fighter with no look.
    /// ⭐ the oracle is the ART — `is_authored_effect` reads the rows out of the
    /// baked manifests — so this asks exactly what the renderer will ask.
    #[test]
    fn the_kit_looks_like_oiler_and_the_art_all_ships() {
        let set = oiler_moveset();
        let mut effects = std::collections::BTreeSet::new();
        for m in &set.moves {
            for problem in
                m.presentation_problems(ambition_platformer2d::sprite_sheet::fx::is_authored_effect)
            {
                panic!("{problem}");
            }
            for ev in &m.events {
                if let MoveEventKind::Vfx { effect, .. } = &ev.kind {
                    effects.insert(effect.clone());
                }
            }
        }
        assert!(
            effects.len() >= 12,
            "a jab, a smash, a launcher, four specials and a recovery cannot all \
             look the same: {effects:?}"
        );
        // ⭐ and every one of them comes off HIS sheet. This is the assertion the
        // twenty-three rendered rows were waiting for.
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

    /// **EVERY PRESS A BODY CAN MAKE REACHES A MOVE, IN BOTH POSTURES.**
    ///
    /// ⭐ this is what the CPU's kit builder enumerates and what a human's stick
    /// resolves — the same function, so a repertoire that answers here answers
    /// for both.
    #[test]
    fn both_postures_reach_at_least_eight_distinct_moves() {
        let set = oiler_moveset();
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
        // The recovery is reachable from BOTH — a move you have to fall off the
        // stage to practise is a move nobody learns.
        assert!(on_ground.contains("oil_geyser"));
        assert!(airborne.contains("oil_geyser"));
        // ⛔ and the forward press does not fall through to the jab.
        assert_eq!(
            set.move_for_directional_verb("attack", AttackDir::Forward, true)
                .map(|m| m.id.as_str()),
            Some("tilt_forward")
        );
    }

    /// **EVERY MOVE NAMES A CLIP THE SHEET ACTUALLY DRAWS.**
    ///
    /// ⛔ the reason this table exists at all: Oiler's sheet published four rows
    /// (`idle`, `walk`, `talk`, `interact`), so every swing he could have thrown
    /// would have fallen down the structural chain and drawn `idle`. A move that
    /// draws the standing pose is a move nobody can read, and it costs the
    /// gameplay nothing — which is exactly why it stays broken.
    ///
    /// ⚠ the oracle is the BAKED sheet record, so this fails the day somebody
    /// republishes the sheet without the fight rows.
    #[test]
    fn every_move_names_a_row_the_published_sheet_carries() {
        let record =
            ambition_platformer2d::sprite_sheet::character::sheets::record_for_target("oiler")
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
        for m in &oiler_moveset().moves {
            assert!(
                rows.contains(m.clip.clip.as_str()),
                "`{}` draws `{}`, which the published sheet does not have, so it \
                 falls all the way back to the standing pose. Rows: {rows:?}",
                m.id,
                m.clip.clip
            );
        }
    }
}

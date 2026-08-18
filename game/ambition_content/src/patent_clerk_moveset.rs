//! **The Patent Clerk's repertoire** — the heavyweight, written from the row's own
//! `gameplay_description` rather than from taste.
//!
//! ⭐ **the third adopter removed from `smash_fighter_kit()`** (campaign P3.24).
//!
//! ⚠ **the character had already been designed and nobody had read it back.** Its
//! catalog row carries a `gameplay_description` in full:
//!
//! > *A high-mastery heavyweight controller who classifies bodies as MASS,
//! > ENERGY, MOVING, or AT REST; manipulates relative velocity and local reference
//! > frames; and turns careful observation into unusually strong parries and
//! > finishers.*
//!
//! Three words in that decide this table — **heavyweight**, **controller**,
//! **finishers** — and they are not mine:
//!
//! * *heavyweight* → the slowest startups in the game and the largest volumes.
//!   Every swing here is a commitment; the clerk does not poke.
//! * *controller* → the tilts SET UP rather than kill. The down-tilt sends along
//!   the floor, the up-tilt pops straight up: both leave the opponent somewhere
//!   the clerk chose, which is what a controller trades damage for.
//! * *finishers* → and then the smashes are the hardest in the game, because a
//!   body this slow only ever gets the one.
//!
//! ⛔ **the CLASSIFICATION mechanic is NOT in here** — MASS/ENERGY/MOVING/AT REST,
//! the reference-frame manipulation. Those are systems, not swings. A moveset
//! says what a hit IS; a mechanic that changes what a body's velocity MEANS
//! belongs to the character's abilities and the ruleset. Writing them as move
//! windows would be the wholesale-migration failure mode wearing a content
//! commit.
//!
//! ## 2026-08-16: the five that were missing, and the effects
//!
//! Jon: *"We need to make sure they also have full smash movesets."* Eleven of
//! the sixteen were here, and the five that were not are the ones a fighter
//! cannot do without — **a forward tilt and the four specials, one of which is
//! his only way back to the stage.** A body with no recovery loses every stock
//! to the blast zone, which is the defect the whole showcase is currently
//! blocked on.
//!
//! ⚠ **that does not reopen the paragraph above.** An elevator that RISES is a
//! swing with an authored impulse — the vocabulary every recovery in this repo
//! is built from. What stays out is the part that would redefine velocity for
//! everybody: `reference_frame` here displaces HIM, and says nothing about what
//! anybody else's motion means.
//!
//! ⛔ **the eleven original moves are untouched.** Their damage, timing, volumes
//! and launch angles are exactly as the heavyweight/controller/finisher reading
//! set them; a retune riding a "make it complete" commit is how a design gets
//! lost. What they gained is their own art: fourteen rendered effects that were
//! bound to nothing.
//!
//! ⛔ three cues carry a `.loop` suffix their sprite rows do not
//! (`reference_frame_grid`, `proper_time_tick`, `simultaneity_slice`), so the
//! derived `vfx.<family>.<row>` name misses the bank for those three.

use ambition_characters::smash_repertoire::{DownSpecial, NeutralSpecial, SmashRepertoire};
use ambition_platformer2d::entity_catalog::{ImpulseMode, MoveSpec, MovesetContract};

use ambition_characters::moveset_authoring::{
    committed_tail, impulse, on_contact, strike, vfx_at, vfx_cued,
};

/// Burst sizes, as multiples of the presentation default. Jon, 2026-08-16:
/// *"try to make the hitboxes and vfx placement make sense, right now we are
/// seeing crazy upscaled vfx"*. A stamp is small; a light cone is not.
const STAMP_FX: f32 = 0.55;
const SWING_FX: f32 = 0.80;
const PROOF_FX: f32 = 1.35;

/// The rise `elevator_thought` commands — the equivalence principle as a
/// recovery. A SPEED applied with [`ImpulseMode::Set`], for the reason every
/// recovery here is: a clerk pressing this at terminal velocity gets the climb a
/// standing one does.
pub const ELEVATOR_SPEED: f32 = 920.0;
pub const ELEVATOR_AT_S: f32 = 0.22;
/// ⛔ not a feel number: the tail must outlast the arc or repeated presses gain
/// height, which is flight. `the_elevator_is_a_save_and_not_a_flight` holds it.
pub const ELEVATOR_ENDS_S: f32 = 1.18;

/// The least steering any window of a move leaves its owner.
pub fn tightest_lock(spec: &MoveSpec) -> f32 {
    spec.windows
        .iter()
        .map(|w| w.motion_scale)
        .fold(f32::MAX, f32::min)
}

/// See the module doc. Sixteen moves, the genre's standard verb map.
pub fn patent_clerk_moveset() -> MovesetContract {
    // ── grounded ─────────────────────────────────────────────────────────────
    //
    // ⚠ the slowest jab in the game, and it is supposed to be. A heavyweight's
    // fast option is still a decision — 0.08s is long enough that a goblin can
    // walk into it, hit twice and leave.
    let jab = strike(
        "jab",
        "jab",
        0.08,
        0.08,
        0.20,
        (28.0, 0.0),
        (22.0, 18.0),
        5,
        60.0,
        1.10,
        None,
        None,
    );
    let jab = vfx_at(jab, 0.08, "stamp_at_rest", (28.0, 0.0), STAMP_FX);

    // CONTROLLER, not killer: it pops them straight up, at a launch too weak to
    // finish anybody. What it buys is the next four moves happening above a body
    // that cannot walk away.
    let up_tilt = strike(
        "tilt_up",
        "attack_up",
        0.10,
        0.09,
        0.20,
        (8.0, -30.0),
        (22.0, 26.0),
        6,
        75.0,
        1.15,
        Some((0.0, -1.0)),
        None,
    );
    let up_tilt = vfx_cued(
        up_tilt,
        0.10,
        "proper_time_tick",
        (8.0, -30.0),
        SWING_FX,
        "vfx.patent_clerk.proper_time_tick.loop",
    );
    let up_tilt = on_contact(up_tilt, "player.hit");

    // The other half of the setup: along the floor, almost no vertical. They end
    // up at the ledge, which is where the clerk wants everybody.
    let down_tilt = strike(
        "tilt_down",
        "attack_down",
        0.09,
        0.08,
        0.19,
        (28.0, 14.0),
        (24.0, 10.0),
        5,
        70.0,
        1.20,
        Some((1.0, -0.10)),
        None,
    );
    let down_tilt = vfx_cued(
        down_tilt,
        0.09,
        "simultaneity_slice",
        (28.0, 14.0),
        SWING_FX,
        "vfx.patent_clerk.simultaneity_slice.loop",
    );
    let down_tilt = on_contact(down_tilt, "player.hit");

    // ── smashes: the FINISHERS ───────────────────────────────────────────────
    //
    // ⚠ the hardest hits in the game, on the longest commitments in the game. A
    // body this slow gets one of these per stock if it is playing well, so it has
    // to be the one that ends things.
    let mut f_smash = strike(
        "smash_forward",
        "smash_forward",
        0.38,
        0.08,
        0.42,
        (42.0, -4.0),
        (30.0, 24.0),
        19,
        175.0,
        3.30,
        Some((1.0, -0.45)),
        None,
    );
    f_smash.smash_charge_mult = 1.7;
    let f_smash = vfx_at(f_smash, 0.06, "stamp_mass", (0.0, -8.0), STAMP_FX);
    let f_smash = vfx_at(
        f_smash,
        0.38,
        "mass_energy_exchange",
        (42.0, -4.0),
        PROOF_FX,
    );
    let f_smash = vfx_at(f_smash, 0.44, "stamp_energy", (42.0, -4.0), STAMP_FX);
    let f_smash = on_contact(f_smash, "player.hit");

    let mut up_smash = strike(
        "smash_up",
        "smash_up",
        0.34,
        0.10,
        0.38,
        (6.0, -36.0),
        (26.0, 32.0),
        17,
        170.0,
        3.15,
        Some((0.0, -1.0)),
        None,
    );
    up_smash.smash_charge_mult = 1.7;
    let up_smash = vfx_at(up_smash, 0.34, "light_cone", (6.0, -36.0), PROOF_FX);
    let up_smash = on_contact(up_smash, "player.hit");

    let mut down_smash = strike(
        "smash_down",
        "smash_down",
        0.32,
        0.11,
        0.40,
        (0.0, 16.0),
        (42.0, 13.0),
        15,
        155.0,
        2.85,
        Some((0.95, -0.45)),
        None,
    );
    down_smash.smash_charge_mult = 1.7;
    let down_smash = vfx_at(down_smash, 0.32, "clock_desync", (0.0, 16.0), SWING_FX);
    let down_smash = on_contact(down_smash, "player.hit");

    // ── aerials ──────────────────────────────────────────────────────────────
    //
    // Big and slow in the air too, with one exception below.
    let n_air = strike(
        "air_neutral",
        "air_neutral",
        0.09,
        0.14,
        0.18,
        (0.0, 0.0),
        (28.0, 26.0),
        7,
        75.0,
        1.30,
        None,
        None,
    );
    let n_air = vfx_at(
        n_air,
        0.09,
        "relative_velocity_arrows",
        (0.0, 0.0),
        SWING_FX,
    );
    let n_air = on_contact(n_air, "player.hit");

    let f_air = strike(
        "air_forward",
        "air_forward",
        0.13,
        0.09,
        0.22,
        (32.0, -2.0),
        (26.0, 20.0),
        10,
        105.0,
        1.95,
        Some((1.0, -0.28)),
        None,
    );
    let f_air = vfx_at(f_air, 0.13, "stamp_moving", (32.0, -2.0), SWING_FX);
    let f_air = on_contact(f_air, "player.hit");

    let b_air = strike(
        "air_back",
        "air_back",
        0.15,
        0.08,
        0.26,
        (-34.0, 0.0),
        (26.0, 20.0),
        12,
        145.0,
        2.60,
        Some((-1.0, -0.35)),
        None,
    );
    let b_air = vfx_at(b_air, 0.15, "stamp_at_rest", (-34.0, 0.0), SWING_FX);
    let b_air = on_contact(b_air, "player.hit");

    let u_air = strike(
        "air_up",
        "air_up",
        0.10,
        0.10,
        0.19,
        (2.0, -32.0),
        (22.0, 26.0),
        8,
        95.0,
        1.90,
        Some((0.0, -1.0)),
        None,
    );
    let u_air = vfx_at(u_air, 0.10, "light_cone", (2.0, -32.0), SWING_FX);
    let u_air = on_contact(u_air, "player.hit");

    // ⭐ the exception, and the one place *AT REST* shows up as a swing: it stops
    // dead and drops. Straight down, no drift, the heaviest spike in the game.
    let d_air = strike(
        "air_down",
        "air_down",
        0.16,
        0.09,
        0.28,
        (4.0, 30.0),
        (22.0, 22.0),
        13,
        140.0,
        2.40,
        Some((0.0, 1.0)),
        None,
    );
    let d_air = vfx_at(d_air, 0.16, "mass_energy_exchange", (4.0, 30.0), SWING_FX);
    let d_air = on_contact(d_air, "player.hit");

    // ── 2026-08-16: THE FIVE THAT WERE MISSING ───────────────────────────────

    // ⛔ **a forward tilt, because without one the commonest press in the genre
    // falls down the directional chain to the jab.** The same hole George Booul
    // and Oiler both had. A margin correction: he reaches out and rewrites what
    // you just did.
    let f_tilt = strike(
        "tilt_forward",
        "margin_correction",
        0.12,
        0.09,
        0.22,
        (34.0, -4.0),
        (24.0, 16.0),
        7,
        82.0,
        1.35,
        Some((1.0, -0.30)),
        None,
    );
    let f_tilt = vfx_at(f_tilt, 0.12, "stamp_moving", (34.0, -4.0), SWING_FX);
    let f_tilt = on_contact(f_tilt, "player.hit");

    // **NEUTRAL — `light_argument`.** The speed of light is the same in every
    // frame: no impulse, no drift, a fixed cone that does not care what he was
    // doing when he threw it.
    let n_b = strike(
        "light_argument",
        "light_argument",
        0.22,
        0.12,
        0.32,
        (36.0, -6.0),
        (30.0, 20.0),
        11,
        108.0,
        2.05,
        Some((0.9, -0.45)),
        None,
    );
    let n_b = committed_tail(n_b, 0.66, 0.0);
    let n_b = vfx_at(n_b, 0.22, "light_cone", (36.0, -6.0), PROOF_FX);
    let n_b = on_contact(n_b, "player.hit");

    // **SIDE — `reference_frame`.** He declares a frame and moves in it. ⭐ the
    // impulse fires on the ACTIVE frame rather than the press, and the tail is
    // fully locked, so the pass goes exactly as far as it was going to — which
    // is the heavyweight's version of a dash: no take-backs.
    //
    // ⚠ it displaces HIM and says nothing about anybody else's motion. The
    // reference-frame MECHANIC the module header keeps out stays out.
    let side_b = strike(
        "reference_frame",
        "reference_frame",
        0.20,
        0.11,
        0.28,
        (30.0, 0.0),
        (26.0, 22.0),
        11,
        112.0,
        2.10,
        Some((0.9, -0.40)),
        None,
    );
    let side_b = impulse(side_b, 0.20, (640.0, 0.0), ImpulseMode::Set);
    let side_b = committed_tail(side_b, 0.66, 0.0);
    let side_b = vfx_cued(
        side_b,
        0.20,
        "reference_frame_grid",
        (0.0, 0.0),
        PROOF_FX,
        "vfx.patent_clerk.reference_frame_grid.loop",
    );
    let side_b = vfx_at(
        side_b,
        0.34,
        "relative_velocity_arrows",
        (30.0, 0.0),
        SWING_FX,
    );
    let side_b = on_contact(side_b, "player.hit");

    // **UP — `elevator_thought`. THE RECOVERY, and it is the equivalence
    // principle**: a man in a rising lift cannot tell it from gravity. He does
    // not jump; his frame accelerates and he is in it.
    let mut up_b = strike(
        "elevator_thought",
        "elevator_thought",
        ELEVATOR_AT_S,
        0.12,
        0.20,
        (0.0, 14.0),
        (20.0, 32.0),
        9,
        92.0,
        1.85,
        Some((0.0, -1.0)),
        None,
    );
    // A heavyweight who lands out of the lift owes for it. Offstage that costs
    // nothing, which is the right shape for a way home.
    up_b.landing_lag_s = Some(0.34);
    let up_b = impulse(
        up_b,
        ELEVATOR_AT_S,
        (0.0, -ELEVATOR_SPEED),
        ImpulseMode::Set,
    );
    let up_b = committed_tail(up_b, ELEVATOR_ENDS_S, 0.0);
    let up_b = vfx_at(up_b, 0.06, "elevator_frame", (0.0, 0.0), PROOF_FX);
    let up_b = vfx_cued(
        up_b,
        ELEVATOR_AT_S,
        "proper_time_tick",
        (0.0, 10.0),
        SWING_FX,
        "vfx.patent_clerk.proper_time_tick.loop",
    );
    let up_b = on_contact(up_b, "player.hit");

    // **DOWN — `synchronize_clocks`.** Two clocks, one slice: a wide flat window
    // on the floor either side of him, and the stamp that says it is settled.
    let down_b = strike(
        "synchronize_clocks",
        "synchronize_clocks",
        0.20,
        0.13,
        0.32,
        (0.0, 20.0),
        (44.0, 12.0),
        8,
        86.0,
        1.60,
        Some((0.8, -0.55)),
        None,
    );
    let down_b = committed_tail(down_b, 0.65, 0.0);
    let down_b = vfx_at(down_b, 0.20, "clock_sync", (-30.0, 18.0), SWING_FX);
    let down_b = vfx_at(down_b, 0.26, "clock_desync", (30.0, 18.0), SWING_FX);
    let down_b = vfx_at(down_b, 0.33, "known_result_stamp", (0.0, 4.0), STAMP_FX);
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
    // **DOWN, IN THE AIR — `falling_simultaneity`.** Two clocks still slice one
    // moment with no floor between them; he brings the slice down.
    let mut air_down_b = strike(
        "falling_simultaneity",
        "air_down",
        0.12,
        0.10,
        0.26,
        (0.0, 24.0),
        (22.0, 22.0),
        10,
        98.0,
        1.72,
        Some((0.0, 1.0)),
        None,
    );
    air_down_b.landing_lag_s = Some(0.32);
    let air_down_b = impulse(air_down_b, 0.12, (0.0, 1250.0), ImpulseMode::Set);
    let air_down_b = vfx_at(air_down_b, 0.12, "clock_sync", (0.0, 20.0), SWING_FX);
    let air_down_b = on_contact(air_down_b, "player.hit");

    SmashRepertoire {
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
        // ⚠ **no capture kit yet** — the relationship architecture is being
        // proven on two fighters first (see `SmashCaptureRepertoire`). This is
        // the transitional `None`, and it means exactly one thing: no Grab slot,
        // no grab verbs, nothing about this fighter lying about having one.
        capture: None,
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
    use ambition_platformer2d::entity_catalog::{MoveSpec, WindowTag};

    fn find(set: &MovesetContract, id: &str) -> MoveSpec {
        set.moves
            .iter()
            .find(|m| m.id == id)
            .unwrap_or_else(|| panic!("{id} exists"))
            .clone()
    }

    fn startup(m: &MoveSpec) -> f32 {
        m.windows
            .iter()
            .find(|w| matches!(w.tag, WindowTag::Active))
            .expect("a strike has an active window")
            .start_s
    }

    fn damage(m: &MoveSpec) -> i32 {
        m.windows
            .iter()
            .flat_map(|w| w.volumes.iter())
            .map(|v| v.damage)
            .max()
            .unwrap_or(0)
    }

    fn launch(m: &MoveSpec) -> f32 {
        m.windows
            .iter()
            .flat_map(|w| w.volumes.iter())
            .map(|v| v.knockback)
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

    /// **The row said HEAVYWEIGHT and FINISHERS, and the table has to mean it.**
    ///
    /// Measured against the admiral — the next-heaviest body with a table — so the
    /// claim stays comparative. A heavyweight that is merely *big numbers* is a
    /// balance patch; a heavyweight is slower AND harder than the thing below it.
    #[test]
    fn the_clerk_is_slower_and_hits_harder_than_the_admiral() {
        let clerk = patent_clerk_moveset();
        let admiral = crate::pirate_admiral_moveset::pirate_admiral_moveset();

        assert!(
            startup(&find(&clerk, "jab")) > startup(&find(&admiral, "jab")),
            "the heaviest body has the slowest fast option"
        );
        assert!(
            startup(&find(&clerk, "smash_forward")) > startup(&find(&admiral, "smash_forward")),
            "and the longest commitment on its kill move"
        );
        assert!(
            damage(&find(&clerk, "smash_forward")) > damage(&find(&admiral, "smash_forward")),
            "which is what it is paid for"
        );
    }

    /// **CONTROLLER: the tilts set up, they do not finish.**
    ///
    /// ⭐ the word in the row that is easiest to lose while writing numbers. A
    /// tilt that launches as hard as a smash makes the smash pointless and the
    /// character a brawler — so the gap between them IS the design, and it is
    /// asserted rather than remembered.
    #[test]
    fn the_tilts_set_up_and_the_smashes_finish() {
        let clerk = patent_clerk_moveset();
        let strongest_tilt = ["tilt_up", "tilt_down"]
            .into_iter()
            .map(|id| launch(&find(&clerk, id)))
            .fold(0.0f32, f32::max);
        let weakest_smash = ["smash_forward", "smash_up", "smash_down"]
            .into_iter()
            .map(|id| launch(&find(&clerk, id)))
            .fold(f32::MAX, f32::min);
        assert!(
            strongest_tilt * 2.0 < weakest_smash,
            "a controller's tilts must be worth less than half its finishers \
             ({strongest_tilt} vs {weakest_smash}), or the finisher is decoration"
        );
    }
}

//! Carl Stargan's repertoire — cosmic perspective, as a fighter.
//!
//! Twelve of his effects were rendered and bound to nothing.
//!
//! ## The character, from his own vocabulary
//!
//! A populariser of scale: the pale blue dot is a pixel and the cosmic calendar
//! is fourteen billion years on one page. One idea runs the table:
//!
//! ```text
//!   REACH IS BOUGHT WITH TIME    his longest move is his slowest, always
//! ```
//!
//! Sort his forward line by how far it reaches and the startup never goes down.
//! What it buys is a fighter you can read: the thing that threatens half the
//! stage is also the thing you can see coming, and his jab is instant because it
//! barely leaves his sleeve.
//!
//! that discipline is not his alone — I tried to poison it against Oiler
//! and the poison failed, because his forward line is monotonic too. Recorded
//! rather than quietly dropped: it is a property of tables authored with care,
//! not a distinguishing trait.
//!
//! the SPREAD is what is his. `billions_and_billions` reaches
//! [`FURTHEST_REACH`] against a jab at [`NEAREST_REACH`] — over four times, where
//! the other authored fighters sit far below that — which is "the very small and
//! the very large" stated in hitboxes, and it carries its own comparison.
//!
//! ## Effects sit on the box that throws them
//!
//! one cue carries a `.loop` suffix its row does not
//! (`vfx.carl_stargan.nebula_breath.loop`), so the derived `vfx.<family>.<row>`
//! name misses the bank for that one.

use ambition_characters::moveset_prefabs::{SLASH_ARC_VFX, SLASH_POKE_VFX};
use ambition_characters::smash_capture::{
    author_pummel, author_standing_grab, author_throw, capture_beat, grab_shell,
    CaptureAttemptParams, CaptureCues, CapturePummelParams, CaptureThrowParams,
    SmashCaptureRepertoire,
};
use ambition_characters::smash_repertoire::{DownSpecial, NeutralSpecial, SmashRepertoire};
use ambition_platformer2d::entity_catalog::{ImpulseMode, MoveSpec, MovesetContract, WindowTag};

use ambition_characters::moveset_authoring::{
    committed_tail, impulse, on_contact, strike, strike_tag, vfx_at, vfx_cued,
};

/// Burst sizes, as multiples of the presentation default. See
/// [`crate::emmy_noether_moveset`] for why these are not all the same number.
const POKE_FX: f32 = 0.55;
const SWING_FX: f32 = 0.80;
const COSMIC_FX: f32 = 1.45;

/// The far end of his reach, in world units from his centre — the leading
/// edge of `billions_and_billions`.
pub const FURTHEST_REACH: f32 = 96.0;
/// The near end — the jab, which barely leaves his sleeve.
pub const NEAREST_REACH: f32 = 22.0;

/// The rise `starstuff` commands. Authored as a SPEED and applied with
/// [`ImpulseMode::Set`], for the reason every recovery here is.
pub const STARSTUFF_SPEED: f32 = 900.0;
/// When it takes hold, and when it lets go. the tail outlasts the arc, which
/// is what keeps a recovery from being flight.
pub const STARSTUFF_AT_S: f32 = 0.20;
pub const STARSTUFF_ENDS_S: f32 = 1.14;

/// How far a move's leading edge reaches from the owner's centre.
pub fn reach_of(spec: &MoveSpec) -> f32 {
    spec.windows
        .iter()
        .flat_map(|w| w.volumes.iter())
        .filter_map(|v| match v.shape {
            ambition_platformer2d::entity_catalog::VolumeShape::Rect {
                offset,
                half_extents,
            } => Some(offset.0.abs() + half_extents.0),
            _ => None,
        })
        .fold(0.0_f32, f32::max)
}

/// Does this move reach FORWARD? A volume centred ahead of him, rather than
/// above or below.
///
/// the distinction is what makes [`reach_of`] comparable between moves: an
/// up-smash has a small x-extent because it points up, not because it is short.
pub fn points_forward(spec: &MoveSpec) -> bool {
    spec.windows
        .iter()
        .flat_map(|w| w.volumes.iter())
        .any(|v| match v.shape {
            ambition_platformer2d::entity_catalog::VolumeShape::Rect { offset, .. } => {
                offset.0.abs() >= 12.0
            }
            _ => false,
        })
}

/// The move's startup: the time before its first box exists.
pub fn startup_of(spec: &MoveSpec) -> f32 {
    spec.windows
        .iter()
        .filter(|w| matches!(w.tag, WindowTag::Active) && !w.volumes.is_empty())
        .map(|w| w.start_s)
        .fold(f32::MAX, f32::min)
}

/// See the module doc. Sixteen moves, every clip a row his sheet publishes.
pub fn carl_stargan_moveset() -> MovesetContract {
    // ── near, and instant ────────────────────────────────────────────────────

    let jab = strike(
        "jab",
        "jab",
        0.05,
        0.09,
        0.13,
        (14.0, -6.0),
        (8.0, 12.0),
        3,
        40.0,
        1.05,
        None,
        None,
    );
    let jab = strike_tag(jab, SLASH_POKE_VFX);
    let jab = vfx_at(jab, 0.05, "evidence_ping", (14.0, -6.0), POKE_FX);

    let mut f_tilt = strike(
        "tilt_forward",
        "punch",
        0.09,
        0.10,
        0.19,
        (24.0, -4.0),
        (10.0, 14.0),
        6,
        72.0,
        1.45,
        Some((1.0, -0.30)),
        None,
    );
    f_tilt.start_impulse = Some((120.0, 0.0));
    let f_tilt = vfx_at(f_tilt, 0.09, "perspective_shift", (24.0, -4.0), SWING_FX);
    let f_tilt = on_contact(f_tilt, "player.hit");

    let up_tilt = strike(
        "tilt_up",
        "attack_up",
        0.09,
        0.12,
        0.19,
        (4.0, -24.0),
        (14.0, 20.0),
        6,
        74.0,
        1.50,
        Some((0.1, -1.0)),
        None,
    );
    let up_tilt = vfx_at(
        up_tilt,
        0.09,
        "constellation_resolve",
        (4.0, -24.0),
        SWING_FX,
    );
    let up_tilt = on_contact(up_tilt, "player.hit");

    let down_tilt = strike(
        "tilt_down",
        "attack_down",
        0.10,
        0.12,
        0.20,
        (22.0, 14.0),
        (14.0, 10.0),
        6,
        68.0,
        1.40,
        Some((0.9, -0.35)),
        None,
    );
    let down_tilt = vfx_at(down_tilt, 0.10, "horizon_arc", (22.0, 14.0), SWING_FX);
    let down_tilt = on_contact(down_tilt, "player.hit");

    // ── far, and slow ────────────────────────────────────────────────────────

    // `billions_and_billions`. The furthest anything reaches in this repo,
    // and the longest anyone has to watch it coming.
    let mut f_smash = strike(
        "smash_forward",
        "billions_and_billions",
        // 0.34, and it has to be the largest startup in his forward line —
        // `cosmic_calendar` takes 0.30 and reaches 70. A move that reached
        // further AND arrived sooner would break the one rule this table is
        // authored under, and `reach_is_monotonic_in_startup` caught exactly
        // that at 0.26.
        0.34,
        0.08,
        0.36,
        (58.0, -6.0),
        (38.0, 20.0),
        16,
        126.0,
        2.90,
        Some((1.0, -0.50)),
        None,
    );
    f_smash.smash_charge_mult = 1.80;
    let f_smash = strike_tag(f_smash, SLASH_ARC_VFX);
    let f_smash = vfx_at(f_smash, 0.06, "cosmic_scale_zoom", (0.0, -8.0), SWING_FX);
    let f_smash = vfx_at(f_smash, 0.34, "starstuff_burst", (58.0, -6.0), COSMIC_FX);
    let f_smash = on_contact(f_smash, "player.hit");

    let mut up_smash = strike(
        "smash_up",
        "smash_up",
        0.18,
        0.09,
        0.30,
        (2.0, -34.0),
        (18.0, 26.0),
        13,
        112.0,
        1.90,
        Some((0.0, -1.0)),
        None,
    );
    up_smash.smash_charge_mult = 1.70;
    let up_smash = vfx_at(
        up_smash,
        0.18,
        "constellation_resolve",
        (2.0, -34.0),
        SWING_FX,
    );
    let up_smash = on_contact(up_smash, "player.hit");

    let mut down_smash = strike(
        "smash_down",
        "smash_down",
        0.18,
        0.09,
        0.30,
        (0.0, 20.0),
        (36.0, 12.0),
        13,
        110.0,
        1.85,
        Some((0.0, 1.0)),
        None,
    );
    down_smash.smash_charge_mult = 1.70;
    let down_smash = vfx_at(down_smash, 0.18, "horizon_arc", (0.0, 20.0), SWING_FX);
    let down_smash = on_contact(down_smash, "player.hit");

    // ── the air game ─────────────────────────────────────────────────────────

    let mut n_air = strike(
        "air_neutral",
        "air_neutral",
        0.08,
        0.12,
        0.20,
        (0.0, -6.0),
        (24.0, 22.0),
        8,
        78.0,
        1.60,
        Some((0.5, -0.75)),
        None,
    );
    n_air.landing_lag_s = Some(0.16);
    n_air.autocancel_after_s = Some(0.30);
    let n_air = vfx_cued(
        n_air,
        0.08,
        "nebula_breath",
        (0.0, -6.0),
        SWING_FX,
        "vfx.carl_stargan.nebula_breath.loop",
    );
    let n_air = on_contact(n_air, "player.hit");

    let mut f_air = strike(
        "air_forward",
        "air_forward",
        0.12,
        0.10,
        0.22,
        (30.0, -4.0),
        (18.0, 16.0),
        9,
        86.0,
        1.70,
        Some((1.0, -0.45)),
        None,
    );
    f_air.landing_lag_s = Some(0.18);
    f_air.autocancel_after_s = Some(0.32);
    let f_air = vfx_at(f_air, 0.12, "orbit_lock", (30.0, -4.0), SWING_FX);
    let f_air = on_contact(f_air, "player.hit");

    let mut b_air = strike(
        "air_back",
        "air_back",
        0.10,
        0.09,
        0.22,
        (-26.0, -4.0),
        (16.0, 15.0),
        8,
        90.0,
        1.75,
        Some((-1.0, -0.40)),
        None,
    );
    b_air.landing_lag_s = Some(0.18);
    b_air.autocancel_after_s = Some(0.30);
    let b_air = vfx_at(b_air, 0.10, "orbit_lock", (-26.0, -4.0), SWING_FX);
    let b_air = on_contact(b_air, "player.hit");

    let mut up_air = strike(
        "air_up",
        "air_up",
        0.08,
        0.13,
        0.19,
        (2.0, -28.0),
        (16.0, 22.0),
        7,
        76.0,
        1.55,
        Some((0.0, -1.0)),
        None,
    );
    up_air.landing_lag_s = Some(0.14);
    up_air.autocancel_after_s = Some(0.28);
    let up_air = vfx_at(up_air, 0.08, "voyager_signal", (2.0, -28.0), SWING_FX);
    let up_air = on_contact(up_air, "player.hit");

    let mut d_air = strike(
        "air_down",
        "air_down",
        0.13,
        0.09,
        0.24,
        (0.0, 24.0),
        (16.0, 24.0),
        10,
        98.0,
        1.85,
        Some((0.0, 1.0)),
        None,
    );
    d_air.landing_lag_s = Some(0.26);
    d_air.autocancel_after_s = Some(0.34);
    let d_air = vfx_at(d_air, 0.13, "cosmic_scale_zoom", (0.0, 24.0), SWING_FX);
    let d_air = on_contact(d_air, "player.hit");

    // ── THE FOUR SPECIALS ────────────────────────────────────────────────────

    // NEUTRAL — `cosmic_calendar`. Fourteen billion years on one page: the
    // slowest sweep in the table, and it covers the whole page.
    let n_b = strike(
        "cosmic_calendar",
        "cosmic_calendar",
        0.30,
        0.14,
        0.34,
        (36.0, -4.0),
        (34.0, 26.0),
        11,
        104.0,
        1.80,
        Some((0.8, -0.60)),
        None,
    );
    let n_b = vfx_at(n_b, 0.04, "cosmic_calendar_sweep", (0.0, -6.0), COSMIC_FX);
    let n_b = vfx_at(n_b, 0.30, "perspective_shift", (36.0, -4.0), SWING_FX);
    let n_b = on_contact(n_b, "player.hit");

    // SIDE — `planetary_orbit`. A slingshot: he commits to a pass and comes
    // out of it moving. the impulse fires on the ACTIVE frame, not the press,
    // so the swing carries him THROUGH rather than launching him at nothing.
    let side_b = strike(
        "planetary_orbit",
        "planetary_orbit",
        0.18,
        0.12,
        0.26,
        (30.0, 0.0),
        (24.0, 20.0),
        10,
        102.0,
        1.85,
        Some((0.9, -0.40)),
        None,
    );
    let side_b = impulse(side_b, 0.18, (700.0, 0.0), ImpulseMode::Set);
    let side_b = committed_tail(side_b, 0.62, 0.35);
    let side_b = vfx_at(side_b, 0.18, "planetary_slingshot", (30.0, 0.0), COSMIC_FX);
    let side_b = vfx_at(side_b, 0.36, "orbit_lock", (0.0, 0.0), SWING_FX);
    let side_b = on_contact(side_b, "player.hit");

    // UP — `starstuff`. THE RECOVERY. We are made of it, and it goes up.
    let mut up_b = strike(
        "starstuff",
        "starstuff",
        STARSTUFF_AT_S,
        0.13,
        0.18,
        (0.0, 12.0),
        (18.0, 32.0),
        8,
        90.0,
        1.80,
        Some((0.0, -1.0)),
        None,
    );
    up_b.landing_lag_s = Some(0.28);
    let up_b = impulse(
        up_b,
        STARSTUFF_AT_S,
        (0.0, -STARSTUFF_SPEED),
        ImpulseMode::Set,
    );
    let up_b = committed_tail(up_b, STARSTUFF_ENDS_S, 0.12);
    let up_b = vfx_at(up_b, 0.06, "voyager_signal", (0.0, 0.0), SWING_FX);
    let up_b = vfx_at(
        up_b,
        STARSTUFF_AT_S,
        "starstuff_burst",
        (0.0, 8.0),
        COSMIC_FX,
    );
    let up_b = on_contact(up_b, "player.hit");

    // DOWN — `pale_blue_dot`. A pixel, at distance. The SMALLEST box in the
    // table on the end of the second-longest reach: it hits almost nothing, and
    // it hits it from over there.
    let down_b = strike(
        "pale_blue_dot",
        "pale_blue_dot",
        0.24,
        0.07,
        0.32,
        (62.0, -2.0),
        (7.0, 7.0),
        12,
        116.0,
        1.90,
        Some((0.6, -0.80)),
        None,
    );
    let down_b = strike_tag(down_b, SLASH_POKE_VFX);
    let down_b = vfx_at(down_b, 0.24, "pale_blue_dot_ping", (62.0, -2.0), POKE_FX);
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
    // DOWN, IN THE AIR. The grounded form works the floor; with none under
    // him he takes it down with him instead.
    let mut air_down_b = strike(
        "falling_horizon",
        "air_down",
        0.11,
        0.10,
        0.25,
        (0.0, 24.0),
        (22.0, 22.0),
        10,
        100.0,
        1.76,
        Some((0.0, 1.0)),
        None,
    );
    air_down_b.landing_lag_s = Some(0.30);
    let air_down_b = impulse(air_down_b, 0.11, (0.0, 1220.0), ImpulseMode::Set);
    // this table's own rule: every move throws an effect and every effect is
    // heard. The dot he points at is under him now.
    let air_down_b = vfx_at(air_down_b, 0.11, "pale_blue_dot_ping", (0.0, 22.0), POKE_FX);
    let air_down_b = on_contact(air_down_b, "player.hit");
    // CARL'S CAPTURE KIT. Deliberately unremarkable in its numbers — the
    // roster needs a baseline to read the others against, and Carl is it.
    //
    // but his capture moves carry ART, because his table requires it of
    // every move. `none_of_his_bursts_sit_on_his_navel` asserts that each move
    // throws an effect at all, and it was written over a population with no
    // capture in it. Honouring it is right rather than scoping it away: a silent
    // grab would be the one mute beat in a kit whose whole idea is that every
    // gesture is cosmic-scale. `orbit_lock` for the catch, `evidence_ping` for
    // the pummel, `planetary_slingshot` for the throw.
    // his sheet ships the whole grab family — `grab`, `grab_hold`, `grab_release` — so the capture kit draws the rows it was drawn for.
    let grab = vfx_at(
        author_standing_grab(
            grab_shell("carl_grab", "grab", 0.07, 0.05, 0.20),
            CaptureAttemptParams {
                offset: (12.0, 1.0),
                half_extents: (19.0, 16.0),
                hold_offset: (13.0, 3.0),
            },
        ),
        0.07,
        "orbit_lock",
        (12.0, 1.0),
        1.0,
    );
    let pummel = vfx_at(
        author_pummel(
            capture_beat("carl_pummel", "grab_hold", 0.18),
            0.08,
            CapturePummelParams { damage: 3 },
        ),
        0.08,
        "evidence_ping",
        (12.0, 1.0),
        0.8,
    );
    let forward_throw = vfx_at(
        author_throw(
            capture_beat("carl_fthrow", "grab_release", 0.26),
            0.14,
            CaptureThrowParams {
                damage: 8,
                knockback: 122.0,
                knockback_growth: 2.0,
                launch_dir: (0.85, -0.55),
            },
        ),
        0.14,
        "planetary_slingshot",
        (16.0, -2.0),
        1.1,
    );

    let back_throw = vfx_at(
        author_throw(
            capture_beat("carl_bthrow", "grab_release", 0.28),
            0.15,
            CaptureThrowParams {
                damage: 9,
                knockback: 131.0,
                knockback_growth: 2.1,
                launch_dir: (-1.0, -0.34),
            },
        ),
        0.15,
        "planetary_slingshot",
        (16.0, -2.0),
        1.1,
    );

    let up_throw = vfx_at(
        author_throw(
            capture_beat("carl_uthrow", "grab_release", 0.27),
            0.14,
            CaptureThrowParams {
                damage: 8,
                knockback: 127.0,
                knockback_growth: 2.04,
                launch_dir: (0.0, -1.0),
            },
        ),
        0.14,
        "planetary_slingshot",
        (16.0, -2.0),
        1.1,
    );

    let down_throw = vfx_at(
        author_throw(
            capture_beat("carl_dthrow", "grab_release", 0.29),
            0.15,
            CaptureThrowParams {
                damage: 6,
                knockback: 90.0,
                knockback_growth: 1.6,
                launch_dir: (0.34, -0.92),
            },
        ),
        0.15,
        "planetary_slingshot",
        (16.0, -2.0),
        1.1,
    );

    SmashRepertoire {
        // his taunt throws a burst like everything else he does, which is
        // both his character and what `none_of_his_bursts_sit_on_his_navel`
        // asks of every move in this table. A cosmic sweep over the head: he is
        // not threatening you, he is showing you the scale of the thing.
        taunt: vfx_at(
            ambition_characters::moveset_authoring::taunt("carl_stargan_taunt", 0.9),
            0.20,
            "cosmic_calendar_sweep",
            (0.0, -26.0),
            SWING_FX,
        ),
        // CARL'S DASH ATTACK IS A SHOULDER CHECK, and his own law decided
        // that. `reach_is_monotonic_in_startup` says a longer reach must never
        // be quicker, and a dash attack is his QUICKEST move — so it has to be
        // his SHORTEST. The genre's 40px lunge reaches 70 at 0.05s and would
        // undercut `pale_blue_dot`, which reaches 69 and takes 0.24s. the law
        // is not in the way of the move; it is what the move is.
        dash_attack: vfx_at(
            ambition_characters::moveset_authoring::dash_attack(
                "carl_stargan_dash_attack",
                ambition_characters::moveset_authoring::DashAttackShape {
                    // EXACTLY his jab's reach, which his module doc pins as
                    // `NEAREST_REACH`: the shoulder check is his shortest move
                    // and his fastest, and both halves of that are his law.
                    reach_px: NEAREST_REACH,
                    ..ambition_characters::moveset_authoring::DashAttackShape::GENRE
                },
                8,
                90.0,
            ),
            0.05,
            "evidence_ping",
            (NEAREST_REACH * 0.6, -2.0),
            POKE_FX,
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
        up_air,
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
            // his own three: the ping he reaches with, the shift he lands, and the slingshot he throws you into — his kit guards that every effect comes off his
            // own sheet, and a shared `classic_burst` would violate it.
            cues: CaptureCues {
                reach: "evidence_ping",
                impact: "perspective_shift",
                release: "planetary_slingshot",
            },
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
    use ambition_platformer2d::entity_catalog::MoveEventKind;

    fn find(set: &MovesetContract, id: &str) -> MoveSpec {
        set.moves
            .iter()
            .find(|m| m.id == id)
            .unwrap_or_else(|| panic!("{id} exists"))
            .clone()
    }

    // Fourteen fighters each carried a copy of it: every bound verb names a move
    // this table defines, and the table binds the whole vocabulary. Both are now
    // unwritable defects rather than tested ones. `SmashRepertoire` owns the verb
    // strings, so there is no string in this file to misspell; it is a struct
    // with no `Default` and no private fields, so a missing or renamed slot is a
    // COMPILE error here. What the fourteen copies stood for — that every press
    // is answered, in every posture it is asked in — is checked once, by
    // `ambition_characters::smash_repertoire`, and by the host ratchet
    // `smash_roster_movesets::report_the_smash_kit_every_selectable_fighter_has`.

    /// REACH IS BOUGHT WITH TIME, AS AN ASSERTION.
    ///
    /// Sort his FORWARD line by reach and the startup must never go down.
    ///
    /// forward, not every move, and the distinction is real rather than a
    /// convenience: an up-smash's x-reach is small because it points UP, so
    /// including it would be comparing a vertical move's horizontal extent
    /// against a horizontal move's. The claim is about the line a player spaces
    /// with.
    ///
    /// the poison is Oiler, whose table is authored from a different idea
    /// entirely — if his satisfied this too, it would be a fact about fighters
    /// rather than about Carl.
    #[test]
    fn reach_is_monotonic_in_startup() {
        let set = carl_stargan_moveset();
        let mut grounded: Vec<(f32, f32, String)> = set
            .moves
            .iter()
            .filter(|m| m.gates.grounded == Some(true) && points_forward(m))
            .map(|m| (reach_of(m), startup_of(m), m.id.clone()))
            .collect();
        assert!(
            grounded.len() >= 5,
            "the forward line is {} moves",
            grounded.len()
        );
        grounded.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        for pair in grounded.windows(2) {
            let (near, far) = (&pair[0], &pair[1]);
            assert!(
                far.1 + 1e-4 >= near.1,
                "`{}` reaches {:.0} in {:.2}s but `{}` reaches only {:.0} and takes {:.2}s — \
                 a longer reach must never be quicker",
                far.2,
                far.0,
                far.1,
                near.2,
                near.0,
                near.1
            );
        }

        // NO POISON HERE, and that is a finding rather than an omission.
        // I wrote one — "Oiler's table must violate this, or the rule is about
        // fighters rather than about Carl" — and it FAILED: his forward line is
        // monotonic too. So this is a discipline the authored tables share, not
        // Carl's distinguishing trait. What IS his is the SPREAD, and that claim
        // carries its own comparison in the test below.
    }

    /// The spread is the widest on the grid — the very small and the very
    /// large, in hitboxes.
    #[test]
    fn his_reach_spans_further_than_anybody_elses() {
        let set = carl_stargan_moveset();
        let reaches: Vec<f32> = set
            .moves
            .iter()
            .filter(|m| points_forward(m))
            .map(reach_of)
            .collect();
        let far = reaches.iter().cloned().fold(0.0_f32, f32::max);
        let near = reaches.iter().cloned().fold(f32::MAX, f32::min);
        assert!(
            (far - FURTHEST_REACH).abs() < 1.0,
            "his furthest is {far:.0}, and the module doc says {FURTHEST_REACH}"
        );
        assert!((near - NEAREST_REACH).abs() < 1.0);
        let mine = far / near;

        for (who, other) in [
            ("oiler", crate::oiler_moveset::oiler_moveset()),
            (
                "emmy_noether",
                crate::emmy_noether_moveset::emmy_noether_moveset(),
            ),
        ] {
            let theirs: Vec<f32> = other
                .moves
                .iter()
                .filter(|m| points_forward(m))
                .map(reach_of)
                .filter(|r| *r > 0.0)
                .collect();
            let ratio = theirs.iter().cloned().fold(0.0_f32, f32::max)
                / theirs.iter().cloned().fold(f32::MAX, f32::min);
            assert!(
                mine > ratio * 1.5,
                "Carl's spread is {mine:.1}x and {who}'s is {ratio:.1}x — not a wide enough \
                 gap to be his defining property"
            );
        }
    }

    /// `starstuff` is a save, not flight — arithmetic, not a cooldown.
    #[test]
    fn the_recovery_outlasts_its_own_arc() {
        const G: f32 = 2200.0;
        assert!(STARSTUFF_ENDS_S >= 2.0 * (STARSTUFF_SPEED / G));
        let set = carl_stargan_moveset();
        assert!(find(&set, "starstuff")
            .windows
            .iter()
            .all(|w| !matches!(w.tag, WindowTag::Cancelable { .. })));
    }

    /// NONE OF HIS BURSTS SIT ON HIS NAVEL.
    ///
    /// what guards the sound instead — and catches the double-play the paired form could not —
    /// is `a_paired_burst_is_heard_exactly_once` in `src/moveset_sound.rs`, which drives this
    /// table through the real dispatcher and the real fan-out.
    ///
    /// The PLACEMENT half is his own and stays: the rule was bursts blooming out of a fighter's
    /// chest, and Carl's kit is the widest table on the grid.
    #[test]
    fn none_of_his_bursts_sit_on_his_navel() {
        let set = carl_stargan_moveset();
        let mut placed = 0;
        for m in &set.moves {
            let mut bursts = 0;
            for ev in &m.events {
                if let MoveEventKind::Vfx { at, .. } = &ev.kind {
                    bursts += 1;
                    if *at != (0.0, 0.0) {
                        placed += 1;
                    }
                }
            }
            assert!(bursts > 0, "`{}` throws no effect at all", m.id);
        }
        // the non-vacuity: most bursts must actually carry an offset, or this
        // is a test about a table that never used the field.
        assert!(placed >= 12, "only {placed} bursts are placed on their box");
    }

    /// THE ART IS HIS, AND IT ALL SHIPS.
    #[test]
    fn the_kit_looks_like_carl_and_the_art_all_ships() {
        let set = carl_stargan_moveset();
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
        assert!(effects.len() >= 10, "a thin palette: {effects:?}");
        for effect in &effects {
            let authored = ambition_platformer2d::sprite_sheet::fx::authored_effect(effect)
                .unwrap_or_else(|| panic!("`{effect}` ships"));
            assert!(
                authored.sheet.contains("carl_stargan"),
                "`{effect}` is drawn off `{}`, which is not his sheet",
                authored.sheet
            );
        }
    }

    /// Every clip he names is a row his sheet carries.
    #[test]
    fn every_clip_names_a_row_his_sheet_carries() {
        let set = carl_stargan_moveset();
        let record = ambition_platformer2d::sprite_sheet::character::sheets::record_for_target(
            "carl_stargan",
        )
        .expect("his sheet is baked into the registry");
        let rows: std::collections::BTreeSet<&str> =
            record.rows.iter().map(|r| r.animation.as_str()).collect();
        for m in &set.moves {
            assert!(
                rows.contains(m.clip.clip.as_str()),
                "`{}` draws `{}`, which his sheet does not publish",
                m.id,
                m.clip.clip
            );
        }
    }
}

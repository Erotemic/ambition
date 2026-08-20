//! **The Pirate Admiral's repertoire** — a cutlass, and the reach that comes with
//! carrying one.
//!
//! ⭐ **the second adopter removed from `smash_fighter_kit()`** (campaign P3.24),
//! after the goblin. That floor's goal is DELETION and the count only moves when
//! somebody writes a table.
//!
//! ⚠ **the character was already telling us what its moves are.** Its catalog row
//! says `default_action_set: "pirate_pistol"` and the roster comment beside its id
//! reads *"pistol + cutlass"*; its sprite is authored at `collision_scale: 1.6`,
//! the largest of the three fighters with tables. So: a big body with a long blade
//! — slower than the goblin, longer than the robot, and hitting harder than either
//! when it connects.
//!
//! ```text
//!            reach     jab startup   f-smash damage
//!   goblin    22 px       0.04 s          12
//!   robot     26 px       0.05 s          15
//!   admiral   32 px       0.06 s          17
//! ```
//!
//! ## ⭐⭐ THE SECOND FIGHTER IN THE REPO WITH A RECOVERY, AND IT IS NOT A RISE
//!
//! The first (`smash_george_booul`) authors a straight-up Up-B: a `Set` of
//! `(0, -1020)`, the move IS the height, and one scalar described it exactly.
//! The admiral's `grapple_line` is a boarding line thrown at the stage and
//! hauled in — `(980, -300)`, almost all of it lateral — and it exists to press
//! the abstraction from the other direction:
//!
//! ```text
//!                     across    up      what the move supplies
//!   excluded_middle        0   1020     the whole recovery
//!   grapple_line         980    300     the distance; the BODY supplies the height
//! ```
//!
//! ⭐ **so the two recoveries divide the work differently, which is what makes
//! this a second mechanism rather than the first one at an angle.** An admiral
//! knocked below the lip has to spend his double jump FIRST and grapple second;
//! one knocked out level can grapple immediately. A vertical Up-B collapses that
//! decision into one press.
//!
//! ⛔ **and it is what found the affordance bug.** `MoveFrameData::lift_speed`
//! kept only the against-gravity half of a commanded velocity, so this move —
//! whose useful half is the other one — advertised itself as a 300px/s hop and
//! ranked BELOW the 360px/s juggle aerial in its own kit. Every policy layer read
//! the biggest number as "the recovery". See `air_up` below, which is authored as
//! the poison for exactly that.
//!
//! ⛔ **the PISTOL is not in here, and that is the authority split doing its job.**
//! A ranged verb belongs to the character's `ActionSet` — what this body is
//! CAPABLE of — while this table is what its swings ARE. Putting a shot in the
//! move list would give one press two owners, which is the exact double-ownership
//! `RangedExecution` exists to prevent.

use ambition_characters::smash_capture::{
    CaptureCues,
    author_pummel, author_standing_grab, author_throw, capture_beat, grab_shell,
    CaptureAttemptParams, CapturePummelParams, CaptureThrowParams, SmashCaptureRepertoire,
};
use ambition_characters::smash_repertoire::{DownSpecial, NeutralSpecial, SmashRepertoire};
use ambition_platformer2d::entity_catalog::{ImpulseMode, MovesetContract};

use ambition_characters::moveset_authoring::{
    committed_tail, impulse, on_contact, sfx, strike, vfx, vfx_at,
};

/// **How far across the grapple hauls him**, engine units per second along
/// facing.
///
/// ⭐ the number that IS the move. A recovery is usually authored as a rise; this
/// one spends its whole budget on distance, so the admiral's problem offstage is
/// never *"can I climb back to the lip"* but *"can I reach the lip at all"*.
pub(crate) const GRAPPLE_ACROSS: f32 = 980.0;

/// And how far up. Deliberately small — enough to hold him level for the
/// crossing and nowhere near enough to climb with.
pub(crate) const GRAPPLE_RISE: f32 = 300.0;

/// When the line catches. The windup you can see coming, and the number a
/// recovery search plans around.
pub(crate) const GRAPPLE_AT_S: f32 = 0.16;

/// And when the move lets go. ⛔ **not a feel number.** The rise buys
/// `GRAPPLE_RISE² / 2g ≈ 20px` of climb and takes `GRAPPLE_RISE / g ≈ 0.13s` to
/// spend it, so any tail longer than twice that hands the admiral back BELOW
/// where the move found him — which is what stops a re-pressable recovery from
/// being flight. The guard `the_grapple_is_a_crossing_and_not_a_flight` holds the
/// arithmetic.
pub(crate) const GRAPPLE_ENDS_S: f32 = 0.88;

/// See the module doc. Fifteen moves: the genre's standard verb map plus four
/// specials.
pub fn pirate_admiral_moveset() -> MovesetContract {
    // ── grounded ─────────────────────────────────────────────────────────────
    //
    // Even the jab is a blade: it starts slower than the goblin's whole punish
    // window and reaches half a body further.
    let jab = strike(
        "jab",
        "jab",
        0.06,
        0.07,
        0.16,
        (30.0, 0.0),
        (22.0, 14.0),
        4,
        55.0,
        1.10,
        None,
        None,
    );

    // A rising cutlass arc. Wide, because a sword's up-tilt covers the space in
    // front of the shoulder as well as above it.
    let up_tilt = strike(
        "tilt_up",
        "attack_up",
        0.09,
        0.08,
        0.19,
        (14.0, -28.0),
        (22.0, 24.0),
        6,
        80.0,
        1.35,
        Some((0.25, -1.0)),
        None,
    );

    // A low sweep along the deck. Long, shallow, and it sends them along the
    // ground rather than up — the setup, not the finish.
    let down_tilt = strike(
        "tilt_down",
        "attack_down",
        0.08,
        0.07,
        0.18,
        (30.0, 14.0),
        (24.0, 9.0),
        5,
        60.0,
        1.20,
        Some((1.0, -0.18)),
        None,
    );

    // ── smashes ──────────────────────────────────────────────────────────────
    //
    // ⚠ the slowest kill move of the three tables, and the hardest. An admiral
    // who commits to a full cutlass swing has decided the exchange is worth 0.38s
    // of standing still afterwards.
    let mut f_smash = strike(
        "smash_forward",
        "smash_forward",
        0.34,
        0.08,
        0.38,
        (44.0, -4.0),
        (30.0, 20.0),
        17,
        160.0,
        3.10,
        Some((1.0, -0.42)),
        None,
    );
    f_smash.smash_charge_mult = 1.7;

    let mut up_smash = strike(
        "smash_up",
        "smash_up",
        0.30,
        0.09,
        0.34,
        (8.0, -34.0),
        (24.0, 30.0),
        15,
        155.0,
        2.95,
        Some((0.0, -1.0)),
        None,
    );
    up_smash.smash_charge_mult = 1.7;

    // Both sides at deck height — the boarding-action answer to being flanked.
    let mut down_smash = strike(
        "smash_down",
        "smash_down",
        0.28,
        0.10,
        0.36,
        (0.0, 16.0),
        (40.0, 12.0),
        13,
        140.0,
        2.70,
        Some((0.9, -0.50)),
        None,
    );
    down_smash.smash_charge_mult = 1.7;

    // ── aerials ──────────────────────────────────────────────────────────────
    let n_air = strike(
        "air_neutral",
        "air_neutral",
        0.07,
        0.12,
        0.16,
        (0.0, 0.0),
        (26.0, 22.0),
        6,
        70.0,
        1.30,
        None,
        None,
    );

    let f_air = strike(
        "air_forward",
        "air_forward",
        0.11,
        0.08,
        0.20,
        (32.0, -2.0),
        (24.0, 18.0),
        9,
        100.0,
        1.90,
        Some((1.0, -0.30)),
        None,
    );

    let b_air = strike(
        "air_back",
        "air_back",
        0.13,
        0.07,
        0.24,
        (-34.0, 0.0),
        (24.0, 18.0),
        11,
        135.0,
        2.55,
        Some((-1.0, -0.35)),
        None,
    );

    let u_air = strike(
        "air_up",
        "air_up",
        0.08,
        0.09,
        0.17,
        (2.0, -30.0),
        (20.0, 24.0),
        7,
        90.0,
        1.85,
        Some((0.0, -1.0)),
        None,
    );
    // ⭐⭐ **THE STALL.** A rising cutlass overhead that takes the admiral up with
    // it — the genre's juggle aerial, and the reason this table can chain one
    // hit into the next instead of falling out from under its own combo.
    //
    // ⛔⛔ **AND IT IS A DELIBERATE POISON FOR THE RECOVERY AFFORDANCE.** Until
    // 2026-08-15 a move like this was UNAUTHORABLE: every policy layer read
    // `lift_speed` as "this is the way home", took the biggest one in the kit,
    // and searched with that alone. 360px/s beats the grapple's 300, so this
    // 7-damage juggle would have become the admiral's recovery — thrown at the
    // blastzone while the line that gets him back sat unexplored. The first
    // fighter to author a recovery worked around it by putting a lift on nothing
    // else (see George's `modus_ponens`, whose zero is commented as exactly that
    // workaround); the search now decides which route is useful from where the
    // body is, so the workaround is no longer owed and the move can exist.
    let u_air = impulse(u_air, 0.08, (0.0, -360.0), ImpulseMode::Set);
    let u_air = sfx(u_air, 0.0, "player.robot.slash.air");
    let u_air = on_contact(u_air, "player.robot.slash.impact.flesh.light");
    let u_air = vfx(u_air, 0.08, "burst_round");

    // ⭐ a real spike: point-down cutlass, straight into the blast zone. ⚠ no
    // `on_hit` rebound, same as the goblin's — the robot is the only body that
    // says it can bounce off what it hits, and that is a property of the
    // character rather than of down-airs.
    let d_air = strike(
        "air_down",
        "air_down",
        0.14,
        0.08,
        0.26,
        (6.0, 28.0),
        (20.0, 20.0),
        11,
        130.0,
        2.30,
        Some((0.0, 1.0)),
        None,
    );

    // ── the four specials ────────────────────────────────────────────────────
    //
    // ⭐ **four MECHANISMS, and none of them is another one rotated.** One fires
    // forward and shoves its owner backward; one adds to whatever the body was
    // already doing; one commands a diagonal haul; one commands a full stop. The
    // shared `strike` gives them all the same timeline shape, so what separates
    // them is what they do to the ADMIRAL, which is the only axis a table can
    // differentiate a special on without new engine mechanisms.

    // **NEUTRAL — `grapeshot`.** A pistol at the hip, and the recoil that comes
    // with firing one from a standing start. The volume is short and wide; the
    // admiral is thrown BACKWARD out of it, which is a real spacing tool and a
    // real way to remove yourself from the stage.
    let neutral_b = strike(
        "grapeshot",
        "special",
        0.14,
        0.06,
        0.24,
        (38.0, -4.0),
        (30.0, 16.0),
        9,
        120.0,
        1.90,
        Some((0.85, -0.55)),
        None,
    );
    // ⭐ **negative side, and that sign is the whole move.** The catalog reads
    // this as a route with `lift_side < 0` — a displacement that carries its
    // owner AWAY from whatever it is facing. A recovery search will happily
    // propose it and the kernel will decline it every time it is thrown toward
    // the stage, which is the correct outcome and needs no rule anywhere.
    let neutral_b = impulse(neutral_b, 0.14, (-560.0, -120.0), ImpulseMode::Set);
    let neutral_b = sfx(neutral_b, 0.0, "player.attack.charge");
    let neutral_b = sfx(neutral_b, 0.14, "player.slash");
    let neutral_b = vfx(neutral_b, 0.14, "smoke_burst");
    let neutral_b = on_contact(neutral_b, "world.rock.hit");

    // **SIDE — `boarding_run`.** A shoulder-first charge across the deck.
    //
    // ⛔ **`Add`, not `Set`, and that is the character.** It CONTRIBUTES to the
    // admiral's own momentum, so it is longest out of a run and nearly nothing
    // from a standstill — the opposite trade from a commanded charge. It
    // therefore states no speed, so it advertises no route: a static reader
    // cannot say what an additive impulse produces, and this table does not ask
    // it to pretend.
    let side_b = strike(
        "boarding_run",
        "special",
        0.16,
        0.14,
        0.28,
        (34.0, 0.0),
        (26.0, 22.0),
        13,
        145.0,
        2.40,
        Some((0.9, -0.4)),
        None,
    );
    let side_b = impulse(side_b, 0.16, (620.0, 0.0), ImpulseMode::Add);
    let side_b = committed_tail(side_b, 0.72, 0.15);
    let side_b = sfx(side_b, 0.0, "player.attack.charge");
    let side_b = sfx(side_b, 0.16, "player.slash");
    let side_b = vfx(side_b, 0.16, "shockwave");
    let side_b = on_contact(side_b, "player.robot.slash.impact.metal.gong");

    // **UP — `grapple_line`. THE RECOVERY, and it is not a rise.**
    //
    // ⭐⭐ a boarding line thrown at the stage and hauled in. The commanded
    // velocity is `(980, -300)`: almost all of the energy goes ACROSS, and the
    // small against-gravity component exists to keep the admiral level while he
    // travels rather than to climb.
    //
    // ⭐ **the height is the BODY's job and the distance is the MOVE's**, which
    // is the division of labour that makes this mechanically a different
    // recovery from a vertical Up-B rather than the same one at an angle. An
    // admiral knocked below the lip must spend his double jump FIRST and then
    // grapple; one knocked out level can grapple immediately. A move that
    // supplied both halves would collapse that decision.
    //
    // ⛔ and it is not flight. The tail runs to `GRAPPLE_ENDS_S` with no
    // `Cancelable` window, so the move cannot be re-pressed until it has handed
    // back more altitude than its 20px of climb ever bought.
    let up_b = strike(
        "grapple_line",
        "special_up",
        GRAPPLE_AT_S,
        0.10,
        0.22,
        (30.0, -10.0),
        (28.0, 18.0),
        8,
        95.0,
        1.60,
        Some((0.7, -0.7)),
        None,
    );
    let up_b = impulse(
        up_b,
        GRAPPLE_AT_S,
        (GRAPPLE_ACROSS, -GRAPPLE_RISE),
        ImpulseMode::Set,
    );
    let up_b = committed_tail(up_b, GRAPPLE_ENDS_S, 0.0);
    let mut up_b = up_b;
    // Landing out of it costs, so it is a bad panic button ON the stage.
    up_b.landing_lag_s = Some(0.18);
    let up_b = sfx(up_b, 0.0, "player.attack.charge");
    let up_b = sfx(up_b, GRAPPLE_AT_S, "player.robot.slash.air");
    // ⭐ a recovery activating gets its own burst: seeing one is how the other
    // player knows this fighter is not dead yet.
    let up_b = vfx(up_b, GRAPPLE_AT_S, "classic_burst");
    let up_b = on_contact(up_b, "player.hit");

    // **DOWN — `heave_to`.** The anchor. It commands a FULL STOP: `(0, 0)`, a
    // `Set` of zero.
    //
    // ⭐ the only move in the repo whose commanded velocity is nothing, and it is
    // a real one — it kills the drift a launch gave you, which is what turns a
    // read into a survivable one. ⚠ it advertises NO route (`local.1` is not
    // negative, so the catalog's lift derivation skips it), which is correct:
    // stopping dead in mid-air is not a way home from anywhere.
    let down_b = strike(
        "heave_to",
        "special_down",
        0.12,
        0.10,
        0.30,
        (0.0, 22.0),
        (30.0, 20.0),
        10,
        105.0,
        1.75,
        Some((0.0, 1.0)),
        None,
    );
    let down_b = impulse(down_b, 0.12, (0.0, 0.0), ImpulseMode::Set);
    let down_b = sfx(down_b, 0.12, "player.slash");
    let down_b = vfx(down_b, 0.12, "starburst");
    let down_b = on_contact(down_b, "player.robot.slash.impact.metal.chink");

    // ── 2026-08-16: THE ONE THAT WAS MISSING ─────────────────────────────────
    //
    // ⛔ **the forward tilt.** With all four specials authored, this was the only
    // press on the admiral that fell down the directional chain — a fighter
    // carrying a cutlass answering "forward" with a jab. A LEVEL CUT at chest
    // height: the longest tilt on the grid, because reach is what the cutlass is
    // for, and slower than the goblin's whole jab because carrying one costs.
    let f_tilt = strike(
        "tilt_forward",
        "attack_side",
        0.10,
        0.08,
        0.20,
        (38.0, -4.0),
        (26.0, 14.0),
        7,
        78.0,
        1.30,
        Some((1.0, -0.30)),
        None,
    );
    let f_tilt = vfx_at(f_tilt, 0.10, "air_slice", (38.0, -4.0), 1.0);
    let f_tilt = sfx(f_tilt, 0.10, "enemy.pirate.cutlass_swing");
    let f_tilt = on_contact(f_tilt, "player.hit");

    // ── The boarding grapple ────────────────────────────────────────────────
    //
    // ⭐ **the deliberate opposite of George's**, and the pair is the point: two
    // fighters authored through two different providers, sharing no numbers.
    //
    // The admiral carries a cutlass and boards ships. His grab is FAST (`0.07`
    // startup, half of George's `0.14`) and SHORT (`19` of reach against
    // George's `26`) — you have to be on top of somebody to board them, and once
    // you have decided to, it happens. Recovery `0.20` against George's `0.30`:
    // whiffing costs him a third less, because his grab is a scramble tool and
    // George's is a commitment.
    //
    // ⚠ the hold sits CLOSE and LOW (`13` forward, `+3` down): hauled in against
    // the chest, not held out at arm's length.
    let grab = author_standing_grab(
        grab_shell("pirate_grab", "grab", 0.07, 0.05, 0.20),
        CaptureAttemptParams {
            offset: (12.0, 1.0),
            half_extents: (19.0, 16.0),
            hold_offset: (13.0, 3.0),
        },
    );
    // A FAST, LIGHT pummel — `0.13` and `2`, against George's `0.24` and `4`.
    // Nearly twice the rate for half the damage: the same damage per second by
    // arithmetic and a completely different thing to play against, because every
    // beat is a chance for the hold to break.
    let pummel = author_pummel(
        capture_beat("pirate_pummel", "attack", 0.13),
        0.06,
        CapturePummelParams { damage: 2 },
    );
    // ⚠ **an UPWARD throw wearing the forward slot**, and that is a character
    // fact rather than a mistake. George throws flat and across for stage
    // control; the admiral heaves a body up and slightly forward (`0.55` lateral
    // against `-1.0` vertical) so it lands in front of him and he keeps swinging.
    // Less knockback than George's (`104` against `138`) and more growth (`2.4`
    // against `1.9`): weak early, frightening late.
    let forward_throw = author_throw(
        capture_beat("pirate_fthrow", "attack", 0.26),
        0.14,
        CaptureThrowParams {
            damage: 8,
            knockback: 104.0,
            knockback_growth: 2.4,
            launch_dir: (0.55, -1.0),
        },
    );

    SmashRepertoire {

        taunt: ambition_characters::moveset_authoring::taunt("pirate_admiral_taunt", 0.9),
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
        neutral_special: NeutralSpecial::Authored(neutral_b),
        side_special: side_b,
        up_special: up_b,
        // ⭐ **the second fighter to author one, and through a DIFFERENT
        // provider** — this crate, not the smash demo. That is the falsifier:
        // the capture vocabulary is not quietly tied to one game-owned file.
        capture: SmashCaptureRepertoire {
            cues: CaptureCues::GENERIC,
            grab,
            pummel,
            forward_throw,
            back_throw: None,
            up_throw: None,
            down_throw: None,
        },
        down_special: DownSpecial::OneForm(down_b),
    }
    .into_contract()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d::entity_catalog::{MoveSpec, VolumeShape, WindowTag};

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

    fn reach(m: &MoveSpec) -> f32 {
        m.windows
            .iter()
            .flat_map(|w| w.volumes.iter())
            .map(|v| match v.shape {
                VolumeShape::Rect {
                    offset,
                    half_extents,
                } => offset.0.abs() + half_extents.0,
                _ => 0.0,
            })
            .fold(0.0f32, f32::max)
    }

    fn damage(m: &MoveSpec) -> i32 {
        m.windows
            .iter()
            .flat_map(|w| w.volumes.iter())
            .map(|v| v.damage)
            .max()
            .unwrap_or(0)
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

    /// The commanded (`Set`) velocity a move states, if it states one.
    fn commanded(set: &MovesetContract, id: &str) -> Option<(f32, f32)> {
        use ambition_platformer2d::entity_catalog::MoveEventKind;
        find(set, id).events.iter().find_map(|e| match &e.kind {
            MoveEventKind::Impulse {
                local,
                mode: ImpulseMode::Set,
            } => Some(*local),
            _ => None,
        })
    }

    /// **THE RECOVERY IS A CROSSING, AND THE TABLE SAYS SO IN ITS OWN NUMBERS.**
    ///
    /// ⭐ the one claim this fighter exists to make: its way home spends its
    /// budget on DISTANCE, not on height. Asserting the ratio rather than the
    /// magnitudes is what keeps this true through a retune — an admiral whose
    /// grapple climbed more than it crossed would be George with a different
    /// sprite, and nothing else in the tree would notice.
    #[test]
    fn the_grapple_crosses_further_than_it_climbs() {
        let set = pirate_admiral_moveset();
        let line = commanded(&set, "grapple_line").expect("the recovery displaces its owner");
        assert!(
            line.1 < 0.0,
            "the burst must have an against-gravity component or no policy layer \
             can see it at all: {line:?}"
        );
        assert!(
            line.0 > 0.0,
            "and a lateral one, or this is a vertical Up-B: {line:?}"
        );
        assert!(
            line.0 > 3.0 * -line.1,
            "the crossing must DOMINATE the climb — {}px/s across against \
             {}px/s up. Below that ratio this fighter stops pressing the \
             abstraction from a different direction and becomes a second George",
            line.0,
            -line.1
        );
    }

    /// **THE GRAPPLE IS A CROSSING AND NOT A FLIGHT — and the arithmetic is the
    /// reason.**
    ///
    /// ⭐ this is what lets the recovery exist with no cooldown, no per-airtime
    /// counter and no new rollback state. The body cannot re-press while the move
    /// is playing (no `Cancelable` window), so the only question is whether one
    /// full cycle gains height. It cannot: the move outlasts its own tiny arc, so
    /// by the time the admiral may press again he has fallen back through
    /// everything the rise bought and then some.
    ///
    /// ⛔ the failure this forbids is silent and total — shorten the tail and
    /// spamming the grapple becomes hovering, which ends a platform fighter.
    #[test]
    fn the_grapple_is_a_crossing_and_not_a_flight() {
        let g = ambition_platformer2d::engine_core::DEFAULT_TUNING.gravity;
        let to_apex = GRAPPLE_RISE / g;
        let tail = GRAPPLE_ENDS_S - GRAPPLE_AT_S;
        assert!(
            tail > 2.0 * to_apex,
            "the line climbs for {to_apex:.3}s and is handed back {tail:.3}s after \
             it catches; anything at or under {:.3}s returns the admiral higher \
             than it found him, every press, which is flight",
            2.0 * to_apex
        );
        // Landing out of it costs, so it is a bad panic button ON the stage.
        assert!(
            find(&pirate_admiral_moveset(), "grapple_line")
                .landing_lag_s
                .unwrap_or(0.0)
                > 0.0
        );
    }

    /// ⭐⭐ **THE POISON, AUTHORED: A TINY UPWARD ATTACK OUTRANKS THE RECOVERY ON
    /// THE SCALAR, AND MUST NOT BE THE RECOVERY.**
    ///
    /// ⛔⛔ this is the trap the engine used to fall into, reproduced in real
    /// content rather than in a fixture: `air_up` commands a LARGER
    /// against-gravity speed than `grapple_line` does, because the grapple spent
    /// its budget going sideways. A policy layer that ranks a kit by
    /// `lift_speed` and takes the winner therefore picks a 7-damage juggle
    /// aerial as the admiral's way home — and the CPU throws it at the blastzone
    /// while the move that gets him back sits unexplored.
    ///
    /// ⭐ **this test asserts the SETUP, not the fix.** The fix lives in
    /// `RecoveryLens::best_route`, which searches every route instead of ranking
    /// them, and it is pinned there
    /// (`a_tiny_lifting_move_does_not_suppress_a_viable_recovery`). What is
    /// pinned HERE is that the setup is real — that this table genuinely
    /// contains a kit where the scalar ordering and the useful ordering
    /// disagree. If a retune ever makes them agree, that fixture stops standing
    /// for anything and this test says so.
    #[test]
    fn the_juggle_aerial_outranks_the_recovery_on_lift_speed() {
        let set = pirate_admiral_moveset();
        let aerial = find(&set, "air_up").frame_data();
        let line = find(&set, "grapple_line").frame_data();

        assert!(
            aerial.lift_speed > 0.0,
            "the juggle aerial must really command a rise, or there is no trap here"
        );
        assert!(
            aerial.lift_speed > line.lift_speed,
            "the trap needs the USELESS move to sort FIRST: {} vs {}",
            aerial.lift_speed,
            line.lift_speed
        );
        assert!(
            line.lift_side > aerial.lift_side,
            "and the recovery's advantage must live entirely in the half the \
             scalar discards: {} vs {}",
            line.lift_side,
            aerial.lift_side
        );
        // ⛔ and the aerial is genuinely not a way home: it goes nowhere sideways.
        assert_eq!(aerial.lift_side, 0.0);
    }

    // -----------------------------------------------------------------------
    // The authored table, driven through the REAL decision machinery.
    // -----------------------------------------------------------------------

    use ambition_characters::brain::fighter::options::{
        lifting_candidates, ActionLegality, AttackBinding, AttackCandidate, AttackVerb,
    };
    use ambition_characters::brain::fighter::recovery::{
        BodyKit, RecoveryLens, RecoveryLift, RecoveryQuery,
    };
    use ambition_characters::perception::{
        PerceivedSolid, SelfView, SolidKind, StageView, WorldView,
    };
    use ambition_platformer2d::engine_core as ae;

    const DT: f32 = 1.0 / 60.0;

    /// **The admiral's kit as an AIRBORNE body sees it** — the posture filter the
    /// runtime applies before the brain ever looks, so a grounded-only tilt
    /// cannot be proposed off the side of a stage.
    ///
    /// ⚠ the BINDING is not what this fixture measures (a route is identified by
    /// its move id), so every candidate carries the same placeholder press.
    fn airborne_kit() -> Vec<AttackCandidate> {
        pirate_admiral_moveset()
            .moves
            .iter()
            .filter(|m| m.gates.grounded != Some(true))
            .map(|m| AttackCandidate {
                move_id: m.id.clone(),
                frames: m.frame_data(),
                binding: AttackBinding {
                    verb: AttackVerb::Special,
                    direction: ambition_characters::actor::attack_gesture::AttackDir::Up,
                },
                legality: ActionLegality::Now,
            })
            .collect()
    }

    /// A 1600x800 stage whose only surface is far off to the right: `x` in
    /// `650..1450`, top face at `y = 500`. A body high and far to the left is
    /// ABOVE that face, so its problem is entirely lateral.
    fn offstage_left() -> WorldView {
        WorldView {
            self_view: SelfView {
                pos: ae::Vec2::new(150.0, 200.0),
                gravity_down: ae::Vec2::new(0.0, 1.0),
                half_extent: ae::Vec2::new(12.0, 16.0),
                alive: true,
                on_ground: false,
                health_max: 100,
                ..Default::default()
            },
            stage: StageView {
                bounds: ae::Aabb::new(ae::Vec2::new(800.0, 400.0), ae::Vec2::new(800.0, 400.0)),
            },
            terrain: vec![PerceivedSolid {
                aabb: ae::Aabb::new(ae::Vec2::new(1050.0, 516.0), ae::Vec2::new(400.0, 16.0)),
                kind: SolidKind::Solid,
            }],
            ..Default::default()
        }
    }

    /// ⭐⭐ **THE ACCEPTANCE MEASUREMENT: the admiral's own table, the brain's own
    /// route derivation, and the real movement kernel agree that `grapple_line`
    /// is the way home — and they do it without anybody naming him.**
    ///
    /// Every step is the shipped one. The kit is this file's `MovesetContract`
    /// posture-filtered the way the runtime filters it; the routes come from
    /// `lifting_candidates`, which reads nothing but `MoveFrameData`; and the
    /// verdict comes from `RecoveryLens::best_route`, which clones a body and
    /// drives `step_motion`. There is no character conditional anywhere in that
    /// chain and this test would read identically for any fighter.
    ///
    /// ⛔ **and the ORDER is asserted first, because the order is the trap.**
    /// `air_up` sorts above `grapple_line` on the only number a static reader
    /// has. A layer that took the first candidate would take the juggle aerial.
    #[test]
    fn the_search_picks_the_grapple_out_of_the_admirals_own_kit() {
        let kit = airborne_kit();
        let routes_by_id: Vec<&str> = lifting_candidates(&kit)
            .iter()
            .map(|c| c.move_id.as_str())
            .collect();
        assert_eq!(
            routes_by_id,
            vec!["air_up", "grapple_line", "grapeshot"],
            "the scalar order is the trap this fixture exists inside — if it \
             changes, re-read the comment on `air_up` before touching anything"
        );

        let view = offstage_left();
        let routes: Vec<RecoveryLift> = lifting_candidates(&kit)
            .iter()
            .map(|c| RecoveryLift {
                speed: c.frames.lift_speed,
                side: c.frames.lift_side,
                after_s: c.frames.lift_at_s,
            })
            .collect();
        let kit_facts = BodyKit {
            abilities: ae::AbilitySet {
                double_jump: true,
                ..ae::AbilitySet::basic()
            },
            movement: ae::MovementTuning::default(),
        };
        // ⭐ **the double jump is SPENT**, which is the situation the module doc
        // describes: the admiral buys his height with the body's own verb and
        // then crosses with the move. A fixture that left the jump unspent would
        // be measuring a body that has not got into trouble yet.
        let at = RecoveryQuery {
            pos: view.self_view.pos,
            vel: ae::Vec2::ZERO,
            air_jumps_left: 0,
        };

        let lens = RecoveryLens::from_view(&view, kit_facts, &routes, DT)
            .expect("the stage is known and gravity is non-zero");
        let verdict = lens.best_route(at);
        assert!(
            verdict.regained(),
            "the admiral is 500px from the only surface on the stage and holding \
             a move built to cross exactly that — got {verdict:?}"
        );
        let chosen = verdict.route.expect("a route, not the bare drift");
        assert_eq!(
            lifting_candidates(&kit)[chosen].move_id,
            "grapple_line",
            "the search endorsed the wrong move; routes were {routes_by_id:?}"
        );

        // ⛔ poison: without any route at all the identical body from the
        // identical place must NOT get home, or the lens is answering `Regained`
        // to everything and the assertion above is worthless.
        let unarmed =
            RecoveryLens::from_view(&view, kit_facts, &[], DT).expect("the stage is known");
        assert!(
            !unarmed.best_route(at).regained(),
            "drift alone crossed 500px, so this stage cannot tell a recovery from \
             a fall"
        );

        // ⛔ poison: and the juggle aerial ALONE — the move the scalar ranks
        // first — must not get home either. If it did, the endorsement above
        // would be a coin flip between two working routes.
        let juggle_only = RecoveryLens::from_view(&view, kit_facts, &routes[..1], DT)
            .expect("the stage is known");
        assert!(
            !juggle_only.best_route(at).regained(),
            "the 360px/s juggle aerial reached the stage, which means the trap \
             this fighter was authored to expose is not present in these numbers"
        );
    }

    /// **FOUR SPECIALS, FOUR MECHANISMS.**
    ///
    /// ⛔ four rotations of one strike would be a re-skin, so the assertion is
    /// about what each does to the ADMIRAL: one commands a retreat, one
    /// contributes to his own momentum, one commands a diagonal haul, and one
    /// commands a full stop. No two share a mechanism.
    ///
    /// ⭐ and three of the four exercise a different corner of the catalog's
    /// route derivation — a negative side, an `Add` that states nothing, and a
    /// `Set` of zero — which is how this table doubles as the derivation's
    /// content-level falsifier.
    #[test]
    fn the_four_specials_are_four_different_mechanisms() {
        use ambition_platformer2d::entity_catalog::MoveEventKind;
        let set = pirate_admiral_moveset();

        // Neutral: a recoil. Commanded, and it points BACKWARD.
        let shot = commanded(&set, "grapeshot").expect("the pistol shoves its owner");
        assert!(shot.0 < 0.0 && shot.1 < 0.0);

        // Side: additive travel, so it commands nothing and advertises no route.
        assert!(
            commanded(&set, "boarding_run").is_none(),
            "the charge must ADD to the admiral's momentum, not replace it"
        );
        assert!(
            find(&set, "boarding_run").events.iter().any(|e| matches!(
                &e.kind,
                MoveEventKind::Impulse {
                    mode: ImpulseMode::Add,
                    ..
                }
            )),
            "…and it must still displace him"
        );
        assert_eq!(find(&set, "boarding_run").frame_data().lift_speed, 0.0);
        assert!(
            find(&set, "boarding_run")
                .windows
                .iter()
                .any(|w| matches!(w.tag, WindowTag::Recovery) && w.motion_scale < 1.0),
            "a charge you can steer freely out of is not a commitment"
        );

        // Up: the crossing. Both components, and the lateral one dominates.
        let line = commanded(&set, "grapple_line").expect("the recovery displaces");
        assert!(line.0 > 0.0 && line.1 < 0.0);

        // Down: a full stop, which is a commanded velocity of nothing.
        assert_eq!(commanded(&set, "heave_to"), Some((0.0, 0.0)));
        assert_eq!(
            find(&set, "heave_to").frame_data().lift_speed,
            0.0,
            "stopping dead in mid-air is not a way home from anywhere"
        );
    }

    /// **EVERY IMPORTANT MOVE IS HEARD AND SEEN.**
    ///
    /// ⚠ **the vfx ids are checked against the SHIPPED ART** — the rows of the
    /// published FX spritesheets, which is what the renderer resolves against
    /// and what `MoveSpec::presentation_problems` is handed. A typo here would
    /// be a move that plays nothing. So this asserts membership rather than
    /// spelling.
    #[test]
    fn the_specials_and_the_juggle_carry_their_own_feedback() {
        use ambition_platformer2d::entity_catalog::MoveEventKind;
        let set = pirate_admiral_moveset();
        for id in [
            "grapeshot",
            "boarding_run",
            "grapple_line",
            "heave_to",
            "air_up",
        ] {
            let m = find(&set, id);
            assert!(
                m.events
                    .iter()
                    .any(|e| matches!(&e.kind, MoveEventKind::Sfx { .. })),
                "`{id}` makes no sound"
            );
            assert!(
                m.events.iter().any(|e| match &e.kind {
                    MoveEventKind::Vfx { effect, .. } => {
                        assert!(
                            ambition_platformer2d::sprite_sheet::fx::is_authored_effect(effect),
                            "`{id}` names vfx `{effect}`, which the engine's \
                             vocabulary does not contain — this is a refused load"
                        );
                        true
                    }
                    _ => false,
                }),
                "`{id}` shows nothing"
            );
            assert!(
                m.windows
                    .iter()
                    .flat_map(|w| w.volumes.iter())
                    .any(|v| v.hit_sfx.is_some()),
                "`{id}` lands silently"
            );
        }

        // ⛔ poison: the ordinary swings are NOT dressed up. A table where every
        // move flashed would satisfy the loop above and differentiate nothing.
        let jab = find(&set, "jab");
        assert!(
            !jab.events
                .iter()
                .any(|e| matches!(&e.kind, MoveEventKind::Vfx { .. })),
            "a jab that bursts makes the specials look like nothing"
        );
    }

    /// **Three tables, three fighters, one ORDERING** — and it is checked against
    /// the other two rather than against literals.
    ///
    /// ⭐ this is what stops a repertoire being the previous one renumbered. The
    /// claim in every module doc here is comparative (*shorter*, *slower*,
    /// *harder*), so the test has to be comparative too; pinning the admiral's
    /// numbers alone would go green on a table that had quietly become the
    /// goblin's.
    ///
    /// ⚠ it also means retuning ANY of the three has to keep the ordering true or
    /// say why, which is the point: these are the characters' relationships to
    /// each other, not three independent piles of numbers.
    #[test]
    fn the_admiral_is_longer_slower_and_heavier_than_the_other_two() {
        let admiral = pirate_admiral_moveset();
        let goblin = crate::goblin_moveset::goblin_moveset();
        let robot = crate::player_robot_moveset::player_robot_moveset();

        let jabs = |set: &MovesetContract| {
            let jab = find(set, "jab");
            (reach(&jab), startup(&jab))
        };
        let (a_reach, a_startup) = jabs(&admiral);
        let (r_reach, r_startup) = jabs(&robot);
        let (g_reach, g_startup) = jabs(&goblin);

        assert!(
            a_reach > r_reach && r_reach > g_reach,
            "reach orders admiral > robot > goblin (got {a_reach}, {r_reach}, {g_reach})"
        );
        assert!(
            a_startup > r_startup && r_startup > g_startup,
            "and startup orders the same way — the longer blade is the slower one \
             (got {a_startup}, {r_startup}, {g_startup})"
        );

        let smash = |set: &MovesetContract| damage(&find(set, "smash_forward"));
        assert!(
            smash(&admiral) > smash(&robot) && smash(&robot) > smash(&goblin),
            "and the kill move pays for the commitment: {} > {} > {}",
            smash(&admiral),
            smash(&robot),
            smash(&goblin)
        );
    }
}

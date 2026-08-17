//! **Alice's repertoire** — the cryptographer, and the one who SENDS.
//!
//! ⭐ **written 2026-08-16** (Jon: *"Let's complete the kit for all characters,
//! authoring new moves when we need to."*). Measured before a line of it, Alice
//! was one of four fighters on the grid at **0/16**: no table, no action set, and
//! — the finding that came with the census — no unarmed floor reaching her body
//! either. Every press was silence.
//!
//! ## The character, from her own name
//!
//! Alice and Bob are the two names cryptography uses for the two ends of a
//! channel, and the split is the design: **Alice sends, Bob receives.** So her
//! kit is about GETTING SOMETHING ACROSS — the longest reach among the Hall's
//! people, the quickest recovery on a whiff, and a side special that is
//! literally a key exchange: she crosses the gap and arrives already having
//! agreed on the terms.
//!
//! ```text
//!            reach   jab startup   f-smash damage   the trade
//!   goblin    22 px     0.04 s          12          fast, cheap, close
//!   alice     28 px     0.05 s          13          reach and recovery
//!   bob       26 px     0.07 s          16          slow, and it lands
//! ```
//!
//! ⚠ **her effects are the generic vocabulary, deliberately.** She has no
//! authored FX sheet of her own — Jon: *"It doesn't have to be fancy we can use
//! generic sfx / vfx"* — so every burst below is a row the shipped generic
//! sheets carry (`rune_burst`, `rune_circle`, `magic_seal_break`,
//! `four_point_glint`, `phase_ripple`). The day she gets her own art, this file
//! is where the names change and nothing else does.

use ambition_characters::smash_repertoire::{DownSpecial, NeutralSpecial, SmashRepertoire};
use ambition_platformer2d::entity_catalog::{ImpulseMode, MovesetContract};

use ambition_characters::moveset_authoring::{
    committed_tail, impulse, on_contact, sfx, strike, vfx_at,
};

/// How big a cipher's burst draws.
const GLYPH_FX: f32 = 0.85;
const SEAL_FX: f32 = 1.15;

/// See the module doc. Sixteen presses.
pub fn alice_moveset() -> MovesetContract {
    // **JAB — `challenge`.** She asks a question. Quick, short, and it is not
    // meant to end anything.
    let jab = strike(
        "challenge",
        "jab",
        0.05,
        0.05,
        0.13,
        (24.0, 0.0),
        (17.0, 13.0),
        3,
        48.0,
        1.05,
        None,
        None,
    );
    let jab = vfx_at(jab, 0.05, "four_point_glint", (24.0, 0.0), GLYPH_FX);
    let jab = on_contact(jab, "player.hit");

    // **FORWARD TILT — `cipher_sweep`.** The reach the module doc claims, spent
    // on the press that uses it most.
    let f_tilt = strike(
        "cipher_sweep",
        "attack_side",
        0.08,
        0.07,
        0.17,
        (32.0, -2.0),
        (22.0, 14.0),
        6,
        72.0,
        1.28,
        Some((1.0, -0.28)),
        None,
    );
    let f_tilt = vfx_at(f_tilt, 0.08, "rune_burst", (32.0, -2.0), GLYPH_FX);
    let f_tilt = on_contact(f_tilt, "player.hit");

    // **UP TILT — `nonce_flick`.** A number used once, thrown straight up.
    let u_tilt = strike(
        "nonce_flick",
        "attack_up",
        0.07,
        0.07,
        0.17,
        (8.0, -26.0),
        (16.0, 20.0),
        5,
        74.0,
        1.32,
        Some((0.12, -1.0)),
        None,
    );
    let u_tilt = vfx_at(u_tilt, 0.07, "pickup_twinkle", (8.0, -26.0), GLYPH_FX);
    let u_tilt = on_contact(u_tilt, "player.hit");

    // **DOWN TILT — `padding_oracle`.** She asks the floor one bit at a time.
    let d_tilt = strike(
        "padding_oracle",
        "attack_down",
        0.07,
        0.06,
        0.16,
        (26.0, 13.0),
        (20.0, 10.0),
        4,
        54.0,
        1.16,
        Some((0.9, -0.35)),
        None,
    );
    let d_tilt = vfx_at(d_tilt, 0.07, "phase_ripple", (26.0, 13.0), GLYPH_FX);
    let d_tilt = on_contact(d_tilt, "player.hit");

    // **FORWARD SMASH — `brute_force`.** Every key in order until one opens.
    let f_smash = strike(
        "brute_force",
        "smash_forward",
        0.17,
        0.09,
        0.28,
        (38.0, -2.0),
        (28.0, 20.0),
        13,
        124.0,
        2.20,
        Some((0.95, -0.42)),
        None,
    );
    let f_smash = vfx_at(f_smash, 0.17, "magic_seal_break", (38.0, -2.0), SEAL_FX);
    let f_smash = sfx(f_smash, 0.17, "player.attack.charge");
    let f_smash = on_contact(f_smash, "player.hit");

    // **UP SMASH — `birthday_attack`.** Two of them meet overhead, which is more
    // likely than anybody expects.
    let u_smash = strike(
        "birthday_attack",
        "smash_up",
        0.16,
        0.10,
        0.28,
        (4.0, -32.0),
        (22.0, 32.0),
        12,
        120.0,
        2.25,
        Some((0.10, -1.0)),
        None,
    );
    let u_smash = vfx_at(u_smash, 0.16, "rune_circle", (4.0, -32.0), SEAL_FX);
    let u_smash = on_contact(u_smash, "player.hit");

    // **DOWN SMASH — `side_channel`.** She does not read the message; she reads
    // what leaked out either side of it.
    let d_smash = strike(
        "side_channel",
        "smash_down",
        0.17,
        0.09,
        0.30,
        (0.0, 19.0),
        (40.0, 12.0),
        11,
        112.0,
        2.00,
        Some((0.8, -0.58)),
        None,
    );
    let d_smash = vfx_at(d_smash, 0.17, "phase_ripple", (-28.0, 17.0), GLYPH_FX);
    let d_smash = vfx_at(d_smash, 0.17, "phase_ripple", (28.0, 17.0), GLYPH_FX);
    let d_smash = on_contact(d_smash, "player.hit");

    // **NEUTRAL AIR — `entropy_pool`.** Noise, all around her.
    let n_air = strike(
        "entropy_pool",
        "air_neutral",
        0.06,
        0.10,
        0.16,
        (0.0, 0.0),
        (26.0, 22.0),
        6,
        68.0,
        1.38,
        Some((0.55, -0.75)),
        None,
    );
    let n_air = vfx_at(n_air, 0.06, "rune_burst", (0.0, 0.0), GLYPH_FX);
    let n_air = on_contact(n_air, "player.hit");

    // **FORWARD AIR — `signature`.** She signs it on the way past.
    let f_air = strike(
        "signature",
        "air_forward",
        0.08,
        0.07,
        0.18,
        (30.0, -4.0),
        (22.0, 17.0),
        8,
        92.0,
        1.70,
        Some((0.95, -0.45)),
        None,
    );
    let f_air = vfx_at(f_air, 0.08, "four_point_glint", (30.0, -4.0), GLYPH_FX);
    let f_air = on_contact(f_air, "player.hit");

    // **BACK AIR — `revocation`.** The key is withdrawn behind her, hard.
    let b_air = strike(
        "revocation",
        "air_back",
        0.09,
        0.06,
        0.20,
        (-30.0, -2.0),
        (22.0, 17.0),
        9,
        100.0,
        1.85,
        Some((-0.95, -0.40)),
        None,
    );
    let b_air = vfx_at(b_air, 0.09, "magic_seal_break", (-30.0, -2.0), GLYPH_FX);
    let b_air = on_contact(b_air, "player.hit");

    // **UP AIR — `public_key`.** Held up where anyone may take it.
    let u_air = strike(
        "public_key",
        "air_up",
        0.07,
        0.08,
        0.17,
        (2.0, -26.0),
        (19.0, 22.0),
        7,
        84.0,
        1.62,
        Some((0.08, -1.0)),
        None,
    );
    let u_air = vfx_at(u_air, 0.07, "pickup_twinkle", (2.0, -26.0), GLYPH_FX);
    let u_air = on_contact(u_air, "player.hit");

    // **DOWN AIR — `known_plaintext`.** She already knows what is under you.
    let d_air = strike(
        "known_plaintext",
        "air_down",
        0.11,
        0.07,
        0.22,
        (2.0, 24.0),
        (19.0, 19.0),
        9,
        106.0,
        1.95,
        Some((0.0, 1.0)),
        None,
    );
    let d_air = vfx_at(d_air, 0.11, "rune_burst", (2.0, 24.0), GLYPH_FX);
    let d_air = on_contact(d_air, "player.hit");

    // **NEUTRAL — `one_time_pad`.** Used once and never again: her single
    // hardest hit, with the recovery to match.
    let n_b = strike(
        "one_time_pad",
        "attack",
        0.18,
        0.09,
        0.32,
        (30.0, -4.0),
        (28.0, 22.0),
        13,
        118.0,
        2.10,
        Some((0.9, -0.48)),
        None,
    );
    let n_b = committed_tail(n_b, 0.66, 0.05);
    let n_b = vfx_at(n_b, 0.18, "magic_seal_break", (30.0, -4.0), SEAL_FX);
    let n_b = sfx(n_b, 0.18, "player.directional_special");
    let n_b = on_contact(n_b, "player.hit");

    // **SIDE — `key_exchange`.** She crosses the gap and arrives having already
    // agreed the terms. ⭐ `Set`, so the distance is the move's and not her
    // momentum's.
    let side_b = strike(
        "key_exchange",
        "attack_side",
        0.13,
        0.10,
        0.24,
        (28.0, 0.0),
        (24.0, 18.0),
        10,
        104.0,
        1.92,
        Some((0.95, -0.36)),
        None,
    );
    let side_b = impulse(side_b, 0.13, (640.0, 0.0), ImpulseMode::Set);
    let side_b = committed_tail(side_b, 0.58, 0.10);
    let side_b = vfx_at(side_b, 0.13, "phase_ripple", (0.0, 0.0), SEAL_FX);
    let side_b = sfx(side_b, 0.13, "player.blink");
    let side_b = on_contact(side_b, "player.hit");

    // **UP — `elliptic_curve`. THE RECOVERY.** She rides the curve up. Cheap to
    // land, because her whole design is that a whiff does not end her.
    let mut up_b = strike(
        "elliptic_curve",
        "attack_up",
        0.07,
        0.12,
        0.18,
        (0.0, -12.0),
        (19.0, 30.0),
        7,
        84.0,
        1.60,
        Some((0.12, -1.0)),
        None,
    );
    up_b.landing_lag_s = Some(0.22);
    let up_b = impulse(up_b, 0.07, (0.0, -760.0), ImpulseMode::Set);
    let up_b = committed_tail(up_b, 0.48, 0.25);
    let up_b = vfx_at(up_b, 0.07, "rune_circle", (0.0, 0.0), GLYPH_FX);
    let up_b = sfx(up_b, 0.07, "player.double_jump");
    let up_b = on_contact(up_b, "player.hit");

    // **DOWN — `hash_collision`.** Two inputs, one output, on the floor either
    // side of her.
    let down_b = strike(
        "hash_collision",
        "attack_down",
        0.15,
        0.09,
        0.30,
        (0.0, 18.0),
        (36.0, 12.0),
        10,
        96.0,
        1.72,
        Some((0.75, -0.62)),
        None,
    );
    let down_b = committed_tail(down_b, 0.62, 0.0);
    let down_b = vfx_at(down_b, 0.15, "magic_seal_break", (0.0, 16.0), SEAL_FX);
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
    // **DOWN, IN THE AIR — `collision_dive`.** Two inputs meeting at one output
    // still works with no floor under it; she just arrives at the output.
    let mut air_down_b = strike(
        "collision_dive",
        "air_down",
        0.10,
        0.09,
        0.24,
        (0.0, 23.0),
        (20.0, 20.0),
        9,
        94.0,
        1.70,
        Some((0.0, 1.0)),
        None,
    );
    air_down_b.landing_lag_s = Some(0.24);
    let air_down_b = impulse(air_down_b, 0.10, (0.0, 1180.0), ImpulseMode::Set);
    let air_down_b = vfx_at(air_down_b, 0.10, "magic_seal_break", (0.0, 20.0), GLYPH_FX);
    let air_down_b = on_contact(air_down_b, "player.hit");

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

    /// **Alice is not Bob with different names.** The pair's split is the
    /// design: she reaches further and recovers sooner, he hits harder and
    /// commits longer. A table copied between them would pass every other test.
    #[test]
    fn alice_reaches_further_than_bob_and_bob_hits_harder() {
        let alice = alice_moveset();
        let bob = crate::bob_moveset::bob_moveset();
        let reach = |set: &MovesetContract, id: &str| {
            set.move_by_id(id)
                .unwrap_or_else(|| panic!("{id} exists"))
                .windows
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
        let damage = |set: &MovesetContract, id: &str| {
            set.move_by_id(id)
                .unwrap_or_else(|| panic!("{id} exists"))
                .windows
                .iter()
                .flat_map(|w| w.volumes.iter())
                .map(|v| v.damage)
                .max()
                .unwrap_or(0)
        };
        assert!(
            reach(&alice, "cipher_sweep") > reach(&bob, "wrench_swing"),
            "the sender reaches further"
        );
        assert!(
            damage(&bob, "rivet_smash") > damage(&alice, "brute_force"),
            "and the one who builds things hits harder when he connects"
        );
    }
}

//! Alice's repertoire — the cryptographer, and the one who SENDS.
//!
//! ## The character, from her own name
//!
//! Alice and Bob are the two names cryptography uses for the two ends of a
//! channel, and the split is the design: Alice sends, Bob receives. So her
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
//! The day she gets her own art, this file is where the names change and nothing else does.

use ambition_characters::moveset_authoring::Strike;
use ambition_characters::smash_capture::{
    author_pummel, author_standing_grab, author_throw, capture_beat, grab_shell,
    CaptureAttemptParams, CaptureCues, CapturePummelParams, CaptureThrowParams,
    SmashCaptureRepertoire,
};
use ambition_characters::smash_repertoire::{
    DownSpecial, NeutralSpecial, SmashRepertoire, UpSpecial,
};
use ambition_platformer2d::entity_catalog::{ImpulseMode, MovesetContract};

use ambition_characters::moveset_authoring::{
    committed_tail, impulse, on_contact, sfx, strike, vfx_at,
};

/// How big a cipher's burst draws.
const GLYPH_FX: f32 = 0.85;
const SEAL_FX: f32 = 1.15;

/// See the module doc. Sixteen presses.
pub fn alice_moveset() -> MovesetContract {
    // JAB — `challenge`. She asks a question. Quick, short, and it is not
    // meant to end anything.
    let jab = strike(Strike {
        id: "challenge",
        clip: "jab",
        startup_s: 0.05,
        active_s: 0.05,
        recover_s: 0.13,
        offset: (24.0, 0.0),
        half_extents: (17.0, 13.0),
        damage: 3,
        knockback: 48.0,
        knockback_growth: 1.05,
        launch_dir: None,
        on_hit: None,
    });
    let jab = vfx_at(jab, 0.05, "four_point_glint", (24.0, 0.0), GLYPH_FX);
    let jab = on_contact(jab, "player.hit");

    // FORWARD TILT — `cipher_sweep`. The reach the module doc claims, spent
    // on the press that uses it most.
    let f_tilt = strike(Strike {
        id: "cipher_sweep",
        clip: "attack_side",
        startup_s: 0.08,
        active_s: 0.07,
        recover_s: 0.17,
        offset: (32.0, -2.0),
        half_extents: (22.0, 14.0),
        damage: 6,
        knockback: 72.0,
        knockback_growth: 1.28,
        launch_dir: Some((1.0, -0.28)),
        on_hit: None,
    });
    let f_tilt = vfx_at(f_tilt, 0.08, "rune_burst", (32.0, -2.0), GLYPH_FX);
    let f_tilt = on_contact(f_tilt, "player.hit");

    // UP TILT — `nonce_flick`. A number used once, thrown straight up.
    let u_tilt = strike(Strike {
        id: "nonce_flick",
        clip: "attack_up",
        startup_s: 0.07,
        active_s: 0.07,
        recover_s: 0.17,
        offset: (8.0, -26.0),
        half_extents: (16.0, 20.0),
        damage: 5,
        knockback: 74.0,
        knockback_growth: 1.32,
        launch_dir: Some((0.12, -1.0)),
        on_hit: None,
    });
    let u_tilt = vfx_at(u_tilt, 0.07, "pickup_twinkle", (8.0, -26.0), GLYPH_FX);
    let u_tilt = on_contact(u_tilt, "player.hit");

    // DOWN TILT — `padding_oracle`. She asks the floor one bit at a time.
    let d_tilt = strike(Strike {
        id: "padding_oracle",
        clip: "attack_down",
        startup_s: 0.07,
        active_s: 0.06,
        recover_s: 0.16,
        offset: (26.0, 13.0),
        half_extents: (20.0, 10.0),
        damage: 4,
        knockback: 54.0,
        knockback_growth: 1.16,
        launch_dir: Some((0.9, -0.35)),
        on_hit: None,
    });
    let d_tilt = vfx_at(d_tilt, 0.07, "phase_ripple", (26.0, 13.0), GLYPH_FX);
    let d_tilt = on_contact(d_tilt, "player.hit");

    // FORWARD SMASH — `brute_force`. Every key in order until one opens.
    let f_smash = strike(Strike {
        id: "brute_force",
        clip: "smash_forward",
        startup_s: 0.17,
        active_s: 0.09,
        recover_s: 0.28,
        offset: (38.0, -2.0),
        half_extents: (28.0, 20.0),
        damage: 13,
        knockback: 124.0,
        knockback_growth: 2.20,
        launch_dir: Some((0.95, -0.42)),
        on_hit: None,
    });
    let f_smash = vfx_at(f_smash, 0.17, "magic_seal_break", (38.0, -2.0), SEAL_FX);
    let f_smash = sfx(f_smash, 0.17, "player.attack.charge");
    let f_smash = on_contact(f_smash, "player.hit");

    // UP SMASH — `birthday_attack`. Two of them meet overhead, which is more
    // likely than anybody expects.
    let u_smash = strike(Strike {
        id: "birthday_attack",
        clip: "smash_up",
        startup_s: 0.16,
        active_s: 0.10,
        recover_s: 0.28,
        offset: (4.0, -32.0),
        half_extents: (22.0, 32.0),
        damage: 12,
        knockback: 120.0,
        knockback_growth: 2.25,
        launch_dir: Some((0.10, -1.0)),
        on_hit: None,
    });
    let u_smash = vfx_at(u_smash, 0.16, "rune_circle", (4.0, -32.0), SEAL_FX);
    let u_smash = on_contact(u_smash, "player.hit");

    // DOWN SMASH — `side_channel`. She does not read the message; she reads
    // what leaked out either side of it.
    let d_smash = strike(Strike {
        id: "side_channel",
        clip: "smash_down",
        startup_s: 0.17,
        active_s: 0.09,
        recover_s: 0.30,
        offset: (0.0, 19.0),
        half_extents: (40.0, 12.0),
        damage: 11,
        knockback: 112.0,
        knockback_growth: 2.00,
        launch_dir: Some((0.8, -0.58)),
        on_hit: None,
    });
    let d_smash = vfx_at(d_smash, 0.17, "phase_ripple", (-28.0, 17.0), GLYPH_FX);
    let d_smash = vfx_at(d_smash, 0.17, "phase_ripple", (28.0, 17.0), GLYPH_FX);
    let d_smash = on_contact(d_smash, "player.hit");

    // NEUTRAL AIR — `entropy_pool`. Noise, all around her.
    let n_air = strike(Strike {
        id: "entropy_pool",
        clip: "air_neutral",
        startup_s: 0.06,
        active_s: 0.10,
        recover_s: 0.16,
        offset: (0.0, 0.0),
        half_extents: (26.0, 22.0),
        damage: 6,
        knockback: 68.0,
        knockback_growth: 1.38,
        launch_dir: Some((0.55, -0.75)),
        on_hit: None,
    });
    let n_air = vfx_at(n_air, 0.06, "rune_burst", (0.0, 0.0), GLYPH_FX);
    let n_air = on_contact(n_air, "player.hit");

    // FORWARD AIR — `signature`. She signs it on the way past.
    let f_air = strike(Strike {
        id: "signature",
        clip: "air_forward",
        startup_s: 0.08,
        active_s: 0.07,
        recover_s: 0.18,
        offset: (30.0, -4.0),
        half_extents: (22.0, 17.0),
        damage: 8,
        knockback: 92.0,
        knockback_growth: 1.70,
        launch_dir: Some((0.95, -0.45)),
        on_hit: None,
    });
    let f_air = vfx_at(f_air, 0.08, "four_point_glint", (30.0, -4.0), GLYPH_FX);
    let f_air = on_contact(f_air, "player.hit");

    // BACK AIR — `revocation`. The key is withdrawn behind her, hard.
    let b_air = strike(Strike {
        id: "revocation",
        clip: "air_back",
        startup_s: 0.09,
        active_s: 0.06,
        recover_s: 0.20,
        offset: (-30.0, -2.0),
        half_extents: (22.0, 17.0),
        damage: 9,
        knockback: 100.0,
        knockback_growth: 1.85,
        launch_dir: Some((-0.95, -0.40)),
        on_hit: None,
    });
    let b_air = vfx_at(b_air, 0.09, "magic_seal_break", (-30.0, -2.0), GLYPH_FX);
    let b_air = on_contact(b_air, "player.hit");

    // UP AIR — `public_key`. Held up where anyone may take it.
    let u_air = strike(Strike {
        id: "public_key",
        clip: "air_up",
        startup_s: 0.07,
        active_s: 0.08,
        recover_s: 0.17,
        offset: (2.0, -26.0),
        half_extents: (19.0, 22.0),
        damage: 7,
        knockback: 84.0,
        knockback_growth: 1.62,
        launch_dir: Some((0.08, -1.0)),
        on_hit: None,
    });
    let u_air = vfx_at(u_air, 0.07, "pickup_twinkle", (2.0, -26.0), GLYPH_FX);
    let u_air = on_contact(u_air, "player.hit");

    // DOWN AIR — `known_plaintext`. She already knows what is under you.
    let d_air = strike(Strike {
        id: "known_plaintext",
        clip: "air_down",
        startup_s: 0.11,
        active_s: 0.07,
        recover_s: 0.22,
        offset: (2.0, 24.0),
        half_extents: (19.0, 19.0),
        damage: 9,
        knockback: 106.0,
        knockback_growth: 1.95,
        launch_dir: Some((0.0, 1.0)),
        on_hit: None,
    });
    let d_air = vfx_at(d_air, 0.11, "rune_burst", (2.0, 24.0), GLYPH_FX);
    let d_air = on_contact(d_air, "player.hit");

    // NEUTRAL — `one_time_pad`. Used once and never again: her single
    // hardest hit, with the recovery to match.
    let n_b = strike(Strike {
        id: "one_time_pad",
        clip: "attack",
        startup_s: 0.18,
        active_s: 0.09,
        recover_s: 0.32,
        offset: (30.0, -4.0),
        half_extents: (28.0, 22.0),
        damage: 13,
        knockback: 118.0,
        knockback_growth: 2.10,
        launch_dir: Some((0.9, -0.48)),
        on_hit: None,
    });
    let n_b = committed_tail(n_b, 0.66, 0.05);
    let n_b = vfx_at(n_b, 0.18, "magic_seal_break", (30.0, -4.0), SEAL_FX);
    let n_b = sfx(n_b, 0.18, "player.directional_special");
    let n_b = on_contact(n_b, "player.hit");

    // SIDE — `key_exchange`. She crosses the gap and arrives having already
    // agreed the terms. `Set`, so the distance is the move's and not her
    // momentum's.
    let side_b = strike(Strike {
        id: "key_exchange",
        clip: "attack_side",
        startup_s: 0.13,
        active_s: 0.10,
        recover_s: 0.24,
        offset: (28.0, 0.0),
        half_extents: (24.0, 18.0),
        damage: 10,
        knockback: 104.0,
        knockback_growth: 1.92,
        launch_dir: Some((0.95, -0.36)),
        on_hit: None,
    });
    let side_b = impulse(side_b, 0.13, (640.0, 0.0), ImpulseMode::Set);
    let side_b = committed_tail(side_b, 0.58, 0.10);
    let side_b = vfx_at(side_b, 0.13, "phase_ripple", (0.0, 0.0), SEAL_FX);
    let side_b = sfx(side_b, 0.13, "player.blink");
    let side_b = on_contact(side_b, "player.hit");

    // UP — `elliptic_curve`. THE RECOVERY, AND IT IS A PORTAL PAIR, not an arc.
    //
    // ⭐⭐ JON'S MOVE, 2026-09-05: *"up b opens a portal under him, and a portal
    // at the very top of the stage, and when he falls into it he comes out the
    // higher portal … it's a portal so just use the portal crate rules."*
    // ⇒ So the RISE comes from `ambition_portal2d`'s own transit and not from an
    // impulse this move throws. Everything else about the move is unchanged —
    // the swing still hits, the rune still draws, the landing lag still costs.
    //
    // ⛔ THE IMPULSE IS GONE, DELIBERATELY, and this is the one behavioural
    // change: a `-760` `Set` alongside the pair would make the portal decorative
    // and the arc the actual recovery, which is the opposite of the move. ⚠ If
    // this reads worse in play than the curve did, the impulse is one line to
    // restore — the slot is provisional (Jon: *"we can tune who the moves belong
    // to later"*) and so is this.
    //
    // ⓘ Cheap to land, because her whole design is that a whiff does not end her.
    let mut up_b = strike(Strike {
        id: "elliptic_curve",
        clip: "attack_up",
        startup_s: 0.07,
        active_s: 0.12,
        recover_s: 0.18,
        offset: (0.0, -12.0),
        half_extents: (19.0, 30.0),
        damage: 7,
        knockback: 84.0,
        knockback_growth: 1.60,
        launch_dir: Some((0.12, -1.0)),
        on_hit: None,
    });
    up_b.landing_lag_s = Some(0.22);
    // ⭐ THE PAIR OPENS ON THE SAME BEAT THE IMPULSE USED TO FIRE, so the
    // move's timing is untouched: the rune draws, and the way up is there.
    let up_b = ambition_characters::smash_portal::author_portal_pair(
        up_b,
        0.07,
        ambition_characters::smash_portal::PortalPairParams {
            // ⚠ TUNING, and a knob rather than a derived constant. 320px is a
            // little over twice the engine's own jump ceiling, so the exit is
            // somewhere she could not simply jump to — which is the whole
            // reason to open a hole instead.
            rise: 320.0,
            // Wide and shallow: you fall INTO it, so the horizontal mouth is
            // what matters and depth would only make it a wall.
            half_extent: (26.0, 6.0),
            // Long enough to fall through after the animation, short enough that
            // the stage is not permanently rearranged by one recovery.
            lifetime_s: 2.5,
            // ⛔ A ROUTE, NOT AN ESCAPE HATCH — it stays open for anyone,
            // including whoever is chasing her. That is the interesting version
            // and it is the one that makes the move a stage event rather than a
            // private button; `true` would shut it behind her.
            close_on_transit: false,
            // Straight first. The angled variant is Jon's flavour and wants its
            // own commit, because it is also the cheapest test of whether the
            // placement seam takes an orientation at all.
            // ⛔ THE BASE STAYS STRAIGHT. Her recovery must work when the
            // player asks for nothing — an up-B that leaned by default would
            // punish a neutral stick, which is what a panicked player holds.
            tilt_degrees: 0.0,
            // ⭐⭐ AND THE PLAYER ANGLES IT — Jon, 2026-09-05: *"we can even
            // exercise angled portals with directional input on the up b as a
            // flavor that isn't actually in smash and is ours."* Hold a
            // direction on the way out and the shaft leans that way, so the
            // recovery covers horizontal distance as well as vertical.
            //
            // ⚠ 32° EITHER WAY, and the cap is doing real work: at 45° the exit
            // normal is as horizontal as it is vertical and the pair stops being
            // a way UP at all. This leaves the move unmistakably a recovery
            // while giving the angle enough range to be worth aiming.
            aim_tilt_degrees: 32.0,
            // ⛔ 8+, never 0..=7: the low eight overlap the NAMED authored pairs
            // and a room that authored that colour would find its portals linked
            // to hers.
            channel_index: 8,
        },
    );
    let up_b = committed_tail(up_b, 0.48, 0.25);
    let up_b = vfx_at(up_b, 0.07, "rune_circle", (0.0, 0.0), GLYPH_FX);
    let up_b = sfx(up_b, 0.07, "player.double_jump");
    let up_b = on_contact(up_b, "player.hit");

    // DOWN — `hash_collision`. Two inputs, one output, on the floor either
    // side of her.
    let down_b = strike(Strike {
        id: "hash_collision",
        clip: "attack_down",
        startup_s: 0.15,
        active_s: 0.09,
        recover_s: 0.30,
        offset: (0.0, 18.0),
        half_extents: (36.0, 12.0),
        damage: 10,
        knockback: 96.0,
        knockback_growth: 1.72,
        launch_dir: Some((0.75, -0.62)),
        on_hit: None,
    });
    let down_b = committed_tail(down_b, 0.62, 0.0);
    let down_b = vfx_at(down_b, 0.15, "magic_seal_break", (0.0, 16.0), SEAL_FX);
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
    // DOWN, IN THE AIR — `collision_dive`. Two inputs meeting at one output
    // still works with no floor under it; she just arrives at the output.
    let mut air_down_b = strike(Strike {
        id: "collision_dive",
        clip: "air_down",
        startup_s: 0.10,
        active_s: 0.09,
        recover_s: 0.24,
        offset: (0.0, 23.0),
        half_extents: (20.0, 20.0),
        damage: 9,
        knockback: 94.0,
        knockback_growth: 1.70,
        launch_dir: Some((0.0, 1.0)),
        on_hit: None,
    });
    air_down_b.landing_lag_s = Some(0.24);
    let air_down_b = impulse(air_down_b, 0.10, (0.0, 1180.0), ImpulseMode::Set);
    let air_down_b = vfx_at(air_down_b, 0.10, "magic_seal_break", (0.0, 20.0), GLYPH_FX);
    let air_down_b = on_contact(air_down_b, "player.hit");
    // ALICE'S CAPTURE KIT. Quick and low-committal: the shortest startup on the
    // roster and a middling pummel. Her throw stays flat-ish and pushes for stage
    // control rather than for a kill.
    // the grab draws `attack`, not `grab`: these sheets publish no `grab` row,
    // and each table's own `every_clip_names_a_row_..._sheet_carries` guard says
    // so. `ClipBinding`'s fallbacks would have covered it at runtime, but a move
    // that NAMES a row nobody publishes is a lie the guard is right to refuse.
    let grab = author_standing_grab(
        grab_shell("alice_grab", "attack", 0.06, 0.05, 0.18),
        CaptureAttemptParams {
            offset: (12.0, 1.0),
            half_extents: (18.0, 15.0),
            hold_offset: (13.0, 3.0),
        },
    );
    let pummel = author_pummel(
        capture_beat("alice_pummel", "attack", 0.16),
        0.07,
        CapturePummelParams { damage: 3 },
    );
    let forward_throw = author_throw(
        capture_beat("alice_fthrow", "attack", 0.24),
        0.13,
        CaptureThrowParams {
            damage: 7,
            knockback: 118.0,
            knockback_growth: 2.1,
            launch_dir: (0.9, -0.45),
        },
    );

    let back_throw = author_throw(
        capture_beat("alice_bthrow", "attack", 0.26),
        0.14,
        CaptureThrowParams {
            damage: 8,
            knockback: 127.44,
            knockback_growth: 2.21,
            launch_dir: (-1.0, -0.28),
        },
    );

    let up_throw = author_throw(
        capture_beat("alice_uthrow", "attack", 0.25),
        0.13,
        CaptureThrowParams {
            damage: 7,
            knockback: 122.72,
            knockback_growth: 2.14,
            launch_dir: (0.0, -1.0),
        },
    );

    let down_throw = author_throw(
        capture_beat("alice_dthrow", "attack", 0.27),
        0.14,
        CaptureThrowParams {
            damage: 5,
            knockback: 87.32,
            knockback_growth: 1.68,
            launch_dir: (0.36, -0.92),
        },
    );

    SmashRepertoire {
        taunt: ambition_characters::moveset_authoring::taunt("alice_taunt", 0.9),
        dash_attack: ambition_characters::moveset_authoring::dash_attack(
            "alice_dash_attack",
            ambition_characters::moveset_authoring::DashAttackShape::GENRE,
            8,
            90.0,
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

    /// Alice is not Bob with different names. The pair's split is the
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

#[cfg(test)]
mod portal_recovery_tests {
    use ambition_characters::smash_portal::{PortalPairParams, PORTAL_PAIR};
    use ambition_platformer2d::entity_catalog::MoveEventKind;

    /// Her up-B opens a portal pair, and does NOT also throw an impulse.
    ///
    /// ⛔⛔ THE SECOND HALF IS THE ASSERTION THAT MATTERS. A move carrying both
    /// would recover on the impulse and leave the portals as scenery — the
    /// pair would look right, play wrong, and every test asserting "it opens a
    /// portal" would still pass. ⇒ The recovery is the portal or it is not this
    /// move.
    #[test]
    fn the_up_special_recovers_through_a_portal_rather_than_an_arc() {
        let kit = super::alice_moveset();
        let up_b = kit
            .moves
            .iter()
            .find(|m| m.id == "elliptic_curve")
            .expect("Alice authors her up-special");

        let pair = up_b
            .events
            .iter()
            .find_map(|ev| match &ev.kind {
                MoveEventKind::Effect(effect) if effect.key == PORTAL_PAIR => Some(effect),
                _ => None,
            })
            .expect("the up-special opens a portal pair");
        let params: PortalPairParams = pair.params.hydrate().expect("portal params hydrate");
        assert!(
            params.rise > 0.0,
            "the pair's exit is not above its entrance, so falling in returns \
             her where she started"
        );

        assert!(
            up_b.start_impulse.is_none(),
            "the up-special still throws a start impulse alongside its portal \
             pair, so the arc is the recovery and the portals are scenery"
        );
        // ⛔⛔ AN IMPULSE IS ITS OWN EVENT KIND, not an effect with a telling
        // name. The first version of this assertion looked for
        // `Effect { key: contains("impulse") }`, which cannot match anything —
        // poisoning the impulse back in left the test GREEN, and that is the
        // only reason this line is right.
        let thrown: Vec<(f32, &ambition_platformer2d::entity_catalog::ImpulseMode)> = up_b
            .events
            .iter()
            .filter_map(|ev| match &ev.kind {
                MoveEventKind::Impulse { mode, .. } => Some((ev.at_s, mode)),
                _ => None,
            })
            .collect();
        assert!(
            thrown.is_empty(),
            "the up-special throws {thrown:?} beside its portal pair, so the arc \
             is the recovery and the portals are scenery"
        );
    }
}

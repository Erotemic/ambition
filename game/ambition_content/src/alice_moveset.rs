//! Alice's repertoire: the cryptographer, and the one who sends.
//!
//! ## The character, from her own name
//!
//! Alice and Bob are the two names cryptography uses for the two ends of a
//! channel, and the split is the design: Alice sends, Bob receives. So her
//! kit is about getting something across: the longest reach among the Hall's
//! people, the quickest recovery on a whiff, and a side special that is a key
//! exchange: she crosses the gap and arrives having agreed on the terms.
//!
//! ```text
//!            reach   jab startup   f-smash damage   the trade
//!   goblin    22 px     0.04 s          12          fast, cheap, close
//!   alice     28 px     0.05 s          13          reach and recovery
//!   bob       26 px     0.07 s          16          slow, and it lands
//! ```
//!
//! When she gets her own art, only the names in this file change.

use ambition_entity_catalog::authoring::Strike;
use ambition_entity_catalog::smash_capture::{
    author_pummel, author_standing_grab, author_throw, capture_beat, grab_shell,
    CaptureAttemptParams, CaptureCues, CapturePummelParams, CaptureThrowParams,
    SmashCaptureRepertoire,
};
use ambition_entity_catalog::smash_repertoire::{
    DownSpecial, NeutralSpecial, SmashRepertoire, UpSpecial,
};
use ambition_entity_catalog::{AutolinkVolume, ImpulseMode, MovesetContract};

use ambition_entity_catalog::authoring::{
    armor, committed_tail, impulse, multihit, on_contact, sfx, strike, vfx_at, Pulse,
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
        knockback_growth: 3.12,
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
        knockback_growth: 5.81,
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
        knockback_growth: 2.68,
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
    // Armour: a one-time pad is the one cipher that cannot be broken. She takes
    // a hit during the wind-up and swings anyway.
    //
    // Only over the startup, 0.06s..0.18s, ending when the hitbox opens. Armour
    // over the active frames would win every simultaneous exchange; this rewards
    // committing first.
    //
    // She still takes the damage: `WindowTag::Armor` is not invulnerability, and
    // a hit before 0.06s still beats it.
    let n_b = armor(n_b, 0.06, 0.18);
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

    // Up: `elliptic_curve`. The recovery is a portal pair, not an arc.
    //
    // Jon: *"up b opens a portal under him, and a portal at the very top of the
    // stage, and when he falls into it he comes out the higher portal … it's a
    // portal so just use the portal crate rules."* So the rise comes from
    // `ambition_portal2d`'s transit, not an impulse. The swing still hits, the
    // rune still draws, the landing lag still costs.
    //
    // No impulse: a `-760` `Set` beside the pair would make the portals
    // decorative. If it plays worse, the impulse is one line to restore; the
    // slot is provisional (Jon: *"we can tune who the moves belong to later"*).
    //
    // Cheap to land: a whiff does not end her.
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
    // The pair opens on the beat the impulse used to fire, so the timing is
    // unchanged.
    let up_b = ambition_entity_catalog::smash_portal::author_portal_pair(
        up_b,
        0.07,
        ambition_entity_catalog::smash_portal::PortalPairParams {
            // Jon: *"the second portal appears too high, I want it to be placed so its
            // above the main surface level, but in the visible part of the stage."* The
            // smash ruleset's `CEILING_BLAST_MARGIN_PX` is 240, so a 320px rise put the
            // exit outside the playable box.
            //
            // Still a knob: 150px is about three body heights, above her jump reach,
            // and inside the visible stage.
            rise: 150.0,
            // Wide and shallow: you fall into it, so the horizontal mouth matters.
            half_extent: (26.0, 6.0),
            // Long enough to fall through after the animation; short enough that one
            // recovery does not rearrange the stage.
            lifetime_s: 2.5,
            // A route, not an escape hatch: it stays open for anyone, including her
            // chaser. That makes it a stage event; `true` would shut it behind her.
            close_on_transit: false,
            // The base stays straight, so the recovery works with a neutral stick
            // (what a panicked player holds).
            tilt_degrees: 0.0,
            // The player can angle it: hold a direction on the way out and the shaft
            // leans that way, so the recovery covers horizontal distance too (a flavour
            // that is not in Smash).
            //
            // 32° either way. At 45° the exit is as horizontal as vertical and stops
            // being a way up.
            aim_tilt_degrees: 32.0,
            // 8 or more, never 0..=7: the low eight overlap the named authored pairs,
            // and a room using that colour would link its portals to hers.
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
    // Two inputs, one output: two holding pulses, then the launch. The pulses
    // use `VolumeReaction::Autolink`, the genre's multi-hit rule: intermediate
    // hits keep the victim in the next box, and only the last one launches.
    //
    // The anchor's x is zero: the hold point is mirrored by facing, and this move
    // is symmetric about her.
    let down_b = multihit(
        down_b,
        2,
        Pulse {
            // Tighter than the finishing sweep: the collision happens at the digest.
            offset: (0.0, 16.0),
            half_extents: (28.0, 12.0),
            damage: 2,
            // Separated: the runtime's re-hit rule treats touching windows as one
            // track, so they would hit once.
            active_s: 0.030,
            gap_s: 0.028,
            autolink: AutolinkVolume {
                anchor: (0.0, 6.0),
                // She is planted, so she has no motion to pass on; the hold is all
                // correction.
                carry: 0.0,
                pull: 19.0,
                max_speed: 900.0,
            },
        },
    );
    let down_b = committed_tail(down_b, 0.62, 0.0);
    let down_b = vfx_at(down_b, 0.15, "magic_seal_break", (0.0, 16.0), SEAL_FX);
    let down_b = on_contact(down_b, "player.hit");

    // Down-B has two forms, like Bowser's: a slam in the air, an arc and slam
    // on the ground. Context-dependent specials are acceptable, though most
    // should not be.
    //
    // A special gated to one posture is not answered in the other: the
    // directional chain falls through to the neutral special.
    // `special_air_down` comes before `special_down` in that chain.
    // Down, in the air: `collision_dive`. With no floor, she arrives at the
    // output.
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
    // Alice's capture kit: quick and low-commitment, the shortest startup on
    // the roster and a middling pummel. Her throw is flat-ish, for stage control,
    // not kills.
    // The grab draws `attack`, not `grab`: these sheets publish no `grab` row,
    // and each table's clip guard refuses unpublished rows.
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
        taunt: ambition_entity_catalog::authoring::taunt("alice_taunt", 0.9),
        dash_attack: ambition_entity_catalog::authoring::dash_attack(
            "alice_dash_attack",
            ambition_entity_catalog::authoring::DashAttackShape::GENRE,
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
        // Every smash fighter has a grab. The values are per character on purpose.
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

    /// The pad is armoured during the wind-up and not after. The end of the
    /// armour is the test: armour over the active frames would win every
    /// simultaneous exchange, and a check for `WindowTag::Armor` alone would
    /// pass that.
    #[test]
    fn her_one_time_pad_is_armoured_only_while_she_winds_up() {
        use ambition_entity_catalog::WindowTag;
        let pad = crate::authored_movesets::shipped("npc_alice")
            .move_by_id("one_time_pad")
            .expect("one_time_pad exists")
            .clone();
        let armor = pad
            .windows
            .iter()
            .find(|w| w.tag == WindowTag::Armor)
            .expect("the unbreakable cipher is armoured");
        let active = pad
            .windows
            .iter()
            .find(|w| w.tag == WindowTag::Active && !w.volumes.is_empty())
            .expect("it is still a strike");
        assert!(
            armor.end_s <= active.start_s,
            "armour must close as the hitbox opens, or the trade is free: \
             armour ends {}, hitbox opens {}",
            armor.end_s,
            active.start_s,
        );
        assert!(
            armor.start_s > 0.0,
            "a fast enough answer must still beat it, so it cannot open on frame 0"
        );
    }

    /// `hash_collision` holds twice and launches once. The holds are
    /// `VolumeReaction::Autolink`; the finisher carries no reaction.
    ///
    /// The gap between pulses matters: the re-hit rule refuses a second hit
    /// across a contiguous track, so touching windows would land once.
    #[test]
    fn her_hash_collision_holds_twice_and_launches_once() {
        use ambition_entity_catalog::VolumeReaction;
        let collision = crate::authored_movesets::shipped("npc_alice")
            .move_by_id("hash_collision")
            .expect("hash_collision exists")
            .clone();
        let striking: Vec<_> = collision
            .windows
            .iter()
            .filter(|w| !w.volumes.is_empty())
            .collect();
        assert_eq!(
            striking.len(),
            3,
            "two inputs and one output: {} striking window(s)",
            striking.len()
        );
        for (index, window) in striking[..2].iter().enumerate() {
            assert!(
                window
                    .volumes
                    .iter()
                    .all(|v| matches!(v.reaction, Some(VolumeReaction::Autolink(_)))),
                "input {index} must HOLD, or the finisher swings at nobody"
            );
        }
        assert!(
            striking[2].volumes.iter().all(|v| v.reaction.is_none()),
            "the output must launch: a held finisher is a move that never lets go"
        );
        for pair in striking.windows(2) {
            assert!(
                pair[0].end_s < pair[1].start_s,
                "the windows must be SEPARATED or the runtime lands one hit: \
                 {} then {}",
                pair[0].end_s,
                pair[1].start_s,
            );
        }
    }

    // Verb binding is checked by construction: `SmashRepertoire` owns the verb
    // strings and is a struct with no `Default`, so a missing slot is a compile
    // error. Coverage in every posture is checked by
    // `ambition_entity_catalog::smash_repertoire` and by
    // `smash_roster_movesets::report_the_smash_kit_every_selectable_fighter_has`.

    /// Alice is not Bob with different names. The pair's split is the
    /// design: she reaches further and recovers sooner, he hits harder and
    /// commits longer. A table copied between them would pass every other test.
    #[test]
    fn alice_reaches_further_than_bob_and_bob_hits_harder() {
        let alice = crate::authored_movesets::shipped("npc_alice");
        let bob = crate::authored_movesets::shipped("npc_bob");
        let reach = |set: &MovesetContract, id: &str| {
            set.move_by_id(id)
                .unwrap_or_else(|| panic!("{id} exists"))
                .windows
                .iter()
                .flat_map(|w| w.volumes.iter())
                .map(|v| match v.shape {
                    ambition_entity_catalog::VolumeShape::Rect {
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
    use ambition_entity_catalog::smash_portal::{PortalPairParams, PORTAL_PAIR};
    use ambition_entity_catalog::MoveEventKind;

    /// Her up-B opens a portal pair and does not also throw an impulse. With both,
    /// she would recover on the impulse and the portals would be scenery, while
    /// "it opens a portal" still passed.
    #[test]
    fn the_up_special_recovers_through_a_portal_rather_than_an_arc() {
        let kit = crate::authored_movesets::shipped("npc_alice");
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
        // An impulse is its own event kind (`MoveEventKind::Impulse`), not an
        // `Effect` with a telling key.
        let thrown: Vec<(f32, &ambition_entity_catalog::ImpulseMode)> = up_b
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

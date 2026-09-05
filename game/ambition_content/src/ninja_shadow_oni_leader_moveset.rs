//! Shadow Oni Leader moveset.
//!
//! His attack identity is counter-punching: very fast startup, very short active
//! windows, and long recovery. The repertoire varies timing/commitment rather
//! than adding teleport, clone, or smoke mechanics; those belong to abilities or
//! techniques rather than hit definitions.

use ambition_characters::moveset_authoring::Strike;
use ambition_characters::smash_capture::{
    author_pummel, author_standing_grab, author_throw, capture_beat, grab_shell,
    CaptureAttemptParams, CaptureCues, CapturePummelParams, CaptureThrowParams,
    SmashCaptureRepertoire,
};
use ambition_characters::smash_repertoire::{
    DownSpecial, NeutralSpecial, SmashRepertoire, UpSpecial,
};
use ambition_platformer2d::entity_catalog::MovesetContract;

use ambition_characters::moveset_authoring::{
    committed_tail, impulse, on_contact, sfx, strike, vfx_at,
};
use ambition_platformer2d::entity_catalog::ImpulseMode;

/// See the module doc. Eleven moves, the genre's standard verb map.
pub fn ninja_shadow_oni_leader_moveset() -> MovesetContract {
    // ── grounded ─────────────────────────────────────────────────────────────
    //
    // the fastest jab in the game, and the shortest. It answers a goblin's
    // jab and beats it — and if the goblin was not there, the oni is standing
    // still for a fifth of a second holding an empty hand.
    let jab = strike(Strike {
        id: "jab",
        clip: "jab",
        startup_s: 0.03,
        active_s: 0.04,
        recover_s: 0.20,
        offset: (24.0, 0.0),
        half_extents: (17.0, 13.0),
        damage: 3,
        knockback: 50.0,
        knockback_growth: 1.05,
        launch_dir: None,
        on_hit: None,
    });

    let up_tilt = strike(Strike {
        id: "tilt_up",
        clip: "attack_up",
        startup_s: 0.05,
        active_s: 0.04,
        recover_s: 0.24,
        offset: (10.0, -26.0),
        half_extents: (17.0, 20.0),
        damage: 5,
        knockback: 72.0,
        knockback_growth: 1.30,
        launch_dir: Some((0.1, -1.0)),
        on_hit: None,
    });

    let down_tilt = strike(Strike {
        id: "tilt_down",
        clip: "attack_down",
        startup_s: 0.05,
        active_s: 0.04,
        recover_s: 0.22,
        offset: (24.0, 13.0),
        half_extents: (19.0, 10.0),
        damage: 4,
        knockback: 58.0,
        knockback_growth: 1.18,
        launch_dir: Some((1.0, -0.22)),
        on_hit: None,
    });

    // ── smashes ──────────────────────────────────────────────────────────────
    //
    // the FASTEST smashes on the grid and the most punishing to miss, which
    // is the whole character in one pair of numbers. Everybody else's kill move
    // is slow to start; his is slow to *end*. A goblin that eats it was caught
    // reacting; a goblin that saw it coming gets 0.44s to answer.
    let mut f_smash = strike(Strike {
        id: "smash_forward",
        clip: "smash_forward",
        startup_s: 0.20,
        active_s: 0.05,
        recover_s: 0.44,
        offset: (36.0, -2.0),
        half_extents: (26.0, 19.0),
        damage: 16,
        knockback: 150.0,
        knockback_growth: 3.05,
        launch_dir: Some((1.0, -0.42)),
        on_hit: None,
    });
    f_smash.smash_charge_mult = 1.7;

    let mut up_smash = strike(Strike {
        id: "smash_up",
        clip: "smash_up",
        startup_s: 0.18,
        active_s: 0.05,
        recover_s: 0.42,
        offset: (6.0, -32.0),
        half_extents: (22.0, 28.0),
        damage: 15,
        knockback: 148.0,
        knockback_growth: 2.90,
        launch_dir: Some((0.0, -1.0)),
        on_hit: None,
    });
    up_smash.smash_charge_mult = 1.7;

    let mut down_smash = strike(Strike {
        id: "smash_down",
        clip: "smash_down",
        startup_s: 0.16,
        active_s: 0.06,
        recover_s: 0.46,
        offset: (0.0, 15.0),
        half_extents: (34.0, 12.0),
        damage: 13,
        knockback: 132.0,
        knockback_growth: 2.70,
        launch_dir: Some((0.95, -0.50)),
        on_hit: None,
    });
    down_smash.smash_charge_mult = 1.7;

    // ── aerials ──────────────────────────────────────────────────────────────
    let n_air = strike(Strike {
        id: "air_neutral",
        clip: "air_neutral",
        startup_s: 0.04,
        active_s: 0.06,
        recover_s: 0.22,
        offset: (0.0, 0.0),
        half_extents: (23.0, 21.0),
        damage: 5,
        knockback: 66.0,
        knockback_growth: 1.28,
        launch_dir: None,
        on_hit: None,
    });

    let f_air = strike(Strike {
        id: "air_forward",
        clip: "air_forward",
        startup_s: 0.06,
        active_s: 0.05,
        recover_s: 0.24,
        offset: (28.0, -2.0),
        half_extents: (21.0, 17.0),
        damage: 8,
        knockback: 98.0,
        knockback_growth: 1.85,
        launch_dir: Some((1.0, -0.30)),
        on_hit: None,
    });

    let b_air = strike(Strike {
        id: "air_back",
        clip: "air_back",
        startup_s: 0.07,
        active_s: 0.04,
        recover_s: 0.28,
        offset: (-30.0, 0.0),
        half_extents: (21.0, 17.0),
        damage: 11,
        knockback: 128.0,
        knockback_growth: 2.45,
        launch_dir: Some((-1.0, -0.36)),
        on_hit: None,
    });

    let u_air = strike(Strike {
        id: "air_up",
        clip: "air_up",
        startup_s: 0.04,
        active_s: 0.05,
        recover_s: 0.22,
        offset: (2.0, -28.0),
        half_extents: (19.0, 21.0),
        damage: 6,
        knockback: 84.0,
        knockback_growth: 1.75,
        launch_dir: Some((0.0, -1.0)),
        on_hit: None,
    });

    let d_air = strike(Strike {
        id: "air_down",
        clip: "air_down",
        startup_s: 0.08,
        active_s: 0.05,
        recover_s: 0.30,
        offset: (5.0, 26.0),
        half_extents: (19.0, 19.0),
        damage: 10,
        knockback: 118.0,
        knockback_growth: 2.20,
        launch_dir: Some((0.0, 1.0)),
        on_hit: None,
    });

    // special we want for oni and goblin … I think oni has a bunch of sfx and
    // vfx ready for it."* He does: fourteen authored rows on his own FX sheet,
    // and this table had never named ONE of them. Every effect below is his.
    //
    // the axis holds through all five. Fastest to start, shortest active
    // window, recovery of more than three times it — a special that opened
    // slowly or lingered would be a different character wearing the mask.

    // the forward tilt, which fell down the chain to the jab. `missed_
    // answer_cut` is the row for it: the answer that goes through where you were.
    let f_tilt = strike(Strike {
        id: "tilt_forward",
        clip: "attack_side",
        startup_s: 0.04,
        active_s: 0.04,
        recover_s: 0.22,
        offset: (30.0, -2.0),
        half_extents: (20.0, 13.0),
        damage: 5,
        knockback: 66.0,
        knockback_growth: 1.22,
        launch_dir: Some((1.0, -0.28)),
        on_hit: None,
    });
    let f_tilt = vfx_at(f_tilt, 0.04, "missed_answer_cut", (30.0, -2.0), 0.9);
    let f_tilt = sfx(f_tilt, 0.04, "enemy.shadow_oni.slash");
    let f_tilt = on_contact(f_tilt, "player.hit");

    let n_b = strike(Strike {
        id: "shadow_answer",
        clip: "attack",
        startup_s: 0.06,
        active_s: 0.04,
        recover_s: 0.34,
        offset: (26.0, -4.0),
        half_extents: (26.0, 22.0),
        damage: 12,
        knockback: 112.0,
        knockback_growth: 2.05,
        launch_dir: Some((0.9, -0.50)),
        on_hit: None,
    });
    let n_b = committed_tail(n_b, 0.62, 0.0);
    let n_b = vfx_at(n_b, 0.02, "oni_eye_flash", (0.0, -10.0), 0.8);
    let n_b = sfx(n_b, 0.02, "enemy.shadow_oni.alert");
    let n_b = vfx_at(n_b, 0.06, "shadow_answer_slash", (26.0, -4.0), 1.15);
    let n_b = sfx(n_b, 0.06, "enemy.shadow_oni.slash");
    let n_b = on_contact(n_b, "player.hit");

    // SIDE — `iaijutsu`. The draw and the cut are one motion, so the impulse
    // and the active window are the same instant. He crosses the distance
    // already having swung.
    let side_b = strike(Strike {
        id: "iaijutsu",
        clip: "attack_side",
        startup_s: 0.05,
        active_s: 0.05,
        recover_s: 0.30,
        offset: (34.0, 0.0),
        half_extents: (30.0, 16.0),
        damage: 11,
        knockback: 106.0,
        knockback_growth: 1.95,
        launch_dir: Some((0.95, -0.35)),
        on_hit: None,
    });
    let side_b = impulse(side_b, 0.05, (700.0, 0.0), ImpulseMode::Set);
    let side_b = committed_tail(side_b, 0.58, 0.0);
    let side_b = vfx_at(side_b, 0.01, "silent_step", (0.0, 14.0), 0.8);
    let side_b = vfx_at(side_b, 0.05, "iaijutsu_glint", (34.0, 0.0), 1.0);
    let side_b = sfx(side_b, 0.05, "enemy.shadow_oni.slash");
    let side_b = on_contact(side_b, "player.hit");

    // UP — `smoke_fold`. THE RECOVERY, and the reason this batch is not
    // cosmetic: with no special at all he had a double jump and nothing else.
    // He does not climb — he leaves, and arrives. the hit is on the ARRIVAL,
    // not the departure, so covering the spot he left is not a punish.
    let mut up_b = strike(Strike {
        id: "smoke_fold",
        clip: "attack_up",
        startup_s: 0.05,
        active_s: 0.05,
        recover_s: 0.26,
        offset: (0.0, -8.0),
        half_extents: (20.0, 30.0),
        damage: 8,
        knockback: 84.0,
        knockback_growth: 1.60,
        launch_dir: Some((0.15, -1.0)),
        on_hit: None,
    });
    up_b.landing_lag_s = Some(0.30);
    let up_b = impulse(up_b, 0.05, (0.0, -780.0), ImpulseMode::Set);
    let up_b = committed_tail(up_b, 0.52, 0.0);
    let up_b = vfx_at(up_b, 0.0, "blink_depart", (0.0, 0.0), 1.0);
    let up_b = sfx(up_b, 0.0, "enemy.shadow_oni.vanish");
    let up_b = vfx_at(up_b, 0.05, "smoke_fold", (0.0, 4.0), 1.1);
    let up_b = sfx(up_b, 0.05, "faction.ninja.smoke_poof");
    let up_b = vfx_at(up_b, 0.10, "blink_arrive", (0.0, -8.0), 1.0);
    let up_b = on_contact(up_b, "player.hit");

    // DOWN — `command_seal`. *"A leader's hardest order is the one obeyed
    // instantly."* He plants a seal and the ring closes on it: no displacement,
    // no reach, and the longest tail he owns. The order is given; standing there
    // while it is obeyed is the cost.
    // ⭐⭐ AND IT IS NOW ACTUALLY A COUNTER, which is not a redesign — it is the
    // move finally being what its own presentation has always said it is. Every
    // cue below was authored for a counter and kept beneath a plain strike: a
    // `counter_ring` at 0.06s and `faction.ninja.parry_flash` on the same frame.
    // ⇒ The art and the audio announced a parry and the mechanics were a
    // damage-10 poke. Same class as prose describing code that is not there,
    // except the reader is the PLAYER, who was being told the wrong thing about
    // a move every time it came out.
    //
    // ⭐ THE ANSWER IS SMOKE. `smash.sleep` is the Performer's engine and this
    // is its second customer with a completely unrelated fiction: she holds the
    // room with her voice, he drops a smoke seal and you wake up on the floor.
    // ⇒ Distinct from both counters already on the roster — George's answers
    // with a GRAB, the Author's with an ambush TELEPORT — which is the point of
    // the response being an arbitrary technique rather than a fixed reaction.
    //
    // ⛔ SHORT SLEEP, AND THE REASON IS THE GUARANTEE. The Performer earns 1.4s
    // by standing next to somebody while rooted for 0.6s; this is handed over by
    // a successful parry, which is already a full punish. Half her duration is
    // still a free smash and does not read as a stun-lock.
    let down_b = ambition_characters::smash_counter::counter_move(
        "command_seal",
        "attack_down",
        // His original 0.06s tell, kept: *"a leader's hardest order is the one
        // obeyed instantly."*
        0.06,
        // ⭐ THE STANCE IS THE OLD ACTIVE WINDOW, DOUBLED. A 0.05s hitbox is a
        // poke; a 0.05s parry window is unusable — it is three frames at 60Hz
        // and the reads it would demand are not reads, they are guesses.
        0.10,
        0.36,
        ambition_characters::smash_counter::CounterParams {
            // A heartbeat, not a duration: `parry_window_timer` decays and the
            // stance re-arms it every live frame.
            window_s: 0.05,
            response: ambition_characters::smash_sleep::SLEEP.to_string(),
            response_params: ambition_platformer2d::entity_catalog::ParamValue::from_typed(
                &ambition_characters::smash_sleep::SleepParams {
                    duration_s: 0.7,
                    // Tight and centred on the seal: the smoke catches whoever
                    // was close enough to swing at him, which by construction is
                    // whoever he just parried.
                    half_extents: (34.0, 26.0),
                },
            )
            .expect("the seal's sleep params serialize"),
            // ⛔ HE SWALLOWS SHOTS. A ninja who returned them would be reflecting
            // with a smoke bomb, and the roster's reflector is already George's
            // riposte — stated here rather than defaulted so the choice is
            // visible at both ends.
            absorbs_projectiles: true,
        },
    );
    let down_b = committed_tail(down_b, 0.70, 0.0);
    let down_b = vfx_at(down_b, 0.0, "command_seal", (0.0, 10.0), 1.0);
    let down_b = sfx(down_b, 0.0, "enemy.shadow_oni.alert");
    let down_b = vfx_at(down_b, 0.06, "counter_ring", (0.0, 6.0), 1.2);
    let down_b = sfx(down_b, 0.06, "faction.ninja.parry_flash");
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
    // DOWN, IN THE AIR — `falling_seal`. The order is given on the way
    // down. Same seal, no floor to plant it on, so it closes around him as he
    // drops — and he arrives with it.
    let mut air_down_b = strike(Strike {
        id: "falling_seal",
        clip: "air_down",
        startup_s: 0.05,
        active_s: 0.05,
        recover_s: 0.28,
        offset: (0.0, 22.0),
        half_extents: (24.0, 22.0),
        damage: 9,
        knockback: 92.0,
        knockback_growth: 1.70,
        launch_dir: Some((0.0, 1.0)),
        on_hit: None,
    });
    air_down_b.landing_lag_s = Some(0.28);
    let air_down_b = impulse(air_down_b, 0.05, (0.0, 1250.0), ImpulseMode::Set);
    let air_down_b = vfx_at(air_down_b, 0.0, "command_seal", (0.0, 0.0), 0.9);
    // ⛔⛔ ITS PARRY CUES ARE GONE, AND REMOVING THEM IS MY DEBT RATHER THAN A
    // SEPARATE DESIGN CALL. This dive wore `counter_ring` and
    // `faction.ninja.parry_flash` while the GROUNDED seal beside it was also a
    // plain strike — two moves telling the same small lie, which at least told
    // it consistently. ⇒ The moment the grounded one became a real counter, a
    // player learns "ring plus flash means he is parrying" and this move
    // punishes that read: it is a fast-fall SPIKE (`impulse (0, 1250)`,
    // `launch_dir (0, 1)`) with no defensive frame anywhere in it.
    //
    // ⭐ `smoke_fold` AND `smoke_poof` ARE WHAT IT ACTUALLY DOES, and both are
    // already his: the seal closes around him as he drops — which is this move's
    // own comment, three lines up — and he arrives in it. The `command_seal`
    // above is untouched, so the seal imagery the comment claims is still there;
    // what has gone is the claim to be answering an attack.
    let air_down_b = vfx_at(air_down_b, 0.05, "smoke_fold", (0.0, 18.0), 1.0);
    let air_down_b = sfx(air_down_b, 0.05, "faction.ninja.smoke_poof");
    let air_down_b = on_contact(air_down_b, "player.hit");

    // ONI'S CAPTURE KIT. The FASTEST grab in the game and the longest recovery
    // behind it — a read, not a poke. Whiffing this is the punish window his whole kit
    // is balanced around.
    // the grab draws `attack`, not `grab`: these sheets publish no `grab` row,
    // and each table's own `every_clip_names_a_row_..._sheet_carries` guard says
    // so. `ClipBinding`'s fallbacks would have covered it at runtime, but a move
    // that NAMES a row nobody publishes is a lie the guard is right to refuse.
    let grab = author_standing_grab(
        grab_shell("oni_grab", "attack", 0.05, 0.04, 0.26),
        CaptureAttemptParams {
            offset: (12.0, 1.0),
            half_extents: (21.0, 16.0),
            hold_offset: (13.0, 3.0),
        },
    );
    let pummel = author_pummel(
        capture_beat("oni_pummel", "attack", 0.15),
        0.06,
        CapturePummelParams { damage: 3 },
    );
    let forward_throw = author_throw(
        capture_beat("oni_fthrow", "attack", 0.25),
        0.12,
        CaptureThrowParams {
            damage: 9,
            knockback: 126.0,
            knockback_growth: 2.0,
            launch_dir: (0.8, -0.6),
        },
    );

    let back_throw = author_throw(
        capture_beat("oni_bthrow", "attack", 0.27),
        0.13,
        CaptureThrowParams {
            damage: 10,
            knockback: 136.08,
            knockback_growth: 2.1,
            launch_dir: (-1.0, -0.37),
        },
    );

    let up_throw = author_throw(
        capture_beat("oni_uthrow", "attack", 0.26),
        0.12,
        CaptureThrowParams {
            damage: 9,
            knockback: 131.04,
            knockback_growth: 2.04,
            launch_dir: (0.0, -1.0),
        },
    );

    let down_throw = author_throw(
        capture_beat("oni_dthrow", "attack", 0.28),
        0.13,
        CaptureThrowParams {
            damage: 7,
            knockback: 93.24,
            knockback_growth: 1.6,
            launch_dir: (0.32, -0.92),
        },
    );
    SmashRepertoire {
        taunt: ambition_characters::moveset_authoring::taunt("ninja_shadow_oni_leader_taunt", 0.9),
        // 0.30 recovery, not the genre's 0.26 — nothing he swings recovers
        // in under 3x its active window, which is what stops any of it being
        // thrown casually. 0.09 active buys 0.27, and his law wants MORE.
        dash_attack: ambition_characters::moveset_authoring::dash_attack(
            "ninja_shadow_oni_leader_dash_attack",
            ambition_characters::moveset_authoring::DashAttackShape {
                recover_s: 0.30,
                ..ambition_characters::moveset_authoring::DashAttackShape::GENRE
            },
            7,
            82.5,
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
    use ambition_platformer2d::entity_catalog::{MoveSpec, MoveWindow, WindowTag};

    fn find(set: &MovesetContract, id: &str) -> MoveSpec {
        set.moves
            .iter()
            .find(|m| m.id == id)
            .unwrap_or_else(|| panic!("{id} exists"))
            .clone()
    }

    /// only a STRIKE has one. A pummel and a throw are timelines with no
    /// Active window at all, so the swing tests below iterate `strikes()` rather
    /// than every move the contract carries — asking a throw for its startup is
    /// asking about a window it never has.
    fn active(m: &MoveSpec) -> &MoveWindow {
        m.windows
            .iter()
            .find(|w| matches!(w.tag, WindowTag::Active))
            .expect("a strike has an active window")
    }

    /// Every move that actually swings.
    fn strikes(set: &MovesetContract) -> Vec<&MoveSpec> {
        set.moves
            .iter()
            .filter(|m| m.windows.iter().any(|w| matches!(w.tag, WindowTag::Active)))
            .collect()
    }

    fn startup(m: &MoveSpec) -> f32 {
        active(m).start_s
    }

    fn active_len(m: &MoveSpec) -> f32 {
        let w = active(m);
        w.end_s - w.start_s
    }

    fn recovery(m: &MoveSpec) -> f32 {
        m.duration_s - active(m).end_s
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

    /// THE SHADOW ANSWERS: he is quicker to start than the quickest body that
    /// already had a table, and quicker to finish answering than any of them.
    ///
    /// Comparative against the GOBLIN, which is the fast one — measuring against
    /// the admiral or the clerk would make "fast" mean "not a heavyweight" and
    /// prove nothing.
    #[test]
    fn he_answers_faster_and_for_less_time_than_the_goblin() {
        let oni = ninja_shadow_oni_leader_moveset();
        let goblin = crate::goblin_moveset::goblin_moveset();

        assert!(
            startup(&find(&oni, "jab")) < startup(&find(&goblin, "jab")),
            "the shadow answers first"
        );

        let longest = |set: &MovesetContract| {
            strikes(set)
                .into_iter()
                .map(|m| active_len(m))
                .fold(0.0f32, f32::max)
        };
        assert!(
            longest(&oni) < longest(&goblin),
            "and his widest window is still narrower than the goblin's ({} vs {}) \
             — one breath, and you are either in it or you are not",
            longest(&oni),
            longest(&goblin)
        );
    }

    /// THE ORDER OBEYED INSTANTLY CANNOT BE RECALLED: every move recovers for
    /// more than three times its own active window.
    ///
    /// this is the axis, and it is what stops the table being the goblin's
    /// with smaller numbers. A fighter can be given fast startups by typing
    /// smaller floats; a fighter whose every swing costs more than triple what it
    /// buys has a different relationship to committing.
    ///
    /// the poison is that the GOBLIN must fail this, or the ratio is a
    /// property of `strike`'s shape rather than a property of him.
    #[test]
    fn every_swing_costs_more_than_three_times_the_moment_it_buys() {
        let oni = ninja_shadow_oni_leader_moveset();
        // SWINGS only. The claim is about what a swing costs, and a pummel or
        // a throw is not a swing — it holds no window to be three times longer
        // than. the count is the zero floor: a filter that removed everything
        // would satisfy this loop by iterating nothing.
        let swings = strikes(&oni);
        assert!(
            swings.len() >= 16,
            "only {} of his moves swing at all — this is being asserted over a \
             population that shrank",
            swings.len()
        );
        for m in swings {
            assert!(
                recovery(m) > active_len(m) * 3.0,
                "`{}` recovers {}s for an active window of {}s — under 3x, which \
                 is a swing he could throw casually",
                m.id,
                recovery(m),
                active_len(m)
            );
        }

        let goblin = crate::goblin_moveset::goblin_moveset();
        assert!(
            goblin
                .moves
                .iter()
                .any(|m| recovery(m) <= active_len(m) * 3.0),
            "the goblin is supposed to have cheap swings; if every table passes \
             this, the ratio describes `strike` rather than the oni leader"
        );
    }

    /// And he is not simply BETTER. The fast answer is paid for: his kill
    /// move commits longer after the fact than the admiral's does, and the
    /// admiral is the slow one.
    #[test]
    fn his_kill_move_commits_longer_than_the_admirals() {
        let oni = ninja_shadow_oni_leader_moveset();
        let admiral = crate::pirate_admiral_moveset::pirate_admiral_moveset();
        let (o, a) = (find(&oni, "smash_forward"), find(&admiral, "smash_forward"));
        assert!(
            startup(&o) < startup(&a),
            "he starts his finisher first ({} vs {})",
            startup(&o),
            startup(&a)
        );
        assert!(
            recovery(&o) > recovery(&a),
            "and stands in it longer afterwards ({} vs {}) — the reply is instant \
             and the price is paid at the other end",
            recovery(&o),
            recovery(&a)
        );
    }

    /// ⛔⛔ THE SEAL IS A COUNTER, AND ITS OWN ART SAID SO FIRST. `counter_ring`
    /// and `faction.ninja.parry_flash` were authored on this move while it was a
    /// damage-10 poke — the presentation announced a parry the mechanics did not
    /// have, which is the player being told the wrong thing every time it came
    /// out. ⇒ This test holds BOTH halves together: it is a counter now, and it
    /// still wears the cues that always claimed it was one.
    #[test]
    fn the_command_seal_parries_and_keeps_the_cues_that_always_said_so() {
        let set = ninja_shadow_oni_leader_moveset();
        let seal = find(&set, "command_seal");

        let params: ambition_platformer2d::characters::smash_counter::CounterParams = seal
            .windows
            .iter()
            .filter_map(|window| window.sustain_effect.as_ref())
            .find(|effect| {
                effect.key == ambition_platformer2d::characters::smash_counter::COUNTER
            })
            .expect("the seal holds a counter stance")
            .params
            .hydrate()
            .expect("counter params hydrate");

        // ⛔ AND IT NO LONGER POKES. A counter that also swung would put its own
        // strike into the set of things its parry can catch.
        assert!(
            !seal
                .windows
                .iter()
                .flat_map(|w| w.volumes.iter())
                .any(|v| v.damage > 0),
            "the seal still carries a damaging volume, so it is a strike wearing \
             a parry ring"
        );

        // ⭐ THE ANSWER IS SMOKE, not a grab (George's) and not a teleport
        // (the Author's). The response being an arbitrary technique is the whole
        // reason three counters on one roster are three different moves.
        assert_eq!(
            params.response,
            ambition_platformer2d::characters::smash_sleep::SLEEP,
            "the seal must answer with the sleep pulse"
        );
        let sleep: ambition_platformer2d::characters::smash_sleep::SleepParams =
            params.response_params.hydrate().expect("sleep params hydrate");

        // ⛔ SHORTER THAN THE PERFORMER'S, and the comparison is the point rather
        // than the number: she EARNS 1.4s by standing next to somebody while
        // rooted, and this is handed over by a successful parry, which is already
        // a full punish. A guaranteed sleep must not also be the longest one.
        let monologue = crate::performer_moveset::performer_moveset();
        let hers: ambition_platformer2d::characters::smash_sleep::SleepParams = monologue
            .moves
            .iter()
            .find(|m| m.id == "performer_monologue")
            .expect("her neutral special")
            .events
            .iter()
            .find_map(|event| match &event.kind {
                ambition_platformer2d::entity_catalog::MoveEventKind::Effect(effect)
                    if effect.key == ambition_platformer2d::characters::smash_sleep::SLEEP =>
                {
                    effect.params.hydrate().ok()
                }
                _ => None,
            })
            .expect("she sings");
        assert!(
            sleep.duration_s < hers.duration_s,
            "the ninja's guaranteed sleep ({}s) outlasts the Performer's earned \
             one ({}s)",
            sleep.duration_s,
            hers.duration_s,
        );

        // ⭐ AND THE CUES SURVIVED. If the conversion had dropped them, the move
        // would have become a counter that no longer looks like one — trading one
        // half of the mismatch for the other.
        let cues: Vec<&str> = seal
            .events
            .iter()
            .filter_map(|event| match &event.kind {
                ambition_platformer2d::entity_catalog::MoveEventKind::Vfx { effect, .. } => {
                    Some(effect.as_str())
                }
                _ => None,
            })
            .collect();
        assert!(
            cues.contains(&"counter_ring"),
            "the parry ring is gone: {cues:?}"
        );
    }

    /// ⛔⛔ AND THE DIVE MUST NOT WEAR THE PARRY'S CLOTHES. `falling_seal` is a
    /// fast-fall spike with no defensive frame, and it carried `counter_ring`
    /// and `faction.ninja.parry_flash` — harmless while the grounded seal was
    /// also a plain strike, and a trap the moment it became a real counter: the
    /// player learns the cue on one move and is punished for reading it on the
    /// other. ⇒ The cost of fixing a lie is checking who else was telling it.
    #[test]
    fn his_falling_seal_does_not_wear_the_counters_cues() {
        let set = ninja_shadow_oni_leader_moveset();
        let dive = find(&set, "falling_seal");
        let cues: Vec<String> = dive
            .events
            .iter()
            .filter_map(|event| match &event.kind {
                ambition_platformer2d::entity_catalog::MoveEventKind::Vfx { effect, .. } => {
                    Some(effect.clone())
                }
                ambition_platformer2d::entity_catalog::MoveEventKind::Sfx { cue } => {
                    Some(cue.clone())
                }
                _ => None,
            })
            .collect();
        for lie in ["counter_ring", "faction.ninja.parry_flash"] {
            assert!(
                !cues.iter().any(|c| c == lie),
                "the dive still announces `{lie}`, which only the counter does: {cues:?}"
            );
        }
        // ⭐ AND IT IS STILL A DIVE, so this is a cue swap and not a quiet
        // declawing of the move.
        assert!(
            dive.windows
                .iter()
                .flat_map(|w| w.volumes.iter())
                .any(|v| v.damage > 0),
            "the dive lost its hitbox along with its borrowed cues"
        );
    }
}

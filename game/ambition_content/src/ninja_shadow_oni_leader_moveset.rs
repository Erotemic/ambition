//! Shadow Oni Leader moveset.
//!
//! His attack identity is counter-punching: very fast startup, very short active
//! windows, and long recovery. The repertoire varies timing/commitment rather
//! than adding teleport, clone, or smoke mechanics; those belong to abilities or
//! techniques rather than hit definitions.

use ambition_entity_catalog::authoring::Strike;
use ambition_entity_catalog::smash_capture::{
    author_pummel, author_standing_grab, author_throw, capture_beat, grab_shell,
    CaptureAttemptParams, CaptureCues, CapturePummelParams, CaptureThrowParams,
    SmashCaptureRepertoire,
};
use ambition_entity_catalog::smash_repertoire::{
    DownSpecial, NeutralSpecial, SmashRepertoire, UpSpecial,
};
use ambition_entity_catalog::MovesetContract;

use ambition_entity_catalog::authoring::{
    committed_tail, impulse, on_contact, sfx, strike, vfx_at,
};
use ambition_entity_catalog::ImpulseMode;

/// See the module doc. Eleven moves, the genre's standard verb map.
pub fn ninja_shadow_oni_leader_moveset() -> MovesetContract {
    // ── grounded ─────────────────────────────────────────────────────────────
    //
    // The fastest and shortest jab in the game. It beats a goblin's jab; on a
    // whiff he stands still for a fifth of a second.
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
    // The fastest smashes on the grid and the most punishing to miss. Other
    // kill moves are slow to start; his are slow to end. A goblin that sees it
    // coming gets 0.44s to answer.
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
        knockback_growth: 6.00,
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

    // Specials. Every effect below comes from his own FX sheet.
    //
    // The axis holds for all five: fastest to start, shortest active window,
    // recovery more than three times the active window.

    // The forward tilt, so the press does not fall down the chain to the jab.
    // `missed_answer_cut` is its row: the answer that goes through where you
    // were.
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
    // The answer confirms into the draw. `shadow_answer` is his fastest button
    // (0.06s startup, 0.34s recovery). If it lands, he may cancel into
    // `iaijutsu`, whose flow already branches on being blocked.
    //
    // `OnHit`, not `OnBlock`. A block-cancel would let him skip the recovery,
    // and `iaijutsu` already teleports him out when blocked; two escapes would
    // make pressing it free. A hit-confirm rewards the read and leaves whiffs
    // and blocks priced as before.
    //
    // The window opens after the active frames close (0.10s), so the cancel is
    // chosen after the result is known.
    let n_b = ambition_entity_catalog::authoring::cancelable(
        n_b,
        0.10,
        0.30,
        // `special`, not `special_forward`: `trigger_moveset_moves` asks
        // `cancel_names_for(base_verb_of(verb), ..)`, which reduces
        // `special_forward` to `special` before the window is checked. The
        // directional spelling never matches.
        &["special"],
        ambition_entity_catalog::CancelCondition::OnHit,
    );
    let n_b = committed_tail(n_b, 0.62, 0.0);
    let n_b = vfx_at(n_b, 0.02, "oni_eye_flash", (0.0, -10.0), 0.8);
    let n_b = sfx(n_b, 0.02, "enemy.shadow_oni.alert");
    let n_b = vfx_at(n_b, 0.06, "shadow_answer_slash", (26.0, -4.0), 1.15);
    let n_b = sfx(n_b, 0.06, "enemy.shadow_oni.slash");
    let n_b = on_contact(n_b, "player.hit");

    // Side: `iaijutsu`. The draw and the cut are one motion, so the impulse and
    // the active window are the same instant.
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
    // If you block it, he is already gone: a `TechniqueFlow`.
    //
    // A timeline states when, not what happened. A shielded iaijutsu would leave
    // him in front of a guard with 0.30s of recovery. The flow lets him escape
    // only when blocked.
    //
    // It is a read both ways: block it and he escapes, so shielding the dash is
    // not free; do not block it and you take 11 and a launch.
    //
    // Four nodes, all existing vocabulary: wait until the swing touches anything
    // (`Overlapped` is also true of a blocked strike, which is why the wait uses
    // it and the branch does not), branch on a guard, and if blocked, spend his
    // teleport (the same technique his counter answers with).
    let side_b = ambition_entity_catalog::MoveSpec {
        flow: Some(ambition_entity_catalog::TechniqueFlow {
            nodes: vec![
                // 0: wait until the swing touches something. The timeout is past the active
                // window (0.05 + 0.05) and short of the tail: a swing that touched nothing
                // whiffed, and a whiff must be punishable.
                ambition_entity_catalog::FlowNode::Wait {
                    on: ambition_entity_catalog::FlowSignal::Overlapped,
                    timeout_s: 0.16,
                    then: 1,
                    on_timeout: 3,
                },
                // 1: a guard, or a body? A branch, not a second wait: the contact is
                // resolved and cannot change (see `FlowNode::Branch`).
                ambition_entity_catalog::FlowNode::Branch {
                    on: ambition_entity_catalog::FlowSignal::Blocked,
                    then: 2,
                    otherwise: 3,
                },
                // 2: behind them, through the guard he just fed.
                ambition_entity_catalog::FlowNode::Emit {
                    effect: ambition_entity_catalog::EffectRef {
                        key: ambition_entity_catalog::smash_teleport::TELEPORT
                            .to_string(),
                        params: ambition_entity_catalog::ParamValue::from_typed(
                            &ambition_entity_catalog::smash_teleport::TeleportParams {
                                behind_nearest_foe: true,
                                behind_gap: 26.0,
                                // Whoever just blocked him is within his reach.
                                distance: 180.0,
                                // Not a recovery: a ledge grabbing this arrival would turn an escape into
                                // a stall.
                                ledge_assist: 0.0,
                                // Through the shieldstun he is leaving, and no longer.
                                intangible_s: 0.12,
                                depart_vfx: "smoke_fold".to_string(),
                                arrive_vfx: "silent_step".to_string(),
                            },
                        )
                        .expect("the oni's escape teleport params serialize"),
                    },
                    then: 3,
                },
                ambition_entity_catalog::FlowNode::Finish,
            ],
        }),
        ..side_b
    };

    // Up: `smoke_fold`, his recovery. He does not climb; he leaves and arrives.
    // The hit is on the arrival, so covering the spot he left is not a punish.
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

    // Down: `command_seal`. "A leader's hardest order is the one obeyed
    // instantly." He plants a seal: no displacement, no reach, and his longest
    // tail.
    //
    // It is a counter, as its cues always said (`counter_ring` at 0.06s and
    // `faction.ninja.parry_flash` on the same frame).
    //
    // The answer is smoke: `smash.sleep`, the Performer's engine with a different
    // fiction. It differs from the roster's other counters (George's grabs, the
    // Director's teleports).
    //
    // A short sleep, because a successful parry is already a full punish. The
    // Performer earns 1.4s by standing rooted next to someone; half of that is
    // still a free smash and does not read as a stun-lock.
    let down_b = ambition_entity_catalog::smash_counter::counter_move(
        "command_seal",
        "attack_down",
        // His original 0.06s tell, kept.
        0.06,
        // The stance is the old active window, doubled. A 0.05s parry window is
        // three frames at 60Hz: guessing, not reading.
        0.10,
        0.36,
        ambition_entity_catalog::smash_counter::CounterParams {
            // A heartbeat, not a duration: `parry_window_timer` decays, and the stance
            // re-arms it every live frame.
            window_s: 0.05,
            // Its own answer, as every counter but the clerk's is.
            answers_the_attacker: false,
            response: ambition_entity_catalog::smash_sleep::SLEEP.to_string(),
            response_params: ambition_entity_catalog::ParamValue::from_typed(
                &ambition_entity_catalog::smash_sleep::SleepParams {
                    duration_s: 0.7,
                    // Tight and centred on the seal: the smoke catches whoever was close
                    // enough to swing at him, which is whoever he parried.
                    half_extents: (34.0, 26.0),
                },
            )
            .expect("the seal's sleep params serialize"),
            // He absorbs shots. The roster's reflector is George's riposte; stated
            // here so the choice is visible.
            absorbs_projectiles: true,
        },
    );
    let down_b = committed_tail(down_b, 0.70, 0.0);
    let down_b = vfx_at(down_b, 0.0, "command_seal", (0.0, 10.0), 1.0);
    let down_b = sfx(down_b, 0.0, "enemy.shadow_oni.alert");
    let down_b = vfx_at(down_b, 0.06, "counter_ring", (0.0, 6.0), 1.2);
    let down_b = sfx(down_b, 0.06, "faction.ninja.parry_flash");
    let down_b = on_contact(down_b, "player.hit");

    // Down-B has two forms, like Bowser's: a slam in the air, an arc and slam
    // on the ground. Context-dependent specials are acceptable, though most
    // should not be.
    //
    // A special gated to one posture is not answered in the other: the
    // directional chain falls through to the neutral special.
    // `special_air_down` comes before `special_down` in that chain.
    // Down, in the air: `falling_seal`. The seal closes around him as he drops,
    // and he arrives with it.
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
    // No parry cues on the dive. It is a fast-fall spike (`impulse (0, 1250)`,
    // `launch_dir (0, 1)`) with no defensive frame, so `counter_ring` and the
    // parry flash would teach a read that it then punishes. It uses
    // `smoke_fold` and `smoke_poof`: the seal closes around him and he arrives
    // in it.
    let air_down_b = vfx_at(air_down_b, 0.05, "smoke_fold", (0.0, 18.0), 1.0);
    let air_down_b = sfx(air_down_b, 0.05, "faction.ninja.smoke_poof");
    let air_down_b = on_contact(air_down_b, "player.hit");

    // Oni's capture kit: the fastest grab in the game with the longest recovery
    // behind it. A whiff is the punish window his kit is balanced around.
    // The grab draws `attack`, not `grab`: these sheets publish no `grab` row,
    // and each table's clip guard refuses unpublished rows.
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
        taunt: ambition_entity_catalog::authoring::taunt("ninja_shadow_oni_leader_taunt", 0.9),
        // 0.30 recovery, not the usual 0.26: nothing he swings recovers in under 3x
        // its active window. 0.09 active needs more than 0.27.
        dash_attack: ambition_entity_catalog::authoring::dash_attack(
            "ninja_shadow_oni_leader_dash_attack",
            ambition_entity_catalog::authoring::DashAttackShape {
                recover_s: 0.30,
                ..ambition_entity_catalog::authoring::DashAttackShape::GENRE
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
mod answer_tests {

    /// The answer confirms on a hit and not on a block. A block-cancel would let
    /// him skip the recovery, and `iaijutsu` already escapes when blocked. A check
    /// that only found a `Cancelable` window would pass either version.
    #[test]
    fn the_shadow_answer_confirms_on_a_hit_and_not_on_a_block() {
        use ambition_entity_catalog::{CancelCondition, WindowTag};
        let answer = crate::authored_movesets::shipped("npc_ninja_shadow_oni_leader")
            .move_by_id("shadow_answer")
            .expect("shadow_answer exists")
            .clone();
        let active = answer
            .windows
            .iter()
            .find(|w| w.tag == WindowTag::Active && !w.volumes.is_empty())
            .expect("it still cuts");
        let cancel = answer
            .windows
            .iter()
            .find_map(|w| match &w.tag {
                WindowTag::Cancelable { into, condition } => Some((w, into, condition)),
                _ => None,
            })
            .expect("the answer has a second half");
        assert_eq!(
            cancel.2,
            &CancelCondition::OnHit,
            "a block-cancel stacks two escapes and makes the fastest button free"
        );
        // Ask the runtime's question: what the press offers after
        // `base_verb_of`, not the authored list compared with itself.
        let base = ambition_entity_catalog::base_verb_of("special_forward");
        let offered = ambition_entity_catalog::cancel_names_for(base, false);
        assert!(
            cancel.1.iter().any(|verb| offered.contains(&verb.as_str())),
            "it confirms into the DRAW: the window names {:?} and a \
             `special_forward` press offers {offered:?}",
            cancel.1
        );
        assert!(
            cancel.0.start_s >= active.end_s,
            "the window opens once the verdict is in ({} vs {}), not as a buffer \
             held from the press",
            cancel.0.start_s,
            active.end_s,
        );
    }
}

#[cfg(test)]
mod tests {
    /// The oni's flow validates, and this checks the shape, not the node count.
    ///
    /// `TechniqueFlow::problems()` exists because each failure is silent at
    /// runtime (a transition past the end, no reachable `Finish`, a `Wait` that
    /// never times out): a move that does nothing, or a fighter stuck in a
    /// special.
    #[test]
    fn the_onis_iaijutsu_authors_a_flow_that_validates_and_escapes_only_on_block() {
        use ambition_entity_catalog::{FlowNode, FlowSignal};
        let set = crate::authored_movesets::shipped("npc_ninja_shadow_oni_leader");
        let side_b = set
            .moves
            .iter()
            .find(|m| m.id == "iaijutsu")
            .expect("his side-B is in the table");
        let flow = side_b
            .flow
            .as_ref()
            .expect("the iaijutsu authors no flow, so a shielded dash is a free punish again");
        assert!(
            flow.problems().is_empty(),
            "the oni's flow does not validate: {:?}",
            flow.problems()
        );

        // Branch on `Blocked`, not `Overlapped`. The wait uses `Overlapped` because
        // it is also true of a blocked strike; a branch on it would escape on every
        // connect, making the move safe on everything, not just on shield.
        let branch_signal = flow.nodes.iter().find_map(|n| match n {
            FlowNode::Branch { on, .. } => Some(*on),
            _ => None,
        });
        assert_eq!(
            branch_signal,
            Some(FlowSignal::Blocked),
            "the escape branches on {branch_signal:?} — only a BLOCK may buy it, \
             or the dash becomes safe on hit as well"
        );

        // The wait must time out before the move ends: a whiffed dash must be
        // punishable.
        let timeout = flow.nodes.iter().find_map(|n| match n {
            FlowNode::Wait { timeout_s, .. } => Some(*timeout_s),
            _ => None,
        });
        let active_ends = 0.05 + 0.05;
        assert!(
            timeout.is_some_and(|t| t > active_ends && t < side_b.duration_s),
            "the wait times out at {timeout:?}, which is not between the end of \
             the active window ({active_ends}) and the end of the move \
             ({}) — a whiff must not reach the escape",
            side_b.duration_s
        );
    }

    use super::*;
    use ambition_entity_catalog::{MoveSpec, MoveWindow, WindowTag};

    fn find(set: &MovesetContract, id: &str) -> MoveSpec {
        set.moves
            .iter()
            .find(|m| m.id == id)
            .unwrap_or_else(|| panic!("{id} exists"))
            .clone()
    }

    /// Only a strike has one. Pummels and throws have no Active window, so the
    /// swing tests iterate `strikes()`, not every move.
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

    // Verb binding is checked by construction: `SmashRepertoire` owns the verb
    // strings and is a struct with no `Default`, so a missing slot is a compile
    // error. Coverage in every posture is checked by
    // `ambition_entity_catalog::smash_repertoire` and by
    // `smash_roster_movesets::report_the_smash_kit_every_selectable_fighter_has`.

    /// He starts faster than the fastest existing body and finishes answering
    /// sooner.
    ///
    /// Compared against the goblin, the fast one; comparing with the admiral or
    /// the clerk would only show he is not a heavyweight.
    #[test]
    fn he_answers_faster_and_for_less_time_than_the_goblin() {
        let oni = crate::authored_movesets::shipped("npc_ninja_shadow_oni_leader");
        let goblin = crate::authored_movesets::shipped("goblin");

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

    /// Every move recovers for more than three times its active window.
    ///
    /// This is his axis, and what separates him from a goblin with smaller
    /// numbers. The goblin must fail this, or the ratio is a property of
    /// `strike`'s shape, not of him.
    #[test]
    fn every_swing_costs_more_than_three_times_the_moment_it_buys() {
        let oni = crate::authored_movesets::shipped("npc_ninja_shadow_oni_leader");
        // Swings only: a pummel or throw holds no window. The count is the zero
        // floor: a filter that removed everything would pass trivially.
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

        let goblin = crate::authored_movesets::shipped("goblin");
        assert!(
            goblin
                .moves
                .iter()
                .any(|m| recovery(m) <= active_len(m) * 3.0),
            "the goblin is supposed to have cheap swings; if every table passes \
             this, the ratio describes `strike` rather than the oni leader"
        );
    }

    /// He is not simply better: his kill move commits longer than the admiral's,
    /// and the admiral is the slow one.
    #[test]
    fn his_kill_move_commits_longer_than_the_admirals() {
        let oni = crate::authored_movesets::shipped("npc_ninja_shadow_oni_leader");
        let admiral = crate::authored_movesets::shipped("npc_pirate_admiral");
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

    /// The seal is a counter, and it still wears the cues (`counter_ring`,
    /// `faction.ninja.parry_flash`) that always said so.
    #[test]
    fn the_command_seal_parries_and_keeps_the_cues_that_always_said_so() {
        let set = crate::authored_movesets::shipped("npc_ninja_shadow_oni_leader");
        let seal = find(&set, "command_seal");

        let params: ambition_entity_catalog::smash_counter::CounterParams = seal
            .windows
            .iter()
            .filter_map(|window| window.sustain_effect.as_ref())
            .find(|effect| {
                effect.key == ambition_entity_catalog::smash_counter::COUNTER
            })
            .expect("the seal holds a counter stance")
            .params
            .hydrate()
            .expect("counter params hydrate");

        // It no longer strikes: a counter that also swung would put its own strike
        // among the things its parry catches.
        assert!(
            !seal
                .windows
                .iter()
                .flat_map(|w| w.volumes.iter())
                .any(|v| v.damage > 0),
            "the seal still carries a damaging volume, so it is a strike wearing \
             a parry ring"
        );

        // The answer is smoke, not a grab (George's) or a teleport (the
        // Director's): the response is an arbitrary technique.
        assert_eq!(
            params.response,
            ambition_entity_catalog::smash_sleep::SLEEP,
            "the seal must answer with the sleep pulse"
        );
        let sleep: ambition_entity_catalog::smash_sleep::SleepParams =
            params.response_params.hydrate().expect("sleep params hydrate");

        // Shorter than the Performer's: she earns 1.4s by standing rooted next to
        // someone, while this comes from a parry, which is already a full punish.
        let monologue = crate::authored_movesets::shipped("performer");
        let hers: ambition_entity_catalog::smash_sleep::SleepParams = monologue
            .moves
            .iter()
            .find(|m| m.id == "performer_monologue")
            .expect("her neutral special")
            .events
            .iter()
            .find_map(|event| match &event.kind {
                ambition_entity_catalog::MoveEventKind::Effect(effect)
                    if effect.key == ambition_entity_catalog::smash_sleep::SLEEP =>
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

        // The cues survived: without them the counter would no longer look like
        // one.
        let cues: Vec<&str> = seal
            .events
            .iter()
            .filter_map(|event| match &event.kind {
                ambition_entity_catalog::MoveEventKind::Vfx { effect, .. } => {
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

    /// The dive must not wear the parry's cues. `falling_seal` is a fast-fall
    /// spike with no defensive frame; parry cues on it would punish a player for
    /// reading them.
    #[test]
    fn his_falling_seal_does_not_wear_the_counters_cues() {
        let set = crate::authored_movesets::shipped("npc_ninja_shadow_oni_leader");
        let dive = find(&set, "falling_seal");
        let cues: Vec<String> = dive
            .events
            .iter()
            .filter_map(|event| match &event.kind {
                ambition_entity_catalog::MoveEventKind::Vfx { effect, .. } => {
                    Some(effect.clone())
                }
                ambition_entity_catalog::MoveEventKind::Sfx { cue } => {
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
        // It is still a dive: a cue swap, not a nerf.
        assert!(
            dive.windows
                .iter()
                .flat_map(|w| w.volumes.iter())
                .any(|v| v.damage > 0),
            "the dive lost its hitbox along with its borrowed cues"
        );
    }
}

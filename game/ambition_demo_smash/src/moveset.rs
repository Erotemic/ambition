//! Shared authored platform-fighter repertoire for demo fighters that do not
//! provide a character-owned table.
//!
//! The table covers directional ground attacks, smashes, aerials, landing lag,
//! autocancel, charge scaling, and knockback growth through ordinary `MoveSpec`
//! data. Authored `knockback_growth` uses absolute px/s per damage point; values
//! here are chosen to match the stage's base-relative growth policy.

use ambition_entity_catalog::authoring::{
    active_start, cancelable, on_contact, sfx, strike, vfx, Strike,
};
use ambition_entity_catalog::{
    CancelCondition, HitVolume, MoveGates, MoveLoop, MoveSpec, MoveWindow, MovesetContract,
    RecoveryUse, VolumeShape, WindowTag,
};

/// Where the rapid jab's loop jumps back to, and the instant it jumps back
/// from. Named because the pulse windows, the finisher start and the guard
/// must all agree.
pub(crate) const FLURRY_FROM_S: f32 = 0.06;
pub(crate) const FLURRY_TO_S: f32 = 0.20;

/// Ground moves are grounded-only, so an airborne body falls through to its
/// aerials instead of a tilt.
pub(crate) fn grounded_only() -> MoveGates {
    MoveGates {
        // A posture does not know about meters: cost is the move's own
        // statement (see `MoveGates::meter_cost`).
        costs: Vec::new(),
        grounded: Some(true),
        // A grounded attack roots its owner, matching `SmashRepertoire`'s
        // `GROUNDED`, so both authoring roads feel the same.
        roots_steering: true,
        recovery_route: None,
        // Not a recovery: a posture cannot know whether a move is an up-B.
        recovery: RecoveryUse::None,
        // A posture says nothing about being held. A move that refuses to
        // start from a saddle says so itself (`call_the_shark` does).
        forbidden_while_held: false,
        // A posture names no fallback; that is the move's own statement.
        when_refused: None,
    }
}

/// Aerials are airborne-only, so a grounded press never reaches a move whose
/// design is that landing costs you.
pub(crate) fn airborne_only() -> MoveGates {
    MoveGates {
        // A posture does not know about meters: cost is the move's own
        // statement (see `MoveGates::meter_cost`).
        costs: Vec::new(),
        grounded: Some(false),
        // An aerial keeps its drift: air control is the trade for the ground
        // control above.
        roots_steering: false,
        recovery_route: None,
        // Not a recovery: a posture cannot know whether a move is an up-B.
        recovery: RecoveryUse::None,
        // A posture says nothing about being held. A move that refuses to
        // start from a saddle says so itself (`call_the_shark` does).
        forbidden_while_held: false,
        // A posture names no fallback; that is the move's own statement.
        when_refused: None,
    }
}

// ---------------------------------------------------------------------------
// The platform-fighter half. The move-building combinators (`strike`,
// `impulse`, `cancelable`, `committed_tail`, `on_hit`, `active_start`) live in
// `ambition_entity_catalog::authoring`. `Feel` is this game's opinion about how
// a swing is heard and seen.
// ---------------------------------------------------------------------------

/// What a move feels like, as six named roles instead of per-move art.
///
/// Every move picks a role, so a jab and a forward smash look and sound
/// different, and a new move needs no new asset. An SFX cue the bank never
/// rendered is silence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Feel {
    /// The fast, cheap one: a swing sound only, so a smash still stands out.
    Poke,
    Heavy,
    /// Sends them upward: a round burst and a light contact, because a juggle
    /// starts here.
    Launcher,
    /// A signature special: charged, starburst, a solid contact.
    Special,
    /// A recovery activating: its own sound and burst, so players can see the
    /// fighter is not dead yet.
    Recovery,
    /// A committed plunge: the smoke of something arriving fast.
    Dive,
}

pub(crate) fn feel(m: MoveSpec, feel: Feel) -> MoveSpec {
    let at = active_start(&m);
    let (windup_cue, hit_cue, swing_cue, burst) = match feel {
        Feel::Poke => (None, None, "player.slash", None),
        Feel::Heavy => (
            Some("player.attack.charge"),
            Some("player.robot.slash.impact.metal.gong"),
            "player.slash",
            Some("shockwave"),
        ),
        Feel::Launcher => (
            None,
            Some("player.robot.slash.impact.flesh.light"),
            "player.slash",
            Some("burst_round"),
        ),
        // `sonic_boom` lives on `generic_exotic_fx`, so a signature special
        // gets a look the shared explosion sheet does not have.
        Feel::Special => (
            Some("player.attack.charge"),
            Some("world.rock.hit"),
            "player.slash",
            Some("sonic_boom"),
        ),
        Feel::Recovery => (
            Some("player.attack.charge"),
            Some("player.hit"),
            "player.robot.slash.air",
            Some("classic_burst"),
        ),
        Feel::Dive => (
            None,
            Some("player.robot.slash.impact.pogo"),
            "player.robot.slash.air",
            Some("smoke_burst"),
        ),
    };
    let mut m = m;
    if let Some(cue) = windup_cue {
        m = sfx(m, 0.0, cue);
    }
    m = sfx(m, at, swing_cue);
    if let Some(effect) = burst {
        m = vfx(m, at, effect);
    }
    if let Some(cue) = hit_cue {
        m = on_contact(m, cue);
    }
    m
}

/// The rest of the jab string: jab 2 and the rapid jab that finishes it.
///
/// Authored once and pushed into every table that wants the string, including
/// George's own moveset, so there is one authoring site.
pub(crate) fn jab_string_continuations() -> Vec<MoveSpec> {
    // Jab 2: the same beat again, a little harder, and the door to the
    // finisher. Its own move: the chain is a cancel table over ordinary moves.
    let mut jab2 = strike(Strike {
        id: "jab2",
        clip: "attack",
        startup_s: 0.04,
        active_s: 0.06,
        recover_s: 0.16,
        offset: (27.0, 0.0),
        half_extents: (18.0, 14.0),
        damage: 3,
        knockback: 60.0,
        // The stage's declaration in the stage's units: 0.02 of base.
        knockback_growth: 60.0 * crate::SMASH_KNOCKBACK_GROWTH,
        launch_dir: None,
        on_hit: None,
    });
    jab2.gates = grounded_only();
    let jab2 = cancelable(jab2, 0.10, 0.26, &["jab3"], CancelCondition::Always);

    // Jab 3: the rapid jab and its finisher, on one timeline. Holding Attack
    // through the loop keeps the flurry going; letting go (or reaching the
    // maximum) exits into the launcher. `MoveLoop` supports this shape: what
    // the move authors after `to_s` is the finisher.
    //
    // The pulses use fixed knockback (`Some(0.0)`). Growing knockback would
    // carry the victim out of the flurry at high percent. Tuning values: one
    // damage and a 46 px/s hold per pulse, a 0.14s lap, at most 1.2s of loop.
    let mut jab3 = strike(Strike {
        id: "jab3",
        clip: "attack",
        startup_s: 0.06,
        active_s: 0.07,
        recover_s: 0.26,
        offset: (30.0, -2.0),
        half_extents: (22.0, 16.0),
        damage: 5,
        knockback: 105.0,
        knockback_growth: 105.0 * crate::SMASH_KNOCKBACK_GROWTH,
        // Away and slightly up: the jab route ends in space, not a kill.
        launch_dir: Some((1.0, -0.35)),
        on_hit: None,
    });
    jab3.gates = grounded_only();
    {
        // The finisher volume, lifted off its window so the loop can be
        // authored in front of it. Derived, not retyped: the pulse inherits
        // the finisher's presentation tag.
        let finisher = jab3.windows[1]
            .volumes
            .pop()
            .expect("the strike builder authors one volume");
        let pulse = HitVolume {
            shape: VolumeShape::Rect {
                offset: (28.0, 0.0),
                half_extents: (19.0, 14.0),
            },
            damage: 1,
            knockback: 46.0,
            knockback_growth: Some(0.0),
            launch_dir: None,
            ..finisher.clone()
        };
        let active = |start_s: f32, end_s: f32, volume: HitVolume| MoveWindow {
            start_s,
            end_s,
            tag: WindowTag::Active,
            volumes: vec![volume],
            motion_scale: 1.0,
            sustain_effect: None,
        };
        jab3.windows = vec![
            MoveWindow {
                start_s: 0.0,
                end_s: FLURRY_FROM_S,
                tag: WindowTag::Startup,
                volumes: Vec::new(),
                motion_scale: 1.0,
                sustain_effect: None,
            },
            active(FLURRY_FROM_S, 0.10, pulse.clone()),
            active(0.13, 0.17, pulse),
            active(FLURRY_TO_S, 0.27, finisher),
            MoveWindow {
                start_s: 0.27,
                end_s: 0.53,
                tag: WindowTag::Recovery,
                volumes: Vec::new(),
                motion_scale: 1.0,
                sustain_effect: None,
            },
        ];
        jab3.duration_s = 0.53;
        jab3.repeat = Some(MoveLoop {
            from_s: FLURRY_FROM_S,
            to_s: FLURRY_TO_S,
            max_s: 1.2,
        });
    }
    vec![jab2, jab3]
}

/// Where a smash freezes: four frames into its windup, for every move.
///
/// Authored, not derived. The engine fallback `CHARGE_POSE_FRACTION` makes the
/// pose a fraction of the windup, so a slow smash would hold later than a fast
/// one. The hold must be on the first frames of the animation, before the
/// frames with hitboxes.
///
/// It must be inside the leading startup and before the first active window.
/// Every windup here is at least 0.22s. `CatalogError::ChargeHoldOutsideWindup`
/// refuses a pose that is not.
const CHARGE_POSE_AT_S: f32 = 4.0 / 60.0;

/// Shared by this demo's fighters. The moveset rides the character, so
/// giving George a different one only edits his definition.
pub fn fighter_moveset() -> MovesetContract {
    let mut moves = Vec::new();

    // ── grounded ─────────────────────────────────────────────────────────────
    //
    // The jab is fast and safe; that is what makes the smash a decision.
    let mut jab = strike(Strike {
        id: "jab",
        clip: "attack",
        startup_s: 0.05,
        active_s: 0.06,
        recover_s: 0.14,
        offset: (26.0, 0.0),
        half_extents: (18.0, 14.0),
        damage: 3,
        knockback: 55.0,
        knockback_growth: 1.10,
        launch_dir: None,
        on_hit: None,
    });
    jab.gates = grounded_only();
    // The chain: a second press inside the window takes the named successor.
    //
    // A jab string continues on a whiff (`Always`), as in the genre. With
    // `OnHit`, CPUs that whiffed the first jab never chained. Other `OnHit`
    // windows in this file are real combo confirms and stay `OnHit`.
    let jab = cancelable(jab, 0.11, 0.25, &["jab2"], CancelCondition::Always);
    moves.push(jab);

    moves.extend(jab_string_continuations());

    let mut up_tilt = strike(Strike {
            id: "tilt_up",
            clip: "attack",
            startup_s: 0.07,
            active_s: 0.08,
            recover_s: 0.18,
            offset: (10.0, -30.0),
            half_extents: (20.0, 22.0),
            damage: 5,
            knockback: 70.0,
            knockback_growth: 1.40,
            // Straight up: an anti-air that starts a juggle.
            launch_dir:
        Some((0.15, -1.0)),
            on_hit: None,
        });
    up_tilt.gates = grounded_only();
    moves.push(up_tilt);

    let mut down_tilt = strike(Strike {
        id: "tilt_down",
        clip: "attack",
        startup_s: 0.06,
        active_s: 0.06,
        recover_s: 0.16,
        offset: (26.0, 16.0),
        half_extents: (20.0, 10.0),
        damage: 4,
        knockback: 60.0,
        knockback_growth: 1.20,
        // A low poke that pops them up into the juggle.
        launch_dir: Some((0.5, -0.85)),
        on_hit: None,
    });
    down_tilt.gates = grounded_only();
    moves.push(down_tilt);

    // ── the smashes ──────────────────────────────────────────────────────────
    //
    // A forward smash has eighteen frames of startup you cannot take back.
    // It pays with a launch three times the jab's that grows with percent,
    // so at 120% it ends the stock. The charge multiplier rewards holding.
    let mut f_smash = strike(Strike {
            id: "smash_forward",
            clip: "attack",
            startup_s: 0.30,
            active_s: 0.07,
            recover_s: 0.34,
            offset: (40.0, -4.0),
            half_extents: (28.0, 20.0),
            damage: 15,
            knockback: 150.0,
            knockback_growth: 3.00,
            // Slightly upward and away: the classic kill angle.
            launch_dir:
        Some((1.0, -0.42)),
            on_hit: None,
        });
    f_smash.gates = grounded_only();
    // A fully held charge lands 1.7x as hard: `smash_charge_mult` scales damage
    // and knockback by how far the owner got through the leading Startup
    // window. `charge` sets the multiplier and the spec in one call, because
    // the spec alone buys nothing.
    let mut f_smash = ambition_entity_catalog::authoring::charge(
        f_smash,
        ambition_entity_catalog::authoring::Charge {
            hold_at_s: CHARGE_POSE_AT_S,
            max_hold_s: ambition_entity_catalog::SmashChargeSpec::DEFAULT_MAX_HOLD_S,
            stores: false,
            roots: true,
            sustain: ambition_entity_catalog::ChargeSustain::WhileHeld,
            gesture: ambition_entity_catalog::ChargeGesture::Smash,
            multiplier: 1.7,
        },
    );
    // Tip and base. The volume above is the tip, authored first, so a body
    // reached by both takes the tip. This is the base: the same swing at the
    // wrong distance, which hurts but does not kill. The list order is the
    // priority.
    for window in f_smash
        .windows
        .iter_mut()
        .filter(|w| matches!(w.tag, WindowTag::Active))
    {
        let tip = window.volumes[0].clone();
        window.volumes.push(HitVolume {
            shape: VolumeShape::Rect {
                // Inboard of the tip and overlapping it, so a body between
                // the two is reached by both.
                offset: (14.0, -4.0),
                half_extents: (16.0, 20.0),
            },
            damage: 8,
            knockback: 70.0,
            knockback_growth: Some(70.0 * crate::SMASH_KNOCKBACK_GROWTH),
            // Flatter and weaker: a base hit leaves them next to you.
            launch_dir: Some((1.0, -0.15)),
            ..tip
        });
    }
    moves.push(f_smash);

    let mut up_smash = strike(Strike {
        id: "smash_up",
        clip: "attack",
        startup_s: 0.26,
        active_s: 0.08,
        recover_s: 0.32,
        offset: (8.0, -38.0),
        half_extents: (24.0, 30.0),
        damage: 14,
        knockback: 140.0,
        knockback_growth: 2.80,
        launch_dir: Some((0.12, -1.0)),
        on_hit: None,
    });
    up_smash.gates = grounded_only();
    // `charge` sets the multiplier and the spec in one call.
    let up_smash = ambition_entity_catalog::authoring::charge(
        up_smash,
        ambition_entity_catalog::authoring::Charge {
            hold_at_s: CHARGE_POSE_AT_S,
            max_hold_s: ambition_entity_catalog::SmashChargeSpec::DEFAULT_MAX_HOLD_S,
            stores: false,
            roots: true,
            sustain: ambition_entity_catalog::ChargeSustain::WhileHeld,
            gesture: ambition_entity_catalog::ChargeGesture::Smash,
            multiplier: 1.7,
        },
    );
    moves.push(up_smash);

    let mut down_smash = strike(Strike {
        id: "smash_down",
        clip: "attack",
        startup_s: 0.22,
        active_s: 0.08,
        recover_s: 0.30,
        offset: (0.0, 18.0),
        half_extents: (40.0, 14.0),
        damage: 12,
        knockback: 130.0,
        knockback_growth: 2.60,
        // Low and outward: the edge-guarding smash, not a launcher.
        launch_dir: Some((1.0, -0.25)),
        on_hit: None,
    });
    down_smash.gates = grounded_only();
    // `charge` sets the multiplier and the spec in one call.
    let down_smash = ambition_entity_catalog::authoring::charge(
        down_smash,
        ambition_entity_catalog::authoring::Charge {
            hold_at_s: CHARGE_POSE_AT_S,
            max_hold_s: ambition_entity_catalog::SmashChargeSpec::DEFAULT_MAX_HOLD_S,
            stores: false,
            roots: true,
            sustain: ambition_entity_catalog::ChargeSustain::WhileHeld,
            gesture: ambition_entity_catalog::ChargeGesture::Smash,
            multiplier: 1.6,
        },
    );
    moves.push(down_smash);

    // ── aerials ──────────────────────────────────────────────────────────────
    //
    // Landing lag and auto-cancel make an aerial a decision: throw it early
    // in a jump and land clean; throw it late and pay.
    let mut n_air = strike(Strike {
        id: "air_neutral",
        clip: "attack",
        startup_s: 0.06,
        active_s: 0.14,
        recover_s: 0.16,
        offset: (14.0, 0.0),
        half_extents: (26.0, 22.0),
        damage: 6,
        knockback: 75.0,
        knockback_growth: 1.50,
        launch_dir: None,
        on_hit: None,
    });
    n_air.gates = airborne_only();
    n_air.landing_lag_s = Some(0.10);
    n_air.autocancel_after_s = Some(0.26);
    moves.push(n_air);

    let mut f_air = strike(Strike {
        id: "air_forward",
        clip: "attack",
        startup_s: 0.09,
        active_s: 0.08,
        recover_s: 0.22,
        offset: (32.0, -4.0),
        half_extents: (22.0, 18.0),
        damage: 9,
        knockback: 105.0,
        knockback_growth: 2.10,
        launch_dir: Some((1.0, -0.35)),
        on_hit: None,
    });
    f_air.gates = airborne_only();
    f_air.landing_lag_s = Some(0.18);
    f_air.autocancel_after_s = Some(0.30);
    moves.push(f_air);

    let mut b_air = strike(Strike {
        id: "air_back",
        clip: "attack",
        startup_s: 0.10,
        active_s: 0.07,
        recover_s: 0.24,
        offset: (-32.0, -2.0),
        half_extents: (22.0, 18.0),
        damage: 11,
        knockback: 125.0,
        knockback_growth: 2.50,
        // Backwards and slightly up: the strongest aerial, and the one you
        // turn around for.
        launch_dir: Some((-1.0, -0.38)),
        on_hit: None,
    });
    b_air.gates = airborne_only();
    b_air.landing_lag_s = Some(0.20);
    b_air.autocancel_after_s = Some(0.32);
    moves.push(b_air);

    let mut u_air = strike(Strike {
        id: "air_up",
        clip: "attack",
        startup_s: 0.07,
        active_s: 0.09,
        recover_s: 0.20,
        offset: (4.0, -34.0),
        half_extents: (22.0, 24.0),
        damage: 7,
        knockback: 90.0,
        knockback_growth: 1.80,
        launch_dir: Some((0.1, -1.0)),
        on_hit: None,
    });
    u_air.gates = airborne_only();
    u_air.landing_lag_s = Some(0.14);
    u_air.autocancel_after_s = Some(0.28);
    moves.push(u_air);

    let mut d_air = strike(Strike {
            id: "air_down",
            clip: "attack",
            startup_s: 0.12,
            active_s: 0.10,
            recover_s: 0.26,
            offset: (6.0, 30.0),
            half_extents: (20.0, 22.0),
            damage: 10,
            knockback: 110.0,
            knockback_growth: 2.20,
            // Straight down: a spike. Offstage it takes a stock.
            launch_dir:
        Some((0.0, 1.0)),
            on_hit: None,
        });
    d_air.gates = airborne_only();
    // The heaviest lag in the set: a missed spike over the stage costs.
    d_air.landing_lag_s = Some(0.28);
    d_air.autocancel_after_s = Some(0.40);
    moves.push(d_air);

    // A grab, as every fighter in the genre has one. Middleweight numbers:
    // slower than the admiral's `0.07` snatch, faster than George's `0.16`,
    // and its throw sits below a smash and below his.
    let capture = ambition_entity_catalog::smash_capture::SmashCaptureRepertoire {
        cues: ambition_entity_catalog::smash_capture::CaptureCues::GENERIC,
        grab: ambition_entity_catalog::smash_capture::author_standing_grab(
            ambition_entity_catalog::smash_capture::grab_shell(
                "grab", "grab", 0.12, 0.05, 0.24,
            ),
            ambition_entity_catalog::smash_capture::CaptureAttemptParams {
                offset: (20.0, 0.0),
                half_extents: (22.0, 14.0),
                hold_offset: (18.0, -2.0),
            },
        ),
        pummel: ambition_entity_catalog::smash_capture::author_pummel(
            ambition_entity_catalog::smash_capture::capture_beat(
                "pummel", "attack", 0.18,
            ),
            0.09,
            ambition_entity_catalog::smash_capture::CapturePummelParams { damage: 3 },
        ),
        forward_throw: ambition_entity_catalog::smash_capture::author_throw(
            ambition_entity_catalog::smash_capture::capture_beat(
                "throw_forward",
                "attack",
                0.28,
            ),
            0.16,
            ambition_entity_catalog::smash_capture::CaptureThrowParams {
                damage: 9,
                knockback: 120.0,
                knockback_growth: 2.1,
                launch_dir: (0.9, -0.5),
            },
        ),
        back_throw: Some(
            ambition_entity_catalog::smash_capture::author_throw(
                ambition_entity_catalog::smash_capture::capture_beat(
                    "throw_back",
                    "attack",
                    0.3,
                ),
                0.17,
                ambition_entity_catalog::smash_capture::CaptureThrowParams {
                    damage: 10,
                    knockback: 130.0,
                    knockback_growth: 2.21,
                    launch_dir: (-1.0, -0.31),
                },
            ),
        ),
        up_throw: Some(
            ambition_entity_catalog::smash_capture::author_throw(
                ambition_entity_catalog::smash_capture::capture_beat(
                    "throw_up", "attack", 0.29,
                ),
                0.16,
                ambition_entity_catalog::smash_capture::CaptureThrowParams {
                    damage: 9,
                    knockback: 125.0,
                    knockback_growth: 2.14,
                    launch_dir: (0.0, -1.0),
                },
            ),
        ),
        down_throw: Some(
            ambition_entity_catalog::smash_capture::author_throw(
                ambition_entity_catalog::smash_capture::capture_beat(
                    "throw_down",
                    "attack",
                    0.31,
                ),
                0.17,
                ambition_entity_catalog::smash_capture::CaptureThrowParams {
                    damage: 7,
                    knockback: 89.0,
                    knockback_growth: 1.68,
                    launch_dir: (0.36, -0.92),
                },
            ),
        ),
    };
    // Side special: a command grab, built from authoring alone. A capture is a
    // move whose `Active` window sustains `smash.capture_attempt`;
    // `author_standing_grab` attaches that to any `MoveSpec`, and the captor
    // branch of `resolve_combat_action` keys off the capture state, not the
    // move. So it pummels and throws through the same four verbs.
    //
    // Before this the stand-ins had nothing on the special button. Some
    // special presses are still unanswered; see
    // `the_only_presses_this_fighter_cannot_answer_are_specials`.
    //
    // It is the standing grab's committed cousin: startup 0.26 against 0.12
    // (not a panic option), more reach than the standing grab's `20.0`, a
    // long recovery, and it travels.
    let mut command_grab =
        ambition_entity_catalog::smash_capture::author_standing_grab(
            ambition_entity_catalog::smash_capture::grab_shell(
                "lunge_grab",
                "special",
                0.26,
                0.06,
                0.38,
            ),
            ambition_entity_catalog::smash_capture::CaptureAttemptParams {
                // Reaches forward from a lunging body, so the box sits further
                // out and a little taller than the standing grab's.
                offset: (34.0, 0.0),
                half_extents: (28.0, 18.0),
                // The same hold as the standing grab: the follow-up throws are
                // shared.
                hold_offset: (18.0, -2.0),
            },
        );
    // Additive, not `Set`: a grab that deleted your run would make dashing
    // into it worse than walking. (George's side-B erases momentum because
    // that is its identity.)
    command_grab.start_impulse = Some((330.0, 0.0));
    moves.push(command_grab);

    // Down special: `riposte`. It adds no defensive mechanic: the perfect
    // shield already denies a qualifying attack and names the attacker; this
    // move holds that window open and says what to do about it.
    //
    // Its answer is the capture attempt `lunge_grab` uses, landing in
    // `CapturedBy`. The response is a key, so another technique can answer
    // by changing one string.
    //
    // The long recovery (0.44s against a 0.16s stance) is the price, so a
    // whiffed counter loses neutral. The hold matches the other grabs, because
    // the follow-up throws are shared.
    let riposte = ambition_entity_catalog::smash_counter::counter_move(
        "riposte",
        "special",
        0.06,
        0.16,
        0.44,
        ambition_entity_catalog::smash_counter::CounterParams {
            // A heartbeat, not a duration: `parry_window_timer` decays and the
            // stance re-arms it every live frame. Three ticks of slack at 60Hz.
            window_s: 0.05,
            // Its own answer, as for every counter except the clerk's.
            answers_the_attacker: false,
            response: ambition_entity_catalog::smash_capture::CAPTURE_ATTEMPT
                .to_string(),
            response_params: ambition_entity_catalog::ParamValue::from_typed(
                &ambition_entity_catalog::smash_capture::CaptureAttemptParams {
                    // Closer than the lunge: the attacker is already inside
                    // your guard.
                    offset: (24.0, 0.0),
                    half_extents: (24.0, 22.0),
                    hold_offset: (18.0, -2.0),
                },
            )
            .expect("the riposte's capture params serialize"),
            // This counter reflects projectiles; absorbing is a different
            // fighter's stance. Stated, not defaulted, so the choice is visible.
            absorbs_projectiles: false,
        },
    );
    moves.push(riposte);

    // Neutral special: `read_and_seize`, a hit confirm. A `MoveSpec` timeline
    // says when; this move needs "swing, and if that connected, grab;
    // otherwise recover". That is a `TechniqueFlow` emitting
    // `smash.capture_attempt`, as `lunge_grab` and `riposte` do.
    //
    // It waits on `Connected`, not `Overlapped`: a shielded poke sets
    // `overlapped` (the staling fact), so it would grab through a guard.
    //
    // The wait starts with the move, so the 0.22s timeout ends 0.08s after the
    // active window (0.09 → 0.14). The whiff punish is the 0.40s recovery; the
    // timeout only bounds the wait. The wait must outlast `startup + active`,
    // or the grab can never come out (guarded below).
    let mut confirm = strike(Strike {
        id: "read_and_seize",
        clip: "special",
        startup_s: 0.09,
        active_s: 0.05,
        recover_s: 0.40,
        offset: (24.0, -2.0),
        half_extents: (18.0, 20.0),
        // Deliberately weak: the payoff is the grab.
        damage: 2,
        knockback: 40.0,
        knockback_growth: 0.80,
        launch_dir: None,
        on_hit: None,
    });
    confirm.gates = grounded_only();
    confirm.flow = Some(ambition_entity_catalog::TechniqueFlow {
        nodes: vec![
            // 0: wait for the verdict on this swing.
            ambition_entity_catalog::FlowNode::Wait {
                on: ambition_entity_catalog::FlowSignal::Connected,
                timeout_s: 0.22,
                then: 1,
                on_timeout: 2,
            },
            // 1: it landed. Grab, at the standing grab's reach.
            ambition_entity_catalog::FlowNode::Emit {
                effect: ambition_entity_catalog::EffectRef {
                    key: ambition_entity_catalog::smash_capture::CAPTURE_ATTEMPT
                        .to_string(),
                    params: ambition_entity_catalog::ParamValue::from_typed(
                        &ambition_entity_catalog::smash_capture::CaptureAttemptParams {
                            offset: (22.0, 0.0),
                            half_extents: (20.0, 20.0),
                            hold_offset: (18.0, -2.0),
                        },
                    )
                    .expect("the confirm's capture params serialize"),
                },
                then: 2,
            },
            // 2: done either way; the move plays out its recovery.
            ambition_entity_catalog::FlowNode::Finish,
        ],
    });
    // Validated where it is authored: a dangling transition or unreachable
    // `Finish` is silent at runtime.
    assert_eq!(
        confirm
            .flow
            .as_ref()
            .expect("just authored")
            .problems(),
        Vec::<String>::new(),
        "`read_and_seize`'s flow is invalid, so the move would misbehave in a way \
         nothing at runtime would report"
    );
    moves.push(confirm);

    // Up special: `slip_upward`, the recovery. Before it, `special_up_air`
    // fell through to nothing, because the other specials are `grounded_only`.
    //
    // A teleport, not an arc: the technique brings ledge assist, wall clamping
    // and destination resolution. Airborne only: on the ground, up-B falls
    // back to the neutral special.
    let recovery = ambition_entity_catalog::smash_teleport::author_teleport(
        {
            let mut shell = ambition_entity_catalog::authoring::hitless_special(
                "slip_upward",
                "special",
                0.10,
                0.46,
            );
            shell.gates = airborne_only();
            // One use per airtime, helpless after (stated by the repertoire
            // slot below).
            shell.landing_lag_s = Some(0.16);
            shell
        },
        0.10,
        ambition_entity_catalog::smash_teleport::TeleportParams {
            // Aimed, not an ambush: `behind_nearest_foe` would put a
            // recovering fighter next to the edgeguarder.
            behind_nearest_foe: false,
            behind_gap: 0.0,
            // Tuned against the engine's jump arc: worth having, but not a
            // free return from anywhere.
            distance: 250.0,
            // Without ledge assist a recovery that lands a pixel under the lip
            // is a death.
            ledge_assist: 26.0,
            // Brief, and it is the counterplay: the blink cannot be hit, so an
            // edgeguard covers where you arrive.
            intangible_s: 0.18,
            depart_vfx: "rune_circle".to_string(),
            arrive_vfx: "rune_circle".to_string(),
        },
    );
    moves.push(recovery);

    let capture_verbs: Vec<(String, String)> = capture
        .bound()
        .into_iter()
        .map(|(verb, spec)| {
            let binding = (verb.to_string(), spec.id.clone());
            moves.push(spec);
            binding
        })
        .collect();

    let verbs = [
        ("attack", "jab"),
        ("attack_up", "tilt_up"),
        ("attack_down", "tilt_down"),
        ("smash_forward", "smash_forward"),
        ("smash_up", "smash_up"),
        ("smash_down", "smash_down"),
        ("attack_air", "air_neutral"),
        ("attack_air_forward", "air_forward"),
        ("attack_air_back", "air_back"),
        ("attack_air_up", "air_up"),
        ("attack_air_down", "air_down"),
        // The two specials this contract binds; see `lunge_grab` and
        // `riposte` above.
        ("special_forward", "lunge_grab"),
        ("special_down", "riposte"),
        ("special", "read_and_seize"),
        ("special_up", "slip_upward"),
    ]
    .into_iter()
    .map(|(verb, id)| (verb.to_string(), id.to_string()))
    .chain(capture_verbs)
    .collect();

    MovesetContract { verbs, moves }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_entity_catalog::AttackDir;

    /// What a press answers, not what a verb list binds.
    ///
    /// `directional_verb_chain` falls back to the base verb, so a missing
    /// `attack_forward` still answers with the `jab`. This test enumerates
    /// every press instead of inspecting keys.
    #[test]
    fn the_only_presses_this_fighter_cannot_answer_are_specials() {
        use ambition_entity_catalog::AttackDir;
        let set = fighter_moveset();
        let dirs = [
            ("neutral", AttackDir::Neutral),
            ("forward", AttackDir::Forward),
            ("up", AttackDir::Up),
            ("down", AttackDir::Down),
            ("back", AttackDir::Back),
        ];

        let mut silent: Vec<String> = Vec::new();
        for base in ["attack", "smash", "special"] {
            for (dir_name, dir) in dirs {
                for (stance, grounded) in [("ground", true), ("air", false)] {
                    if set.move_for_directional_verb(base, dir, grounded).is_none() {
                        silent.push(format!("{base}_{dir_name}_{stance}"));
                    }
                }
            }
        }

        // Every silent press is a `smash` or a `special`; the `attack` family
        // answers all ten.
        assert!(
            silent.iter().all(|p| !p.starts_with("attack_")),
            "an `attack` press went unanswered: {silent:?} — the base-verb \
             fallback is what makes a jab answer a forward tilt, and if it has \
             stopped, every planning claim about this fighter's reach is stale"
        );

        // Two of the ten special presses are silent: `special_neutral_air`
        // and `special_back_air`. `special_forward` and `special_down` answer
        // in both stances, and the neutral special answers the remaining ground
        // presses through the fallback. The aerial column is left because the
        // neutral special is `grounded_only`.
        let specials: Vec<&String> = silent.iter().filter(|p| p.starts_with("special_")).collect();
        assert_eq!(
            specials.len(),
            2,
            "the special gap changed size: {specials:?}. If a special was \
             AUTHORED this is good news and the number wants updating here and \
             in `awaiting-maintainer-decision.md`; if one was LOST, that is a \
             regression the roster question was about."
        );
        assert!(
            !silent.iter().any(|p| p.starts_with("special_forward")),
            "the command grab stopped answering a forward special: {silent:?}"
        );
    }

    /// Every `(base, direction, stance)` press, and which of them a contract
    /// answers with nothing. Shared so both fighters use one instrument.
    fn silent_presses(
        set: &ambition_entity_catalog::MovesetContract,
    ) -> Vec<String> {
        let dirs = [
            ("neutral", AttackDir::Neutral),
            ("forward", AttackDir::Forward),
            ("up", AttackDir::Up),
            ("down", AttackDir::Down),
            ("back", AttackDir::Back),
        ];
        let mut silent = Vec::new();
        for base in ["attack", "smash", "special"] {
            for (dir_name, dir) in dirs {
                for (stance, grounded) in [("ground", true), ("air", false)] {
                    if set.move_for_directional_verb(base, dir, grounded).is_none() {
                        silent.push(format!("{base}_{dir_name}_{stance}"));
                    }
                }
            }
        }
        silent
    }

    /// The two fighters' silent presses are a subset and a difference.
    ///
    /// Siblings pin each fighter's count (the stand-in above, **15**, and
    /// `the_presses_george_leaves_unanswered_are_the_ones_the_genre_lacks`,
    /// **7**). Two counts do not say one set is inside the other, so this
    /// asserts the relation: George's silent set is a strict subset of the
    /// stand-in's, and the rest is exactly eight `special` presses. The
    /// stand-in is George's genre shape without the special button.
    ///
    /// A ratchet. Authoring a stand-in special fails the second assertion (lower
    /// the number in the same commit). Losing a George special breaks the subset,
    /// which is a regression.
    #[test]
    fn the_stand_in_is_george_s_genre_shape_with_the_special_button_removed() {
        let stand_in = silent_presses(&fighter_moveset());
        let george = silent_presses(&crate::george_booul_moveset::george_booul_moveset());

        let escaped: Vec<&String> = george.iter().filter(|p| !stand_in.contains(p)).collect();
        assert!(
            escaped.is_empty(),
            "a press George cannot answer is one the STAND-IN can: {escaped:?}. That breaks the subset, so the stand-in is no longer George minus the specials and the roster question needs re-deriving rather than re-counting."
        );

        let mut extra: Vec<&String> = stand_in.iter().filter(|p| !george.contains(p)).collect();
        extra.sort();
        assert!(
            extra.iter().all(|p| p.starts_with("special_")),
            "the stand-in's surplus silence is no longer all specials: {extra:?}"
        );
        assert_eq!(
            extra.len(),
            2,
            "the stand-in/George special gap moved to {}: {extra:?}. If a special was AUTHORED on the stand-in this is the good failure — lower the number here in the same commit. If George LOST one, the subset assertion above would have fired first.",
            extra.len()
        );
    }

    /// No grab this demo authors reaches further than 96px.
    ///
    /// Nothing else bounds an authored grab's reach (not the params schema,
    /// `acquire_captures`, or the content pass), so a typo in `half_extents`
    /// could catch across the stage. A ceiling, not an engine clamp: a clamp
    /// would silently truncate a deliberate long reach.
    ///
    /// It covers only this crate's two movesets. `ambition_content`'s fighters
    /// (including tethers) are checked by the guard of the same name in
    /// `ambition_content::authored_movesets`, which holds the tether allowlist.
    ///
    /// The platform is 480px wide, so 96 is a fifth of it.
    #[test]
    fn no_grab_this_demo_authors_reaches_further_than_the_stage_allows() {
        use ambition_entity_catalog::smash_capture::{
            CaptureAttemptParams, CAPTURE_ATTEMPT,
        };

        /// A fifth of the shipped platform's width.
        const MAX_REACH_PX: f32 = 96.0;

        let mut seen = 0usize;
        for (who, set) in [
            ("the stand-in fighter", fighter_moveset()),
            ("George", crate::george_booul_moveset::george_booul_moveset()),
        ] {
            for spec in &set.moves {
                for window in &spec.windows {
                    let Some(effect) = window.sustain_effect.as_ref() else {
                        continue;
                    };
                    if effect.key != CAPTURE_ATTEMPT {
                        continue;
                    }
                    let params: CaptureAttemptParams = effect
                        .params
                        .hydrate()
                        .expect("an authored capture attempt must hydrate");
                    seen += 1;
                    // The far edge of the reach box, along the captor's facing.
                    let reach = params.offset.0.abs() + params.half_extents.0.abs();
                    assert!(
                        reach <= MAX_REACH_PX,
                        "{who}'s `{}` reaches {reach}px (offset {:?} + half {:?}), past the \
                         {MAX_REACH_PX}px ceiling. If this is a deliberate tether, raise \
                         MAX_REACH_PX here in the same commit; if it is a typo, this is the \
                         only thing that would have caught it",
                        spec.id, params.offset, params.half_extents
                    );
                    assert!(
                        params.half_extents.0 > 0.0 && params.half_extents.1 > 0.0,
                        "{who}'s `{}` has a non-positive grab box {:?}, so it can never catch \
                         anybody",
                        spec.id, params.half_extents
                    );
                }
            }
        }

        // Population floor: a census that walked nothing also finds no
        // offender.
        assert!(
            seen >= 3,
            "found only {seen} authored capture attempt(s); the demo has at least three (two \
             stand-in grabs and George's), so this census is measuring nothing rather than \
             passing"
        );
    }

    /// The side special is a real capture, not a strike with the name.
    ///
    /// It asserts the special's own move captures and is a different move from
    /// the standing grab. Either claim alone passes on a wrong binding.
    #[test]
    fn the_side_special_is_a_command_grab_and_not_the_standing_grab_renamed() {
        use ambition_entity_catalog::WindowTag;
        let set = fighter_moveset();

        let special = set
            .move_for_verb("special_forward")
            .expect("the stand-in fighter binds a side special");
        let standing = set
            .move_for_verb("grab")
            .expect("the stand-in fighter binds a standing grab");
        assert_ne!(
            special.id, standing.id,
            "the side special resolves to the STANDING grab, so the special \
             button is an alias and the command grab does not exist"
        );

        // Live during `Active` only. Sustained through startup, it would catch
        // bodies before the lunge commits.
        let live: Vec<&WindowTag> = special
            .windows
            .iter()
            .filter(|w| w.sustain_effect.is_some())
            .map(|w| &w.tag)
            .collect();
        assert_eq!(
            live,
            vec![&WindowTag::Active],
            "the command grab's capture attempt is live on {live:?} — it must be \
             live on exactly the Active window, or it is either a move that \
             cannot catch anybody or one that catches during its own startup"
        );
        assert_eq!(
            special
                .windows
                .iter()
                .find(|w| w.tag == WindowTag::Active)
                .and_then(|w| w.sustain_effect.as_ref())
                .map(|e| e.key.as_str()),
            Some(ambition_entity_catalog::smash_capture::CAPTURE_ATTEMPT),
            "the special's live window sustains some OTHER effect, so it is not \
             a capture at all"
        );

        // It travels: a command grab that closes no distance is worse than
        // the standing grab.
        let (dx, _) = special
            .start_impulse
            .expect("a command grab that does not lunge is a slower standing grab");
        assert!(
            dx > 0.0,
            "the command grab's impulse is {dx}, so it lunges backwards or \
             stands still"
        );

        // The committed grab: with equal startup it would be a strictly better
        // standing grab.
        assert!(
            special.windows.iter().any(|w| w.tag == WindowTag::Active)
                && standing.windows.iter().any(|w| w.tag == WindowTag::Active),
            "one of the two grabs has no active window"
        );
        let first_active = |m: &ambition_entity_catalog::MoveSpec| {
            m.windows
                .iter()
                .find(|w| w.tag == WindowTag::Active)
                .map(|w| w.start_s)
                .expect("checked above")
        };
        assert!(
            first_active(special) > first_active(standing),
            "the command grab goes live at {}s and the standing grab at {}s — \
             a command grab that is not slower is a free upgrade and retires \
             the button it is supposed to complement",
            first_active(special),
            first_active(standing)
        );
    }

    /// Every verb resolves to a move that exists. A verb pointing at a
    /// missing id is a press that silently does nothing.
    #[test]
    fn every_authored_verb_resolves() {
        let set = fighter_moveset();
        for (verb, id) in &set.verbs {
            assert!(
                set.move_by_id(id).is_some(),
                "verb `{verb}` names move `{id}`, which is not in the contract"
            );
        }
    }

    /// A forward smash is not a renamed jab: it commits longer, hurts more,
    /// throws harder, and scales with the victim's damage.
    #[test]
    fn the_forward_smash_is_a_real_smash_and_not_the_jab_renamed() {
        let set = fighter_moveset();
        let jab = set.move_for_verb("attack").expect("a fighter has a jab");
        let smash = set
            .move_for_verb("smash_forward")
            .expect("a fighter has a forward smash");

        let launch = |mv: &MoveSpec| {
            mv.windows
                .iter()
                .flat_map(|w| w.volumes.iter())
                // No authored growth defers to the stage (its fraction of the
                // base), the comparable number. Do not rely on `Option`
                // ordering: `None < Some(_)` would read "states nothing" as
                // "grows least".
                .map(|v| {
                    (
                        v.damage,
                        v.knockback,
                        v.knockback_growth
                            .unwrap_or(v.knockback * crate::SMASH_KNOCKBACK_GROWTH),
                    )
                })
                .next()
                .expect("a strike has a volume")
        };
        let (jab_damage, jab_kb, jab_growth) = launch(jab);
        let (smash_damage, smash_kb, smash_growth) = launch(smash);

        assert!(
            smash.duration_s > jab.duration_s * 2.0,
            "the smash commits {:.2}s against the jab's {:.2}s, which is not a \
             commitment",
            smash.duration_s,
            jab.duration_s
        );
        assert!(smash_damage >= jab_damage * 3);
        assert!(smash_kb >= jab_kb * 2.0);
        assert!(
            smash_growth > jab_growth,
            "the smash does not scale harder with percent than the jab, so a \
             stock never ends on it"
        );
        assert!(
            smash.smash_charge_mult > 1.0,
            "holding the smash pays nothing, so there is no reason to charge it"
        );
        // The payoff is reachable. This roster authors each smash's charge
        // pose (see `CHARGE_POSE_AT_S`); a smash with no policy fires at once
        // and the multiplier is unpayable. This keeps the engine fallback
        // (`CHARGE_POSE_FRACTION`) from becoming the contract.
        assert!(
            smash.smash_charge.is_some(),
            "this smash derives its charge pose from the engine fallback \
             instead of authoring one"
        );
        let policy = smash
            .charge_policy()
            .expect("the smash resolves no charge policy, so it cannot be held");
        assert!(
            policy.hold_at_s > 0.0,
            "the hold sits at the very first instant of the move, so there is \
             no windup to commit to before the charge"
        );
        // Strictly before the first strike: Active membership is
        // `start_s <= t < end_s`, so a hold on that instant would charge with
        // the hitbox out.
        let first_active = smash
            .windows
            .iter()
            .filter(|w| {
                matches!(
                    w.tag,
                    ambition_entity_catalog::WindowTag::Active
                )
            })
            .map(|w| w.start_s)
            .fold(f32::MAX, f32::min);
        assert!(
            policy.hold_at_s < first_active,
            "the charge freezes at {} and this smash goes live at {first_active}",
            policy.hold_at_s
        );
    }

    /// Every authored growth equals the stage's own declaration, in the
    /// stage's units.
    ///
    /// Guards a unit mismatch. A volume's `knockback_growth` is absolute px/s per
    /// point; the ruleset's is a fraction of base. Both are `f32` "growth", and
    /// an authored move outranks the ruleset, so a fraction-shaped number would
    /// grow about 40x slower with nothing failing. A move may differ on purpose,
    /// but by a visible factor, not a unit.
    #[test]
    fn an_authored_growth_is_the_stage_declaration_in_the_stage_units() {
        for mv in &fighter_moveset().moves {
            for volume in mv.windows.iter().flat_map(|w| w.volumes.iter()) {
                // No growth defers to the stage, and fixed knockback
                // (`Some(0.0)`) is deliberate. Only a stated non-zero growth
                // can carry the slip.
                let Some(authored) = volume.knockback_growth.filter(|g| *g > 0.0) else {
                    continue;
                };
                let expected = volume.knockback * crate::SMASH_KNOCKBACK_GROWTH;
                assert!(
                    (authored - expected).abs() < 0.01,
                    "`{}` launches at {} and grows {}/point, but the stage \
                     declares {} of base = {expected}/point. A growth that is \
                     off by a FACTOR is the fraction-vs-absolute unit slip, and \
                     it silently opts this move out of the percent loop",
                    mv.id,
                    volume.knockback,
                    authored,
                    crate::SMASH_KNOCKBACK_GROWTH,
                );
            }
        }
    }

    /// The aerials commit, and the auto-cancel window is real.
    ///
    /// `autocancel_after_s` is ignored unless `landing_lag_s` is authored, so
    /// an aerial with a window and no lag is inert.
    #[test]
    fn every_aerial_authors_both_halves_of_the_landing_rule() {
        let set = fighter_moveset();
        let mut checked = 0;
        for verb in [
            "attack_air",
            "attack_air_forward",
            "attack_air_back",
            "attack_air_up",
            "attack_air_down",
        ] {
            let mv = set.move_for_verb(verb).expect("authored above");
            checked += 1;
            let lag = mv.landing_lag_s.unwrap_or(0.0);
            let cancel = mv
                .autocancel_after_s
                .expect("an aerial with lag and no cancel window can only be paid");
            assert!(lag > 0.0, "{verb} lands free, so it is not a commitment");
            assert!(
                cancel < mv.duration_s,
                "{verb}'s auto-cancel opens at {cancel:.2}s of a {:.2}s move, so it \
                 never opens at all",
                mv.duration_s
            );
        }
        assert_eq!(checked, 5, "the loop did not reach every aerial");
    }

    /// A grounded press cannot reach an aerial, and the reverse. The gates make
    /// one button eleven moves.
    #[test]
    fn the_directional_chain_lands_on_the_right_move_for_the_posture() {
        let set = fighter_moveset();
        assert_eq!(
            set.move_for_directional_verb("attack", AttackDir::Forward, true)
                .map(|mv| mv.id.as_str()),
            Some("jab"),
            "a grounded forward press should fall through to the jab: there is no \
             forward tilt, and the aerial is gated off the ground"
        );
        assert_eq!(
            set.move_for_directional_verb("attack", AttackDir::Forward, false)
                .map(|mv| mv.id.as_str()),
            Some("air_forward"),
        );
        assert_eq!(
            set.move_for_directional_verb("smash", AttackDir::Forward, true)
                .map(|mv| mv.id.as_str()),
            Some("smash_forward"),
        );
        assert_eq!(
            set.move_for_directional_verb("attack", AttackDir::Up, true)
                .map(|mv| mv.id.as_str()),
            Some("tilt_up"),
        );
    }
}

#[cfg(test)]
mod hit_confirm_tests {
    use ambition_entity_catalog::{FlowNode, FlowSignal};

    /// The neutral special confirms: it waits on a connect and answers with a
    /// grab.
    ///
    /// The signal is the assertion. `Overlapped` is set by a shielded poke,
    /// so a confirm on it would grab through a guard.
    #[test]
    fn the_neutral_special_confirms_on_a_connect_and_not_on_a_shield() {
        let set = super::fighter_moveset();
        let id = set
            .verbs
            .get("special")
            .expect("the contract binds a neutral special");
        let spec = set
            .moves
            .iter()
            .find(|m| &m.id == id)
            .expect("the neutral special names a move the contract carries");
        let flow = spec
            .flow
            .as_ref()
            .expect("the neutral special is a hit confirm, so it authors a flow");
        assert_eq!(
            flow.problems(),
            Vec::<String>::new(),
            "the shipped confirm's flow is invalid: {:?}",
            flow.problems()
        );

        let waits: Vec<FlowSignal> = flow
            .nodes
            .iter()
            .filter_map(|n| match n {
                FlowNode::Wait { on, .. } => Some(*on),
                _ => None,
            })
            .collect();
        assert_eq!(
            waits,
            vec![FlowSignal::Connected],
            "the confirm waits on {waits:?}. `Overlapped` is set by a BLOCKED \
             strike, so a confirm on it grabs through a shield"
        );

        // It must also emit something; otherwise it commits to a read and
        // gets nothing.
        let emitted: Vec<&str> = flow
            .nodes
            .iter()
            .filter_map(|n| match n {
                FlowNode::Emit { effect, .. } => Some(effect.key.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(
            emitted,
            vec![ambition_entity_catalog::smash_capture::CAPTURE_ATTEMPT],
            "the confirm's payoff is {emitted:?} rather than the grab"
        );
    }

    /// The confirm's wait outlasts the window it confirms.
    ///
    /// A timeout shorter than `startup + active` means the flow gives up before
    /// the strike can report a connect, so the grab never comes out. The wait
    /// starts at move start.
    #[test]
    fn the_confirms_wait_outlasts_the_window_it_confirms() {
        let set = super::fighter_moveset();
        let id = set.verbs.get("special").expect("a neutral special is bound");
        let spec = set.moves.iter().find(|m| &m.id == id).expect("it names a move");
        let flow = spec.flow.as_ref().expect("the confirm authors a flow");

        let timeout = flow
            .nodes
            .iter()
            .find_map(|n| match n {
                FlowNode::Wait { timeout_s, .. } => Some(*timeout_s),
                _ => None,
            })
            .expect("the confirm waits");
        // The last moment a strike can report a connect: the end of the
        // authored Active window.
        let active_ends = spec
            .windows
            .iter()
            .filter(|w| matches!(w.tag, ambition_entity_catalog::WindowTag::Active))
            .map(|w| w.end_s)
            .fold(0.0_f32, f32::max);
        assert!(
            active_ends > 0.0,
            "the confirm has no Active window, so there is nothing to confirm"
        );
        assert!(
            timeout > active_ends,
            "the confirm gives up at {timeout}s but its strike stays live until \
             {active_ends}s — the flow stops waiting before the hit can report, \
             so the grab never comes out however clean the confirm was"
        );
    }

    /// Every authored flow in this contract is valid.
    ///
    /// A population check, so the next flow authored here is covered. A flow
    /// with a dangling transition is silent at runtime.
    #[test]
    fn every_authored_flow_in_this_contract_is_valid() {
        let set = super::fighter_moveset();
        let mut seen = 0usize;
        for spec in &set.moves {
            if let Some(flow) = spec.flow.as_ref() {
                seen += 1;
                assert_eq!(
                    flow.problems(),
                    Vec::<String>::new(),
                    "`{}` authors an invalid flow",
                    spec.id
                );
            }
        }
        assert!(
            seen >= 1,
            "no move in this contract authors a flow, so this guard is measuring \
             nothing rather than passing"
        );
    }
}

#[cfg(test)]
mod recovery_tests {
    use ambition_entity_catalog::smash_teleport::{TeleportParams, TELEPORT};
    use ambition_entity_catalog::MoveEventKind;

    /// These fighters can recover, and the recovery is aimed.
    ///
    /// Before it, `special_up_air` fell through to nothing, because every
    /// special was `grounded_only`. `behind_nearest_foe` must be false: it
    /// would teleport a recovering fighter next to the edgeguarder.
    #[test]
    fn the_up_special_is_an_aimed_airborne_recovery() {
        let set = super::fighter_moveset();
        let id = set
            .verbs
            .get("special_up")
            .expect("these fighters bind an up-special");
        let spec = set
            .moves
            .iter()
            .find(|m| &m.id == id)
            .expect("the up-special names a move the contract carries");

        assert_eq!(
            spec.gates.grounded,
            Some(false),
            "the recovery is not airborne-only, so it replaces the grounded \
             up-B — which already answers with the neutral special — with a \
             worse move"
        );

        let params: TeleportParams = spec
            .events
            .iter()
            .find_map(|ev| match &ev.kind {
                MoveEventKind::Effect(effect) if effect.key == TELEPORT => {
                    Some(effect.params.hydrate().expect("teleport params hydrate"))
                }
                _ => None,
            })
            .expect("the recovery teleports");
        assert!(
            !params.behind_nearest_foe,
            "the recovery teleports BEHIND THE NEAREST FOE, so a fighter \
             recovering from offstage arrives next to their edgeguard"
        );
        assert!(
            params.ledge_assist > 0.0,
            "the recovery has no ledge assist ({}), which is the whole reason \
             this is a technique rather than an authored impulse — a blink that \
             drops you a pixel under the lip reads as a bug, not a miss",
            params.ledge_assist
        );
        assert!(
            params.distance > 0.0,
            "the recovery covers no distance ({})",
            params.distance
        );
    }
}

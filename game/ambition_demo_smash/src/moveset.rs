//! Shared authored platform-fighter repertoire for demo fighters that do not
//! provide a character-owned table.
//!
//! The table covers directional ground attacks, smashes, aerials, landing lag,
//! autocancel, charge scaling, and knockback growth through ordinary `MoveSpec`
//! data. Authored `knockback_growth` uses absolute px/s per damage point; values
//! here are chosen to match the stage's base-relative growth policy.

use ambition_platformer2d::characters::moveset_authoring::{
    active_start, cancelable, on_contact, sfx, strike, vfx, Strike,
};
use ambition_platformer2d::entity_catalog::{
    CancelCondition, HitVolume, MoveGates, MoveLoop, MoveSpec, MoveWindow, MovesetContract,
    RecoveryUse, VolumeShape, WindowTag,
};

/// Where the rapid jab's loop jumps back TO, and the instant it jumps back
/// FROM. Named because four places have to agree — the two pulse windows live
/// inside the stretch, the finisher starts at its end, and the guard reads both.
pub(crate) const FLURRY_FROM_S: f32 = 0.06;
pub(crate) const FLURRY_TO_S: f32 = 0.20;

/// Ground moves are grounded-only so an airborne body falls THROUGH them to its
/// aerials rather than throwing a tilt in mid-air.
pub(crate) fn grounded_only() -> MoveGates {
    MoveGates {
        grounded: Some(true),
        // A GROUNDED ATTACK ROOTS ITS OWNER — the same statement
        // `SmashRepertoire`'s own `GROUNDED` makes, because these two constants
        // describe one posture and a fighter authored through this file must
        // not feel different from one authored through the repertoire.
        roots_steering: true,
        recovery_route: None,
        // Not a recovery: these helpers describe a POSTURE, and a posture
        // cannot know whether a move is somebody's up-B.
        recovery: RecoveryUse::None,
        // A posture says nothing about being HELD. Whether a move refuses to
        // start from a saddle is that move's own statement -- `call_the_shark`
        // makes it -- and a stance default answering for every move would be
        // this file deciding a question it cannot see.
        forbidden_while_held: false,
    }
}

// The two helpers that remain have callers in this file and only in this file.

/// Aerials are airborne-only for the mirror reason: a grounded press must not
/// reach a move whose whole design is that landing costs you.
pub(crate) fn airborne_only() -> MoveGates {
    MoveGates {
        grounded: Some(false),
        // An aerial keeps its drift: air control is the trade for the ground
        // control above.
        roots_steering: false,
        recovery_route: None,
        // Not a recovery: these helpers describe a POSTURE, and a posture
        // cannot know whether a move is somebody's up-B.
        recovery: RecoveryUse::None,
        // A posture says nothing about being HELD. Whether a move refuses to
        // start from a saddle is that move's own statement -- `call_the_shark`
        // makes it -- and a stance default answering for every move would be
        // this file deciding a question it cannot see.
        forbidden_while_held: false,
    }
}

// ---------------------------------------------------------------------------
// What is left here is the PLATFORM-FIGHTER half. The move-building
// combinators — `strike`, `impulse`, `cancelable`, `committed_tail`, `on_hit`,
// `active_start` — live in `moveset_authoring`, where every other character's
// table already reaches for them. `Feel` is the half that does not travel: it
// is this game's opinion about how a swing is heard and seen.
// ---------------------------------------------------------------------------

/// WHAT A MOVE FEELS LIKE, as six named classes rather than per-move art.
///
/// the brief this answers is *"differentiate feedback for normal strike,
/// heavy strike, launcher, special, recovery activation, impactful hit"* — six
/// kinds, not one asset per move. So the vocabulary is the ROLE, and every move
/// in every table picks one; a jab and a forward smash are heard and seen apart
/// because they claim different roles, and adding a move costs no new asset.
///
/// An SFX cue the bank never rendered is silence: safe, but silent. The list below is short because
/// six ROLES is the vocabulary, not because the art is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Feel {
    /// The fast, cheap one. A swing sound and nothing else — a jab that flashed
    /// would make the smash below it look like nothing.
    Poke,
    Heavy,
    /// Sends them upward. A round burst and a light contact — the juggle starts
    /// here, so it must read as a beginning rather than an ending.
    Launcher,
    /// A signature special: charged, starburst, a solid contact.
    Special,
    /// A RECOVERY ACTIVATING. Its own sound and its own burst, because
    /// seeing one is how you know a fighter is not dead yet.
    Recovery,
    /// A committed plunge — the smoke of something arriving fast.
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
        // `sonic_boom` lives on `generic_exotic_fx`, one of the eleven sheets
        // no app could reach until the engine started shipping its own effect
        // art. A signature special is exactly the move that should get a look
        // the shared explosion sheet does not have.
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

/// The fighter repertoire, as one authored contract.
///
/// THE REST OF THE JAB STRING — jab 2 and the rapid jab that finishes it.
///
/// ⛔⛔ AUTHORED ONCE AND PUSHED INTO EVERY TABLE THAT WANTS THE STRING. The
/// chain shipped 2026-08-23 onto the shared table alone, and the demo's headline
/// fighter carries his OWN moveset — so George had no `jab2` and no `jab3` at
/// all, and a census of a George mirror could only ever have counted zero. That
/// is the same trap a sweetspot example fell into a day earlier; the answer is
/// one authoring site rather than a second copy to keep in step.
pub(crate) fn jab_string_continuations() -> Vec<MoveSpec> {
    // JAB 2 — the same beat again, a little harder, and the door to the
    // finisher. Authored as its own move because it IS one: the chain is a
    // cancel table over ordinary moves, not a mode some move enters.
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
        // The stage's own declaration in the stage's units: 0.02 of base.
        knockback_growth: 60.0 * crate::SMASH_KNOCKBACK_GROWTH,
        launch_dir: None,
        on_hit: None,
    });
    jab2.gates = grounded_only();
    let jab2 = cancelable(jab2, 0.10, 0.26, &["jab3"], CancelCondition::Always);

    // JAB 3 — THE RAPID JAB, and the finisher it exits into, on ONE timeline.
    //
    // Holding Attack through the loop stretch keeps the flurry going; letting go
    // (or reaching the authored maximum) drops the body into the launcher that
    // ends the route. That is the genre's third jab, and `MoveLoop`'s own doc
    // describes exactly this shape — "what the move authors after `to_s` is the
    // finisher the loop exits into" — so it is one move rather than a bespoke
    // chain of two.
    //
    // ⭐ THE PULSES ARE FIXED KNOCKBACK, and that is the mechanic the flurry is
    // built on rather than a taste call: a repeating hit whose launch GREW with
    // the victim's percent would carry them further on every lap and throw them
    // clear of the flurry exactly when the flurry matters, so the string would
    // dissolve at the damage where a player is counting on it. `Some(0.0)` says
    // the same small push at 0% and at 200%. It could not be authored before
    // 2026-08-23 — a bare `0.0` meant "the stage decides".
    //
    // ⚠ the numbers below are a KNOB, not a measurement: one damage and a
    // 46 px/s hold per pulse, over a 0.14s lap, bounded at 1.2s of looped time
    // so a held button is a commitment and not a stall.
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
        // Away and slightly up: the jab route ends in SPACE, which is what the
        // three-hit commitment buys — not a kill.
        launch_dir: Some((1.0, -0.35)),
        on_hit: None,
    });
    jab3.gates = grounded_only();
    {
        // The finisher volume the builder just made, lifted off its window so
        // the loop can be authored in front of it. ⛔ derived, never retyped:
        // the pulse inherits the finisher's presentation tag, so the flurry and
        // the launcher can never draw from two different arcs.
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

/// Where a smash freezes: FOUR FRAMES into its windup, and the same four frames
/// whatever the move's own startup is.
///
/// ⭐⭐ AUTHORED, not derived. `CHARGE_POSE_FRACTION` is the engine's fallback
/// for a move that says nothing, and it makes the pose a FRACTION of the
/// windup — so a slow smash would hold later in real time than a fast one, for
/// no reason a player could see. A charge pose is an ANIMATION fact: the swing
/// starts, and a few frames in it stops. Jon, 2026-08-23: *"it needs to hold on
/// the first frames of the smash animation, before letting the rest of the
/// animation, which actually has the hitboxes, play."*
///
/// ⛔ INSIDE THE LEADING STARTUP AND STRICTLY BEFORE THE FIRST ACTIVE WINDOW —
/// every windup in this roster is at least 0.22s, so four frames clears it with
/// room. `CatalogError::ChargeHoldOutsideWindup` refuses an authored pose that
/// does not, which is the check this authoring exists to satisfy rather than
/// to lean on.
const CHARGE_POSE_AT_S: f32 = 4.0 / 60.0;

/// Shared by this demo's three fighters today. That is a content decision, not
/// an architectural one: the moveset rides the CHARACTER, so giving George a
/// heavier one is editing his definition and nothing else.
pub fn fighter_moveset() -> MovesetContract {
    let mut moves = Vec::new();

    // ── grounded ─────────────────────────────────────────────────────────────
    //
    // The jab is the fast, safe, boring one — it exists to be thrown at nothing
    // and get away with it, which is what makes the smash below a decision.
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
    // ⭐ THE CHAIN. A second press inside the window takes the successor the
    // window NAMES; without one, a re-press restarted jab 1 and the fastest
    // move in the kit was a full commitment every time you threw it.
    //
    // ⭐ A JAB STRING CONTINUES ON A WHIFF, which is the genre's rule and was
    // measured to be the difference between a live mechanic and a dead one. The
    // window was authored `OnHit` on the argument that a chain is a REWARD —
    // true of a smash's combo-confirm and false of a jab, which every game in
    // this genre lets you throw three times at empty air. Measured 2026-08-23
    // over a 90-second George mirror: the CPUs started the jab once and it did
    // not connect, so `OnHit` refused the only chance the chain ever had.
    // ⛔ this is the STRING's rule and not the cancel table's: every other
    // `OnHit` window in this file is a genuine combo confirm and stays one.
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
            // Straight up: an anti-air that starts a juggle rather than sending the
            launch_dir:
        // opponent away.
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
    // the move the demo did not have. A forward smash is eighteen frames
    // of startup you cannot take back, and the reason anybody accepts that is
    // the launch at the end of it: three times the jab's, growing with the
    // victim's percent, so at 120% it is the thing that ends the stock. The
    // charge multiplier is what a HELD press pays for.
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
            // Slightly upward and away: the classic kill angle. A contact-derived
            launch_dir:
        // direction would send a crouching opponent along the floor instead.
        Some((1.0, -0.42)),
            on_hit: None,
        });
    f_smash.gates = grounded_only();
    // A fully-held charge lands 1.7× as hard. `smash_charge_mult` scales damage
    // AND knockback by how far the owner's clock got through the leading
    // Startup window before release, so the commitment and the payoff are the
    // same authored number.
    f_smash.smash_charge_mult = 1.7;
    f_smash.smash_charge = Some(ambition_platformer2d::entity_catalog::SmashChargeSpec {
        hold_at_s: CHARGE_POSE_AT_S,
        max_hold_s: ambition_platformer2d::entity_catalog::SmashChargeSpec::DEFAULT_MAX_HOLD_S,
        stores: false,
        roots: true,
        sustain: ambition_platformer2d::entity_catalog::ChargeSustain::WhileHeld,
    });
    // ⭐ THE TIP AND THE BASE. The volume above is the TIP — authored first, so
    // it is the one a body reached by both takes. This is the base: the same
    // swing landed at the wrong distance, which hurts and does not kill.
    //
    // Spacing becomes a skill on this move without the move becoming two: get
    // the range right and the smash is what the smash is worth, stand too close
    // and it is a tilt with a long recovery. ⛔ the order in this list IS the
    // priority — writing the base first would make every forward smash a base
    // hit, and nothing would warn you.
    for window in f_smash
        .windows
        .iter_mut()
        .filter(|w| matches!(w.tag, WindowTag::Active))
    {
        let tip = window.volumes[0].clone();
        window.volumes.push(HitVolume {
            shape: VolumeShape::Rect {
                // Inboard of the tip and overlapping it, so a body between the
                // two is genuinely reached by both and the rule has to choose.
                offset: (14.0, -4.0),
                half_extents: (16.0, 20.0),
            },
            damage: 8,
            knockback: 70.0,
            knockback_growth: Some(70.0 * crate::SMASH_KNOCKBACK_GROWTH),
            // Flatter and weaker: a base hit puts them next to you, not away.
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
    up_smash.smash_charge_mult = 1.7;
    up_smash.smash_charge = Some(ambition_platformer2d::entity_catalog::SmashChargeSpec {
        hold_at_s: CHARGE_POSE_AT_S,
        max_hold_s: ambition_platformer2d::entity_catalog::SmashChargeSpec::DEFAULT_MAX_HOLD_S,
        stores: false,
        roots: true,
        sustain: ambition_platformer2d::entity_catalog::ChargeSustain::WhileHeld,
    });
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
        // Low and outward — the edge-guarding smash, not a launcher.
        launch_dir: Some((1.0, -0.25)),
        on_hit: None,
    });
    down_smash.gates = grounded_only();
    down_smash.smash_charge_mult = 1.6;
    down_smash.smash_charge = Some(ambition_platformer2d::entity_catalog::SmashChargeSpec {
        hold_at_s: CHARGE_POSE_AT_S,
        max_hold_s: ambition_platformer2d::entity_catalog::SmashChargeSpec::DEFAULT_MAX_HOLD_S,
        stores: false,
        roots: true,
        sustain: ambition_platformer2d::entity_catalog::ChargeSustain::WhileHeld,
    });
    moves.push(down_smash);

    // ── aerials ──────────────────────────────────────────────────────────────
    //
    // landing lag and auto-cancel are what make an aerial a DECISION, and
    // both were engine features with no adopter. The pair reads: throw this one
    // early in a jump and land clean; throw it late and pay for it.
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
        // have to turn around for.
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
            // Straight DOWN — a spike. Offstage this is a stock; onstage it is a
            launch_dir:
        // bounce the opponent has to deal with.
        Some((0.0, 1.0)),
            on_hit: None,
        });
    d_air.gates = airborne_only();
    // The heaviest lag in the set: a missed spike over the stage should hurt.
    d_air.landing_lag_s = Some(0.28);
    d_air.autocancel_after_s = Some(0.40);
    moves.push(d_air);

    // A GRAB, BECAUSE EVERY FIGHTER IN THE GENRE HAS ONE.
    //
    // Only George authored one, and the default seats are the STAND-INS. A rock-paper- scissors
    // triangle with one leg on one character is not the game.
    //
    // a middleweight's numbers, deliberately between the two fighters that
    // already author one: slower than the admiral's `0.07` snatch, faster than
    // George's `0.16` commitment, and its throw sits below both a smash and his.
    let capture = ambition_platformer2d::characters::smash_capture::SmashCaptureRepertoire {
        cues: ambition_platformer2d::characters::smash_capture::CaptureCues::GENERIC,
        grab: ambition_platformer2d::characters::smash_capture::author_standing_grab(
            ambition_platformer2d::characters::smash_capture::grab_shell(
                "grab", "grab", 0.12, 0.05, 0.24,
            ),
            ambition_platformer2d::characters::smash_capture::CaptureAttemptParams {
                offset: (20.0, 0.0),
                half_extents: (22.0, 14.0),
                hold_offset: (18.0, -2.0),
            },
        ),
        pummel: ambition_platformer2d::characters::smash_capture::author_pummel(
            ambition_platformer2d::characters::smash_capture::capture_beat(
                "pummel", "attack", 0.18,
            ),
            0.09,
            ambition_platformer2d::characters::smash_capture::CapturePummelParams { damage: 3 },
        ),
        forward_throw: ambition_platformer2d::characters::smash_capture::author_throw(
            ambition_platformer2d::characters::smash_capture::capture_beat(
                "throw_forward",
                "attack",
                0.28,
            ),
            0.16,
            ambition_platformer2d::characters::smash_capture::CaptureThrowParams {
                damage: 9,
                knockback: 120.0,
                knockback_growth: 2.1,
                launch_dir: (0.9, -0.5),
            },
        ),
        back_throw: Some(
            ambition_platformer2d::characters::smash_capture::author_throw(
                ambition_platformer2d::characters::smash_capture::capture_beat(
                    "throw_back",
                    "attack",
                    0.3,
                ),
                0.17,
                ambition_platformer2d::characters::smash_capture::CaptureThrowParams {
                    damage: 10,
                    knockback: 130.0,
                    knockback_growth: 2.21,
                    launch_dir: (-1.0, -0.31),
                },
            ),
        ),
        up_throw: Some(
            ambition_platformer2d::characters::smash_capture::author_throw(
                ambition_platformer2d::characters::smash_capture::capture_beat(
                    "throw_up", "attack", 0.29,
                ),
                0.16,
                ambition_platformer2d::characters::smash_capture::CaptureThrowParams {
                    damage: 9,
                    knockback: 125.0,
                    knockback_growth: 2.14,
                    launch_dir: (0.0, -1.0),
                },
            ),
        ),
        down_throw: Some(
            ambition_platformer2d::characters::smash_capture::author_throw(
                ambition_platformer2d::characters::smash_capture::capture_beat(
                    "throw_down",
                    "attack",
                    0.31,
                ),
                0.17,
                ambition_platformer2d::characters::smash_capture::CaptureThrowParams {
                    damage: 7,
                    knockback: 89.0,
                    knockback_growth: 1.68,
                    launch_dir: (0.36, -0.92),
                },
            ),
        ),
    };
    // A COMMAND GRAB ON SIDE-SPECIAL — the parity inventory's P11 road that
    // needed authoring and nothing else.
    //
    // ⭐ **THE WHOLE POINT IS THAT NO ENGINE WORK WAS REQUIRED.** A capture is a
    // move whose `Active` window sustains `smash.capture_attempt`;
    // `author_standing_grab` attaches that to any `MoveSpec` and never asks
    // which verb the move is bound to, and the captor branch of
    // `resolve_combat_action` keys off the capture STATE — "am I holding
    // somebody" — not off the move that caught them. ⇒ So a grab reached by the
    // special button pummels and throws through exactly the same four verbs the
    // standing grab does, with no new road, no new key and no schema change.
    //
    // ⚠ **AND IT CLOSES A DEAD BUTTON, measured rather than assumed.** This
    // contract bound 18 verbs to George's 26: no `special`, no `special_forward`,
    // `special_up` or `special_down`, no `attack_forward`, no `attack_dash`, no
    // `taunt`. George is the one authored fighter and the stand-ins had NOTHING
    // on the special button — a press that resolved to no move at all. ⛔ This
    // fixes ONE of those eight. The other seven are still dead and that is not
    // this move's job to hide.
    //
    // The design follows the same middleweight logic as the grab above: it is
    // the standing grab's slower, longer-reaching, COMMITTED cousin. Startup
    // 0.26 against the standing grab's 0.12 — you cannot mash it as a panic
    // option — a reach that extends well past the standing grab's `20.0`, and a
    // recovery long enough that a whiff is punished. It travels, because a
    // command grab that closes no distance is a worse standing grab.
    let mut command_grab =
        ambition_platformer2d::characters::smash_capture::author_standing_grab(
            ambition_platformer2d::characters::smash_capture::grab_shell(
                "lunge_grab",
                "special",
                0.26,
                0.06,
                0.38,
            ),
            ambition_platformer2d::characters::smash_capture::CaptureAttemptParams {
                // Reaches forward from a lunging body, so the box sits further
                // out and is a little taller than the standing grab's — it has
                // to catch somebody the lunge is arriving at.
                offset: (34.0, 0.0),
                half_extents: (28.0, 18.0),
                // The SAME hold as the standing grab, deliberately. A captive
                // held somewhere else would make the follow-up throws read
                // differently depending on which grab caught them, and the
                // throws are shared.
                hold_offset: (18.0, -2.0),
            },
        );
    // ⛔ ADDITIVE, not `Set`. George's side-B erases the momentum behind it
    // because being unsteerable is that move's whole identity; this one is a
    // grab, and a grab that deleted your run would make dashing into it worse
    // than walking. A dash-cancelled command grab covering more ground is the
    // correct behaviour and it is what `start_impulse` already means.
    command_grab.start_impulse = Some((330.0, 0.0));
    moves.push(command_grab);

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
        // ⭐ The only special this contract binds. See `lunge_grab` above for
        // why it is a grab and why the other seven of George's verbs stay dead.
        ("special_forward", "lunge_grab"),
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
    use ambition_platformer2d::entity_catalog::AttackDir;

    /// ⭐⭐ WHAT A PRESS ANSWERS, not what a verb list binds — the two are
    /// different and the difference is eight presses.
    ///
    /// ⛔ **A VERB COUNT IS A COUNT OF BINDINGS. A PLAYER PRESSES A CHAIN.**
    /// `directional_verb_chain` always falls back to the base verb, so this
    /// contract's missing `attack_forward` is NOT silence — a forward tilt
    /// answers with the `jab`. ⇒ Counting bindings therefore understates what
    /// this fighter answers, and only the press enumeration below is safe to
    /// draw a claim from.
    ///
    /// ⭐ What IS true is narrower and entirely about the special family. This
    /// pins it by enumerating every press rather than by inspecting keys, so the
    /// claim and the guard are the same act.
    #[test]
    fn the_only_presses_this_fighter_cannot_answer_are_specials() {
        use ambition_platformer2d::entity_catalog::AttackDir;
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

        // ⛔ EVERY silent press is a `smash` or a `special`, and the `attack`
        // family answers ALL TEN — which is the assertion the withdrawn claim
        // would have failed.
        assert!(
            silent.iter().all(|p| !p.starts_with("attack_")),
            "an `attack` press went unanswered: {silent:?} — the base-verb \
             fallback is what makes a jab answer a forward tilt, and if it has \
             stopped, every planning claim about this fighter's reach is stale"
        );

        // ⭐ The special family is the gap, and it is EIGHT of the ten special
        // presses — only `special_forward` answers, grounded and aerial, and it
        // does so because of `lunge_grab`. (Five directions x two stances = ten;
        // the comment said seven before I counted them.)
        let specials: Vec<&String> = silent.iter().filter(|p| p.starts_with("special_")).collect();
        assert_eq!(
            specials.len(),
            8,
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
    /// answers with nothing. Shared by the relation test below so the two
    /// fighters are measured by one instrument rather than two copies of one.
    fn silent_presses(
        set: &ambition_platformer2d::entity_catalog::MovesetContract,
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

    /// ⭐⭐ **THE TWO FIGHTERS' SILENCES ARE NOT TWO NUMBERS, THEY ARE A SUBSET
    /// AND A DIFFERENCE — which is the skeleton finding in one assertion.**
    ///
    /// Its siblings each pin one fighter against the press space: the test above
    /// for the stand-in (**15** silent), and
    /// `the_presses_george_leaves_unanswered_are_the_ones_the_genre_lacks` for
    /// George (**7**). ⛔ **Held apart, those two numbers invite a subtraction
    /// they do not license.** 15 and 7 yield "a gap of 8" only if the smaller set
    /// sits inside the larger one, and nothing in either test says it does — two
    /// sets of those sizes can overlap by any amount, and the difference would
    /// still be reported as 8 in every case.
    ///
    /// ⇒ So assert the RELATION. George's silent set is a strict SUBSET of the
    /// stand-in's, and what is left over is exactly eight `special` presses. ⭐
    /// The stand-in is therefore not a different fighter missing different
    /// things — **it is George's genre shape with the special button removed**,
    /// which is what makes *"give the Robots a special"* a well-posed question
    /// with a bounded answer instead of an open authoring job.
    ///
    /// ⚠ **A ratchet, and the two ways it can redden mean opposite things.**
    /// Authoring a special on the stand-in shrinks the difference and fails the
    /// second assertion — the good failure, fixed by lowering the number here in
    /// the same commit. Losing one of George's specials breaks the SUBSET
    /// instead, and that is a regression. Which assertion fires tells you which
    /// happened, so the failure text does not have to guess.
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
            8,
            "the stand-in/George special gap moved to {}: {extra:?}. If a special was AUTHORED on the stand-in this is the good failure — lower the number here in the same commit. If George LOST one, the subset assertion above would have fired first.",
            extra.len()
        );
    }

    /// THE SIDE-SPECIAL IS A REAL CAPTURE, not a strike wearing the name.
    ///
    /// ⭐ The inverted half is the one that matters. Asserting "`special_forward`
    /// resolves" would pass if somebody rebound it to the jab, and asserting
    /// "some move carries a capture attempt" would pass on the standing grab
    /// alone. This asserts the SPECIAL's own move captures, and that it is a
    /// different move from the standing grab — the two claims that together mean
    /// the button does the new thing rather than an old one.
    ///
    /// ⚠ **THIS GUARD WAS DEAD FROM `45e0ceada` TO `7bb880ff3` and its `#[test]`
    /// was restored without being run** — the box that restored it could not
    /// build. ⇒ If it is RED the first time it executes, that is not necessarily
    /// a regression introduced by whoever ran it: check the two commits that
    /// touched this file while it was dead (both add tests only and neither
    /// touches `lunge_grab`, so green is expected) before hunting a live cause.
    /// ⛔⛔ NOTHING BOUNDS AN AUTHORED GRAB'S REACH — not the params schema, not
    /// `acquire_captures`, not the content pass. A typo in `half_extents` is a
    /// grab that catches across the stage, and no other test would say so.
    ///
    /// ⭐ THE REASON THIS IS WORTH A GUARD NOW rather than when it breaks: every
    /// authored grab today is small enough that the missing bound never shows.
    /// The parity inventory's TETHER is the first move that would make a large
    /// number look correct, and the moment somebody authors one is the moment a
    /// stage-crossing typo stops being obviously wrong.
    ///
    /// ⚠ A CEILING, NOT A CLAMP, and deliberately: clamping in the engine would
    /// silently truncate a deliberate long reach, which is worse than refusing
    /// it. If a tether legitimately needs more, RAISE THIS NUMBER in the same
    /// commit that authors it — that is the ratchet working, not an obstacle.
    ///
    /// The bound is stated against the stage rather than the body: the smash
    /// platform is 480px wide, so 96 is a fifth of the ground a fighter stands
    /// on, and the longest thing authored today reaches 62.
    #[test]
    fn no_authored_grab_reaches_further_than_the_stage_allows() {
        use ambition_platformer2d::characters::smash_capture::{
            CaptureAttemptParams, CAPTURE_ATTEMPT,
        };

        /// A fifth of the shipped platform's width. See the note above.
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
                        "{who}'s `{}` reaches {reach}px (offset {:?} + half                          {:?}), past the {MAX_REACH_PX}px ceiling. If this is a                          deliberate tether, raise MAX_REACH_PX here in the same                          commit; if it is a typo, this is the only thing that                          would have caught it",
                        spec.id, params.offset, params.half_extents
                    );
                    assert!(
                        params.half_extents.0 > 0.0 && params.half_extents.1 > 0.0,
                        "{who}'s `{}` has a non-positive grab box {:?}, so it                          can never catch anybody",
                        spec.id, params.half_extents
                    );
                }
            }
        }

        // ⛔ A POPULATION FLOOR. This check's healthy answer is "no offender",
        // and a census that walked nothing answers the same way — which is
        // exactly how it would rot if `sustain_effect` or the key ever moved.
        assert!(
            seen >= 3,
            "found only {seen} authored capture attempt(s); the demo has at              least three (two stand-in grabs and George's), so this census is              measuring nothing rather than passing"
        );
    }

    #[test]
    fn the_side_special_is_a_command_grab_and_not_the_standing_grab_renamed() {
        use ambition_platformer2d::entity_catalog::WindowTag;
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

        // ⛔ Live during `Active` and nowhere else. A capture attempt sustained
        // through startup would catch bodies before the lunge commits, which is
        // the difference between a command grab and an aura.
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
            Some(ambition_platformer2d::characters::smash_capture::CAPTURE_ATTEMPT),
            "the special's live window sustains some OTHER effect, so it is not \
             a capture at all"
        );

        // ⭐ It travels. A command grab that closes no distance is strictly
        // worse than the standing grab it costs more to throw.
        let (dx, _) = special
            .start_impulse
            .expect("a command grab that does not lunge is a slower standing grab");
        assert!(
            dx > 0.0,
            "the command grab's impulse is {dx}, so it lunges backwards or \
             stands still"
        );

        // ⚠ The design claim in one assertion: this is the COMMITTED grab.
        // Equal startup would make it a strictly better standing grab with a
        // longer reach, and nothing would ever press the other one.
        assert!(
            special.windows.iter().any(|w| w.tag == WindowTag::Active)
                && standing.windows.iter().any(|w| w.tag == WindowTag::Active),
            "one of the two grabs has no active window"
        );
        let first_active = |m: &ambition_platformer2d::entity_catalog::MoveSpec| {
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

    /// A forward smash is a different move from a jab, by every measure that
    /// makes it one: it commits longer, hurts more, throws harder, and scales
    /// with the victim's damage.
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
                // A move that authors NO growth defers to the stage, which is
                // the stage's own fraction of its base — the comparable number.
                // ⛔ NOT `Option`'s ordering: `None < Some(_)` would have made
                // "the jab states nothing" read as "the jab grows least", which
                // is true here only by accident.
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
        // ... and the payoff is REACHABLE. This roster authors no charge policy,
        // so every smash of every shipped fighter relies on the one derived
        // from its own leading Startup window; a smash that resolves no policy
        // fires the instant it is pressed and the multiplier above is unpayable.
        // ⛔ AUTHORED, NOT DERIVED. `CHARGE_POSE_FRACTION` is the engine's
        // fallback for a move that says nothing about its pose, and it makes
        // the freeze a FRACTION of the windup — a slow smash would hold later
        // in real time than a fast one. This roster says where each swing
        // stops, and this is what keeps the fallback from quietly becoming the
        // authoring contract.
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
        // ⛔⛔ AND IT IS STRICTLY BEFORE THE FIRST STRIKE. Active membership is
        // `start_s <= t < end_s`, so a hold ON that instant is already inside a
        // live volume — a fighter charging with the hitbox out. The derived
        // policy clamps for this, and every smash in this roster relies on the
        // derivation, so this is the roster asking whether the clamp reached it.
        let first_active = smash
            .windows
            .iter()
            .filter(|w| {
                matches!(
                    w.tag,
                    ambition_platformer2d::entity_catalog::WindowTag::Active
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
    /// the guard for a UNIT MISMATCH that green tests cannot see. A volume's
    /// `knockback_growth` is absolute px/s per point; the ruleset's `knockback_growth` is a
    /// fraction of the move's base. Both are plain `f32`, both are "growth", and an authored
    /// move outranks the ruleset — so the first pass's fraction-shaped numbers made every move
    /// in this table grow ~40× slower than the stage declared, and nothing anywhere failed.
    ///
    /// A move MAY deliberately differ — that is what authoring is for — but it
    /// has to differ by a factor a reader can see, not by a unit.
    #[test]
    fn an_authored_growth_is_the_stage_declaration_in_the_stage_units() {
        for mv in &fighter_moveset().moves {
            for volume in mv.windows.iter().flat_map(|w| w.volumes.iter()) {
                // A volume that authors NO growth defers to the stage by
                // construction, and FIXED knockback (`Some(0.0)`) is a
                // deliberate choice no unit slip can produce from a non-zero
                // base. Only a stated, non-zero growth can carry the slip.
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
    /// the trap this pins: `autocancel_after_s` is IGNORED unless
    /// `landing_lag_s` is authored, so an aerial with a window and no lag reads
    /// as tuned and is inert.
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

    /// A grounded press cannot reach an aerial, and vice versa — the gates
    /// are what make one button eleven moves.
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

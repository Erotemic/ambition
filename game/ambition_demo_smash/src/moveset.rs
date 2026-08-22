//! **A platform fighter's actual moves**, authored on the character.
//!
//! **none of this needed engine work, and that is the finding.** The move
//! runtime already resolves a directional verb chain
//! (`attack_air_forward → attack_forward → attack_air → attack`), already reads
//! a Smash-strength gesture off a directional flick, already falls back from
//! `smash_forward` to `attack_forward`, and `MoveSpec` already carries
//! `landing_lag_s`, `autocancel_after_s`, `smash_charge_mult` and per-volume
//! `knockback_growth`. All of it was reachable only by AUTHORING, and nobody had.
//!
//! ## The shape of the repertoire
//!
//! ```text
//! attack              jab            fast, small, low launch
//! attack_up           up tilt        anti-air, launches upward
//! attack_down         down tilt      low poke, pops up
//! smash_forward       F-SMASH        slow, committed, big launch + growth
//! smash_up            U-smash        overhead, kills off the top
//! smash_down          D-smash        both sides, low launch
//! attack_air          n-air          the safe one: light lag, generous autocancel
//! attack_air_forward  f-air          the spacing tool: real lag, tight autocancel
//! attack_air_back     b-air          the strongest aerial, backwards
//! attack_air_up       u-air          juggles
//! attack_air_down     d-air          spike; the heaviest landing lag
//! ```
//!
//! **the numbers are a first authored pass and are meant to be tuned by
//! play.** They follow the genre's proportions rather than any one game's
//! frame data: a jab is ~3 frames of startup and a forward smash ~18, a smash
//! launches 3–4× a jab, and an aerial's landing lag is roughly its recovery.
//!
//! **`knockback_growth` here is ABSOLUTE px/s per point of damage, and the stage's
//! `SMASH_KNOCKBACK_GROWTH` is a FRACTION OF THE MOVE'S BASE** — two different units for the same
//! mechanic, and an authored move wins outright. Every growth below is now exactly `base *
//! SMASH_KNOCKBACK_GROWTH`, so authoring a move no longer silently opts it OUT of the stage's own
//! loop.

use ambition_platformer2d::entity_catalog::{
    CancelCondition, ClipBinding, EffectRef, HitVolume, ImpulseMode, MoveEvent, MoveEventKind,
    MoveGates, MoveSpec, MoveWindow, MovesetContract, VolumeShape, WindowTag,
};

/// Ground moves are grounded-only so an airborne body falls THROUGH them to its
/// aerials rather than throwing a tilt in mid-air.
pub(crate) fn grounded_only() -> MoveGates {
    MoveGates {
        grounded: Some(true),
    }
}

// The two helpers that remain have callers in this file and only in this file.

/// Aerials are airborne-only for the mirror reason: a grounded press must not
/// reach a move whose whole design is that landing costs you.
pub(crate) fn airborne_only() -> MoveGates {
    MoveGates {
        grounded: Some(false),
    }
}

/// One strike on one timeline: startup, one active window carrying one volume,
/// recovery.
///
/// Every move here is that shape, so the authored differences are the ones that
/// MATTER — how long you are committed, how far it reaches, how hard it throws,
/// and how much of the throw scales with the victim's damage.
#[allow(clippy::too_many_arguments)]
pub(crate) fn strike(
    id: &str,
    clip: &str,
    startup_s: f32,
    active_s: f32,
    recover_s: f32,
    offset: (f32, f32),
    half_extents: (f32, f32),
    damage: i32,
    knockback: f32,
    knockback_growth: f32,
    launch_dir: Option<(f32, f32)>,
) -> MoveSpec {
    let active_start = startup_s;
    let active_end = startup_s + active_s;
    MoveSpec {
        display_name: None,
        id: id.to_string(),
        clip: ClipBinding {
            clip: clip.to_string(),
            // Every fighter here draws from the robot lineage's sheets, which
            // carry `attack` and little else — so the fallback is what actually
            // plays for most of these. A missing clip must not cost the move its
            // gameplay.
            fallbacks: vec!["attack".to_string(), "idle".to_string()],
        },
        duration_s: active_end + recover_s,
        windows: vec![
            MoveWindow {
                start_s: 0.0,
                end_s: active_start,
                tag: WindowTag::Startup,
                volumes: Vec::new(),
                motion_scale: 1.0,
                sustain_effect: None,
            },
            MoveWindow {
                start_s: active_start,
                end_s: active_end,
                tag: WindowTag::Active,
                volumes: vec![HitVolume {
                    shape: VolumeShape::Rect {
                        offset,
                        half_extents,
                    },
                    damage,
                    knockback,
                    knockback_growth,
                    launch_dir,
                    on_hit: None,
                    // The blade tag: the move runtime draws the slash from the
                    // SAME spawned volume, so the hitbox and the arc can never
                    // point different ways.
                    vfx: Some("slash_arc".to_string()),
                    hit_sfx: None,
                }],
                motion_scale: 1.0,
                sustain_effect: None,
            },
            MoveWindow {
                start_s: active_end,
                end_s: active_end + recover_s,
                tag: WindowTag::Recovery,
                volumes: Vec::new(),
                motion_scale: 1.0,
                sustain_effect: None,
            },
        ],
        events: Vec::new(),
        gates: MoveGates::default(),
        start_impulse: None,
        smash_charge_mult: 1.0,
        landing_lag_s: None,
        autocancel_after_s: None,
    }
}

// ---------------------------------------------------------------------------
// Authoring combinators. Every one of these takes a `MoveSpec` and returns it
// with one more thing said about it, so a table reads as a list of DECISIONS
// rather than a wall of struct literals — and so the two fighters in this crate
// (and any third) say the same things the same way.
// ---------------------------------------------------------------------------

/// The proper-time instant a move's first hit becomes live. Where its feedback
/// belongs, and where a self-displacement usually does.
pub(crate) fn active_start(m: &MoveSpec) -> f32 {
    m.windows
        .iter()
        .find(|w| matches!(w.tag, WindowTag::Active))
        .map_or(0.0, |w| w.start_s)
}

fn event(mut m: MoveSpec, at_s: f32, kind: MoveEventKind) -> MoveSpec {
    m.events.push(MoveEvent { at_s, kind });
    m
}

/// **A TIMED SELF-DISPLACEMENT.** `Set` commands the velocity outright; `Add`
/// contributes to it. See `MoveEventKind::Impulse` — the difference is the
/// difference between a recovery and a hop.
pub(crate) fn impulse(m: MoveSpec, at_s: f32, local: (f32, f32), mode: ImpulseMode) -> MoveSpec {
    event(m, at_s, MoveEventKind::Impulse { local, mode })
}

/// **A CANCEL WINDOW.** The timeline IS the cancel table, so a combo route is
/// authored here and nowhere else.
pub(crate) fn cancelable(
    mut m: MoveSpec,
    start_s: f32,
    end_s: f32,
    into: &[&str],
    condition: CancelCondition,
) -> MoveSpec {
    m.windows.push(MoveWindow {
        start_s,
        end_s,
        tag: WindowTag::Cancelable {
            into: into.iter().map(|s| (*s).to_string()).collect(),
            condition,
        },
        volumes: Vec::new(),
        motion_scale: 1.0,
        sustain_effect: None,
    });
    m
}

/// **A CONDITIONAL TECHNIQUE ON CONTACT** — the engine's `on_hit` seam, applied
/// to every volume the move lands. `pogo_bounce` is the one this crate uses: hit
/// a body on the way down and be thrown back up by it.
pub(crate) fn on_hit(mut m: MoveSpec, key: &str) -> MoveSpec {
    for volume in m.windows.iter_mut().flat_map(|w| w.volumes.iter_mut()) {
        volume.on_hit = Some(EffectRef::new(key));
    }
    m
}

/// **A TAIL THE BODY CANNOT STEER OUT OF.** Extends the move to `to_s` with a
/// Recovery window whose `motion_scale` damps the owner's steering — the genre's
/// "you are committed now", authored rather than hardcoded, and enforced
/// body-side so it binds a CPU and a human identically.
pub(crate) fn committed_tail(mut m: MoveSpec, to_s: f32, motion_scale: f32) -> MoveSpec {
    let from = m.duration_s;
    if to_s <= from {
        return m;
    }
    m.windows.push(MoveWindow {
        start_s: from,
        end_s: to_s,
        tag: WindowTag::Recovery,
        volumes: Vec::new(),
        motion_scale,
        sustain_effect: None,
    });
    m.duration_s = to_s;
    m
}

/// **WHAT A MOVE FEELS LIKE, as six named classes rather than per-move art.**
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
    /// **A RECOVERY ACTIVATING.** Its own sound and its own burst, because
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
        m = event(
            m,
            0.0,
            MoveEventKind::Sfx {
                cue: cue.to_string(),
            },
        );
    }
    m = event(
        m,
        at,
        MoveEventKind::Sfx {
            cue: swing_cue.to_string(),
        },
    );
    if let Some(effect) = burst {
        m = event(
            m,
            at,
            MoveEventKind::Vfx {
                effect: effect.to_string(),
                at: (0.0, 0.0),
                scale: 1.0,
                sfx: None,
            },
        );
    }
    if let Some(cue) = hit_cue {
        for volume in m.windows.iter_mut().flat_map(|w| w.volumes.iter_mut()) {
            volume.hit_sfx = Some(cue.to_string());
        }
    }
    m
}

/// **The fighter repertoire**, as one authored contract.
///
/// Shared by this demo's three fighters today. That is a content decision, not
/// an architectural one: the moveset rides the CHARACTER, so giving George a
/// heavier one is editing his definition and nothing else.
pub fn fighter_moveset() -> MovesetContract {
    let mut moves = Vec::new();

    // ── grounded ─────────────────────────────────────────────────────────────
    //
    // The jab is the fast, safe, boring one — it exists to be thrown at nothing
    // and get away with it, which is what makes the smash below a decision.
    let mut jab = strike(
        "jab",
        "attack",
        0.05,
        0.06,
        0.14,
        (26.0, 0.0),
        (18.0, 14.0),
        3,
        55.0,
        1.10,
        None,
    );
    jab.gates = grounded_only();
    moves.push(jab);

    let mut up_tilt = strike(
        "tilt_up",
        "attack",
        0.07,
        0.08,
        0.18,
        (10.0, -30.0),
        (20.0, 22.0),
        5,
        70.0,
        1.40,
        // Straight up: an anti-air that starts a juggle rather than sending the
        // opponent away.
        Some((0.15, -1.0)),
    );
    up_tilt.gates = grounded_only();
    moves.push(up_tilt);

    let mut down_tilt = strike(
        "tilt_down",
        "attack",
        0.06,
        0.06,
        0.16,
        (26.0, 16.0),
        (20.0, 10.0),
        4,
        60.0,
        1.20,
        // A low poke that pops them up into the juggle.
        Some((0.5, -0.85)),
    );
    down_tilt.gates = grounded_only();
    moves.push(down_tilt);

    // ── the smashes ──────────────────────────────────────────────────────────
    //
    // **the move the demo did not have.** A forward smash is eighteen frames
    // of startup you cannot take back, and the reason anybody accepts that is
    // the launch at the end of it: three times the jab's, growing with the
    // victim's percent, so at 120% it is the thing that ends the stock. The
    // charge multiplier is what a HELD press pays for.
    let mut f_smash = strike(
        "smash_forward",
        "attack",
        0.30,
        0.07,
        0.34,
        (40.0, -4.0),
        (28.0, 20.0),
        15,
        150.0,
        3.00,
        // Slightly upward and away: the classic kill angle. A contact-derived
        // direction would send a crouching opponent along the floor instead.
        Some((1.0, -0.42)),
    );
    f_smash.gates = grounded_only();
    // A fully-held charge lands 1.7× as hard. `smash_charge_mult` scales damage
    // AND knockback by how far the owner's clock got through the leading
    // Startup window before release, so the commitment and the payoff are the
    // same authored number.
    f_smash.smash_charge_mult = 1.7;
    moves.push(f_smash);

    let mut up_smash = strike(
        "smash_up",
        "attack",
        0.26,
        0.08,
        0.32,
        (8.0, -38.0),
        (24.0, 30.0),
        14,
        140.0,
        2.80,
        Some((0.12, -1.0)),
    );
    up_smash.gates = grounded_only();
    up_smash.smash_charge_mult = 1.7;
    moves.push(up_smash);

    let mut down_smash = strike(
        "smash_down",
        "attack",
        0.22,
        0.08,
        0.30,
        (0.0, 18.0),
        (40.0, 14.0),
        12,
        130.0,
        2.60,
        // Low and outward — the edge-guarding smash, not a launcher.
        Some((1.0, -0.25)),
    );
    down_smash.gates = grounded_only();
    down_smash.smash_charge_mult = 1.6;
    moves.push(down_smash);

    // ── aerials ──────────────────────────────────────────────────────────────
    //
    // **landing lag and auto-cancel are what make an aerial a DECISION**, and
    // both were engine features with no adopter. The pair reads: throw this one
    // early in a jump and land clean; throw it late and pay for it.
    let mut n_air = strike(
        "air_neutral",
        "attack",
        0.06,
        0.14,
        0.16,
        (14.0, 0.0),
        (26.0, 22.0),
        6,
        75.0,
        1.50,
        None,
    );
    n_air.gates = airborne_only();
    n_air.landing_lag_s = Some(0.10);
    n_air.autocancel_after_s = Some(0.26);
    moves.push(n_air);

    let mut f_air = strike(
        "air_forward",
        "attack",
        0.09,
        0.08,
        0.22,
        (32.0, -4.0),
        (22.0, 18.0),
        9,
        105.0,
        2.10,
        Some((1.0, -0.35)),
    );
    f_air.gates = airborne_only();
    f_air.landing_lag_s = Some(0.18);
    f_air.autocancel_after_s = Some(0.30);
    moves.push(f_air);

    let mut b_air = strike(
        "air_back",
        "attack",
        0.10,
        0.07,
        0.24,
        (-32.0, -2.0),
        (22.0, 18.0),
        11,
        125.0,
        2.50,
        // Backwards and slightly up: the strongest aerial, and the one you have
        // to turn around for.
        Some((-1.0, -0.38)),
    );
    b_air.gates = airborne_only();
    b_air.landing_lag_s = Some(0.20);
    b_air.autocancel_after_s = Some(0.32);
    moves.push(b_air);

    let mut u_air = strike(
        "air_up",
        "attack",
        0.07,
        0.09,
        0.20,
        (4.0, -34.0),
        (22.0, 24.0),
        7,
        90.0,
        1.80,
        Some((0.1, -1.0)),
    );
    u_air.gates = airborne_only();
    u_air.landing_lag_s = Some(0.14);
    u_air.autocancel_after_s = Some(0.28);
    moves.push(u_air);

    let mut d_air = strike(
        "air_down",
        "attack",
        0.12,
        0.10,
        0.26,
        (6.0, 30.0),
        (20.0, 22.0),
        10,
        110.0,
        2.20,
        // Straight DOWN — a spike. Offstage this is a stock; onstage it is a
        // bounce the opponent has to deal with.
        Some((0.0, 1.0)),
    );
    d_air.gates = airborne_only();
    // The heaviest lag in the set: a missed spike over the stage should hurt.
    d_air.landing_lag_s = Some(0.28);
    d_air.autocancel_after_s = Some(0.40);
    moves.push(d_air);

    // **A GRAB, BECAUSE EVERY FIGHTER IN THE GENRE HAS ONE.**
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
        back_throw: Some(ambition_platformer2d::characters::smash_capture::author_throw(
            ambition_platformer2d::characters::smash_capture::capture_beat("throw_back", "attack", 0.3),
            0.17,
            ambition_platformer2d::characters::smash_capture::CaptureThrowParams {
                damage: 10,
                knockback: 130.0,
                knockback_growth: 2.21,
                launch_dir: (-1.0, -0.31),
            },
        )),
        up_throw: Some(ambition_platformer2d::characters::smash_capture::author_throw(
            ambition_platformer2d::characters::smash_capture::capture_beat("throw_up", "attack", 0.29),
            0.16,
            ambition_platformer2d::characters::smash_capture::CaptureThrowParams {
                damage: 9,
                knockback: 125.0,
                knockback_growth: 2.14,
                launch_dir: (0.0, -1.0),
            },
        )),
        down_throw: Some(ambition_platformer2d::characters::smash_capture::author_throw(
            ambition_platformer2d::characters::smash_capture::capture_beat("throw_down", "attack", 0.31),
            0.17,
            ambition_platformer2d::characters::smash_capture::CaptureThrowParams {
                damage: 7,
                knockback: 89.0,
                knockback_growth: 1.68,
                launch_dir: (0.36, -0.92),
            },
        )),
    };
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

    /// **Every verb resolves to a move that exists.** A verb pointing at a
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

    /// **A forward smash is a different move from a jab**, by every measure that
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
                .map(|v| (v.damage, v.knockback, v.knockback_growth))
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
    }

    /// **Every authored growth equals the stage's own declaration**, in the
    /// stage's units.
    ///
    /// **the guard for a UNIT MISMATCH that green tests cannot see.** A volume's
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
                let expected = volume.knockback * crate::SMASH_KNOCKBACK_GROWTH;
                assert!(
                    (volume.knockback_growth - expected).abs() < 0.01,
                    "`{}` launches at {} and grows {}/point, but the stage \
                     declares {} of base = {expected}/point. A growth that is \
                     off by a FACTOR is the fraction-vs-absolute unit slip, and \
                     it silently opts this move out of the percent loop",
                    mv.id,
                    volume.knockback,
                    volume.knockback_growth,
                    crate::SMASH_KNOCKBACK_GROWTH,
                );
            }
        }
    }

    /// **The aerials commit, and the auto-cancel window is real.**
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

    /// **A grounded press cannot reach an aerial, and vice versa** — the gates
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

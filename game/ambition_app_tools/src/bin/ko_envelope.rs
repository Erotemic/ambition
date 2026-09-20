//! `ko_envelope` — a LAUNCHER-PULSE CENSUS over the prepared moveset graph,
//! and (next phase) a stage-outcome probe over the same launchers.
//!
//! ⭐⭐ **THE ARCHITECTURAL RULE THIS FILE EXISTS TO OBEY:** read authoring from
//! the COMPILED RUNTIME CONTRACT, drive outcomes through the REAL SIMULATION,
//! and reconstruct neither from source text.
//!
//! ⛔⛔ **AND IT IS WRITTEN AGAINST A MEASURED FAILURE.** 2026-09-13, the census
//! this replaces was attempted four times with `grep`/`awk` over
//! `game/ambition_content/src/*_moveset.rs`, and every attempt was wrong:
//!
//! * `paste - -` desynchronised on 300 `knockback:` lines against 295
//!   `knockback_growth:` lines and reported a mean growth/base ratio of **13.7**.
//! * `knockback_growth: TORQUE_GROWTH` (a named constant) and
//!   `knockback_growth: Some(1.35)` were silently skipped by a numeric-only
//!   pattern, desynchronising everything after them.
//! * A search for the literal `forward_throw:` found 10 sites, all in schema and
//!   test code, and concluded **no fighter authors throws**. Every fighter
//!   authors five capture moves. The field's own doc says why the grep failed:
//!   *"this replaces a grep. A goal check read the movesets looking for
//!   `capture: Some`, which is the kind of guard that answers a question the
//!   compiler can answer better."*
//! * A `*_moveset.rs` glob reported 14 movesets where the directory holds 19.
//!
//! ⇒ Every one of those is unreachable from here. A throw is found by asking the
//! contract for a VERB; its numbers are hydrated from the same typed
//! `EffectRef` the engine itself reads. There is no spelling to miss, no
//! shorthand to mis-parse, and no file list to keep current.
//!
//! ```text
//! PreparedCharacterRegistry → SmashRoster::assemble → registry.get(id)
//!   → kit.projectable_moveset() → MovesetContract
//!       strikes: every HitVolume on every WindowTag::Active window
//!       throws:  move_for_verb(CAPTURE_THROW_*_VERB)
//!                  → effect_refs() where key == "smash.capture_throw"
//!                  → hydrate::<CaptureThrowParams>()
//! ```

use std::collections::BTreeMap;

// ⛔ A `#[path]` MODULE, NOT A LIB — this package declares in its own manifest
// that it has no lib on purpose, because a lib relinks every binary in it
// whenever any of them changes. `trap_probe` and `wire_probe` include this same
// file the same way.
//
// ⭐ AND IT IS WORTH REUSING RATHER THAN REWRITING: its header records four
// separate findings that each cost a run measuring nothing — `NoWindow` omits
// the render app entirely, a hand-stepped `update()` does not wait for the wgpu
// device `run()` waits for, the two hosts do not announce the round the same
// way, and a seated fighter's `Brain` overwrites any control frame delivered
// from outside.
#[path = "../probe_stage.rs"]
mod probe_stage;

use ambition_entity_catalog::smash_capture::{CaptureThrowParams, CAPTURE_CARRY, CAPTURE_THROW};
use ambition_entity_catalog::{
    HitVolume, MovesetContract, WindowTag, CAPTURE_THROW_BACK_VERB, CAPTURE_THROW_DOWN_VERB,
    CAPTURE_THROW_FORWARD_VERB, CAPTURE_THROW_UP_VERB,
};

/// The four canonical throw verbs, in report order.
///
/// ⛔ THESE ARE THE CATALOG'S OWN CONSTANTS, not strings spelled here.
/// `smash_capture::verbs` re-exports these very items — *"re-exports, not a
/// second definition … Spelling them again here would be two places for a typo
/// to become a press that does nothing"* — so a lookup through them cannot
/// disagree with what `SmashCaptureRepertoire::bound` installed.
const THROW_VERBS: [(&str, &str); 4] = [
    ("forward_throw", CAPTURE_THROW_FORWARD_VERB),
    ("back_throw", CAPTURE_THROW_BACK_VERB),
    ("up_throw", CAPTURE_THROW_UP_VERB),
    ("down_throw", CAPTURE_THROW_DOWN_VERB),
];

/// ONE AUTHORED PULSE THAT CAN SEND A BODY, in the two forms this game authors.
///
/// ⛔ **NOT A NEW GAMEPLAY ABSTRACTION.** This is the balance instrument's
/// projection of two existing authoring forms onto the one question it asks —
/// "what does this pulse do to a victim at percent p". Nothing in the engine
/// needs to know it exists, and nothing should be taught to.
#[derive(Debug, Clone)]
enum Launcher {
    /// One `HitVolume` of one `Active` window.
    ///
    /// ⚠ ONE VOLUME IS A PULSE, NOT A MOVE. A tipper and a sourspot are
    /// separate balance facts with different envelopes, and an earlier hit of a
    /// multi-hit move adds damage before a later finisher lands — so a
    /// threshold measured here is a PULSE threshold and is reported as one.
    Strike {
        fighter: String,
        move_id: String,
        /// Every input verb that names this move, from the contract's own map.
        /// Roles come from authored bindings, never from guessing at ids like
        /// `fsmash` or `jab`.
        verbs: Vec<String>,
        window: usize,
        volume: usize,
        hit: HitVolume,
    },
    /// One authored throw, hydrated from the effect the engine reads.
    ///
    /// A throw is naturally one release pulse, so its threshold IS meaningful at
    /// the move level.
    Throw {
        fighter: String,
        move_id: String,
        verb: String,
        /// Which of the four slots, for report grouping.
        slot: &'static str,
        params: CaptureThrowParams,
    },
}

impl Launcher {
    fn fighter(&self) -> &str {
        match self {
            Launcher::Strike { fighter, .. } | Launcher::Throw { fighter, .. } => fighter,
        }
    }

    fn move_id(&self) -> &str {
        match self {
            Launcher::Strike { move_id, .. } | Launcher::Throw { move_id, .. } => move_id,
        }
    }

    fn kind(&self) -> &'static str {
        match self {
            Launcher::Strike { .. } => "strike",
            Launcher::Throw { .. } => "throw",
        }
    }

    /// Stable identity: what a later KO row joins back to.
    fn key(&self) -> String {
        match self {
            Launcher::Strike {
                fighter,
                move_id,
                window,
                volume,
                ..
            } => format!("{fighter}/{move_id}/w{window}/v{volume}"),
            Launcher::Throw {
                fighter, move_id, ..
            } => format!("{fighter}/{move_id}/throw"),
        }
    }

    /// Borrow this launcher as a deliverable pulse.
    ///
    /// ⭐ THE ONLY PLACE THE VARIANT IS INSPECTED ON THE MEASURING PATH. Every
    /// site downstream — the repeatability self-test, the row loop, `launch_at`,
    /// `kills`, `ko_threshold` — takes a `Pulse` and never asks which kind it
    /// holds, so a throw cannot quietly fall down a strike-shaped code path and
    /// report a number about the fixture.
    fn pulse(&self) -> Pulse<'_> {
        match self {
            Launcher::Strike { hit, .. } => Pulse::Strike(hit),
            Launcher::Throw { params, .. } => Pulse::Throw(params),
        }
    }

    fn damage(&self) -> i32 {
        match self {
            Launcher::Strike { hit, .. } => hit.damage,
            Launcher::Throw { params, .. } => params.damage,
        }
    }

    fn base_knockback(&self) -> f32 {
        match self {
            Launcher::Strike { hit, .. } => hit.knockback,
            Launcher::Throw { params, .. } => params.knockback,
        }
    }

    /// What the AUTHOR wrote. `None` on a strike means "the volume did not
    /// decide"; a throw always states one.
    fn authored_growth(&self) -> Option<f32> {
        match self {
            Launcher::Strike { hit, .. } => hit.knockback_growth,
            Launcher::Throw { params, .. } => Some(params.knockback_growth),
        }
    }

    /// What the ENGINE will actually use.
    ///
    /// ⛔ `None` IS NOT ZERO. `resolved_hitbox_knockback_magnitude` does
    /// `growth.unwrap_or_else(|| base * ruleset_growth.max(0.0))`, so an
    /// unauthored growth resolves to `base × the ruleset's own growth` — and
    /// `Some(0.0)` is the documented FIXED-knockback case, which is a different
    /// thing entirely. Reading a bare `0.0` as unspecified once made the
    /// fixed-knockback case the one value nobody could author.
    fn effective_growth(&self, ruleset_growth: f32) -> f32 {
        self.authored_growth()
            .unwrap_or_else(|| self.base_knockback() * ruleset_growth.max(0.0))
    }

    /// Fixed knockback: the launch ignores the victim's percent AND weight,
    /// because the launch law short-circuits a zero growth and returns
    /// `base` untouched.
    fn is_fixed(&self) -> bool {
        self.authored_growth() == Some(0.0)
    }

    fn launch_dir(&self) -> Option<(f32, f32)> {
        match self {
            Launcher::Strike { hit, .. } => hit.launch_dir,
            Launcher::Throw { params, .. } => Some(params.launch_dir),
        }
    }

    /// The role this pulse answers to, from AUTHORED VERB BINDINGS.
    fn role(&self) -> String {
        match self {
            Launcher::Throw { slot, .. } => (*slot).to_string(),
            Launcher::Strike { verbs, move_id, .. } => {
                if verbs.is_empty() {
                    return format!("unbound:{move_id}");
                }
                // The most specific binding wins; ties broken by the contract's
                // own deterministic (BTreeMap) order.
                let mut best = verbs[0].as_str();
                for v in verbs {
                    if v.len() > best.len() {
                        best = v.as_str();
                    }
                }
                best.to_string()
            }
        }
    }
}

/// Why one fighter, verb or volume could not become a launcher.
///
/// ⭐ MALFORMED ROWS ARE REPORTED, NOT DROPPED. A census that silently skips
/// what it cannot read is the same instrument as a grep that finds nothing and
/// calls it absence.
#[derive(Debug)]
struct Malformed {
    fighter: String,
    what: String,
    why: String,
}

/// A capture slot that resolves, is authored on purpose, and DOES NOT LAUNCH.
///
/// ⭐ RECORDED RATHER THAN DROPPED, so its absence from the KO table reads as
/// authored intent instead of a hole somebody should fill. The goblin's down
/// press hoists its captive (`CAPTURE_CARRY`) — *"A CARRY, NOT A THROW, AND NOT
/// BOTH"* — and a census that reported it as a missing down-throw would invite
/// exactly the repair that guard exists to forbid.
#[derive(Debug)]
struct NonLaunching {
    fighter: String,
    slot: &'static str,
    move_id: String,
}

/// Build every launcher one fighter authors.
fn launchers_of(
    fighter: &str,
    contract: &MovesetContract,
    bad: &mut Vec<Malformed>,
    non_launching: &mut Vec<NonLaunching>,
) -> Vec<Launcher> {
    let mut out = Vec::new();

    // Reverse the contract's own verb map: move id → every verb naming it.
    let mut verbs_of: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for (verb, move_id) in &contract.verbs {
        verbs_of.entry(move_id.as_str()).or_default().push(verb.clone());
    }

    // STRIKES — every volume of every Active window, never collapsed.
    for spec in &contract.moves {
        for (w, window) in spec.windows.iter().enumerate() {
            if !matches!(window.tag, WindowTag::Active) {
                continue;
            }
            for (v, hit) in window.volumes.iter().enumerate() {
                out.push(Launcher::Strike {
                    fighter: fighter.to_string(),
                    move_id: spec.id.clone(),
                    verbs: verbs_of.get(spec.id.as_str()).cloned().unwrap_or_default(),
                    window: w,
                    volume: v,
                    hit: hit.clone(),
                });
            }
        }
    }

    // THROWS — asked for by VERB, hydrated from the engine's own effect.
    //
    // ⛔⛔ A RESOLVED CAPTURE VERB IS NOT NECESSARILY A LAUNCHER, and assuming it
    // was is a defect this instrument shipped with for exactly one run. A slot
    // may carry `CAPTURE_CARRY` instead: the goblin's down press HOISTS its
    // captive rather than launching it, authored on purpose and guarded by
    // `the_goblins_down_throw_hauls_instead_of_launching`, whose own words are
    // *"A CARRY, NOT A THROW, AND NOT BOTH"*. The first version of this loop
    // called that malformed — which would have meant breaking a correct fighter
    // to green an instrument.
    //
    // ⇒ THREE OUTCOMES, not two: a throw (launches, measured here), a carry (a
    // capture slot that deliberately does not launch, recorded so its absence
    // from the KO table reads as intent rather than a hole), and absent.
    for (slot, verb) in THROW_VERBS {
        let Some(spec) = contract.move_for_verb(verb) else {
            // Not a defect: an unauthored throw is the authored way to say
            // "this fighter has none".
            continue;
        };
        let throws: Vec<_> = spec
            .effect_refs()
            .into_iter()
            .filter(|(_, effect)| effect.key == CAPTURE_THROW)
            .collect();
        let carries = spec
            .effect_refs()
            .into_iter()
            .filter(|(_, effect)| effect.key == CAPTURE_CARRY)
            .count();
        if throws.is_empty() && carries > 0 {
            non_launching.push(NonLaunching {
                fighter: fighter.to_string(),
                slot,
                move_id: spec.id.clone(),
            });
            continue;
        }
        if throws.len() != 1 {
            bad.push(Malformed {
                fighter: fighter.to_string(),
                what: format!("{slot} ({verb}) → {}", spec.id),
                why: format!(
                    "resolves to a move carrying {} `{CAPTURE_THROW}` and {carries} \
                     `{CAPTURE_CARRY}` effects; expected exactly 1 throw, or a carry",
                    throws.len()
                ),
            });
            continue;
        }
        match throws[0].1.params.hydrate::<CaptureThrowParams>() {
            Ok(params) => out.push(Launcher::Throw {
                fighter: fighter.to_string(),
                move_id: spec.id.clone(),
                verb: verb.to_string(),
                slot,
                params,
            }),
            Err(err) => bad.push(Malformed {
                fighter: fighter.to_string(),
                what: format!("{slot} ({verb}) → {}", spec.id),
                why: format!("installed throw params did not hydrate: {err}"),
            }),
        }
    }

    out
}

/// `count / median / p25 / p75 / min / max` over one column of one role.
#[derive(Debug, Default)]
struct Dist {
    n: usize,
    min: f32,
    p25: f32,
    median: f32,
    p75: f32,
    max: f32,
}

fn dist(mut xs: Vec<f32>) -> Dist {
    if xs.is_empty() {
        return Dist::default();
    }
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let at = |q: f32| -> f32 {
        let i = ((xs.len() as f32 - 1.0) * q).round() as usize;
        xs[i.min(xs.len() - 1)]
    };
    Dist {
        n: xs.len(),
        min: xs[0],
        p25: at(0.25),
        median: at(0.5),
        p75: at(0.75),
        max: xs[xs.len() - 1],
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// PHASE 2: THE STAGE-OUTCOME PROBE
//
// ⛔⛔ **A KO IS NOT A RESET.** `measure_cell` in `smash_in_the_host.rs` re-seats
// an entire match per cell, which is correct and unaffordable here: the matrix
// is thousands of trials. Reusing one match is possible, but ONLY behind an
// explicit reset contract, because a knockout spends a stock, opens
// `DeathInterlude`, adds `PendingRespawn`/`OutOfPlay`, grants `RespawnGrace`
// (which publishes `Invulnerability::RESPAWN`), teleports the body, and leaves
// `BodyKnockedOut` messages in the buffer that a fresh cursor would re-read as
// the NEXT trial's result.
//
// ⇒ Every clause below is an assertion, not a hope, and each is tied to a fact
// read out of the engine rather than assumed.
// ─────────────────────────────────────────────────────────────────────────────

/// What one pulse did to one victim at one percent.
#[derive(Debug, Clone)]
struct Trial {
    /// Read off `HitEvent.knockback` — the engine's own resolved number, never
    /// recomputed here. `None` means no `LaunchSpeed` reached the victim, which
    /// is a fixture failure rather than a measurement of zero.
    resolved_launch: Option<f32>,
    ko: bool,
    /// Did the launch reach this body's own tumble threshold, read live off its
    /// motion model rather than restated as `500.0`.
    tumbled: bool,
    ticks_to_ko: Option<usize>,
    /// Victim state at the moment the pulse was fired, for the determinism
    /// probe. Two trials of the same pulse at the same percent must agree here.
    at_strike: String,
    /// The meter the victim actually entered the pulse carrying.
    entry_percent: i32,
    /// What the launch arithmetic saw. For a throw this is `entry + damage`,
    /// because `apply_capture_throws` damages BEFORE reading the meter.
    effective_percent: i32,
    /// WAS THE VICTIM STILL NON-ACTIONABLE WHEN IT CROSSED THE BLAST LINE?
    ///
    /// ⭐ TWO DIFFERENT EVENTS WEAR ONE NAME IN EVERY TABLE THIS TOOL HAS
    /// PRINTED. A move that launches a body through the blast line while it is
    /// still in hitstun or tumbling took the stock — the victim could not have
    /// acted. A body that leaves the world after control returned merely FELL,
    /// and scoring that as kill power credits the move with a stock the stage
    /// took. Only the first is evidence about knockback.
    ///
    /// `None` when no KO occurred.
    ko_forced: Option<bool>,
}

/// The outcome of a threshold search, including the ones that are not a number.
#[derive(Debug)]
enum Threshold {
    /// Lowest entry percent that KOs, verified: `at-1` survives, `at` kills.
    At(i32),
    /// Never killed anywhere in the tested range.
    Above(i32),
    /// KO then survive as percent RISES. Samples preserved rather than
    /// collapsed into a scalar that would be a fabrication.
    NonMonotonic(Vec<(i32, bool)>),
    /// The pulse never connected — nothing measured.
    NoContact,
    /// The probe could not obtain a clean trial at this percent.
    ///
    /// ⛔ THIS IS NOT A SURVIVAL, AND THE PREVIOUS FORM SAID IT WAS. A refused
    /// reset pushed `(p, false)`, so a percent the INSTRUMENT failed to measure
    /// was recorded as a percent the VICTIM lived through — and the threshold
    /// walked straight past it. Run 2 refused 8 of 64 rows this way.
    Refused(i32),
    /// CALIBRATION ONLY — trials at ONE percent disagreed with each other.
    ///
    /// ⛔ THIS IS THE CELL MAJORITY-OF-THREE USED TO HIDE. The coarse path
    /// outvotes a 2:1 split and prints a crisp integer; that integer then feeds
    /// `G_new = G_old * p0/p1` and becomes a SHIPPED growth. A threshold that
    /// could not reproduce itself is not a number to author from, so calibration
    /// refuses it by name instead of rounding it off.
    Unstable { percent: i32, yes: i32, no: i32 },
}

impl Threshold {
    fn cell(&self) -> String {
        match self {
            Threshold::At(p) => p.to_string(),
            Threshold::Above(p) => format!(">{p}"),
            Threshold::NonMonotonic(s) => format!("NON_MONOTONIC{s:?}"),
            Threshold::NoContact => "NO_CONTACT".into(),
            Threshold::Refused(p) => format!("REFUSED@{p}"),
            Threshold::Unstable { percent, yes, no } => {
                format!("UNSTABLE@{percent}({yes}y/{no}n)")
            }
        }
    }
}

/// WHAT THREE TRIALS AT ONE PERCENT ACTUALLY SAID.
///
/// ⛔ `bool` WAS THE WRONG RETURN AND IT DESTROYED THE FINDING AT THE SEAM.
/// `kills` ran up to three trials and collapsed them with `yes > no`, so a cell
/// that killed twice and survived once became indistinguishable from one that
/// killed three times. The disagreement was printed to stderr and then thrown
/// away, which means every caller — including the one that authors a shipped
/// growth — saw a confidence the measurement did not have.
#[derive(Clone, Copy)]
enum Vote {
    Killed,
    Survived,
    /// The same initial state produced BOTH outcomes. Counts kept, because
    /// 2:1 and 1:2 are different evidence about where the edge sits.
    Mixed { yes: i32, no: i32 },
}

impl Vote {
    /// The historical majority-of-three verdict.
    ///
    /// ⚠ THE COARSE PATH STILL USES THIS, DELIBERATELY. Every table this tool
    /// has ever recorded was measured under majority rule, and changing the
    /// default would silently re-define what those runs mean — comparability
    /// across them is the one thing a later reading cannot buy back. Calibration
    /// opts out by refusing the cell instead; the survey keeps its semantics.
    fn majority(self) -> bool {
        match self {
            Vote::Killed => true,
            Vote::Survived => false,
            Vote::Mixed { yes, no } => yes > no,
        }
    }
}

/// ⛔ `true`/`false` ARE PRESERVED VERBATIM, AND THAT IS THE POINT.
///
/// The determinism probe prints six of these side by side and asks whether they
/// agree. Every run recorded before `Vote` existed rendered a `bool`, so keeping
/// the unanimous spellings byte-identical keeps those runs comparable with new
/// ones. The ONLY new string is the case a `bool` could not express: a cell that
/// disagreed with itself used to be majority-ized into a confident `true` before
/// anyone could see it, which is the defect that arm was built to catch.
impl std::fmt::Display for Vote {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Vote::Killed => write!(f, "true"),
            Vote::Survived => write!(f, "false"),
            Vote::Mixed { yes, no } => write!(f, "MIXED({yes}y/{no}n)"),
        }
    }
}

/// THE ATTACKER'S OWN METER FOR EVERY TRIAL — `attacker_pct=<n>`, default 0.
///
/// ⛔⛤ ZERO IS NOT A NEUTRAL CHOICE, IT IS A CONTROL. `rage_scale` reads the
/// ATTACKER's damage and multiplies every resolved launch, so a sweep that lets
/// it float is measuring the fixture's accumulated damage as much as the
/// authored value — `lib.rs:4161` records a case where borrowed rage turned a
/// survival into a knockout and "only controlling rage could tell the two apart".
/// Pinning it to 0 is what makes rows comparable.
///
/// ⭐ AND IT IS EXACTLY WHY THE FLAG EXISTS. A kill percent measured at rage 1.0
/// is the WEAKEST the move will ever be; the question "does this move still kill
/// when the attacker is behind" cannot be asked of a rage-pinned table at all.
/// Cached, because `reset_trial` runs once per trial and a step-1 scan is ~110 of
/// them per cell.
fn attacker_meter() -> i32 {
    static V: std::sync::OnceLock<i32> = std::sync::OnceLock::new();
    *V.get_or_init(|| {
        std::env::args()
            .find_map(|a| {
                a.strip_prefix("attacker_pct=")
                    .and_then(|n| n.parse::<i32>().ok())
            })
            .filter(|n| *n >= 0)
            .unwrap_or(0)
    })
}

/// WHAT DELIVERS ONE PULSE — and the ONLY thing the threshold machinery varies.
///
/// ⛔⛤ A SECOND SEARCH WOULD BE A SECOND SET OF SEMANTICS. `ko_threshold`'s
/// rules — what a refusal is, what a non-monotonic curve is, when a threshold
/// counts as verified, majority-of-three — are the expensive, hard-won part of
/// this instrument. A parallel throw search would be a second chance to get
/// each of them subtly wrong, so strikes and throws differ ONLY in how the
/// pulse is delivered and share everything after it.
///
/// Two references, so this is `Copy` and costs a threshold search nothing.
#[derive(Clone, Copy)]
enum Pulse<'a> {
    Strike(&'a HitVolume),
    Throw(&'a CaptureThrowParams),
}

/// THE STAGE'S BLAST ENVELOPE AS THE RUNNING GAME HOLDS IT.
///
/// ⛔⛔ READ OFF THE LIVE `RoomGeometry`, NEVER RESTATED FROM THE DEMO'S
/// CONSTANTS. `FALL_BLAST_MARGIN_PX` and friends are `game/ambition_demo_smash`'s
/// own numbers, and a copy here would be a SECOND stage that silently stops
/// matching the first — the identical mistake `KoProbe::new` already refuses to
/// make for `centre`. It is also the mistake that cost this investigation a
/// session: a gravity constant transcribed from `platformer_defaults.ron` (2250)
/// described a body the engine was accelerating at 1450, and every conclusion
/// built on it was wrong.
#[derive(Clone, Copy)]
struct Blast {
    size: ambition_platformer2d::engine_core::Vec2,
    fall: f32,
    side: Option<f32>,
    rise: Option<f32>,
}

impl Blast {
    /// WHICH LINE THE BODY CROSSED — the question `HitSource::LeftTheWorld`
    /// cannot answer.
    ///
    /// ⛔ THE KERNEL'S OWN ARITHMETIC, transcribed from `apply_world_hazard_gate`
    /// rather than re-invented: clamp into the world box, project the excess onto
    /// the frame's down/side axes, and compare against the per-axis margin. A
    /// second formula here would be free to disagree with the one that actually
    /// killed the body, which would make this a story about a KO rather than a
    /// measurement of one. Gravity is screen-down on this stage, so `down` is +y.
    fn classify(&self, pos: ambition_platformer2d::engine_core::Vec2) -> &'static str {
        let clamped_x = pos.x.clamp(0.0, self.size.x);
        let clamped_y = pos.y.clamp(0.0, self.size.y);
        let past_fall = pos.y - clamped_y;
        let past_side = (pos.x - clamped_x).abs();
        // Order mirrors the gate: fall always kills, side and rise are opt-in.
        if past_fall > self.fall {
            "fall"
        } else if self.side.is_some_and(|m| past_side > m) {
            if pos.x > clamped_x {
                "side-right"
            } else {
                "side-left"
            }
        } else if self.rise.is_some_and(|m| -past_fall > m) {
            "rise"
        } else {
            // ⛔ NOT "unknown" AS A SHRUG. The body was declared out by the engine
            // and yet sits inside every margin this stage declares, which means
            // the position sampled here is not the position the gate judged.
            "INSIDE-ALL-MARGINS-sample-disagrees-with-gate"
        }
    }
}

struct KoProbe {
    app: bevy::prelude::App,
    attacker: bevy::prelude::Entity,
    victim: bevy::prelude::Entity,
    centre: f32,
    tumble_speed: f32,
    victim_weight: f32,
    /// The live blast envelope, or `None` when the running app publishes no
    /// `RoomGeometry` at all — reported as absence rather than back-filled from
    /// the demo's constants, because a probe that substitutes a plausible number
    /// for a missing one cannot tell you it was missing.
    blast: Option<Blast>,
    /// How many trials had to prise the victim off a ledge before they could
    /// start. ⭐ THE CONFIRMING MEASUREMENT FOR THE REGIME-2 DIAGNOSIS: the ledge
    /// hang is a HYPOTHESIS until this counter moves, and a fix that silently
    /// worked would leave me claiming a mechanism I never observed.
    ledge_releases: usize,
}

impl KoProbe {
    fn new(attacker_id: &str, victim_id: &str) -> Self {
        use ambition_platformer2d::characters::brain::Brain;

        let staged = probe_stage::stage(probe_stage::StageRequest {
            cast: [attacker_id, victim_id],
            demo_host: false,
            rendered: false,
        });
        let probe_stage::Staged {
            mut app,
            seat0,
            seat1,
        } = staged;

        // ⛔ STOCKS FIRST, AND IT IS A CORRECTNESS PRECONDITION rather than a
        // speed trick: `STARTING_STOCKS` is 3, and Smash's
        // `take_eliminated_fighters_out_of_play` DESPAWNS an eliminated body —
        // so the fourth probe KO would query a destroyed entity.
        for body in [seat0, seat1] {
            if let Some(mut stocks) = app
                .world_mut()
                .get_mut::<ambition_platformer2d::actor::FighterStocks>(body)
            {
                stocks.remaining = 9_999;
                stocks.started_with = 9_999;
            }
            // ⛔ BOTH brains, not just seat 0. A live CPU victim is what broke
            // the first version of `ring_out`: it walked during the window and
            // something reset its meter mid-reading.
            app.world_mut().entity_mut(body).insert(Brain::stand_still());
        }
        app.update();

        // ⭐⭐ OPTIONAL: RUN THE MATRIX WITH THE GROWTH CURVE OFF (`identity`).
        //
        // The review's standing ruling is that `GrowthBaseCurve` is EVIDENCE and
        // not something to ship, and that calibration needs a curve-free floor to
        // work from. A flag rather than an edit-and-rebuild because curve-on and
        // curve-off must come from ONE binary: rebuilding between them would put
        // a tree difference inside the comparison, which is exactly the confound
        // that made `envelope_AFTER_law` unusable.
        //
        // ⛔ WRITE THE DECLARATION, NOT THE RESOLVED RESOURCE. `ResolvedCombatTuning`
        // is re-derived from `DeclaredCombatRules` EVERY TICK, so a write to the
        // resolved end is overwritten before the next hit and the run would report
        // the demo's own curve wearing an IDENTITY label.
        //
        // ⛔ AND THE OVERRIDE IS NOT TRUSTED. It is read back THROUGH the fold and
        // asserted. A lever that silently failed would produce a curve-off table
        // identical to the curve-on one, which reads as "the curve does nothing" —
        // the most expensive possible false negative, because the honest response
        // to it would be to raise the exponent.
        if std::env::args().any(|a| a == "identity") {
            {
                let mut declared = app
                    .world_mut()
                    .get_resource_mut::<ambition_platformer2d::combat::rules::DeclaredCombatRules>(
                    )
                    .expect(
                        "the smash experience declares combat rules on entry; without that \
                         resource there is no curve to turn off and this run would be \
                         measuring an undeclared world",
                    );
                // `None` resolves to `GrowthBaseCurve::IDENTITY` — the law exactly
                // as first written, and what every undeclared Ambition room uses.
                declared.growth_base = None;
            }
            // One tick for the projection system to re-fold the declaration.
            app.update();
            let live = app
                .world()
                .resource::<ambition_platformer2d::combat::rules::ResolvedCombatTuning>()
                .growth_base;
            assert_eq!(
                live,
                ambition_platformer2d::combat::rules::GrowthBaseCurve::IDENTITY,
                "the IDENTITY override did not reach the resolved rules: the combat road \
                 still reads {live:?}. Every cell in this run would be the demo's declared \
                 curve labelled as curve-free."
            );
            println!(
                "# ⭐ GrowthBaseCurve OVERRIDDEN TO IDENTITY, verified through the fold \
                 (growth_base={live:?}) — this run is the CURVE-FREE baseline"
            );
        }

        // Read off the REAL stage, the way `ring_out::stage_centre_and_reach`
        // does — never restated as a literal, because `STAGE_SIZE` and the blast
        // margins are the demo's own constants and a copy here would be a second
        // stage that silently stops matching the first.
        let room = ambition_demo_smash::smash_stage();
        let centre = room.world.size.x / 2.0;

        // The victim's OWN tumble threshold, off its live motion model.
        let tumble_speed = app
            .world()
            .get::<ambition_platformer2d::actor::MotionModel>(seat1)
            .and_then(|model| match model {
                ambition_platformer2d::actor::MotionModel::AxisSwept(axis) => {
                    Some(axis.params.abilities.tumble_speed)
                }
                _ => None,
            })
            .unwrap_or(0.0);

        // The divisor the knockback law actually uses, read not restated.
        let victim_weight = app
            .world()
            .get::<ambition_platformer2d::combat::components::CombatTuning>(seat1)
            .map(|t| t.weight)
            .unwrap_or(1.0);

        // ⭐⭐ THE BLAST ENVELOPE FROM THE LIVE WORLD, not from the demo's
        // constants. `RoomGeometry` is the session-root component wrapping the
        // active room's collision world — the same `World` whose `edges` the
        // movement kernel destructures in `apply_world_hazard_gate` to decide
        // `ResetCause::LeftTheWorld`. Asking it is asking the authority.
        let blast = {
            let world = app.world_mut();
            let mut q = world.query::<&ambition_platformer2d::engine_core::RoomGeometry>();
            q.iter(world).next().map(|geometry| Blast {
                size: geometry.0.size,
                fall: geometry.0.edges.fall,
                side: geometry.0.edges.side,
                rise: geometry.0.edges.rise,
            })
        };

        let probe = Self {
            app,
            attacker: seat0,
            victim: seat1,
            centre,
            tumble_speed,
            victim_weight,
            blast,
            ledge_releases: 0,
        };

        // ⭐⭐ EVERY PARAMETER THE KNOCKBACK ARITHMETIC DEPENDS ON, READ OFF THE
        // LIVE VICTIM AND THE LIVE STAGE — printed once per matchup, because these
        // are PER-VICTIM facts and a single global header would quietly describe
        // the wrong body for every matchup after the first.
        //
        // ⛔ NOT ONE OF THESE IS DERIVED FROM A SOURCE CONSTANT. That rule is not
        // fastidiousness: this investigation spent a session on conclusions built
        // from a gravity value transcribed out of a defaults file that did not
        // apply to the body being measured. A number that agrees with the running
        // game is evidence; a number that agrees with the repo is a restatement.
        println!(
            "# victim live facts: weight={:.3} tumble_speed={:.1} {}",
            probe.victim_weight,
            probe.tumble_speed,
            probe.victim_gravity(),
        );
        match probe.blast {
            Some(b) => println!(
                "# stage live blast envelope: size=({:.0},{:.0}) fall={:.0} side={} rise={}",
                b.size.x,
                b.size.y,
                b.fall,
                b.side
                    .map(|v| format!("{v:.0}"))
                    .unwrap_or_else(|| "NONE(not a blast zone)".into()),
                b.rise
                    .map(|v| format!("{v:.0}"))
                    .unwrap_or_else(|| "NONE(rises forever)".into()),
            ),
            // ⛔ LOUD ABSENCE. Without this the KO-boundary column would read
            // "unknown" for every cell and look like a classification result.
            None => println!(
                "# ⛔ stage live blast envelope: NO RoomGeometry IN THE RUNNING APP — \
                 KO boundaries CANNOT be classified this run, and no margin has been \
                 substituted from the demo's constants."
            ),
        }
        probe
    }

    fn pos(&self, body: bevy::prelude::Entity) -> ambition_platformer2d::engine_core::Vec2 {
        self.app
            .world()
            .get::<ambition_platformer2d::platformer::body::BodyKinematics>(body)
            .map(|k| k.pos)
            .unwrap_or_default()
    }

    fn park(&mut self, body: bevy::prelude::Entity, x: f32) {
        probe_stage::pin_grounded_at_rest(
            &mut self.app,
            body,
            ambition_platformer2d::engine_core::Vec2::new(x, 200.0),
        );
    }

    /// THE GRAVITY THIS BODY IS ACTUALLY INTEGRATED AT — read, never named.
    ///
    /// ⛔⛔ THE SINGLE MOST EXPENSIVE ERROR IN THIS INVESTIGATION WAS A GRAVITY
    /// CONSTANT COPIED OUT OF A DEFAULTS FILE. `platformer_defaults.ron` says
    /// 2250 and that is the PLAYER's number; `resolve_body_motion_frames` has two
    /// arms, and a match seat takes the `Without<PlayerEntity>` one, which is
    /// `config.tuning.movement.gravity * surface.gravity_scale` — 1450 here. Every
    /// apex, threshold and "contradiction" derived from 2250 described a body that
    /// does not exist, and one of them was written up as a missing engine
    /// mechanic before the arithmetic was rechecked.
    ///
    /// ⭐ THREE ROADS, PRINTED SIDE BY SIDE, because agreement between them is
    /// the measurement and any one alone is a claim: the resolved frame the
    /// kernel integrates, the configured tuning, and the surface scale that
    /// multiplies it. A capture or a mount sets `gravity_scale` to 0.0, so a body
    /// can be configured at 1450 and accelerating at nothing.
    fn victim_gravity(&self) -> String {
        let w = self.app.world();
        let resolved = w
            .get::<ambition_platformer2d::world::ResolvedMotionFrame>(self.victim)
            .map(|f| f.get().gravity_acceleration());
        let configured = w
            .get::<ambition_platformer2d::actor::ActorConfig>(self.victim)
            .map(|c| c.tuning.movement.gravity);
        let scale = w
            .get::<ambition_platformer2d::engine_core::ActorSurfaceState>(self.victim)
            .map(|s| s.gravity_scale);
        let opt = |v: Option<f32>| {
            v.map(|x| format!("{x:.2}"))
                .unwrap_or_else(|| "ABSENT".into())
        };
        format!(
            "resolved_g=({}) config_g={} gravity_scale={} product={}",
            resolved
                .map(|v| format!("{:.1},{:.1}", v.x, v.y))
                .unwrap_or_else(|| "ABSENT".into()),
            opt(configured),
            opt(scale),
            match (configured, scale) {
                (Some(g), Some(s)) => format!("{:.1}", g * s),
                _ => "ABSENT".into(),
            },
        )
    }

    /// IS THE VICTIM HANGING ON A LEDGE? The regime-2 cause, made visible.
    ///
    /// The hang is policy-private axis maneuver state (ADR 0024), so this reads
    /// it rather than owning it; `knock_off_ledge` is the only thing that clears
    /// it here.
    fn victim_ledge(&self) -> String {
        self.app
            .world()
            .get::<ambition_platformer2d::actor::MotionModel>(self.victim)
            .and_then(|m| match m {
                ambition_platformer2d::actor::MotionModel::AxisSwept(axis) => {
                    axis.state.ledge_grab
                }
                _ => None,
            })
            .map(|g| {
                format!(
                    "HANGING(anchor=({:.1},{:.1}) climbing={} elapsed={:.2})",
                    g.contact.anchor.x, g.contact.anchor.y, g.climbing, g.elapsed
                )
            })
            .unwrap_or_else(|| "none".into())
    }

    /// Every victim fact a trial could INHERIT, in one tab-free field.
    ///
    /// ⛔ NOT A HEALTH CHECK. `reset_trial` already asserts the conditions it
    /// knows to assert; this exists precisely for the fact it does NOT know
    /// about, so it reports state rather than judging it. The differing column
    /// between a pre-KO trial and a post-KO trial is the answer.
    fn snapshot(&self) -> String {
        use ambition_platformer2d::characters::actor::{BodyCombat, BodyHealth};
        let w = self.app.world();
        let v = self.victim;
        let ground = w
            .get::<ambition_platformer2d::engine_core::BodyGroundState>(v)
            .map(|g| g.on_ground)
            .unwrap_or(false);
        let (pos, vel) = w
            .get::<ambition_platformer2d::platformer::body::BodyKinematics>(v)
            .map(|k| (k.pos, k.vel))
            .unwrap_or_default();
        let (hitlag, hitstun, hitstop) = w
            .get::<BodyCombat>(v)
            .map(|c| (c.is_in_hitlag(), c.hitstun_timer, c.hitstop_timer))
            .unwrap_or((false, 0.0, 0.0));
        let dmg = w
            .get::<BodyHealth>(v)
            .map(|h| h.damage_taken())
            .unwrap_or(0);
        let mut flags = Vec::new();
        if w.get::<ambition_platformer2d::combat::stocks::RespawnGrace>(v)
            .is_some()
        {
            flags.push("grace");
        }
        if w.get::<ambition_platformer2d::combat::death_rules::DeathInterlude>(v)
            .is_some()
        {
            flags.push("interlude");
        }
        if w.get::<ambition_platformer2d::combat::stocks::PendingRespawn>(v)
            .is_some()
        {
            flags.push("pending");
        }
        if w.get::<ambition_platformer2d::combat::death_rules::OutOfPlay>(v)
            .is_some()
        {
            flags.push("outofplay");
        }
        if w.get::<ambition_platformer2d::combat::capture::CapturedBy>(v)
            .is_some()
        {
            flags.push("captured");
        }
        let flags = if flags.is_empty() {
            "none".to_string()
        } else {
            flags.join("+")
        };
        format!(
            "ground={ground} pos=({:.1},{:.1}) vel=({:.1},{:.1}) hitlag={hitlag} \
             hitstun={hitstun:.3} hitstop={hitstop:.3} dmg={dmg} flags={flags}",
            pos.x, pos.y, vel.x, vel.y
        )
    }

    /// ⛔ THE CONTRACT. A trial may not begin until every one of these is true,
    /// and each is CHECKED rather than waited-out by tick count.
    fn reset_trial(&mut self, entry_percent: i32, victim_x: f32) -> bool {
        use ambition_platformer2d::characters::actor::{BodyHealth, Invulnerability};

        // ⛔⛔⛔ THE MATCH CAN END UNDERNEATH THE PROBE, AND EVERY ROW AFTER THAT
        // IS FICTION. THIS IS THE WORST FAILURE THIS INSTRUMENT HAS HAD.
        //
        // MEASURED 2026-09-13: of 209 refusals in one matchup, 184 (88%) had
        // `seats_now=0` — NOT ONE `MatchSeat` ENTITY LEFT. The round had ended
        // and the whole cast was despawned, so `self.victim` was a DANGLING
        // HANDLE: `pin_grounded_at_rest` silently early-returned because
        // `get_mut` found no component, `pos()` answered `(0,0)` from
        // `unwrap_or_default()`, the grounded premise failed, and every
        // remaining trial refused forever. That is the snowball.
        //
        // ⛔ AND IT PRINTED A FULL TABLE ANYWAY. Twenty-four `REFUSED@0` throw
        // cells were published as throw measurements when the cast they
        // "measured" did not exist. A refused row is not a neutral blank: it
        // reads as a fact about a MOVE. An instrument that answers about itself
        // in the artifact's own table is worse than one that produces nothing.
        //
        // ⇒ ROWS PRINTED BEFORE THE TIMEOUT ARE REAL and are deliberately kept;
        // everything after is refused. So this ABORTS LOUDLY rather than
        // returning `false` — a `false` here is indistinguishable from an
        // ordinary refusal and would go on filling the table with fiction.
        let seats_now = {
            let w = self.app.world_mut();
            let mut q = w.query::<&ambition_platformer2d::actor::MatchSeat>();
            q.iter(w).count()
        };
        if seats_now == 0 {
            println!(
                "# ⛔⛔ ABORTED — THE MATCH ENDED AND THE CAST WAS DESPAWNED. \
                 Rows ABOVE this line were measured against a live match and stand; \
                 there are no rows below because every later trial would refuse against \
                 a dangling entity handle. The probe simulates more ticks than \
                 SMASH_TIME_LIMIT_TICKS (8*60*60), so the match clock runs out mid-run."
            );
            eprintln!(
                "KO_ABORT: seats_now=0 at x={victim_x:.0} pct={entry_percent} — \
                 match over, cast despawned, table truncated deliberately"
            );
            std::process::exit(2);
        }

        // 1. Let the real respawn lifecycle finish. `respawn_when_the_interlude_
        //    closes` gates on `!DeathInterlude.open()` and then hands back every
        //    fact the spend took, so waiting on the MARKERS is waiting on the
        //    engine's own answer.
        let mut settled = false;
        for _ in 0..600 {
            let w = self.app.world();
            let between_lives = w
                .get::<ambition_platformer2d::combat::death_rules::DeathInterlude>(self.victim)
                .is_some()
                || w.get::<ambition_platformer2d::combat::stocks::PendingRespawn>(self.victim)
                    .is_some()
                || w.get::<ambition_platformer2d::combat::death_rules::OutOfPlay>(self.victim)
                    .is_some();
            // 2. BOTH grace witnesses. `stocks.rs` clears the bit when the clock
            //    expires AND retracts it when the component is removed, so the
            //    two agreeing is a measurement; either alone is a claim. A trial
            //    begun under live grace hits an untouchable victim and reports a
            //    FALSE SURVIVAL — the worst failure this probe can have.
            let protected = w
                .get::<ambition_platformer2d::combat::stocks::RespawnGrace>(self.victim)
                .is_some()
                || w.get::<BodyHealth>(self.victim)
                    .is_some_and(|h| h.health.invulnerable.holds(Invulnerability::RESPAWN));
            let frozen = w
                .get::<ambition_platformer2d::characters::actor::BodyCombat>(self.victim)
                .is_some_and(|c| c.is_in_hitlag() || c.hitstun_timer > 0.0);
            let captured = w
                .get::<ambition_platformer2d::combat::capture::CapturedBy>(self.victim)
                .is_some();
            if !between_lives && !protected && !frozen && !captured {
                settled = true;
                break;
            }
            self.app.update();
        }
        if !settled {
            // ⭐ SAY WHICH CONDITION BLOCKED IT. A refusal that reports only
            // "refused" sends the next reader guessing, and run 3 refused 16 of
            // 60 cells — every one of them a LEDGE cell, clustered where
            // thresholds are low and therefore where nearly every trial ends in
            // a KO. That points at the respawn cycle rather than the position,
            // but pointing is not knowing, so the probe names the marker still
            // standing when its budget ran out.
            let w = self.app.world();
            let mut blocked = Vec::new();
            if w.get::<ambition_platformer2d::combat::death_rules::DeathInterlude>(self.victim)
                .is_some()
            {
                blocked.push("interlude");
            }
            if w.get::<ambition_platformer2d::combat::stocks::PendingRespawn>(self.victim)
                .is_some()
            {
                blocked.push("pending");
            }
            if w.get::<ambition_platformer2d::combat::death_rules::OutOfPlay>(self.victim)
                .is_some()
            {
                blocked.push("outofplay");
            }
            if w.get::<ambition_platformer2d::combat::stocks::RespawnGrace>(self.victim)
                .is_some()
            {
                blocked.push("grace");
            }
            if w.get::<ambition_platformer2d::combat::capture::CapturedBy>(self.victim)
                .is_some()
            {
                blocked.push("captured");
            }
            if w.get::<ambition_platformer2d::characters::actor::BodyCombat>(self.victim)
                .is_some_and(|c| c.is_in_hitlag() || c.hitstun_timer > 0.0)
            {
                blocked.push("frozen");
            }
            eprintln!(
                "KO_REFUSE: stage=settle x={victim_x:.0} pct={entry_percent} blocked=[{}]",
                if blocked.is_empty() {
                    "NONE-none-of-the-tracked-markers".to_string()
                } else {
                    blocked.join("+")
                }
            );
            return false;
        }

        // 3. Place, still, and metered.
        //
        // ⛔⛔⛔ LET GO OF THE LEDGE FIRST, OR THE PIN IS OVERWRITTEN EVERY TICK.
        //
        // MEASURED 2026-09-13. 54 of 67 refusals in a full matchup reported the
        // victim at x=574 or x=66 with `vel=(0,0)`, unmoved across 24+ ticks. The
        // platform spans 80..560 and the body is 30 wide, so those are `560+14`
        // and `80-14` — one pixel of overlap, which `spans_overlap_for_support`
        // rejects at exactly `EDGE_OVERLAP_SLOP`. That is the precise coordinate
        // where support ends: a LEDGE HANG, latched.
        //
        // The hang is a TETHER. It re-asserts its anchor after this fixture's
        // pin, which is why a body pinned at stage centre (320,320) reported
        // (574,320) one tick later — a 254px move that looked like a teleport and
        // was blamed on depenetration for hours. It cannot have been:
        // `is_contact_range_snap` caps any pushout at the body's own half-diagonal
        // (~28px) and BOTH resolution paths filter through it.
        //
        // ⛔ AND IT SNOWBALLS, which is why refusals grew 25 -> 209 across a run
        // and why whatever `launchers_of` pushes LAST (throws) appeared broken.
        // Nothing in a stand-still fixture ever lets go, so once a KO'd fighter
        // catches an edge on the way back, every later trial inherits it.
        //
        // ⭐ `knock_off_ledge` IS THE SANCTIONED RELEASE — the typed
        // combat->movement op over the axis policy's private hang state, the same
        // one a real hit uses. It also arms `LEDGE_KNOCK_OFF_COOLDOWN` (0.35s), so
        // the body cannot immediately re-latch the edge it was just taken off.
        // Reaching into `axis.state.ledge_grab` directly would be a second
        // authority on what leaving a ledge means.
        let released = {
            let world = self.app.world_mut();
            let mut q = world.query::<(
                &mut ambition_platformer2d::actor::MotionModel,
                &mut ambition_platformer2d::engine_core::BodyLedgeState,
            )>();
            match q.get_mut(world, self.victim) {
                Ok((mut model, mut ledge)) => {
                    // ⚠ `&mut *`, NOT `&mut`. `get_mut` hands back `Mut<T>` change
                    // trackers; Rust auto-derefs a method RECEIVER but never a
                    // function ARGUMENT, so `&mut model` is `&mut Mut<MotionModel>`
                    // and does not satisfy `&mut MotionModel`.
                    ambition_platformer2d::engine_core::movement::knock_off_ledge(
                        &mut *model,
                        &mut *ledge,
                    )
                }
                Err(_) => false,
            }
        };
        if released {
            self.ledge_releases += 1;
            // ⭐⭐ THE CONFIRMING MEASUREMENT, AND IT MUST BE EMITTED.
            //
            // The ledge-hang diagnosis is a HYPOTHESIS until this line appears in
            // a run. The danger is specific: if the release works, the refusals it
            // was built to remove DISAPPEAR, and a silent counter would leave me
            // reporting a mechanism nothing ever observed — a fix whose success is
            // indistinguishable from the bug never having existed. `knock_off_ledge`
            // returns true only when it actually took a hang away, so each of these
            // lines is one trial that WAS latched and now is not.
            let p = self.pos(self.victim);
            eprintln!(
                "KO_LEDGE_RELEASE: n={} x={victim_x:.0} pct={entry_percent} \
                 was_at=({:.1},{:.1})",
                self.ledge_releases, p.x, p.y,
            );
        }
        self.park(self.victim, victim_x);
        self.park(self.attacker, victim_x - 240.0);
        // ⭐ DID THE LANDING LOOP EVER SUCCEED? `landed_y` below is read
        // UNCONDITIONALLY, so a loop that ran out of budget hands the pin a
        // MID-FALL height and the grounded premise then refuses a body the
        // fixture itself put in the air. Recording the tick separates "landed
        // and was moved afterwards" from "never landed at all" — two different
        // defects that produce the same refusal.
        let mut landed_after: Option<usize> = None;
        // ⭐ THE DESCENT ITSELF, RECORDED — because `landed_after=None` on 209/209
        // refusals says the loop never saw a landing, and NOTHING says why.
        //
        // ⛔⛔ THE COMMENT THAT STOOD HERE USED GRAVITY 2250 AND WAS WRONG, and it
        // was wrong in the direction that hid the answer. 2250 is the PLAYER's
        // gravity (`platformer_defaults.ron`); a match seat is an ACTOR body, and
        // `resolve_body_motion_frames` gives the `Without<PlayerEntity>` arm
        // `config.tuning.movement.gravity * surface.gravity_scale`, which is
        // `BodyMovementTuning::BASELINE.gravity` = 1450 at `gravity_scale` 1.0.
        //
        // ⭐ MEASURED, not re-derived from a second constant. Seven refusals in
        // this very probe caught the victim in free fall one tick after the pin:
        //     dy = 0.4000px  => g = dy * 3600 = 1440
        //     vel_y = 24.2   => g = vel * 60  = 1452
        // Two independent channels (displacement and velocity), seven samples,
        // both landing on 1450 through one-decimal printing, and both consistent
        // only if one `app.update()` is exactly one sim tick — which it is.
        //
        // ⇒ THE TRAIL'S ORIGINAL QUESTION IS ANSWERED AND IT WAS NOT A DESCENT.
        // The body does not fall through the platform: it does not fall AT ALL.
        // 21 of 25 trails read one identical pose for every sample, and the pose
        // is x=574 or x=66 — the platform spans 80..560 and the body is 30 wide,
        // so those are `560+14` and `80-14`: one pixel of overlap left, which
        // `spans_overlap_for_support` rejects at exactly `EDGE_OVERLAP_SLOP`.
        // A body frozen at zero velocity on the precise pixel where support ends
        // is a LEDGE HANG, and the tether re-asserts its anchor over this
        // fixture's pin every tick. See the release below.
        //
        // ⛔ A TRAIL, NOT A GUESS. Six hypotheses in this investigation have died
        // by measurement — including the depenetration story this trail was built
        // to support, which `is_contact_range_snap` refuses outright: it caps any
        // pushout at the body's own half-diagonal (~28px), and the observed move
        // is 254px. No collision code in this engine can produce it.
        let mut trail: Vec<(f32, f32)> = Vec::new();
        for tick in 0..40 {
            self.park(self.attacker, victim_x - 240.0);
            self.app.update();
            // Dense at the start (where a 16-tick fall lives), sparse after.
            if tick < 8 || tick % 8 == 0 {
                let p = self.pos(self.victim);
                trail.push((p.x, p.y));
            }
            let grounded = self
                .app
                .world()
                .get::<ambition_platformer2d::engine_core::BodyGroundState>(self.victim)
                .is_some_and(|g| g.on_ground);
            if grounded {
                landed_after = Some(tick);
                break;
            }
        }
        // ⛔⛔ DO NOT `park` AGAIN HERE — MEASURED 2026-09-13, AND THIS WAS THE
        // DEFECT UNDER RUN 1'S WHOLE TABLE.
        //
        // `park` places a body at y = 200.0, which is ABOVE the platform surface
        // (PLATFORM_TOP is 300). The loop above spends up to forty ticks waiting
        // for the victim to LAND, and the previous form then lifted it straight
        // back into the air and ran one update — so every trial began airborne,
        // and `ground=` varied with where that single tick left it. A body
        // struck in the air takes a different road than one struck standing,
        // which is why four identical trials alternated survive/KO on an
        // IDENTICAL resolved launch of 403.0, and why the oracle row's centre
        // threshold moved 291 -> 283 when a coarser step reshuffled the order.
        //
        // Correct x WITHOUT restoring the height the landing just resolved.
        //
        // ⛔⛤ AND THROUGH THE PIN, DELIBERATELY NOT THROUGH `transit_body`: this
        // body is being slid along a floor it is ALREADY STANDING ON, and the
        // grounded premise asserted just below is the entire point of the
        // reset. The transit authority would clear that contact by design. See
        // `pin` for the measurement that settled it.
        //
        // The `y` is read back from the body the loop above landed, so only the
        // `x` is the trial's to choose — the pin takes a full pose, and handing
        // it anything else would restore the height this comment forbids.
        let landed_y = self.pos(self.victim).y;
        probe_stage::pin_grounded_at_rest(
            &mut self.app,
            self.victim,
            ambition_platformer2d::engine_core::Vec2::new(victim_x, landed_y),
        );
        // ⭐ THE POSE THE PIN ACTUALLY ACHIEVED, READ BEFORE ANY TICK RUNS.
        //
        // ⛔ THIS SEPARATES TWO COMPLETELY DIFFERENT DEFECTS that the refusal
        // line could not tell apart. The pin is called with `(victim_x,
        // landed_y)` and the refusal then reports `pos.x = 574` for a
        // `victim_x` of 320 — so EITHER the write never reached the entity being
        // read (a stale/duplicate victim), OR it landed and something overrode
        // it during the update that follows. Sampling here, with no `update()`
        // in between, is the only way to say which.
        let pinned_at = self.pos(self.victim);
        self.app.update();

        // ⭐ AND THE PREMISE IS NOW CHECKED RATHER THAN HOPED FOR. A trial that
        // cannot start the victim standing still on the floor does not start.
        let grounded = self
            .app
            .world()
            .get::<ambition_platformer2d::engine_core::BodyGroundState>(self.victim)
            .is_some_and(|g| g.on_ground);
        if !grounded {
            // ⛔⛤ THIS IS THE REFUSAL THAT ACTUALLY FIRES, AND IT SAID NOTHING.
            //
            // MEASURED 2026-09-13: across the strike matrix AND the stage-3 run —
            // between them well over a hundred refused cells — the settle stage
            // above logged `KO_REFUSE` exactly ZERO times. Every refusal this
            // instrument has ever printed came through HERE, and this arm was a
            // bare `return false`, so the one explanation a reader needed was the
            // only one never written down. The careful message upstream describes
            // a path that has never been taken.
            //
            // ⛔ An empty `KO_REFUSE` grep therefore proved NOTHING about a
            // refused cell, which is worse than no instrumentation at all: it
            // reads as "the reset was fine" to anyone who checks.
            let p = self.pos(self.victim);
            // ⭐ THE SUSPECT, MEASURED RATHER THAN ASSUMED: A STAGED LAUNCH THE
            // PIN CANNOT SEE.
            //
            // `constrain_body_pose` writes `pos` and `vel` and nothing else — by
            // contract it "does not fabricate or clear contact facts" — so a
            // launch STAGED by the previous trial survives the park untouched.
            // `kernel.rs` says so in as many words: *"the launch stays staged.
            // `PendingLaunch` survives until a tick on which nobody else owns the
            // pose, and the kernel spends it then."* That tick is this reset's
            // own `update()`, which would fling a victim the fixture had just set
            // down at rest — and a refusal reporting `pos=(574,320)` off the
            // platform's right edge (it spans 80..560) is what that looks like.
            //
            // ⛔ `pending_launch_state`, NOT `take_launch`. The read-only form
            // exists for exactly this: draining it here would SPEND the launch
            // and destroy the evidence, and the next trial would then quietly
            // differ from the one that produced the number printed below.
            // ⛔ DUMP THE STATE, DO NOT GUESS AT IT AGAIN. Three hypotheses have
            // now been falsified by measurement — a missing component, a cleared
            // message, and the staged launch this very line was added to test —
            // and each cost a full run because the output could not tell the
            // candidates apart. The remaining suspects are all cheap to PRINT:
            //
            //  * `carried_run`/`carried_hold` — same class as the staged launch
            //    (BodyFlightState survives `constrain_body_pose`, which writes
            //    only pos/vel), and the airborne law does
            //    `approach(along, *carried_run, ..)`, which would ACCELERATE a
            //    body the fixture just set to rest.
            //  * `contact_initialized` — the pin "does not fabricate or clear
            //    contact facts" by contract, so an invalidated baseline survives
            //    it and `on_ground` reads false for a body genuinely resting on
            //    the floor. That would refuse regardless of position.
            //  * `landed_after == None` — the loop never saw a landing, so
            //    `landed_y` was sampled mid-fall.
            let (staged, carried_run, carried_hold) = self
                .app
                .world()
                .get::<ambition_platformer2d::engine_core::BodyFlightState>(self.victim)
                .map(|f| {
                    (
                        f.pending_launch_state().velocity,
                        f.carried_run,
                        f.carried_hold,
                    )
                })
                .unwrap_or_default();
            let vel = self
                .app
                .world()
                .get::<ambition_platformer2d::engine_core::BodyKinematics>(self.victim)
                .map(|k| k.vel)
                .unwrap_or_default();
            let (on_ground, contact_init) = self
                .app
                .world()
                .get::<ambition_platformer2d::engine_core::BodyGroundState>(self.victim)
                .map(|g| (g.on_ground, g.contact_initialized))
                .unwrap_or_default();
            // ⭐ IS THE PROBE STILL HOLDING THE RIGHT BODY? A KO respawns a
            // fighter, and if the ruleset respawns it as a NEW ENTITY then
            // `self.victim` is a stale handle: its pose would be frozen wherever
            // it died (off the ledge, at rest, never landing — exactly shapes A
            // and B), and EVERY later trial in the run would refuse, which is
            // the snowball actually observed. Reported, not assumed.
            let seat = self
                .app
                .world()
                .get::<ambition_platformer2d::actor::MatchSeat>(self.victim)
                .map(|s| s.0 as i32)
                .unwrap_or(-1);
            let seats_now = {
                let w = self.app.world_mut();
                let mut q = w.query::<&ambition_platformer2d::actor::MatchSeat>();
                q.iter(w).count()
            };
            eprintln!(
                "KO_REFUSE: stage=ground x={victim_x:.0} pct={entry_percent} \
                 pinned_at=({:.1},{:.1}) pos=({:.1},{:.1}) vel=({:.1},{:.1}) \
                 landed_y={landed_y:.1} landed_after={landed_after:?} \
                 on_ground={on_ground} contact_init={contact_init} \
                 staged=({:.1},{:.1}) carried_run={carried_run:.1} \
                 carried_hold={carried_hold:.3} seat={seat} seats_now={seats_now} \
                 ledge={} {}",
                pinned_at.x,
                pinned_at.y,
                p.x,
                p.y,
                vel.x,
                vel.y,
                staged.x,
                staged.y,
                // ⭐ THE TWO FACTS THAT WERE MISSING WHILE FIVE HYPOTHESES DIED.
                // A frozen body is either held by something (ledge) or not being
                // accelerated (gravity_scale) — and neither was ever printed, so
                // every refusal looked equally mysterious.
                self.victim_ledge(),
                self.victim_gravity(),
            );
            // ⭐ AND THE DESCENT THAT FAILED, on its own line so the one above
            // stays parseable. Parked at y=200; a resting body is y=276; the
            // platform solid spans y 300..332. Where this trail crosses those
            // numbers — or refuses to — is the whole question.
            if landed_after.is_none() {
                let path: Vec<String> = trail
                    .iter()
                    .map(|(x, y)| format!("({x:.0},{y:.0})"))
                    .collect();
                eprintln!(
                    "KO_REFUSE_TRAIL: x={victim_x:.0} pct={entry_percent} \
                     parked=(.,200) resting_y=276 platform_y=300..332 path=[{}]",
                    path.join(" ")
                );
            }
            return false;
        }

        for (body, meter) in [(self.victim, entry_percent), (self.attacker, attacker_meter())] {
            // ⛔ THE ATTACKER IS PINNED TO ZERO because `rage_scale` reads its
            // meter and multiplies EVERY resolved launch. The sweep that picked
            // the shipped percent scale ran with an uncontrolled ~1.25x rage.
            if let Some(mut health) = self.app.world_mut().get_mut::<BodyHealth>(body) {
                health.set_damage_taken(meter);
            }
        }
        self.app.update();
        true
    }

    /// Fire one authored volume at the parked victim and read the engine's own
    /// verdict.
    fn fire(&mut self, pulse: Pulse<'_>, entry_percent: i32, victim_x: f32) -> Option<Trial> {
        use ambition_platformer2d::combat::events::{HitEvent, HitKnockbackMagnitude, HitTarget};
        use ambition_platformer2d::combat::stocks::BodyKnockedOut;
        use ambition_platformer2d::engine_core::Vec2 as EVec2;

        if !self.reset_trial(entry_percent, victim_x) {
            return None;
        }
        // ⭐ THE STATE THE TRIAL ACTUALLY BEGINS IN, captured AFTER the reset.
        // The first version of this diagnostic snapshotted before `reset_trial`
        // and therefore reported the PREVIOUS trial's leftovers — informative
        // about what carries over, and silent about the only thing that decides
        // an outcome, which is the state at the moment of the strike.
        // ⛔⛔ THE TRIAL'S PREMISE, ASSERTED RATHER THAN HOPED FOR.
        //
        // `reset_trial` proves the victim is GROUNDED. It does not prove the
        // victim is UNENCUMBERED, and those are different claims: a staged launch,
        // carried run/hold momentum, or a latched ledge all survive
        // `constrain_body_pose` (which writes pos and vel and nothing else) and
        // all three would silently change what the next pulse measures.
        //
        // ⛔ AND IT ABORTS RATHER THAN REFUSING. A `return None` here is
        // indistinguishable from an ordinary refusal and would go on filling the
        // table — which is precisely how 24 `REFUSED@0` throw cells were once
        // published as measurements of moves. The existing `seats_now == 0` guard
        // takes the same road for the same reason. If this fires, the matrix is
        // not salvageable and a truncated honest table beats a complete false one.
        {
            let (staged, carried_run, carried_hold) = self
                .app
                .world()
                .get::<ambition_platformer2d::engine_core::BodyFlightState>(self.victim)
                .map(|f| {
                    (
                        f.pending_launch_state().velocity,
                        f.carried_run,
                        f.carried_hold,
                    )
                })
                .unwrap_or_default();
            let ledge = self.victim_ledge();
            // Generous, because this is a CONTAMINATION test and not a precision
            // one: anything this small cannot move a body meaningfully in the
            // ticks before the pulse lands, and a tighter bound would fail on
            // ordinary float residue.
            const NEGLIGIBLE: f32 = 1.0;
            let dirty = staged.length() > NEGLIGIBLE
                || carried_run.abs() > NEGLIGIBLE
                || carried_hold.abs() > 0.01
                || ledge != "none";
            if dirty {
                println!(
                    "# ⛔⛔ ABORTED — A TRIAL WAS ASKED TO BEGIN ON A CONTAMINATED \
                     VICTIM. Rows above this line began clean and stand; there are no \
                     rows below because every later cell would measure leftovers of \
                     the trial before it."
                );
                eprintln!(
                    "KO_ABORT: dirty-trial-start x={victim_x:.0} pct={entry_percent} \
                     staged=({:.2},{:.2}) carried_run={carried_run:.2} \
                     carried_hold={carried_hold:.3} ledge={ledge}",
                    staged.x, staged.y
                );
                std::process::exit(2);
            }
        }

        let at_strike = self.snapshot();
        let struck_at = self.pos(self.victim);
        let attacker = self.attacker;

        // ⛔⛔ CURSORS MADE **ONCE, HERE**, AND ADVANCED INSIDE THE LOOP.
        //
        // This is the whole defence against a previous trial's knockout being
        // credited to this one. A cursor created now is already positioned past
        // everything the buffer holds, so the reads below see only what this
        // pulse produces; a cursor created per tick would re-read from the
        // buffer's START every tick and rediscover every earlier trial's KO.
        //
        // ⚠ There is no `get_cursor_from_end` in bevy_ecs 0.19.1 — `get_cursor`
        // is the only constructor, and every call site in this repo uses it.
        // "Made once, before the loop" IS the end-of-stream guarantee; it is a
        // discipline rather than an API, which is exactly why it is written down
        // here instead of assumed.
        let mut hit_cursor = self
            .app
            .world()
            .resource::<bevy::ecs::message::Messages<HitEvent>>()
            .get_cursor();
        let mut ko_cursor = self
            .app
            .world()
            .resource::<bevy::ecs::message::Messages<BodyKnockedOut>>()
            .get_cursor();

        // ⭐⭐ DELIVERY IS THE ONLY THING THAT VARIES. The cursors above are
        // already watching, and the settle loop below is already shared — see
        // `Pulse`.
        //
        // ⛔ THE THROW DOES NOT SPEND ITS OWN TICK HERE. It installs the hold
        // and WRITES the request; the loop's first `self.app.update()` is what
        // executes it, so the launch's `HitEvent` lands in front of a cursor
        // that already exists. An `update()` inside this arm would risk the
        // event being published before the loop began watching for it.
        let spawned: Option<bevy::prelude::Entity> = match pulse {
            Pulse::Strike(hit) => Some(
                self.app
                    .world_mut()
                    .spawn((
                        ambition_platformer2d::combat::strike::Hitbox {
                            owner: attacker,
                            source: ambition_platformer2d::vfx::HitSide::Enemy,
                            anchor: ambition_platformer2d::combat::strike::HitboxAnchor::World {
                                center: struck_at,
                            },
                            half_extent: EVec2::new(48.0, 48.0),
                            shape: None,
                            facing: 1.0,
                            damage: hit.damage,
                            knockback:
                                ambition_platformer2d::combat::strike::HitboxKnockback::LaunchSpeed {
                                    base: hit.knockback,
                                    growth: hit.knockback_growth,
                                },
                            launch_dir: hit.launch_dir.map(|(x, y)| EVec2::new(x, y)),
                            frame_down: EVec2::new(0.0, 1.0),
                            strike_sfx: None,
                            reaction: None,
                        },
                        ambition_platformer2d::combat::strike::HitboxHits::default(),
                        ambition_platformer2d::combat::strike::HitboxLifetime { remaining_s: 0.1 },
                    ))
                    .id(),
            ),
            Pulse::Throw(params) => {
                // ⛔⛔⛔ A THROW HAPPENS AT THE CAPTOR, NOT AT THE VICTIM'S PARKED x.
                //
                // MEASURED 2026-09-13, and it made a published column a lie.
                // Installing `CapturedBy` does not close a distance: the hold
                // system constrains the captive to `captor.pos + hold_offset`
                // EVERY TICK, so the victim is teleported to the attacker before
                // the throw resolves. `reset_trial` parks the attacker at
                // `victim_x - 240`, so a "centre" throw (victim_x = 320) actually
                // fired from x ~= 96 — which IS the left ledge — and a "ledge"
                // throw (520) fired from mid-platform at ~296.
                //
                // ⇒ The centre/ledge columns for throws reported WHERE THE VICTIM
                // WAS PARKED while the throw happened 240px away. back_throw read
                // centre=11% and ledge=91%: a centre KO EASIER than a ledge one,
                // which is backwards and is what exposed it. At 11% that throw
                // launches 175 px/s — nowhere near enough to cross the stage — and
                // the KO exits were landing at x = -405..-409, far off the LEFT
                // side. Both facts are impossible from x=320 and inevitable from
                // x~=96.
                //
                // ⭐ THE STRIKE ROWS WERE NEVER AFFECTED: a strike spawns its
                // hitbox at `struck_at`, the victim's own pose, so the attacker's
                // placement cannot reach them. And the throw LAUNCH column is
                // unaffected too — it is a velocity observed at release, which no
                // position changes.
                //
                // The fix is to put the captor where the trial says the throw
                // happens. `hold_offset_local.x` is 16, so parking the attacker at
                // `victim_x - 16` lands the hold anchor on `victim_x` itself.
                //
                // ⛔ NOT `park`, WHICH FORCES y = 200 — that is 76px above a
                // resting body and would throw from mid-air. The attacker's
                // settled height is read back and preserved, so only x moves.
                let anchor_x = victim_x - 16.0;
                let attacker_y = self.pos(attacker).y;
                probe_stage::pin_grounded_at_rest(
                    &mut self.app,
                    attacker,
                    EVec2::new(anchor_x, attacker_y),
                );
                // ⚠ `lasting`, NOT `default()`. A default `SmashHoldState` has
                // `escape_seconds == 0.0`, which `escaped()` correctly reads as
                // a hold ALREADY OVER — its own doc warns that a fixture
                // reaching for `default()` watches its capture end on tick one
                // and calls that a timeout.
                self.app.world_mut().entity_mut(self.victim).insert((
                    ambition_platformer2d::combat::capture::CapturedBy {
                        captor: attacker,
                        hold_offset_local: EVec2::new(16.0, 0.0),
                        prior_gravity_scale: 1.0,
                    },
                    ambition_platformer2d::characters::control::ScriptedControl,
                    ambition_platformer2d::characters::control::ControlHolds::only(
                        ambition_platformer2d::characters::control::ControlHold::Relationship,
                    ),
                    ambition_platformer2d::characters::smash_hold_state::SmashHoldState::lasting(
                        10.0,
                    ),
                ));
                self.app.world_mut().write_message(
                    ambition_platformer2d::combat::capture::CaptureThrowRequested {
                        captor: attacker,
                        damage: params.damage,
                        knockback: params.knockback,
                        knockback_growth: params.knockback_growth,
                        launch_dir: EVec2::new(params.launch_dir.0, params.launch_dir.1),
                    },
                );
                None
            }
        };

        let mut resolved_launch = None;
        let mut ko = false;
        let mut ticks_to_ko = None;
        let mut ko_forced = None;
        // ⭐ THE TRIAL ENDS WHEN IT IS DECIDED, NOT WHEN THE BUDGET RUNS OUT.
        //
        // Measured 2026-09-13: a SURVIVING trial spent all 150 ticks watching a
        // body that had already come to rest, and at ~8.9ms/frame that budget —
        // not the simulation — was the whole cost of this instrument (~170s per
        // pulse row, ~5.7h for the matrix, against a 90m timeout).
        //
        // ⛔ THE ARMING CONDITIONS ARE THE CORRECTNESS ARGUMENT, and each one
        // excludes a way a body can be still WITHOUT the trial being over:
        //   * `resolved_launch.is_some()` — before contact the victim is parked
        //     and motionless, which looks exactly like "come to rest".
        //   * `!is_in_hitlag()` — impact hitstop FREEZES both bodies. A victim
        //     mid-freeze is perfectly still and has not yet travelled anywhere.
        //   * `hitstun_timer <= 0` — still being carried by the launch.
        //   * grounded and slow, for `SETTLED_TICKS` CONSECUTIVE ticks, so a
        //     single frame of ground contact mid-arc cannot end the trial.
        // A body that satisfies all four has been launched, has finished its
        // launch, and is standing on the floor: it cannot reach a blast line.
        const SETTLED_TICKS: usize = 4;
        const SLOW_PX_S: f32 = 12.0;
        let mut settled_for = 0usize;
        // ⛔⛤ A THROW PUBLISHES NO `HitEvent`, SO THE CURSOR ABOVE NEVER SEES IT.
        //
        // MEASURED 2026-09-13: `apply_capture_throws` calls
        // `apply_body_hit_reaction` DIRECTLY — that function mutates velocity and
        // publishes nothing, and the only `MessageWriter`s in `capture/systems.rs`
        // emit capture REQUESTS. So for a throw `resolved_launch` stayed `None`
        // forever, which is why every throw row printed `-` for `launch@100` and
        // `tumble%`, and why `launch_at` returned `None` on every call.
        //
        // ⇒ Watch for the RELEASE instead. The captive is released BY the throw,
        // so the first tick `CapturedBy` is gone is the tick the launch was
        // applied, and the victim's velocity right then IS the launch.
        let mut awaiting_release = matches!(pulse, Pulse::Throw(_));
        for tick in 0..150 {
            self.app.update();
            if resolved_launch.is_none() {
                let messages = self.app.world().resource::<bevy::ecs::message::Messages<HitEvent>>();
                for ev in hit_cursor.read(messages) {
                    if !matches!(ev.target, HitTarget::Body(e) if e == self.victim) {
                        continue;
                    }
                    if let Some(kb) = ev.knockback.as_ref() {
                        if let HitKnockbackMagnitude::LaunchSpeed(v) = kb.magnitude {
                            resolved_launch = Some(v);
                            break;
                        }
                    }
                }
            }
            // ⭐ OBSERVE THE THROW'S LAUNCH, DO NOT RECOMPUTE IT.
            //
            // ⛔ The tempting alternative is to evaluate the launch law on the
            // authored `CaptureThrowParams` and report that. It would be a
            // FABRICATION dressed as a measurement: the probe would print the
            // formula's own prediction and agree with itself no matter what the
            // engine actually did, which is exactly the failure the whole
            // instrument exists to avoid.
            //
            // ⚠ AND THE NUMBER IS ONE TICK LATE, stated rather than hidden. The
            // same `update()` that released the captive also ran gravity, so this
            // reads the launch minus one frame of it, and ONLY on the vertical
            // component. It is NOT silently compensated: a correction would be a
            // second model layered over a measurement, and the residual is small
            // against launches of 500-1200.
            //
            // ⛔ THIS COMMENT SAID "about 37 px/s at GRAVITY 2250 / 60Hz" AND THAT
            // WAS FALSE FOR EVERY BODY THIS PROBE HAS EVER MEASURED. 2250 is the
            // PLAYER's gravity; a match seat is an actor body accelerated at
            // `tuning.movement.gravity * gravity_scale` = 1450, so the lag is
            // ~24 px/s, not ~37. The probe now READS the live resolved frame
            // (`victim_gravity`) instead of naming any constant at all — a number
            // transcribed from a defaults file is not a measurement of the body
            // in front of you, and this one was wrong by 55%.
            if awaiting_release
                && self
                    .app
                    .world()
                    .get::<ambition_platformer2d::combat::capture::CapturedBy>(self.victim)
                    .is_none()
            {
                awaiting_release = false;
                if resolved_launch.is_none() {
                    resolved_launch = self
                        .app
                        .world()
                        .get::<ambition_platformer2d::engine_core::BodyKinematics>(self.victim)
                        .map(|k| k.vel.length());
                }
            }
            // ⭐ THE LAST POSE THE BODY HELD WHILE STILL IN PLAY, sampled BEFORE
            // the knockout test. A KO'd fighter is taken out of play and
            // respawned, so a pose read after the event describes where it came
            // BACK, not where it left — which would classify every kill as
            // whatever boundary the respawn point sits inside.
            let live_pos = self.pos(self.victim);
            // ⭐ SAMPLED BESIDE `live_pos`, AND FOR EXACTLY ITS REASON. A KO'd body
            // is taken out of play and respawned, so hitstun and tumble read AFTER
            // the knockout event describe the body that came BACK — which carries
            // neither, and would score every kill as an un-forced fall.
            //
            // ⛔ `tumble_until_landing` IS DELIBERATELY ABSENT FROM THIS PREDICATE.
            // It outlives helplessness: control returns before the tumble does, so a
            // victim still flagged by it may have been able to act. Including it
            // would count recoverable falls as forced kills, which is the precise
            // overstatement this field exists to end. `tumble_timer` is the
            // helpless part, and it is the one asked here.
            let live_forced = {
                let w = self.app.world();
                let stunned = w
                    .get::<ambition_platformer2d::characters::actor::BodyCombat>(self.victim)
                    .is_some_and(|c| c.is_in_hitlag() || c.hitstun_timer > 0.0);
                let tumbling = w
                    .get::<ambition_platformer2d::actor::MotionModel>(self.victim)
                    .is_some_and(|m| match m {
                        ambition_platformer2d::actor::MotionModel::AxisSwept(axis) => {
                            axis.state.tumble_timer > 0.0
                        }
                        _ => false,
                    });
                stunned || tumbling
            };
            if !ko {
                let messages = self
                    .app
                    .world()
                    .resource::<bevy::ecs::message::Messages<BodyKnockedOut>>();
                if ko_cursor.read(messages).any(|k| {
                    k.body == self.victim
                        && matches!(
                            k.cause,
                            ambition_platformer2d::combat::HitSource::LeftTheWorld
                        )
                }) {
                    ko = true;
                    ticks_to_ko = Some(tick);
                    ko_forced = Some(live_forced);
                    // ⭐⭐ WHICH LINE IT CROSSED. `HitSource::LeftTheWorld` says a
                    // body is gone and NOTHING about the direction, so a vertical
                    // kill and a horizontal one have been indistinguishable in
                    // every table this probe has ever printed — while the whole
                    // question under investigation is whether upward launches
                    // reach the ceiling.
                    //
                    // ⛔ ON STDERR, NOT AS A COLUMN. Three gates diff this table
                    // byte-for-byte against `envelope_AFTER_LAWONLY`; widening it
                    // would destroy the only comparison road still standing.
                    match self.blast {
                        Some(b) => eprintln!(
                            "KO_BOUNDARY: boundary={} x={victim_x:.0} pct={entry_percent} \
                             tick={tick} forced={live_forced} last_live=({:.1},{:.1})",
                            b.classify(live_pos),
                            live_pos.x,
                            live_pos.y,
                        ),
                        None => eprintln!(
                            "KO_BOUNDARY: boundary=UNCLASSIFIABLE-no-RoomGeometry \
                             x={victim_x:.0} pct={entry_percent} tick={tick} \
                             last_live=({:.1},{:.1})",
                            live_pos.x, live_pos.y,
                        ),
                    }
                }
            }
            if ko {
                break;
            }
            if resolved_launch.is_some() {
                let w = self.app.world();
                let frozen = w
                    .get::<ambition_platformer2d::characters::actor::BodyCombat>(self.victim)
                    .is_some_and(|c| c.is_in_hitlag() || c.hitstun_timer > 0.0);
                let grounded = w
                    .get::<ambition_platformer2d::engine_core::BodyGroundState>(self.victim)
                    .is_some_and(|g| g.on_ground);
                let slow = w
                    .get::<ambition_platformer2d::platformer::body::BodyKinematics>(self.victim)
                    .is_some_and(|k| k.vel.x.abs() < SLOW_PX_S && k.vel.y.abs() < SLOW_PX_S);
                if !frozen && grounded && slow {
                    settled_for += 1;
                    if settled_for >= SETTLED_TICKS {
                        let _ = tick;
                        break;
                    }
                } else {
                    settled_for = 0;
                }
            }
        }
        if let Some(strike) = spawned {
            if self.app.world().get_entity(strike).is_ok() {
                self.app.world_mut().entity_mut(strike).despawn();
            }
        }

        // ⛔⛤ A THROW THAT NEVER EXECUTED IS A REFUSAL, NOT A SURVIVAL.
        //
        // `apply_capture_throws` silently `continue`s when its `find` matches
        // nothing — a victim missing one of the eleven components its query
        // demands, a hold already over, a captor that is itself captured. A
        // cell scored "did not KO" on the strength of that would read as
        // *throws are weak*, which is the exact complaint under investigation,
        // and the instrument would be answering about itself.
        //
        // The captive is released BY the throw, so `CapturedBy` still being
        // here means the system never ran.
        if let Pulse::Throw(_) = pulse {
            let executed = self
                .app
                .world()
                .get::<ambition_platformer2d::combat::capture::CapturedBy>(self.victim)
                .is_none();
            // Whatever happened, this fixture's hold must not survive into the
            // next trial — a leaked `CapturedBy` would make the NEXT pulse
            // measure a captive.
            self.app.world_mut().entity_mut(self.victim).remove::<(
                ambition_platformer2d::combat::capture::CapturedBy,
                ambition_platformer2d::characters::control::ScriptedControl,
                ambition_platformer2d::characters::control::ControlHolds,
                ambition_platformer2d::characters::smash_hold_state::SmashHoldState,
            )>();
            if !executed {
                // ⛔ THE OTHER SILENT `None`, and it is the one that produced the
                // stage-3 matrix's 24/24 `REFUSED@0`. Distinguishing it from the
                // grounded refusal above is the whole question: both returned
                // quietly, so the table could not say which had happened.
                eprintln!(
                    "KO_REFUSE: stage=throw-never-executed x={victim_x:.0} \
                     pct={entry_percent} — CapturedBy survived the trial loop, so \
                     apply_capture_throws' find matched nothing"
                );
                return None;
            }
        }

        Some(Trial {
            tumbled: resolved_launch.is_some_and(|v| self.tumble_speed > 0.0 && v >= self.tumble_speed),
            resolved_launch,
            ko,
            ticks_to_ko,
            entry_percent,
            // ⭐ AND THIS IS WHY THE FIELD EXISTS. `apply_capture_throws`
            // damages BEFORE reading the meter, so a throw's launch arithmetic
            // sees `entry + damage` where a strike's sees `entry`.
            effective_percent: match pulse {
                Pulse::Strike(_) => entry_percent,
                Pulse::Throw(params) => entry_percent + params.damage,
            },
            at_strike,
            ko_forced,
        })
    }

    /// Fire a STRIKE pulse. `run_contact` and `run_determinism` are strike-only
    /// diagnostics by design — they ask about contact and reproducibility of a
    /// swing — so they keep a name that says so.
    fn strike(&mut self, hit: &HitVolume, entry_percent: i32, victim_x: f32) -> Option<Trial> {
        self.fire(Pulse::Strike(hit), entry_percent, victim_x)
    }

    /// Coarse sweep → bracket → binary search → VERIFY both sides.
    ///
    /// ⛔ NOT A BARE BINARY SEARCH. The raw launch formula is monotonic in
    /// percent, but a TRAJECTORY is not obliged to be: platforms, ceilings,
    /// downward launches and landings all intervene. A search that assumes
    /// monotonicity would return a confident number for a curve that does not
    /// have one.
    /// The resolved launch at one percent, retrying a refused reset.
    fn launch_at(&mut self, pulse: Pulse<'_>, percent: i32, victim_x: f32) -> Option<f32> {
        for _ in 0..3 {
            if let Some(t) = self.fire(pulse, percent, victim_x) {
                return t.resolved_launch;
            }
        }
        None
    }

    /// Does this pulse KO at `percent`? DECIDED BY MAJORITY OF THREE.
    ///
    /// ⛔ ONE TRIAL IS NOT AN ANSWER NEAR A BOUNDARY. Run 2 self-detected 15
    /// disagreements across 100 cells and they were all one shape: the binary
    /// search converged on a percent BECAUSE a trial there killed, and the
    /// verification at that same percent then did not. Far from a threshold this
    /// probe is exactly reproducible — 8 of 8 trials byte-identical at 200% on a
    /// move whose threshold is above 300 — so the flakiness is not general. It
    /// lives within a percent or two of the blast line, where the body crosses
    /// or fails to cross on the strength of a single tick.
    ///
    /// ⭐ THE RIGHT PRECISION IS THE ONE THE QUESTION NEEDS. Tuning how a game
    /// FEELS does not turn on ±1%, so the answer is not a more exact search; it
    /// is a cell that reports the same value twice. Majority of three, short-
    /// circuiting as soon as either side reaches two.
    ///
    /// A refused reset is RETRIED rather than counted, and `None` — the probe
    /// could not obtain a clean trial — is returned so the caller can say so
    /// instead of silently scoring a survival.
    fn kills(&mut self, pulse: Pulse<'_>, percent: i32, victim_x: f32) -> Option<Vote> {
        let (mut yes, mut no) = (0, 0);
        while yes < 2 && no < 2 {
            let mut trial = None;
            for _ in 0..3 {
                if let Some(t) = self.fire(pulse, percent, victim_x) {
                    trial = Some(t);
                    break;
                }
            }
            match trial {
                Some(t) if t.ko => yes += 1,
                Some(_) => no += 1,
                None => return None,
            }
        }
        // ⭐⭐ A MIXED VOTE IS A MEASUREMENT, NOT NOISE TO BE SWALLOWED.
        //
        // The loop short-circuits at two, so reaching here with BOTH counters
        // non-zero means 2:1 — the same initial state produced different outcomes.
        // Majority-of-three then reports a single number for a cell that could not
        // reproduce itself, and `117%` reads as exact when it is a band.
        //
        // ⛔ AND THIS IS THE SIGNAL I SPENT THREE ATTEMPTS FAILING TO DERIVE.
        // Measured this session: `special` moved 221 -> 160 and George's forward
        // throw 175 -> 115 when only the sweep STEP changed, while `back_throw`
        // (further below `tumble_speed` than either) did not move at all. I
        // proposed three arithmetic screens for which cells straddle a KO regime —
        // proximity to tumble_speed, the boundary label, launch-per-percent — and
        // measurement falsified all three. Two cells at L_ko ~645 behaved
        // oppositely.
        //
        // ⇒ Stop predicting which cells are unstable and let the trials say so.
        // A cell whose three runs disagree is ON an edge, whatever the arithmetic
        // thinks, and that is knowable from data the probe already computes.
        if yes > 0 && no > 0 {
            eprintln!(
                "KO_MIXED_VOTE: pct={percent} x={victim_x:.0} yes={yes} no={no} \
                 — trials disagreed at the SAME initial state; this cell's threshold \
                 is a BAND, not a point, and re-sweeping it finely is the only way \
                 to know what it is"
            );
            return Some(Vote::Mixed { yes, no });
        }
        Some(if yes > no { Vote::Killed } else { Vote::Survived })
    }

    fn ko_threshold(&mut self, pulse: Pulse<'_>, victim_x: f32, max_percent: i32) -> Threshold {
        // ⚠ 50 RATHER THAN 25, AND IT IS A TRADE I AM MAKING ON PURPOSE.
        // Widening the coarse step halves the sweep (13 samples -> 7) and costs
        // the binary search one extra iteration, so the THRESHOLD it converges
        // on is unchanged — but the sweep is also how `NonMonotonic` is spotted,
        // and half as many samples is half as many chances to see a curve double
        // back. The both-sides verification below still runs on every cell, so a
        // non-monotonicity AT the threshold is still caught; one hiding strictly
        // between two coarse samples is not. That is the exposure.
        // ⛔⛔ AND THE EXPOSURE ABOVE HAS NOW BEEN PAID, ONCE, IN A CALIBRATION.
        //
        // MEASURED on George's forward throw: the step-50 sweep samples launch
        // 401.6 at p=100 and 520.4 at p=150, and `tumble_speed = 500` sits BETWEEN
        // them. The non-tumbling window (launch ~402-500) is stepped clean over —
        // and that window is where the move kills by FALLING off the stage edge
        // rather than by crossing the side blast line. The reported threshold was
        // 175 (`boundary=side-right`, L=579.8); raising growth revealed a KO at 91
        // (`boundary=fall`, L=436.3). A 25% DROP in the launch required to kill,
        // which no authored value can cause — two KO mechanisms, and the sweep only
        // ever saw one of them.
        //
        // ⇒ `step=<n>` is argv-driven so a suspect cell can be re-swept finely
        // WITHOUT re-defining what every recorded run measured. 50 stays the
        // default for exactly that reason.
        //
        // ⚠ A cell whose KO launch lands near `tumble_speed` is the one to distrust:
        // that is where the mechanism changes, so that is where a coarse sample can
        // straddle two regimes and report the wrong one as "the" threshold.
        let step: i32 = std::env::args()
            .find_map(|a| a.strip_prefix("step=").and_then(|n| n.parse::<i32>().ok()))
            .filter(|n| *n > 0)
            .unwrap_or(50);
        // ⭐⭐ CALIBRATION MODE: EXACT, NOT BRACKETED — and the difference is a
        // number this session MEASURED, not a preference.
        //
        // George's forward throw reports a threshold of 175 at step=50 and 115 at
        // step=10 from the SAME authored value, because a KO window narrower than
        // the step is stepped clean over. Calibration inverts
        // `G_new = G_old * p0/p1`, which takes whatever `p0` it is handed on
        // faith — so a swept threshold does not merely mis-report, it BAKES the
        // sweep's own artifact into a shipped growth. A survey may be coarse; a
        // value you are about to author may not.
        //
        // ⭐ THE FIRST KILL IS THE ANSWER, with no bracket and no both-sides
        // re-verification. Ascending by 1 means `p - 1` was DIRECTLY measured one
        // iteration earlier and survived — which is precisely the fact the coarse
        // path spends two extra trials reconstructing after its binary search.
        //
        // ⛔ AND A MIXED VOTE IS INVALID HERE RATHER THAN OUTVOTED. Majority-of-
        // three turns "this cell could not reproduce itself" into a crisp integer,
        // and that integer would become a shipped value.
        if std::env::args().any(|a| a == "calib") {
            let mut p = 0;
            while p <= max_percent {
                match self.kills(pulse, p, victim_x) {
                    None => return Threshold::Refused(p),
                    Some(Vote::Mixed { yes, no }) => {
                        return Threshold::Unstable { percent: p, yes, no }
                    }
                    Some(Vote::Killed) => return Threshold::At(p),
                    Some(Vote::Survived) => {}
                }
                p += 1;
            }
            // Same separation the coarse path makes: never killing and never
            // connecting are different answers, and only one is about the game.
            return if self.launch_at(pulse, max_percent, victim_x).is_some() {
                Threshold::Above(max_percent)
            } else {
                Threshold::NoContact
            };
        }
        let mut samples: Vec<(i32, bool)> = Vec::new();
        let mut bracket: Option<(i32, i32)> = None;
        let mut p = 0;
        while p <= max_percent {
            let Some(ko) = self.kills(pulse, p, victim_x).map(Vote::majority) else {
                return Threshold::Refused(p);
            };
            samples.push((p, ko));
            if ko && bracket.is_none() {
                bracket = Some(((p - step).max(0), p));
            }
            // A KO followed by a survival at a HIGHER percent is a real
            // finding, not noise to be smoothed.
            if bracket.is_some() && !ko {
                return Threshold::NonMonotonic(samples);
            }
            if bracket.is_some() {
                break;
            }
            p += step;
        }
        let Some((mut lo, mut hi)) = bracket else {
            // ⭐ "NEVER KILLED" AND "NEVER CONNECTED" ARE DIFFERENT ANSWERS and
            // only one of them is about the game. One probe at the top of the
            // range separates them.
            let connected = self
                .launch_at(pulse, max_percent, victim_x)
                .is_some();
            return if connected {
                Threshold::Above(max_percent)
            } else {
                Threshold::NoContact
            };
        };
        while hi - lo > 1 {
            let mid = (lo + hi) / 2;
            let Some(killed) = self.kills(pulse, mid, victim_x).map(Vote::majority) else {
                return Threshold::Refused(mid);
            };
            if killed {
                hi = mid;
            } else {
                lo = mid;
            }
        }
        // ⛔ ZERO IS THE FLOOR, AND IT HAS NO BELOW — measured 2026-09-13.
        //
        // `hi == 0` means the pulse KOs at zero percent. The previous form still
        // ran the both-sides check at `(hi - 1).max(0)`, which is 0 AGAIN, so
        // `below` was necessarily the same measurement as `at` and the
        // contradiction branch fired every time. It then recorded the sample as
        // `hi - 1` = `-1`, a percent no trial ever ran. Every zero-percent ledge
        // KO in run 1 was reported `NON_MONOTONIC[(0, true), (-1, true), ...]`
        // on the strength of that. A KO at the floor is a finding, not a fault.
        if hi == 0 {
            return Threshold::At(0);
        }
        // VERIFY BOTH SIDES rather than trusting the walk.
        //
        // ⛔ THROUGH `kills`, NOT `strike`. These two lines were the last place a
        // single trial decided anything, and they are precisely where run 2's
        // fifteen disagreements surfaced: the search converged on `hi` because a
        // trial there killed, and then ONE trial at that same `hi` said it did
        // not. `is_some_and` also folded a refused reset into "did not kill",
        // which is the same silent survival the `Refused` variant exists to stop.
        let Some(below) = self.kills(pulse, hi - 1, victim_x).map(Vote::majority) else {
            return Threshold::Refused(hi - 1);
        };
        let Some(at) = self.kills(pulse, hi, victim_x).map(Vote::majority) else {
            return Threshold::Refused(hi);
        };
        if below || !at {
            samples.push((hi - 1, below));
            samples.push((hi, at));
            return Threshold::NonMonotonic(samples);
        }
        Threshold::At(hi)
    }
}

/// PHASE 2 ENTRY: KO envelopes for reachable strike pulses, reference vs heavy.
///
/// ⛔ THE SELF-TEST RUNS FIRST AND GATES EVERYTHING. Reusing one staged match
/// across trials is only sound if a repeated trial repeats; if it does not, a
/// whole table would be contaminated by the previous stock and look like data.
/// ⭐ AT WHICH PERCENTS DOES THE PULSE FAIL TO CONNECT AT ALL?
///
/// The determinism probe reported `launch=-` — no contact — for a 300% trial
/// whose at-strike state was IDENTICAL to eight trials at 200% that connected
/// cleanly, and it did so in both runs. That is reproducible and tracks the
/// percent rather than the trial's position in the sequence.
///
/// ⛔ THIS IS NOT COSMETIC. `ko_threshold` records a no-contact trial as a
/// SURVIVAL, so a percent band where contact silently fails reads as a band the
/// victim survived, and the threshold walks straight past it.
///
/// Two hypotheses were rejected by reading the code rather than by running this:
/// the emitted magnitude variant is decided by the `HitboxKnockback` this probe
/// constructs (`LaunchSpeed`), so no variant is being dropped by the reader; and
/// `set_damage_taken` is `accumulated = damage.max(0)`, so there is no cap and
/// no state transition at a high meter. Having no hypothesis left is the reason
/// to measure rather than to keep guessing.
fn run_contact() {
    let mut probe = KoProbe::new("npc_pirate_admiral", "player_robot_v3");
    let launchers = {
        let world = probe.app.world();
        let registry = world
            .get_resource::<ambition_platformer2d::characters::prepared::PreparedCharacterRegistry>()
            .expect("registry");
        let prepared = registry.get("npc_pirate_admiral").expect("attacker");
        let contract = prepared.kit.projectable_moveset().expect("moveset");
        let mut bad = Vec::new();
        let mut nl = Vec::new();
        launchers_of("npc_pirate_admiral", contract, &mut bad, &mut nl)
    };
    let Some(Launcher::Strike { hit, move_id, .. }) = launchers
        .iter()
        .find(|l| l.role() == "smash_forward" && matches!(l, Launcher::Strike { .. }))
    else {
        println!("# no attack_forward strike found");
        return;
    };
    println!("# contact sweep: npc_pirate_admiral `{move_id}` vs player_robot_v3, centre");
    println!("# a row with contact=false is one `ko_threshold` would have recorded as a SURVIVAL");
    println!("percent\tcontact\tlaunch\tko\tat_strike");
    for p in (0..=300).step_by(25) {
        match probe.strike(hit, p, probe.centre) {
            Some(t) => println!(
                "{p}\t{}\t{}\t{}\t{}",
                t.resolved_launch.is_some(),
                t.resolved_launch
                    .map(|v| format!("{v:.1}"))
                    .unwrap_or_else(|| "-".into()),
                t.ko,
                t.at_strike
            ),
            None => println!("{p}\tRESET_REFUSED\t-\t-\t-"),
        }
    }
}

/// ⭐ DOES A TRIAL INHERIT ANYTHING FROM THE TRIAL BEFORE IT?
///
/// Run 1 said yes and named the shape. Two cells reported a percent that the
/// coarse sweep measured as a SURVIVAL and the verification measured as a KO,
/// and in both the binary search had converged to `lo + 1` — every probe after
/// the first knockout came back a knockout. The oracle row moved with it
/// (centre 291 on the slow path, 283 on the fast one).
///
/// ⛔ THE SELF-TEST IN `run_probe` CANNOT SEE THIS. It fires one pulse at one
/// percent twice with identical history, so it establishes that the probe is
/// deterministic given the same past — which is not the property the threshold
/// search needs. The search needs the answer at a percent to be independent of
/// what ran before it, and that is what this measures.
///
/// This prints state rather than asserting, because the question is WHICH fact
/// carries over, and a guess at that is worth nothing.
fn run_determinism() {
    // ⛔⛔ NEAR A REAL BOUNDARY, AND THE FIRST VERSION OF THIS TEST WAS NOT.
    //
    // It probed 200% on a move whose centre threshold is above 300, so every
    // trial was a comfortable survival and all eight agreed. That established
    // the probe is reproducible where nothing is in doubt — a guard that cannot
    // fail — and it is why run 2 still produced fifteen disagreeing cells after
    // this test had passed. The flakiness lives within a percent or two of the
    // blast line, so the test has to stand there.
    //
    // ⛔ AND THE SECOND ATTEMPT MISSED TOO, FOR A DIFFERENT REASON. It used 50%,
    // taken from a `smash_forward` LEDGE contradiction, but then fired
    // `tilt_forward` at CENTRE — a different move at a different position, where
    // the threshold is above 300. Result: 6 of 6 `kills` and 8 of 8 single
    // trials all agreeing on a comfortable survival. A guard that cannot fail,
    // twice over.
    //
    // ⛔ THE THIRD ATTEMPT MISSED TOO, AND FOR THE MOST AVOIDABLE REASON YET: it
    // took 145 from RUN 2, a table produced by the broken instrument. 8 of 8
    // single trials came back a unanimous KILL at launch 721.9. A percent
    // inherited from void data is not a threshold on this tree.
    //
    // ⭐ SO THE BOUNDARY IS MEASURED ON THIS TREE, NOT INHERITED. Sweeping
    // `smash_forward` at centre with the current binary:
    //     125% -> survive (launch 644.4)
    //     150% -> KO      (launch 741.2)
    // and 145% was already a unanimous kill, so the flip sits below it. 130 is
    // inside the bracket and is where a single trial should waver if it ever
    // does — which the single-trial control below is there to report.
    const NEAR: i32 = 130;
    const LETHAL: i32 = 300;

    let mut probe = KoProbe::new("npc_pirate_admiral", "player_robot_v3");
    let launchers = {
        let world = probe.app.world();
        let registry = world
            .get_resource::<ambition_platformer2d::characters::prepared::PreparedCharacterRegistry>()
            .expect("registry");
        let prepared = registry.get("npc_pirate_admiral").expect("attacker");
        let contract = prepared.kit.projectable_moveset().expect("moveset");
        let mut bad = Vec::new();
        let mut nl = Vec::new();
        launchers_of("npc_pirate_admiral", contract, &mut bad, &mut nl)
    };
    // The pulse the run-1 latch actually appeared on: a forward tilt, whose
    // centre cell was one of the two genuine disagreements.
    let Some(Launcher::Strike { hit, move_id, .. }) = launchers
        .iter()
        .find(|l| l.role() == "smash_forward" && matches!(l, Launcher::Strike { .. }))
    else {
        println!("# no attack_forward strike found — cannot reproduce the run-1 latch");
        return;
    };
    println!("# determinism probe on npc_pirate_admiral `{move_id}` vs player_robot_v3");
    println!("# every line below is the SAME pulse at the SAME percent unless marked");
    println!("phase\ttrial\tpercent\tko\tlaunch\tticks\tpre_state");

    let x = probe.centre;

    // ⭐⭐ THE ARM THAT TESTS THE ACTUAL CLAIM.
    //
    // The single-trial rows below may legitimately disagree with each other at
    // this percent — that IS the defect, and reproducing it is half the point.
    // The claim that needs testing is the REMEDY: that `kills`, deciding by
    // majority of three, returns the same verdict every time at a percent where
    // one trial does not. A test that only ever calls `strike` cannot say
    // anything about that, and calling this file's determinism probe "passing"
    // on the strength of single trials is how run 2 shipped fifteen bad cells.
    //
    // ⛔ If these six disagree, majority-of-three is NOT enough and no matrix
    // should be run on it.
    let mut votes = Vec::new();
    for _ in 0..6 {
        votes.push(
            probe
                .kills(Pulse::Strike(hit), NEAR, x)
                .map(|k| k.to_string())
                .unwrap_or_else(|| "REFUSED".into()),
        );
    }
    println!("# kills()@{NEAR}% x6 (majority-of-three each): {}", votes.join(" "));
    // ⛔ THIS LINE MAY NOT CLAIM BOUNDARY-NESS IT HAS NOT ESTABLISHED.
    //
    // It previously printed "the remedy holds at a boundary percent" whenever
    // the six votes agreed — including on percents that were nowhere near a
    // boundary, which is what happened on all three attempts (200% and 50% were
    // comfortable survivals; 145% a comfortable kill, 8 of 8 single trials
    // agreeing at launch 721.9). Agreement at a percent where ONE trial is
    // already unanimous says nothing whatever about majority-of-three.
    //
    // The single-trial arm is the control: only if IT disagrees is this percent
    // a boundary, and only then does agreement among the votes mean anything.
    let votes_agree = votes.iter().all(|v| v == &votes[0]);
    println!(
        "# ⇒ kills() {}",
        if votes_agree {
            "agreed 6/6"
        } else {
            "⛔ DISAGREED — majority-of-three is insufficient; do not run the matrix"
        }
    );
    println!(
        "# ⇒ whether that MEANS anything depends on the single-trial rows below: \
         if they are unanimous too, {NEAR}% is not a boundary and this test is \
         a guard that cannot fail."
    );

    let x = probe.centre;
    for i in 0..4 {
        let t = probe.strike(hit, NEAR, x);
        let pre = t
            .as_ref()
            .map(|t| t.at_strike.clone())
            .unwrap_or_else(|| "RESET_REFUSED".into());
        println!(
            "before_any_ko\t{i}\t{NEAR}\t{}\t{}\t{}\t{pre}",
            t.as_ref().map(|t| t.ko.to_string()).unwrap_or_else(|| "-".into()),
            t.as_ref()
                .and_then(|t| t.resolved_launch)
                .map(|v| format!("{v:.1}"))
                .unwrap_or_else(|| "-".into()),
            t.as_ref()
                .and_then(|t| t.ticks_to_ko)
                .map(|v| v.to_string())
                .unwrap_or_else(|| "-".into()),
        );
    }

    let t = probe.strike(hit, LETHAL, x);
    let pre = t
        .as_ref()
        .map(|t| t.at_strike.clone())
        .unwrap_or_else(|| "RESET_REFUSED".into());
    // ⛔ THE LAUNCH COLUMN IS FORMATTED, NOT HARDCODED — and it was hardcoded.
    //
    // This row printed a literal `-` where every other row prints a measured
    // launch, and I read my own placeholder as a measurement: it became a
    // reported "no contact at 300%", a commit message recording a defect as
    // KNOWN AND NOT YET EXPLAINED, and a contact sweep to chase it. The sweep
    // came back 13 of 13 connected. The trial had been fine the whole time.
    //
    // ⇒ A column that can only ever print one value is not reporting anything.
    println!(
        "INDUCE_KO\t-\t{LETHAL}\t{}\t{}\t{}\t{pre}",
        t.as_ref().map(|t| t.ko.to_string()).unwrap_or_else(|| "-".into()),
        t.as_ref()
            .and_then(|t| t.resolved_launch)
            .map(|v| format!("{v:.1}"))
            .unwrap_or_else(|| "-".into()),
        t.as_ref()
            .and_then(|t| t.ticks_to_ko)
            .map(|v| v.to_string())
            .unwrap_or_else(|| "-".into()),
    );

    for i in 0..4 {
        let t = probe.strike(hit, NEAR, x);
        let pre = t
            .as_ref()
            .map(|t| t.at_strike.clone())
            .unwrap_or_else(|| "RESET_REFUSED".into());
        println!(
            "after_a_ko\t{i}\t{NEAR}\t{}\t{}\t{}\t{pre}",
            t.as_ref().map(|t| t.ko.to_string()).unwrap_or_else(|| "-".into()),
            t.as_ref()
                .and_then(|t| t.resolved_launch)
                .map(|v| format!("{v:.1}"))
                .unwrap_or_else(|| "-".into()),
            t.as_ref()
                .and_then(|t| t.ticks_to_ko)
                .map(|v| v.to_string())
                .unwrap_or_else(|| "-".into()),
        );
    }
    println!(
        "# READ IT AS: if `before_any_ko` and `after_a_ko` disagree at {NEAR}%, the trial \
         is NOT independent and the differing column of `pre_state` names what carried over."
    );
}

/// WHY IS EVERY THROW CELL `REFUSED@0`? — NAME the missing precondition instead
/// of guessing at it.
///
/// ⛔⛔ THE STAGE-3 MATRIX REFUSED ALL 24 THROW CELLS — every role, every
/// attacker, both victims, at entry percent 0. Read as data those rows say
/// *"throws never launch very far"*, which is the ORIGINAL COMPLAINT apparently
/// confirmed by 24 independent cells. `PREDICTION_THROWS.md` pre-registered this
/// exact shape as an ACTUATOR failure precisely so it could not be read that way.
///
/// `apply_capture_throws` takes an ELEVEN-component query on the captive and
/// `continue`s in SILENCE when its `find` matches nothing. EIGHT of those are
/// REQUIRED (non-`Option`), so a seated fighter missing ANY ONE produces a
/// uniform, quiet refusal that looks identical to a weak throw.
///
/// ⭐ AND THE KNOWN-GOOD RECIPE IS ALREADY IN THE TREE: `capture/systems.rs`'s
/// own `throw_app`/`grounded_body` fixture builds a captive the system DOES see.
/// The only open question is which column a LIVE match fighter lacks that the
/// fixture spells out — `gravity/resolve.rs:51` says a PLAYER entity carries no
/// `ActorSurfaceState` at all, while `actor_spawn` and `body_seed` both insert
/// one. This prints the difference rather than reasoning about it.
///
/// ⛔ IT DELIBERATELY INSERTS NOTHING TO "FIX" THE BODY. Adding a default
/// `ActorSurfaceState` to a victim that already has one would overwrite its real
/// `gravity_scale` and corrupt every later trial — a repair applied before the
/// diagnosis, which is how a fixture starts measuring itself.
fn run_throw_diag() {
    use ambition_platformer2d::characters::actor as ca;
    use ambition_platformer2d::engine_core as ec;
    use ambition_platformer2d::engine_core::Vec2 as EVec2;

    let attacker_id = "npc_pirate_admiral";
    let victim_id = "player_robot_v3";
    println!("# THROW ACTUATOR DIAGNOSTIC — {attacker_id} throwing {victim_id}");
    println!("# 24/24 stage-3 throw cells were REFUSED@0. This asks WHY.");

    let mut probe = KoProbe::new(attacker_id, victim_id);

    // ⛔ SEPARATE THE TWO WAYS `fire` RETURNS `None`. A refusal from the RESET is
    // a statement about the fixture's ability to stand a body up, and has
    // nothing to do with the throw road; only a reset that SUCCEEDED lets the
    // rest of this diagnostic mean anything.
    if !probe.reset_trial(0, probe.centre) {
        println!("⛔ reset_trial REFUSED — the refusal happens BEFORE any throw.");
        println!("   The REFUSED@0 cells are then NOT evidence about the throw road.");
        return;
    }
    println!("✅ reset_trial succeeded — the victim stands still on the floor.");

    macro_rules! has {
        ($e:expr, $t:ty) => {{
            if probe.app.world().get::<$t>($e).is_some() {
                "present"
            } else {
                "⛔ MISSING"
            }
        }};
    }

    let v = probe.victim;
    let a = probe.attacker;

    println!("\n# REQUIRED on the captive — any ONE missing refuses the whole query:");
    println!("   BodyKinematics      {}", has!(v, ec::BodyKinematics));
    println!("   BodyFlightState     {}", has!(v, ec::BodyFlightState));
    println!("   BodyCombat          {}", has!(v, ca::BodyCombat));
    println!("   BodyHealth          {}", has!(v, ca::BodyHealth));
    println!("   ActorSurfaceState   {}", has!(v, ec::ActorSurfaceState));
    println!("   BodyGroundState     {}", has!(v, ec::BodyGroundState));
    println!("   BodyDodgeState      {}", has!(v, ec::BodyDodgeState));

    println!("\n# OPTIONAL in the query — absence here is NOT a cause:");
    println!(
        "   ControlHolds        {}",
        has!(v, ambition_platformer2d::characters::control::ControlHolds)
    );

    println!("\n# the CAPTOR arm: Query<&BodyKinematics, Without<CapturedBy>>");
    println!("   attacker BodyKinematics {}", has!(a, ec::BodyKinematics));
    println!(
        "   attacker CapturedBy     {}  (must be ABSENT to match)",
        has!(a, ambition_platformer2d::combat::capture::CapturedBy)
    );

    // ── REPRODUCE THE MATRIX'S ACTUAL FIRST CALL, not an approximation of it. ──
    //
    // ⛔⛔ THIS IS THE STEP THAT DECIDES IT. The hand-rolled actuation below
    // proves the throw ROAD works; it does NOT prove `fire` works, and `fire` is
    // what the matrix ran. `ko_threshold` opens with `kills(pulse, 0, ..)`, and
    // `kills` returns `None` — which prints as `REFUSED@0` — only when THREE
    // consecutive `fire` calls all refuse. So three calls here reproduce the
    // exact condition, and the `KO_REFUSE` lines on stderr now name WHICH of the
    // two silent refusal stages fired.
    //
    // ⛔ AND IT RUNS FIRST, BEFORE the manual actuation below, because that one
    // leaves the victim launched and airborne — a `fire` measured after it would
    // be measuring the wreckage of the previous experiment.
    let params = CaptureThrowParams {
        damage: 9,
        knockback: 104.0,
        knockback_growth: 2.40,
        launch_dir: (1.0, -0.5),
    };
    println!("\n# `kills(Pulse::Throw, 0, centre)` fires `fire` up to 3x. Reproducing:");
    let mut refusals = 0;
    for attempt in 0..3 {
        match probe.fire(Pulse::Throw(&params), 0, probe.centre) {
            // ⚠ `tumbled`, `entry_percent` and `effective_percent` are printed
            // here because they were CAPTURED and read by nothing — three fields
            // whose doc comments state exactly why each was worth recording,
            // written every trial and never surfaced. A measurement a binary
            // takes and does not emit is not a measurement.
            Some(t) => println!(
                "   attempt {attempt}: Some — ko={} resolved_launch={:?} \
                 tumbled={} entry%={} effective%={}",
                t.ko, t.resolved_launch, t.tumbled, t.entry_percent, t.effective_percent
            ),
            None => {
                refusals += 1;
                println!("   attempt {attempt}: ⛔ None (a KO_REFUSE line on stderr names the stage)");
            }
        }
    }
    println!(
        "   ⇒ {refusals}/3 refused. {}",
        if refusals == 3 {
            "THIS is the matrix's REFUSED@0, reproduced."
        } else {
            "`fire` does NOT uniformly refuse here — the matrix's cause is elsewhere."
        }
    );

    // ── Now actuate ONE throw exactly as `fire` does, and watch it land. ──
    probe.app.world_mut().entity_mut(v).insert((
        ambition_platformer2d::combat::capture::CapturedBy {
            captor: a,
            hold_offset_local: EVec2::new(16.0, 0.0),
            prior_gravity_scale: 1.0,
        },
        ambition_platformer2d::characters::control::ScriptedControl,
        ambition_platformer2d::characters::control::ControlHolds::only(
            ambition_platformer2d::characters::control::ControlHold::Relationship,
        ),
        ambition_platformer2d::characters::smash_hold_state::SmashHoldState::lasting(10.0),
    ));
    println!(
        "\n# hold installed — CapturedBy on the victim: {}",
        has!(v, ambition_platformer2d::combat::capture::CapturedBy)
    );

    probe.app.world_mut().write_message(
        ambition_platformer2d::combat::capture::CaptureThrowRequested {
            captor: a,
            damage: 9,
            knockback: 100.0,
            knockback_growth: 0.0,
            launch_dir: EVec2::new(1.0, -1.0),
        },
    );

    // ⭐ SEVERAL TICKS, NOT ONE. The captive is released BY the throw, so a
    // surviving `CapturedBy` means the system never ran — but a message written
    // from OUTSIDE the schedule might also simply be read a tick later than the
    // fixture assumes, and "never" and "not yet" are different diagnoses.
    let mut cleared_on = None;
    for tick in 0..6 {
        probe.app.update();
        if probe
            .app
            .world()
            .get::<ambition_platformer2d::combat::capture::CapturedBy>(v)
            .is_none()
        {
            cleared_on = Some(tick);
            break;
        }
    }

    println!("\n# VERDICT");
    match cleared_on {
        Some(t) => {
            println!("  ✅ the throw EXECUTED on tick {t} — the capture released.");
            println!("     ⇒ the actuator works here, so the matrix's refusal is");
            println!("       something the PER-TRIAL path does differently. Look at");
            println!("       `fire`'s ordering, not at the component set.");
        }
        None => {
            println!("  ⛔ `CapturedBy` SURVIVED six ticks — `apply_capture_throws`");
            println!("     never ran for this captive. Its `find` matched nothing.");
            println!("     ⇒ the cause is a MISSING REQUIRED COMPONENT above, or the");
            println!("       message never reached `CombatSet::Materialize`.");
        }
    }
    let (vx, vy) = {
        let p = probe.pos(v);
        (p.x, p.y)
    };
    println!("  victim at ({vx:.1},{vy:.1})");
}

fn run_probe() {
    const REFERENCE: &str = "player_robot_v3";
    const HEAVY: &str = ambition_demo_smash::SMASH_GEORGE_BOOUL;
    // ⛔⛔ THIS IS A THREE-FIGHTER SAMPLE. IT IS NOT THE ROSTER, AND ANY STATISTIC
    // TAKEN OVER IT MUST NOT BE CALLED A ROSTER STATISTIC.
    //
    // The AUTHORED-VALUE census elsewhere in this tool genuinely walks every
    // seatable fighter. The STAGE-OUTCOME matrix below does not: it walks these
    // three attackers against two victims, and nothing in the output says so.
    //
    // That gap has already produced a false claim. A commit message reported
    // "roster medians: smash_forward 183 -> 120, smash_down 208 -> 163, smash_up
    // 275 -> 219" and "20 smash pulses" as evidence that a global nonlinear law fit
    // THE ROSTER. Those numbers came from three fighters. The per-fighter
    // measurements were sound; the population they were generalised to did not
    // exist, and a law was justified on it.
    //
    // ⇒ Either seat every attacker here, or say "three-fighter calibration sample"
    // every single time these cells are cited. The honest phrasing is the cheap
    // half; widening the sample costs runtime this tool has not been given.
    //
    // (Kept deliberately: the original rationale — the question is whether ROLES
    // separate, and every fighter authors the same role set — is still why three
    // attackers is a reasonable SAMPLE. It is not why it would be a population.)
    let attackers = ["npc_pirate_admiral", "smash_george_booul", "npc_bob"];
    // ⭐ THE CEILING IS ARGV-DRIVEN, AND 300 REMAINS THE DEFAULT so that every run
    // recorded in PROVENANCE_RUNS.md replays to the same numbers. Raising it does
    // not re-measure history; it measures what history could not see.
    //
    // ⛔ A `>300` CELL IS NOT A MEASUREMENT OF A WEAK MOVE — IT IS THE ABSENCE OF
    // ONE. Measured on the curve-free baseline: 12 of 19 centre cells read `>300`
    // (every tilt, most aerials, attack_dash, special_down, up_throw, down_throw).
    // Calibration inverts `growth' = growth * (p0/p1)`, which has NO p0 for a
    // censored cell, so two thirds of the roster cannot be authored from a 300-cap
    // table at all.
    //
    // ⚠ COST, STATED: `ko_threshold` sweeps in steps of 50 (see :1836), so the
    // coarse pass grows linearly with this number and each cell runs a full trial
    // per sample. `ceiling=600` roughly doubles the sweep.
    let max_percent: i32 = std::env::args()
        .find_map(|a| {
            a.strip_prefix("ceiling=")
                .and_then(|n| n.parse::<i32>().ok())
        })
        .unwrap_or(300);

    // ⭐ ONE MATCHUP PER PROCESS. The six cells of the matrix share nothing —
    // separate Apps, separate worlds — so running them as six processes is pure
    // wall-clock division that cannot touch a single measured number. With no
    // arguments the tool still walks the whole matrix in one process.
    let argv: Vec<String> = std::env::args().collect();
    let after: Vec<&str> = argv
        .iter()
        .skip_while(|a| a.as_str() != "probe")
        .skip(1)
        .map(|s| s.as_str())
        // ⛔ `identity` IS A FLAG, NOT A FIGHTER. Without this filter
        // `probe <attacker> identity` would read "identity" as the VICTIM id,
        // fail the registry lookup, and print a table for a matchup nobody asked
        // for — a silent wrong-population error, which is the failure this
        // instrument exists to avoid.
        //
        // ⭐ `ceiling=<n>` JOINS THE FILTER FOR THE SAME REASON, and the reason is
        // no longer hypothetical: measured this session, running this binary with
        // no `probe` token produced 788 rows of a whole-roster MOVESET CENSUS —
        // well-formed, plausibly headed, real fighter names, and about a different
        // question entirely. An unfiltered flag lands in `after[0]` and silently
        // BECOMES the attacker. Nothing downstream can tell that from a table
        // somebody meant to ask for.
        .filter(|a| {
            *a != "identity"
                && *a != "calib"
                && !a.starts_with("ceiling=")
                && !a.starts_with("step=")
                && !a.starts_with("only=")
                && !a.starts_with("attacker_pct=")
        })
        .collect();
    let pairs: Vec<(String, String)> = if after.len() >= 2 {
        vec![(after[0].to_string(), after[1].to_string())]
    } else {
        let mut v = Vec::new();
        for victim in [REFERENCE, HEAVY] {
            for a in attackers {
                v.push((a.to_string(), victim.to_string()));
            }
        }
        v
    };

    println!("# ko_envelope PROBE — stage outcomes, 1.25 frozen, no authored value changed");
    // ⭐ THE SWEEP CEILING IS A LIVE FACT OF THE RUN, not a constant a reader may
    // look up in the source: `>N` means different things in two tables taken at
    // different N, and the source only ever states the CURRENT default. A table
    // that does not carry its own ceiling cannot be compared to one that does.
    println!(
        "# sweep ceiling: max_percent={max_percent} — a `>{max_percent}` cell is CENSORED \
         (no threshold found at or below it), NOT a measurement that the move is weak"
    );
    // ⭐ THE COARSE STEP IS A LIVE FACT TOO, and for the same reason the ceiling is:
    // two tables taken at different steps can report different thresholds for the
    // SAME ruleset, because a KO regime narrower than the step can be stepped over
    // (measured: George's forward throw, side-right at 175 vs fall at 91). A table
    // that does not carry its step cannot be compared to one that does.
    //
    // ⛔ It is ALSO the poison-verification for `step=`: without this line the flag
    // reaching the sweep could only be inferred from the very result it exists to
    // test, which is circular.
    // ⭐⭐ CALIBRATION MODE — OFF unless asked for, and the two flags below are
    // designed together rather than shipped as separate conveniences.
    //
    // `calib` replaces coarse sweep + bracket + binary search with an exact
    // ascending scan at step 1, and refuses a self-disagreeing cell instead of
    // outvoting it. That is what AUTHORING a value needs and it is far too slow
    // for a survey: a cell that kills near 110% costs ~110 trials instead of ~10.
    // ⇒ `only=<substring>` matches `move_id`, because a whole-roster exact pass
    // would run for hours and nobody would wait for it.
    let calib = std::env::args().any(|a| a == "calib");
    let only: Option<String> =
        std::env::args().find_map(|a| a.strip_prefix("only=").map(|s| s.to_string()));
    // ⛔⛔ THE STEP LINE MUST NOT DESCRIBE A PASS THAT DID NOT RUN — MEASURED, in
    // the first calibration run this flag ever served.
    //
    // The header printed `# sweep step: 50` while `calib` had replaced the coarse
    // sweep outright and an exact step-1 scan produced the row. That is false
    // provenance in the one place a later reader trusts without checking: this
    // file's own rule is that two tables taken at different steps are not
    // comparable, so a table claiming a step it never used is WORSE than one
    // carrying no step at all — it invites exactly the comparison it breaks.
    if calib {
        println!(
            "# sweep step: NOT IN EFFECT — `calib` replaced the coarse sweep with an \
             exact ascending step-1 scan; see the calibration line below"
        );
    } else {
        println!(
            "# sweep step: {} (coarse pass; a KO window narrower than this can be missed \
             entirely — distrust any cell whose KO launch lands near tumble_speed)",
            std::env::args()
                .find_map(|a| a.strip_prefix("step=").and_then(|n| n.parse::<i32>().ok()))
                .filter(|n| *n > 0)
                .unwrap_or(50)
        );
    }
    // ⛔ POISON-VERIFICATION FOR BOTH, for the reason the step line already gives:
    // a flag whose arrival can only be inferred from the result it exists to
    // change is a flag nobody can prove reached the sweep.
    println!(
        "# calibration mode: {} | {}",
        if calib {
            "ON — exact ascending scan, step=1, first KO is the threshold, mixed vote ⇒ UNSTABLE"
        } else {
            "off — coarse sweep + bracket + binary search, mixed vote ⇒ majority-of-three"
        },
        match &only {
            Some(w) => format!("only= ACTIVE: measuring launchers whose move_id contains {w:?}"),
            None => "no only= filter: every bound launcher measured".to_string(),
        }
    );
    // ⛔⛔ EVERY KO PERCENT BELOW IS A NO-RECOVERY LOWER BOUND, and reading one
    // as a kill percent overstates the game's lethality.
    //
    // Both bodies carry `Brain::stand_still()`. The victim therefore never DIs,
    // never jumps, never air-dodges and never uses a recovery move — it is
    // launched and it travels until something stops it. A real victim fights the
    // launch, so its true threshold is HIGHER than the number in these columns,
    // by an amount this instrument does not measure. What the columns ARE good
    // for is comparison between cells measured the same way: role against role,
    // centre against ledge, reference weight against heavy.
    println!(
        "# victim brain: stand_still — no DI, no jump, no recovery. KO% is a \
         NO-RECOVERY LOWER BOUND, not a kill percent."
    );
    // ⛔⛔ THIS LINE MUST NOT CLAIM A PIN THAT DID NOT HAPPEN. The step header
    // carried exactly this defect into the first calibration run it ever served —
    // it announced `step: 50` while a step-1 scan produced the row. A header is
    // the one thing a later reader trusts without re-deriving it.
    //
    // ⛔ AND IT DOES NOT RESTATE `rage_per_damage`/`rage_max_scale`. Those are
    // authored in `ambition_demo_smash`, and a constant transcribed into a probe
    // is the error that made this file's own gravity comment wrong by 55%. The
    // meter is what this tool set; the multiplier is the ruleset's to state.
    if attacker_meter() == 0 {
        println!("# attacker meter pinned to 0 so rage_scale cannot multiply a resolved launch.");
    } else {
        println!(
            "# attacker meter = {}% — NOT PINNED. `rage_scale` reads the attacker's own \
             damage and multiplies EVERY resolved launch, so these rows are NOT comparable \
             with rage-pinned tables and are NOT a lower bound.",
            attacker_meter()
        );
    }
    // ⛔⛔ AERIAL ROLES ARE MEASURED UNDER CONDITIONS THEY NEVER OCCUR IN, and
    // their rows are NOT comparable with the grounded ones.
    //
    // Every trial parks the victim standing on the platform, so an `attack_air*`
    // pulse is being fired at a grounded body from a grounded attacker. Run 2
    // duly reported `centre_never = n/n` for attack_air, attack_air_up,
    // attack_air_down and attack_dash. That is a statement about the FIXTURE,
    // not about aerials: a real aerial connects with an airborne victim who has
    // no floor to be driven into and no landing to absorb the launch.
    //
    // ⇒ Read the grounded roles (attack*, smash*, tilt*) as measurements and the
    // aerial roles as not-yet-measured. Giving them an airborne victim is a
    // separate fixture, not a tweak to this one.
    // ⛔⛔ THIS CAVEAT USED TO EXPLAIN AWAY A DEFECT, IN THE HEADER A READER
    // TRUSTS. It said the aerial cells "measure the fixture, not the move"
    // because an aerial meets a grounded parked victim — true as a caveat, and
    // NOT why those cells are empty. Measured 2026-09-13 with both refusal arms
    // logging: a full matchup produced 209 `KO_REFUSE` lines and EVERY ONE was
    // `stage=ground` — the reset's own grounded premise — covering the aerials,
    // the dash attack, both specials and all four throws alike. A real
    // instrument failure was wearing a design limitation's clothes.
    println!(
        "# ⚠ AERIAL ROLES (attack_air*, attack_dash) fire at a GROUNDED, PARKED victim — \
         a fixture they never meet in play, so read their cells with that in mind."
    );
    println!(
        "# ⛔ A `REFUSED@n` CELL IS AN INSTRUMENT FAILURE, NOT A STATEMENT ABOUT THE MOVE. \
         Measured: 209/209 refusals in a full matchup were `stage=ground` (the reset could not \
         stand the victim still on the floor), and they SNOWBALL — refusals grow through a run, \
         so the roles `launchers_of` pushes last (specials, throws) absorb most of them. \
         Cause under investigation; four hypotheses falsified by measurement so far. \
         ⛔ Do NOT read a refused row as 'this move is weak'."
    );
    // ⭐ THROW ROWS READ DIFFERENTLY FROM STRIKE ROWS IN THE SAME TABLE, and a
    // reader with only the TSV cannot tell. Three differences, all load-bearing:
    println!(
        "# ⭐ THROW ROWS (*_throw) ARE MEASURED PULSES, NOT STRIKES. \
         (1) w/v is '-': window/volume address a strike's hit volume and a throw has no such address. \
         (2) The KO%% column is the ENTRY percent — `apply_capture_throws` applies the throw's own \
         damage BEFORE the launch reads the meter, so the launch arithmetic saw KO%% + dmg. \
         (3) A refused throw cell means the throw NEVER EXECUTED (hold expired, or a component its \
         query demands was missing) — it is NOT a survival, and must never be read as 'throws are weak'."
    );

    {
        for (attacker_id, victim_id) in &pairs {
            let (attacker_id, victim_id) = (attacker_id.as_str(), victim_id.as_str());
            let mut probe = KoProbe::new(attacker_id, victim_id);
            println!(
                "\n# {attacker_id} vs {victim_id}  weight={:.2}  tumble_speed={:.1}",
                probe.victim_weight, probe.tumble_speed
            );

            // Build this attacker's bound strike pulses from the runtime contract.
            let launchers = {
                let world = probe.app.world();
                let registry = world
                    .get_resource::<ambition_platformer2d::characters::prepared::PreparedCharacterRegistry>()
                    .expect("registry");
                let Some(prepared) = registry.get(attacker_id) else {
                    println!("#   NOT IN REGISTRY — skipped");
                    continue;
                };
                let Some(contract) = prepared.kit.projectable_moveset() else {
                    println!("#   no projectable moveset — skipped");
                    continue;
                };
                let mut bad = Vec::new();
                let mut nl = Vec::new();
                // ⭐ THROWS ADMIT UNCONDITIONALLY, AND THE ASYMMETRY IS REAL —
                // this is not the strike rule with a hole punched in it.
                //
                // An unbound strike is a hit volume that NO verb reaches, so no
                // player can produce it and a row for it would measure a volume
                // the game never fires. A throw has no `verbs` list that could be
                // empty: `launchers_of` builds each one through `move_for_verb`,
                // so it exists only BECAUSE a verb resolved to it. Bound by
                // construction, and requiring a `verbs` field it does not have
                // would silently exclude every throw — which is precisely the
                // state this commit is ending.
                launchers_of(attacker_id, contract, &mut bad, &mut nl)
                    .into_iter()
                    .filter(|l| match l {
                        Launcher::Strike { verbs, .. } => !verbs.is_empty(),
                        Launcher::Throw { .. } => true,
                    })
                    // ⭐ `only=` NARROWS THE POPULATION, IT DOES NOT CHANGE A NUMBER.
                    // Each launcher's threshold search is independent of every
                    // other's — separate pulses, separate resets — so a filtered run
                    // and a full run report identical cells for the moves they share.
                    .filter(|l| match &only {
                        Some(want) => l.move_id().contains(want.as_str()),
                        None => true,
                    })
                    .collect::<Vec<_>>()
            };

            // ⭐ THE SELF-TEST, AND RUN 1 PROVED THE OLD ONE INADEQUATE.
            //
            // It fired one pulse at one percent twice and compared only `ko`.
            // That passed — and the table it admitted contained cells where a
            // percent measured as a survival in the coarse sweep came back a KO
            // in verification, because every trial was starting AIRBORNE and the
            // outcome turned on where a single update left the body.
            //
            // ⛔ COMPARE THE WHOLE OUTCOME, NOT THE VERDICT. Two trials can
            // agree on `ko` and still have begun in different states, which is
            // the disagreement that matters; `at_strike` is what catches it.
            // Three repetitions rather than two, because an alternating defect
            // (survive/KO/survive/KO — exactly what run 1 showed) is invisible
            // to any even-numbered comparison of the first two.
            //
            // ⛔ NOT `if let Some(Launcher::Strike { .. })`. Written that way, a
            // fighter whose first launcher is a THROW skips the self-test
            // entirely and has its whole table admitted with no independence
            // check — the `if let` falls through in silence and the run looks
            // exactly like one that passed. The guard must cover whatever pulse
            // is actually first.
            if let Some(self_test) = launchers.first().map(Launcher::pulse) {
                let mut seen = Vec::new();
                for _ in 0..3 {
                    seen.push(match probe.fire(self_test, 100, probe.centre) {
                        Some(t) => format!(
                            "ko={} launch={} {}",
                            t.ko,
                            t.resolved_launch
                                .map(|v| format!("{v:.1}"))
                                .unwrap_or_else(|| "-".into()),
                            t.at_strike
                        ),
                        None => "RESET_REFUSED".to_string(),
                    });
                }
                println!("#   repeatability@100% x3:");
                for s in &seen {
                    println!("#     {s}");
                }
                if seen.iter().any(|s| s != &seen[0]) {
                    println!(
                        "#   ⛔ TRIALS ARE NOT INDEPENDENT — the same pulse at the same percent \
                         gave different results, so every threshold below would be an artefact \
                         of trial order. Table suppressed."
                    );
                    continue;
                }
            }

            println!(
                "role\tmove\tw/v\tbase\tgrowth\tlaunch@100\ttumble%\tcentre_KO%\tledge_KO%\tko_ticks"
            );
            // ⛔⛔ DERIVED FROM THE PLATFORM, AND INBOARD OF ITS EDGE.
            //
            // `centre + 240.0` parked the victim at x=560, which is EXACTLY
            // where the floor ends: `smash_stage` builds one solid at
            // min=(80,300) size=(480,32), so the platform spans 80..560. A body
            // standing on the last pixel decides `on_ground` sub-pixel, and it
            // showed — 15 of run 2's 16 self-contradicting cells were LEDGE
            // cells, and four ledge thresholds collapsed to 0-25%. That is the
            // fixture tipping a body off a cliff it was already teetering on,
            // not a measurement of knockback.
            //
            // Read the real edge off the stage and stand a margin inboard, so the
            // cell measures a launch from NEAR the ledge instead of a coin flip
            // about whether the victim was ever standing.
            // ⭐ BOTH EDGES, because one of them is the wrong one for half the roster.
            // See the per-move choice at the `ledge_ko` call below.
            let (left_ledge_x, right_ledge_x) = {
                let room = ambition_demo_smash::smash_stage();
                let right = room
                    .world
                    .blocks
                    .iter()
                    .map(|b| b.aabb.max.x)
                    .fold(f32::MIN, f32::max);
                let left = room
                    .world
                    .blocks
                    .iter()
                    .map(|b| b.aabb.min.x)
                    .fold(f32::MAX, f32::min);
                if right > f32::MIN && left < f32::MAX {
                    (left + 40.0, right - 40.0)
                } else {
                    (probe.centre - 200.0, probe.centre + 200.0)
                }
            };
            println!(
                "#   ledge_x: left={left_ledge_x:.1} right={right_ledge_x:.1} (derived platform \
                 edges, 40px inboard). ⭐ The LEDGE column measures the OUTWARD ledge, chosen \
                 per move by the sign of its launch_dir.x — NOT always the right one."
            );
            for l in &launchers {
                let pulse = l.pulse();
                let move_id = l.move_id();
                // ⛔ THROUGH `launch_at` LIKE THE OTHER TWO. This value is both
                // the printed column AND the midpoint the linearity check tests
                // the fitted line against, so a single refused reset here does
                // not merely blank a cell — it disarms the check that decides
                // whether the tumble crossing may be solved at all.
                let launch_at_100 = probe.launch_at(pulse, 100, probe.centre);
                // ⭐ THE TUMBLE CROSSING IS SOLVED, NOT SWEPT — and the solve
                // CHECKS ITS OWN PREMISE instead of assuming it.
                //
                // The launch law is linear in victim percent, so three
                // samples answer what a 13-trial sweep answered: two fix the
                // line, the third must land on it. Measured against the slow
                // path's own jab row (base 55.0, growth 1.10): l(0)=55.0,
                // l(200)=330.0, midpoint 192.5 — and the engine reported exactly
                // 192.5 at 100%, which is what makes the line trustworthy here.
                //
                // ⛔ If the third point misses, this prints NONLINEAR and no
                // crossing, because a fitted line through a curve that is not
                // one is a fabricated number.
                let l0 = probe.launch_at(pulse, 0, probe.centre);
                let l200 = probe.launch_at(pulse, 200, probe.centre);
                let tumble_cell = match (l0, launch_at_100, l200) {
                    (Some(a), Some(mid), Some(b)) => {
                        let predicted = (a + b) / 2.0;
                        if (predicted - mid).abs() > 0.5 {
                            format!("NONLINEAR({a:.1}/{mid:.1}/{b:.1})")
                        } else if probe.tumble_speed <= 0.0 {
                            "no-tumble-speed".to_string()
                        } else if a >= probe.tumble_speed {
                            "0".to_string()
                        } else {
                            let slope = (b - a) / 200.0;
                            if slope <= 0.0 {
                                format!(">{max_percent}")
                            } else {
                                let cross = ((probe.tumble_speed - a) / slope).ceil() as i32;
                                if cross > max_percent {
                                    format!(">{max_percent}")
                                } else {
                                    cross.to_string()
                                }
                            }
                        }
                    }
                    _ => "-".to_string(),
                };
                let centre_ko = probe.ko_threshold(pulse, probe.centre, max_percent);
                // ⛔⛔ THE OUTWARD LEDGE, CHOSEN PER MOVE — AND THE OLD COLUMN WAS
                // ANSWERING A DIFFERENT QUESTION FOR HALF THE ROSTER.
                //
                // Every move used to be measured at the RIGHT ledge. For a
                // negative-x launcher that aims the victim back ACROSS THE WHOLE
                // STAGE, and the result was still labelled `ledge_KO%`. Measured
                // before this fix, the two backward movers were exactly the two
                // rows that looked "inverted" (ledge HARDER than centre):
                // `back_throw` ledge 271 vs centre 135, `attack_air_back` 282 vs 200.
                // That is not ledge kill potential; it is cross-stage kill potential.
                //
                // The fix is not to special-case throw names — it is to let the
                // move's own authored launch direction pick which edge is OUTWARD.
                // `facing` is pinned +1, so world launch follows the sign of
                // `launch_dir.x`; a `None` dir derives from attacker-relative
                // position and the fixture stands the attacker to the LEFT, so it
                // launches rightward and keeps the right ledge.
                let ledge_x = match l.launch_dir() {
                    Some((x, _)) if x < -0.05 => left_ledge_x,
                    _ => right_ledge_x,
                };
                let ledge_ko = probe.ko_threshold(pulse, ledge_x, max_percent);
                // ⛔ `w0v0` WOULD BE A LIE ON A THROW. These indices ADDRESS a
                // strike's hit volume inside its move; a throw has no such
                // address, and zeros would read as "window 0, volume 0" — a real
                // location — rather than "does not apply".
                let wv = match l {
                    Launcher::Strike { window, volume, .. } => format!("w{window}v{volume}"),
                    Launcher::Throw { .. } => "-".to_string(),
                };
                // ⭐ LAUNCH-MOTION TIMING, which is a named suspect in this
                // investigation and was being recorded and thrown away — the
                // compiler said so ("field `ticks_to_ko` is never read"). Ticks
                // from contact to blast line at the cell's own threshold is how
                // "the launch feels slow" stops being a matter of opinion.
                // ⛔ BY REFERENCE: `Threshold::NonMonotonic` carries a `Vec`, so
                // this enum is not `Copy` and matching it by value would move
                // `centre_ko` out from under the `centre_ko.cell()` below.
                let ko_ticks = match &centre_ko {
                    Threshold::At(p) => match probe.fire(pulse, *p, probe.centre) {
                        Some(t) => {
                            // ⭐⭐ WHETHER THIS CELL MEASURES KILL POWER AT ALL.
                            //
                            // A threshold is evidence about knockback only if the
                            // victim crossed the blast line while it could not act.
                            // If control had already returned, the move did not take
                            // the stock — the body FELL, and the stage took it. Both
                            // outcomes have been printing as one number in every
                            // table this tool has produced, which flatters exactly
                            // the weak-but-far moves a calibration would then
                            // "correct" by raising growth.
                            //
                            // ⛔ ON STDERR, NOT AS A COLUMN. Three gates diff this
                            // table byte-for-byte against `envelope_AFTER_LAWONLY`;
                            // widening it would destroy the only comparison road
                            // still standing.
                            eprintln!(
                                "KO_FORCED: move={move_id} x=centre pct={p} forced={} \
                                 — `no` means the victim had CONTROL BACK when it left \
                                 the world, so this cell is a FALL, not kill power",
                                match t.ko_forced {
                                    Some(true) => "yes",
                                    Some(false) => "no",
                                    None => "no-ko-in-verification-trial",
                                }
                            );
                            t.ticks_to_ko
                                .map(|v| v.to_string())
                                .unwrap_or_else(|| "-".into())
                        }
                        None => "-".to_string(),
                    },
                    _ => "-".to_string(),
                };
                println!(
                    "{}\t{move_id}\t{wv}\t{:.1}\t{}\t{}\t{}\t{}\t{}\t{}",
                    l.role(),
                    l.base_knockback(),
                    l.authored_growth()
                        .map(|g| format!("{g:.2}"))
                        .unwrap_or_else(|| "None".into()),
                    launch_at_100
                        .map(|v| format!("{v:.1}"))
                        .unwrap_or_else(|| "-".into()),
                    tumble_cell,
                    centre_ko.cell(),
                    ledge_ko.cell(),
                    ko_ticks,
                );
            }
        }
    }
}

fn main() {
    // ⭐ BEFORE `probe`, because a table measured with a non-independent trial is
    // worth less than no table: run 1 produced one and its oracle row had moved.
    if std::env::args().any(|a| a == "contact") {
        run_contact();
        return;
    }
    if std::env::args().any(|a| a == "determinism") {
        run_determinism();
        return;
    }
    // ⭐ BEFORE `probe` for the same reason `contact` is: a matrix whose throw
    // rows are all REFUSED is not a measurement of throws, and this says which.
    if std::env::args().any(|a| a == "throwdiag") {
        run_throw_diag();
        return;
    }
    if std::env::args().any(|a| a == "probe") {
        run_probe();
        return;
    }
    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    // ⛔ THE REGISTRY IS FILLED BY A `Startup` SYSTEM. A build that has never
    // updated has a catalog and NO REGISTRY AT ALL, so every id would miss and
    // the census would read as an empty roster.
    for _ in 0..4 {
        app.update();
    }

    let world = app.world();
    let registry = world
        .get_resource::<ambition_platformer2d::characters::prepared::PreparedCharacterRegistry>()
        .expect("the composed host has a prepared-character registry");
    // ⛔⛤ **NOT THE LIVE RESOURCE — AND THE FIRST RUN OF THIS TOOL PROVED WHY.**
    // `project_combat_rules` folds `DeclaredCombatRules` every tick, but the
    // SMASH declaration only exists once the Smash experience is entered. A
    // census that merely boots the app host and reads the resolved resource gets
    // `Default`: measured 2026-09-13, this printed
    // `victim_percent_knockback_scale=1 knockback_growth=0 rage_max_scale=1
    // stale_floor=1 stale_knockback_influence=1` where Smash declares
    // `1.25 / 0.02 / 1.4 / 0.55 / 0.30`. The `knockback_growth=0` then collapsed
    // every unauthored-growth volume's effective growth to ZERO.
    //
    // ⇒ Ask the DECLARATION, which is the authority and is `pub`, and fold it
    // the same way the engine does. The stage probe additionally asserts this
    // against the live resource once a real Smash match exists — a declaration
    // and a running world agreeing is a measurement; either alone is a claim.
    let declared = ambition_demo_smash::smash_declared_combat_rules();
    let rules = ambition_platformer2d::combat::rules::ResolvedCombatTuning::resolve(
        Some(declared),
        0.0,
        false,
    );
    assert!(
        rules.knockback_growth > 0.0,
        "the folded Smash ruleset reports knockback_growth={} — a zero fallback silently \
         reads every unauthored growth as zero, which is the defect this line replaced",
        rules.knockback_growth
    );

    // ⭐ THE POPULATION IS THE SHIPPED HOST'S OWN ANSWER, printed so the census
    // can never be read as covering a cast it did not cover.
    let roster = ambition_demo_smash::select::SmashRoster::assemble(registry);
    let ids: Vec<String> = roster.ids().map(|s| s.to_string()).collect();
    println!("# ko_envelope — launcher-pulse census");
    println!("# roster (SmashRoster::assemble, the seatable cast): {} fighters", ids.len());
    for id in &ids {
        println!("#   {id}");
    }
    println!("# ruleset: victim_percent_knockback_scale={} knockback_growth(fallback)={} \
              rage_max_scale={} stale_floor={} stale_knockback_influence={}",
        rules.victim_percent_knockback_scale,
        rules.knockback_growth,
        rules.rage_max_scale,
        rules.stale_floor,
        rules.stale_knockback_influence,
    );
    println!("# THROW ROAD ASYMMETRY (current engine facts, not endorsements):");
    println!("#   throws: percent_scale=YES  staling=NO  rage=NO  damage applied BEFORE launch");
    println!("#   strikes: percent_scale=YES staling=YES rage=YES");

    let mut all: Vec<Launcher> = Vec::new();
    let mut bad: Vec<Malformed> = Vec::new();
    let mut non_launching: Vec<NonLaunching> = Vec::new();
    for id in &ids {
        let Some(prepared) = registry.get(id) else {
            bad.push(Malformed {
                fighter: id.clone(),
                what: "registry".into(),
                why: "on the assembled roster but absent from the registry".into(),
            });
            continue;
        };
        let Some(contract) = prepared.kit.projectable_moveset() else {
            bad.push(Malformed {
                fighter: id.clone(),
                what: "moveset".into(),
                why: "prepared with no projectable moveset".into(),
            });
            continue;
        };
        all.extend(launchers_of(id, contract, &mut bad, &mut non_launching));
    }

    println!("\nfighter\tmove_id\tkind\trole\tverbs\twindow\tvolume\tdamage\tbase_kb\tauthored_growth\teffective_growth\tgrowth_over_base\tfixed\tlaunch_dir\tkey");
    for l in &all {
        let (w, v, verbs) = match l {
            Launcher::Strike {
                window,
                volume,
                verbs,
                ..
            } => (window.to_string(), volume.to_string(), verbs.join("|")),
            Launcher::Throw { verb, .. } => ("-".into(), "-".into(), verb.clone()),
        };
        let eg = l.effective_growth(rules.knockback_growth);
        let base = l.base_knockback();
        let ratio = if base > 0.0 { eg / base } else { f32::NAN };
        let dir = l
            .launch_dir()
            .map(|(x, y)| format!("({x:.2},{y:.2})"))
            .unwrap_or_else(|| "default".into());
        println!(
            "{}\t{}\t{}\t{}\t{}\t{w}\t{v}\t{}\t{:.1}\t{}\t{:.4}\t{:.4}\t{}\t{dir}\t{}",
            l.fighter(),
            l.move_id(),
            l.kind(),
            l.role(),
            verbs,
            l.damage(),
            base,
            l.authored_growth()
                .map(|g| format!("{g:.3}"))
                .unwrap_or_else(|| "None".into()),
            eg,
            ratio,
            l.is_fixed(),
            l.key(),
        );
    }

    // ⭐⭐ BOUND ROLES ONLY, SIDE BY SIDE — the one table the whole instrument
    // exists to put in front of a reader.
    //
    // ⛔ UNBOUND PULSES ARE EXCLUDED HERE AND COUNTED SEPARATELY. 115 of 532
    // strike pulses answer to NO verb (`attack`, `attack_up`, `attack_air*` and
    // friends, identical across 15 fighters): a press cannot reach them, so
    // letting them into a role distribution drags every median toward a
    // stand-in's numbers while looking like content.
    //
    // ⛔ AND NO SPREAD RATIO IS PRINTED. Some authored pulses have a genuinely
    // zero growth/base — fixed knockback is a real authored role — so a
    // max/min ratio over this set divides by a legitimate zero. Min, median and
    // max say what a ratio would have, without being undefined.
    {
        let mut bound: BTreeMap<String, Vec<&Launcher>> = BTreeMap::new();
        let mut unbound = 0usize;
        for l in &all {
            match l {
                Launcher::Strike { verbs, .. } if verbs.is_empty() => unbound += 1,
                _ => bound.entry(l.role()).or_default().push(l),
            }
        }
        println!("\n# BOUND-ROLE COMPARISON (reachable pulses only)");
        println!("# excluded: {unbound} unbound strike pulses answering to no verb");
        println!("role\tn\tbase_med\tgrowth_med\tratio_min\tratio_med\tratio_max");
        for (role, ls) in &bound {
            let bases = dist(ls.iter().map(|l| l.base_knockback()).collect());
            let growths = dist(
                ls.iter()
                    .map(|l| l.effective_growth(rules.knockback_growth))
                    .collect(),
            );
            let ratios = dist(
                ls.iter()
                    .filter(|l| l.base_knockback() > 0.0)
                    .map(|l| l.effective_growth(rules.knockback_growth) / l.base_knockback())
                    .collect(),
            );
            println!(
                "{role}\t{}\t{:.1}\t{:.3}\t{:.4}\t{:.4}\t{:.4}",
                ls.len(),
                bases.median,
                growths.median,
                ratios.min,
                ratios.median,
                ratios.max
            );
        }
    }

    // ROLE SUMMARIES — the shape a gameplay question can actually be asked of.
    let mut by_role: BTreeMap<String, Vec<&Launcher>> = BTreeMap::new();
    for l in &all {
        by_role.entry(l.role()).or_default().push(l);
    }
    println!("\n# role summaries (n / min / p25 / median / p75 / max)");
    println!("role\tcol\tn\tmin\tp25\tmedian\tp75\tmax");
    for (role, ls) in &by_role {
        for (col, xs) in [
            ("base_kb", ls.iter().map(|l| l.base_knockback()).collect::<Vec<_>>()),
            (
                "effective_growth",
                ls.iter().map(|l| l.effective_growth(rules.knockback_growth)).collect(),
            ),
            (
                "growth_over_base",
                ls.iter()
                    .filter(|l| l.base_knockback() > 0.0)
                    .map(|l| l.effective_growth(rules.knockback_growth) / l.base_knockback())
                    .collect(),
            ),
            ("damage", ls.iter().map(|l| l.damage() as f32).collect()),
        ] {
            let d = dist(xs);
            println!(
                "{role}\t{col}\t{}\t{:.4}\t{:.4}\t{:.4}\t{:.4}\t{:.4}",
                d.n, d.min, d.p25, d.median, d.p75, d.max
            );
        }
    }

    println!("\n# totals: {} launchers ({} strikes, {} throws) over {} fighters",
        all.len(),
        all.iter().filter(|l| matches!(l, Launcher::Strike { .. })).count(),
        all.iter().filter(|l| matches!(l, Launcher::Throw { .. })).count(),
        ids.len(),
    );
    if non_launching.is_empty() {
        println!("# non-launching capture slots: none");
    } else {
        println!(
            "# NON-LAUNCHING CAPTURE SLOTS: {} (authored carries — NOT gaps, and not to be \
             \"fixed\" into throws)",
            non_launching.len()
        );
        for c in &non_launching {
            println!("#   {} · {} · {} · CAPTURE_CARRY", c.fighter, c.slot, c.move_id);
        }
    }
    if bad.is_empty() {
        println!("# malformed rows: none");
    } else {
        println!("# MALFORMED ROWS: {}", bad.len());
        for m in &bad {
            println!("#   {} · {} · {}", m.fighter, m.what, m.why);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Boot the composed host once and hand back the registry's view.
    fn composed() -> bevy::prelude::App {
        let mut app = ambition_app::app::build_visible_app(
            ambition_app::app::VisibleRenderMode::NoWindow,
            true,
        );
        for _ in 0..4 {
            app.update();
        }
        app
    }

    /// ⛔ THE CENSUS POPULATION IS A CLAIM. An empty roster would make every
    /// downstream distribution vacuously "fine".
    #[test]
    fn the_composed_smash_roster_is_not_empty() {
        let app = composed();
        let registry = app
            .world()
            .get_resource::<ambition_platformer2d::characters::prepared::PreparedCharacterRegistry>()
            .expect("registry");
        let roster = ambition_demo_smash::select::SmashRoster::assemble(registry);
        let n = roster.ids().count();
        assert!(
            n > 0,
            "SmashRoster::assemble produced NO seatable fighters, so a census over it \
             would report an empty game rather than a defect"
        );
    }

    /// ⭐⭐ THIS TEST REPLACES EVERY GREP THIS FILE'S HEADER DESCRIBES.
    ///
    /// It asserts the thing four source parsers got wrong: the roster's fighters
    /// really do author throws, reachable by VERB through the runtime contract.
    #[test]
    fn every_resolved_throw_verb_carries_exactly_one_hydratable_throw_effect() {
        let app = composed();
        let registry = app
            .world()
            .get_resource::<ambition_platformer2d::characters::prepared::PreparedCharacterRegistry>()
            .expect("registry");
        let roster = ambition_demo_smash::select::SmashRoster::assemble(registry);

        let mut resolved = 0usize;
        let mut carries = 0usize;
        let mut fighters_with_throws = 0usize;
        for id in roster.ids() {
            let Some(prepared) = registry.get(id) else {
                continue;
            };
            let Some(contract) = prepared.kit.projectable_moveset() else {
                continue;
            };
            let mut mine = 0usize;
            for (slot, verb) in THROW_VERBS {
                let Some(spec) = contract.move_for_verb(verb) else {
                    continue;
                };
                let throws: Vec<_> = spec
                    .effect_refs()
                    .into_iter()
                    .filter(|(_, e)| e.key == CAPTURE_THROW)
                    .collect();
                let carried = spec
                    .effect_refs()
                    .into_iter()
                    .filter(|(_, e)| e.key == CAPTURE_CARRY)
                    .count();
                // ⛔ A CARRY IS NOT A DEFECT. The goblin's down press hoists
                // instead of launching, on purpose, guarded by
                // `the_goblins_down_throw_hauls_instead_of_launching`. The first
                // version of this arm asserted `throws.len() == 1` outright and
                // reddened on a CORRECT fighter — a repair from here would have
                // broken the game to green the instrument.
                if throws.is_empty() && carried > 0 {
                    carries += 1;
                    continue;
                }
                assert_eq!(
                    throws.len(),
                    1,
                    "`{id}`'s {slot} resolves to `{}` carrying {} `{CAPTURE_THROW}` and \
                     {carried} `{CAPTURE_CARRY}` effects. A capture slot is a throw (exactly \
                     one throw effect) or a carry (at least one carry effect and no throw); \
                     this is neither.",
                    spec.id,
                    throws.len()
                );
                throws[0]
                    .1
                    .params
                    .hydrate::<CaptureThrowParams>()
                    .unwrap_or_else(|e| {
                        panic!("`{id}`'s {slot} params did not hydrate as CaptureThrowParams: {e}")
                    });
                mine += 1;
                resolved += 1;
            }
            if mine > 0 {
                fighters_with_throws += 1;
            }
        }
        // ⭐ THE FLOOR IS THE POINT. Without it this arm passes on a roster where
        // every lookup silently returns `None` — which is precisely the
        // conclusion four source-parsing attempts reached, and every one of them
        // was wrong. A bare "no malformed rows" cannot tell an empty census from
        // a healthy one.
        assert!(
            resolved >= 20,
            "only {resolved} throws resolved across the whole roster ({carries} carries). \
             Every fighter authors a capture kit, so a number this small is a BROKEN LOOKUP \
             rather than a game without throws."
        );
        println!("{resolved} throws + {carries} carries over {fighters_with_throws} fighters");
    }

    /// `Some(0.0)` is FIXED knockback, and it must not be folded into "the
    /// volume did not decide". Reading a bare `0.0` as unspecified once made the
    /// documented fixed-knockback case the one value nobody could author.
    #[test]
    fn a_zero_growth_stays_fixed_and_an_absent_growth_takes_the_ruleset_fallback() {
        let fixed = Launcher::Throw {
            fighter: "f".into(),
            move_id: "m".into(),
            verb: CAPTURE_THROW_UP_VERB.into(),
            slot: "up_throw",
            params: CaptureThrowParams {
                damage: 6,
                knockback: 145.0,
                knockback_growth: 0.0,
                launch_dir: (0.0, -1.0),
            },
        };
        assert!(fixed.is_fixed(), "an authored 0.0 growth is fixed knockback");
        assert_eq!(
            fixed.effective_growth(0.02),
            0.0,
            "a FIXED throw must not silently acquire the ruleset's fallback growth"
        );

        let unauthored = Launcher::Strike {
            fighter: "f".into(),
            move_id: "m".into(),
            verbs: vec!["attack".into()],
            window: 0,
            volume: 0,
            hit: HitVolume {
                knockback_growth: None,
                ..sample_volume()
            },
        };
        assert!(!unauthored.is_fixed(), "`None` is not fixed knockback");
        assert!(
            (unauthored.effective_growth(0.02) - 100.0 * 0.02).abs() < 1e-6,
            "an unauthored growth resolves to base × the ruleset's growth, not zero"
        );
    }

    /// A role must come from an AUTHORED BINDING, never from guessing at a name.
    #[test]
    fn a_strikes_role_comes_from_its_authored_verb_not_its_id() {
        let l = Launcher::Strike {
            fighter: "f".into(),
            move_id: "looks_like_a_jab".into(),
            verbs: vec!["attack".into(), "attack_forward_strong".into()],
            window: 0,
            volume: 0,
            hit: sample_volume(),
        };
        assert_eq!(
            l.role(),
            "attack_forward_strong",
            "the most specific authored binding names the role"
        );
        let unbound = Launcher::Strike {
            fighter: "f".into(),
            move_id: "orphan".into(),
            verbs: vec![],
            window: 0,
            volume: 0,
            hit: sample_volume(),
        };
        assert_eq!(
            unbound.role(),
            "unbound:orphan",
            "a move no verb names is reported as unbound rather than guessed at"
        );
    }

    #[test]
    fn a_distribution_reports_its_own_count() {
        let d = dist(vec![1.0, 2.0, 3.0, 4.0]);
        assert_eq!(d.n, 4);
        assert_eq!(d.min, 1.0);
        assert_eq!(d.max, 4.0);
        assert_eq!(dist(vec![]).n, 0, "an empty column reports n=0, not a value");
    }

    fn sample_volume() -> HitVolume {
        HitVolume {
            shape: ambition_entity_catalog::VolumeShape::Rect {
                offset: (0.0, 0.0),
                half_extents: (10.0, 10.0),
            },
            damage: 5,
            knockback: 100.0,
            knockback_growth: Some(2.0),
            launch_dir: None,
            reaction: None,
            on_hit: None,
            vfx: None,
            hit_sfx: None,
        }
    }
}

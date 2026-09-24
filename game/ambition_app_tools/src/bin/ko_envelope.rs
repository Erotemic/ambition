//! `ko_envelope`: a launcher-pulse census over the prepared moveset graph,
//! and a stage-outcome probe over the same launchers.
//!
//! Rule: read authoring from the compiled runtime contract, drive outcomes
//! through the real simulation, and reconstruct neither from source text.
//! Text searches over `*_moveset.rs` miss named constants, `Some(..)` forms,
//! and files, so this tool does not use them. A throw is found by asking the
//! contract for a verb. Its numbers are hydrated from the same typed
//! `EffectRef` the engine reads.
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

// A `#[path]` module, not a lib: this package has no lib on purpose, because a
// lib relinks every binary when any of them changes. `trap_probe` and
// `wire_probe` include this file the same way. Its header records the stage
// fixture pitfalls it handles.
#[path = "../probe_stage.rs"]
mod probe_stage;

use ambition_entity_catalog::smash_capture::{CaptureThrowParams, CAPTURE_CARRY, CAPTURE_THROW};
use ambition_entity_catalog::{
    HitVolume, MovesetContract, WindowTag, CAPTURE_THROW_BACK_VERB, CAPTURE_THROW_DOWN_VERB,
    CAPTURE_THROW_FORWARD_VERB, CAPTURE_THROW_UP_VERB,
};

/// The four canonical throw verbs, in report order.
///
/// These are the catalog's own constants (re-exported by
/// `smash_capture::verbs`), so a lookup cannot disagree with what
/// `SmashCaptureRepertoire::bound` installed.
const THROW_VERBS: [(&str, &str); 4] = [
    ("forward_throw", CAPTURE_THROW_FORWARD_VERB),
    ("back_throw", CAPTURE_THROW_BACK_VERB),
    ("up_throw", CAPTURE_THROW_UP_VERB),
    ("down_throw", CAPTURE_THROW_DOWN_VERB),
];

/// One authored pulse that can launch a body, in the two forms this game
/// authors.
///
/// This is a projection for the balance instrument only, not a gameplay
/// abstraction. The engine does not know about it.
#[derive(Debug, Clone)]
enum Launcher {
    /// One `HitVolume` of one `Active` window.
    ///
    /// One volume is a pulse, not a move. A tipper and a sourspot have
    /// different envelopes, and an earlier hit of a multi-hit move adds damage
    /// before the finisher. A threshold measured here is a pulse threshold.
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
    /// This is the only place the measuring path inspects the variant.
    /// Downstream code takes a `Pulse`, so a throw cannot fall into a
    /// strike-only path.
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

    /// What the author wrote. `None` on a strike means "the volume did not
    /// decide". A throw always states one.
    fn authored_growth(&self) -> Option<f32> {
        match self {
            Launcher::Strike { hit, .. } => hit.knockback_growth,
            Launcher::Throw { params, .. } => Some(params.knockback_growth),
        }
    }

    /// What the engine will use.
    ///
    /// `None` is not zero. `resolved_hitbox_knockback_magnitude` resolves an
    /// unauthored growth to `base × ruleset growth`. `Some(0.0)` is the
    /// fixed-knockback case.
    fn effective_growth(&self, ruleset_growth: f32) -> f32 {
        self.authored_growth()
            .unwrap_or_else(|| self.base_knockback() * ruleset_growth.max(0.0))
    }

    /// Fixed knockback: the launch ignores victim percent and weight, because
    /// the launch law returns `base` for a zero growth.
    fn is_fixed(&self) -> bool {
        self.authored_growth() == Some(0.0)
    }

    fn launch_dir(&self) -> Option<(f32, f32)> {
        match self {
            Launcher::Strike { hit, .. } => hit.launch_dir,
            Launcher::Throw { params, .. } => Some(params.launch_dir),
        }
    }

    /// The role this pulse answers to, from authored verb bindings.
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
/// Malformed rows are reported, not dropped, so a skip is not read as absence.
#[derive(Debug)]
struct Malformed {
    fighter: String,
    what: String,
    why: String,
}

/// A capture slot that resolves, is authored on purpose, and does not launch.
///
/// Recorded so its absence from the KO table reads as intent. Example: the
/// goblin's down press carries its captive (`CAPTURE_CARRY`) instead of
/// throwing it.
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

    // Strikes: every volume of every Active window, never collapsed.
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

    // Throws: found by verb, hydrated from the engine's own effect.
    //
    // A resolved capture verb is not always a launcher. A slot can carry
    // `CAPTURE_CARRY` instead (the goblin's down press; guarded by
    // `the_goblins_down_throw_hauls_instead_of_launching`). So there are three
    // outcomes: a throw (measured), a carry (recorded as intent), or absent.
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
// Phase 2: the stage-outcome probe
//
// A KO is not a reset. `measure_cell` in `smash_in_the_host.rs` re-seats a
// match per cell, which is too slow for thousands of trials. This probe reuses
// one match behind an explicit reset contract, because a KO spends a stock,
// opens `DeathInterlude`, adds `PendingRespawn`/`OutOfPlay`, grants
// `RespawnGrace` (which publishes `Invulnerability::RESPAWN`), teleports the
// body, and leaves `BodyKnockedOut` messages that a fresh cursor would read as
// the next trial's result. Each reset clause below is asserted.
// ─────────────────────────────────────────────────────────────────────────────

/// What one pulse did to one victim at one percent.
#[derive(Debug, Clone)]
struct Trial {
    /// Read off `HitEvent.knockback` — the engine's own resolved number, never
    /// recomputed here. `None` means no `LaunchSpeed` reached the victim, which
    /// is a fixture failure rather than a measurement of zero.
    resolved_launch: Option<f32>,
    ko: bool,
    /// Did the launch reach this body's own tumble threshold (read live off
    /// its motion model).
    tumbled: bool,
    ticks_to_ko: Option<usize>,
    /// Victim state at the moment the pulse was fired, for the determinism
    /// probe. Two trials of the same pulse at the same percent must agree here.
    at_strike: String,
    /// The meter the victim actually entered the pulse carrying.
    entry_percent: i32,
    /// What the launch arithmetic saw. For a throw this is `entry + damage`,
    /// because `apply_capture_throws` damages before reading the meter.
    effective_percent: i32,
    /// Was the victim still non-actionable when it crossed the blast line?
    ///
    /// If yes, the move took the stock. If control had returned, the body fell,
    /// and that is not evidence about knockback. `None` when no KO occurred.
    ko_forced: Option<bool>,
}

/// The outcome of a threshold search, including the ones that are not a number.
#[derive(Debug)]
enum Threshold {
    /// Lowest entry percent that KOs, verified: `at-1` survives, `at` kills.
    At(i32),
    /// Never killed anywhere in the tested range.
    Above(i32),
    /// KO then survive as percent rises. Samples are kept, not collapsed into
    /// a scalar.
    NonMonotonic(Vec<(i32, bool)>),
    /// The pulse never connected — nothing measured.
    NoContact,
    /// The probe could not obtain a clean trial at this percent.
    ///
    /// This is not a survival. Recording it as one would let the threshold
    /// search walk past a percent that was never measured.
    Refused(i32),
    /// Calibration only: trials at one percent disagreed with each other.
    ///
    /// The coarse path hides this with majority-of-three. Calibration refuses
    /// the cell instead, because its result feeds `G_new = G_old * p0/p1` and
    /// becomes a shipped growth.
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

/// What the trials at one percent said.
///
/// Not a `bool`: collapsing trials with `yes > no` hides a 2:1 split, and
/// calibration must see that split.
#[derive(Clone, Copy)]
enum Vote {
    Killed,
    Survived,
    /// The same initial state produced both outcomes. Counts kept, because
    /// 2:1 and 1:2 are different evidence about where the edge sits.
    Mixed { yes: i32, no: i32 },
}

impl Vote {
    /// The majority-of-three verdict.
    ///
    /// The coarse path still uses this, so new tables stay comparable with
    /// earlier runs. Calibration refuses a mixed cell instead.
    fn majority(self) -> bool {
        match self {
            Vote::Killed => true,
            Vote::Survived => false,
            Vote::Mixed { yes, no } => yes > no,
        }
    }
}

/// Unanimous votes print as `true`/`false`, the same as the old `bool`
/// output, so determinism-probe output stays comparable across runs. Only a
/// mixed cell prints a new string.
impl std::fmt::Display for Vote {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Vote::Killed => write!(f, "true"),
            Vote::Survived => write!(f, "false"),
            Vote::Mixed { yes, no } => write!(f, "MIXED({yes}y/{no}n)"),
        }
    }
}

/// The attacker's own meter for every trial: `attacker_pct=<n>`, default 0.
///
/// This is a control. `rage_scale` reads the attacker's damage and scales every
/// launch, so a floating value would mix fixture damage into the result.
/// A kill percent at rage 1.0 is the weakest the move gets; set this flag to
/// ask about other rage values. Cached, because `reset_trial` calls it once per
/// trial.
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

/// What delivers one pulse. This is the only thing the threshold machinery
/// varies.
///
/// Strikes and throws share one search (`ko_threshold`) and differ only in
/// delivery, so there is one set of rules for refusals, non-monotonic curves,
/// verification, and majority votes. Two references, so this is `Copy`.
#[derive(Clone, Copy)]
enum Pulse<'a> {
    Strike(&'a HitVolume),
    Throw(&'a CaptureThrowParams),
}

/// The stage's blast envelope, as the running game holds it.
///
/// Read off the live `RoomGeometry`, never copied from the demo's constants
/// (`FALL_BLAST_MARGIN_PX` and others). A copy would be a second stage that can
/// drift from the first. `KoProbe::new` follows the same rule for `centre`.
#[derive(Clone, Copy)]
struct Blast {
    size: ambition_platformer2d::engine_core::Vec2,
    fall: f32,
    side: Option<f32>,
    rise: Option<f32>,
}

impl Blast {
    /// Which line the body crossed. `HitSource::LeftTheWorld` does not say.
    ///
    /// Uses the same arithmetic as `apply_world_hazard_gate`: clamp into the
    /// world box, project the excess onto the frame's down/side axes, and
    /// compare with the per-axis margin. Gravity is screen-down on this stage,
    /// so `down` is +y.
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
            // The engine declared the body out, but the sample is inside every
            // margin: the sampled position is not the one the gate judged.
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
    /// The live blast envelope, or `None` when the app publishes no
    /// `RoomGeometry`. Reported as absent, never filled from demo constants.
    blast: Option<Blast>,
    /// How many trials had to release the victim from a ledge before they
    /// could start. This counter confirms the ledge-hang diagnosis.
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

        // Stocks first, for correctness: `STARTING_STOCKS` is 3, and
        // `take_eliminated_fighters_out_of_play` despawns an eliminated body,
        // so the fourth KO would query a destroyed entity.
        for body in [seat0, seat1] {
            if let Some(mut stocks) = app
                .world_mut()
                .get_mut::<ambition_platformer2d::actor::FighterStocks>(body)
            {
                stocks.remaining = 9_999;
                stocks.started_with = 9_999;
            }
            // Both brains. A live CPU victim moves during the window and can
            // reset its own meter mid-reading.
            app.world_mut().entity_mut(body).insert(Brain::stand_still());
        }
        app.update();

        // Optional: run with the growth curve off (`identity`).
        //
        // `GrowthBaseCurve` is evidence, not shipped tuning, and calibration
        // needs a curve-free floor. A flag keeps curve-on and curve-off in one
        // binary, so no tree difference enters the comparison.
        //
        // Write the declaration, not `ResolvedCombatTuning`: the resolved
        // resource is re-derived from `DeclaredCombatRules` every tick. The
        // override is read back through the fold and asserted, because a silent
        // failure would look like "the curve does nothing".
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

        // Read off the real stage, as `ring_out::stage_centre_and_reach` does,
        // never restated from the demo's constants.
        let room = ambition_demo_smash::smash_stage();
        let centre = room.world.size.x / 2.0;

        // The victim's own tumble threshold, off its live motion model.
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

        // The blast envelope from the live world. `RoomGeometry` wraps the
        // active room's collision world, the same `World` whose `edges`
        // `apply_world_hazard_gate` reads to decide `ResetCause::LeftTheWorld`.
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

        // Every parameter the knockback arithmetic depends on, read off the
        // live victim and stage. Printed per matchup, because these are
        // per-victim facts. None is taken from a source constant.
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
            // Report the absence, so the KO-boundary column does not read
            // "unknown" as if it were a result.
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

    /// The gravity this body is integrated at, read, never named.
    ///
    /// `platformer_defaults.ron` gives 2250, but that is the player's value. A
    /// match seat takes the `Without<PlayerEntity>` arm of
    /// `resolve_body_motion_frames`: `config.tuning.movement.gravity *
    /// surface.gravity_scale` (1450 here). This prints three values side by
    /// side: the resolved frame, the configured tuning, and the surface scale.
    /// A capture or mount sets `gravity_scale` to 0.0.
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

    /// Is the victim hanging on a ledge?
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

    /// Every victim fact a trial could inherit, in one tab-free field.
    ///
    /// Not a health check. `reset_trial` asserts what it knows about. This
    /// reports state for what it does not know about: compare a pre-KO and a
    /// post-KO trial to find the differing column.
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

    /// The reset contract. A trial does not begin until each condition is
    /// true. Each one is checked, not waited out by tick count.
    fn reset_trial(&mut self, entry_percent: i32, victim_x: f32) -> bool {
        use ambition_platformer2d::characters::actor::{BodyHealth, Invulnerability};

        // The match can end under the probe. Then the cast is despawned,
        // `self.victim` is a dangling handle, `pos()` returns `(0,0)` from
        // `unwrap_or_default()`, and every later trial refuses. Refused rows
        // read as facts about moves, so this aborts loudly instead of returning
        // `false`. Rows printed before the abort are real and are kept.
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

        // 1. Let the real respawn lifecycle finish.
        //    `respawn_when_the_interlude_closes` gates on `!DeathInterlude.open()`
        //    and then removes the markers, so waiting on the markers waits on
        //    the engine.
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
            // 2. Both grace witnesses. `stocks.rs` clears the bit when the clock
            //    expires and when the component is removed, so both must agree.
            //    A trial under live grace hits an untouchable victim and reports
            //    a false survival.
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
            // Name the marker that still blocks when the budget runs out, so
            // a refusal says why.
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
        // Release the ledge first, or the ledge hang overwrites the pin every
        // tick. A KO'd fighter can catch an edge on the way back, and a
        // stand-still fixture never lets go, so the hang carries into every
        // later trial. The hang shows as the body at x=574 or x=66 (the
        // platform spans 80..560, the body is 30 wide) with zero velocity.
        //
        // `knock_off_ledge` is the sanctioned release: the typed
        // combat->movement op that a real hit uses. It also arms
        // `LEDGE_KNOCK_OFF_COOLDOWN` (0.35s), so the body cannot re-latch at
        // once. Do not write `axis.state.ledge_grab` directly.
        let released = {
            let world = self.app.world_mut();
            let mut q = world.query::<(
                &mut ambition_platformer2d::actor::MotionModel,
                &mut ambition_platformer2d::engine_core::BodyLedgeState,
            )>();
            match q.get_mut(world, self.victim) {
                Ok((mut model, mut ledge)) => {
                    // `&mut *`, not `&mut`: `get_mut` returns `Mut<T>`, and Rust
                    // does not auto-deref a function argument.
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
            // Print each release. `knock_off_ledge` returns true only when it
            // removed a hang, so each line is one trial that was latched. This
            // output confirms the ledge-hang diagnosis.
            let p = self.pos(self.victim);
            eprintln!(
                "KO_LEDGE_RELEASE: n={} x={victim_x:.0} pct={entry_percent} \
                 was_at=({:.1},{:.1})",
                self.ledge_releases, p.x, p.y,
            );
        }
        self.park(self.victim, victim_x);
        self.park(self.attacker, victim_x - 240.0);
        // Record whether the landing loop succeeded. `landed_y` is read
        // unconditionally, so a loop that runs out of budget gives a mid-fall
        // height. The tick separates "never landed" from "landed and then
        // moved".
        let mut landed_after: Option<usize> = None;
        // Record the descent, so a refusal with `landed_after=None` shows why.
        //
        // A match seat falls at 1450 (`BodyMovementTuning::BASELINE.gravity`
        // via the `Without<PlayerEntity>` arm of `resolve_body_motion_frames`),
        // not the player's 2250. One `app.update()` is one sim tick.
        //
        // A body that sits at x=574 or x=66 with the same pose in every sample
        // is in a ledge hang, not falling through the platform. See the
        // release above. `is_contact_range_snap` caps any pushout at the body's
        // half-diagonal (~28px), so depenetration cannot explain a large move.
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
        // Do not `park` again here. `park` sets y = 200.0, above the platform
        // (PLATFORM_TOP is 300), so every trial would start airborne. An
        // airborne body takes a different path when struck, which makes
        // identical trials disagree.
        //
        // Correct x through the pin and keep the landed height. Do not use
        // `transit_body`: this body slides along a floor it already stands on,
        // and the transit authority clears that contact. See `pin`.
        let landed_y = self.pos(self.victim).y;
        probe_stage::pin_grounded_at_rest(
            &mut self.app,
            self.victim,
            ambition_platformer2d::engine_core::Vec2::new(victim_x, landed_y),
        );
        // The pose the pin achieved, read before any tick runs. This separates
        // "the write never reached this entity" from "something overrode it
        // during the next update".
        let pinned_at = self.pos(self.victim);
        self.app.update();

        // Check the premise: a trial starts only with the victim standing still
        // on the floor.
        let grounded = self
            .app
            .world()
            .get::<ambition_platformer2d::engine_core::BodyGroundState>(self.victim)
            .is_some_and(|g| g.on_ground);
        if !grounded {
            // This is the refusal that fires in practice, so it must explain
            // itself. An empty `KO_REFUSE` grep proves nothing without it.
            let p = self.pos(self.victim);
            // Print the suspects, do not guess:
            //
            //  * A staged launch. `constrain_body_pose` writes only `pos` and
            //    `vel` and does not clear contact facts, so a launch staged by
            //    the previous trial survives. The kernel spends `PendingLaunch`
            //    on the next free tick, which is this reset's own `update()`.
            //    Read it with `pending_launch_state`, not `take_launch`, which
            //    would spend it.
            //  * `carried_run`/`carried_hold`. `BodyFlightState` also survives
            //    the pin, and the airborne law does `approach(along, *carried_run,
            //    ..)`, which accelerates a body at rest.
            //  * `contact_initialized`. An invalidated baseline survives the pin,
            //    so `on_ground` reads false for a body resting on the floor.
            //  * `landed_after == None`. The loop never saw a landing, so
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
            // Is the probe still holding the right body? If a KO respawned the
            // fighter as a new entity, `self.victim` is stale and every later
            // trial refuses. Reported, not assumed.
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
                // A frozen body is either held (ledge) or not accelerated
                // (`gravity_scale`). Print both.
                self.victim_ledge(),
                self.victim_gravity(),
            );
            // The failed descent, on its own line so the one above stays
            // parseable. Parked at y=200; a resting body is at y=276; the
            // platform solid spans y 300..332.
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
            // Pin the attacker to zero: `rage_scale` reads its meter and
            // scales every resolved launch.
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
        // Assert the trial's premise after the reset.
        //
        // `reset_trial` proves the victim is grounded, not that it is
        // unencumbered. A staged launch, carried run/hold momentum, or a
        // latched ledge all survive `constrain_body_pose`, and each changes what
        // the pulse measures. This aborts instead of returning `None`, for the
        // same reason as the `seats_now == 0` guard: a refusal would read as a
        // measurement.
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
            // Loose, because this tests contamination, not precision. A tighter
            // bound would fail on float residue.
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

        // Make the cursors once, here, and advance them inside the loop.
        //
        // A cursor made now starts past everything the buffer holds, so the
        // reads see only this pulse's events. A cursor made per tick would
        // re-read earlier trials' KOs. bevy_ecs 0.19.1 has no
        // `get_cursor_from_end`; making the cursor before the loop is the
        // end-of-stream guarantee.
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

        // Delivery is the only thing that varies (see `Pulse`).
        //
        // The throw does not spend its own tick here. It installs the hold and
        // writes the request; the loop's first `update()` executes it, so the
        // launch's `HitEvent` lands after the cursors exist.
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
                // A throw happens at the captor, not at the victim's parked x.
                // The hold system moves the captive to `captor.pos +
                // hold_offset` every tick, so the victim goes to the attacker
                // before the throw resolves. Place the captor where the trial
                // says the throw happens: `hold_offset_local.x` is 16, so park
                // the attacker at `victim_x - 16`.
                //
                // Strike rows do not depend on this: a strike spawns its hitbox
                // at the victim's own pose.
                //
                // Do not use `park`, which forces y = 200 (76px above a resting
                // body). Keep the attacker's settled height; move only x.
                let anchor_x = victim_x - 16.0;
                let attacker_y = self.pos(attacker).y;
                probe_stage::pin_grounded_at_rest(
                    &mut self.app,
                    attacker,
                    EVec2::new(anchor_x, attacker_y),
                );
                // `lasting`, not `default()`. A default `SmashHoldState` has
                // `escape_seconds == 0.0`, which `escaped()` reads as a hold
                // that is already over.
                self.app.world_mut().entity_mut(self.victim).insert((
                    ambition_platformer2d::combat::capture::CapturedBy {
                        captor: attacker,
                        hold_offset_local: EVec2::new(16.0, 0.0),
                        prior_gravity_scale: 1.0,
                    },
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
        // The trial ends when it is decided, not when the budget runs out.
        // A surviving body at rest otherwise costs the full 150 ticks.
        //
        // Each arming condition excludes a way to be still without the trial
        // being over:
        //   * `resolved_launch.is_some()`: before contact the victim is parked
        //     and still.
        //   * `!is_in_hitlag()`: hitstop freezes both bodies.
        //   * `hitstun_timer <= 0`: the launch still carries the body.
        //   * grounded and slow for `SETTLED_TICKS` consecutive ticks, so one
        //     frame of ground contact mid-arc cannot end the trial.
        // A body that meets all four cannot reach a blast line.
        const SETTLED_TICKS: usize = 4;
        const SLOW_PX_S: f32 = 12.0;
        let mut settled_for = 0usize;
        // A throw publishes no `HitEvent`, so the cursor above never sees it.
        // `apply_capture_throws` calls `apply_body_hit_reaction` directly, and
        // that publishes nothing. Watch for the release instead: the throw
        // releases the captive, so on the first tick without `CapturedBy` the
        // victim's velocity is the launch.
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
            // Observe the throw's launch; do not recompute it. Evaluating the
            // launch law on `CaptureThrowParams` would print the formula's own
            // prediction, whatever the engine did.
            //
            // The value is one tick late: the same `update()` also ran gravity,
            // so the vertical component is short by one frame of gravity
            // (~24 px/s at 1450). This is not compensated; it is small against
            // launches of 500-1200. `victim_gravity` prints the live value.
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
            // The last pose while still in play, sampled before the KO test. A
            // KO'd fighter respawns, so a later pose shows the respawn point.
            let live_pos = self.pos(self.victim);
            // Sampled beside `live_pos` for the same reason: after the KO the
            // respawned body has no hitstun or tumble.
            //
            // `tumble_until_landing` is left out on purpose. Control returns
            // before it clears, so including it would count recoverable falls
            // as forced kills. `tumble_timer` is the helpless part.
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
                    // Which line it crossed. `HitSource::LeftTheWorld` does not
                    // give a direction. Printed on stderr, not as a column, so
                    // the table stays byte-comparable with
                    // `envelope_AFTER_LAWONLY`.
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

        // A throw that never executed is a refusal, not a survival.
        // `apply_capture_throws` skips silently when its `find` matches
        // nothing (a victim missing a queried component, a hold already over,
        // a captor that is itself captured). The throw releases the captive,
        // so `CapturedBy` still present means the system never ran.
        if let Pulse::Throw(_) = pulse {
            let executed = self
                .app
                .world()
                .get::<ambition_platformer2d::combat::capture::CapturedBy>(self.victim)
                .is_none();
            // The fixture's hold must not leak into the next trial, or the next
            // pulse measures a captive.
            self.app.world_mut().entity_mut(self.victim).remove::<(
                ambition_platformer2d::combat::capture::CapturedBy,
                ambition_platformer2d::characters::control::ControlHolds,
                ambition_platformer2d::characters::smash_hold_state::SmashHoldState,
            )>();
            if !executed {
                // The other silent `None`. Name it, so the table can separate
                // it from the grounded refusal.
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
            // `apply_capture_throws` damages before reading the meter, so a
            // throw's launch sees `entry + damage` and a strike's sees `entry`.
            effective_percent: match pulse {
                Pulse::Strike(_) => entry_percent,
                Pulse::Throw(params) => entry_percent + params.damage,
            },
            at_strike,
            ko_forced,
        })
    }

    /// Fire a strike pulse. `run_contact` and `run_determinism` are
    /// strike-only diagnostics, so they use this name.
    fn strike(&mut self, hit: &HitVolume, entry_percent: i32, victim_x: f32) -> Option<Trial> {
        self.fire(Pulse::Strike(hit), entry_percent, victim_x)
    }

    /// The resolved launch at one percent, retrying a refused reset.
    fn launch_at(&mut self, pulse: Pulse<'_>, percent: i32, victim_x: f32) -> Option<f32> {
        for _ in 0..3 {
            if let Some(t) = self.fire(pulse, percent, victim_x) {
                return t.resolved_launch;
            }
        }
        None
    }

    /// Does this pulse KO at `percent`? Decided by majority of three.
    ///
    /// Far from a threshold, trials repeat exactly. Within a percent or two of
    /// the blast line, one tick can decide the outcome, so one trial is not an
    /// answer. Majority of three short-circuits when a side reaches two.
    ///
    /// A refused reset is retried, not counted. `None` means no clean trial
    /// was possible, so the caller can report it instead of a survival.
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
        // A mixed vote is a measurement. Reaching here with both counters
        // non-zero means 2:1: the same start gave different outcomes, so the
        // cell sits on an edge. Arithmetic screens did not predict which
        // cells do this; the trials show it directly.
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

    /// Coarse sweep, bracket, binary search, then verify both sides.
    ///
    /// Not a bare binary search: the launch formula is monotonic in percent,
    /// but a trajectory is not (platforms, ceilings, downward launches, and
    /// landings intervene).
    fn ko_threshold(&mut self, pulse: Pulse<'_>, victim_x: f32, max_percent: i32) -> Threshold {
        // Coarse step default 50. The threshold is the same as with 25, but
        // the sweep also spots `NonMonotonic`, and fewer samples can miss a
        // curve that doubles back between two samples. The both-sides check
        // still catches one at the threshold.
        //
        // A window narrower than the step can be skipped. Example: George's
        // forward throw has a non-tumbling window (launch ~402-500, below
        // `tumble_speed = 500`) where it kills by falling off the stage edge;
        // step 50 misses it. Distrust a cell whose KO launch is near
        // `tumble_speed`. Use `step=<n>` to re-sweep a suspect cell; the
        // default stays 50 so recorded runs keep their meaning.
        let step: i32 = std::env::args()
            .find_map(|a| a.strip_prefix("step=").and_then(|n| n.parse::<i32>().ok()))
            .filter(|n| *n > 0)
            .unwrap_or(50);
        // Calibration mode: exact, not bracketed.
        //
        // Calibration inverts `G_new = G_old * p0/p1` and trusts `p0`, so a
        // swept threshold would put a sweep artifact into a shipped growth.
        // Ascend by 1; the first kill is the answer, because `p - 1` was just
        // measured and survived. A mixed vote is invalid here, not outvoted.
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
            // A KO followed by a survival at a higher percent is a real
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
            // "Never killed" and "never connected" are different answers. One
            // probe at the top of the range separates them.
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
        // Zero is the floor. `hi == 0` means the pulse KOs at zero percent;
        // there is no lower percent to verify, so skip the both-sides check.
        if hi == 0 {
            return Threshold::At(0);
        }
        // Verify both sides; do not trust the walk. Use `kills`, not `strike`,
        // so one trial does not decide and a refused reset is not read as a
        // survival.
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

/// At which percents does the pulse fail to connect?
///
/// A no-contact band can read as a band the victim survived, so the threshold
/// search would walk past it. The emitted magnitude variant comes from the
/// `HitboxKnockback` this probe builds (`LaunchSpeed`), and `set_damage_taken`
/// is `accumulated = damage.max(0)` with no cap, so neither explains it. This
/// measures it.
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

/// Does a trial inherit anything from the trial before it?
///
/// The self-test in `run_probe` fires one pulse at one percent twice with the
/// same history. That proves determinism given the same past. The threshold
/// search needs the answer at a percent to be independent of what ran before,
/// and this measures that. It prints state instead of asserting, because the
/// question is which fact carries over.
fn run_determinism() {
    // Stand near a real boundary: flakiness lives within a percent or two of
    // the blast line, and a test at a comfortable percent cannot fail.
    //
    // Measured on this tree for `smash_forward` at centre:
    //     125% -> survive (launch 644.4)
    //     150% -> KO      (launch 741.2)
    // and 145% already kills every time, so the flip sits below it. The
    // single-trial control below shows whether 130 wavers.
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
    // A forward tilt, whose centre cell showed a genuine disagreement.
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

    // This arm tests the remedy: that `kills` (majority of three) gives the
    // same verdict every time at a percent where one trial does not. The
    // single-trial rows may disagree; that is the defect being reproduced.
    // If these six disagree, majority of three is not enough; run no matrix.
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
    // Do not claim a boundary that is not shown. Agreement among the votes
    // means something only if the single-trial control disagrees at this
    // percent.
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
    // Print the measured launch; do not print a placeholder. A fixed `-`
    // reads as "no contact".
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

/// Why does a throw cell refuse at 0%? Name the missing precondition.
///
/// A refused throw cell looks like a weak throw. `apply_capture_throws` takes
/// an eleven-component query on the captive (eight required) and skips
/// silently when `find` matches nothing. The fixture in `capture/systems.rs`
/// (`throw_app`/`grounded_body`) builds a captive the system sees. This prints
/// what a live match fighter has compared with that fixture. Note:
/// `gravity/resolve.rs` says a player entity has no `ActorSurfaceState`, while
/// `actor_spawn` and `body_seed` insert one.
///
/// It inserts nothing to repair the body. A default `ActorSurfaceState` would
/// overwrite the real `gravity_scale` and corrupt later trials.
fn run_throw_diag() {
    use ambition_platformer2d::characters::actor as ca;
    use ambition_platformer2d::engine_core as ec;
    use ambition_platformer2d::engine_core::Vec2 as EVec2;

    let attacker_id = "npc_pirate_admiral";
    let victim_id = "player_robot_v3";
    println!("# THROW ACTUATOR DIAGNOSTIC — {attacker_id} throwing {victim_id}");
    println!("# 24/24 stage-3 throw cells were REFUSED@0. This asks WHY.");

    let mut probe = KoProbe::new(attacker_id, victim_id);

    // Separate the two ways `fire` returns `None`. A reset refusal is about
    // the fixture, not the throw path. Continue only if the reset succeeded.
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

    // ── Reproduce the matrix's first call. ──
    //
    // The manual actuation below proves the throw path works, not that `fire`
    // works. `ko_threshold` starts with `kills(pulse, 0, ..)`, which returns
    // `None` (`REFUSED@0`) only when three `fire` calls refuse. Three calls here
    // reproduce that, and the `KO_REFUSE` lines on stderr name the stage.
    // This runs first, because the manual actuation leaves the victim airborne.
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
            // Print `tumbled`, `entry_percent`, and `effective_percent`, which
            // are recorded every trial and otherwise never shown.
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

    // Several ticks, not one. A surviving `CapturedBy` means the system never
    // ran, but a message written from outside the schedule can be read a tick
    // late. "Never" and "not yet" are different diagnoses.
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
    // This is a three-fighter sample, not the roster. The authored-value
    // census walks every seatable fighter; this stage-outcome matrix does not.
    // Cite these cells as a "three-fighter calibration sample", never as roster
    // statistics. Three is enough to ask whether roles separate, because every
    // fighter authors the same role set.
    let attackers = ["npc_pirate_admiral", "smash_george_booul", "npc_bob"];
    // The ceiling comes from argv; 300 stays the default so runs recorded in
    // PROVENANCE_RUNS.md replay to the same numbers.
    //
    // A `>300` cell is not a weak move; it is a missing measurement.
    // Calibration inverts `growth' = growth * (p0/p1)` and has no `p0` for a
    // censored cell. Cost: the coarse pass grows linearly with the ceiling, so
    // `ceiling=600` about doubles the sweep.
    let max_percent: i32 = std::env::args()
        .find_map(|a| {
            a.strip_prefix("ceiling=")
                .and_then(|n| n.parse::<i32>().ok())
        })
        .unwrap_or(300);

    // One matchup per process. The matrix cells share nothing (separate Apps),
    // so this only divides wall-clock time. With no arguments the tool walks
    // the whole matrix in one process.
    let argv: Vec<String> = std::env::args().collect();
    let after: Vec<&str> = argv
        .iter()
        .skip_while(|a| a.as_str() != "probe")
        .skip(1)
        .map(|s| s.as_str())
        // Flags are not fighters. Without this filter, `probe <attacker>
        // identity` reads "identity" as the victim id, and an unfiltered
        // `ceiling=<n>` in `after[0]` becomes the attacker. Either gives a
        // table for a matchup nobody asked for.
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
    // Print the sweep ceiling: `>N` means different things at different N,
    // and a table must carry its own ceiling to be comparable.
    println!(
        "# sweep ceiling: max_percent={max_percent} — a `>{max_percent}` cell is CENSORED \
         (no threshold found at or below it), NOT a measurement that the move is weak"
    );
    // Print the coarse step too: tables at different steps can report
    // different thresholds for the same ruleset, because a narrow KO regime
    // can be stepped over. The line also proves that `step=` reached the sweep.
    //
    // Calibration mode is off unless asked for. `calib` replaces the coarse
    // sweep, bracket, and binary search with an exact step-1 scan, and refuses
    // a self-disagreeing cell. It is too slow for a survey (a cell that kills
    // near 110% costs ~110 trials), so `only=<substring>` filters by `move_id`.
    let calib = std::env::args().any(|a| a == "calib");
    let only: Option<String> =
        std::env::args().find_map(|a| a.strip_prefix("only=").map(|s| s.to_string()));
    // The step line must describe the pass that ran. Under `calib` the step is
    // 1, not the coarse step.
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
    // Print both flags so their arrival can be checked directly.
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
    // Every KO percent below is a no-recovery lower bound. Both bodies have
    // `Brain::stand_still()`, so the victim never DIs, jumps, air-dodges, or
    // recovers. A real victim's threshold is higher by an unmeasured amount.
    // Use the columns to compare cells measured the same way: role with role,
    // centre with ledge, reference weight with heavy.
    println!(
        "# victim brain: stand_still — no DI, no jump, no recovery. KO% is a \
         NO-RECOVERY LOWER BOUND, not a kill percent."
    );
    // Print the pin only when it happened. Do not restate
    // `rage_per_damage`/`rage_max_scale`: `ambition_demo_smash` authors them.
    // This tool states only the meter it set.
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
    // Aerial roles are measured under conditions they never occur in. Each
    // trial parks the victim on the platform, so an `attack_air*` pulse hits a
    // grounded body from a grounded attacker. Read the grounded roles (attack*,
    // smash*, tilt*) as measurements and the aerial roles as not yet measured.
    // An empty aerial cell can also be a reset refusal (`stage=ground` in
    // `KO_REFUSE`), not this limitation; check stderr.
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
    // Throw rows read differently from strike rows in the same table. Say how.
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
                // Throws are admitted unconditionally. An unbound strike is a
                // hit volume no verb reaches, so it is skipped. A throw has no
                // `verbs` list: `launchers_of` builds each one through
                // `move_for_verb`, so it is bound by construction.
                launchers_of(attacker_id, contract, &mut bad, &mut nl)
                    .into_iter()
                    .filter(|l| match l {
                        Launcher::Strike { verbs, .. } => !verbs.is_empty(),
                        Launcher::Throw { .. } => true,
                    })
                    // `only=` narrows the population and does not change a
                    // number: each launcher's search is independent.
                    .filter(|l| match &only {
                        Some(want) => l.move_id().contains(want.as_str()),
                        None => true,
                    })
                    .collect::<Vec<_>>()
            };

            // The self-test. Compare the whole outcome, not only `ko`: two
            // trials can agree on `ko` and start in different states, and
            // `at_strike` catches that. Use three repetitions, because an
            // alternating defect (survive/KO/survive) is invisible to a
            // comparison of the first two.
            //
            // Do not match only `Launcher::Strike`: a fighter whose first
            // launcher is a throw would skip the self-test silently.
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
            // Derive the ledge positions from the platform, inboard of its
            // edge. `smash_stage` builds one solid at min=(80,300)
            // size=(480,32), so the platform spans 80..560. A body on the last
            // pixel decides `on_ground` sub-pixel, and the cell becomes a coin
            // flip. Read the real edge and stand a margin inboard.
            // Both edges: see the per-move choice at the `ledge_ko` call below.
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
                // Through `launch_at` like the other two. This value is both
                // the printed column and the midpoint for the linearity check.
                let launch_at_100 = probe.launch_at(pulse, 100, probe.centre);
                // Solve the tumble crossing; do not sweep it. The launch law is
                // linear in victim percent, so two samples fix the line and a
                // third must land on it. If the third misses, print NONLINEAR
                // and no crossing.
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
                // Pick the outward ledge per move. At the right ledge, a
                // negative-x launcher sends the victim across the whole stage,
                // which measures cross-stage kill power, not ledge kill power.
                // The move's authored launch direction picks the edge. `facing`
                // is pinned +1, so world launch follows the sign of
                // `launch_dir.x`. A `None` dir derives from attacker-relative
                // position; the attacker stands to the left, so it launches
                // right and keeps the right ledge.
                let ledge_x = match l.launch_dir() {
                    Some((x, _)) if x < -0.05 => left_ledge_x,
                    _ => right_ledge_x,
                };
                let ledge_ko = probe.ko_threshold(pulse, ledge_x, max_percent);
                // A throw has no window/volume address. Do not print `w0v0`,
                // which reads as a real location.
                let wv = match l {
                    Launcher::Strike { window, volume, .. } => format!("w{window}v{volume}"),
                    Launcher::Throw { .. } => "-".to_string(),
                };
                // Ticks from contact to blast line at the cell's threshold:
                // measures whether a launch feels slow.
                // Match by reference: `Threshold::NonMonotonic` holds a `Vec`,
                // so the enum is not `Copy`, and `centre_ko.cell()` is used
                // below.
                let ko_ticks = match &centre_ko {
                    Threshold::At(p) => match probe.fire(pulse, *p, probe.centre) {
                        Some(t) => {
                            // Does this cell measure kill power? Only if the
                            // victim crossed the blast line while unable to
                            // act. Otherwise the body fell and the stage took
                            // the stock. On stderr, not as a column, so the
                            // table stays byte-comparable with
                            // `envelope_AFTER_LAWONLY`.
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
    // Before `probe`: a table measured with non-independent trials is worse
    // than no table.
    if std::env::args().any(|a| a == "contact") {
        run_contact();
        return;
    }
    if std::env::args().any(|a| a == "determinism") {
        run_determinism();
        return;
    }
    // Before `probe`, like `contact`: this says why throw rows refuse.
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
    // A `Startup` system fills the registry. Before the first update there is
    // no registry, and the census would read as an empty roster.
    for _ in 0..4 {
        app.update();
    }

    let world = app.world();
    let registry = world
        .get_resource::<ambition_platformer2d::characters::prepared::PreparedCharacterRegistry>()
        .expect("the composed host has a prepared-character registry");
    // Not the live resource. `project_combat_rules` folds `DeclaredCombatRules`
    // every tick, but the Smash declaration exists only after the Smash
    // experience is entered. Before that the resolved resource is `Default`
    // (for example `knockback_growth=0`, which zeroes every unauthored growth).
    // Ask the declaration and fold it as the engine does. The stage probe also
    // asserts it against the live resource inside a real match.
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

    // The population is the shipped host's own roster, printed so the census
    // is not read as covering a cast it did not cover.
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

    // Bound roles only, side by side.
    //
    // Unbound pulses (no verb reaches them) are excluded here and counted
    // separately, because they would pull every median toward stand-in values.
    // No spread ratio is printed: fixed knockback has a real zero
    // growth/base, so a max/min ratio can divide by zero. Min, median, and max
    // carry the same information.
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

    // Role summaries.
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

    /// The census population is a claim: an empty roster would make every
    /// distribution vacuously fine.
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

    /// The roster's fighters author throws, reachable by verb through the
    /// runtime contract.
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
                // A carry is not a defect. The goblin's down press carries
                // instead of launching, on purpose (guarded by
                // `the_goblins_down_throw_hauls_instead_of_launching`).
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
        // The floor matters: without it this passes on a roster where every
        // lookup returns `None`. "No malformed rows" cannot tell an empty
        // census from a healthy one.
        assert!(
            resolved >= 20,
            "only {resolved} throws resolved across the whole roster ({carries} carries). \
             Every fighter authors a capture kit, so a number this small is a BROKEN LOOKUP \
             rather than a game without throws."
        );
        println!("{resolved} throws + {carries} carries over {fighters_with_throws} fighters");
    }

    /// `Some(0.0)` is fixed knockback. It must not fold into "the volume did
    /// not decide".
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

    /// A role must come from an authored binding, never from a name guess.
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

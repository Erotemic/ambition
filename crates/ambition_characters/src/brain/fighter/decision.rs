//! **FB4b — the rig that turns the fighter brain into inputs.** (§13)
//!
//! Everything below this file is pure and already tested: `classify` (L1),
//! `generate_options` (L2), `refine_by_rollout` (L3), the `HabitModel`, the
//! `DelayedPerception` buffer. None of it emitted a control frame, so the whole
//! L3 investment sat unexercised on the ladder — every row stays
//! `rollout_depth: 0` until a brain that PLAYS can be measured.
//!
//! This is that brain. It is mostly plumbing plus three careful pieces, and each
//! of the three is rollback state rather than cache:
//!
//! * **cadence** — a decision every `decision_interval_ticks`, with the chosen
//!   intent HELD in between. A brain that re-decided every tick would be
//!   frame-perfect in a way no player is, and the held intent is what a human's
//!   hand actually does between thoughts.
//! * **APM** — enforced at the ONE emission point, so the humanity histogram
//!   measures what the brain DID rather than what it wanted.
//! * **noise** — one `u64` stream, stepped only when consumed, spending samples
//!   on press TIMING only. The moveset aims the melee; there is no aim noise in
//!   v1.
//!
//! ## Every field of `FighterState` gates behaviour, so every field rewinds
//!
//! That is the derive-memo rule applied in advance rather than after a desync:
//! a field that decides what the brain does next is not a cache of something
//! recomputable, it is the brain's position in its own loop. `BossPatternState`'s
//! `rng_seed` is the precedent — it is snapshot-registered for exactly this
//! reason, and a noise stream that did not rewind would make the same fighter
//! throw a different jab on a replay.

use crate::actor::control::ActorControlFrame;
use crate::brain::fighter::habit::{Choice, HabitModel};
use crate::brain::fighter::options::{
    generate_options, AttackCandidate, MovementVerb, UtilityWeights,
};
use crate::brain::fighter::profile::FighterBrainProfile;
use crate::brain::fighter::rollout::{refine_by_rollout, ShadowTuning};
use crate::brain::fighter::situation::{classify, Situation};
use crate::brain::BrainSnapshot;
use crate::perception::{DelayedPerception, WorldView};

/// Sim rate the cadence and reaction conversions assume when nothing says
/// otherwise. The rig takes `tick_hz` explicitly everywhere it matters; this is
/// only the default a config is built with.
pub const DEFAULT_TICK_HZ: f32 = 60.0;

/// **How often the brain thinks**, in ticks. §5's 10–20 Hz at a 60 Hz sim.
pub const DEFAULT_DECISION_INTERVAL_TICKS: u32 = 5;

/// The immutable half: who this fighter is and how it is allowed to play.
#[derive(Clone, Debug, PartialEq)]
pub struct FighterCfg {
    pub profile: FighterBrainProfile,
    pub tuning: ShadowTuning,
    /// Ticks between decisions. `0` is coerced to 1 — a brain that decides
    /// "every zero ticks" is a divide-by-zero dressed as a config.
    pub decision_interval_ticks: u32,
    pub tick_hz: f32,
}

impl FighterCfg {
    pub fn new(profile: FighterBrainProfile) -> Self {
        Self {
            profile,
            tuning: ShadowTuning::default(),
            decision_interval_ticks: DEFAULT_DECISION_INTERVAL_TICKS,
            tick_hz: DEFAULT_TICK_HZ,
        }
    }

    fn interval(&self) -> u32 {
        self.decision_interval_ticks.max(1)
    }
}

/// **The APM ledger: a RATE, not a log.**
///
/// Two integers, because the question is "presses per minute so far" and a
/// history of press times answers it no better while being unbounded state a
/// rollback has to carry.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ApmLedger {
    pub presses: u32,
    pub elapsed_ticks: u32,
}

impl ApmLedger {
    /// Actions per minute so far. Zero elapsed ticks is zero APM rather than a
    /// division by zero — a brain that has not lived yet has not acted.
    pub fn apm(self, tick_hz: f32) -> f32 {
        if self.elapsed_ticks == 0 || tick_hz <= 0.0 {
            return 0.0;
        }
        let minutes = self.elapsed_ticks as f32 / (tick_hz * 60.0);
        if minutes <= 0.0 {
            return 0.0;
        }
        self.presses as f32 / minutes
    }

    /// Whether one more press would still be under `cap`.
    ///
    /// Asked BEFORE the press, so the cap is a ceiling the brain never crosses
    /// rather than one it crosses and then sits above. A non-positive cap means
    /// "uncapped" — a fixture that wants a frame-perfect brain says `0.0` rather
    /// than an enormous number.
    fn may_press(self, cap: f32, tick_hz: f32) -> bool {
        if cap <= 0.0 {
            return true;
        }
        let would_be = ApmLedger {
            presses: self.presses + 1,
            elapsed_ticks: self.elapsed_ticks.max(1),
        };
        would_be.apm(tick_hz) <= cap
    }
}

/// SplitMix64. One step per CONSUMED sample, which is what makes the stream
/// reproducible under rollback: a tick that reads no noise leaves the seed
/// exactly where it was.
fn split_mix_next(seed: &mut u64) -> u64 {
    *seed = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *seed;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// The next sample in `[-1, 1)`.
fn next_signed_unit(seed: &mut u64) -> f32 {
    let bits = split_mix_next(seed);
    // 53 bits into [0,1), then mapped — the same shape as an f64 uniform, at f32
    // precision, so the distribution is not lumpy at the ends.
    let unit = (bits >> 11) as f64 / (1u64 << 53) as f64;
    (unit * 2.0 - 1.0) as f32
}

/// **The mutable half.** Every field decides what the brain does next, so every
/// field is rollback state.
///
/// No `PartialEq`: `DelayedPerception` holds a `VecDeque<WorldView>` and does not
/// derive it, and comparing two brains by their perception buffers is not a
/// question anything asks. Rollback compares the SNAPSHOT bytes, not the struct.
#[derive(Clone, Debug)]
pub struct FighterState {
    /// What the brain is allowed to see. The ONLY read path — `Perceived` can be
    /// minted nowhere else, which is what makes "the delay buffer is on the only
    /// read path" a type fact rather than a test.
    pub perception: DelayedPerception,
    pub habits: HabitModel,
    /// The intent emitted every tick between decisions. A human's hand does not
    /// go neutral because they stopped thinking.
    pub held: ActorControlFrame,
    /// Ticks until the next decision. Counts DOWN.
    pub ticks_until_decision: u32,
    pub apm: ApmLedger,
    /// A press the brain has committed to, `Some(ticks_until_press)`. The delay
    /// is the execution noise: a human who decides to jab does not jab on the
    /// same frame every time.
    pub pending_press: Option<u32>,
    /// The noise stream. Advanced only when a sample is consumed.
    pub noise: u64,
    /// The foe as it was at the LAST decision, so the next one can name what the
    /// foe did in between and feed the habit model.
    pub last_foe: Option<FoeSample>,
}

/// The observable facts about a foe that a habit is inferred from. Deliberately
/// small: a habit is a read of what somebody DOES, and everything here is
/// visible from across the stage.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FoeSample {
    pub attacking: bool,
    pub on_ground: bool,
    pub shielding: bool,
    /// Signed: positive means the foe was closing the gap.
    pub closing: f32,
}

impl FighterState {
    pub fn new(cfg: &FighterCfg, seed: u64) -> Self {
        Self {
            perception: DelayedPerception::from_reaction_ms(cfg.profile.reaction_ms, cfg.tick_hz),
            habits: HabitModel::new(cfg.profile.read_weight.max(0.0)),
            held: ActorControlFrame::neutral(),
            ticks_until_decision: 0,
            apm: ApmLedger::default(),
            pending_press: None,
            noise: seed,
            last_foe: None,
        }
    }
}

/// **One tick of the fighter brain.**
///
/// Order matters and is the spec's: observe, emit the held intent, age the
/// clocks, mature a pending press, then — on a decision tick — think.
///
/// `view` is this tick's LIVE world, handed in by the integration layer. It goes
/// straight into the delay buffer and is never read directly; what the brain
/// reasons over is whatever the buffer hands back.
pub fn tick_fighter(
    cfg: &FighterCfg,
    state: &mut FighterState,
    snapshot: &BrainSnapshot,
    view: Option<&WorldView>,
    out: &mut ActorControlFrame,
) {
    if let Some(view) = view {
        state.perception.observe(view.clone());
    }

    // The APM window is wall time, not decision time: a brain that thinks slowly
    // and presses every thought is still pressing at that rate.
    state.apm.elapsed_ticks = state.apm.elapsed_ticks.saturating_add(1);

    // EMIT the held intent first, so every tick produces the same frame the last
    // decision asked for. Edges are cleared below unless something arms them
    // this tick — a `melee_pressed` that stayed true would be a button held down
    // forever.
    let mut frame = state.held.clone();
    frame.melee_pressed = false;
    frame.jump_pressed = false;
    frame.dash_pressed = false;

    if state.ticks_until_decision > 0 {
        state.ticks_until_decision -= 1;
    }

    // A committed press matures. Checked BEFORE the decision so a press armed by
    // the previous decision is not silently replaced by the next one.
    match state.pending_press {
        Some(0) => {
            state.pending_press = None;
            // **THE ONE EMISSION POINT.** A press with no APM token is DROPPED
            // and the held movement stays, which is what makes the humanity
            // histogram a measurement of behaviour rather than of intent.
            if state.apm.may_press(cfg.profile.apm_cap, cfg.tick_hz) {
                frame.melee_pressed = true;
                state.apm.presses = state.apm.presses.saturating_add(1);
            }
        }
        Some(ticks) => state.pending_press = Some(ticks - 1),
        None => {}
    }

    if state.ticks_until_decision == 0 {
        state.ticks_until_decision = cfg.interval();
        decide(cfg, state, snapshot, &mut frame);
    }

    state.held = frame.clone();
    *out = frame;
}

/// The decision tick: perceive, classify, generate, refine, translate.
fn decide(
    cfg: &FighterCfg,
    state: &mut FighterState,
    snapshot: &BrainSnapshot,
    frame: &mut ActorControlFrame,
) {
    let Some(view) = state.perception.perceive() else {
        return;
    };
    let situation = classify(view);

    // **HABIT OBSERVATION IS PART OF THE DECISION TICK** (§13.5). The foe's
    // observable choice since the last decision is fed to the model under the
    // situation that was live when it happened. This is FB5's missing writer —
    // until now the only thing that called `observe` was a test.
    let sample = foe_sample(view);
    if let (Some(previous), Some(current)) = (state.last_foe, sample) {
        state
            .habits
            .observe(situation, infer_choice(previous, current));
    }
    state.last_foe = sample;

    // **THE KIT RIDES THE SNAPSHOT** (§13.2). The brain cannot see the body's
    // moveset — `ambition_combat` depends on `ambition_characters` and not the
    // reverse — so the actors-side snapshot builder fills `attack_kit` from the
    // body's real `ActorMoveset`, exactly like `actor_aerial`. Body-derived truth
    // arriving through the world-in port.
    let options = generate_options(
        view,
        situation,
        &snapshot.attack_kit,
        &cfg.profile.utility_weights,
    );

    // L3 refines L2's ranking when the profile pays for rollouts. `None` means
    // this profile does not, or there was nothing to refine.
    let refined = refine_by_rollout(
        view,
        situation,
        &options,
        &state.habits,
        &cfg.profile,
        &cfg.tuning,
        cfg.tick_hz,
        // How long this body is COMMITTED to whatever it decides: exactly until
        // it decides again.
        cfg.interval(),
    );

    // MOVEMENT: the best verb the rollout did not veto.
    //
    // ⚠ **a verdict nothing consumes is not a verdict.** L3 now rolls each
    // movement line and names the ones that end with this body out of the world;
    // if the rig still took `movement.first()`, that list would be a field in a
    // struct and the fighter would keep walking off the stage — which is the
    // exact defect class this codebase keeps rediscovering (a registration that
    // is inert, a seam that is unreachable, a refusal that cannot fire).
    //
    // L2 scores where the floor is NOW. The rollout is the only thing in the
    // brain that knows where the body will BE, so on this one question it
    // outranks the score rather than adjusting it.
    let vetoed = refined
        .as_ref()
        .map(|refined| refined.suicidal_movement.as_slice())
        .unwrap_or(&[]);
    // **NO VERB HAS SPOKEN YET, SO THERE IS NO LATERAL INPUT YET.** `frame`
    // arrives holding the last decision's answer; clearing here rather than
    // inside each verb makes "nothing was chosen" mean "nothing is pressed"
    // structurally, instead of depending on every branch below to remember.
    //
    // ⚠ this replaced an explicit `halt()` on the empty case. That branch was
    // correct when it was written and became UNREACHABLE the moment the
    // least-bad fallback landed — every vetoed verb is a modelled verb, so an
    // emptied list always has a longest-lived line to fall back to. An
    // unreachable refusal reads as protection while protecting nothing;
    // `ladder_probe` confirmed it fires zero times across five matches.
    frame.locomotion.x = 0.0;
    let chosen = options
        .movement
        .iter()
        .find(|option| !vetoed.contains(&option.verb))
        .map(|option| option.verb)
        // Every option is fatal — but doing nothing is not automatically the
        // safe alternative, and for an airborne body it never is. Take the line
        // that dies latest and leave the most room for the world to change.
        .or_else(|| {
            refined
                .as_ref()
                .and_then(|refined| refined.least_bad_movement)
        });
    if let Some(verb) = chosen {
        apply_movement(verb, view, frame);
    }
    trace_decision(view, &options, vetoed, chosen);

    // ATTACK: a chosen attack becomes a PENDING press, jittered by the profile's
    // execution noise. The winner is L3's when L3 spoke, L2's otherwise.
    let wants_attack = refined
        .as_ref()
        .map(|refined| refined.move_id.clone())
        .or_else(|| options.attacks.first().map(|attack| attack.move_id.clone()));
    if wants_attack.is_some() && state.pending_press.is_none() {
        let jitter = if cfg.profile.execution_noise > 0.0 {
            let sample = next_signed_unit(&mut state.noise).abs();
            (sample * cfg.profile.execution_noise * cfg.interval() as f32).round() as u32
        } else {
            0
        };
        state.pending_press = Some(jitter);
    }
}

/// **One line per decision, when `AMBITION_FIGHTER_TRACE=1`.**
///
/// A diagnostic affordance for `ladder_probe`, and it exists because the last
/// two attempts to fix the fighter walking off the stage were reasoning where a
/// measurement was available — one produced a paralysis that read as a 3×
/// improvement, the other made the number worse. What the probe could see was
/// WHERE the body died and how fast; what it could not see is which verb the
/// brain picked and which ones the rollout struck off, and that is the whole
/// question.
///
/// ⚠ **off by default and read ONCE.** The env lookup is a `LazyLock`, so a
/// production tick pays one relaxed atomic load. It prints to stderr rather than
/// through `bevy::log` deliberately: the probe is a binary that prints a table
/// to stdout, and the trace has to be separable from it by redirection.
///
/// ⚠ **it is not rollback-safe and does not pretend to be.** Under a rollback
/// host a resimulated frame decides again and prints again. The probe is a
/// fixed-tick host; anyone turning this on under GGRS is reading a log with
/// repeats in it, which is a fact about the trace and not about the brain.
fn trace_decision(
    view: crate::perception::Perceived<'_>,
    options: &crate::brain::fighter::options::OptionSet,
    vetoed: &[crate::brain::fighter::options::MovementVerb],
    chosen: Option<crate::brain::fighter::options::MovementVerb>,
) {
    static ENABLED: std::sync::LazyLock<bool> = std::sync::LazyLock::new(|| {
        std::env::var("AMBITION_FIGHTER_TRACE").is_ok_and(|value| value != "0")
    });
    if !*ENABLED {
        return;
    }
    let me = view.self_view;
    eprintln!(
        "[fighter] x={:.0} vx={:.0} ground={} stage={} [{:.0}..{:.0}] floor_edge={:?} offered={:?} vetoed={:?} chose={:?}",
        me.pos.x,
        me.vel.x,
        me.on_ground,
        view.stage.is_known(),
        view.stage.bounds.min.x,
        view.stage.bounds.max.x,
        view.floor_edge_distance().map(|d| d.round()),
        options
            .movement
            .iter()
            .map(|option| option.verb)
            .collect::<Vec<_>>(),
        vetoed,
        chosen,
    );
}

/// What the foe looks like from across the stage, or `None` when there is no foe.
fn foe_sample(view: crate::perception::Perceived<'_>) -> Option<FoeSample> {
    let foe = view.nearest_hostile()?;
    let toward = foe.pos - view.self_view.pos;
    Some(FoeSample {
        attacking: matches!(
            foe.phase,
            crate::perception::BodyPhase::AttackStartup
                | crate::perception::BodyPhase::AttackActive
        ),
        on_ground: foe.on_ground,
        shielding: foe.shield_raised,
        // Positive when the foe's velocity points at me.
        closing: -(foe.vel.x * toward.x.signum()),
    })
}

/// The foe's observable choice between two samples, in §13.5's order.
fn infer_choice(previous: FoeSample, current: FoeSample) -> Choice {
    if current.attacking && !previous.attacking {
        Choice::Attack
    } else if !current.on_ground && previous.on_ground {
        Choice::Jump
    } else if current.shielding {
        Choice::Shield
    } else if current.closing > 0.0 {
        Choice::Approach
    } else if current.closing < 0.0 {
        Choice::Retreat
    } else {
        Choice::Wait
    }
}

/// Translate a movement verb into control-frame fields.
///
/// ⚠ **the sign comes from the perceived foe, not from the actor's facing.**
/// Facing is what the body currently shows and lags a decision; the direction
/// that makes `Approach` mean approach is the one toward the thing being
/// approached.
fn apply_movement(
    verb: MovementVerb,
    view: crate::perception::Perceived<'_>,
    frame: &mut ActorControlFrame,
) {
    let toward = view
        .nearest_hostile()
        .map(|foe| (foe.pos.x - view.self_view.pos.x).signum())
        .unwrap_or(0.0);
    frame.shield_held = false;
    // **EVERY VERB AUTHORS THE WHOLE MOVEMENT INTENT.** `frame` arrives holding
    // the last decision's answer, so a verb that merely adds to it inherits the
    // rest — and `Jump` used to add a jump on top of whatever walk was already
    // running. That is a body still walking in a direction THIS decision did not
    // choose, and on a stage with edges it is the direction the veto had just
    // struck off the list: veto Retreat, choose Jump, keep walking right.
    //
    // Lateral is cleared by the CALLER before any verb speaks, so a verb that
    // does not set it (Jump, Blink) leaves the body with no walk rather than
    // with the previous decision's. Facing is deliberately not cleared: which
    // way a body looks between decisions is the held intent doing its job.
    //
    // **AND THE JUMP BUTTON IS RELEASED THE SAME WAY.** `jump_held` was written
    // `true` at two verbs and `false` nowhere, so one jump pinned the button down
    // for the rest of the match — the `locomotion.x` leak again, one field over,
    // and I fixed the stick and left the button. A held jump is a real input (it
    // is what buys height), which is exactly why it has to be re-stated by
    // whichever verb is chosen rather than inherited by whichever verb is not.
    frame.jump_held = false;
    match verb {
        MovementVerb::Approach => {
            frame.locomotion.x = toward;
            frame.facing = toward;
        }
        MovementVerb::Retreat => {
            frame.locomotion.x = -toward;
            frame.facing = toward;
        }
        MovementVerb::Jump => {
            frame.jump_pressed = true;
            frame.jump_held = true;
        }
        MovementVerb::Dash => {
            frame.dash_pressed = true;
            frame.locomotion.x = toward;
            frame.facing = toward;
        }
        MovementVerb::Shield => {
            frame.locomotion.x = 0.0;
            frame.shield_held = true;
        }
        MovementVerb::Blink => {
            frame.blink_pressed = true;
        }
        MovementVerb::Recover => {
            // Toward the stage centre, which is the one thing `Recovery` cares
            // about — and up, because a body below the ledge needs height more
            // than it needs lateral progress.
            //
            // ⚠ this was `-pos.x.signum()`, which is "toward the WORLD ORIGIN"
            // and is only the stage centre for a stage built around x=0. Rooms in
            // this engine start at (0,0) and extend positive, so the origin is a
            // CORNER: every body on the left half of every stage recovered by
            // driving further left, into the blastzone it was trying to escape.
            // The stage knows where its middle is; ask it.
            let home = ((view.stage.bounds.min.x + view.stage.bounds.max.x) * 0.5
                - view.self_view.pos.x)
                .signum();
            frame.locomotion.x = home;
            frame.facing = home;
            frame.jump_pressed = true;
            frame.jump_held = true;
        }
    }
}

/// Which situation the brain last classified — read by the ladder rig and the
/// humanity checks, both of which need to know what the brain thought it was
/// doing rather than only what it emitted.
pub fn situation_of(state: &FighterState) -> Option<Situation> {
    state.perception.perceive().map(classify)
}

/// The kit an option generator needs, as the snapshot carries it.
pub type AttackKit = Vec<AttackCandidate>;

/// The utility weights a profile plays under, exposed so a fixture can assert
/// the rig uses the PROFILE's rather than a default.
pub fn weights_of(cfg: &FighterCfg) -> &UtilityWeights {
    &cfg.profile.utility_weights
}

// A CHILD of the decision module: its tests reach `FighterState`'s fields and
// the private clocks, which is the design rather than an accident.
#[cfg(test)]
#[path = "decision/tests.rs"]
mod tests;

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

use ambition_platformer2d_core::{self as ae, Vec2};

use crate::actor::control::ActorControlFrame;
use crate::brain::fighter::habit::{Choice, HabitModel};
use crate::brain::fighter::options::{
    generate_options, AttackCandidate, MovementVerb, UtilityWeights,
};
use crate::brain::fighter::profile::FighterBrainProfile;
use crate::brain::fighter::recovery::{BodyKit, RecoveryLens};
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

/// **A press the brain has committed to, and the press it is.**
///
/// ⚠ this was a bare `Option<u32>` — a delay with no memory of what it was
/// delaying (GPT 5.6, 2026-07-31, finding 2). The scored move was chosen, the
/// count matured, and a NEUTRAL melee edge came out; `trigger_moveset_moves`
/// then resolved whatever the default gesture maps to, which for a body with
/// directional variants is the plain jab whatever the brain had picked.
///
/// The binding rides the jitter because the jitter is EXECUTION noise — a human
/// who decides to up-tilt and presses two frames late still up-tilts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PendingAttack {
    /// Ticks remaining before the press is emitted. Counts DOWN.
    pub ticks: u32,
    /// What to press when it matures.
    pub binding: super::options::AttackBinding,
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
    pub pending_press: Option<PendingAttack>,
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
    // ⚠ **`clear_edges`, not three fields by hand.** This open-coded melee,
    // jump and dash — and `MovementVerb::Blink` sets `blink_pressed`, which was
    // not among them, so one Blink decision emitted a press edge on EVERY tick
    // until the next decision overwrote it (GPT 5.6, 2026-07-31, finding 3).
    // Cooldowns masked some of it; the control stream was still several presses
    // for one choice. The helper was incomplete too — it is complete now, and
    // this is the caller that proves it: an edge added to the frame and not to
    // `clear_edges` re-fires here.
    let mut frame = state.held.clone();
    frame.clear_edges();

    if state.ticks_until_decision > 0 {
        state.ticks_until_decision -= 1;
    }

    // A committed press matures. Checked BEFORE the decision so a press armed by
    // the previous decision is not silently replaced by the next one.
    match state.pending_press {
        Some(PendingAttack { ticks: 0, binding }) => {
            state.pending_press = None;
            // **THE ONE EMISSION POINT.** A press with no APM token is DROPPED
            // and the held movement stays, which is what makes the humanity
            // histogram a measurement of behaviour rather than of intent.
            if state.apm.may_press(cfg.profile.apm_cap, cfg.tick_hz) {
                press_the_chosen_attack(binding, &mut frame);
                state.apm.presses = state.apm.presses.saturating_add(1);
            }
        }
        Some(pending) => {
            state.pending_press = Some(PendingAttack {
                ticks: pending.ticks - 1,
                ..pending
            })
        }
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

    // **RECOVERY CANCELS A QUEUED ATTACK.** (traced 2026-07-31)
    //
    // A press is armed at one decision and matures several ticks later, and the
    // situation can change in between — which on a platform stage it does, in
    // the one direction that matters. The trace caught it exactly: an attack
    // armed while airborne OVER the lip (`floor_edge=Some(45)`, still `Neutral`)
    // matured two decisions later with the body past the edge and asking to
    // `Recover`, and every attack in this engine LUNGES. So the fighter's own
    // queued swing carried it out at 700 px/s while its emitted input said left.
    //
    // ⚠ **and it is a DROP, not a ban** — the distinction matters now that L2
    // offers a recovering body its lifting moves. The stale press dies here;
    // `generate_options` runs below and re-arms from the Recovery option set in
    // this same tick, so a body whose kit contains a way home presses that
    // instead of nothing. What cannot survive is a press decided under a
    // different situation, which is the whole of the 2026-07-31 trace.
    //
    // (The old note here said *"L2 already refuses to offer attacks in
    // `Recovery`"*. It no longer does — refusing was right about attacking and
    // wrong about the repertoire, since a genre fighter's answer to being
    // offstage IS a move.)
    if situation == Situation::Recovery {
        state.pending_press = None;
    }

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

    // ⭐ **THE ROLLOUT PREDICTS THE BODY IT IS IN, not a default one.** The
    // config's tuning carries the foe assumptions and the hit response; the
    // MOVEMENT half comes from the body's own authored `MovementTuning` when the
    // snapshot carries it. Without this a character that authors its own gravity
    // or run speed is predicted as somebody else — and the shadow's copied
    // constants were three-for-three wrong for weeks under exactly that shape.
    let tuning = match snapshot.movement_tuning.as_ref() {
        Some(movement) => cfg.tuning.clone().with_movement(movement),
        None => cfg.tuning.clone(),
    };

    // ⭐ **THE RECOVERY LENS — the one real-kernel seam in the decision.**
    //
    // Built once per decision (never per rolled line, and never per tick): the
    // world lowering allocates a block per perceived solid and does not change
    // between the lines of one decision. `None` — no kit on the snapshot, or a
    // view that names no stage — leaves L3 exactly as it was, which is what makes
    // this safe for every brain seat that is not a fighter on a stage.
    //
    // Both halves are body-derived truth from the world-in port, the same channel
    // `movement_tuning` and `attack_kit` arrive on. Nothing here interprets the
    // ability set; it is handed to the kernel, which owns what a body can do.
    let lens = snapshot
        .abilities
        .zip(snapshot.movement_tuning.as_ref())
        .and_then(|(abilities, movement)| {
            RecoveryLens::from_view(
                &view,
                BodyKit {
                    abilities,
                    movement: *movement,
                    // ⭐ **the body's own way up, read off its own kit.** The
                    // strongest lifting move it can press from where it is —
                    // derived from move geometry, never from an identity — so
                    // the veto below is taken against a body that can throw its
                    // recovery rather than one that cannot.
                    lift: super::options::lifting_candidates(&snapshot.attack_kit)
                        .first()
                        .map(|c| super::recovery::RecoveryLift {
                            speed: c.frames.lift_speed,
                            after_s: c.frames.lift_at_s,
                        }),
                },
                1.0 / cfg.tick_hz.max(1.0),
            )
        });

    // L3 refines L2's ranking when the profile pays for rollouts. `None` means
    // this profile does not, or there was nothing to refine.
    let refined = refine_by_rollout(
        view,
        situation,
        &options,
        &state.habits,
        &cfg.profile,
        &tuning,
        cfg.tick_hz,
        // How long this body is COMMITTED to whatever it decides: exactly until
        // it decides again.
        cfg.interval(),
        lens.as_ref(),
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
    frame.locomotion = ae::LocalAxes::ZERO;
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
    trace_decision(
        view,
        &options,
        vetoed,
        chosen,
        frame,
        snapshot.subject.as_deref(),
    );

    // ATTACK: a chosen attack becomes a PENDING press, jittered by the profile's
    // execution noise. The winner is L3's when L3 spoke, L2's otherwise.
    // ⛔ **`and_then`, not `map`.** `RefinedChoice::move_id` is the rollout's
    // preferred attack and it is `None` when L2 offered none — `map` wrapped
    // that in a second `Some`, so every decision that ran a rollout requested an
    // attack that named no move, including in `Recovery`. See the field's doc.
    //
    // ⚠ **the BINDING travels with it**, which is the whole of GPT 5.6's finding
    // 2: the winner used to be reduced to "yes, attack" and a tick count, and the
    // press that matured was a neutral melee edge — so the reach, frame-advantage
    // and rollout work decided WHETHER to swing and never WHICH move.
    let wants_attack = refined
        .as_ref()
        .and_then(|refined| refined.binding)
        .or_else(|| options.attacks.first().map(|attack| attack.binding));
    if let (Some(binding), None) = (wants_attack, state.pending_press) {
        let jitter = if cfg.profile.execution_noise > 0.0 {
            let sample = next_signed_unit(&mut state.noise).abs();
            (sample * cfg.profile.execution_noise * cfg.interval() as f32).round() as u32
        } else {
            0
        };
        state.pending_press = Some(PendingAttack {
            ticks: jitter,
            binding,
        });
    }
}

/// **Press the move the brain chose**, in the ordinary gesture vocabulary.
///
/// The verb picks the button and the direction picks the stick, which is exactly
/// what `resolve_attack_gesture` reads and `move_for_directional_verb` resolves —
/// so a fighter reaches its up-tilt the same way a player does, and a move with
/// no binding was never in the kit to be chosen.
///
/// ⚠ **the axis is in the BODY's local frame and the direction is relative to
/// FACING**, so `Forward` is `+x` and `Back` is `-x` before the frame applies
/// facing (`attack_dir_from_axis` multiplies `axis.x * facing`, and the emitted
/// axis is pre-facing). Up is NEGATIVE y — the screen convention `InputState`
/// carries, stated here because getting it backwards silently swaps a body's
/// up-tilt and its down-air.
fn press_the_chosen_attack(binding: super::options::AttackBinding, frame: &mut ActorControlFrame) {
    use super::options::AttackVerb;
    use crate::actor::attack_gesture::AttackDir;

    frame.attack_axis = match binding.direction {
        AttackDir::Neutral => ae::LocalAxes::ZERO,
        AttackDir::Forward => ae::LocalAxes::new(1.0, 0.0),
        AttackDir::Back => ae::LocalAxes::new(-1.0, 0.0),
        AttackDir::Up => ae::LocalAxes::new(0.0, -1.0),
        AttackDir::Down => ae::LocalAxes::new(0.0, 1.0),
    };
    match binding.verb {
        AttackVerb::Basic => {
            frame.melee_pressed = true;
            frame.melee_strong_hint = false;
        }
        AttackVerb::Smash => {
            frame.melee_pressed = true;
            frame.melee_strong_hint = true;
        }
        AttackVerb::Special => {
            frame.special_pressed = true;
        }
    }
}

/// **Publish the decision as a structured causal fact — and render one line of
/// it when `AMBITION_FIGHTER_TRACE=1`.**
///
/// This used to be an `eprintln!` and nothing else. It exists because the last
/// two attempts to fix the fighter walking off the stage were reasoning where a
/// measurement was available — one produced a paralysis that read as a 3×
/// improvement, the other made the number worse. What `ladder_probe` could see
/// was WHERE the body died and how fast; what it could not see is which verb the
/// brain picked and which ones the rollout struck off, and that is the whole
/// question.
///
/// It is a FACT now, for three reasons the text line could not meet:
///
/// * **it is queryable.** `explanation.first("fighter_decision").get("chose")`
///   is a field lookup; the same thing over stderr is a regex over prose that
///   breaks when somebody improves the wording.
/// * **it correlates.** A fact carries a tick, a subject and a generation, so
///   the verb this brain chose can be joined to the movement it produced and
///   the damage that followed. Two unrelated `eprintln!`s cannot be joined at
///   all.
/// * **it labels a repeat.** The old docstring conceded it was *"not
///   rollback-safe and does not pretend to be"* — under a rollback host a
///   resimulated frame decided again and printed again, and two identical lines
///   are indistinguishable from one decision made twice. `Execution` says which.
///
/// ⚠ **one authority.** The stderr line is RENDERED from the fact, so the two
/// cannot drift. A second `eprintln!` carrying a field the fact lacks is the
/// bug this replaces, one level up.
///
/// ⚠ **the tick is stamped by the scope owner, not by this function.** A brain
/// five hops below the ECS does not know the world's clock, and a decision
/// counter guessed here would be a second clock that no other domain could join
/// against. `CausalLog::set_tick` is the one place with the answer.
fn trace_decision(
    view: crate::perception::Perceived<'_>,
    options: &crate::brain::fighter::options::OptionSet,
    vetoed: &[crate::brain::fighter::options::MovementVerb],
    chosen: Option<crate::brain::fighter::options::MovementVerb>,
    frame: &ActorControlFrame,
    // Which body this is, from the snapshot's world-in port. `None` publishes an
    // unattributed fact — honest for a fixture, and useless on a stage with two
    // fighters, which is why the integration layer fills it.
    subject: Option<&str>,
) {
    static ENABLED: std::sync::LazyLock<bool> = std::sync::LazyLock::new(|| {
        std::env::var("AMBITION_FIGHTER_TRACE").is_ok_and(|value| value != "0")
    });

    #[cfg(feature = "causal")]
    let publishing = ambition_causal::recording();
    #[cfg(not(feature = "causal"))]
    let publishing = false;

    if !*ENABLED && !publishing {
        return;
    }

    let me = view.self_view;
    let offered: Vec<_> = options.movement.iter().map(|option| option.verb).collect();
    // ⚠ the subject leads the line for the same reason the fact carries one: two
    // fighters on a stage produced two interleaved streams with nothing to tell
    // them apart, and this trace exists because reasoning about that failed.
    let line = format!(
        "[fighter{}] x={:.0} vx={:.0} ground={} phase={:?} stage={} [{:.0}..{:.0}] floor_edge={:?} offered={:?} vetoed={:?} chose={:?} emit_x={:.1}",
        match subject {
            Some(id) => format!(" {id}"),
            None => String::new(),
        },
        me.pos.x,
        me.vel.x,
        me.on_ground,
        me.phase,
        view.stage.is_known(),
        view.stage.bounds.min.x,
        view.stage.bounds.max.x,
        view.floor_edge_distance().map(|d| d.round()),
        offered,
        vetoed,
        chosen,
        frame.locomotion.x,
    );

    #[cfg(feature = "causal")]
    if publishing {
        use ambition_causal::{domains, CausalFact, FactDetail, SubjectKey};
        // The summary is the same line a human reads; every value a TOOL would
        // want is a field beside it, so nothing has to be parsed back out.
        //
        // The SUBJECT comes from the snapshot, because the brain cannot know
        // which body it is and must not: an unattributed decision fact cannot
        // answer "why did THIS fighter do that" the moment a second fighter is
        // on the stage, which for a fighting game is every interesting tick.
        let mut fact = CausalFact::new(
            domains::BRAIN,
            0,
            FactDetail::new(
                "fighter_decision",
                match chosen {
                    Some(verb) => format!("chose {verb:?}"),
                    None => "chose nothing — every verb was vetoed".to_string(),
                },
            ),
        )
        .field("chose", format!("{chosen:?}"))
        .field("offered", format!("{offered:?}"))
        .field("vetoed", format!("{vetoed:?}"))
        .field("vetoed_count", vetoed.len() as i64)
        .field("pos_x", me.pos.x)
        .field("vel_x", me.vel.x)
        .field("on_ground", me.on_ground)
        .field("phase", format!("{:?}", me.phase))
        .field("stage_known", view.stage.is_known())
        .field(
            "floor_edge_distance",
            view.floor_edge_distance().unwrap_or(f32::INFINITY),
        )
        .field("emit_locomotion_x", frame.locomotion.x);
        if let Some(subject) = subject {
            fact = fact.about(SubjectKey::Sim(subject.to_string()));
        }
        ambition_causal::record(fact);
    }

    if *ENABLED {
        eprintln!("{line}");
    }
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
    // ⛔ **`locomotion.x` IS BODY-LOCAL, and this used to write world `x`.**
    // The kernel says so at `LocalAxes`: *"controlled-body-local axes: +x local
    // side/right … produced by resolving raw ScreenAxes against the body's
    // current AccelerationFrame"*. `player.rs` obeys it; the smash brain obeys
    // it through `side_face_toward_target`, which its comments call correct
    // under any gravity. This brain wrote `(foe.pos.x - self.pos.x).signum()` —
    // a WORLD sign — into that field, in every verb.
    //
    // ⚠ nothing caught it because the conversion is a REINTERPRETATION:
    // `LocalAxes::from_vec(self.locomotion)` copies the components and renames
    // the type, so the type asserts a transform nobody performed.
    //
    // ⚠ and nothing SAW it because the two conventions agree in the only
    // configuration that gets played: under screen-down gravity `side` is world
    // `+x` and `to_local` is the identity, so this change is byte-identical
    // there. It diverges exactly where this brain already reasons correctly —
    // `fighter_from_self(view, gravity_down)` builds the shadow model in the
    // gravity frame, and `is_punishable(foe, me.gravity_down)` reads it. The
    // rollout was frame-aware and the emit was not.
    let frame_axes = view.self_view.acceleration_frame();
    // ⚠ **`f32::signum(0.0)` is `1.0`**, not `0.0` — so a delta that lies
    // exactly along the body's gravity axis (nothing to the side at all) would
    // come back as FULL THROTTLE sideways. The first version of this fix had
    // that bug and the rotated-frame test caught it: forty ticks of `1.0`
    // toward an axis the foe was not on. The deadzone is the same one
    // `smash/emit.rs::signum_or` uses.
    let side_toward = |world_delta: Vec2| {
        let side = frame_axes.to_local(world_delta).x;
        if side.abs() < 0.001 {
            0.0
        } else {
            side.signum()
        }
    };
    let toward = view
        .nearest_hostile()
        .map(|foe| side_toward(foe.pos - view.self_view.pos))
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
    // with the previous decision's.
    //
    // ⭐ **and each verb now writes the WHOLE `LocalAxes`, which is what the
    // heading above always claimed.** It used to assign `locomotion.x` and leave
    // `.y` to whatever the held frame carried — the code authored one component
    // while its own rule said "the whole movement intent". Behaviour is
    // identical because nothing in this brain has ever written `.y` and `held`
    // starts at `neutral()`, so it was 0.0 for the frame's whole life; the point
    // is that the rule and the code now say the same thing. Facing is deliberately not cleared: which
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
            frame.locomotion = ae::LocalAxes::new(toward, 0.0);
            frame.facing = toward;
        }
        MovementVerb::Retreat => {
            frame.locomotion = ae::LocalAxes::new(-toward, 0.0);
            frame.facing = toward;
        }
        MovementVerb::Jump => {
            frame.jump_pressed = true;
            frame.jump_held = true;
        }
        MovementVerb::Dash => {
            frame.dash_pressed = true;
            frame.locomotion = ae::LocalAxes::new(toward, 0.0);
            frame.facing = toward;
        }
        MovementVerb::Dodge => {
            // **THE SAME BUTTON AS `Dash`, AND THE BODY TURNS IT INTO A ROLL**
            // (or an air dodge off the ground). The brain does not get to pick
            // which — `apply_dodge` claims the buffer first on any body that
            // owns the ability — so all this verb decides is the DIRECTION, and
            // the stick is what carries it: `apply_dodge` rolls along
            // `local_stick.x`, falling back to facing when the stick is neutral.
            //
            // ⭐ **away from a swing, into everything else**, which is the whole
            // of what separates the genre's two uses of the roll. A roll is
            // i-frames plus travel: spent AWAY from an attack it is the evade,
            // spent TOWARD a standing opponent it is the approach that cannot be
            // poked out of. The read is perceivable — is anybody swinging at me
            // — so a human watching the same stage could make it too, which is
            // the no-cheat contract this brain is held to.
            let threatened = view
                .actors
                .iter()
                .any(|actor| actor.hostile_to_self && actor.alive && actor.phase.is_attacking());
            let roll = if threatened { -toward } else { toward };
            frame.dash_pressed = true;
            frame.locomotion = ae::LocalAxes::new(roll, 0.0);
            // ⚠ facing tracks the FOE, not the roll. A body that rolls away
            // while turning its back would come out of the roll facing the
            // blastzone, and its next swing would point at nothing.
            frame.facing = toward;
        }
        MovementVerb::Shield => {
            frame.locomotion = ae::LocalAxes::ZERO;
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
            // Body-local too — see the note where `toward` is derived. The
            // stage centre is a WORLD point, so the delta to it is a world
            // vector and has to be resolved like any other.
            let centre = Vec2::new(
                (view.stage.bounds.min.x + view.stage.bounds.max.x) * 0.5,
                view.self_view.pos.y,
            );
            let home = side_toward(centre - view.self_view.pos);
            frame.locomotion = ae::LocalAxes::new(home, 0.0);
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

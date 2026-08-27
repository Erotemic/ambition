//! Running the scenario suite through the real brain, and reporting what it did.
//!
//! `docs/planning/engine/fighter-brain.md` asks for "the smallest headless runner
//! that executes those scenarios through the real fighter-brain/controller seam
//! and records useful outcomes". This is that runner's first half: everything
//! measurable WITHOUT a match.
//!
//! survival and damage are deliberately not here. They need two bodies
//! actually fighting, which is a match harness (`ambition_demo_smash_app` has
//! one) rather than a brain rig — and a number labelled "survival %" produced
//! without anyone dying would be worse than no number at all. What a brain alone
//! can be asked is: *what did you decide, how often did you press, and do you do
//! the same thing twice.*
//!
//! That is not a degenerate ladder — it is `BrainSnapshot::idle()`, which carries no attack kit,
//! and the decision tests say so in their own words: *"no scene here can arm one"*. An empty kit
//! means `generate_options` offers movement only, so there is no attack for a rung's scoring to
//! differ about.
//!
//! Same for ladder ordering. Building the kit fixture is this rig's next slice and is what also
//! unlocks survival/damage.

use super::data::{FighterCfg, FighterState};
use super::decision::tick_fighter;
use super::options::{ActionLegality, AttackBinding, AttackCandidate, AttackVerb};
use super::profile::FighterBrainProfile;
use super::scenarios::{suite, Scenario};
use crate::actor::attack_gesture::AttackDir;
use crate::actor::control::ActorControlFrame;
use crate::brain::BrainSnapshot;

/// Ticks per second the rig scores against — the sim's fixed rate, so an APM
/// reading here means the same thing it would in a match.
pub const RIG_TICK_HZ: f32 = 60.0;

/// Long enough for a rate to be a rate: ten seconds of decisions.
pub const RIG_TICKS: u32 = (RIG_TICK_HZ as u32) * 10;

/// A kit shaped like the one production builds.
///
/// the rig ran with `BrainSnapshot::idle()` first and every rung emitted zero
/// presses, because an empty kit leaves `generate_options` offering movement
/// only. A brain with nothing to throw cannot be told apart from another brain
/// with nothing to throw, so the ladder read as degenerate.
///
/// Mirrors `build_attack_kit` in the actor tick: one candidate per (verb,
/// direction) the moveset answers for, each carrying its move's frame data. The
/// numbers here are a plausible spread rather than any character's real moveset
/// — the rig measures the DECIDING, and a scenario that named a specific
/// character would be measuring content instead.
fn rig_kit() -> Vec<AttackCandidate> {
    let frames = |startup_s: f32, reach: f32, damage: i32| ambition_entity_catalog::MoveFrameData {
        total_s: startup_s + 0.1 + 0.2,
        charge_hold_at_s: None,
        startup_s,
        active_spans: vec![(startup_s, startup_s + 0.1)],
        recovery_s: 0.2,
        cancel_windows: Vec::new(),
        reach,
        ignores_guard: false,
        // A forward poke of that length — the shape these fixtures mean.
        coverage: (reach > 0.0).then(|| ambition_entity_catalog::MoveCoverage {
            min: (0.0, -12.0),
            max: (reach, 12.0),
        }),
        max_damage: damage,
        max_knockback: 0.0,
        start_impulse: (0.0, 0.0),
        // The rig's moves displace nobody: it measures the DECIDING, and a
        // candidate that carried a way home would put a recovery route into
        // every scenario that has nothing to do with one.
        lift_speed: 0.0,
        lift_at_s: 0.0,
        lift_side: 0.0,
    };
    // Fast-and-short, slow-and-long, and an aerial — enough that scoring has a
    // trade-off to make. One candidate is not a choice.
    vec![
        AttackCandidate {
            move_id: "rig_jab".into(),
            frames: frames(0.03, 40.0, 2),
            binding: AttackBinding {
                verb: AttackVerb::Basic,
                direction: AttackDir::Forward,
            },
            legality: ActionLegality::Now,
        },
        AttackCandidate {
            move_id: "rig_smash".into(),
            frames: frames(0.18, 90.0, 12),
            binding: AttackBinding {
                verb: AttackVerb::Smash,
                direction: AttackDir::Forward,
            },
            legality: ActionLegality::Now,
        },
        AttackCandidate {
            move_id: "rig_uptilt".into(),
            frames: frames(0.06, 55.0, 5),
            binding: AttackBinding {
                verb: AttackVerb::Basic,
                direction: AttackDir::Up,
            },
            legality: ActionLegality::Now,
        },
    ]
}

/// One scenario played by one ladder rung.
#[derive(Clone, Debug, PartialEq)]
pub struct ScenarioOutcome {
    pub scenario: &'static str,
    pub level: u8,
    /// Actions per minute the brain actually emitted.
    pub apm: f32,
    /// What the profile authorises.
    pub apm_cap: f32,
    /// Distinct control frames produced — a decision that never changes is a
    /// brain that is not reacting to the situation it was handed.
    pub distinct_frames: usize,
}

impl ScenarioOutcome {
    /// Did the brain stay inside the press budget its level authors?
    pub fn within_apm_cap(&self) -> bool {
        self.apm <= self.apm_cap
    }
}

/// Play one scenario with one profile and report what the brain did.
///
/// The seed is a parameter because determinism is one of the things being
/// measured: the same seed must produce the same run.
pub fn play(scenario: &Scenario, profile: FighterBrainProfile, seed: u64) -> ScenarioOutcome {
    let level = profile.level;
    let apm_cap = profile.apm_cap;
    let cfg = FighterCfg::new(profile);
    let mut state = FighterState::new(&cfg, seed);
    let mut snapshot = BrainSnapshot::idle();
    snapshot.attack_kit = rig_kit();
    let mut out = ActorControlFrame::neutral();
    let mut frames: Vec<ActorControlFrame> = Vec::new();
    let mut view = scenario.view.clone();
    for tick in 0..RIG_TICKS {
        // THE OPPONENT HAS TO MOVE, or the ladder is invisible. A rung's
        // headline difference is `reaction_ms`, and a delayed view of a world
        // that never changes IS the live view — so a static scenario makes every
        // level emit identical frames and a rig built on one would report the
        // ladder as degenerate when it had simply never been asked a question
        // that distinguishes rungs.
        //
        // So the opponent paces: one slow horizontal sweep across the stage,
        // deterministic in `tick`, which is exactly the case a late-seeing brain
        // must lead and an early-seeing one need not.
        let phase = (tick as f32) / (RIG_TICK_HZ * 2.0);
        let sweep = (phase * std::f32::consts::TAU).sin() * 120.0;
        for (actor, origin) in view.actors.iter_mut().zip(scenario.view.actors.iter()) {
            actor.pos.x = origin.pos.x + sweep;
        }
        view.sim_time = tick as f32 / RIG_TICK_HZ;
        tick_fighter(&cfg, &mut state, &snapshot, Some(&view), &mut out);
        if !frames.iter().any(|seen| seen == &out) {
            frames.push(out.clone());
        }
    }
    ScenarioOutcome {
        scenario: scenario.name,
        level,
        apm: state.apm.apm(RIG_TICK_HZ),
        apm_cap,
        distinct_frames: frames.len(),
    }
}

/// The whole suite across the whole authored ladder — the repeatable report.
/// ⛔⛔ IT WALKS 1..=9, AND A COMPOSITION NEED NOT REGISTER ALL NINE. This crate
/// cannot know which rungs a game publishes — `FighterBrainProfile::for_level`
/// answers for any level, so an unregistered one yields a GENERIC FALLBACK that
/// is not a ladder rung. The smash demo registers five (1, 3, 5, 6, 9) and its
/// `ladder_rig` says the rest are *"invalid for this measurement"*. ⇒ **a caller
/// reading a calibration off these rows must filter to the levels ITS
/// composition registers**, or four of the nine rows are synthetic and the curve
/// through them is partly invented.
pub fn report(seed: u64) -> Vec<ScenarioOutcome> {
    let scenarios = suite();
    let mut rows = Vec::new();
    for level in 1..=9u8 {
        let profile = FighterBrainProfile::for_level(level);
        for scenario in &scenarios {
            rows.push(play(scenario, profile.clone(), seed));
        }
    }
    rows
}

#[cfg(test)]
mod tests;
